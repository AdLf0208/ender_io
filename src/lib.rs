//! Ender IO for Horizon — slice 1, the item conduit.
//!
//! Upstream a conduit bundle is a block entity whose node holds, per side, the connection
//! configuration, and whose network moves items between the inventories its connections reach.
//! This slice keeps the per-side configuration in the block entity's saved compound (as
//! upstream's node data is), draws the connection state as the block's six boolean properties,
//! and moves the items at the end of the level's tick: every cycle, each extracting side hands
//! up to [`runtime::SPEED`] items to the network's inserting sides. Each side's extraction is
//! gated by its own redstone control (`always_active`, `active_with_signal`,
//! `active_without_signal`, or upstream's default `never_active`), read from the block's
//! neighbour signal.
//!
//! The rules — the faces, the default configuration, the connection masks, the insert order and
//! the cycle — live in [`runtime`], byte-identical to the retired `.hnmod` bundle's `runtime.rs`
//! and tested by `cargo test` without the game. [`support`] is the glue: the block entity part
//! that queues a conduit, the level tick that drains the queue, the network and the commands.
//!
//! | Moment | Here |
//! |---|---|
//! | a placed conduit has a block entity | [`Hooks::register`]: `block_entity_type("conduit")` |
//! | its tick | [`support::ConduitPart`] (`block_entity_ticks`, `block_entity_tick`) |
//! | the connections and the transfer | [`Hooks::level_tick_post`] → [`support::on_level_tick_post`] |
//! | `/enderio info|config` | [`Hooks::register_commands`], [`Hooks::run_mod_command`] |
pub mod runtime;

mod support;

use std::sync::{Arc, Mutex};

use mod_api::{
    self, Flow, Hooks, HostSide, LevelTickPostEvent, RegisterCommandsEvent, Registrar,
    RegistrationContext, RunModCommandEvent, ServerHost,
};

use crate::support::{CONDUIT, ConduitPart, Queue};

/// The mod's session state: the conduits whose block entity ticked this tick.
#[derive(Default)]
struct Mod {
    queue: Arc<Mutex<Queue>>,
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the conduit's block entity type (the home of its per-side configuration) and
    /// attach the part that makes it tick. A refusal here can only mean the same part attached
    /// twice, which is a broken build rather than a world.
    fn register(&self, registrar: &mut dyn Registrar, _context: &RegistrationContext) {
        let kind = registrar
            .block_entity_type("conduit")
            .blocks(&[CONDUIT])
            .build();
        let attached =
            registrar.block_entity_part(&kind, ConduitPart::new(Arc::clone(&self.queue)));
        debug_assert!(
            attached.is_ok(),
            "the conduit part attaches once: {attached:?}"
        );
    }

    /// After the level's tick: refresh each queued conduit's connections and, on a network's
    /// cycle tick, move its items. A client's tick runs nothing.
    fn level_tick_post(&self, host: HostSide<'_>, event: &mut LevelTickPostEvent) {
        let HostSide::Server(host) = host else {
            return;
        };
        // The pass reaches containers through this level's `ITEMS` capability, and
        // `LevelRef::capability` answers only for the level the server ticks now. Outside that
        // window the queue is left alone and the next tick retries, rather than dropping a
        // conduit's work: the queue is bounded by the number of conduits, so waiting costs
        // nothing.
        if !event.level.in_place(host) {
            return;
        }
        let queued = {
            // Only this expression runs under the lock: one `remove` on the port's own state,
            // which cannot panic and so cannot poison it.
            self.queue
                .lock()
                .expect("the conduit queue lock is held for one drain")
                .take(event.level)
        };
        support::on_level_tick_post(host, event.level, queued);
    }

    /// `/enderio info|config`.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        support::declare_commands(event.commands);
    }

    /// Run the two usages [`Hooks::register_commands`] declares.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        support::run_command(host, event)
    }
}
