// Declare `/enderio config|info`.
fn observe_register(commands: &mut Vec<NativeCommand>) {
    crate::game::enderio_state::on_register_commands(commands);
}
