#!/usr/bin/env bash
# Build the marketdata-uniffi static library for one Go platform and stage it
# for scripts/assemble-go-module.sh.
#
# Usage:
#   scripts/build-go-static-lib.sh <rust-target> <goos_goarch> <out-root>
#
# Example:
#   scripts/build-go-static-lib.sh aarch64-apple-darwin darwin_arm64 go-static/
#
# Produces:
#   <out-root>/<goos_goarch>/libmarketdata_uniffi.a   (debug info stripped)
#   <out-root>/<goos_goarch>/native-static-libs.txt   (system libs rustc says the
#                                                      final link needs)
#   <out-root>/<goos_goarch>/lib<name>.a              (any non-system import
#                                                      library named in that list,
#                                                      e.g. libwindows.0.53.0.a)
#
# Environment:
#   STRIP    Command used to drop debug info. Defaults to `strip -S` on macOS
#            and `strip --strip-debug` elsewhere. Windows targets are not
#            stripped unless STRIP is set explicitly.
#   NO_STRIP Set to 1 to keep debug info.
#   MACOSX_DEPLOYMENT_TARGET  Defaults to 11.0 for Apple targets.
#   CARGO_TARGET_DIR          Defaults to target/go-static.
#   CARGO_PROFILE_RELEASE_LTO / CARGO_PROFILE_RELEASE_CODEGEN_UNITS
#                             Default to fat / 1 to keep the archive small.
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 <rust-target> <goos_goarch> <out-root>" >&2
  exit 2
fi

TARGET="$1"
PLATFORM="$2"
OUT="$3/$PLATFORM"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# Separate target dir: the LTO profile below would otherwise invalidate the
# regular release artifacts used by the cdylib builds and local development.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target/go-static}"
TARGET_DIR="$CARGO_TARGET_DIR"

# Fat LTO + a single codegen unit drops unreachable code from the archive.
# Measured on linux/amd64: 63 MB -> 18 MB before strip, 12 MB after.
export CARGO_PROFILE_RELEASE_LTO="${CARGO_PROFILE_RELEASE_LTO:-fat}"
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS="${CARGO_PROFILE_RELEASE_CODEGEN_UNITS:-1}"

case "$PLATFORM" in
  darwin_arm64|darwin_amd64|linux_amd64|linux_arm64|windows_amd64) ;;
  *) echo "error: unsupported platform '$PLATFORM'" >&2; exit 2 ;;
esac

# Pin the macOS deployment target so C objects from build scripts (ring) match
# the Rust objects and do not inherit the build host's SDK version. Go 1.23+
# itself requires macOS 11.
if [[ "$TARGET" == *-apple-darwin ]]; then
  export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-11.0}"
fi

if [[ "$TARGET" == *-windows-* && -z "${STRIP:-}" ]]; then
  # The windows-gnu archive embeds raw-dylib import members (e.g. ntdll.dll)
  # and section-less .dwo members that both GNU strip and llvm-objcopy reject.
  # With LTO it is already ~14 MB, so ship it unstripped.
  NO_STRIP=1
fi
if [[ -z "${STRIP:-}" ]]; then
  if [[ "$(uname -s)" == "Darwin" ]]; then STRIP="strip -S"; else STRIP="strip --strip-debug"; fi
fi

mkdir -p "$OUT"
LOG="$(mktemp)"
trap 'rm -f "$LOG"' EXIT

cd "$ROOT"
# rustc only prints native-static-libs when it actually links the staticlib.
# Cleaning just this crate forces that without rebuilding its dependencies.
cargo clean --release --target "$TARGET" -p marketdata-uniffi >/dev/null 2>&1 || true
# --color never: CI sets CARGO_TERM_COLOR=always, and colour codes inside the
# `note:` line would defeat the parsing below.
cargo rustc --color never -p marketdata-uniffi --release --lib --crate-type staticlib \
  --target "$TARGET" -- --print native-static-libs 2>&1 | tee "$LOG"

# Strip any ANSI escapes anyway, in case colour is forced another way.
# $'...' makes the escape byte literal so BSD sed (macOS) and GNU sed agree.
NATIVE_LIBS="$(sed -e $'s/\x1b\\[[0-9;]*m//g' "$LOG" | sed -n 's/.*note: native-static-libs: //p' | head -1 | tr -d '\r')"
if [[ -z "$NATIVE_LIBS" ]]; then
  echo "error: rustc did not report native-static-libs for $TARGET" >&2
  exit 1
fi

LIB="$TARGET_DIR/$TARGET/release/libmarketdata_uniffi.a"
if [[ ! -f "$LIB" ]]; then
  echo "error: expected static library not found: $LIB" >&2
  exit 1
fi

cp "$LIB" "$OUT/libmarketdata_uniffi.a"
BEFORE=$(wc -c < "$OUT/libmarketdata_uniffi.a" | tr -d ' ')
if [[ "${NO_STRIP:-0}" != "1" ]]; then
  # macOS strip prints one "already stripped" warning per object; keep the
  # output only when the command actually fails.
  STRIP_LOG="$(mktemp)"
  # shellcheck disable=SC2086
  if ! $STRIP "$OUT/libmarketdata_uniffi.a" > "$STRIP_LOG" 2>&1; then
    cat "$STRIP_LOG" >&2
    echo "error: '$STRIP' failed on $OUT/libmarketdata_uniffi.a" >&2
    rm -f "$STRIP_LOG"
    exit 1
  fi
  rm -f "$STRIP_LOG"
fi
AFTER=$(wc -c < "$OUT/libmarketdata_uniffi.a" | tr -d ' ')
printf '%s\n' "$NATIVE_LIBS" > "$OUT/native-static-libs.txt"

# Some -l entries are import libraries shipped inside crates (windows-targets),
# not system libraries. Copy those next to the archive so consumers can link.
SEARCH_DIRS=()
while IFS= read -r dir; do
  [[ -d "$dir" ]] && SEARCH_DIRS+=("$dir")
done < <(cat "$TARGET_DIR/$TARGET"/release/build/*/output 2>/dev/null \
         | sed -n -E 's/^cargo::?rustc-link-search=(native=|all=)?//p' | sort -u)

for token in $NATIVE_LIBS; do
  [[ "$token" == -l* ]] || continue
  name="${token#-l}"
  for dir in "${SEARCH_DIRS[@]+"${SEARCH_DIRS[@]}"}"; do
    if [[ -f "$dir/lib$name.a" ]]; then
      cp "$dir/lib$name.a" "$OUT/"
      echo "bundled import library: lib$name.a (from $dir)"
      break
    fi
  done
done

echo "platform:           $PLATFORM ($TARGET)"
echo "native-static-libs: $NATIVE_LIBS"
echo "archive size:       $BEFORE -> $AFTER bytes"
