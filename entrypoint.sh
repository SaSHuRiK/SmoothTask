#!/bin/bash

# SmoothTask container entrypoint script
# This script handles container-specific setup and runs the SmoothTask daemon

set -e

echo "Starting SmoothTask container..."

# Check if we're running with proper cgroup permissions
echo "Checking cgroup permissions..."
if [ -w /sys/fs/cgroup ]; then
    echo "✓ Cgroup write permissions available"
else
    echo "⚠ Cgroup write permissions not available - some features may be limited"
fi

# Check if we're running in a container
echo "Detecting container environment..."
if [ -f /.dockerenv ] || [ -f /.containerenv ]; then
    echo "✓ Running in container environment"
    CONTAINER_ENV="true"
else
    echo "ℹ Not running in standard container environment"
    CONTAINER_ENV="false"
fi

# Set up environment variables
export RUST_LOG=${RUST_LOG:-info}
export SMOOTHTASK_CONFIG=${SMOOTHTASK_CONFIG:-/etc/smoothtask/config.yml}

# Check if config file exists
if [ ! -f "$SMOOTHTASK_CONFIG" ]; then
    echo "❌ Configuration file not found at $SMOOTHTASK_CONFIG"
    echo "Please mount a configuration file or create one."
    exit 1
fi

echo "Configuration file: $SMOOTHTASK_CONFIG"
echo "Log level: $RUST_LOG"

# Run the daemon
echo "Starting SmoothTask daemon..."
exec /usr/local/bin/smoothtaskd --config "$SMOOTHTASK_CONFIG"
