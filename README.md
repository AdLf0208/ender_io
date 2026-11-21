# Ender IO for Horizon

A native Horizon port of **Ender IO 8.2.12-beta** (`Team-EnderIO/EnderIO`,
**CC0** in the jar's `mods.toml`, **Unlicense** text in the repository): **slice 1**,
the **item conduit** — a conduit block whose six sides connect to other conduits
and to inventories, with the per-side extract/insert configuration upstream has,
each side's extraction gated by its own redstone control, moving up to 4 items
every 20 ticks between the inventories it joins.

This is one flat SDK package (`api = "0.2"`): the code in `src/` and the carried
content in `assets/` and `data/` travel in the same `.hmod`.

**LOCAL-ONLY** until the maintainer approves publishing (ARCHITECTURE.md §8);
CC0/Unlicense needs no permission to publish, only the campaign's approval. The
content of this slice is carried from the exact ATM10 jar — provenance in
`../_shared/carry-work/enderio/SOURCE.md` and `LICENSE-NOTICE.md`.

- SDK nightly `0a875da91dbb0fdcb7ba8c42c11341aa229af51a`, mixins
  `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, for Minecraft 26.3. The pin is
  `[native]` in `horizon-mod.toml`; the launcher compiles against it.
- Class/tier: `content` / `deep` (per `horizon/docs/modpacks.md`). A `deep` mod is
  a campaign of slices, so this is slice 1: carried content (block, item, models,
  texture, lang, loot, tab) plus the conduit's logic (block entity, level tick,
  commands), not a data-only carry.
- Upstream behaviour and the slice boundary: [notes.md](notes.md) and
  [FINDINGS.md](FINDINGS.md).

## Layout

```text
horizon-mod.toml      the only manifest: id/name/version/api/side/capabilities + [native] pin
Cargo.toml            one crate; its only dependency is horizon-mod-api from .horizon/sdk/
src/lib.rs            the one #[mod_api::horizon_mod] impl Hooks
src/support.rs        the glue: the conduit's block entity part, the level tick, the commands
src/runtime.rs        the pure rules (faces, configuration, cycle, insert order) and their tests
assets/enderio/       the carried blockstates, models, texture, lang
data/enderio/         the carried block, creative tab and loot table
tools/e2e.sh          the in-game scenario (needs a launcher-built game; see notes.md)
enderio.hmod          the deliverable
```

## Using it

- Place conduits between inventories; a conduit joins another conduit or any
  block that has an item container on the touching side.
- By default no side moves anything: every connected side's extract toggle is on,
  no side inserts, and every side's redstone control is `never_active`
  (upstream's `ItemConduitConnectionConfig.DEFAULT`, whose `canExtract` answers
  false for it). Set the source side's control and the destination side's insert:
  `/enderio config <x> <y> <z> west redstone always_active` then
  `/enderio config <x> <y> <z> east insert on`.
- The redstone controls are upstream's four: `always_active`,
  `active_with_signal`, `active_without_signal` and `never_active`; the two
  signal modes read the conduit's neighbour signal (a redstone block or torch
  beside it), as upstream's `level.hasNeighborSignal` does.
- `/enderio info <x> <y> <z>` prints the connections, the configuration and the
  sides whose redstone control is not `never_active`.
- `/enderio config … <insert|extract|round> <on|off>` and
  `… <redstone> <always_active|active_with_signal|active_without_signal|never_active>`
  set one option; `round` spreads the source over several destinations.
- The conduit moves up to 4 items once a second per extracting side.

## What this slice does not do

Filters, speed tiers (enhanced/vibrant conduits), priority and dye channels; the
conduit screen; the bundle, facades, the Yeta Wrench and
Conduit Probe; waterlogging; the energy, fluid and redstone conduits; every
machine; the mobs, armour and items; the alloy system (so there is no crafting
recipe yet); and the block-entity renderer for the arms. Each is a later slice —
the full list is in [notes.md](notes.md).

## Building

```sh
cargo test --offline                                  # the rules and the glue's codecs
cargo clippy --all-targets -- -D warnings
cargo fmt --check                                     # not --all: that checks the fetched SDK
rm -f Cargo.lock && cargo horizon-mod build            # -> target/horizon-mod/enderio.hmod
cargo horizon-mod check enderio.hmod
cargo horizon-mod fits enderio.hmod --build 0a875da
```

`.horizon/sdk/` is fetched per clone and never packed; `Cargo.lock` must not be
packed either (it changes the artifact).
