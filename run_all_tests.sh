#!/bin/bash
# Script to run all Dvop tests successfully
# Runs tests individually where needed to avoid GTK threading issues

set -e

echo "🧪 Running All Dvop Tests"
echo "========================="
echo ""

# Unit tests
echo "📦 Unit Tests..."
unit_output=$(cargo test --lib 2>&1)
unit_result=$(echo "$unit_output" | grep "test result:")
echo "   $unit_result"
unit_count=$(echo "$unit_result" | grep -oP '\d+(?= passed)' || echo "0")
echo ""

# Quick integration tests  
echo "⚡ Quick Integration Tests..."
quick_output=$(cargo test --test quick_tests 2>&1)
quick_result=$(echo "$quick_output" | grep "test result:")
echo "   $quick_result"
quick_count=$(echo "$quick_result" | grep -oP '\d+(?= passed)' || echo "0")
echo ""

# Deep E2E tests (run individually)
echo "🔬 Deep E2E Tests..."
echo ""

e2e_total=0
e2e_passed=0
e2e_failed=0

# Get E2E test list
e2e_tests=$(cargo test --test e2e_tests -- --list --format terse 2>/dev/null | grep '^test_feature_' | cut -d: -f1)
e2e_total=$(echo "$e2e_tests" | wc -l)

e2e_counter=0
for test_name in $e2e_tests; do
    e2e_counter=$((e2e_counter + 1))
    printf "\r  Running E2E test %d/%d... " $e2e_counter $e2e_total
    
    if cargo test --test e2e_tests "$test_name" -- --exact --quiet 2>&1 | grep -q "test result: ok"; then
        e2e_passed=$((e2e_passed + 1))
    else
        e2e_failed=$((e2e_failed + 1))
    fi
done

echo ""
if [ $e2e_failed -eq 0 ]; then
    echo "  ✅ All $e2e_total E2E tests passed"
else
    echo "  ⚠️  $e2e_passed passed, $e2e_failed failed"
fi
echo ""

echo "========================="
echo "📊 Final Summary:"
echo "   Unit Tests: ${unit_count:-0} passed"
echo "   Quick Tests: ${quick_count:-0} passed"
echo "   E2E Tests: $e2e_passed/$e2e_total passed"
echo ""
total_passed=$((${unit_count:-0} + ${quick_count:-0} + e2e_passed))
echo "   TOTAL: $total_passed passed, $e2e_failed failed"
echo "========================="

if [ $e2e_failed -eq 0 ]; then
    echo "✅ All tests passed!"
    exit 0
else
    echo "⚠️  Some tests failed"
    exit 1
fi
