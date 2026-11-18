# Licence provenance

- Mod: `enderio` — original: <https://github.com/Team-EnderIO/EnderIO> (Ender IO).
- Original author(s): CrazyPants, tterrag, HenryLoenwind, MatthiasM, CyanideX, EpicSquid,
  Rover656, HypherionSA, liliandev, Ferri_Arnus, dphaldes.
- Original jar this port was written against: `enderio-8.2.12-beta.jar` (6,598,496 B,
  SHA-256 `5fe098153dddc2e7ccd9d38b0341dbc61f3072150958c916e5ef63f395e4eab1`), the exact
  ATM10 pack file. Its `META-INF/neoforge.mods.toml` declares **`license="CC0"`**, and the
  repository's `LICENSE.txt` is the **Unlicense** text.
- Licence text read from: the jar's `META-INF/neoforge.mods.toml` and
  <https://github.com/Team-EnderIO/EnderIO/blob/dev/1.21.1/LICENSE.txt> (the repository's
  public-domain dedication). Both are public-domain dedications → campaign licence rank 0.
- This port is **CC0-1.0** (`Cargo.toml`), © 2026 the Horizon port authors, who dedicate it to
  the public domain as the original does. `LICENSE` at the package root is the Unlicense text
  the original ships.
- Carried content provenance (every file, its source path in the jar and any change):
  [`../_shared/carry-work/enderio/SOURCE.md`](../_shared/carry-work/enderio/SOURCE.md).

## What travels in the package

`enderio.hmod` is one flat SDK package: `horizon-mod.toml`, `Cargo.toml`, `src/lib.rs`,
`src/support.rs`, `src/runtime.rs`, the carried `assets/enderio/` and `data/enderio/`, and
**`LICENSE` + `LICENSE-NOTICE.md` at the package root** (the builder packs root files beside
the code). The retired `.hnmod` shipped the notice inside its embedded content archive; one
package needs no such detour.

## Local-only

The port stays **local-only until the maintainer approves publishing**
(`ARCHITECTURE.md` §8). Rank 0 carries no downstream obligation, but the campaign's approval
is still required before any submission.
