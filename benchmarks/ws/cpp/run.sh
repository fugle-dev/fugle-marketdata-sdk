#!/bin/bash
# Build and run the C++ WebSocket benchmark client
# Usage: ./run.sh --url ws://localhost:8765 --timeout 60000

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR/../../.."
CPP_BINDINGS="$PROJECT_ROOT/bindings/cpp"
CPP_LIB="$SCRIPT_DIR"

# Build if needed
if [ ! -f "$SCRIPT_DIR/ws-bench-cpp" ] || [ "$CPP_BINDINGS/marketdata_uniffi.hpp" -nt "$SCRIPT_DIR/ws-bench-cpp" ]; then
    # Need cpp-feature dylib
    if [ ! -f "$CPP_LIB/libmarketdata_uniffi.dylib" ]; then
        (cd "$PROJECT_ROOT" && cargo build -p marketdata-uniffi --features cpp --release 2>/dev/null)
        cp "$PROJECT_ROOT/target/release/libmarketdata_uniffi.dylib" "$CPP_LIB/"
        # Rebuild normal for other languages
        (cd "$PROJECT_ROOT" && cargo build -p marketdata-uniffi --release 2>/dev/null)
    fi
    c++ -std=c++20 -O2 -I"$CPP_BINDINGS" \
        "$SCRIPT_DIR/bench.cpp" "$CPP_BINDINGS/marketdata_uniffi.cpp" \
        -L"$CPP_LIB" -lmarketdata_uniffi \
        -o "$SCRIPT_DIR/ws-bench-cpp" 2>/dev/null
fi

export DYLD_LIBRARY_PATH="$CPP_LIB:$DYLD_LIBRARY_PATH"
exec "$SCRIPT_DIR/ws-bench-cpp" "$@"
