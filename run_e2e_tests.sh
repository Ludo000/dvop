#!/bin/bash
# Run all E2E tests individually to ensure they all pass
# (GTK threading limitation requires individual execution)

set -e

echo "🧪 Running Deep E2E Tests"
echo "========================="
echo ""

# Get list of all E2E tests
test_list=$(cargo test --test e2e_tests -- --list --format terse 2>/dev/null | grep '^test_feature_' | cut -d: -f1)

total=0
passed=0
failed=0
failed_tests=()

# Count total tests
total=$(echo "$test_list" | wc -l)

echo "📋 Found $total E2E tests to run"
echo ""

# Run each test individually
counter=0
for test_name in $test_list; do
    counter=$((counter + 1))
    
    # Show progress
    printf "\r  Progress: %d/$total tests... " $counter
    
    # Run single test
    if cargo test --test e2e_tests "$test_name" -- --exact --quiet 2>&1 | grep -q "test result: ok"; then
        passed=$((passed + 1))
    else
        failed=$((failed + 1))
        failed_tests+=("$test_name")
    fi
done

echo ""
echo ""
echo "========================="
echo "📊 E2E Test Results:"
echo "   Total: $total"
echo "   Passed: $passed"
echo "   Failed: $failed"
echo "========================="

if [ $failed -eq 0 ]; then
    echo "✅ All E2E tests passed!"
    exit 0
else
    echo "❌ Failed tests:"
    for test in "${failed_tests[@]}"; do
        echo "     - $test"
    done
    exit 1
fi
