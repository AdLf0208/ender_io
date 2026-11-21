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

/// Upstream `RedstoneControl` (`api/io/RedstoneControl`): how a side's own
/// redstone signal gates what it does. The item conduit applies it to
/// extraction (`ItemConduitConnectionConfig.canExtract`); its default is
/// [`RedstoneControl::NeverActive`], so a fresh conduit extracts nothing until
/// the player sets one of the other three on the side.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RedstoneControl {
    /// Acts whatever the signal is (upstream id 0).
    AlwaysActive,
    /// Acts only while the block beside it gives a signal (id 1).
    ActiveWithSignal,
    /// Acts only while there is no signal (id 2).
    ActiveWithoutSignal,
    /// Never acts (id 3, upstream's `ItemConduitConnectionConfig.DEFAULT`).
    #[default]
    NeverActive,
}

impl RedstoneControl {
    /// Whether a side with this control acts, given its block's signal.
    pub fn is_active(self, has_signal: bool) -> bool {
        match self {
            RedstoneControl::AlwaysActive => true,
            RedstoneControl::ActiveWithSignal => has_signal,
            RedstoneControl::ActiveWithoutSignal => !has_signal,
            RedstoneControl::NeverActive => false,
        }
    }

    /// Whether the mode reads the signal at all (upstream `isRedstoneSensitive`).
    pub fn is_sensitive(self) -> bool {
        matches!(
            self,
            RedstoneControl::ActiveWithSignal | RedstoneControl::ActiveWithoutSignal
        )
    }

    /// The mode's upstream id, as the block entity saves it.
    pub fn saved(self) -> i32 {
        match self {
            RedstoneControl::AlwaysActive => 0,
            RedstoneControl::ActiveWithSignal => 1,
            RedstoneControl::ActiveWithoutSignal => 2,
            RedstoneControl::NeverActive => 3,
        }
    }

    /// The mode an id names; a value outside 0..=3 falls back to
    /// [`RedstoneControl::NeverActive`], the fresh-conduit default (the same
    /// policy [`ConduitConfig::from_saved`] applies to a damaged mask).
    pub fn from_saved(id: i32) -> Self {
        match id {
            0 => RedstoneControl::AlwaysActive,
            1 => RedstoneControl::ActiveWithSignal,
            2 => RedstoneControl::ActiveWithoutSignal,
            _ => RedstoneControl::NeverActive,
        }
    }

    /// The upstream serialized name (`always_active`).
    pub fn name(self) -> &'static str {
        match self {
            RedstoneControl::AlwaysActive => "always_active",
            RedstoneControl::ActiveWithSignal => "active_with_signal",
            RedstoneControl::ActiveWithoutSignal => "active_without_signal",
            RedstoneControl::NeverActive => "never_active",
        }
    }

    /// The mode a name or short name sets: upstream's four serialized names,
    /// plus `always`, `with_signal`, `without_signal` and `never`.
    pub fn named(text: &str) -> Option<Self> {
        match text.to_lowercase().as_str() {
            "always_active" | "always" => Some(RedstoneControl::AlwaysActive),
            "active_with_signal" | "with_signal" => Some(RedstoneControl::ActiveWithSignal),
            "active_without_signal" | "without_signal" => {
                Some(RedstoneControl::ActiveWithoutSignal)
            }
            "never_active" | "never" => Some(RedstoneControl::NeverActive),
            _ => None,
        }
    }
}

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
/// insert, whether the extract sides round-robin their destinations, and which
/// redstone control gates each side's extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConduitConfig {
    /// One bit per face that may insert.
    insert: u8,
    /// One bit per face that may extract (the toggle; the redstone control
    /// decides whether an extracting side acts now).
    extract: u8,
    round_robin: bool,
    /// The redstone control of each face's extraction, in face order.
    extract_control: [RedstoneControl; FACE_COUNT],
}

impl Default for ConduitConfig {
    fn default() -> Self {
        ConduitConfig::new()
    }
}

impl ConduitConfig {
    /// Upstream's default: every side's extract toggle on, insert nowhere, no
    /// round robin, and extraction gated by
    /// [`RedstoneControl::NeverActive`] — so a fresh conduit moves nothing
    /// until a side's redstone control is set (`ItemConduitConnectionConfig`
    /// `canExtract`).
    pub const fn new() -> Self {
        ConduitConfig {
            insert: 0,
            extract: ALL_FACES,
            round_robin: false,
            extract_control: [RedstoneControl::NeverActive; FACE_COUNT],
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

    /// The redstone control of `face`'s extraction.
    pub fn extract_control(&self, face: Face) -> RedstoneControl {
        if face < FACE_COUNT {
            self.extract_control[face]
        } else {
            RedstoneControl::NeverActive
        }
    }

    /// Whether `face` extracts now: its toggle is on **and** its redstone
    /// control is active for the block's signal.
    pub fn can_extract(&self, face: Face, has_signal: bool) -> bool {
        self.extract(face) && self.extract_control(face).is_active(has_signal)
    }

    pub fn with_insert(self, face: Face, on: bool) -> Self {
        ConduitConfig {
            insert: set(self.insert, face, on),
            ..self
        }
    }

    pub fn with_extract(self, face: Face, on: bool) -> Self {
        ConduitConfig {
            extract: set(self.extract, face, on),
            ..self
        }
    }

    pub fn with_round_robin(self, on: bool) -> Self {
        ConduitConfig {
            round_robin: on,
            ..self
        }
    }

    /// `face`'s extraction under `control`.
    pub fn with_extract_control(mut self, face: Face, control: RedstoneControl) -> Self {
        if face < FACE_COUNT {
            self.extract_control[face] = control;
        }
        self
    }

    /// The configuration as the block entity saves it (upstream keeps each
    /// side's connection config in its node data).
    pub fn saved(&self) -> Saved {
        Saved {
            insert: i32::from(self.insert),
            extract: i32::from(self.extract),
            round_robin: self.round_robin,
            extract_control: std::array::from_fn(|face| self.extract_control[face].saved()),
        }
    }

    /// Restore [`ConduitConfig::saved`]'s shape. Bits past the six sides are dropped
    /// and an id outside 0..=3 reads as [`RedstoneControl::NeverActive`], so a
    /// damaged save behaves as a fresh conduit.
    pub fn from_saved(saved: Saved) -> Self {
        let mask = |value: i32, default: u8| {
            u8::try_from(value)
                .map(|value| value & ALL_FACES)
                .unwrap_or(default)
        };
        ConduitConfig {
            insert: mask(saved.insert, 0),
            extract: mask(saved.extract, ALL_FACES),
            round_robin: saved.round_robin,
            extract_control: std::array::from_fn(|face| {
                RedstoneControl::from_saved(saved.extract_control[face])
            }),
        }
    }
}

/// A conduit's saved configuration: the two side masks and the round-robin
/// flag of slice 1, plus one redstone-control id per side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Saved {
    /// The insert mask, one bit per face.
    pub insert: i32,
    /// The extract mask, one bit per face.
    pub extract: i32,
    pub round_robin: bool,
    /// The redstone-control ids of the six faces, in face order.
    pub extract_control: [i32; FACE_COUNT],
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

/// The connected sides of a conduit, in face order, with their flags. `extract` is the side's
/// own toggle; the ticker asks [`ConduitConfig::can_extract`] with the conduit's signal to know whether
/// it extracts now, since the redstone control lives beside the toggle.
pub fn ports(bits: u8, config: &ConduitConfig) -> Vec<Port> {
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
        let config = ConduitConfig::new();
        for face in 0..FACE_COUNT {
            assert!(config.extract(face), "face {face} extracts by default");
            assert!(!config.insert(face), "face {face} inserts only when set");
        }
        assert!(!config.round_robin());
    }

    #[test]
    fn config_toggles_one_side_and_keeps_the_others() {
        let config = ConduitConfig::new()
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
        let config = ConduitConfig::new()
            .with_insert(NORTH, true)
            .with_insert(EAST, true)
            .with_extract(DOWN, false)
            .with_round_robin(true);
        let saved = config.saved();
        assert_eq!(ConduitConfig::from_saved(saved), config);
        let never = [RedstoneControl::NeverActive.saved(); FACE_COUNT];
        assert_eq!(
            ConduitConfig::from_saved(Saved {
                insert: 0,
                extract: 0b11_1111,
                round_robin: false,
                extract_control: never,
            }),
            ConduitConfig::new()
        );
        assert_eq!(
            ConduitConfig::from_saved(Saved {
                insert: -1,
                extract: -1,
                round_robin: false,
                extract_control: [9; FACE_COUNT],
            }),
            ConduitConfig::new()
        );
    }

    #[test]
    fn a_fresh_conduit_extracts_nothing_until_its_redstone_control_is_set() {
        // Upstream's `ItemConduitConnectionConfig.DEFAULT` has the extract toggle
        // on and `RedstoneControl.NEVER_ACTIVE`, whose `canExtract` is false; the
        // gametest moves an item only after `ALWAYS_ACTIVE` is set.
        let fresh = ConduitConfig::new();
        for face in 0..FACE_COUNT {
            assert!(
                !fresh.can_extract(face, false) && !fresh.can_extract(face, true),
                "face {face} extracts on a fresh conduit"
            );
            assert!(
                fresh
                    .with_extract_control(face, RedstoneControl::AlwaysActive)
                    .can_extract(face, false),
                "face {face} does not extract under AlwaysActive"
            );
        }
    }

    #[test]
    fn a_sides_redstone_control_gates_its_extraction() {
        let always = ConduitConfig::new().with_extract_control(EAST, RedstoneControl::AlwaysActive);
        let with =
            ConduitConfig::new().with_extract_control(EAST, RedstoneControl::ActiveWithSignal);
        let without =
            ConduitConfig::new().with_extract_control(EAST, RedstoneControl::ActiveWithoutSignal);
        let never = ConduitConfig::new().with_extract_control(EAST, RedstoneControl::NeverActive);

        assert!(always.can_extract(EAST, false) && always.can_extract(EAST, true));
        assert!(!with.can_extract(EAST, false) && with.can_extract(EAST, true));
        assert!(without.can_extract(EAST, false) && !without.can_extract(EAST, true));
        assert!(!never.can_extract(EAST, false) && !never.can_extract(EAST, true));
        // The control is per side: EAST's setting leaves WEST alone.
        assert!(!always.can_extract(WEST, true));
        assert!(!RedstoneControl::AlwaysActive.is_sensitive());
        assert!(RedstoneControl::ActiveWithSignal.is_sensitive());
        assert!(RedstoneControl::ActiveWithoutSignal.is_sensitive());
        assert!(!RedstoneControl::NeverActive.is_sensitive());
    }

    #[test]
    fn the_extract_toggle_still_wins_over_an_active_redstone_control() {
        let config = ConduitConfig::new()
            .with_extract(EAST, false)
            .with_extract_control(EAST, RedstoneControl::AlwaysActive);
        assert!(!config.extract(EAST), "the toggle is off");
        assert!(
            !config.can_extract(EAST, true),
            "an off side still extracts"
        );
        // A face past the six is never extractable and reads as the default.
        assert!(
            !ConduitConfig::new()
                .with_extract_control(FACE_COUNT, RedstoneControl::AlwaysActive)
                .can_extract(FACE_COUNT, true)
        );
        assert_eq!(
            ConduitConfig::new().extract_control(FACE_COUNT),
            RedstoneControl::NeverActive
        );
    }

    #[test]
    fn redstone_controls_keep_upstreams_ids_and_names() {
        let modes = [
            (0, RedstoneControl::AlwaysActive, "always_active"),
            (1, RedstoneControl::ActiveWithSignal, "active_with_signal"),
            (
                2,
                RedstoneControl::ActiveWithoutSignal,
                "active_without_signal",
            ),
            (3, RedstoneControl::NeverActive, "never_active"),
        ];
        for (id, control, name) in modes {
            assert_eq!(control.saved(), id);
            assert_eq!(control.name(), name);
            assert_eq!(RedstoneControl::from_saved(id), control);
            assert_eq!(RedstoneControl::named(name), Some(control));
        }
        for (name, control) in [
            ("always", RedstoneControl::AlwaysActive),
            ("with_signal", RedstoneControl::ActiveWithSignal),
            ("without_signal", RedstoneControl::ActiveWithoutSignal),
            ("never", RedstoneControl::NeverActive),
        ] {
            assert_eq!(RedstoneControl::named(name), Some(control), "{name}");
        }
        // A damaged id reads as the fresh default; an unknown word is no mode.
        assert_eq!(RedstoneControl::from_saved(4), RedstoneControl::NeverActive);
        assert_eq!(
            RedstoneControl::from_saved(-1),
            RedstoneControl::NeverActive
        );
        assert_eq!(RedstoneControl::named("sometimes"), None);
        assert_eq!(RedstoneControl::named(""), None);
    }

    #[test]
    fn the_per_side_redstone_controls_round_trip_through_the_saved_config() {
        let config = ConduitConfig::new()
            .with_extract_control(DOWN, RedstoneControl::AlwaysActive)
            .with_extract_control(UP, RedstoneControl::ActiveWithSignal)
            .with_extract_control(NORTH, RedstoneControl::ActiveWithoutSignal)
            .with_insert(SOUTH, true);
        let saved = config.saved();
        assert_eq!(saved.extract_control, [0, 1, 2, 3, 3, 3]);
        assert_eq!(ConduitConfig::from_saved(saved), config);
        // A damaged id reads as the fresh default rather than an arbitrary mode.
        let damaged = Saved {
            extract_control: [7, -2, 2, 3, 3, 3],
            ..saved
        };
        let restored = ConduitConfig::from_saved(damaged);
        assert_eq!(restored.extract_control(DOWN), RedstoneControl::NeverActive);
        assert_eq!(restored.extract_control(UP), RedstoneControl::NeverActive);
        assert_eq!(
            restored.extract_control(NORTH),
            RedstoneControl::ActiveWithoutSignal,
            "a valid id beside a damaged one still reads"
        );
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
        let config = ConduitConfig::new()
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
