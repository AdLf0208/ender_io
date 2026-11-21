# Validation

> **Current (0.2.2, `0a875da`, 2026-11-21):** the slice-3 (per-side redstone control) ladder is in
> [`notes.md`](notes.md#slice-3--per-side-redstone-control-022-2026-11-21); the package's evidence is
> `../_shared/evidence/enderio/sdk-*.log` and the in-game legs
> `../_shared/evidence/enderio-e2e/e2e-launcher{,-control,-selftest}-0.2.2.log`.
>
> | Check | Result |
> |---|---|
> | `cargo test --offline` | **27 passed**, 0 failed (red first: 16/9, then 24/3) |
> | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings |
> | `cargo fmt --check` | exit 0 |
> | `cargo horizon-mod build` ×2, `cmp` | byte-identical, **53,656 B**, 20 entries, no `Cargo.lock` |
> | `cargo horizon-mod check enderio.hmod` | exit 0, `enderio 0.2.2 … 0a875da…/e59da90…` |
> | `cargo horizon-mod fits … --build 0a875da` | `0a875da  made for`, exit 0 |
> | `cargo horizon-mod fits … --build 0b14f8a` (a different build) | `0b14f8a  fits`, exit 0 |
> | `verify-sdk-package.py enderio` | 1/1 |
> | in game (launcher-built, `enderio-e2e` instance) | `PASS` + `EXIT:0` |
> | control (same Nightly, no package) | `FAIL: expected 'Conduit: links west,east', got ''` + `EXIT:1` |
> | `SELFTEST=1` | `FAIL: SELFTEST: the source chest is empty …` + `EXIT:1` |
> | `check-e2e-logs.py enderio-e2e --strict` | `logs=6 ok=6` |

> **Historical (0.1.0, `.hnmod`):** the sections below record the retired format's ladder and its
> in-game runs. The 0.2.0 SDK package's ladder is in
> [`notes.md`](notes.md#migration-to-the-sdk-format-020-2026-11-18) and
> `../_shared/evidence/enderio/sdk-*.log`; no 0.2.0 in-game run was possible here (the launcher
> is not installed).

Validated on Linux with Rust 1.97.0 against the exact revisions in
`compatibility.json`:
Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`, horizon-mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57` (schema 6). Every build, test and
e2e step ran through `ports/tools/build-slot.sh` with `CARGO_INCREMENTAL=0`;
the in-game runs used the pin-matched shared `horizon-ctl`.

| Check | Result |
|---|---|
| `cargo fmt --all --check` | Pass |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | Pass (0 warnings) |
| `cargo test --locked --offline` (standalone) | **10 passed**, 0 failed (RED first: 104 compile errors) |
| `tools/horizon.py stage` | `status: ok`, 4 injections, 2 modules |
| `tools/horizon.py check` | `cargo_exit: 0`, **0 warnings** |
| `tools/horizon.py test` | **10 passed** (`game::enderio::tests`, 1240 filtered) |
| `tools/horizon.py build` (fast) | `cargo_exit: 0` |
| `tools/e2e.sh` | `PASS` + `EXIT:0` (ctl transcript in notes.md) |
| `SELFTEST=1 tools/e2e.sh` | **exit 1**, `FAIL: SELFTEST: the source chest is empty …` (fail-capable) |
| `tools/package.sh` | `.hmod` 24,185 B / 12 entries, `.hnmod` 10,839 B / 8 entries |
| `tools/hmod-load.sh` | `PASS` + `EXIT:0`; negative control: `Unknown block type 'enderio:conduit'` |

Evidence: `../_shared/evidence/enderio/` (runtime RED/GREEN, check/test/build,
e2e, selftest, packaging, load check).

## Re-verification at the campaign pin (2026-11-09, mode B)

Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9` (game.rs `64813cf4…`),
horizon-mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57` (schema 6), through
`ports/tools/build-slot.sh`, `CARGO_INCREMENTAL=0`.

| Check | Result |
|---|---|
| fmt / clippy | exit 0 / exit 0, 0 warnings |
| `cargo test --locked --offline` | 10 passed, 0 failed |
| `horizon.py stage` / `check` / `test` | exit 0 / `cargo_exit: 0`, 0 warnings / 10 passed, 1240 filtered |
| `horizon.py build` (in the e2e lease) | `cargo_exit: 0`, fast profile |
| e2e default | `PASS` + `EXIT:0` |
| e2e `SELFTEST=1` | `FAIL` + `EXIT:1` |
| packaging twice | `content/enderio.hmod` 24,185 B `07d1a3c8…edfd3` (12 entries); `enderio.hnmod` 10,839 B `a9605c4c…c46fa` (8 entries) — identical in both passes |

Evidence: `ports/_shared/evidence/enderio/{e2e.log,e2e-selftest.log,package-second.log}`.

## Embedded content + re-verification (2026-11-10)

Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9` (game.rs `64813cf4…`),
horizon-mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57` (schema 6), every step
through `ports/tools/build-slot.sh` with `CARGO_INCREMENTAL=0`.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0, 0 warnings |
| `cargo test --locked --offline` | green |
| `horizon.py stage` / `check` / `test` | exit 0 / `cargo_exit: 0`, 0 warnings / embedded tests green |
| `tools/package.sh` | `content/enderio.hmod` 24,185 B `07d1a3c8af0236a8…`, `enderio.hnmod` 35,163 B `20aaca058ba36cd5…` (10 entries, embeds the content) |
| e2e (no `--dev-mod`) | default `PASS` + `EXIT:0`; `SELFTEST=1` `FAIL` + `EXIT:1` |
| `upstream_ready.py enderio` | **READY (exit 0)** |
