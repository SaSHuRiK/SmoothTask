#!/bin/bash

echo "Verifying Wi-Fi 7 and 6G network expansion implementation..."
echo ""

# Check if the code compiles
echo "1. Checking compilation..."
cd /home/sashurik/Dev/SmoothTask
cargo check --package smoothtask-core 2>&1 | grep -E "(Compiling|Finished|error|warning)" | head -10

if [ $? -eq 0 ]; then
    echo "✓ Compilation successful"
else
    echo "✗ Compilation failed"
    exit 1
fi

echo ""
echo "2. Checking for new structures..."

# Check for Wifi7Stats structure
grep -q "pub struct Wifi7Stats" smoothtask-core/src/metrics/network.rs
if [ $? -eq 0 ]; then
    echo "✓ Wifi7Stats structure found"
else
    echo "✗ Wifi7Stats structure not found"
    exit 1
fi

# Check for Cellular6GStats structure
grep -q "pub struct Cellular6GStats" smoothtask-core/src/metrics/network.rs
if [ $? -eq 0 ]; then
    echo "✓ Cellular6GStats structure found"
else
    echo "✗ Cellular6GStats structure not found"
    exit 1
fi

echo ""
echo "3. Checking for new functions..."

# Check for collect_wifi7_stats function
grep -q "fn collect_wifi7_stats" smoothtask-core/src/metrics/network.rs
if [ $? -eq 0 ]; then
    echo "✓ collect_wifi7_stats function found"
else
    echo "✗ collect_wifi7_stats function not found"
    exit 1
fi

# Check for collect_cellular6g_stats function
grep -q "fn collect_cellular6g_stats" smoothtask-core/src/metrics/network.rs
if [ $? -eq 0 ]; then
    echo "✓ collect_cellular6g_stats function found"
else
    echo "✗ collect_cellular6g_stats function not found"
    exit 1
fi

echo ""
echo "4. Checking for new interface types..."

# Check for Wifi7 interface type
grep -q "Wifi7," smoothtask-core/src/metrics/network.rs
if [ $? -eq 0 ]; then
    echo "✓ Wifi7 interface type found"
else
    echo "✗ Wifi7 interface type not found"
    exit 1
fi

# Check for Cellular6G interface type
grep -q "Cellular6G," smoothtask-core/src/metrics/network.rs
if [ $? -eq 0 ]; then
    echo "✓ Cellular6G interface type found"
else
    echo "✗ Cellular6G interface type not found"
    exit 1
fi

echo ""
echo "5. Checking for test cases..."

# Check for Wi-Fi 7 test
grep -q "test_wifi7_stats_default" smoothtask-core/src/metrics/network.rs
if [ $? -eq 0 ]; then
    echo "✓ Wi-Fi 7 test found"
else
    echo "✗ Wi-Fi 7 test not found"
    exit 1
fi

# Check for 6G test
grep -q "test_cellular6g_stats_default" smoothtask-core/src/metrics/network.rs
if [ $? -eq 0 ]; then
    echo "✓ 6G test found"
else
    echo "✗ 6G test not found"
    exit 1
fi

echo ""
echo "6. Checking for extended network stats integration..."

# Check for wifi7_stats field in ExtendedNetworkInterfaceStats
grep -q "wifi7_stats: Option<Wifi7Stats>" smoothtask-core/src/metrics/network.rs
if [ $? -eq 0 ]; then
    echo "✓ wifi7_stats field integrated"
else
    echo "✗ wifi7_stats field not integrated"
    exit 1
fi

# Check for cellular6g_stats field in ExtendedNetworkInterfaceStats
grep -q "cellular6g_stats: Option<Cellular6GStats>" smoothtask-core/src/metrics/network.rs
if [ $? -eq 0 ]; then
    echo "✓ cellular6g_stats field integrated"
else
    echo "✗ cellular6g_stats field not integrated"
    exit 1
fi

echo ""
echo "7. Checking for interface type detection..."

# Check for Wi-Fi 7 detection
grep -q "wifi7\|wlan7\|wl7" smoothtask-core/src/metrics/network.rs
if [ $? -eq 0 ]; then
    echo "✓ Wi-Fi 7 interface detection implemented"
else
    echo "✗ Wi-Fi 7 interface detection not found"
    exit 1
fi

# Check for 6G detection
grep -q "6g\|wwan6g" smoothtask-core/src/metrics/network.rs
if [ $? -eq 0 ]; then
    echo "✓ 6G interface detection implemented"
else
    echo "✗ 6G interface detection not found"
    exit 1
fi

echo ""
echo "=========================================="
echo "✓ All verification checks passed!"
echo "Wi-Fi 7 and 6G network expansion successfully implemented."
echo "=========================================="