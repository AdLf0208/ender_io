// After the level's tick: refresh the conduits' drawn connections and, on
// each network's cycle tick, move items between the inventories they reach
// (`native_level_tick_post`, API 23).
fn observe_level_tick_post(game: &mut crate::game::Game<'_>, dimension: &str) {
    crate::game::enderio_state::on_level_tick_post(game, dimension);
}
