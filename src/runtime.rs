// Pure item-conduit logic for Ender IO (standalone crate lib, injected as
// `crate::game::enderio`). No game access lives here.
//
// Upstream's item conduit (`ItemConduit`) moves `transfer_rate` items per
// network cycle, a cycle being `ticks_per_cycle` game ticks: the defaults are
// 4 items every 20 ticks. Its connections are per side, and so is the
// configuration: `ItemConduitConnectionConfig.DEFAULT` extracts from every
// connected side, inserts nowhere until the player enables insertion on a
// side in the conduit's screen, and does not round-robin between insert
// sides. A side with round robin on starts at the next insert side each
// cycle, so one source spreads its items over several destinations.

/// One of a conduit's six sides, in vanilla's `Direction.values()` order.
pub type Face = usize;

pub const FACE_COUNT: usize = 6;

pub const DOWN: Face = 0;
pub const UP: Face = 1;
pub const NORTH: Face = 2;
pub const SOUTH: Face = 3;
pub const WEST: Face = 4;
pub const EAST: Face = 5;

/// `Direction.values()`'s names, which are also the block's state properties.
pub const FACE_NAMES: [&str; FACE_COUNT] = ["down", "up", "north", "south", "west", "east"];

/// Upstream `ItemConduit`'s `transfer_rate` default: items per cycle.
pub const SPEED: u32 = 4;

/// Upstream `ItemConduit`'s `ticks_per_cycle` default: game ticks per cycle.
pub const CYCLE_TICKS: u64 = 20;

/// Every face's connection bit, for [`connection_bits`]' `u8`.
const ALL_FACES: u8 = (1 << FACE_COUNT) - 1;

/// The face's unit vector, as `Direction.normal()` gives it.
pub const fn face_normal(face: Face) -> [i32; 3] {
    match face {
        DOWN => [0, -1, 0],
        UP => [0, 1, 0],
        NORTH => [0, 0, -1],
        SOUTH => [0, 0, 1],
        WEST => [-1, 0, 0],
        EAST => [1, 0, 0],
        _ => [0, 0, 0],
    }
}

/// The face a state property names (`down`..`east`), if it names one.
pub fn face_named(name: &str) -> Option<Face> {
    FACE_NAMES.iter().position(|known| *known == name)
}

/// What a face of a conduit reaches, read from the block beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Neighbour {
    /// Air, or a block with no items.
    Nothing,
    /// Another conduit of the same type, which the network continues through.
    Conduit,
    /// A block with an item container reachable from the touching side.
    Inventory,
}

/// The drawn connection state: a face connects to another conduit or to a
/// container, as upstream's `canConnectToConduit`/`canConnectToBlock` do.
pub fn connection_bits(neighbours: [Neighbour; FACE_COUNT]) -> u8 {
    neighbours
        .iter()
        .enumerate()
        .filter(|(_, neighbour)| matches!(neighbour, Neighbour::Conduit | Neighbour::Inventory))
        .fold(0, |bits, (face, _)| bits | (1 << face))
}

/// Whether `face` is connected in a state from [`connection_bits`].
pub fn is_connected(bits: u8, face: Face) -> bool {
    face < FACE_COUNT && bits & (1 << face) != 0
}

/// A conduit's per-side configuration (upstream
/// `ItemConduitConnectionConfig`): which connected sides extract, which
/// insert, and whether the extract sides round-robin their destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// One bit per face that may insert.
    insert: u8,
    /// One bit per face that may extract.
    extract: u8,
    round_robin: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config::new()
    }
}

impl Config {
    /// Upstream's default: extract from every side, insert nowhere, no round
    /// robin.
    pub const fn new() -> Self {
        Config {
            insert: 0,
            extract: ALL_FACES,
            round_robin: false,
        }
    }

    pub fn insert(&self, face: Face) -> bool {
        face < FACE_COUNT && self.insert & (1 << face) != 0
    }

    pub fn extract(&self, face: Face) -> bool {
        face < FACE_COUNT && self.extract & (1 << face) != 0
    }

    pub fn round_robin(&self) -> bool {
        self.round_robin
    }

    pub fn with_insert(self, face: Face, on: bool) -> Self {
        Config {
            insert: set(self.insert, face, on),
            ..self
        }
    }

    pub fn with_extract(self, face: Face, on: bool) -> Self {
        Config {
            extract: set(self.extract, face, on),
            ..self
        }
    }

    pub fn with_round_robin(self, on: bool) -> Self {
        Config {
            round_robin: on,
            ..self
        }
    }

    /// The two side masks and the round-robin flag, the shape the block
    /// entity saves (upstream keeps them in its node data).
    pub fn saved(&self) -> (i32, i32, bool) {
        (
            i32::from(self.insert),
            i32::from(self.extract),
            self.round_robin,
        )
    }

    /// Restore the masks [`Config::saved`] wrote; bits past the six sides are
    /// dropped, and a value that is no mask at all falls back to that mask's
    /// default, so a damaged save behaves as a fresh conduit.
    pub fn from_saved(insert: i32, extract: i32, round_robin: bool) -> Self {
        let mask = |value: i32, default: u8| {
            u8::try_from(value)
                .map(|value| value & ALL_FACES)
                .unwrap_or(default)
        };
        Config {
            insert: mask(insert, 0),
            extract: mask(extract, ALL_FACES),
            round_robin,
        }
    }
}

/// `mask` with `face`'s bit set or cleared.
const fn set(mask: u8, face: Face, on: bool) -> u8 {
    if face >= FACE_COUNT {
        return mask;
    }
    if on {
        mask | (1 << face)
    } else {
        mask & !(1 << face)
    }
}

/// One connected side of one conduit, with what its configuration allows
/// there. The glue pairs it with the conduit's position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Port {
    pub face: Face,
    pub insert: bool,
    pub extract: bool,
}

/// The connected sides of a conduit, in face order, with their flags.
pub fn ports(bits: u8, config: &Config) -> Vec<Port> {
    (0..FACE_COUNT)
        .filter(|&face| is_connected(bits, face))
        .map(|face| Port {
            face,
            insert: config.insert(face),
            extract: config.extract(face),
        })
        .collect()
}

/// The order an extract side tries the network's insert sides in, as indexes
/// into `ports`. Without round robin it is always the list order (upstream's
/// default); with it the list starts at `cycle % len`, so one more destination
/// is tried first each cycle. Ports that may not insert are left out.
pub fn insert_order(ports: &[Port], cycle: u64, round_robin: bool) -> Vec<usize> {
    let mut order: Vec<usize> = ports
        .iter()
        .enumerate()
        .filter(|(_, port)| port.insert)
        .map(|(index, _)| index)
        .collect();
    if round_robin && !order.is_empty() {
        let start = (cycle % order.len() as u64) as usize;
        order.rotate_left(start);
    }
    order
}

/// Whether this tick is the network's cycle tick. `phase` spreads a level's
/// networks over the cycle (upstream's `tickOffset`), so they do not all move
/// items on the same tick.
pub fn is_cycle_tick(game_time: u64, phase: u64) -> bool {
    game_time.wrapping_add(phase).is_multiple_of(CYCLE_TICKS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_follow_vanillas_direction_order() {
        assert_eq!(FACE_NAMES, ["down", "up", "north", "south", "west", "east"]);
        for (index, name) in FACE_NAMES.iter().enumerate() {
            assert_eq!(face_named(name), Some(index), "{name}");
        }
        assert_eq!(face_named("sideways"), None);
        assert_eq!(face_normal(NORTH), [0, 0, -1]);
        assert_eq!(face_normal(EAST), [1, 0, 0]);
        assert_eq!(face_normal(SOUTH), [0, 0, 1]);
    }

    #[test]
    fn the_default_config_extracts_from_every_side_and_inserts_nowhere() {
        let config = Config::new();
        for face in 0..FACE_COUNT {
            assert!(config.extract(face), "face {face} extracts by default");
            assert!(!config.insert(face), "face {face} inserts only when set");
        }
        assert!(!config.round_robin());
    }

    #[test]
    fn config_toggles_one_side_and_keeps_the_others() {
        let config = Config::new()
            .with_insert(NORTH, true)
            .with_extract(SOUTH, false)
            .with_round_robin(true);
        assert!(config.insert(NORTH));
        assert!(!config.extract(SOUTH));
        assert!(config.round_robin());
        assert!(!config.insert(UP));
        assert!(config.extract(UP));
    }

    #[test]
    fn config_round_trips_through_its_saved_masks() {
        let config = Config::new()
            .with_insert(NORTH, true)
            .with_insert(EAST, true)
            .with_extract(DOWN, false)
            .with_round_robin(true);
        let (insert, extract, round_robin) = config.saved();
        assert_eq!(Config::from_saved(insert, extract, round_robin), config);
        assert_eq!(Config::from_saved(0, 0b11_1111, false), Config::new());
        assert_eq!(Config::from_saved(-1, -1, false), Config::new());
    }

    #[test]
    fn a_face_connects_to_a_conduit_or_an_inventory_only() {
        let neighbours = [
            Neighbour::Inventory,
            Neighbour::Conduit,
            Neighbour::Nothing,
            Neighbour::Inventory,
            Neighbour::Nothing,
            Neighbour::Nothing,
        ];
        let bits = connection_bits(neighbours);
        assert!(is_connected(bits, DOWN));
        assert!(is_connected(bits, UP));
        assert!(!is_connected(bits, NORTH));
        assert!(is_connected(bits, SOUTH));
        assert!(!is_connected(bits, WEST));
        assert!(!is_connected(bits, EAST));
        assert_eq!(connection_bits([Neighbour::Nothing; FACE_COUNT]), 0);
    }

    #[test]
    fn ports_are_the_connected_sides_with_their_flags() {
        let neighbours = [
            Neighbour::Inventory,
            Neighbour::Inventory,
            Neighbour::Nothing,
            Neighbour::Nothing,
            Neighbour::Nothing,
            Neighbour::Nothing,
        ];
        let bits = connection_bits(neighbours);
        let config = Config::new()
            .with_insert(UP, true)
            .with_extract(DOWN, false);
        assert_eq!(
            ports(bits, &config),
            vec![
                Port {
                    face: DOWN,
                    insert: false,
                    extract: false,
                },
                Port {
                    face: UP,
                    insert: true,
                    extract: true,
                },
            ]
        );
    }

    fn inserts(faces: &[Face]) -> Vec<Port> {
        faces
            .iter()
            .map(|&face| Port {
                face,
                insert: true,
                extract: false,
            })
            .collect()
    }

    #[test]
    fn insert_order_takes_the_first_insert_when_round_robin_is_off() {
        let ports = inserts(&[NORTH, SOUTH]);
        assert_eq!(insert_order(&ports, 0, false), vec![0, 1]);
        assert_eq!(insert_order(&ports, 7, false), vec![0, 1]);
        // A connected side that only extracts is not a destination.
        let mixed = [
            Port {
                face: NORTH,
                insert: false,
                extract: true,
            },
            Port {
                face: SOUTH,
                insert: true,
                extract: false,
            },
        ];
        assert_eq!(insert_order(&mixed, 0, false), vec![1]);
    }

    #[test]
    fn round_robin_rotates_one_insert_per_cycle() {
        let ports = inserts(&[NORTH, SOUTH, EAST]);
        assert_eq!(insert_order(&ports, 0, true), vec![0, 1, 2]);
        assert_eq!(insert_order(&ports, 1, true), vec![1, 2, 0]);
        assert_eq!(insert_order(&ports, 2, true), vec![2, 0, 1]);
        assert_eq!(insert_order(&ports, 3, true), vec![0, 1, 2]);
        assert!(insert_order(&[], 5, true).is_empty());
    }

    #[test]
    fn a_network_cycles_once_every_twenty_ticks() {
        assert!(is_cycle_tick(0, 0));
        assert!(!is_cycle_tick(1, 0));
        assert!(is_cycle_tick(20, 0));
        assert!(is_cycle_tick(19, 1));
        assert!(!is_cycle_tick(20, 1));
    }

    #[test]
    fn the_speed_is_four_items_per_twenty_tick_cycle() {
        assert_eq!(SPEED, 4, "upstream `transfer_rate` default");
        assert_eq!(CYCLE_TICKS, 20, "upstream `ticks_per_cycle` default");
    }
}
