#!/bin/bash
# Test script to verify documentation coverage for dependencies

echo "Testing documentation coverage for system dependencies..."

# Test 1: Check if glib-2.0 is mentioned in SETUP_GUIDE.md
echo "Test 1: Checking for glib-2.0 in SETUP_GUIDE.md..."
if grep -q "glib-2.0\|glib2-devel\|libglib2.0-dev" docs/SETUP_GUIDE.md; then
    echo "✅ PASS: glib-2.0 dependencies are documented"
else
    echo "❌ FAIL: glib-2.0 dependencies are missing from documentation"
    exit 1
fi

# Test 2: Check if pkg-config is mentioned
echo "Test 2: Checking for pkg-config in SETUP_GUIDE.md..."
if grep -q "pkg-config\|pkgconf" docs/SETUP_GUIDE.md; then
    echo "✅ PASS: pkg-config is documented"
else
    echo "❌ FAIL: pkg-config is missing from documentation"
    exit 1
fi

# Test 3: Check if troubleshooting section exists
echo "Test 3: Checking for troubleshooting section..."
if grep -q "Устранение неполадок\|Troubleshooting" docs/SETUP_GUIDE.md; then
    echo "✅ PASS: Troubleshooting section exists"
else
    echo "❌ FAIL: Troubleshooting section is missing"
    exit 1
fi

# Test 4: Check if all major distros are covered
echo "Test 4: Checking for major distribution coverage..."
required_distros=("Ubuntu\|Debian" "Fedora" "Arch" "openSUSE")
all_found=true

for distro in "${required_distros[@]}"; do
    if grep -q "$distro" docs/SETUP_GUIDE.md; then
        echo "  ✅ $distro: covered"
    else
        echo "  ❌ $distro: missing"
        all_found=false
    fi
done

if [ "$all_found" = true ]; then
    echo "✅ PASS: All major distributions are covered"
else
    echo "❌ FAIL: Some distributions are missing"
    exit 1
fi

# Test 5: Check if README links to SETUP_GUIDE
echo "Test 5: Checking if README links to SETUP_GUIDE..."
if grep -q "SETUP_GUIDE" README.md; then
    echo "✅ PASS: README links to SETUP_GUIDE"
else
    echo "❌ FAIL: README doesn't link to SETUP_GUIDE"
    exit 1
fi

echo ""
echo "🎉 All documentation tests passed!"
echo "Documentation coverage is complete for system dependencies."
