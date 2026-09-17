#!/usr/bin/env bash
# Check C++ UniFFI bindings regenerated in place against the committed ones.
#
# Usage (after regenerating into bindings/cpp/):
#   scripts/check-cpp-bindings.sh
#
# uniffi-bindgen-cpp v0.9.0+v0.29.4 emits the declarations of types that do
# not depend on each other in a varying order, so marketdata_uniffi.hpp is
# compared with its lines sorted: a reordering passes, a changed, added or
# removed line fails. The other generated files are stable and must match
# exactly, and no file may be added or removed. On failure the plain diff is
# printed.
set -euo pipefail

dir=bindings/cpp
hpp="$dir/marketdata_uniffi.hpp"
fail=0

others=$(git status --porcelain -- "$dir" ":(exclude)$hpp")
if [ -n "$others" ]; then
  echo "$others"
  git diff -- "$dir" ":(exclude)$hpp"
  fail=1
fi

if [ ! -f "$hpp" ] || ! git cat-file -e "HEAD:$hpp" 2>/dev/null \
  || ! cmp -s <(git show "HEAD:$hpp" | LC_ALL=C sort) <(LC_ALL=C sort "$hpp"); then
  echo "$hpp differs beyond declaration order:"
  git diff -- "$hpp"
  fail=1
fi

exit "$fail"
