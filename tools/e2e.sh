#!/usr/bin/env bash
# In-game check for Ender IO slice 1: the item conduit.
#
# The End arena (stone floor at y=99, everything at y=100): an empty level keeps
# ambient item entities out of the container counts.
#
#  A  chest (0) - conduit (1) - chest (2), insert enabled on the conduit's east
#     side: 5 stone leave the source chest and arrive in the target chest, and
#     the conduit's state shows east/west connected;
#  B  chest (8) .. chest (10) with no conduit between them: nothing moves
#     (control: the conduit is what moves items);
#  C  chest (16) - conduit (17) - chest (18) with the default configuration
#     (insert off, as upstream's ItemConduitConnectionConfig.DEFAULT): nothing
#     moves although the conduit is connected (control: insertion is configured,
#     not automatic);
#  D  chest (24) - conduit (25) - chest (26, full of dirt), insert enabled:
#     nothing moves because the target has no room (control: a refused item
#     stays in its slot).
#
# SELFTEST=1 flips A's assertion (the source must still hold its stone), so the
# scenario must FAIL; that proves the check can fail.
#
# FORMAT (2026-11-18): the port is one flat SDK package, so the code and the
# content travel together — there is no `--dev-mod` folder, no embedded content
# and no second archive to compare. `GAME` must be a game the **launcher** built
# from this package (`horizon-launcher native-build ports/enderio --instance <i>
# --trust-native-code`, the mod installed in that instance); an SDK `.hmod` with
# native code cannot load on an unmodded game. `CONTROL_GAME=` (optional) is a
# launcher-built game **without** this package: the same ids must be unknown
# there, so the run cannot pass vacuously. The 0.1.0 `.hnmod` runs in
# `ports/_shared/evidence/enderio/` are historical evidence for the same rules.
#
# Every mod command is typed with `ctl chat "/cmd"` because `ctl command` runs
# the console path and bypasses the native `run_mod_command` hook.
set -euo pipefail
export LC_ALL=C

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/enderio
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}
GAME=${GAME:-}
CONTROL_GAME=${CONTROL_GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/enderio-e2e.sock}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
WORLD=EnderIOE2E

[ -n "$GAME" ] || {
    echo "missing GAME: a launcher-built horizon-client carrying this package" >&2
    exit 1
}

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/enderio-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 80 >&2 || true; exit 1; }

# The full state of a block in the End (`enderio:conduit[east=true,...]`).
state() { ctl block "$1" "$2" "$3" | jq -r '.state // .block // "unknown"'; }
# count X Y Z ITEM: how many of ITEM the container at X,Y,Z holds.
count() {
    ctl container "$1" "$2" "$3" \
        | jq -r --arg i "$4" '[.slots[] | select(.item == $i) | .count] | add // 0'
}
cleanup() {
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
}
finish() { local rc=$?; cleanup; echo "EXIT:$rc"; return $rc; }
trap finish EXIT

launch() {
    local game=$1
    rm -f "$SOCK"
    cd "$SCRATCH"
    "$CTL" --socket "$SOCK" launch \
        --game "$game" --world "$WORLD" --seed 20261105 --creative --headless --wait \
        --resources "$RES" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

# A stone floor at y=99 in the End with clear air above, the player on it.
arena() {
    ctl command "execute in minecraft:the_end run tp @s 0.5 100 3.5"
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5
        if [ "$(ctl block 0 99 0 | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill -8 99 -8 34 99 8 minecraft:stone"
    ctl command "execute in minecraft:the_end run fill -8 100 -8 34 108 8 minecraft:air"
    ctl command "execute in minecraft:the_end run kill @e[type=!player]"
    ctl wait 20
}

block_at() { # block_at X Z BLOCK
    ctl command "execute in minecraft:the_end run setblock $1 100 $2 $3"
}

# The control leg: a launcher-built game without this package must not know the
# ids. It runs first, in its own world, so the modded legs below start from a
# clean scratch home.
if [ -n "$CONTROL_GAME" ]; then
    [ -x "$CONTROL_GAME" ] || { echo "missing control horizon-client: $CONTROL_GAME" >&2; exit 1; }
    echo "== control: the same ids are unknown without this package =="
    launch "$CONTROL_GAME"
    ctl command "execute in minecraft:the_end run setblock 0 100 0 enderio:conduit" >/dev/null
    ctl wait 4
    control_block=$(ctl block 0 100 0 | jq -r '.name // .state // .block // "unknown"')
    control_reply=$(ctl messages 10 | jq -r '.chat[]' | grep -m1 'Unknown block type' || true)
    echo "  setblock reply: $control_reply"
    echo "  conduit there: $control_block"
    case "$control_reply" in
        "Unknown block type 'enderio:conduit'"*) ;;
        *) fail "the control game accepted enderio:conduit: the ids are not this package's" ;;
    esac
    case "$control_block" in
        *enderio:conduit*) fail "the control game has a conduit at 0,100,0: the ids are not this package's" ;;
    esac
    ctl quit >/dev/null 2>&1 || true
    rm -rf "$SCRATCH"
    mkdir -p "$HORIZON_HOME" "$HORIZON_LAUNCHER_HOME" "$XDG_DATA_HOME" "$XDG_CONFIG_HOME"
fi

echo "== launch (The End arena, one SDK package: code + content) =="
launch "$GAME"
arena

echo "== A: a conduit moves 5 stone from the west chest to the east chest =="
block_at 0 0 minecraft:chest
block_at 1 0 enderio:conduit
block_at 2 0 minecraft:chest
ctl wait 4
ctl container 0 100 0 minecraft:stone 5 >/dev/null
ctl wait 4
echo "  source before: $(count 0 100 0 minecraft:stone), target before: $(count 2 100 0 minecraft:stone)"
[ "$(count 0 100 0 minecraft:stone)" = 5 ] || fail "the source chest did not take 5 stone"
say "enderio info 1 100 0"
ctl wait 4
link_line=$(ctl messages 10 | jq -r '.chat[]' | grep -m1 'Conduit: links' || true)
echo "  $link_line"
case "$link_line" in
    "Conduit: links west,east") ;;
    *) fail "expected 'Conduit: links west,east', got '$link_line'" ;;
esac
conduit_state=$(state 1 100 0)
echo "  conduit state: $conduit_state"
case "$conduit_state" in
    *"east=true"*"west=true"*) ;;
    *) fail "the conduit's state does not show its west and east connections: $conduit_state" ;;
esac
# Before insertion is enabled nothing may move: upstream's default inserts
# nowhere.
ctl wait 45
[ "$(count 2 100 0 minecraft:stone)" = 0 ] \
    || fail "the conduit inserted before insertion was enabled"
say "enderio config 1 100 0 east insert on"
ctl wait 45
echo "  source after: $(count 0 100 0 minecraft:stone), target after: $(count 2 100 0 minecraft:stone)"
if [ "$SELFTEST" = 1 ]; then
    echo "(selftest: asserting A's source still holds its 5 stone)"
    [ "$(count 0 100 0 minecraft:stone)" = 5 ] \
        || fail "SELFTEST: the source chest is empty (5 stone moved), so the check is alive"
else
    [ "$(count 0 100 0 minecraft:stone)" = 0 ] \
        || fail "the source chest still holds $(count 0 100 0 minecraft:stone) stone"
    [ "$(count 2 100 0 minecraft:stone)" = 5 ] \
        || fail "the target chest holds $(count 2 100 0 minecraft:stone) stone, not 5"
fi

echo "== B: two chests with no conduit between move nothing (control) =="
block_at 8 0 minecraft:chest
block_at 10 0 minecraft:chest
ctl wait 4
ctl container 8 100 0 minecraft:stone 5 >/dev/null
ctl wait 45
echo "  source: $(count 8 100 0 minecraft:stone), target: $(count 10 100 0 minecraft:stone)"
[ "$(count 8 100 0 minecraft:stone)" = 5 ] || fail "B's source lost stone without a conduit"
[ "$(count 10 100 0 minecraft:stone)" = 0 ] || fail "B's target gained stone without a conduit"

echo "== C: a connected conduit with the default config moves nothing (control) =="
block_at 16 0 minecraft:chest
block_at 17 0 enderio:conduit
block_at 18 0 minecraft:chest
ctl wait 4
ctl container 16 100 0 minecraft:stone 5 >/dev/null
ctl wait 45
echo "  source: $(count 16 100 0 minecraft:stone), target: $(count 18 100 0 minecraft:stone)"
echo "  conduit state: $(state 17 100 0)"
[ "$(count 16 100 0 minecraft:stone)" = 5 ] || fail "C moved stone with insertion off"
[ "$(count 18 100 0 minecraft:stone)" = 0 ] || fail "C's target gained stone with insertion off"

echo "== D: a full target refuses the items (control) =="
block_at 24 0 minecraft:chest
block_at 25 0 enderio:conduit
block_at 26 0 minecraft:chest
ctl wait 4
ctl container 24 100 0 minecraft:stone 5 >/dev/null
# `ctl container` fills one empty slot a call, so 27 calls fill all of a
# single chest (27 x 64 dirt) and leave the target with no room at all.
for _ in $(seq 1 27); do ctl container 26 100 0 minecraft:dirt 64 >/dev/null; done
[ "$(ctl container 26 100 0 | jq '[.slots[] | select(. == null)] | length')" = 0 ] \
    || fail "D's target chest is not full"
say "enderio config 25 100 0 east insert on"
ctl wait 45
echo "  source: $(count 24 100 0 minecraft:stone), target stone: $(count 26 100 0 minecraft:stone), target dirt: $(count 26 100 0 minecraft:dirt)"
[ "$(count 24 100 0 minecraft:stone)" = 5 ] || fail "D's source lost stone into a full chest"
[ "$(count 26 100 0 minecraft:stone)" = 0 ] || fail "D's full target took stone"

echo "== E: the rate is a few items per cycle, not the whole stack (control) =="
block_at 32 0 minecraft:chest
block_at 33 0 enderio:conduit
block_at 34 0 minecraft:chest
ctl wait 4
ctl container 32 100 0 minecraft:stone 20 >/dev/null
say "enderio config 33 100 0 east insert on"
ctl wait 25
moved=$(count 34 100 0 minecraft:stone)
echo "  after 25 ticks: source $(count 32 100 0 minecraft:stone), moved $moved of 20"
[ "$moved" -ge 4 ] || fail "E moved $moved items in a cycle, expected at least 4"
[ "$moved" -lt 20 ] || fail "E moved the whole stack in one go: no per-cycle limit"

echo "PASS: a conduit handed 5 stone from one chest to another only after its east side was set to insert, its state showed both connections, a 20-stone source moved only its first few items per cycle, and chests with no conduit / an unconfigured conduit / a full target moved nothing"
