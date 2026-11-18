# Ender IO — a Horizon port

**Ender IO** is a Minecraft mod. Ender IO for Modern Minecraft. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [Team-EnderIO/EnderIO](https://github.com/Team-EnderIO/EnderIO) — licence **Unlicense**.

| | |
|---|---|
| Upstream mod | Ender IO ([source](https://github.com/Team-EnderIO/EnderIO)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `enderio.hmod` (install it with the Horizon launcher) |
| Licence | **Unlicense** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

### What this slice does not do

Filters, speed tiers (enhanced/vibrant conduits), priority and dye channels; the
conduit screen and redstone control; the bundle, facades, the Yeta Wrench and
Conduit Probe; waterlogging; the energy, fluid and redstone conduits; every
machine; the mobs, armour and items; the alloy system (so there is no crafting
recipe yet); and the block-entity renderer for the arms. Each is a later slice —
the full list is in [notes.md](notes.md).

## Not yet ported

- item **filters** (extract/insert filter slots), **speed tiers**
- the **conduit screen** (menu/slots/toggles), the redstone control;
- the **bundle** (several types in one block), **facades**, the Yeta Wrench
- **energy, fluid and redstone conduits**;
- every **machine** (alloy smelter, sag mill, Stirling generator, soul
- **mobs, armour and items** (ender armour/weapons, staffs, electromagnet,
- the **alloy system** and therefore the conduit recipe;
- a **block-entity renderer** for the arms (cosmetic; the model is static).

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `enderio.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Ender IO was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **Unlicense**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/Team-EnderIO/EnderIO

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run enderio` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/enderio/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
