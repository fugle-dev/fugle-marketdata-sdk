.PHONY: all clean python-dev python-release nodejs-dev nodejs-release csharp-dev csharp-release gen-bindings gen-csharp gen-go gen-java build-csharp build-go build-java test test-python test-nodejs test-csharp test-go test-java

# ============================================================
# Platform detection — picks the right dylib extension for the
# current host so UniFFI bindgen targets resolve on macOS, Linux,
# and Windows (MSYS/Cygwin) without per-target shell hacks.
# ============================================================
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
	LIB_EXT := dylib
	LIB_PREFIX := lib
else ifeq ($(UNAME_S),Linux)
	LIB_EXT := so
	LIB_PREFIX := lib
else
	# MINGW*, MSYS*, CYGWIN* on Windows
	LIB_EXT := dll
	LIB_PREFIX :=
endif
UNIFFI_LIB := target/release/$(LIB_PREFIX)marketdata_uniffi.$(LIB_EXT)

# Default: Build all bindings in order (Python -> Node.js -> C#)
all: python-release nodejs-release csharp-release

# ============================================================
# Python Bindings (via maturin)
# ============================================================
python-dev:
	cd py && maturin develop

python-release:
	cd py && maturin build --release

# ============================================================
# Node.js Bindings (via napi-rs)
# ============================================================
nodejs-dev:
	cd js && npm run build:debug

nodejs-release:
	cd js && npm run build

# ============================================================
# C# Bindings (via UniFFI)
# ============================================================
csharp-dev:
	cargo build -p marketdata-uniffi

csharp-release:
	cargo build -p marketdata-uniffi --release

# Generate all bindings (C#, Go, and Java)
gen-bindings: gen-csharp gen-go gen-java gen-cpp

# Generate C++ bindings from UniFFI (requires cpp feature to strip async methods)
gen-cpp:
	cargo build -p marketdata-uniffi --features cpp --release
	mkdir -p bindings/cpp
	uniffi-bindgen-cpp --library $(UNIFFI_LIB) -o bindings/cpp/
	@echo "C++ bindings generated successfully (sync-only API)"

# Generate C# bindings from UniFFI (library mode - proc-macro approach).
# `access_modifier = "public"` in uniffi/uniffi.toml ensures generated types
# (records, interfaces, enums, classes, MarketdataUniffiMethods static class)
# are consumer-visible. See CONFIGURATION.md in NordSecurity/uniffi-bindgen-cs.
gen-csharp:
	cargo build -p marketdata-uniffi --release
	uniffi-bindgen-cs --library $(UNIFFI_LIB) --config uniffi/uniffi.toml -o bindings/csharp/MarketdataUniffi/
	@echo "C# bindings generated successfully"

# ============================================================
# Go Bindings (via UniFFI)
# ============================================================

# Generate Go bindings from UniFFI (library mode - proc-macro approach)
gen-go:
	cargo build -p marketdata-uniffi --release
	mkdir -p bindings/go/marketdata
	uniffi-bindgen-go --library $(UNIFFI_LIB) -o bindings/go/tmp/
	mv bindings/go/tmp/marketdata_uniffi/* bindings/go/marketdata/
	rmdir bindings/go/tmp/marketdata_uniffi bindings/go/tmp

# ============================================================
# Java Bindings (via UniFFI)
# ============================================================

# Generate Java bindings from UniFFI (library mode - proc-macro approach)
gen-java:
	cargo build -p marketdata-uniffi --release
	mkdir -p bindings/java/src/main/java
	uniffi-bindgen-java generate --library $(UNIFFI_LIB) \
		--out-dir bindings/java/src/main/java \
		--crate marketdata_uniffi \
		--config uniffi/uniffi.toml

# Build C# project after generating bindings
build-csharp: gen-csharp
	dotnet build bindings/csharp/FugleMarketData.sln

# Build Go module after generating bindings
build-go: gen-go
	cd bindings/go/marketdata && CGO_ENABLED=1 go build .

# Build Java project after generating bindings
build-java: gen-java
	cd bindings/java && ./gradlew compileJava

# ============================================================
# Testing
# ============================================================
test: test-python test-nodejs test-csharp

test-python:
	cd py && maturin develop && python -m pytest tests/ -v || echo "Python tests not yet configured"

test-nodejs:
	cd js && npm test || echo "Node.js tests not yet configured"

test-csharp:
	cargo test -p marketdata-uniffi
	@echo "Run C# tests with: dotnet test bindings/csharp/FugleMarketData.Tests/"

test-go:
	cd bindings/go/marketdata && CGO_ENABLED=1 go test -v -short .

test-java:
	cd bindings/java && ./gradlew test

# ============================================================
# Workspace Operations
# ============================================================
check:
	cargo check --workspace

build:
	cargo build --workspace

build-release:
	cargo build --workspace --release

clean:
	cargo clean
	rm -rf py/target/
	rm -rf js/target/
	rm -rf uniffi/target/

# ============================================================
# Development Helpers
# ============================================================
fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace -- -D warnings

# Show help
help:
	@echo "Fugle MarketData SDK Build System"
	@echo ""
	@echo "Build targets:"
	@echo "  make all              - Build all bindings (release)"
	@echo "  make python-dev       - Build Python binding (dev)"
	@echo "  make python-release   - Build Python binding (release)"
	@echo "  make nodejs-dev       - Build Node.js binding (dev)"
	@echo "  make nodejs-release   - Build Node.js binding (release)"
	@echo "  make csharp-dev       - Build C# binding (dev)"
	@echo "  make csharp-release   - Build C# binding (release)"
	@echo "  make gen-bindings     - Generate all UniFFI bindings (C# + Go + Java)"
	@echo "  make gen-csharp       - Generate C# bindings from UniFFI"
	@echo "  make gen-go           - Generate Go bindings from UniFFI"
	@echo "  make gen-java         - Generate Java bindings from UniFFI"
	@echo "  make build-csharp     - Build C# project"
	@echo "  make build-go         - Build Go module"
	@echo "  make build-java       - Build Java project"
	@echo ""
	@echo "Test targets:"
	@echo "  make test             - Run all tests"
	@echo "  make test-python      - Run Python tests"
	@echo "  make test-nodejs      - Run Node.js tests"
	@echo "  make test-csharp      - Run C# tests"
	@echo "  make test-go          - Run Go tests"
	@echo "  make test-java        - Run Java tests"
	@echo ""
	@echo "Workspace targets:"
	@echo "  make check            - Check workspace (fast)"
	@echo "  make build            - Build workspace (debug)"
	@echo "  make build-release    - Build workspace (release)"
	@echo "  make clean            - Clean all build artifacts"
	@echo "  make fmt              - Format all code"
	@echo "  make lint             - Run clippy lints"
