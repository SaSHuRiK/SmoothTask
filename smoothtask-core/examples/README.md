# SmoothTask Examples

This directory contains example programs demonstrating the usage of SmoothTask core functionality.

## Available Examples

### 1. cgroups_example.rs

**Description**: Demonstrates cgroups v2 utilities for process management.

**Features**:
- Check cgroups v2 availability
- Create and manage cgroups
- Read/write cgroup parameters
- Process management in cgroups

**Usage**:
```bash
cargo run --example cgroups_example
```

### 2. ebpf_example.rs

**Description**: Basic eBPF metrics collection example.

**Features**:
- Check eBPF support
- Configure eBPF metrics collector
- Initialize eBPF programs
- Collect and display system metrics
- Handle errors and recovery
- Configuration management

**Usage**:
```bash
cargo run --example ebpf_example
```

### 3. ebpf_advanced_example.rs

**Description**: Advanced eBPF integration with multi-threading and dynamic configuration.

**Features**:
- Multi-threaded monitoring
- Dynamic configuration updates
- Error handling and recovery
- Metrics analysis and warnings
- Integration patterns
- State management

**Usage**:
```bash
cargo run --example ebpf_advanced_example
```

## Running Examples

To run any example, use the following command:

```bash
cargo run --example <example_name>
```

For example:
```bash
cargo run --example ebpf_example
```

## Requirements

- Rust toolchain (stable or nightly)
- Linux system with appropriate capabilities
- For eBPF examples: Linux kernel 5.4+ and CAP_BPF capability
- Development dependencies (libbpf, etc.)

## Building

All examples can be built using:

```bash
cargo build --examples
```

## Example Structure

Each example follows a similar structure:

1. **Setup**: Initialize required components
2. **Configuration**: Set up configuration parameters
3. **Main Logic**: Demonstrate the core functionality
4. **Cleanup**: Proper resource cleanup
5. **Error Handling**: Comprehensive error handling

## Integration Patterns

The examples demonstrate various integration patterns:

- **Basic Usage**: Simple API calls and metric collection
- **Error Recovery**: Handling and recovering from errors
- **Configuration Management**: Dynamic configuration updates
- **Multi-threading**: Concurrent monitoring and processing
- **State Management**: Application state handling

## Best Practices

The examples illustrate recommended best practices:

- Proper error handling and recovery
- Resource management
- Configuration validation
- Thread safety
- Graceful degradation

## Troubleshooting

If you encounter issues running examples:

1. Check kernel version and eBPF support
2. Verify required capabilities (CAP_BPF)
3. Ensure development dependencies are installed
4. Check system logs for detailed error information

## Contributing

New examples demonstrating additional functionality are welcome. Please follow the existing structure and documentation patterns.