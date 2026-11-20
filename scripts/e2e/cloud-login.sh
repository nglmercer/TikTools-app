#!/usr/bin/env bash
# Start one headless workspace and sign it in to the local backend with an
# email/password account, without a browser. Leaves the host running.
#
# Usage: scripts/e2e/cloud-login.sh <home> <ipc-name> <email> <password> [backend-origin]
#   TT_BIN           tiktools CLI (default: target/debug/tiktools)
#   TT_DEV_PLUGINS   development plugins directory (default: .dev-plugins)
# Prints the host pid and the signed-in state. Cookies go to <home>/cookies.txt.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
bin="${TT_BIN:-$root/target/debug/tiktools}"
plugins="${TT_DEV_PLUGINS:-$root/.dev-plugins}"
home="$1"; name="$2"; email="$3"; password="$4"; origin="${5:-http://127.0.0.1:3000}"
mkdir -p "$home"
jar="$home/cookies.txt"
export TIKTOOLS_HOME="$home" TIKTOOLS_IPC_NAME="$name"

TIKTOOLS_CLOUD_ORIGIN="$origin" TIKTOOLS_DEV_PLUGINS_DIR="$plugins" \
  nohup timeout 3600 "$bin" host --ipc >"$home/host.log" 2>&1 &
pid=$!
for _ in $(seq 1 60); do "$bin" system info >/dev/null 2>&1 && break; sleep 0.5; done
"$bin" system info >/dev/null 2>&1 || { echo "host did not start; see $home/host.log"; exit 1; }

# 1. Email/password sign-in on the backend (session cookie kept in the jar).
signin=$(curl -s -c "$jar" -o "$home/signin.json" -w "%{http_code}" -H "Origin: $origin" \
  -H "Content-Type: application/json" \
  -d "$(python3 -c 'import json,sys;print(json.dumps({"email":sys.argv[1],"password":sys.argv[2]}))' "$email" "$password")" \
  "$origin/api/auth/sign-in/email")
[ "$signin" = "200" ] || { echo "sign-in failed with HTTP $signin"; exit 1; }

# 2. The host starts a native login; the backend issues a handoff for the same
#    session; the callback delivers the code to the host's loopback listener.
"$bin" api call cloud.login.begin >"$home/begin.json"
read -r redirect challenge state < <(python3 - "$home/begin.json" <<'PY'
import json,sys,urllib.parse
url=json.load(open(sys.argv[1]))["browserUrl"]
q=urllib.parse.parse_qs(urllib.parse.urlparse(url).query)
print(q["redirectUri"][0], q["codeChallenge"][0], q["state"][0])
PY
)
handoff=$(curl -s -b "$jar" -o "$home/handoff.json" -w "%{http_code}" -H "Origin: $origin" \
  -H "Content-Type: application/json" \
  -d "$(python3 -c 'import json,sys;print(json.dumps({"redirectUri":sys.argv[1],"codeChallenge":sys.argv[2],"state":sys.argv[3]}))' "$redirect" "$challenge" "$state")" \
  "$origin/api/v1/desktop/handoff")
[ "$handoff" = "200" ] || { echo "handoff failed with HTTP $handoff"; exit 1; }

# 3. Wait for the host to receive the code, then deliver the callback.
"$bin" api call cloud.login.complete >"$home/complete.json" 2>&1 &
complete_pid=$!
sleep 1
callback=$(python3 -c "import json;print(json.load(open('$home/handoff.json'))['callbackUrl'])")
curl -s -o /dev/null "$callback"
wait "$complete_pid" || { echo "login completion failed; see $home/complete.json"; exit 1; }

state_out=$("$bin" api call cloud.status)
echo "$state_out" | python3 -c "import json,sys;d=json.load(sys.stdin);print('signed-in' if d.get('state')=='signed-in' else 'NOT signed in', d.get('email'))"
echo "host pid=$pid home=$home name=$name"
