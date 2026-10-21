#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}
export RUST_TEST_THREADS=${RUST_TEST_THREADS:-2}
scratch=$(mktemp -d "${TMPDIR:-$HOME}/tt-tests.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
unset TIKTOOLS_SESSION_COOKIE TIKTOOLS_DATA_DIR TIKTOOLS_PLUGINS_DIR TIKTOOLS_PLUGIN_DATA_DIR TIKTOOLS_LOG_DIR TIKTOOLS_TEMP_DIR
export TIKTOOLS_HOME="$scratch/home"
export TIKTOOLS_BUILTIN_PLUGINS_DIR="$scratch/builtin"
export TIKTOOLS_DEV_PLUGINS_DIR="$scratch/development"
failed=0
for crate in tiktools-cli tiktools-control-api tiktools-client tiktools-core tiktools-tiktok; do
  if cargo test -p "$crate" --locked; then
    echo "PASS: $crate"
  else
    echo "FAIL: $crate" >&2
    failed=1
  fi
done
if cargo test -p tiktools-core --locked --features persistence,native-tiktok,http; then
  echo 'PASS: core with CLI capabilities'
else
  echo 'FAIL: core with CLI capabilities' >&2
  failed=1
fi
if cargo test -p tiktools-plugin-sdk --locked --features http-client; then
  echo 'PASS: plugin SDK and native TLS initialization'
else
  echo 'FAIL: plugin SDK' >&2
  failed=1
fi
host=$(rustc -vV | sed -n 's/^host: //p')
case "$host" in
  *android*)
    if cargo test -p tiktools-plugin-loader --locked --no-default-features --test android_portable; then
      echo 'PASS: Android portable plugin probes'
    else
      echo 'FAIL: Android portable plugin probes' >&2
      failed=1
    fi
    if cargo test -p tiktools-plugin-loader --locked --no-default-features --lib process::tests::; then
      echo 'PASS: process framing, environment, timeouts and cleanup'
    else
      echo 'FAIL: process runtime tests' >&2
      failed=1
    fi ;;
  *) echo 'SKIPPED: Android portable plugin probes (native Android required)' ;;
esac
echo 'SKIPPED: SQLite stress in the default core build (executed with persistence in the full capability suite)'
echo 'SKIPPED: desktop, frontend, real TikTok authentication/network, external native plugins and desktop native-addon fixtures'
exit "$failed"
