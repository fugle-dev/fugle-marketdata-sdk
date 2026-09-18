#!/usr/bin/env bash
# Build and run a throwaway consumer against an assembled Go module
# (scripts/assemble-go-module.sh output) on the current host.
#
# Usage:
#   scripts/smoke-go-module.sh <module-dir> <version>
#
# Running it (not just building) matters: the generated bindings verify UniFFI
# API checksums at init and panic if bindings/go/marketdata/marketdata_uniffi.go
# is stale. The consumer must link the static archive, not a shared library.
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <module-dir> <version>" >&2
  exit 2
fi

MODULE="$(cd "$1" && pwd)"
VERSION="${2#v}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

cd "$WORK"
cat > go.mod <<GOMOD
module example.com/consumer

go 1.23

require github.com/fugle-dev/fugle-marketdata-go v${VERSION}

replace github.com/fugle-dev/fugle-marketdata-go => "${MODULE}"
GOMOD
cat > main.go <<'GO'
package main

import (
	"fmt"
	"os"

	mkt "github.com/fugle-dev/fugle-marketdata-go"
)

func main() {
	// The hand-written constructor, not the generated one: it is what
	// users call, and it once returned a typed-nil error on success.
	client, err := mkt.NewFugleRestClient(mkt.WithApiKey("smoke-test-key"))
	if err != nil {
		fmt.Println("constructor error:", err)
		os.Exit(1)
	}
	defer client.Destroy()
	fmt.Printf("ok: %T\n", client.Stock().Intraday())
}
GO
# The module's go.mod may require a newer patch release than the consumer
# declares; let the go command reconcile it.
go mod tidy
CGO_ENABLED=1 go build -o consumer .
echo "Host: $(uname -m) / $(go env GOOS)/$(go env GOARCH)"
echo "Dynamic dependencies:"
if [[ "$(uname -s)" == "Darwin" ]]; then
  otool -L consumer | tee deps.txt
else
  ldd consumer | tee deps.txt
fi
if grep -q marketdata_uniffi deps.txt; then
  echo "error: consumer is dynamically linked to marketdata_uniffi; expected static linking" >&2
  exit 1
fi
env -u LD_LIBRARY_PATH -u DYLD_LIBRARY_PATH ./consumer
