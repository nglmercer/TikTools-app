#!/usr/bin/env bash
# Start two isolated headless TikTools workspaces on this machine and prove each
# one answers from its own home. Phase 1 of the two-workspace sync E2E.
#
# Usage: scripts/e2e/two-workspaces.sh [--keep]
#   TT_BIN   path to the tiktools CLI (default: target/debug/tiktools)
#   TT_WORK  scratch directory (default: a new mktemp directory)
#
# Each workspace needs BOTH TIKTOOLS_HOME and TIKTOOLS_IPC_NAME set to the same
# values for every client call; the host and the client must agree on them.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
bin="${TT_BIN:-$root/target/debug/tiktools}"
work="${TT_WORK:-$(mktemp -d /tmp/tt-e2e.XXXXXX)}"
keep=0
[ "${1:-}" = "--keep" ] && keep=1

[ -x "$bin" ] || { echo "tiktools CLI not found at $bin (run: cargo build -p tiktools-cli)"; exit 1; }

run_id="$$"
names=("tt-e2e-A-$run_id" "tt-e2e-B-$run_id")
homes=("$work/ws-a" "$work/ws-b")
pids=()

stop_hosts() {
  for pid in "${pids[@]:-}"; do kill "$pid" 2>/dev/null || true; done
  [ "$keep" = 1 ] || rm -rf "$work"
}
trap stop_hosts EXIT

call() { # call <index> <method> [params-json]
  local i=$1; shift
  TIKTOOLS_HOME="${homes[$i]}" TIKTOOLS_IPC_NAME="${names[$i]}" \
    "$bin" api call "$@"
}

for i in 0 1; do
  mkdir -p "${homes[$i]}"
  TIKTOOLS_HOME="${homes[$i]}" TIKTOOLS_IPC_NAME="${names[$i]}" \
    "$bin" host --ipc >"$work/host-$i.log" 2>&1 &
  pids+=("$!")
done

# Wait until both answer, bounded so a broken start fails fast.
for i in 0 1; do
  ok=0
  for _ in $(seq 1 60); do
    if call "$i" system.info >"$work/info-$i.json" 2>/dev/null; then ok=1; break; fi
    sleep 0.5
  done
  [ "$ok" = 1 ] || { echo "workspace $i did not answer; see $work/host-$i.log"; exit 1; }
done

home_a=$(python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['home'])" "$work/info-0.json")
home_b=$(python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['home'])" "$work/info-1.json")
pid_a=$(python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['pid'])" "$work/info-0.json")
pid_b=$(python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['pid'])" "$work/info-1.json")

if [ "$home_a" = "$home_b" ] || [ "$pid_a" = "$pid_b" ]; then
  echo "FAIL: the two workspaces are not isolated (homes: $home_a / $home_b)"
  exit 1
fi
[ "$home_a" = "${homes[0]}" ] && [ "$home_b" = "${homes[1]}" ] || {
  echo "FAIL: a workspace answered from an unexpected home"; exit 1; }

echo "OK: two isolated workspaces"
echo "  A home=$home_a pid=$pid_a"
echo "  B home=$home_b pid=$pid_b"
echo "Phase 2 (sign-in and sync) is not automated yet; see scripts/e2e/README.md."
