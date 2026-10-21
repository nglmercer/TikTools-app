#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
for tool in rustc cargo clang python cmake pkg-config protoc; do
  command -v "$tool" >/dev/null || { echo "Missing $tool; install the Termux rust, clang, python, cmake, pkg-config and protobuf packages" >&2; exit 1; }
done
rustc -vV
cargo --version
clang --version
python - <<'VERSION'
import re, subprocess, sys
version = subprocess.check_output(['rustc', '--version'], text=True)
match = re.search(r'(\d+)\.(\d+)\.(\d+)', version)
if not match or tuple(map(int, match.groups())) < (1, 95, 0):
    sys.exit('TikTools requires Rust >= 1.95; update the Termux rust package')
VERSION
host=$(rustc -vV | sed -n 's/^host: //p')
case "$host" in
  *android*) echo "Native Android compiler: $host ($(uname -m))" ;;
  *) echo "Notice: $host is not native Android; this cannot validate Termux acceptance" >&2 ;;
esac
df -h .
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}
cargo check -p tiktools-cli --locked
