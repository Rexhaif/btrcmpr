#!/usr/bin/env bash
set -euo pipefail
use_selected=0
if [[ ${1:-} == --selected ]]; then use_selected=1; shift; fi
if (( $# != 5 )); then echo "usage: $0 [--selected] c|d ours|zstd FILE THREADS REPEATS" >&2; exit 2; fi
if (( use_selected )); then
    project_root=$(cd -- "$(dirname -- "$0")/.." && pwd)
    profile_binary=$(python3 "$project_root/scripts/selected_build.py" --profile)
    # The benchmark runner locks this same cache directory.
    exec {perf_slot}<"$(dirname -- "$profile_binary")"
    if ! flock -n "$perf_slot"; then
        echo "another benchmark or profiler holds the timing lock" >&2
        exit 1
    fi
    export ZSTD_WIDE=1
else
    cargo build --quiet --release --features baseline --example profile
    profile_binary="${CARGO_TARGET_DIR:-target}/release/examples/profile"
fi
tmp=$(mktemp -d)
trap 'rm -rf -- "$tmp"' EXIT
export PERF_CTL="$tmp/ctl" PERF_ACK="$tmp/ack"
mkfifo "$PERF_CTL" "$PERF_ACK"
"${PERF:-perf}" stat -D -1 --control="fifo:$PERF_CTL,$PERF_ACK" -e cycles:u,instructions:u,branches:u,branch-misses:u,cache-misses:u -- "$profile_binary" "$@"
