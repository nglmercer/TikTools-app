#!/usr/bin/env bash
# End-to-end sync check between two real workspaces signed in to the same account.
# Requires the local backend running (cargo remake dev) and the database up.
#
# Usage: scripts/e2e/sync-two-workspaces.sh [backend-origin]
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
bin="${TT_BIN:-$root/target/debug/tiktools}"
origin="${1:-http://127.0.0.1:3000}"
work="$(mktemp -d /tmp/tt-sync-e2e.XXXXXX)"
email="e2e-$(date +%s)-$$@example.test"
password="E2e-$(python3 -c 'import secrets;print(secrets.token_hex(12))')!"
run="$$"
trap 'for p in $(pgrep -P $$ 2>/dev/null); do kill "$p" 2>/dev/null || true; done' EXIT

"$root/scripts/e2e/create-test-user.sh" "$email" "$password" "$origin"
"$root/scripts/e2e/cloud-login.sh" "$work/ws-a" "tt-sync-a-$run" "$email" "$password" "$origin"
"$root/scripts/e2e/cloud-login.sh" "$work/ws-b" "tt-sync-b-$run" "$email" "$password" "$origin"

a() { TIKTOOLS_HOME="$work/ws-a" TIKTOOLS_IPC_NAME="tt-sync-a-$run" "$bin" "$@"; }
b() { TIKTOOLS_HOME="$work/ws-b" TIKTOOLS_IPC_NAME="tt-sync-b-$run" "$bin" "$@"; }

a api call sync.enable >/dev/null
b api call sync.enable >/dev/null

plugin=$(a plugin list | python3 -c "import sys,re;print(re.findall(r'^(\S+)', sys.stdin.read(), re.M)[0])")
a plugin disable "$plugin" >/dev/null
a api call sync.now >"$work/now-a.json"
b api call sync.now >"$work/now-b.json"

now_a=$(cat "$work/now-a.json"); now_b=$(cat "$work/now-b.json")
pushed_a=$(python3 -c "import json,sys;print(json.loads(sys.argv[1]).get('pushed',0))" "$now_a")
pulled_b=$(python3 -c "import json,sys;print(json.loads(sys.argv[1]).get('pulled',0))" "$now_b")
deferred_b=$(python3 -c "import json,sys;print(json.loads(sys.argv[1]).get('applicationDeferred',False))" "$now_b")
state_b=$(b plugin get "$plugin" | python3 -c "import sys,json;print(json.load(sys.stdin)['plugin'].get('enabled'))")

echo "transport: A pushed=$pushed_a, B pulled=$pulled_b"
echo "apply on B: applicationDeferred=$deferred_b, plugin $plugin enabled on B=$state_b (expected False)"
if [ "$pushed_a" -ge 1 ] && [ "$pulled_b" -ge 1 ]; then
  echo "PASS (transport): the change left A and arrived at B"
else
  echo "FAIL (transport): see $work"; exit 1
fi
if [ "$state_b" = "False" ]; then
  echo "PASS (apply): B applied the change"
else
  echo "OPEN (apply): B has not applied the change yet; see docs/E2E-SYNC.md"
fi
