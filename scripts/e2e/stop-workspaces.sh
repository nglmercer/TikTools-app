#!/usr/bin/env bash
# Stop headless TikTools hosts started by the e2e scripts (IPC names tt-*).
# Matches on the TIKTOOLS_IPC_NAME in each process environment, never by command line.
set -euo pipefail
stopped=0
for env in /proc/[0-9]*/environ; do
  [ -r "$env" ] || continue
  pid="${env#/proc/}"; pid="${pid%/environ}"
  name=$(tr "\0" "\n" < "$env" 2>/dev/null | sed -n "s/^TIKTOOLS_IPC_NAME=//p" || true)
  case "$name" in tt-*) kill "$pid" 2>/dev/null && stopped=$((stopped+1));; esac
done
echo "stopped $stopped e2e host(s)"
