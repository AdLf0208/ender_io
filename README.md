# enderio (Horizon port)

A Horizon port of [enderio](Team-EnderIO/EnderIO) — Unlicense.

**Status: first slices.** What is below is what the port actually does today.

## Implemented so far

- The block.** `enderio:conduit` (template `minecraft:stone`, hardness 1.5,
- blast resistance 10, no occlusion, a 3 px core outline) with six boolean
- state properties — `down`, `up`, `north`, `south`, `west`, `east` — and a
- `multipart` blockstate that draws the 3 px core plus one 2 px arm per
- connected side. The block carries a self-drop loot table.
- Connections.** A face connects to another `enderio:conduit` or to a block
- with an item container reachable from the touching side (the pin's
- `native_container_slots`, API 48). The connection state is recomputed after

## Not yet ported

- Implemented: the conduit block and item, carried texture/models/lang/loot/tab,
- the connection state, the per-side configuration, the network cycle, item
- transfer, `/enderio info|config`.
- Not implemented (planned slices, see `FINDINGS.md`):
- item **filters** (extract/insert filter slots), **speed tiers**
- (enhanced/vibrant item conduits), **priority** and **dye channels**;
- the **conduit screen** (menu/slots/toggles), the redstone control;
- the **bundle** (several types in one block), **facades**, the Yeta Wrench

## Install

Build the bundle with a native-mod capable launcher, or add the file from the
registry once it is listed. Pinned to Horizon `1d63e0a` and horizon-mixins
`5a64af93` for game 26.3.

## Build from source

```sh
cargo test
python3 tools/horizon.py check --horizon <horizon checkout>
python3 tools/horizon.py build --horizon <horizon checkout>
python3 tools/package.py
```

## Credits and licence

Original mod: Team-EnderIO/EnderIO. Port: AdLf0208. Licence: Unlicense — see [LICENSE](LICENSE);
the original notices are preserved.
