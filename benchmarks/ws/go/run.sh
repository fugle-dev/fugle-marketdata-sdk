#!/bin/bash
# Build and run the Go WebSocket benchmark client
# Usage: ./run.sh --url ws://localhost:8765 --timeout 60000
#
# Needs the UniFFI native library in target/release (the binding links it
# with an rpath there): cargo build -p marketdata-uniffi --release

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# go build is incremental, so this is cheap when nothing changed and never
# leaves a stale binary built against an older binding.
(cd "$SCRIPT_DIR" && CGO_ENABLED=1 go build -o ws-bench-go .)

exec "$SCRIPT_DIR/ws-bench-go" "$@"
