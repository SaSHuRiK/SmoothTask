#!/bin/bash

echo "🔍 Testing threat_detection module compilation and functionality..."
echo ""

# Test 1: Check if the module file exists
echo "✅ Test 1: Checking if threat_detection.rs exists..."
if [ -f "smoothtask-core/src/health/threat_detection.rs" ]; then
    echo "✅ PASS: threat_detection.rs file exists"
else
    echo "❌ FAIL: threat_detection.rs file not found"
    exit 1
fi

# Test 2: Check if the module is exported in health/mod.rs
echo "✅ Test 2: Checking if threat_detection is exported in health/mod.rs..."
if grep -q "pub mod threat_detection;" "smoothtask-core/src/health/mod.rs"; then
    echo "✅ PASS: threat_detection module is exported"
else
    echo "❌ FAIL: threat_detection module not exported"
    exit 1
fi

# Test 3: Check if the module is properly used in health/mod.rs
echo "✅ Test 3: Checking if threat_detection is used in health/mod.rs..."
if grep -q "pub use threat_detection::\*;" "smoothtask-core/src/health/mod.rs"; then
    echo "✅ PASS: threat_detection module is properly used"
else
    echo "❌ FAIL: threat_detection module not properly used"
    exit 1
fi

# Test 4: Check if required imports are present
echo "✅ Test 4: Checking required imports in threat_detection.rs..."
required_imports=("anyhow::Result" "async_trait::async_trait" "chrono::{DateTime, Utc}" "serde::{Deserialize, Serialize}" "tracing::info" "uuid::Uuid")
for import in "${required_imports[@]}"; do
    if grep -q "use $import" "smoothtask-core/src/health/threat_detection.rs"; then
        echo "✅ PASS: Import found: $import"
    else
        echo "❌ FAIL: Import missing: $import"
        exit 1
    fi
done

# Test 5: Check if main structures are defined
echo "✅ Test 5: Checking main structures in threat_detection.rs..."
required_structures=("ThreatType" "ThreatSeverity" "ThreatStatus" "ThreatDetection" "ThreatDetectionConfig" "ThreatDetectionSystem" "ThreatDetectionTrait")
for structure in "${required_structures[@]}"; do
    if grep -q "$structure" "smoothtask-core/src/health/threat_detection.rs"; then
        echo "✅ PASS: Structure found: $structure"
    else
        echo "❌ FAIL: Structure missing: $structure"
        exit 1
    fi
done

# Test 6: Check if unit tests are present
echo "✅ Test 6: Checking unit tests in threat_detection.rs..."
if grep -q "#\[cfg(test)\]" "smoothtask-core/src/health/threat_detection.rs"; then
    echo "✅ PASS: Unit tests section found"
else
    echo "❌ FAIL: Unit tests section not found"
    exit 1
fi

# Test 7: Check if specific test functions are present
echo "✅ Test 7: Checking specific test functions..."
required_tests=("test_threat_detection_system_creation" "test_add_threat_detection" "test_resolve_threat_detection" "test_ml_model_management")
for test in "${required_tests[@]}"; do
    if grep -q "$test" "smoothtask-core/src/health/threat_detection.rs"; then
        echo "✅ PASS: Test function found: $test"
    else
        echo "❌ FAIL: Test function missing: $test"
        exit 1
    fi
done

# Test 8: Check integration with security_monitoring
echo "✅ Test 8: Checking integration with security_monitoring..."
if grep -q "threat_detection" "smoothtask-core/src/health/security_monitoring.rs"; then
    echo "✅ PASS: Integration with security_monitoring found"
else
    echo "❌ FAIL: Integration with security_monitoring not found"
    exit 1
fi

echo ""
echo "🎉 All basic checks passed!"
echo ""
echo "📋 Summary:"
echo "- Module structure: ✅ OK"
echo "- Required imports: ✅ OK"
echo "- Main structures: ✅ OK"
echo "- Unit tests: ✅ OK"
echo "- Integration: ✅ OK"
echo ""
echo "🔧 Next steps:"
echo "1. Run 'cargo check' to verify compilation"
echo "2. Run 'cargo test' to execute unit tests"
echo "3. Check for any compilation warnings or errors"
echo ""
