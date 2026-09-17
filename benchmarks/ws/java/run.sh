#!/bin/bash
# Build and run the Java WebSocket benchmark client
# Usage: ./run.sh --url ws://localhost:8765 --timeout 60000

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR/../../.."
JAVA_SRC="$PROJECT_ROOT/bindings/java"
NATIVE_LIB_DIR="$PROJECT_ROOT/target/release"

# JDK 21: an explicit JAVA_HOME wins, then macOS java_home, then the PATH.
if [ -z "$JAVA_HOME" ] && [ -x /usr/libexec/java_home ]; then
    JAVA_HOME="$(/usr/libexec/java_home -v 21 2>/dev/null || true)"
fi
if [ -n "$JAVA_HOME" ]; then
    JAVA="$JAVA_HOME/bin/java"
    JAVAC="$JAVA_HOME/bin/javac"
    export JAVA_HOME
else
    JAVA=java
    JAVAC=javac
fi

# Compile the SDK every time: Gradle skips it when nothing changed, and a
# check for an existing build directory would run against stale classes.
(cd "$JAVA_SRC" && ./gradlew compileJava -q)

# Find JNA jar
JNA_JAR=$(find ~/.gradle -name 'jna-5*.jar' 2>/dev/null | head -1)
if [ -z "$JNA_JAR" ]; then
    (cd "$JAVA_SRC" && ./gradlew dependencies -q)
    JNA_JAR=$(find ~/.gradle -name 'jna-5*.jar' 2>/dev/null | head -1)
fi
if [ -z "$JNA_JAR" ]; then
    echo "JNA jar not found under ~/.gradle" >&2
    exit 1
fi

# Compile benchmark
CLASSPATH="$JAVA_SRC/build/classes/java/main:$JNA_JAR"
"$JAVAC" -cp "$CLASSPATH" -d "$SCRIPT_DIR" "$SCRIPT_DIR/WebSocketBenchmark.java"

# Run benchmark
export DYLD_LIBRARY_PATH="$NATIVE_LIB_DIR:$DYLD_LIBRARY_PATH"
"$JAVA" -cp "$SCRIPT_DIR:$CLASSPATH" \
    -Djna.library.path="$NATIVE_LIB_DIR" \
    WebSocketBenchmark "$@"
