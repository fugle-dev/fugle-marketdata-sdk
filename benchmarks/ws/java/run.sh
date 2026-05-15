#!/bin/bash
# Build and run the Java WebSocket benchmark client
# Usage: ./run.sh --url ws://localhost:8765 --timeout 60000

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR/../../.."
JAVA_SRC="$PROJECT_ROOT/bindings/java"
NATIVE_LIB_DIR="$PROJECT_ROOT/target/release"

export JAVA_HOME="${JAVA_HOME:-/opt/homebrew/Cellar/openjdk@21/21.0.10/libexec/openjdk.jdk/Contents/Home}"

# Build SDK if needed
if [ ! -d "$JAVA_SRC/build/classes/java/main" ]; then
    (cd "$JAVA_SRC" && ./gradlew compileJava -q 2>/dev/null)
fi

# Find JNA jar
JNA_JAR=$(find ~/.gradle -name 'jna-5*.jar' 2>/dev/null | head -1)
if [ -z "$JNA_JAR" ]; then
    (cd "$JAVA_SRC" && ./gradlew dependencies -q 2>/dev/null)
    JNA_JAR=$(find ~/.gradle -name 'jna-5*.jar' 2>/dev/null | head -1)
fi

# Compile benchmark
CLASSPATH="$JAVA_SRC/build/classes/java/main:$JNA_JAR"
"$JAVA_HOME/bin/javac" -cp "$CLASSPATH" -d "$SCRIPT_DIR" "$SCRIPT_DIR/WebSocketBenchmark.java"

# Run benchmark
export DYLD_LIBRARY_PATH="$NATIVE_LIB_DIR:$DYLD_LIBRARY_PATH"
"$JAVA_HOME/bin/java" -cp "$SCRIPT_DIR:$CLASSPATH" \
    -Djna.library.path="$NATIVE_LIB_DIR" \
    WebSocketBenchmark "$@"
