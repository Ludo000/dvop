#!/bin/bash
# Script to run all Dvop tests successfully
# Runs tests individually where needed to avoid GTK threading issues

echo "🧪 Running All Dvop Tests"
echo "========================="
echo ""

unit_failed=0
quick_failed=0

# Unit tests — GTK widgets must be created on a single thread
echo "📦 Unit Tests..."
cargo test --lib --quiet -- --test-threads=1 > /tmp/unit_tests.log 2>&1 || unit_failed=1
unit_result=$(grep "test result:" /tmp/unit_tests.log 2>/dev/null || echo "test result: 0 passed")
unit_count=$(echo "$unit_result" | grep -oP '\d+(?= passed)' | head -1 || echo "0")
unit_fail_count=$(echo "$unit_result" | grep -oP '\d+(?= failed)' | head -1 || echo "0")

if [ "$unit_fail_count" -gt "0" ] || [ "$unit_failed" -ne 0 ]; then
    echo "   ⚠️  $unit_result"
    grep -E "FAILED|panicked at" /tmp/unit_tests.log | tail -10
else
    echo "   ✅ $unit_result"
fi
echo ""

# Quick integration tests
echo "⚡ Quick Integration Tests..."
cargo test --test quick_tests --quiet -- --test-threads=1 2>&1 \
    | grep -v "GtkSourceView-CRITICAL" > /tmp/quick_tests.log || quick_failed=1
quick_result=$(grep "test result:" /tmp/quick_tests.log 2>/dev/null || echo "test result: 0 passed")
echo "   $quick_result"
quick_count=$(echo "$quick_result" | grep -oP '\d+(?= passed)' || echo "0")
quick_fail_count=$(echo "$quick_result" | grep -oP '\d+(?= failed)' | head -1 || echo "0")
echo ""

# Deep E2E tests (run individually)
echo "🔬 Deep E2E Tests..."
echo "   Compiling tests..."

# Build tests first
cargo test --test e2e_tests --no-run --quiet 2>&1 > /tmp/e2e_build.log || {
    echo "   ⚠️  Failed to build E2E tests"
    cat /tmp/e2e_build.log
    exit 1
}

# Get E2E test list
e2e_tests=$(cargo test --test e2e_tests -- --list 2>&1 | grep '^test_feature_' | awk '{print $1}' | sed 's/:$//')
e2e_total=$(echo "$e2e_tests" | wc -l)

if [ "$e2e_total" -eq 0 ]; then
    echo "   ⚠️  No E2E tests found"
    e2e_passed=0
    e2e_failed=0
else
    e2e_passed=0
    e2e_failed=0
    e2e_counter=0
    
    for test_name in $e2e_tests; do
        e2e_counter=$((e2e_counter + 1))
        printf "\r   Running E2E test %d/%d... " $e2e_counter $e2e_total
        
        if cargo test --test e2e_tests "$test_name" -- --exact --quiet 2>&1 | grep -q "test result: ok"; then
            e2e_passed=$((e2e_passed + 1))
        else
            e2e_failed=$((e2e_failed + 1))
        fi
    done
    echo ""
fi

if [ $e2e_failed -eq 0 ]; then
    echo "   ✅ All $e2e_total E2E tests passed"
else
    echo "   ⚠️  $e2e_passed passed, $e2e_failed failed"
fi
echo ""

echo "========================="
echo "📊 Final Summary:"
echo "   Unit Tests: ${unit_count:-0} passed"
echo "   Quick Tests: ${quick_count:-0} passed"
echo "   E2E Tests: $e2e_passed/$e2e_total passed"
echo ""
total_passed=$((${unit_count:-0} + ${quick_count:-0} + e2e_passed))
total_failed=$((${unit_fail_count:-0} + ${quick_fail_count:-0} + e2e_failed))
echo "   TOTAL: $total_passed passed, $total_failed failed"
echo "========================="

if [ "$unit_fail_count" -eq 0 ] && [ "$quick_fail_count" -eq 0 ] && [ $e2e_failed -eq 0 ] && [ $e2e_total -gt 0 ]; then
    echo "✅ All tests passed!"
    exit 0
else
    echo "⚠️  Some tests failed or incomplete"
    exit 1
fi
