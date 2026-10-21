#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
echo 'TikTools Termux Doctor'
uname -sm
for tool in rustc cargo clang cmake pkg-config protoc python; do
  if command -v "$tool" >/dev/null; then
    printf '%s: %s\n' "$tool" "$(command -v "$tool")"
  else
    printf '%s: unavailable\n' "$tool"
  fi
done
if command -v rustc >/dev/null; then rustc -vV; fi
df -h .
echo 'TikTok network: not tested; desktop WebView: excluded from CLI'
cli=${TIKTOOLS_BIN:-$PWD/target/debug/tiktools}
if [[ ! -x "$cli" ]]; then
  echo 'Runtime checks: skipped (build cargo build -p tiktools-cli --locked first)'
  exit 1
fi
scratch=$(mktemp -d "${TMPDIR:-$HOME}/tt-doctor.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
export TIKTOOLS_HOME="$scratch/home"
for item in DATA PLUGINS PLUGIN_DATA LOG TEMP BUILTIN_PLUGINS DEV_PLUGINS; do
  export "TIKTOOLS_${item}_DIR=$scratch/$item"
done
"$cli" --standalone --json system info
"$cli" --standalone --json system doctor
