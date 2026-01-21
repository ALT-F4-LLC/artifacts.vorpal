#!/usr/bin/env bash
#
# test-detect-changed-artifacts.sh
#
# Regression tests for detect-changed-artifacts.sh
#
# Usage: ./test-detect-changed-artifacts.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DETECT_SCRIPT="$SCRIPT_DIR/detect-changed-artifacts.sh"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

TESTS_PASSED=0
TESTS_FAILED=0

# Test helper functions
pass() {
    echo -e "${GREEN}PASS${NC}: $1"
    TESTS_PASSED=$((TESTS_PASSED + 1))
}

fail() {
    echo -e "${RED}FAIL${NC}: $1"
    echo "  Expected: $2"
    echo "  Got: $3"
    TESTS_FAILED=$((TESTS_FAILED + 1))
}

# Test: --list returns artifacts
test_list_returns_artifacts() {
    local result
    result="$("$DETECT_SCRIPT" --list | wc -l | tr -d ' ')"
    if [[ "$result" -gt 0 ]]; then
        pass "--list returns artifacts (found $result)"
    else
        fail "--list returns artifacts" ">0 artifacts" "$result"
    fi
}

# Test: --all returns valid JSON array
test_all_returns_json() {
    local result
    result="$("$DETECT_SCRIPT" --all)"
    if echo "$result" | jq -e 'type == "array"' > /dev/null 2>&1; then
        pass "--all returns valid JSON array"
    else
        fail "--all returns valid JSON array" "JSON array" "$result"
    fi
}

# Test: Empty diff returns empty array
test_empty_diff_returns_empty() {
    local result
    result="$("$DETECT_SCRIPT" HEAD HEAD)"
    if [[ "$result" == "[]" ]]; then
        pass "Same commit returns empty array"
    else
        fail "Same commit returns empty array" "[]" "$result"
    fi
}

# REGRESSION TEST: Changes to non-artifact files should NOT trigger full rebuild
# This tests the bug fix where CORE_FILES mechanism was removed
test_non_artifact_changes_dont_trigger_full_rebuild() {
    # Get total artifact count
    local total_artifacts
    total_artifacts="$("$DETECT_SCRIPT" --all | jq 'length')"

    # Test with commits 5fc8933 to 98beb7a (squash merge of PR #9) which added beads and jj
    # These commits also modified src/vorpal.rs, Cargo.lock, and workflow files
    # but should ONLY return beads and jj (not all artifacts)
    local result
    result="$("$DETECT_SCRIPT" 5fc8933e5f5ddb18b00d2d307d676e4c503814c7 98beb7acbc192e0471891fed1942026c7fbc6296)"

    local result_count
    result_count="$(echo "$result" | jq 'length')"

    # Should return exactly 2 artifacts (beads and jj), not all artifacts
    if [[ "$result_count" -eq 2 ]]; then
        # Verify it's beads and jj
        local has_beads has_jj
        has_beads="$(echo "$result" | jq 'contains(["beads"])')"
        has_jj="$(echo "$result" | jq 'contains(["jj"])')"

        if [[ "$has_beads" == "true" && "$has_jj" == "true" ]]; then
            pass "Non-artifact file changes don't trigger full rebuild (regression test)"
        else
            fail "Non-artifact file changes don't trigger full rebuild" '["beads","jj"]' "$result"
        fi
    else
        fail "Non-artifact file changes don't trigger full rebuild" "2 artifacts" "$result_count artifacts: $result"
    fi
}

# REGRESSION TEST: Script doesn't have hardcoded artifact mechanisms
test_no_hardcoded_artifacts() {
    local failed=false

    if grep -q "CORE_FILES" "$DETECT_SCRIPT"; then
        fail "CORE_FILES mechanism removed (regression)" "no CORE_FILES" "CORE_FILES found in script"
        failed=true
    else
        pass "CORE_FILES mechanism removed (regression)"
    fi

    if grep -q "is_core_file" "$DETECT_SCRIPT"; then
        fail "is_core_file function removed (regression)" "no is_core_file" "is_core_file found in script"
        failed=true
    else
        pass "is_core_file function removed (regression)"
    fi

    if grep -q "DEPENDENTS" "$DETECT_SCRIPT"; then
        fail "DEPENDENTS mechanism removed (regression)" "no DEPENDENTS" "DEPENDENTS found in script"
        failed=true
    else
        pass "DEPENDENTS mechanism removed (regression)"
    fi
}

# Run all tests
main() {
    echo "Running detect-changed-artifacts.sh tests..."
    echo "============================================="
    echo ""

    test_list_returns_artifacts
    test_all_returns_json
    test_empty_diff_returns_empty
    test_non_artifact_changes_dont_trigger_full_rebuild
    test_no_hardcoded_artifacts

    echo ""
    echo "============================================="
    echo "Results: $TESTS_PASSED passed, $TESTS_FAILED failed"

    if [[ "$TESTS_FAILED" -gt 0 ]]; then
        exit 1
    fi
}

main "$@"
