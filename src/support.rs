//! Game glue for Ender IO's item conduit: the block entity that saves the per-side
//! configuration, the connection state on the block's six properties, the network walk and the
//! cycle that moves items, plus the `/enderio` commands.
//!
//! Upstream a conduit bundle is a block entity whose node holds, per side, the connection
//! configuration, and whose network moves items between the inventories its connections reach.
//! This slice keeps the per-side configuration in the block entity's saved compound (as
//! upstream's node data is), draws the connection state as the block's six boolean properties,
//! and does the moving after the level's tick: every cycle, each extracting side hands up to
//! [`SPEED`] items to the network's inserting sides.
//!
//! The rules live in [`crate::runtime`] and are tested without the game; this file reads the
//! world, calls those rules and writes the result back. The SDK cannot enumerate a level's block
//! entities, so a conduit's own tick only queues it and [`on_level_tick_post`] drains the queue
//! for that level — the same shape `laserio`, `integratedtunnels` and `naturesaura` use.

use std::collections::BTreeSet;

use mod_api::{
    BlockEntityPart, BlockEntityRef, BlockEntityTickEvent, BlockEntityTickReply,
    BlockEntityTicksEvent, BlockKey, BlockPart, BlockUpdateFlags, Capability, Command,
    CommandArgumentValue, Compound, Direction, Flow, Host, HostTransaction, ITEMS, ItemResource,
    LevelRef, ResourceHandler, RunModCommandEvent, ServerHost, Tag, Text, move_resources,
};

use crate::runtime::{
    CYCLE_TICKS, ConduitConfig, FACE_COUNT, FACE_NAMES, Face, Neighbour, Port, RedstoneControl,
    SPEED, Saved, connection_bits, face_named, face_normal, insert_order, is_connected,
    is_cycle_tick, ports,
};

/// The conduit bundle block (and its item).
pub(crate) const CONDUIT: &str = "enderio:conduit";

/// The readouts 0.1.0 printed, byte for byte: the existing e2e assertions match them.
pub(crate) const USAGE_INFO: &str = "Usage: /enderio info <x> <y> <z>";
pub(crate) const USAGE_CONFIG: &str =
    "Usage: /enderio config <x> <y> <z> <side> <insert|extract|round|redstone> <on|off|mode>";
pub(crate) const NO_CONDUIT: &str = "No conduit there.";

type Pos = [i32; 3];

// A compound's key type is UTF-16 and the API does not export it, so a key is taken from
// `Tag::String`'s own payload: the same conversion a mod's NBT string uses, with no name needed.

fn key_tag(name: &str) -> Tag {
    Tag::String(name.into())
}

fn tag<'a>(nbt: &'a Compound, name: &str) -> Option<&'a Tag> {
    let Tag::String(utf16) = key_tag(name) else {
        return None;
    };
    nbt.get(&utf16)
}

fn put(nbt: &mut Compound, name: &str, value: Tag) {
    if let Tag::String(utf16) = key_tag(name) {
        nbt.insert(utf16, value);
    }
}

fn int(nbt: &Compound, name: &str) -> Option<i32> {
    match tag(nbt, name) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    }
}

/// The extract mask of a fresh conduit: every side, upstream's
/// `ItemConduitConnectionConfig.DEFAULT`.
const DEFAULT_EXTRACT: u8 = (1 << FACE_COUNT) - 1;

/// A conduit's saved configuration; a fresh conduit has upstream's defaults (every side's
/// extract toggle on, insert nowhere, no round robin, extraction gated by `never_active`),
/// which is also what an empty compound reads as.
fn read_config(nbt: &Compound) -> ConduitConfig {
    ConduitConfig::from_saved(Saved {
        insert: int(nbt, "insert").unwrap_or(0),
        extract: int(nbt, "extract").unwrap_or(i32::from(DEFAULT_EXTRACT)),
        round_robin: int(nbt, "roundRobin").unwrap_or(0) != 0,
        extract_control: extract_controls(nbt),
    })
}

/// The six per-side redstone-control ids, in face order. Absent (a 0.2.1 world), the wrong
/// length, or another tag type reads as all `never_active`, the fresh conduit's default.
fn extract_controls(nbt: &Compound) -> [i32; FACE_COUNT] {
    match tag(nbt, "extractControl") {
        Some(Tag::IntArray(ids)) if ids.len() == FACE_COUNT => {
            std::array::from_fn(|face| ids[face])
        }
        _ => [RedstoneControl::NeverActive.saved(); FACE_COUNT],
    }
}

fn write_config(nbt: &mut Compound, config: &ConduitConfig) {
    let saved = config.saved();
    put(nbt, "insert", Tag::Int(saved.insert));
    put(nbt, "extract", Tag::Int(saved.extract));
    put(nbt, "roundRobin", Tag::Int(i32::from(saved.round_robin)));
    put(
        nbt,
        "extractControl",
        Tag::IntArray(saved.extract_control.to_vec()),
    );
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

/// `/enderio info`'s first line, the connected sides as the block's properties show them.
fn links_line(connected: &str) -> String {
    format!("Conduit: links {connected}")
}

/// `/enderio info`'s second line, the saved configuration.
fn config_line(extract: &str, insert: &str, round_robin: bool) -> String {
    format!(
        "Conduit: config extract {extract}; insert {insert}; round robin {}",
        if round_robin { "on" } else { "off" }
    )
}

/// `/enderio info`'s redstone line: the sides whose extraction control is not the fresh
/// default, or one line saying every side is still `never_active`.
fn redstone_line(config: &ConduitConfig) -> String {
    let modes: Vec<String> = (0..FACE_COUNT)
        .filter(|&face| config.extract_control(face) != RedstoneControl::NeverActive)
        .map(|face| {
            format!(
                "{}={}",
                FACE_NAMES[face],
                config.extract_control(face).name()
            )
        })
        .collect();
    if modes.is_empty() {
        "Conduit: redstone never_active on every side".to_string()
    } else {
        format!("Conduit: redstone {}", modes.join(","))
    }
}

/// One `/enderio config` write, parsed before the conduit is touched so a typo cannot leave a
/// half-applied change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Setting {
    Insert(bool),
    Extract(bool),
    Round(bool),
    Redstone(RedstoneControl),
}

impl Setting {
    /// The configuration with this setting applied to `face`.
    fn apply(self, config: ConduitConfig, face: Face) -> ConduitConfig {
        match self {
            Setting::Insert(on) => config.with_insert(face, on),
            Setting::Extract(on) => config.with_extract(face, on),
            Setting::Round(on) => config.with_round_robin(on),
            Setting::Redstone(control) => config.with_extract_control(face, control),
        }
    }

    /// The value as the confirmation prints it: `on`/`off`, or the mode's upstream name.
    fn value_text(self) -> &'static str {
        match self {
            Setting::Insert(on) | Setting::Extract(on) | Setting::Round(on) => {
                if on {
                    "on"
                } else {
                    "off"
                }
            }
            Setting::Redstone(control) => control.name(),
        }
    }
}

/// `/enderio config`'s confirmation.
fn set_line(face: Face, option: &str, value: &str) -> String {
    format!("Conduit: {} {option} now {value}", FACE_NAMES[face])
}

/// The position one face reaches.
fn beside(pos: Pos, face: Face) -> Pos {
    let [dx, dy, dz] = face_normal(face);
    [pos[0] + dx, pos[1] + dy, pos[2] + dz]
}

/// The level the server is ticking now: the one a block entity's tick runs in.
fn in_place_level(host: &dyn ServerHost) -> Option<LevelRef> {
    host.levels()
        .list_levels()
        .into_iter()
        .find(|&level| level.in_place(host))
}

/// Conduits whose block entity ticked this tick, per level, drained at the end of that level's
/// tick (the SDK cannot enumerate a level's block entities).
#[derive(Default)]
pub(crate) struct Queue {
    pending: std::collections::HashMap<LevelRef, BTreeSet<Pos>>,
}

impl Queue {
    fn push(&mut self, level: LevelRef, pos: Pos) {
        self.pending.entry(level).or_default().insert(pos);
    }

    /// Everything queued for `level`, in position order (empty afterwards).
    pub(crate) fn take(&mut self, level: LevelRef) -> BTreeSet<Pos> {
        self.pending.remove(&level).unwrap_or_default()
    }
}

/// The part this mod attaches to the conduit block entity type: it makes a placed conduit tick,
/// and its tick only queues the conduit for the level tick's end.
pub(crate) struct ConduitPart {
    queue: std::sync::Arc<std::sync::Mutex<Queue>>,
}

impl ConduitPart {
    pub(crate) fn new(queue: std::sync::Arc<std::sync::Mutex<Queue>>) -> ConduitPart {
        ConduitPart { queue }
    }
}

#[mod_api::horizon_part]
impl BlockPart for ConduitPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for ConduitPart {
    /// A conduit's block entity ticks on the server.
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One conduit tick: queue it for the level tick's end. A tick that reads the world and
    /// writes only block properties changes none of its own saved data, so it answers
    /// [`BlockEntityTickReply::Unchanged`] and does not dirty the chunk every tick.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = in_place_level(host) else {
            return;
        };
        // Only this expression runs under the lock: one insert on the port's own state, which
        // cannot panic and so cannot poison it.
        self.queue
            .lock()
            .expect("the conduit queue lock is held for one insert")
            .push(level, event.pos);
        *event.reply = BlockEntityTickReply::Unchanged;
    }
}

/// The conduit block entity at `pos`, if the block there is a conduit.
fn conduit_at(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: Pos,
    conduit: BlockKey,
) -> Option<BlockEntityRef> {
    (level.block(host, pos) == Some(conduit))
        .then(|| host.block_entity(level, pos))
        .flatten()
}

/// The configuration the conduit at `pos` saves; the defaults where it has none.
fn config_at(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: Pos,
    conduit: BlockKey,
) -> Option<ConduitConfig> {
    conduit_at(host, level, pos, conduit)
        .and_then(|entity| entity.data(host))
        .map(|data| read_config(&data))
}

/// The side of the block beside a conduit that faces it: the face opposite the conduit's own (a
/// conduit above a chest takes and puts items through the chest's upward side).
fn side_towards_conduit(face: Face) -> Direction {
    Direction::ALL[face].opposite()
}

/// The item handler of the block beside `face`, if that block has one reachable from the
/// touching side (the pin's `ITEMS` capability, NeoForge's `Capabilities.Item.BLOCK`).
fn inventory(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: Pos,
    face: Face,
) -> Option<Capability<dyn ResourceHandler<ItemResource>>> {
    // The neighbour is reached through the side facing the conduit.
    let side = side_towards_conduit(face);
    let handler = level.capability(host, &ITEMS, pos, Some(side))?;
    (handler.size(host) > 0).then_some(handler)
}

/// What the block beside face `face` of the conduit at `pos` is.
fn neighbour(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: Pos,
    face: Face,
    conduit: BlockKey,
) -> Neighbour {
    let next = beside(pos, face);
    if level.block(host, next) == Some(conduit) {
        return Neighbour::Conduit;
    }
    match inventory(host, level, next, face) {
        Some(_) => Neighbour::Inventory,
        None => Neighbour::Nothing,
    }
}

fn neighbours(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: Pos,
    conduit: BlockKey,
) -> [Neighbour; FACE_COUNT] {
    std::array::from_fn(|face| neighbour(host, level, pos, face, conduit))
}

/// Give the conduit at `pos` the connection state `bits`, writing only the faces that changed.
/// The state is the block's six boolean properties, so the drawn arms follow it.
fn show_connections(host: &mut dyn ServerHost, level: LevelRef, pos: Pos, bits: u8) {
    for (face, name) in FACE_NAMES.iter().enumerate() {
        let want = if is_connected(bits, face) {
            "true"
        } else {
            "false"
        };
        if level.block_property(host, pos, name).as_deref() == Some(want) {
            continue;
        }
        // `UPDATE_CLIENTS`, as a crop's growth: the clients get the write and neighbours'
        // shapes update, without neighbour updates of their own.
        host.set_block_property_with(pos, name, want, BlockUpdateFlags::CLIENTS.bits());
    }
}

/// The network a conduit belongs to: every conduit reachable through other conduits.
fn network_of(
    host: &mut dyn ServerHost,
    level: LevelRef,
    start: Pos,
    conduit: BlockKey,
) -> Vec<Pos> {
    let mut seen: BTreeSet<Pos> = BTreeSet::new();
    let mut queue = vec![start];
    seen.insert(start);
    while let Some(pos) = queue.pop() {
        for face in 0..FACE_COUNT {
            let next = beside(pos, face);
            if seen.contains(&next)
                || neighbour(host, level, pos, face, conduit) != Neighbour::Conduit
            {
                continue;
            }
            seen.insert(next);
            queue.push(next);
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

/// The inventories the network's conduits reach, in conduit then face order. Each side's
/// extraction is gated here by its redstone control and the conduit's own signal (upstream
/// `ConduitBundleBlockEntity.hasRedstoneSignal` = `level.hasNeighborSignal(pos)`), so the
/// cycle below only sees the sides that may extract now.
fn endpoints(
    host: &mut dyn ServerHost,
    level: LevelRef,
    conduits: &[Pos],
    conduit: BlockKey,
) -> Vec<Endpoint> {
    let mut found = Vec::new();
    for &pos in conduits {
        let Some(config) = config_at(host, level, pos, conduit) else {
            continue;
        };
        let bits = connection_bits(neighbours(host, level, pos, conduit));
        let has_signal = level.has_neighbor_signal(host, pos);
        for mut port in ports(bits, &config) {
            if neighbour(host, level, pos, port.face, conduit) != Neighbour::Inventory {
                continue;
            }
            port.extract = config.can_extract(port.face, has_signal);
            found.push(Endpoint {
                conduit: pos,
                face: port.face,
                port,
            });
        }
    }
    found
}

/// A network cycle's phase, from the lowest conduit's position: it spreads a level's networks
/// over the cycle as upstream's `tickOffset` does.
fn phase_of(conduits: &[Pos]) -> u64 {
    let Some([x, y, z]) = conduits.first().copied() else {
        return 0;
    };
    (i64::from(x) + i64::from(y) + i64::from(z)).rem_euclid(CYCLE_TICKS as i64) as u64
}

/// Move items in one network for one cycle: each extracting side hands up to [`SPEED`] items to
/// the inserting sides, in the order its configuration gives. An inventory that has room takes
/// the items; the next one gets what is left, as upstream walks its insert paths.
fn tick_network(
    host: &mut dyn ServerHost,
    level: LevelRef,
    conduits: &[Pos],
    cycle: u64,
    conduit: BlockKey,
) {
    let ends = endpoints(host, level, conduits, conduit);
    for source in &ends {
        if !source.port.extract {
            continue;
        }
        let insert: Vec<Endpoint> = ends
            .iter()
            .copied()
            // Self-feed stays off, as upstream's default: a conduit does not hand items back to
            // the inventory it took them from.
            .filter(|endpoint| {
                endpoint.port.insert
                    && !(endpoint.conduit == source.conduit && endpoint.face == source.face)
            })
            .collect();
        if insert.is_empty() {
            continue;
        }
        let ports: Vec<Port> = insert.iter().map(|endpoint| endpoint.port).collect();
        let round_robin = config_at(host, level, source.conduit, conduit)
            .is_some_and(|config| config.round_robin());
        let (from_pos, from_face) = (beside(source.conduit, source.face), source.face);
        let Some(from) = inventory(host, level, from_pos, from_face) else {
            continue;
        };
        let mut budget = SPEED;
        for index in insert_order(&ports, cycle, round_robin) {
            if budget == 0 {
                break;
            }
            let (to_pos, to_face) = (
                beside(insert[index].conduit, insert[index].face),
                insert[index].face,
            );
            let Some(to) = inventory(host, level, to_pos, to_face) else {
                continue;
            };
            // The capability's own move, in a transaction: an item the target refuses stays in
            // its slot and the next destination gets what is left of the cycle's budget.
            let went = host
                .transaction(|host, tx| move_resources(host, &from, &to, |_| true, budget, tx))
                .unwrap_or(0);
            budget -= went.min(budget);
        }
    }
}

/// After the level's tick: refresh every queued conduit's drawn connections and, on each
/// network's cycle tick, move items.
pub(crate) fn on_level_tick_post(
    host: &mut dyn ServerHost,
    level: LevelRef,
    queued: BTreeSet<Pos>,
) {
    if queued.is_empty() {
        return;
    }
    let Some(conduit) = host.block_key(CONDUIT) else {
        return;
    };
    let Some(game_time) = level.game_time(host) else {
        return;
    };
    let game_time = game_time.max(0) as u64;
    let cycle = game_time / CYCLE_TICKS;
    let mut done: BTreeSet<Pos> = BTreeSet::new();
    for pos in queued {
        if done.contains(&pos) || level.block(host, pos) != Some(conduit) {
            continue;
        }
        let network = network_of(host, level, pos, conduit);
        for &member in &network {
            let bits = connection_bits(neighbours(host, level, member, conduit));
            show_connections(host, level, member, bits);
        }
        if is_cycle_tick(game_time, phase_of(&network)) {
            tick_network(host, level, &network, cycle, conduit);
        }
        done.extend(network);
    }
}

fn feedback(host: &mut dyn ServerHost, line: &str) {
    let _ = host.commands().feedback(Text::literal(line));
}

/// Declare `/enderio config` and `/enderio info`, both operator-only (level 2).
///
/// `config` writes a conduit's per-side extraction and insertion, and `info` reads the same
/// configuration, at coordinates the caller chooses: the review of 2026-11-18 found that a
/// level-0 registration let any player redirect another player's item transfers from any
/// distance. Upstream's own controls are its conduit screen and probe, which are not ported,
/// so both commands are operator diagnostics.
pub(crate) fn declare_commands(commands: &mut Vec<Command>) {
    commands.push(Command::op(
        "enderio config <pos:block_pos> <side:word> <option:word> <value:word>",
        2,
    ));
    commands.push(Command::op("enderio info <pos:block_pos>", 2));
}

/// The level a command runs in, after `execute in`.
fn command_level(host: &dyn ServerHost, event: &RunModCommandEvent<'_>) -> Option<LevelRef> {
    host.levels()
        .list_levels()
        .into_iter()
        .find(|level| level.dimension() == event.command.dimension)
}

/// Run the `/enderio` commands a player types.
pub(crate) fn run_command(host: &mut dyn ServerHost, event: &mut RunModCommandEvent<'_>) -> Flow {
    let invocation = event.command;
    let words: Vec<&str> = invocation.literals.iter().map(String::as_str).collect();
    if words.first() != Some(&"enderio") {
        return Flow::Continue;
    }
    let Some(action) = words.get(1).copied() else {
        return Flow::Continue;
    };
    if !matches!(action, "info" | "config") {
        return Flow::Continue;
    }
    let usage = if action == "info" {
        USAGE_INFO
    } else {
        USAGE_CONFIG
    };
    let Some(CommandArgumentValue::BlockPos(pos)) = invocation.argument("pos") else {
        feedback(host, usage);
        return Flow::Cancel;
    };
    let pos = *pos;
    let Some(level) = command_level(host, event) else {
        feedback(host, NO_CONDUIT);
        return Flow::Cancel;
    };
    let Some(conduit) = host.block_key(CONDUIT) else {
        feedback(host, NO_CONDUIT);
        return Flow::Cancel;
    };
    if action == "info" {
        let Some(entity) = conduit_at(host, level, pos, conduit) else {
            feedback(host, NO_CONDUIT);
            return Flow::Cancel;
        };
        let Some(data) = entity.data(host) else {
            feedback(host, NO_CONDUIT);
            return Flow::Cancel;
        };
        let config = read_config(&data);
        let connected = face_list(|face| {
            level.block_property(host, pos, FACE_NAMES[face]).as_deref() == Some("true")
        });
        feedback(host, &links_line(&connected));
        feedback(
            host,
            &config_line(
                &face_list(|face| config.extract(face)),
                &face_list(|face| config.insert(face)),
                config.round_robin(),
            ),
        );
        feedback(host, &redstone_line(&config));
        return Flow::Cancel;
    }
    let Some(CommandArgumentValue::String(side)) = invocation.argument("side") else {
        feedback(host, USAGE_CONFIG);
        return Flow::Cancel;
    };
    let Some(CommandArgumentValue::String(option)) = invocation.argument("option") else {
        feedback(host, USAGE_CONFIG);
        return Flow::Cancel;
    };
    let Some(CommandArgumentValue::String(value)) = invocation.argument("value") else {
        feedback(host, USAGE_CONFIG);
        return Flow::Cancel;
    };
    let Some(face) = face_named(&side.to_lowercase()) else {
        feedback(
            host,
            &format!("Unknown side '{side}': down, up, north, south, west or east."),
        );
        return Flow::Cancel;
    };
    let option = option.to_lowercase();
    let setting = match option.as_str() {
        "insert" | "extract" | "round" | "round_robin" => {
            let Some(on) = on_off(value) else {
                feedback(host, &format!("'{value}' is not on or off."));
                return Flow::Cancel;
            };
            match option.as_str() {
                "insert" => Setting::Insert(on),
                "extract" => Setting::Extract(on),
                _ => Setting::Round(on),
            }
        }
        "redstone" => {
            let Some(control) = RedstoneControl::named(value) else {
                feedback(
                    host,
                    &format!(
                        "Unknown redstone control '{value}': always_active, active_with_signal, \
                         active_without_signal or never_active."
                    ),
                );
                return Flow::Cancel;
            };
            Setting::Redstone(control)
        }
        other => {
            feedback(
                host,
                &format!("Unknown option '{other}': insert, extract, round or redstone."),
            );
            return Flow::Cancel;
        }
    };
    let Some(entity) = conduit_at(host, level, pos, conduit) else {
        feedback(host, NO_CONDUIT);
        return Flow::Cancel;
    };
    let Some(mut data) = entity.data(host) else {
        feedback(host, NO_CONDUIT);
        return Flow::Cancel;
    };
    let config = setting.apply(read_config(&data), face);
    write_config(&mut data, &config);
    // The saved compound is the config, so the write goes back through the entity's own load
    // path: the chunk is marked changed and clients get the block update.
    if entity.replace_data(host, data).is_err() {
        feedback(host, NO_CONDUIT);
        return Flow::Cancel;
    }
    feedback(host, &set_line(face, &option, setting.value_text()));
    Flow::Cancel
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{DOWN, EAST, NORTH, UP, WEST};

    /// The review finding (2026-11-18): `config` writes a conduit's per-side extraction and
    /// insertion at coordinates the caller chooses, and `info` reads the same configuration of
    /// another player's conduit, so a level-0 registration let any player do either. Both are
    /// operator-only; this test fails on a `Command::new` registration (level 0).
    #[test]
    fn the_conduit_commands_are_operator_only() {
        let mut commands = Vec::new();
        declare_commands(&mut commands);
        assert_eq!(commands.len(), 2, "config and info");
        for command in &commands {
            assert!(
                command.usage.starts_with("enderio "),
                "{} is this mod's",
                command.usage
            );
            assert_eq!(
                command.level, 2,
                "{} acts on a conduit at chosen coordinates",
                command.usage
            );
        }
    }

    #[test]
    fn a_fresh_compound_reads_upstreams_default_configuration() {
        let config = read_config(&Compound::new());
        for face in 0..FACE_COUNT {
            assert!(config.extract(face), "face {face} extracts by default");
            assert!(!config.insert(face), "face {face} inserts only when set");
        }
        assert!(!config.round_robin());
    }

    #[test]
    fn the_config_round_trips_through_the_saved_compound() {
        let config = ConduitConfig::new()
            .with_insert(NORTH, true)
            .with_insert(EAST, true)
            .with_extract(DOWN, false)
            .with_round_robin(true)
            .with_extract_control(UP, RedstoneControl::ActiveWithSignal)
            .with_extract_control(WEST, RedstoneControl::AlwaysActive);
        let mut nbt = Compound::new();
        write_config(&mut nbt, &config);
        assert_eq!(read_config(&nbt), config);
        // The keys are the ones 0.1.0 wrote, so an existing world's conduit keeps its settings.
        assert_eq!(int(&nbt, "insert"), Some(0b10_0100));
        assert_eq!(int(&nbt, "extract"), Some(0b11_1110));
        assert_eq!(int(&nbt, "roundRobin"), Some(1));
        // The new key is one id per side, in face order (down, up, north, south, west, east).
        assert_eq!(
            tag(&nbt, "extractControl"),
            Some(&Tag::IntArray(vec![3, 1, 3, 3, 0, 3]))
        );
    }

    #[test]
    fn a_compound_without_a_usable_redstone_array_reads_never_active() {
        // A 0.2.1 world has no key at all; a hand-edited or damaged one may have the wrong
        // shape or an id outside 0..=3. Every such side behaves as a fresh conduit.
        let fresh = ConduitConfig::new();
        assert_eq!(read_config(&Compound::new()), fresh);

        let mut short = Compound::new();
        put(&mut short, "extractControl", Tag::IntArray(vec![0, 0]));
        assert_eq!(read_config(&short), fresh);

        let mut damaged = Compound::new();
        put(
            &mut damaged,
            "extractControl",
            Tag::IntArray(vec![0, 9, -1, 3, 3, 3]),
        );
        let config = read_config(&damaged);
        assert_eq!(config.extract_control(DOWN), RedstoneControl::AlwaysActive);
        assert_eq!(config.extract_control(UP), RedstoneControl::NeverActive);
        assert_eq!(config.extract_control(NORTH), RedstoneControl::NeverActive);
    }

    #[test]
    fn the_info_readout_names_the_sides_whose_redstone_control_is_not_the_default() {
        assert_eq!(
            redstone_line(&ConduitConfig::new()),
            "Conduit: redstone never_active on every side"
        );
        let config = ConduitConfig::new()
            .with_extract_control(WEST, RedstoneControl::AlwaysActive)
            .with_extract_control(EAST, RedstoneControl::ActiveWithSignal);
        assert_eq!(
            redstone_line(&config),
            "Conduit: redstone west=always_active,east=active_with_signal"
        );
    }

    #[test]
    fn a_damaged_saved_config_falls_back_to_the_defaults() {
        let mut nbt = Compound::new();
        put(&mut nbt, "insert", Tag::Int(-1));
        put(&mut nbt, "extract", Tag::Int(0x7fff));
        assert_eq!(
            read_config(&nbt),
            ConduitConfig::new().with_round_robin(false)
        );
    }

    #[test]
    fn the_info_readouts_are_the_zero_one_zero_text() {
        assert_eq!(links_line("west,east"), "Conduit: links west,east");
        assert_eq!(links_line("none"), "Conduit: links none");
        assert_eq!(
            config_line("down,up,east", "none", false),
            "Conduit: config extract down,up,east; insert none; round robin off"
        );
        assert_eq!(
            config_line("none", "north", true),
            "Conduit: config extract none; insert north; round robin on"
        );
        assert_eq!(USAGE_INFO, "Usage: /enderio info <x> <y> <z>");
        assert_eq!(
            USAGE_CONFIG,
            "Usage: /enderio config <x> <y> <z> <side> <insert|extract|round|redstone> <on|off|mode>"
        );
        assert_eq!(NO_CONDUIT, "No conduit there.");
    }

    #[test]
    fn the_config_readout_names_some_faces_or_none() {
        let config = ConduitConfig::new()
            .with_insert(EAST, true)
            .with_extract(DOWN, false)
            .with_extract(WEST, false);
        assert_eq!(face_list(|face| config.insert(face)), "east");
        assert_eq!(
            face_list(|face| config.extract(face)),
            "up,north,south,east"
        );
        assert_eq!(
            face_list(|face| config.insert(face) && face == DOWN),
            "none"
        );
    }

    #[test]
    fn the_config_confirmation_names_the_side_and_the_state() {
        assert_eq!(
            set_line(EAST, "insert", "on"),
            "Conduit: east insert now on"
        );
        assert_eq!(set_line(UP, "round", "off"), "Conduit: up round now off");
        assert_eq!(
            set_line(WEST, "redstone", "always_active"),
            "Conduit: west redstone now always_active"
        );
    }

    #[test]
    fn on_and_off_parse_in_any_case() {
        assert_eq!(on_off("on"), Some(true));
        assert_eq!(on_off("TRUE"), Some(true));
        assert_eq!(on_off("Off"), Some(false));
        assert_eq!(on_off("false"), Some(false));
        assert_eq!(on_off("maybe"), None);
        assert_eq!(on_off(""), None);
    }

    #[test]
    fn side_names_are_the_block_properties() {
        assert_eq!(face_named("down"), Some(DOWN));
        assert_eq!(face_named("east"), Some(EAST));
        assert_eq!(face_named("sideways"), None);
    }

    #[test]
    fn a_face_reaches_the_position_beside_it() {
        let pos = [4, 64, -7];
        assert_eq!(beside(pos, DOWN), [4, 63, -7]);
        assert_eq!(beside(pos, UP), [4, 65, -7]);
        assert_eq!(beside(pos, NORTH), [4, 64, -8]);
        assert_eq!(beside(pos, EAST), [5, 64, -7]);
        // The inventory is reached through the side of it that faces the conduit.
        assert_eq!(side_towards_conduit(EAST), Direction::ALL[EAST].opposite());
        assert_eq!(side_towards_conduit(DOWN), Direction::ALL[UP]);
    }
}
