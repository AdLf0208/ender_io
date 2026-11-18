# Ender IO → Horizon: upstream findings

Ported from **Ender IO 8.2.12-beta** (`Team-EnderIO/EnderIO`, **CC0** in the
jar's `mods.toml`, Unlicense text in the repo), Minecraft 1.21.1 NeoForge, the
exact ATM10 jar (see `ports/_shared/carry-work/enderio/SOURCE.md`).

Behaviour was read from the public repository at commit
`8989fcebda3eb9e678fea4138f577c47a22a5e8d` (2026-11-04, shallow clone in
`ports/_shared/upstream/enderio`). The clone is analysis only; this port is a
clean-room Rust rewrite, and no Java was copied or translated. The jar is
8.2.12-beta while the clone is `main`; the conduit code paths read here
(`ItemConduit`, `ItemConduitTicker`, `ItemConduitConnectionConfig`,
`ConduitBundleBlock`, `ConduitNetworkImpl`'s tick schedule) are the same code
the jar ships, but the note is honest that the tag was not checked out.

## Files read

- `enderio/src/main/java/com/enderio/enderio/content/conduits/type/item/ItemConduit.java`
- `.../type/item/ItemConduitTicker.java`
- `.../type/item/ItemConduitConnectionConfig.java`
- `.../conduits/network/ConduitNetworkImpl.java` (`addTickingNode`, `getTickableConduits`)
- `.../api/conduits/ticker/ConduitTickerBase.java`,
  `.../api/conduits/connection/path/SpeedAndTickRatePair.java`
- `.../conduits/bundle/ConduitBundleBlock.java`, `.../legacy/DynamicConnectionState.java`
- `init/EIOBlocks.java` (the conduit bundle's block properties), `init/EIOConduits.java`
- the jar's `assets/enderio/**` conduit models, blockstate, texture and lang.

## Connection rules

- **To another conduit**: `canConnectToConduit(other)` is
  `ConduitsConfig.COMMON.CAN_MIX_ITEM_CONDUIT_TIERS` — conduits of the same
  type (the item-conduit tiers) join into one network when mixing is allowed.
- **To a block**: `canConnectToBlock(...)` is "the block has an
  `IItemHandler` on that side" (`capabilityAccessor.getSidedCapability(...)`).
  Air, stone and a block whose container is not reachable from the touching
  side do not connect.
- **When it is recomputed**: `shouldCheckConnectionsOnNeighborChange()` is
  `false` for item conduits; `ConduitBundleBlock.neighborChanged` and
  `updateShape` call `ConduitBundleBlockEntity.updateConnections`, and the
  bundle recomputes its shape from the stored connections. The drawn state is
  the block entity's, not a vanilla block-state property.
- A conduit bundle is **one block** (`enderio:conduit`) that holds any number
  of conduit types; the item is a `ConduitBlockItem` whose component names the
  type (`enderio:conduit: "enderio:item"`).

## Per-side configuration (upstream defaults)

`ItemConduitConnectionConfig.DEFAULT`:

| Field | Default | Meaning |
|---|---|---|
| `isInsert` | **false** | a connected side takes items only when the player enables it |
| `isExtract` | **true** | every connected side pulls by default |
| `insertChannel` / `extractChannel` | `GREEN` | dye channels (not in slice 1) |
| `extractRedstoneControl` | `NEVER_ACTIVE` | no redstone gating unless set |
| `isRoundRobin` | **false** | an extract side always tries its insert sides in path order |
| `isSelfFeed` | **false** | a conduit does not put items back into the inventory it took them from |
| `priority` | `0` | ordering between extract paths (not in slice 1) |

`reconnected()` keeps `isInsert = false, isExtract = true`, so a freshly
connected inventory extracts and inserts nowhere. A two-chest conduit therefore
moves nothing until the destination side's **insert** is switched on — that is
the behaviour this slice reproduces, and its e2e proves it.

## Transfer

`ItemConduitTicker.tickNetwork` → `tickChannel`, per dye channel:

1. For each **extract connection** of the network (every connected side with
   `canExtract`): its insert paths are the network's connected sides with
   `canInsert` on the same channel.
2. `speed = extractConduit.transferRatePerCycle()` — the item conduit's
   `transfer_rate`, **default 4**.
3. Walk the extract inventory's slots in order; take up to `speed` items from
   each (capped to the item's max stack size; filters, if any).
4. Insert into the first insert path that takes them: start index 0, or the
   per-side round-robin cursor when `isRoundRobin`; each path's own cap is its
   `SpeedAndTickRatePair` adjusted speed. The cursor advances to
   `senderIndex + 1` after a successful insert.
5. A path equal to the extract connection itself is skipped unless
   `isSelfFeed`; when `extracted >= speed`, the extract connection is done.
6. `isEmpty(extractHandler, i + 1)` — once the remaining slots are empty, the
   extract connection is done for the cycle.

**Rate and cycle**: `ItemConduit`'s `ticks_per_cycle`, default **20**. A
network ticks a conduit only when `i % tickRate == 0` over the `(gameTime +
tickOffset) % 20` phase, i.e. the base item conduit moves its items **once a
second**, up to 4 items per extract side per cycle (the tooltip's "Effective
Rate: 4 Items/sec"). `tickOffset` spreads networks over the 20 ticks.

**Network shape**: a network is the set of conduits reachable through other
conduits, and the inventories are its endpoint connections. `native_move_items`
within one container answers 0, so a network never shuffles a container into
itself.

## The block

`EIOBlocks.CONDUIT_BUNDLE`: `strength(1.5f, 10)`, `noLootTable()` (the item
comes from `getCloneItemStack`/code), `noOcclusion()`, `dynamicShape()`,
`mapColor(MapColor.STONE)`, waterloggable, pistons cannot push it
(`PushReaction.BLOCK`). Its shape is the bundle's core and connection arms
(3 px core, 2 px arms), computed from the connections.

## This slice's mapping

| Upstream | Here | Why |
|---|---|---|
| Per-side config in the node data | the block entity's saved compound (`insert`, `extract`, `roundRobin` ints) | the pin saves a mod block entity's compound with the chunk |
| Screen to toggle insert/extract | `/enderio config <pos> <side> <insert\|extract\|round> on\|off` | slice 1 has no screen (machines/deep work); the toggle itself is the upstream data |
| `isRoundRobin` cursor in node data | the cycle number, derived from the level's game time | a stateless cursor that survives restarts; same rotation |
| Connections in the bundle's shape | six boolean block-state properties driving a multipart model | the pin has no block-entity renderer in this tier; `docs/mods.md` names exactly this ("how a pipe says which sides it connects to") |
| Network tick offset per network | cycle phase from the lowest conduit's position | same spreading, no saved state |
| Channels, redstone control, priority, filters, tiers | not implemented | slice 2+ (see the gap list in `notes.md`) |

## Next slices (planned gaps, not in this port)

1. **Filters and speed tiers**: the extract/insert filter slots
   (`EXTRACT_FILTER_SLOT = 0`, `INSERT_FILTER_SLOT = 1`), the filter item
   capability, and the enhanced/vibrant item conduits (`transfer_rate` 8/16? —
   the tier values come from the conduit data files at
   `data/enderio/enderio/conduit/*.json`), with `canConnectToConduit` mixing.
2. **The conduit screen** (menu + slots + toggles) replacing the `/enderio`
   command; redstone control and dye channels.
3. **The bundle**: several conduit types in one block, facades (painted
   blocks), the Yeta Wrench and Conduit Probe; waterlogging.
4. **Energy, fluid and redstone conduits** (their own tickers, tanks,
   `block_entity_energy`/`block_entity_fluids`).
5. **The machines** (alloy smelter, sag mill, Stirling generator, obelisks,
   soul binder, …) and their menus and screens.
6. **Mobs, armour and items** (the ender armour/weapons, staffs, the
   electromagnet, soul vials, the enderios, the broken spawner, endermen
   head/skulls, the XP vacuum).
7. **Recipes**: the conduit recipe needs `enderio:conduit_binder` and the
   `c:ingots/conductive_alloy`/`c:ingots/pulsating_alloy` tags from the alloy
   system; carry it with the alloy slice.
