// Game-side glue for Ender IO's item conduit (injected into `crate::game` as
// `enderio_state`; it needs the game, so it is not part of the standalone
// library).
//
// Upstream a conduit bundle is a block entity whose node holds, per side, the
// connection configuration, and whose network moves items between the
// inventories its connections reach. This slice keeps the per-side
// configuration in the block entity's saved compound (as upstream's node data
// is), draws the connection state as the block's six boolean properties, and
// does the moving after the level's tick: every cycle, each extracting side
// hands up to `SPEED` items to the network's inserting sides.

use std::collections::BTreeSet;

use horizon_client::containers::{BlockEntity, ModBlockEntity};
use horizon_client::text::Text;
use horizon_core::geometry::Direction;
use horizon_world::nbt::{Compound, JavaString, Tag};

use crate::game::enderio::{
    CYCLE_TICKS, Config, FACE_COUNT, FACE_NAMES, Face, Neighbour, Port, SPEED, connection_bits,
    face_named, face_normal, insert_order, is_connected, is_cycle_tick, ports,
};
use crate::game::native_hooks::flags;
use crate::game::native_server::{NativeCommand, NativeServerPlayer};

/// The conduit bundle block (and its item).
const CONDUIT: &str = "enderio:conduit";

type Pos = [i32; 3];

fn key(name: &str) -> JavaString {
    JavaString::from(name)
}

fn int(nbt: &Compound, name: &str) -> Option<i32> {
    match nbt.get(&key(name)) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    }
}

/// A conduit's saved configuration; a fresh conduit has upstream's defaults
/// (extract everywhere, insert nowhere, no round robin).
fn read_config(nbt: &Compound) -> Config {
    Config::from_saved(
        int(nbt, "insert").unwrap_or(0),
        int(nbt, "extract").unwrap_or(0b11_1111),
        int(nbt, "roundRobin").unwrap_or(0) != 0,
    )
}

fn write_config(nbt: &mut Compound, config: &Config) {
    let (insert, extract, round_robin) = config.saved();
    nbt.insert(key("insert"), Tag::Int(insert));
    nbt.insert(key("extract"), Tag::Int(extract));
    nbt.insert(key("roundRobin"), Tag::Int(i32::from(round_robin)));
}

/// A placed conduit gets a block entity; a fresh one holds the default
/// configuration, which [`read_config`] answers for an empty compound.
pub(super) fn on_block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if block == CONDUIT && entity.is_none() {
        *entity = Some((CONDUIT.to_string(), Compound::new()));
    }
}

/// The conduit block entity at `pos`, if it is ours.
fn conduit_at<'a>(game: &'a crate::game::Game<'_>, pos: Pos) -> Option<&'a ModBlockEntity> {
    match game.world.block_entities.get(&(pos[0], pos[1], pos[2])) {
        Some(BlockEntity::Mod(entity)) if entity.id == CONDUIT => Some(entity),
        _ => None,
    }
}

/// Every conduit in the level, in a stable order.
fn conduit_positions(game: &crate::game::Game<'_>) -> Vec<Pos> {
    let mut positions: Vec<Pos> = game
        .world
        .block_entities
        .iter()
        .filter_map(|(pos, entity)| match entity {
            BlockEntity::Mod(entity) if entity.id == CONDUIT => Some([pos.0, pos.1, pos.2]),
            _ => None,
        })
        .collect();
    positions.sort_unstable();
    positions
}

/// What the block beside face `face` of the conduit at `pos` is.
fn neighbour(game: &crate::game::Game<'_>, dimension: &str, pos: Pos, face: Face) -> Neighbour {
    let [dx, dy, dz] = face_normal(face);
    let beside = [pos[0] + dx, pos[1] + dy, pos[2] + dz];
    if game.native_block(beside) == CONDUIT {
        return Neighbour::Conduit;
    }
    // The neighbour is reached through the side facing the conduit.
    let across = Direction::ALL[face].opposite();
    match game.native_container_slots(dimension, beside, across) {
        Some(slots) if !slots.is_empty() => Neighbour::Inventory,
        _ => Neighbour::Nothing,
    }
}

fn neighbours(game: &crate::game::Game<'_>, dimension: &str, pos: Pos) -> [Neighbour; FACE_COUNT] {
    std::array::from_fn(|face| neighbour(game, dimension, pos, face))
}

/// A conduit's connection state from its six neighbours.
fn state_of(neighbours: [Neighbour; FACE_COUNT]) -> u8 {
    connection_bits(neighbours)
}

/// Give the conduit at `pos` the connection state `bits`, writing only the
/// faces that changed. The state is the block's six boolean properties, so
/// the drawn arms follow it.
fn show_connections(game: &mut crate::game::Game<'_>, pos: Pos, bits: u8) {
    for face in 0..FACE_COUNT {
        let want = is_connected(bits, face);
        let value = if want { "true" } else { "false" };
        if game.native_block_property(pos, FACE_NAMES[face]).as_deref() == Some(value) {
            continue;
        }
        // `flags::UPDATE_CLIENTS`, as a crop's growth: the clients get the
        // write and neighbours' shapes update, without neighbour updates of
        // their own.
        game.native_set_block_property_with(pos, FACE_NAMES[face], value, flags::UPDATE_CLIENTS);
    }
}

/// The network a conduit belongs to: every conduit reachable through other
/// conduits, following the computed connection state.
fn network_of(game: &crate::game::Game<'_>, dimension: &str, start: Pos) -> Vec<Pos> {
    let mut seen: BTreeSet<Pos> = BTreeSet::new();
    let mut queue = vec![start];
    seen.insert(start);
    while let Some(pos) = queue.pop() {
        for face in 0..FACE_COUNT {
            let [dx, dy, dz] = face_normal(face);
            let beside = [pos[0] + dx, pos[1] + dy, pos[2] + dz];
            if seen.contains(&beside) || neighbour(game, dimension, pos, face) != Neighbour::Conduit
            {
                continue;
            }
            seen.insert(beside);
            queue.push(beside);
        }
    }
    seen.into_iter().collect()
}

/// One inventory a network reaches: the conduit and face that reach it.
#[derive(Debug, Clone, Copy)]
struct Endpoint {
    conduit: Pos,
    face: Face,
    port: Port,
}

impl Endpoint {
    /// The inventory's position and the side of it the conduit reaches, which
    /// is the face opposite the conduit's own (a conduit above a chest takes
    /// and puts items through the chest's upward side).
    fn inventory(&self) -> (Pos, Direction) {
        let [dx, dy, dz] = face_normal(self.face);
        (
            [
                self.conduit[0] + dx,
                self.conduit[1] + dy,
                self.conduit[2] + dz,
            ],
            Direction::ALL[self.face].opposite(),
        )
    }
}

/// The inventories the network's conduits reach, in conduit then face order.
fn endpoints(game: &crate::game::Game<'_>, dimension: &str, conduits: &[Pos]) -> Vec<Endpoint> {
    let mut endpoints = Vec::new();
    for &pos in conduits {
        let Some(entity) = conduit_at(game, pos) else {
            continue;
        };
        let config = read_config(&entity.nbt);
        let bits = state_of(neighbours(game, dimension, pos));
        for port in ports(bits, &config) {
            if neighbour(game, dimension, pos, port.face) != Neighbour::Inventory {
                continue;
            }
            endpoints.push(Endpoint {
                conduit: pos,
                face: port.face,
                port,
            });
        }
    }
    endpoints
}

/// A network cycle's phase, from the lowest conduit's position: it spreads a
/// level's networks over the cycle as upstream's `tickOffset` does.
fn phase_of(conduits: &[Pos]) -> u64 {
    let Some([x, y, z]) = conduits.first().copied() else {
        return 0;
    };
    (i64::from(x) + i64::from(y) + i64::from(z)).rem_euclid(CYCLE_TICKS as i64) as u64
}

/// Move items in one network for one cycle: each extracting side hands up to
/// `SPEED` items to the inserting sides, in the order its configuration
/// gives. An inventory that has room takes the items; the next one gets what
/// is left, as upstream walks its insert paths.
fn tick_network(game: &mut crate::game::Game<'_>, dimension: &str, conduits: &[Pos], cycle: u64) {
    let endpoints = endpoints(game, dimension, conduits);
    for source in &endpoints {
        if !source.port.extract {
            continue;
        }
        let insert: Vec<Endpoint> = endpoints
            .iter()
            .copied()
            // Self-feed stays off, as upstream's default: a conduit does not
            // hand items back to the inventory it took them from.
            .filter(|endpoint| {
                endpoint.port.insert
                    && !(endpoint.conduit == source.conduit && endpoint.face == source.face)
            })
            .collect();
        let ports: Vec<Port> = insert.iter().map(|endpoint| endpoint.port).collect();
        let config = conduit_at(game, source.conduit).map(|entity| read_config(&entity.nbt));
        let round_robin = config.is_some_and(|config| config.round_robin());
        let mut budget = SPEED;
        for index in insert_order(&ports, cycle, round_robin) {
            if budget == 0 {
                break;
            }
            let (from_pos, from_face) = source.inventory();
            let (to_pos, to_face) = insert[index].inventory();
            let moved = game.native_move_items(
                dimension, from_pos, from_face, to_pos, to_face, None, budget,
            );
            budget -= moved;
        }
    }
}

/// After the level's tick: refresh every conduit's drawn connections and, on
/// the network's cycle tick, move items.
pub(super) fn on_level_tick_post(game: &mut crate::game::Game<'_>, dimension: &str) {
    let all = conduit_positions(game);
    if all.is_empty() {
        return;
    }
    let game_time = game.world.game_time.max(0) as u64;
    let cycle = game_time / CYCLE_TICKS;
    let mut done: BTreeSet<Pos> = BTreeSet::new();
    for &pos in &all {
        if done.contains(&pos) {
            continue;
        }
        let network = network_of(game, dimension, pos);
        for &member in &network {
            let bits = state_of(neighbours(game, dimension, member));
            show_connections(game, member, bits);
        }
        if is_cycle_tick(game_time, phase_of(&network)) {
            tick_network(game, dimension, &network, cycle);
        }
        done.extend(network);
    }
}

/// Declare `/enderio config` and `/enderio info`.
pub(super) fn on_register_commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new(
        "enderio config <pos:block_pos> <side:word> <option:word> <value:word>",
    ));
    commands.push(NativeCommand::new("enderio info <pos:block_pos>"));
}

/// The faces a predicate picks, as `north,south`.
fn face_list(pick: impl Fn(Face) -> bool) -> String {
    let names: Vec<&str> = (0..FACE_COUNT)
        .filter(|&face| pick(face))
        .map(|face| FACE_NAMES[face])
        .collect();
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join(",")
    }
}

/// `on`/`true` and `off`/`false`, in any case.
fn on_off(value: &str) -> Option<bool> {
    match value.to_lowercase().as_str() {
        "on" | "true" => Some(true),
        "off" | "false" => Some(false),
        _ => None,
    }
}

fn parse_pos(words: &[&str; 3]) -> Option<Pos> {
    Some([
        words[0].parse().ok()?,
        words[1].parse().ok()?,
        words[2].parse().ok()?,
    ])
}

/// Run the `/enderio` commands a player types.
pub(super) fn on_perform_command(
    game: &mut crate::game::Game<'_>,
    _player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let words: Vec<&str> = command.split_whitespace().collect();
    match words.as_slice() {
        ["enderio", "info", x, y, z] => {
            *ran = true;
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal("Usage: /enderio info <x> <y> <z>"));
                return;
            };
            let Some(entity) = conduit_at(game, pos) else {
                game.native_feedback(Text::literal("No conduit there."));
                return;
            };
            let config = read_config(&entity.nbt);
            let connected = face_list(|face| {
                game.native_block_property(pos, FACE_NAMES[face]).as_deref() == Some("true")
            });
            game.native_feedback(Text::literal(format!("Conduit: links {connected}")));
            game.native_feedback(Text::literal(format!(
                "Conduit: config extract {}; insert {}; round robin {}",
                face_list(|face| config.extract(face)),
                face_list(|face| config.insert(face)),
                if config.round_robin() { "on" } else { "off" }
            )));
        }
        ["enderio", "config", x, y, z, side, option, value] => {
            *ran = true;
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal(
                    "Usage: /enderio config <x> <y> <z> <side> <insert|extract|round> <on|off>",
                ));
                return;
            };
            let Some(face) = face_named(&side.to_lowercase()) else {
                game.native_feedback(Text::literal(format!(
                    "Unknown side '{side}': down, up, north, south, west or east."
                )));
                return;
            };
            let Some(on) = on_off(value) else {
                game.native_feedback(Text::literal(format!("'{value}' is not on or off.")));
                return;
            };
            let Some(entity) = conduit_at(game, pos) else {
                game.native_feedback(Text::literal("No conduit there."));
                return;
            };
            let mut config = read_config(&entity.nbt);
            match option.to_lowercase().as_str() {
                "insert" => config = config.with_insert(face, on),
                "extract" => config = config.with_extract(face, on),
                "round" | "round_robin" => config = config.with_round_robin(on),
                other => {
                    game.native_feedback(Text::literal(format!(
                        "Unknown option '{other}': insert, extract or round."
                    )));
                    return;
                }
            }
            let entity = match game.world.block_entities.get_mut(&(pos[0], pos[1], pos[2])) {
                Some(BlockEntity::Mod(entity)) if entity.id == CONDUIT => entity,
                _ => {
                    game.native_feedback(Text::literal("No conduit there."));
                    return;
                }
            };
            write_config(&mut entity.nbt, &config);
            game.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
            game.native_feedback(Text::literal(format!(
                "Conduit: {} {} now {}",
                FACE_NAMES[face],
                option.to_lowercase(),
                if on { "on" } else { "off" }
            )));
        }
        _ => {}
    }
}
