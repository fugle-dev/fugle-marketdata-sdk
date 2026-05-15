#!/bin/bash
#
# Validation script to check for deprecated API patterns in documentation.
#
# This script ensures that documentation (READMEs) only shows v0.3.0 API patterns
# and doesn't regress to v0.2.x deprecated patterns.
#
# Exit codes:
#   0 - All checks passed
#   1 - Deprecated patterns found

set -e

echo "=================================="
echo "Documentation Migration Validation"
echo "=================================="
echo ""

EXIT_CODE=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "Checking for deprecated patterns in documentation..."
echo ""

# Deprecated patterns to check
# Note: MIGRATION.md is expected to have these patterns in "Before" examples, so we skip it
DEPRECATED_PATTERNS=(
    'RestClient("'           # Python positional string
    "new RestClient('"       # JavaScript string constructor
    "new WebSocketClient('"  # JavaScript string constructor
)

# Files to check (README files that should NOT have deprecated patterns)
README_FILES=(
    "py/README.md"
    "js/README.md"
    "core/README.md"
    "uniffi/README.md"
    "README.md"
)

FOUND_ISSUES=0

for file in "${README_FILES[@]}"; do
    if [ ! -f "$file" ]; then
        echo -e "${YELLOW}⚠ Skipping $file (not found)${NC}"
        continue
    fi

    echo "Checking $file..."

    for pattern in "${DEPRECATED_PATTERNS[@]}"; do
        # Use grep with line numbers, ignore case doesn't matter for exact matches
        if grep -n "$pattern" "$file" > /dev/null 2>&1; then
            echo -e "${RED}✗ Found deprecated pattern '$pattern' in $file:${NC}"
            grep -n "$pattern" "$file"
            FOUND_ISSUES=1
            EXIT_CODE=1
        fi
    done
done

echo ""
echo "Checking for deprecated patterns in examples..."
echo ""

if [ -d "examples" ]; then
    # Check Python examples
    if find examples -name "*.py" -type f | grep -q .; then
        echo "Checking Python examples..."
        for pattern in 'RestClient("' 'WebSocketClient("'; do
            if find examples -name "*.py" -type f -exec grep -l "$pattern" {} \; 2>/dev/null | grep -q .; then
                echo -e "${RED}✗ Found deprecated pattern '$pattern' in Python examples:${NC}"
                find examples -name "*.py" -type f -exec grep -Hn "$pattern" {} \;
                FOUND_ISSUES=1
                EXIT_CODE=1
            fi
        done
    fi

    # Check JavaScript examples
    if find examples -name "*.js" -type f | grep -q .; then
        echo "Checking JavaScript examples..."
        for pattern in "new RestClient('" "new WebSocketClient('"; do
            if find examples -name "*.js" -type f -exec grep -l "$pattern" {} \; 2>/dev/null | grep -q .; then
                echo -e "${RED}✗ Found deprecated pattern '$pattern' in JavaScript examples:${NC}"
                find examples -name "*.js" -type f -exec grep -Hn "$pattern" {} \;
                FOUND_ISSUES=1
                EXIT_CODE=1
            fi
        done
    fi
else
    echo -e "${YELLOW}⚠ No examples directory found (skipping)${NC}"
fi

echo ""
echo "=================================="
if [ $FOUND_ISSUES -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed!${NC}"
    echo "No deprecated patterns found in documentation."
else
    echo -e "${RED}✗ Validation failed!${NC}"
    echo "Found deprecated v0.2.x patterns in documentation."
    echo "Please update these files to use v0.3.0 API patterns."
fi
echo "=================================="

exit $EXIT_CODE
