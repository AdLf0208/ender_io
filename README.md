# Ender IO for Horizon

A native Horizon port of **Ender IO 8.2.12-beta** (`Team-EnderIO/EnderIO`,
**CC0** in the jar's `mods.toml`, **Unlicense** text in the repository): **slice 1**,
the **item conduit** — a conduit block whose six sides connect to other conduits
and to inventories, with the per-side extract/insert configuration upstream has,
moving up to 4 items every 20 ticks between the inventories it joins.

This is one flat SDK package (`api = "0.2"`): the code in `src/` and the carried
content in `assets/` and `data/` travel in the same `.hmod`.

**LOCAL-ONLY** until the maintainer approves publishing (ARCHITECTURE.md §8);
CC0/Unlicense needs no permission to publish, only the campaign's approval. The
content of this slice is carried from the exact ATM10 jar — provenance in
`../_shared/carry-work/enderio/SOURCE.md` and `LICENSE-NOTICE.md`.

- SDK nightly `0b14f8acfd97fa2c750e42795249dcc981178ca4`, mixins
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
- By default every connected side **extracts** and none **inserts** (upstream's
  `ItemConduitConnectionConfig.DEFAULT`). Turn insertion on for the destination
  side: `/enderio config <x> <y> <z> <down|up|north|south|west|east> insert on`.
- `/enderio info <x> <y> <z>` prints the connections and the configuration.
- `/enderio config … <insert|extract|round> <on|off>` sets one option; `round`
  spreads the source over several destinations.
- The conduit moves up to 4 items once a second per extracting side.

## What this slice does not do

Filters, speed tiers (enhanced/vibrant conduits), priority and dye channels; the
conduit screen and redstone control; the bundle, facades, the Yeta Wrench and
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
cargo horizon-mod fits enderio.hmod --build nightly
```

`.horizon/sdk/` is fetched per clone and never packed; `Cargo.lock` must not be
packed either (it changes the artifact).
