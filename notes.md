# Ender IO → Horizon

Port of **Ender IO 8.2.12-beta** (`Team-EnderIO/EnderIO`, **CC0** in the jar's
`mods.toml`, **Unlicense** text in the repo), Minecraft 1.21.1 NeoForge, from
the exact ATM10 jar: `enderio-8.2.12-beta.jar`, **6,598,496 B**, SHA-1
`4e1d16a227eb36a6ac16d9f615765480027b8e82` (matches the pack's `modrinthHash`),
SHA-256 `5fe098153dddc2e7ccd9d38b0341dbc61f3072150958c916e5ef63f395e4eab1`,
CurseForge hash `47327023`. Provenance and the carried files:
[`../_shared/carry-work/enderio/SOURCE.md`](../_shared/carry-work/enderio/SOURCE.md).

**Slice 1 scope.** The **item conduit**: the `enderio:conduit` block, its
six-side connection state, its per-side configuration, and item transfer
between the inventories it joins, on the port's own tick. Everything else in
Ender IO is a gap (see the list below).

**Class + tier:** `content` / **`deep`** (`ports/ledger.csv`; provenance
`ports/_shared/source-map/atm10-sources.csv` row `enderio`). Approach per
Horizon's `docs/modpacks.md`: a `deep` mod is a long campaign of agent ports,
so this is slice 1 of it — carried content (block, item, model, texture, lang,
loot, tab) plus a native bundle (block entity + level tick + commands), not a
data-only carry and not a whole-mod port.

> **0.2.0 (2026-11-18):** Horizon retired `.hnmod` (mod-API merge #16941). The
> port is now one flat SDK `.hmod` (`api = "0.2"`); the sections below describe
> the **retired 0.1.0 `.hnmod` tree** and remain the record of how slice 1 was
> built and proved. What moved, the hook table, the 0.2.0 ladder and the new
> traps are in [`## Migration to the SDK format`](#migration-to-the-sdk-format-020-2026-11-18).

**Licence:** Unlicense → **rank 0** (public domain); the jar declares CC0, an
equivalent dedication, and 0.2.1 states one licence for the whole package (see
`## Review fixes (2026-11-18)`). Local-only until
the maintainer approves publishing (ARCHITECTURE.md §8); rank 0 carries no
downstream obligation. **Why now:** the licence-first, core-before-add-ons
queue puts it at rank 8 (`python3 ports/tools/port_queue.py`): rank 0, a core
mechanics mod with no required parent.

**Upstream analysis:** repository <https://github.com/Team-EnderIO/EnderIO>,
commit `8989fcebda3eb9e678fea4138f577c47a22a5e8d` (2026-11-04, shallow clone
in `ports/_shared/upstream/enderio`, deleted after the notes were written). The
files read, the extracted behaviour (connection rules, the per-side defaults,
the 4-items-per-20-tick cycle, round robin, self-feed) and the mapping to this
port are in [`FINDINGS.md`](FINDINGS.md). No Java was copied or translated; the
jar's `-sources` artifact was not downloaded.

## What slice 1 does

- **The block.** `enderio:conduit` (template `minecraft:stone`, hardness 1.5,
  blast resistance 10, no occlusion, a 3 px core outline) with six boolean
  state properties — `down`, `up`, `north`, `south`, `west`, `east` — and a
  `multipart` blockstate that draws the 3 px core plus one 2 px arm per
  connected side. The block carries a self-drop loot table.
- **Connections.** A face connects to another `enderio:conduit` or to a block
  with an item container reachable from the touching side (the pin's
  `native_container_slots`, API 48). The connection state is recomputed after
  every level tick and written only when it changed, so placing or removing a
  chest shows up within a tick.
- **Configuration.** Per side, in the conduit's block entity (saved with the
  chunk), with upstream's `ItemConduitConnectionConfig.DEFAULT`: **extract on
  every connected side, insert off, round robin off, self-feed off**.
  `/enderio info <x> <y> <z>` prints the connections and the configuration;
  `/enderio config <x> <y> <z> <side> <insert|extract|round> <on|off>` sets
  one (the conduit screen is a later slice).
- **Transfer.** Once every **20 ticks** (upstream `ticks_per_cycle`), each
  extracting side moves up to **4 items** (upstream `transfer_rate`) into the
  network's inserting sides, in face order or round-robin, one item at a time
  through `native_move_items` (API 48) so an item the target refuses stays in
  its slot. The cycle phase comes from the lowest conduit's position
  (upstream's per-network `tickOffset`).
- **Networks.** Conduits that touch form one network (breadth-first over the
  same block); the inventories at its edge are its ports.

### Deliberate deviations

| Upstream | Here | Why |
|---|---|---|
| Connections drawn by the bundle's block entity | six boolean block-state properties + `multipart` | the pin has no block-entity renderer at this tier; this is `docs/mods.md`'s own pipe example |
| Toggles in the conduit screen | `/enderio config` | the screen is machines-tier work; the saved configuration is the same |
| Round-robin cursor saved per side | derived from the cycle number | stateless across restarts, same rotation |
| Connection recomputed on neighbour change | recomputed after each level tick | simpler, and it also catches containers that appear without a block update |
| The block's outline includes the arms | the 3 px core only | a single shape table cannot vary with six booleans without 64 entries; documented, cosmetic |
| The conduit bundle holds several conduit types | one conduit type per block | slice 1 is the item conduit |
| No recipe | — | the upstream recipe needs `enderio:conduit_binder` and the conductive/pulsating alloy tags, i.e. other Ender IO content; inventing a recipe would be wrong |

## Slice boundary and gaps

Implemented: the conduit block and item, carried texture/models/lang/loot/tab,
the connection state, the per-side configuration, the network cycle, item
transfer, `/enderio info|config`.

Not implemented (planned slices, see `FINDINGS.md`):

1. item **filters** (extract/insert filter slots), **speed tiers**
   (enhanced/vibrant item conduits), **priority** and **dye channels**;
2. the **conduit screen** (menu/slots/toggles) — the redstone control it
   configures **is ported** since 0.2.2 (`/enderio config … redstone <mode>`);
3. the **bundle** (several types in one block), **facades**, the Yeta Wrench
   and Conduit Probe, waterlogging;
4. **energy, fluid and redstone conduits**;
5. every **machine** (alloy smelter, sag mill, Stirling generator, soul
   binder, obelisks, …) with its menu and screen;
6. **mobs, armour and items** (ender armour/weapons, staffs, electromagnet,
   soul vials, enderios, broken spawner, XP vacuum);
7. the **alloy system** and therefore the conduit recipe;
8. a **block-entity renderer** for the arms (cosmetic; the model is static).

Known limits of this slice: connection state is refreshed every level tick for
every loaded conduit (fine for a handful, a per-chunk cache is later work); a
conduit is not waterloggable; the item conduit's own animated texture is
carried and animates on the client.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Verification

Pin **`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`**, mixins
**`5a64af938ec86b00d53d16a408aabc0038fb3e57`** (schema 6). Every build, test
and e2e ran through `ports/tools/build-slot.sh` (`CARGO_INCREMENTAL=0`, no bare
cargo, no `--target-dir`); the in-game runs used the pin-matched shared
`horizon-ctl` (`ports/_shared/bin/horizon-1d63e0a5/horizon-ctl`) and the
port-built fast game.

### Ladder

| Step | Command (through the slot runner) | Result |
|---|---|---|
| runtime RED | `cargo test --locked --offline` (port) | `exit=101`, **104** `error[E…]` (the API did not exist yet) — `evidence/enderio/runtime-red.log` |
| runtime GREEN | `cargo test --locked --offline` | **10 passed**, 0 failed — `evidence/enderio/runtime-green.log` |
| fmt + clippy | `cargo fmt --all --check`; `cargo clippy --all-targets --locked --offline -- -D warnings` | pass, **0 warnings** |
| stage | `python3 tools/horizon.py stage --horizon …/pins/horizon-1d63e0a` | `status: ok`, **4 injections, 2 modules** |
| check | `python3 tools/horizon.py check …` | `cargo_exit: 0`, **0 warnings** — `evidence/enderio/horizon-check.log` |
| embedded tests | `python3 tools/horizon.py test …` (`--lib game::enderio::tests`) | **10 passed**, 0 failed, 1240 filtered — `evidence/enderio/horizon-test.log` |
| fast build | `python3 tools/horizon.py build …` | `cargo_exit: 0` — `evidence/enderio/build.json` |
| e2e | `build-slot.sh e2e -- … -- tools/e2e.sh` | **PASS**, `EXIT:0` — `evidence/enderio/e2e.log` |
| selftest | `SELFTEST=1 tools/e2e.sh` | **exit 1**, the flipped assertion failed — `evidence/enderio/e2e-selftest.log` |
| content `.hmod` | `tools/package.sh` (`cargo-horizon-mod build` + `check`) | **24,185 B**, SHA-256 `07d1a3c8af0236a88083af00de6d484d3a3478099b5813a8176a25d6531edfd3`, **12 entries**, `enderio 8.2.12-beta (mod, API 0.2, both)`, "no code: content only" |
| native `.hnmod` | `tools/package.sh` (`tools/package.py`) | **10,839 B**, SHA-256 `a9605c4cd53705b7ebd30dfc871a7f06240a05171f0068f38fa8f2790b0c46fa`, **8 entries**; `horizon-native.json` pins `horizon` = `1d63e0a5…`, `mixins` = `5a64af93…`, `game` 26.3 |
| `.hmod` load check | `tools/hmod-load.sh` | `PASS` + `EXIT:0`: the packaged `.hmod` registers `enderio:conduit` (block state and item) |
| `.hmod` negative control | same script, second launch **without** `--dev-mod` | `setblock reply: {"chat":["Unknown block type 'enderio:conduit'",…]}`, control block `minecraft:air`, no item given — the check is not vacuous |

### In-game scenario (quoted from `evidence/enderio/e2e.log`)

```
== A: a conduit moves 5 stone from the west chest to the east chest ==
  source before: 5, target before: 0
  Conduit: links west,east
  conduit state: enderio:conduit[down=false,east=true,north=false,south=false,up=false,west=true]
  source after: 0, target after: 5
== B: two chests with no conduit between move nothing (control) ==
  source: 5, target: 0
== C: a connected conduit with the default config moves nothing (control) ==
  source: 5, target: 0
  conduit state: enderio:conduit[down=false,east=true,north=false,south=false,up=false,west=true]
== D: a full target refuses the items (control) ==
  source: 5, target stone: 0, target dirt: 1728
== E: the rate is a few items per cycle, not the whole stack (control) ==
  after 25 ticks: source 12, moved 8 of 20
PASS: a conduit handed 5 stone from one chest to another only after its east side was set to insert, its state showed both connections, a 20-stone source moved only its first few items per cycle, and chests with no conduit / an unconfigured conduit / a full target moved nothing
EXIT:0
```

What the controls prove: the **same** conduit that moves the stone in A moves
nothing in C until `/enderio config … east insert on` (upstream's default
inserts nowhere), nothing moves across a gap with no conduit (B), a target
whose 27 slots are full takes nothing (D), and a 20-stone source moves 8 items
in 25 ticks — two cycles of the upstream `transfer_rate` of 4, not the whole
stack (E). So A's 5 stone is the conduit's own transfer, at its own rate, and
not a spill or a hopper. The state line shows the connection
properties the `multipart` model selects on (the render itself was not
screenshotted; the block-state tie is the content's). The scenario is **fail-capable**: `SELFTEST=1`
asserts the source is still full and fails —

```
FAIL: SELFTEST: the source chest is empty (5 stone moved), so the check is alive
```

with `EXIT:1` (the run logs `selftest_exit=1`).

`tools/test_template.py` passes (it plans this bundle's `block_entity`
injection against the template). `tools/test_stage.py` is the template's
**one-injection** fixture — it asserts exactly one injection and one module, so
it cannot pass for this bundle's four injections and two modules; the staged
ladder above is the evidence, as the port's `AGENTS.md` allows.

Harness notes for the next slice: `ctl container X Y Z ITEM COUNT` puts the
whole count in **one** empty slot (it does not split a stack), so filling a
chest takes one call per slot; `ctl block` reports the full state in `.state`;
`ctl chat "/enderio …"` is the path that reaches `perform_command`
(`ctl command` bypasses it); the End arena keeps ambient item entities out of
the counts.

## Artifact rebuild at the campaign pin (2026-11-09)

**Mode B** — the pin was already the campaign pin (`1d63e0a5` / mixins
`5a64af93`); the delivered `.hmod` predated the current tree and the port had no
`.hnmod`, so a port with 4 injections had no valid native artifact. Both
packages were rebuilt from the tree in one build-slot lease, the ladder was
re-run, and the e2e was re-run (default + selftest).

| Step | Command (through `ports/tools/build-slot.sh`) | Result |
|---|---|---|
| fmt | `cargo fmt --all -- --check` | exit 0 |
| clippy | `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0, **0 warnings** |
| runtime tests | `cargo test --locked --offline` | **10 passed**, 0 failed |
| stage | `python3 tools/horizon.py stage` | exit 0, `stage_changed: true`, revision `1d63e0a5a37a` |
| staged check | `python3 tools/horizon.py check` | `cargo_exit: 0`, **0 warnings** |
| embedded tests | `python3 tools/horizon.py test` | `cargo_exit: 0`, **10 passed / 0 failed / 1240 filtered** (`game::enderio::tests`) |
| package | `tools/package.sh` twice | identical hashes both passes |
| e2e default | `build-slot.sh e2e … -- tools/e2e.sh` | `PASS` + `EXIT:0` |
| e2e selftest | `SELFTEST=1` | `FAIL: SELFTEST: the source chest is empty (5 stone moved), so the check is alive` + `EXIT:1` |

Artifacts (delivered in the port, never `target/`):

| File | Size | SHA-256 | Entries |
|---|---:|---|---:|
| `content/enderio.hmod` | 24,185 B | `07d1a3c8af0236a88083af00de6d484d3a3478099b5813a8176a25d6531edfd3` | 12 |
| `enderio.hnmod` | 10,839 B | `a9605c4cd53705b7ebd30dfc871a7f06240a05171f0068f38fa8f2790b0c46fa` | 8 |

The e2e (The End, `--dev-mod content`, shared `1d63e0a5` `horizon-ctl`) printed:
the conduit links west/east and moves 5 stone from the west chest only after its
east side is set to insert; two chests with no conduit move nothing; a connected
conduit with the default configuration moves nothing; a full target moves
nothing; a 20-stone source moves 8 in 25 ticks (two upstream cycles of 4).
`SELFTEST=1` asserts the false "source still full" expectation and exits 1.

`python3 ports/tools/check-e2e-logs.py enderio --strict` → both logs `ok`.

## Content embedded in the bundle; re-verified in game (2026-11-10)

**Beat: one file per project.** The registry takes one `file` per version, so a
port that ships a `.hmod` *and* a `.hnmod` cannot be listed faithfully. This port
now follows the AE2 pattern proved on `charginggadgets` (PR #526): the built
content mod travels **inside** the bundle at
`mixins/enderio/data/enderio-content.hmod`, and `mixins/enderio/embedded_content.rs`
compiles it in with `horizon_client::native_data!` and injects it at
`content_root` (`target = "horizon_client::game::mods::content_root"`,
`kind = "HEAD"`, `cancellable = true`). The helper writes the file to
`.horizon-cache/enderio-embedded-content.hmod` once, appends it to the dev-mod list
and returns the merged root; the recursive call stops at its own
`dev.contains(&file)` guard, so the vanilla body runs once. The `.hmod` stays in
`content/` as the build input; `tools/package.py` refuses a stale copy in `data/`.

**Proved with no `--dev-mod`:** the e2e launches the game *without* passing the
content folder, so every content id and every behaviour it asserts comes from the
bundle. `EMBEDDED=1` is the flag; the launch line passes an empty dev-mod list.

| Artifact | Size | SHA-256 | Entries |
|---|---:|---|---:|
| `content/enderio.hmod` (input) | 24,185 B | `07d1a3c8af0236a88083af00de6d484d3a3478099b5813a8176a25d6531edfd3` | 12 |
| `enderio.hnmod` (deliverable, embeds it) | 35,163 B | `20aaca058ba36cd57204ed0dcfa3440d32914761cb58606b630837e1e98bc51e` | 10 |

E2E (one lease each, `CARGO_INCREMENTAL=0`, shared `1d63e0a5` `horizon-ctl`, the
port's own fast game): default `PASS` + `EXIT:0`; `SELFTEST=1` `FAIL` + `EXIT:1`.

`python3 ports/tools/check-e2e-logs.py enderio --strict` → every log carries its
own `PASS`/`FAIL` + `EXIT:` pair. `python3 ports/tools/upstream_ready.py enderio` →
**READY (exit 0)**.

## Review repairs (PR #566, 2026-11-11)

Review: `ports/_shared/horizon-mods/reviews/pr-566-enderio.md` (CHANGES_REQUESTED, 2 items + supplement).

| Reviewer's item | This job |
|---|---|
| 1. artifact not staged | **orchestrator** (no write access) |
| 2. relative link `(Team-EnderIO/EnderIO)` | already absolute; re-checked |
| supplement: the stale one-injection/one-module fixtures do not establish the build | **open (tooling)**: `tools/test_stage.py` and `tools/test_template.py` still assume a smaller bundle; the ladder uses `horizon.py` directly. Recorded, not half-fixed |
| (prose) "Not yet ported: Implemented: …" | **rewritten** for players |

Artifact: `enderio.hnmod` **36,724 B** `3071165b84973e94d09211eb615ad2e7a7a1b46de71a75a94005f0ae6538b87a`.

## Migration to the SDK format (0.2.0, 2026-11-18)

Horizon's mod-API merge (#16941, launcher ≥ 0.2.70; PR #17992 cleans up the old files) retired
the `.hnmod` container: one flat `.hmod` carries the code *and* the content, `src/lib.rs` is the
one `impl mod_api::Hooks`, and the launcher compiles it against the pinned game build. **0.2.0**
is the same slice republished in that format — a **first publication** (there is no closed PR to
supersede; the port was never submitted). The reference recipes were
`ports/imfast/notes.md`, `ports/enderstorage/notes.md` and `ports/naturesaura/notes.md`.

Pins: SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (2026-11-18, `package_format` 6,
`content_api` "0.2", toolchain rust 1.99.1), mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, for Minecraft **26.3**. The SDK is cached in-tree
at `../_shared/sdk/0b14f8a/` and copied into `.horizon/sdk/` (gitignored) for offline builds.

### What moved, and where the retired tree is

The retired paths were **moved before the first build** (the builder refuses any path that is not
a package path) into `../_shared/evidence/enderio/retired-hnmod-0.1.0/`: `mixins/`, `content/`
(with its own manifest, built `.hmod` and the old assets/data copies), `tools/{audit,horizon.py,
package.py,package.sh,test_stage.py,test_template.py,notice.py,hmod-load.sh}`, `enderio.hnmod`,
`compatibility.json`, `rust-toolchain.toml`, `Cargo.lock`, the old `Cargo.toml`
(`Cargo.toml.0.1.0`) and the old repo `AGENTS.md`. The retired tree is evidence; nothing was
deleted. `tools/e2e.sh` was kept and updated; `tools/notice.py` and `tools/hmod-load.sh` were
retired with the format (the builder packs root files itself, and a native `.hmod` cannot load on
an unmodded game).

| 0.1.0 path | 0.2.0 path | Notes |
|---|---|---|
| `mixins/enderio/runtime.rs` | `src/runtime.rs` | `cp` + `cmp`, then `sha256` on both and on the packaged entry — **byte-identical**, `869d663b…414d`; `pub mod runtime;` at the crate root |
| `mixins/enderio/support.rs` (module `enderio_state`) | `src/support.rs` (module `support`) | the same glue, rewritten against hosts/events; 9 unit tests of its codecs and readouts |
| `mixins/enderio/{block_entity,level_tick,register,perform,embedded_content}.rs` | gone | one `Hooks` method each, a Part, or nothing to do |
| `mixins/enderio/horizon-mixin.toml` (2 `[[module]]`, 5 `[[injection]]`) | `horizon-mod.toml` (`[native]` + the id) | no plan, no injection schema |
| `mixins/enderio/data/enderio-content.hmod` (the AE2 write-out) | gone | the content is in the same package: `assets/` + `data/` at the root |
| `content/{assets,data}` | `assets/`, `data/` | `cp -a`, then `sha256` diff against the retired copy → **unchanged** (8 + 3 files) |
| `content/horizon-mod.toml` | merged into the root manifest | `api = "0.2"` + `[native]`; `version` is the **port's** 0.2.0, not upstream's 8.2.12-beta |
| `content/enderio.hmod` (24,185 B, content only) | not needed | one package |
| `enderio.hnmod` (36,724 B, embedded content) | `enderio.hmod` (48,460 B, code + content) | |
| `compatibility.json`, `rust-toolchain.toml` | `[native]` pin in `horizon-mod.toml` | the launcher's toolchain builds; no `rust-toolchain.toml` |
| `Cargo.lock` | not kept, `/Cargo.lock` in `.gitignore` | it is packed and changes the artifact (trap 5) |
| `tools/horizon.py`, `package.py`, `package.sh`, `test_stage.py`, `test_template.py`, `audit/` | gone | the SDK ladder is `cargo test` + `cargo horizon-mod build/check/fits` |

**Kept:** `notes.md`, `FINDINGS.md`, `VALIDATION.md`, `LICENSE`, `tools/e2e.sh` (updated for a
launcher-built game and an optional `CONTROL_GAME`). **Added:** `README.md` (rewritten for the SDK
package), `LICENSE-NOTICE.md` (the provenance, packed at the root), `.gitignore`.

### Manifest mapping

- identity: `id = "enderio"` (also the resource namespace), `name = "Ender IO"`,
  `version = "0.2.0"` (the port's version), `authors = [upstream's eleven, AdLf0208]`.
- `api = "0.2"` (exact), `side = "both"` (single player's integrated server belongs to the client
  installation), `capabilities = ["registries"]` (the schema's one value; this package declares a
  block, an item and a block entity type).
- `[native] game = "26.3"`, `horizon = "0b14f8a…"`, `mixins = "e59da90…"` replace
  `[compatibility]` and the bundle's `horizon-native.json` pins.
- `license`, `homepage`, `issues` are **not** `horizon-mod.toml` fields (the builder strips them);
  the licence lives in `Cargo.toml` (`Unlicense`, one licence for code and content) and in
  `LICENSE`/`LICENSE-NOTICE.md`.

### Helper → hook table (every former injection and module)

| 0.1.0 (`horizon-mixin.toml`) | 0.2.0 |
|---|---|
| `[[module]] runtime` → `crate::game::enderio` | `src/runtime.rs`, `pub mod runtime;` — unchanged |
| `[[module]] state` → `crate::game::enderio_state` | `src/support.rs`, `mod support;` — the same functions, host/event types |
| `embedded-content` (`content_root`, HEAD, cancelling) | **gone** — one package carries code and content; there is nothing to write out or append to the dev list |
| `block-entity` (`block_entity` 15, returned `Some((id, Compound::new()))`) | `Hooks::register` → `Registrar::block_entity_type("conduit").blocks(&["enderio:conduit"]).build()`, then `block_entity_part`. No `BlockEntityPart::block_entity` method is needed: a declared type starts with an empty compound, exactly what 0.1.0 returned, and `read_config` answers upstream's defaults for it |
| — (no tick in 0.1.0: the level pass enumerated every conduit) | `BlockEntityPart::block_entity_ticks` (`ticks = true`) + `block_entity_tick` (queues the position, answers `Unchanged`) |
| `level-tick-post` (`level_tick_post` 23) | `Hooks::level_tick_post` → drains the per-level queue and runs `support::on_level_tick_post` |
| `register-commands` (`register_commands` 8) | `Hooks::register_commands` → `support::declare_commands` (`Command::new`, same two usages) |
| `perform-command` (`perform_command` 8, split words) | `Hooks::run_mod_command` → `support::run_command`; typed `CommandArgumentValue::{BlockPos, String}`, `Flow::Cancel` = handled, readout text byte-identical |
| `game.native_block(pos) == CONDUIT` | `LevelRef::block(host, pos) == Some(host.block_key(CONDUIT)?)` (`BlockKey`, resolved once per pass) |
| `game.native_container_slots(dim, pos, side)` | `LevelRef::capability(host, &ITEMS, pos, Some(side))` + `ResourceHandler::size(host) > 0` |
| `game.native_move_items(dim, from, face, to, face, None, budget)` | `mod_api::move_resources(host, &from, &to, |_| true, budget, tx)` inside `host.transaction(…)` |
| `game.native_block_property(pos, name)` | `LevelRef::block_property(host, pos, name)` |
| `game.native_set_block_property_with(pos, name, value, flags::UPDATE_CLIENTS)` | `host.set_block_property_with(pos, name, value, BlockUpdateFlags::CLIENTS.bits())` |
| `game.world.block_entities` (enumerated the whole level) | **not available**: the conduit's tick queues it, the level tick drains the queue (trap 11 of the campaign — `laserio`/`integratedtunnels` shape) |
| `game.world.game_time` | `LevelRef::game_time(host)` |
| `game.world.modified.insert((x>>4, z>>4))` after the config write | `BlockEntityRef::replace_data(host, data)` — the documented transactional load, `setChanged` and client update |
| `game.native_feedback(Text::literal(..))` | `host.commands().feedback(Text::literal(..))` |
| `NativeCommand::new(..)` | `Command::new(..)` |

`uses.json` in the package records 241 surface entries: **5 hooks** (`block_entity`,
`block_entity_tick`, `block_entity_ticks`, `level_tick_post`, `register_commands`, `run_mod_command`
— 6 hook ids plus their fields), 26 methods, 3 Part kinds, both macros, **`registries`** and
**`horizon:items`** capabilities, and `move_resources`.

### The ladder (all in one build slot each, `CARGO_INCREMENTAL=0`, no `--target-dir`)

| Step | Command | Result |
|---|---|---|
| runtime only, before any glue | `cargo test --offline` | **10 passed**, 0 failed — `evidence/enderio/sdk-runtime-tests.log` |
| glue RED | `cargo test --offline` (the tested helpers stubbed with `todo!()`) | **11 passed, 8 failed** (`not yet implemented`) — `sdk-test-red.log` |
| glue GREEN | `cargo test --offline` | **19 passed**, 0 failed (10 `runtime::`, 9 `support::`) — `sdk-test.log` |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** — `sdk-clippy.log` |
| format | `cargo fmt --check` (**not** `--all`: that checks the fetched SDK) | clean after one `cargo fmt` on `src/support.rs`; `src/runtime.rs` unchanged — `sdk-fmt.log` |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `SDK 0b14f8a … in ./.horizon/sdk`, `Built ./target/horizon-mod/enderio.hmod` — `sdk-build.log` |
| same package again | the same command | **48,460 B**, sha256 `322251413fa6…f1c4` both times — `sdk-build-second.log`, `sdk-sha256.txt` |
| registry check | `cargo horizon-mod check enderio.hmod` | `enderio 0.2.0 (native-mod, format native-1/0b14f8a…/e59da90…, for 26.3)` / `48460 bytes, SHA-256 322251413fa6…f1c4, unsigned`, **exit 0** — `sdk-check.log` |
| fits the pinned build | `cargo horizon-mod fits enderio.hmod --build nightly` | `Ender IO:` / `0b14f8a  made for`, **exit 0** — `sdk-fits.log` |
| `runtime.rs` untouched | `unzip -p enderio.hmod src/runtime.rs \| sha256sum` | `869d663b…414d` = the retired file and `src/runtime.rs` — `sdk-runtime-sha256.txt` |
| content carry unchanged | `sha256sum` of every root `assets/`+`data/` file vs the retired copy | identical, 8 + 3 files — `sdk-content-carry.log` |

The package is **20 entries**: `horizon-mod.toml`, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`,
`README.md`, `assets/enderio/…` (8), `data/enderio/…` (3), `src/lib.rs`, `src/runtime.rs`,
`src/support.rs`, `uses.json` — **no `Cargo.lock`** (`sdk-entries.txt`).

### Traps this migration hit (new to this port)

1. **`build-slot.sh` forwards `test`/`build` to the second machine, whose runner already `cd`s
   into the port** — the brief's `cd ports/enderio` then fails with *"No such file or directory"*
   and the run dies before compiling. Every SDK migration here uses **`BUILD_LOCAL=1`** (the far
   box also has no `.horizon/sdk`); recorded so the next agent does not read it as a broken port.
2. **`event.reply` is `&mut BlockEntityTickReply`.** A `block_entity_tick` that leaves it alone
   answers `Changed` and dirties the chunk every tick; ours writes only block properties, so it
   sets `*event.reply = BlockEntityTickReply::Unchanged`.
3. **A face index is not a `Direction`.** The old `Endpoint::inventory()` returned
   `(Pos, Direction)`; the new `inventory()` helper takes the conduit's `Face` and asks the
   opposite side of the *neighbour*. Mixing them is an `E0308` (`expected usize, found Direction`);
   `side_towards_conduit(face)` is the one conversion, and it is tested.
4. **`cargo test`/`clippy` write a `Cargo.lock`.** Remove it immediately before each
   `cargo horizon-mod build` (trap 5 of the campaign); the package packs root files, so a lock in
   the tree changes the bytes. `unzip -l | grep Cargo.lock` = 0 is the check.
5. **The builder packs every root document.** The first package (before the rewrite) still carried
   the 0.1.0 `README.md` that told readers to open `compatibility.json`; rewriting the README and
   adding `LICENSE-NOTICE.md` first changed 46,707 B → 48,434 B in one step (the final 48,460 B
   adds the two comment fixes and the named default mask below). Rewrite, then build.
6. **`cargo fmt` reformats only this crate; `cargo fmt --check` still has to be run *before*
   `cargo fmt`** to see the diffs — and `--all` reports the fetched SDK's own files.
7. **A block entity tick is not told its level.** `in_place_level(host)` (the level whose
   `in_place` is true) is the one the tick belongs to, the same shape `laserio`/`moderndynamics`
   use; a client's `level_tick_post` returns before touching the queue.
8. **A declared block entity type needs no content marker.** The carried
   `data/enderio/horizon/block/conduit.json` is unchanged (no `block_entity` field): the type is
   associated with the block by `block_entity_type("conduit").blocks(&["enderio:conduit"])`.
   `moderndynamics`' content proves the same shape.
9. **Typed commands change the argument path, not the text.** `argument("pos")` already is a
   `BlockPos`, so the 0.1.0 three-word split is gone; the confirmation and readout lines are
   unchanged byte for byte and pinned by `the_info_readouts_are_the_zero_one_zero_text`.
10. **`move_resources` is the item move, not a per-slot hand-off.** It fills from the source
    slots up to the budget in one call (inside `host.transaction`), where 0.1.0 called
    `native_move_items` per destination; the observable cap (4 per cycle, a refused item stays in
    its slot) is the same and is what the scenario asserts.

### Behaviour that changed with the format (deliberate, and why)

- **The connection refresh follows the queue, not a level scan.** 0.1.0 enumerated
  `game.world.block_entities` every level tick; the SDK cannot enumerate a level's block entities,
  so a conduit's own tick queues it and the level tick refreshes exactly those. The engine ticks
  every loaded block entity of a declared ticking type, so the two sets coincide; a conduit whose
  chunk is unloaded is neither ticked nor refreshed, as before.
- **`/enderio` incomplete commands are refused by the dispatcher.** With typed arguments
  (`<pos:block_pos> <side:word> …`) the game rejects a missing argument before the mod is asked,
  so the 0.1.0 usage lines now only answer a *wrong-valued* argument. The lines themselves are
  unchanged.
- **No `log` crate.** The retired `embedded_content.rs` warned through `log::warn!` when it could
  not write its content out; that injection is gone, so the port has no logging path (and none is
  needed). Recorded rather than replaced.
- **The config write goes through `replace_data`**, so the chunk is marked changed and clients get
  the block update by the game's own path instead of a hand-rolled `world.modified` insert.

### Gaps after the migration

1. **No in-game re-run in this environment.** `tools/e2e.sh` now needs a game the **launcher**
   built from this package (`horizon-launcher native-build ports/enderio --instance <i>
   --trust-native-code`) plus, optionally, a `CONTROL_GAME` without it; `horizon-launcher` is not
   installed here, so no 0.2.0 client exists to run. The 0.1.0 runs in
   `../_shared/evidence/enderio/` (`e2e.log`, `e2e-selftest.log`, `hmod-load.log`) are
   **historical** evidence for the *same* `runtime.rs` (byte-identical) and the same rules; what
   the format changed is the transport of the calls, not the behaviour. **No 0.2.0 behaviour claim
   is made.**
2. **The registered block entity / `ITEMS` path is compile-checked, not run.** `check`/`fits` and
   the tests prove the package and its rules; whether a conduit's block entity is created on
   placement and whether a chest is seen through `LevelRef::capability` at this pin is exactly what
   the launcher-built run would show. This is the first item for the next slice.
3. Every slice-1 content gap is unchanged (filters, speed tiers, priority, dye channels, the
   screen, redstone control, the bundle, facades, wrench/probe, waterlogging, the energy/fluid/
   redstone conduits, the machines, the mobs/armour/items, the alloy system and therefore a
   recipe, the arm renderer).

`## Upstream defects fixed`: none found in this migration; the earlier deviations (state
properties instead of a block-entity renderer, commands instead of the screen, the 3 px outline)
remain deviations, not defects.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port

### Changelog (0.2.0) — first publication

> **0.2.0 — first publication, as an SDK `.hmod`.** Ender IO (slice 1, the item conduit) has never
> been submitted, so this version supersedes nothing; it is republished in Horizon's new mod
> format because the mod-API merge (#16941, launcher ≥ 0.2.70) retired `.hnmod`. One flat
> `enderio.hmod` now carries the code *and* the content (the conduit block and item, its models,
> texture, lang, loot table and creative tab): `src/lib.rs` is the one `impl mod_api::Hooks`,
> `src/support.rs` the glue, and `src/runtime.rs` is byte-identical to the retired bundle's
> `runtime.rs`, so the connection rules, the defaults, the cycle and the insert order behave
> exactly as before. Built for `api = "0.2"` against SDK nightly
> `0b14f8acfd97fa2c750e42795249dcc981178ca4` and mixins
> `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, for Minecraft 26.3. The five old injections are now
> typed hooks and declarations: `Hooks::register` declares the conduit's block entity type, a
> `BlockEntityPart` ticks it (the tick queues the conduit), `Hooks::level_tick_post` refreshes the
> connections and moves the items, and `register_commands`/`run_mod_command` serve
> `/enderio info|config` with the same readout text. The embedded-content injection (the AE2
> pattern) is gone with the format: one package, no second archive. Same feature set and the same
> gaps (filters, speed tiers, priority, dye channels, the conduit screen, facades, the wrench and
> probe, waterlogging, the energy/fluid/redstone conduits, machines, mobs and the alloy system).
> Package: 20 entries, **48,460 B**, sha256
> `322251413fa6d3723d7c8a7a798b033dabf916a1aa18c0415a88ec0d621ff1c4`; `check` and `fits` exit 0.
> No 0.2.0 in-game run: the launcher is not installed in this environment (gap 1).

## Review fixes (2026-11-18)

Source review of PR **#1252** (`var/review/1252-enderio.md`): one authorization finding and one
licence mismatch, both fixed in **0.2.1**. The first in-game run of the 0.2.x package (also done
here) additionally exposed a carried loot table in the 1.21.1 shape, fixed below with the
assertion that proves it.

| # | Finding (`file:line`) | What a player could do | Fix | Test that pins it |
|---|---|---|---|---|
| 1 | `src/support.rs:462` — level-0 `config` on any loaded conduit | a player set another player's conduit's per-side extraction/insertion (and round robin) at coordinates they chose, anywhere in the level, with no distance or access check — the persisted flags at `:564–580` disable or redirect that network's item transfers | `declare_commands` registers **both** usages with `Command::op(…, 2)`: `config` writes and `info` reads the same configuration of another player's conduit. The server refuses them below game-master permission before any listener runs. Upstream's own controls are the conduit screen and the probe, neither of which is ported, so both are operator diagnostics | `support::tests::the_conduit_commands_are_operator_only` — RED on the 0.2.0 registration (`left: 0, right: 2`, `evidence/enderio/review-test-red.log`), GREEN after |
| 2 | `mods/enderio.toml:27` + `Cargo.toml:12` — entry said Unlicense, package declared CC0-1.0 for the Rust half | — (a listing/consistency defect: the same archive told two licences) | **One licence for the whole package: `Unlicense`.** `Cargo.toml` now says `license = "Unlicense"` (was `CC0-1.0`), and `LICENSE-NOTICE.md` states the port and the carried content are released under the Unlicense — the same SPDX id the entry lists and the same text upstream's repository ships (`LICENSE` at the package root is that text). The jar's own `license="CC0"` remains recorded as the original's dedication; CC0 and the Unlicense are equivalent public-domain dedications, and nothing here adds a restriction | package-level: `Cargo.toml:13`, `LICENSE-NOTICE.md` and `LICENSE` now agree; the rebuilt package carries them |

### Carried-data defect the first run exposed (fixed, not reproduced)

The conduit's loot table still had the 1.21.1 plural list
`"conditions": [{"condition": "minecraft:survives_explosion"}]`, which 26.3 parses to **false**, so
a broken conduit dropped nothing (the campaign's documented trap,
`ports/_shared/AGENT_MOD_STRUCTURE.md` §6). It is now the singular typed condition
`"condition": {"type": "minecraft:survives_explosion"}`, like powah's carried tables.

The e2e's new section **F** proves it: a conduit is placed, the player switches to survival (a
creative break drops nothing by design), `ctl break` reports
`"drops": {"enderio:conduit": 1}` and one `enderio:conduit` item entity is on the ground. With the
0.2.0 bytes the same section fails (`"drops": {}`); that run is why F exists. Nothing was removed
from the scenario: A–E are unchanged.

### Verification (0.2.1)

| Step | Command | Result |
|---|---|---|
| unit tests | `cargo test` | **20 passed**, 0 failed (`evidence/enderio/sdk-test.log`) |
| lints | `cargo clippy --all-targets -- -D warnings` | exit 0, no warning (`evidence/enderio/sdk-clippy.log`) |
| formatting | `cargo fmt --check` | exit 0, empty (`evidence/enderio/sdk-fmt.log`) |
| package, twice | `rm -f Cargo.lock; cargo horizon-mod build` ×2, `cmp` | **byte-identical**, 49,161 B (`evidence/enderio/sdk-build{,-second}.log`) |
| registry check | `cargo horizon-mod check enderio.hmod` | **exit 0**, 49,161 B, SHA-256 `d85004c4…12f3ee` (`evidence/enderio/sdk-check.log`) |
| fits the build | `cargo horizon-mod fits enderio.hmod --build nightly` | `Ender IO: / 0b14f8a  made for`, **exit 0** (`evidence/enderio/sdk-fits.log`) |
| in-game | `ports/tools/launcher-game.sh run enderio` | `PASS: …` + `EXIT:0` (`evidence/enderio-e2e/e2e-launcher.log`) |
| control (same Nightly, no package) | `GAME=$CONTROL_GAME … bash ports/enderio/tools/e2e.sh` | `FAIL: expected 'Conduit: links west,east', got ''` + `EXIT:1` |
| selftest | `SELFTEST=1 …` | `FAIL: SELFTEST: the source chest is empty (5 stone moved), so the check is alive` + `EXIT:1` |
| log gate | `python3 ports/tools/check-e2e-logs.py enderio-e2e --strict` | `logs=3 ok=3 missing-exit=0 bad-exit=0`, exit 0 |

Harness-only change: the scenario's scratch and socket paths honour `E2E_TMP`
(`${E2E_TMP:-/tmp/opencode}`); this box's `/tmp` is a full tmpfs and the launcher wrapper exports
`E2E_TMP` under `~/.cache/horizon-e2e`. No assertion changed.

Package: `enderio.hmod`, **49,161 B**, sha256
`d85004c41de5abca2b7e9b7e8a892449c40edf0c6a835328dc95ef165e12f3ee`.

## Slice 3 — per-side redstone control (0.2.2, 2026-11-21)

**The gap line** (§"Slice boundary and gaps" item 2 above): *"the **conduit screen** (menu/slots/toggles),
the **redstone control**"*. The screen is machines-tier UI and stays a gap; the **redstone control** is the
per-side rule the ticker consults, so it is this slice — a block-entity setting and one read of the level,
not a new block, renderer or screen. `FINDINGS.md` ("Next slices" item 2) names it too.

### What upstream actually does (read, not guessed)

Repository `Team-EnderIO/EnderIO` at `befd71596adaa091ce4762b6e446629f211ededd` (2026-11-18, shallow
clone in `/tmp`, deleted after the findings; the jar is 8.2.12-beta, the commit is `main`):

| File | Behaviour |
|---|---|
| `api/io/RedstoneControl.java` | `ALWAYS_ACTIVE(0)`, `ACTIVE_WITH_SIGNAL(1)`, `ACTIVE_WITHOUT_SIGNAL(2)`, `NEVER_ACTIVE(3)`; `isActive(signal)` is `true` / `signal` / `!signal` / `false`; `isRedstoneSensitive()` only for the two signal modes |
| `content/conduits/type/item/ItemConduitConnectionConfig.java` | `DEFAULT = (insert false, extract true, …, extractRedstoneControl NEVER_ACTIVE, …)`; `canExtract` is `isExtract() && (sensitive ? isActive(signal) : control == ALWAYS_ACTIVE)` |
| `content/conduits/bundle/ConduitBundleBlockEntity.java:1045-1050` | the signal is `level.hasNeighborSignal(pos)` (plus a redstone conduit's dye channel, not ported) |
| `content/conduits/network/ConduitNetworkImpl.java:424-435` | `canExtract` decides which sides become extract connections; the ticker then only walks those |
| `gametest/…/conduits/item/ItemConduitTests.java` (`itemConduitBasicTransfer`) | with `NEVER_ACTIVE` the item stays in the source; it moves only after the same side is set to `ALWAYS_ACTIVE` |

**So a fresh conduit extracts nothing** — its extract *toggle* is on, its redstone control is
`never_active`, and `canExtract` answers false. Slice 1 reproduced the toggle and missed the gate, which
made every fresh conduit move items. That is a port error, not an upstream defect: **corrected here** (see
the table below).

### The change

- `src/runtime.rs`: `RedstoneControl` (four modes, upstream ids and names, `is_active`, `is_sensitive`,
  `named` with the four serialized names plus `always`/`with_signal`/`without_signal`/`never`);
  `ConduitConfig` gains `extract_control: [RedstoneControl; 6]` (default `never_active`) and
  `can_extract(face, has_signal)` = the toggle **and** the mode; the saved shape is now `Saved`
  (insert, extract, round robin, six control ids). A damaged id reads as `never_active`.
- `src/support.rs`: the block entity's compound gains `extractControl` (`IntArray` of six ids, face order;
  absent/wrong length/wrong type = all `never_active`, so a 0.2.1 world keeps working); `/enderio info`
  prints a third line naming the sides whose control is not the default; `/enderio config … redstone
  <mode>` sets one. The option and value are parsed **before** the conduit is looked up, so a typo cannot
  leave a half-applied write. `endpoints()` reads `LevelRef::has_neighbor_signal` once per conduit per
  cycle and gates each side's extraction through `ConduitConfig::can_extract`.
- `ConduitConfig`, not `Config`: the generated `uses.json` resolves names against the SDK surface, so the
  local `Config` was recorded as the SDK's `type:Config` (`built_at: src/runtime.rs`) and `fits` refused
  every other build ("the type `Config` can no longer be built with a literal"). The rename is the fix
  `artifacts`/`bhc` recorded on 2026-11-20; `uses.json` was not hand-edited.

### Red → green (TDD)

Tests first, with `todo!()` bodies for the new rule, so the red run names the behaviour that is missing:

```
test runtime::tests::a_fresh_conduit_extracts_nothing_until_its_redstone_control_is_set ... FAILED
test runtime::tests::a_sides_redstone_control_gates_its_extraction ... FAILED
test runtime::tests::config_round_trips_through_its_saved_masks ... FAILED
test runtime::tests::redstone_controls_keep_upstreams_ids_and_names ... FAILED
test runtime::tests::the_extract_toggle_still_wins_over_an_active_redstone_control ... FAILED
test runtime::tests::the_per_side_redstone_controls_round_trip_through_the_saved_config ... FAILED
…
test result: FAILED. 16 passed; 9 failed; …          (`evidence/enderio/sdk-test-red.log`)

thread 'runtime::tests::a_fresh_conduit_extracts_nothing_until_its_redstone_control_is_set' (205817) panicked at src/runtime.rs:195:9:
not yet implemented
```

Then the glue's own red, with the rule already green (the readout and the saved array were missing):

```
thread 'support::tests::the_config_round_trips_through_the_saved_compound' (210485) panicked at src/support.rs:648:9:
assertion `left == right` failed
  left: ConduitConfig { insert: 36, extract: 62, round_robin: true, extract_control: [NeverActive, NeverActive, NeverActive, NeverActive, NeverActive, NeverActive] }
 right: ConduitConfig { insert: 36, extract: 62, round_robin: true, extract_control: [NeverActive, ActiveWithSignal, NeverActive, NeverActive, AlwaysActive, NeverActive] }
thread 'support::tests::the_info_readout_names_the_sides_whose_redstone_control_is_not_the_default' (210486) panicked at src/support.rs:132:5:
not yet implemented
```
`evidence/enderio/sdk-glue-red.log`. Green: **27 passed, 0 failed** (`sdk-test.log`).

### Ladder (pin `0a875da` / mixins `e59da90`, every step through `build-slot.sh`, `CARGO_INCREMENTAL=0`)

| Step | Command | Result |
|---|---|---|
| runtime + glue red | `cargo test --offline` | **16 passed, 9 failed** (`sdk-test-red.log`), then **24 passed, 3 failed** (`sdk-glue-red.log`) |
| tests | `cargo test --offline` | **27 passed**, 0 failed — `sdk-test.log` |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** — `sdk-clippy.log` |
| format | `cargo fmt --check` | exit 0 — `sdk-fmt.log` |
| package ×2 | `rm -f Cargo.lock; cargo horizon-mod build`, twice, `cmp` | **byte-identical**, **53,656 B**, 20 entries, no `Cargo.lock` — `sdk-build{,-second}.log`, `sdk-entries.txt` |
| check | `cargo horizon-mod check enderio.hmod` | `enderio 0.2.2 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, **exit 0** — `sdk-check.log` |
| fits (the pin) | `cargo horizon-mod fits enderio.hmod --build 0a875da` | `0a875da  made for`, exit 0 — `sdk-fits.log` |
| fits (a different build) | `cargo horizon-mod fits enderio.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 — `sdk-fits-other.log` |
| gate | `python3 ports/tools/verify-sdk-package.py enderio` | **1/1** |
| in game | `HORIZON_E2E_INSTANCE=enderio-e2e ports/tools/launcher-game.sh build enderio` then `build-slot.sh run --step e2e -- bash ports/enderio/tools/e2e.sh` | `PASS` + `EXIT:0` — `evidence/enderio-e2e/e2e-launcher-0.2.2.log` |
| control (the same Nightly, no package) | `GAME=$CONTROL_GAME …` | `FAIL: expected 'Conduit: links west,east', got ''` + `EXIT:1` — `e2e-launcher-control-0.2.2.log` |
| fail-capability | `SELFTEST=1 …` | `FAIL: SELFTEST: the source chest is empty (5 stone moved), so the check is alive` + `EXIT:1` — `e2e-launcher-selftest-0.2.2.log` |
| log gate | `python3 ports/tools/check-e2e-logs.py enderio-e2e --strict` | `logs=6 ok=6 missing-exit=0 bad-exit=0` |

The in-game legs were run against a game the launcher built **from this package** at `0a875da`
(`native-0a875da-6b1f9c3494ef`), on the instance `enderio-e2e` (the shared `e2e` instance is still on
`native-0b14f8a-…`; repointing a shared instance is not this job's to do). The package's `uses.json`
names `made_for: 0a875da…`.

### In-game scenario (from `evidence/enderio-e2e/e2e-launcher-0.2.2.log`)

```
== A: a conduit moves 5 stone from the west chest to the east chest ==
  source before: 5, target before: 0
  Conduit: links west,east
  Conduit: redstone never_active on every side
  conduit state: enderio:conduit[down=false,east=true,north=false,south=false,up=false,west=true]
  source after: 0, target after: 5
== G: the extract side's redstone control gates extraction (active_with_signal) ==
  bad value reply: Unknown redstone control 'sometimes': always_active, active_with_signal, active_without_signal or never_active.
  unpowered: source 5, target 0
  Conduit: redstone west=active_with_signal
  powered: source 0, target 5
  unpowered again with 5 more: source 5, target 5
== H: active_without_signal is the mirror image (control) ==
  unpowered: source 0, target 5
  powered with 5 more: source 5, target 5
```

What A and G prove that slice 1's run did not: the **same rig** that moves 5 stone with `always_active`
and insertion on moves nothing with the fresh `never_active` control (45 ticks, two cycles); with
`active_with_signal` it moves nothing until a redstone block is placed above the conduit, moves all five in
the first cycle after it, and — with the refilled source — moves nothing again once the block is removed.
H gives the mirror: `active_without_signal` moves unpowered and stops when powered. Every phase is a
fresh count from `ctl container`, and each one would fail if the gate were dropped (with no gate, G's
unpowered phase moves the stone and H's powered phase does too). The control leg fails at A's first
assertion on a game without the package, so no assertion passes vacuously.

### Coverage of the jar's families

| Family | State | Where it is proved |
|---|---|---|
| the conduit block + item (texture, models, lang, tab) | **tested** | A (`setblock`, block state), F (break drops `enderio:conduit`), control leg (the id is unknown without the package) |
| six-side connection state | **tested** | A: `Conduit: links west,east` + the block-state line |
| per-side insert/extract toggles | **tested** | A (insert on/off), C (default off), D/E (movement only when on) |
| **per-side redstone control** (this slice) | **tested** | A (default `never_active` extracts nothing), G (`active_with_signal`, powered/unpowered/again), H (`active_without_signal`), bad-value refusal |
| transfer rate/cycle (4 items per 20 ticks) | **tested** | E (4 of 20 in 25 ticks) |
| `/enderio info|config` | **tested** | A, G: the parsed chat replies (`Conduit: links …`, `Conduit: redstone …`, the refusal) |
| loot table (self-drop) | **tested** | F (`drops: {"enderio:conduit": 1}` + one item entity) |
| item filters (extract/insert filter slots) | **not ported** | needs the filter items and their own configuration surface; gap 1 |
| speed tiers (enhanced/ender item conduit) | **not ported** | needs the bundle/type component; gap 1 — see the measured rate note below |
| priority, dye channels, self-feed | **not ported** | gap 1/2 |
| the conduit screen (menu/slots) | **not ported** | machines-tier UI; gap 2 (the commands stand in) |
| the bundle, facades, Yeta Wrench, Conduit Probe, waterlogging | **not ported** | gap 3 |
| energy / fluid / redstone conduits | **not ported** | gap 4 |
| machines, mobs/armour/items, the alloy system + recipe | **not ported** | gaps 5–7 |
| the arms' block-entity renderer | **not ported** | cosmetic deviation (state properties drive the model); gap 8 |
| the conduit recipe | **not ported** | needs the alloy system (`conduit_binder`, `c:ingots/conductive_alloy`); gap 7 |

### Measured, not fixed: the item conduit's rate

The jar's own conduit data says `data/enderio/enderio/conduit/item.json` → `"transfer_rate": 32` with the
`ticks_per_cycle` codec default 20, and `ItemConduitTicker` uses `transferRatePerCycle()` as its per-cycle
budget; the port still moves `SPEED = 4`, the **codec fallback for pre-8.x data** (`ItemConduit.CODEC`'s
`optionalFieldOf("transfer_rate", 4)`), which the modern jar's data overrides. That is gap 1's business
(the tier values `32`/`64`/`64 @ 10 ticks` all come from the same data), not this slice's, so it is recorded
here with the file/line and left for the tiers slice rather than smuggled in with the redstone change.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, verification-before-completion; BeautifulRust/rust-best-practices

### Changelog (0.2.2)

> **0.2.2 — per-side redstone control, and the fresh-conduit default corrected.** Every side's extraction
> now obeys an upstream `RedstoneControl` (`always_active`, `active_with_signal`, `active_without_signal`,
> `never_active`) saved beside the insert/extract toggles and read from the conduit's neighbour signal. A
> fresh conduit no longer moves items: upstream's default control is `never_active`, whose `canExtract`
> answers false — set the source side's control (`/enderio config <x> <y> <z> <side> redstone
> always_active`) and the destination's insert. `/enderio info` prints the sides whose control is not the
> default. Republished at SDK `0a875da…` (mixins `e59da90…`, unchanged) for Minecraft 26.3; `runtime.rs`'s
> tests grew from 10 to 15 and the glue's from 10 to 12 (27 total). Package: 20 entries, **53,656 B**,
> sha256 `50b385b1b648cfd837394b7444ac0e79e33fe1762601ddd4b579f99eaba516c0`; `check` exit 0, `fits
> 0a875da` **made for**, `fits 0b14f8a` **fits**.
