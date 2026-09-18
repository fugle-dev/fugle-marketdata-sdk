#!/usr/bin/env bash
# Assemble the standalone Go module published to
# github.com/fugle-dev/fugle-marketdata-go.
#
# Usage:
#   scripts/assemble-go-module.sh <static-libs-dir> <out-dir> <version>
#
# <static-libs-dir> holds one directory per platform, as produced by
# scripts/build-go-static-lib.sh:
#   <goos_goarch>/libmarketdata_uniffi.a
#   <goos_goarch>/native-static-libs.txt
#   <goos_goarch>/lib*.a                  (optional extra import libraries)
#
# The output replaces the monorepo's dynamic-linking cgo.go with one generated
# cgo_<goos>_<goarch>.go per platform that links the static archive, so
# `go get` works without LD_LIBRARY_PATH / DYLD_LIBRARY_PATH.
#
# Environment:
#   GO_PLATFORMS  Space-separated platforms that must be present.
#                 Default: "darwin_arm64 darwin_amd64 linux_amd64 linux_arm64 windows_amd64".
#                 Narrow it only for local experiments; releases need all five.
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 <static-libs-dir> <out-dir> <version>" >&2
  exit 2
fi

LIBS="$1"
OUT="$2"
VERSION="${3#v}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/bindings/go/marketdata"
DIST="$ROOT/bindings/go/dist"
PLATFORMS="${GO_PLATFORMS:-darwin_arm64 darwin_amd64 linux_amd64 linux_arm64 windows_amd64}"

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "error: version '$VERSION' is not semver (expected e.g. 0.1.0-rc.1)" >&2
  exit 2
fi
if [[ -e "$OUT" && -n "$(ls -A "$OUT" 2>/dev/null)" ]]; then
  echo "error: output directory '$OUT' is not empty" >&2
  exit 2
fi

# Validate inputs before writing anything.
for platform in $PLATFORMS; do
  for f in libmarketdata_uniffi.a native-static-libs.txt; do
    if [[ ! -f "$LIBS/$platform/$f" ]]; then
      echo "error: missing $LIBS/$platform/$f" >&2
      exit 1
    fi
  done
  # cgo LDFLAGS are copied verbatim into generated source, so only accept
  # plain -l<name> and "-framework <name>" tokens.
  if ! tr -s ' ' '\n' < "$LIBS/$platform/native-static-libs.txt" | grep -v '^$' \
       | awk 'BEGIN{fw=0} { if (fw) { if ($0 !~ /^[A-Za-z0-9_]+$/) exit 1; fw=0; next }
                            if ($0 == "-framework") { fw=1; next }
                            if ($0 !~ /^-l[A-Za-z0-9_.+-]+$/) exit 1 } END{ if (fw) exit 1 }'; then
    echo "error: unexpected token in $LIBS/$platform/native-static-libs.txt" >&2
    exit 1
  fi
done

mkdir -p "$OUT"

# Go sources: everything except the monorepo-only cgo.go and tests.
for f in "$SRC"/*.go; do
  base="$(basename "$f")"
  [[ "$base" == "cgo.go" || "$base" == *_test.go ]] && continue
  cp "$f" "$OUT/"
done
cp "$SRC/marketdata_uniffi.h" "$SRC/go.mod" "$OUT/"

for platform in $PLATFORMS; do
  mkdir -p "$OUT/lib/$platform"
  cp "$LIBS/$platform"/*.a "$OUT/lib/$platform/"
  native_libs="$(tr -s ' \n' ' ' < "$LIBS/$platform/native-static-libs.txt" | sed 's/ *$//')"
  if [[ "$platform" == darwin_* ]]; then
    # libSystem (and its -lc alias) is part of every Darwin link already;
    # repeating it makes ld print "ignoring duplicate libraries" on each build.
    native_libs="$(printf '%s\n' $native_libs | grep -v -x -e '-lSystem' -e '-lc' | tr '\n' ' ' | sed 's/ *$//')"
  fi
  sed -e "s|@PLATFORM@|$platform|g" -e "s|@NATIVE_LIBS@|$native_libs|" \
    "$DIST/cgo.go.tmpl" > "$OUT/cgo_$platform.go"
done

sed "s|@VERSION@|$VERSION|g" "$DIST/README.md" > "$OUT/README.md"
cp "$ROOT/LICENSE-MIT" "$ROOT/LICENSE-APACHE" "$OUT/"

echo "Assembled github.com/fugle-dev/fugle-marketdata-go v$VERSION in $OUT"
( cd "$OUT" && find . -type f | sort | while read -r f; do
    printf '  %10s  %s\n' "$(wc -c < "$f" | tr -d ' ')" "${f#./}"
  done )
