#!/usr/bin/env bash
# Print the Java test counts from the JUnit XML reports and fail on skips.
#
# Usage (after ./gradlew test in bindings/java):
#   scripts/check-java-test-results.sh
#
# Gradle does not print test counts, so a run where tests silently skipped
# looks the same as one where they ran. CI excludes the integration tests by
# tag and loads the native library, so nothing should skip.
set -euo pipefail

dir="$(dirname "$0")/../bindings/java/build/test-results/test"

python3 - "$dir" <<'PY'
import glob, sys
import xml.etree.ElementTree as ET

reports = glob.glob(f"{sys.argv[1]}/TEST-*.xml")
if not reports:
    print("::error::No Java test reports found")
    sys.exit(1)

totals = dict.fromkeys(("tests", "skipped", "failures", "errors"), 0)
for path in reports:
    suite = ET.parse(path).getroot()
    for key in totals:
        totals[key] += int(suite.get(key, 0))

print(" ".join(f"{key}={value}" for key, value in totals.items()))
if totals["skipped"]:
    print(f"::error::{totals['skipped']} Java tests skipped")
    sys.exit(1)
PY
