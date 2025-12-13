# eBPF API Documentation for SmoothTask

This document provides comprehensive API documentation for the eBPF (extended Berkeley Packet Filter) module in SmoothTask, covering all public interfaces, structures, and usage patterns.

## Table of Contents

1. [Overview](#overview)
2. [Module Structure](#module-structure)
3. [Configuration (EbpfConfig)](#configuration-ebpfconfig)
4. [Main Structures](#main-structures)
5. [EbpfMetricsCollector API](#ebpfmetricscollector-api)
6. [Filtering and Aggregation](#filtering-and-aggregation)
7. [Memory Optimization](#memory-optimization)
8. [Temperature Monitoring](#temperature-monitoring)
9. [Utility Functions](#utility-functions)
10. [Error Handling](#error-handling)
11. [Performance Optimization](#performance-optimization)
12. [Integration Examples](#integration-examples)
13. [Best Practices](#best-practices)

## Overview

The eBPF module in SmoothTask provides high-performance system metrics collection using eBPF technology. It allows gathering detailed system information with minimal overhead by running programs directly in the Linux kernel.

### Key Features

- **CPU Metrics**: User/system/idle time tracking
- **Memory Metrics**: Memory usage and allocation patterns
- **System Call Monitoring**: Detailed syscall statistics
- **Network Monitoring**: Packet and byte counters
- **Network Connections**: Active connection tracking
- **GPU Monitoring**: GPU usage and memory statistics
- **Filesystem Monitoring**: File operation tracking
- **Process Monitoring**: Process-specific metrics
- **Parallel Collection**: Optimized data gathering
- **Caching**: Reduced overhead for frequent polling
- **Parallel Program Loading**: Simultaneous loading of multiple eBPF programs
- **Program Caching**: Reuse of loaded eBPF programs to reduce initialization time
- **Timeout Support**: Configurable timeouts for eBPF operations
- **Memory Optimization**: Automatic memory cleanup and optimization

### Requirements

- Linux kernel 5.4+ (for full eBPF support)
- CAP_BPF capability or root privileges
- `libbpf-rs` library
- Feature flag `"ebpf"` enabled during compilation

## Module Structure

```
smoothtask-core/src/metrics/ebpf.rs
├── Configuration (EbpfConfig)
├── Data Structures (EbpfMetrics, GpuStat, etc.)
├── Core Collector (EbpfMetricsCollector)
├── Utility Functions (load_ebpf_program_from_file, etc.)
└── Integration Points
```

## Configuration (EbpfConfig)

The `EbpfConfig` structure defines all configurable parameters for the eBPF module.

### Structure Definition

```rust
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EbpfConfig {
    /// Enable CPU metrics collection via eBPF
    pub enable_cpu_metrics: bool,
    
    /// Enable memory metrics collection via eBPF
    pub enable_memory_metrics: bool,
    
    /// Enable system call monitoring
    pub enable_syscall_monitoring: bool,
    
    /// Enable network activity monitoring
    pub enable_network_monitoring: bool,
    
    /// Enable network connections monitoring
    pub enable_network_connections: bool,
    
    /// Enable GPU performance monitoring
    pub enable_gpu_monitoring: bool,
    
    /// Enable CPU temperature monitoring via eBPF
    pub enable_cpu_temperature_monitoring: bool,
    
    /// Enable filesystem operations monitoring
    pub enable_filesystem_monitoring: bool,
    
    /// Enable process-specific metrics monitoring
    pub enable_process_monitoring: bool,
    
    /// Metrics collection interval
    pub collection_interval: Duration,
    
    /// Enable metrics caching to reduce overhead
    pub enable_caching: bool,
    
    /// Batch size for batch processing
    pub batch_size: usize,
    
    /// Maximum initialization attempts
    pub max_init_attempts: usize,
    
    /// Timeout for eBPF operations (milliseconds)
    pub operation_timeout_ms: u64,
    
    /// Enable high-performance mode (optimized eBPF programs)
    pub enable_high_performance_mode: bool,
    
    /// Enable aggressive caching (reduces accuracy but significantly lowers overhead)
    pub enable_aggressive_caching: bool,
    
    /// Aggressive cache interval (milliseconds)
    pub aggressive_cache_interval_ms: u64,
}
```

### New Configuration Options

The following options were added for performance optimization:

- `enable_high_performance_mode`: Uses optimized eBPF programs for better performance
- `enable_aggressive_caching`: Enables aggressive caching to reduce overhead at the cost of accuracy
- `aggressive_cache_interval_ms`: Interval for aggressive caching in milliseconds
    /// Enable high-performance mode (optimized eBPF programs)
    pub enable_high_performance_mode: bool,
    
    /// Enable aggressive caching (reduces accuracy but significantly lowers overhead)
    pub enable_aggressive_caching: bool,
    
    /// Aggressive cache interval (milliseconds)
    pub aggressive_cache_interval_ms: u64,
}
```

### Default Configuration

```rust
impl Default for EbpfConfig {
    fn default() -> Self {
        Self {
            enable_cpu_metrics: true,
            enable_memory_metrics: true,
            enable_syscall_monitoring: false,
            enable_network_monitoring: false,
            enable_network_connections: false,
            enable_gpu_monitoring: false,
            enable_cpu_temperature_monitoring: true, // Enabled by default
            enable_filesystem_monitoring: false,
            enable_process_monitoring: false,
            collection_interval: Duration::from_secs(1),
            enable_caching: true,
            batch_size: 100,
            max_init_attempts: 3,
            operation_timeout_ms: 1000,
            enable_high_performance_mode: true,
            enable_aggressive_caching: false,
            aggressive_cache_interval_ms: 5000,
        }
    }
}
```

### Configuration Recommendations

- **Production Systems**: Enable only necessary metrics to reduce overhead
- **Development/Testing**: Enable all metrics for comprehensive monitoring
- **High-Load Systems**: Use aggressive caching with appropriate intervals
- **Low-Latency Systems**: Disable caching for real-time accuracy

## Main Structures

### EbpfMetrics

The main metrics structure containing all collected eBPF data.

```rust
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EbpfMetrics {
    /// CPU usage percentage
    pub cpu_usage: f64,
    
    /// Memory usage in bytes
    pub memory_usage: u64,
    
    /// Total system call count
    pub syscall_count: u64,
    
    /// Network packets count
    pub network_packets: u64,
    
    /// Network bytes count
    pub network_bytes: u64,
    
    /// Active network connections count
    pub active_connections: u64,
    
    /// GPU usage percentage
    pub gpu_usage: f64,
    
    /// GPU memory usage in bytes
    pub gpu_memory_usage: u64,
    
    /// Filesystem operations count
    pub filesystem_ops: u64,
    
    /// Active processes count
    pub active_processes: u64,
    
    /// Timestamp in nanoseconds
    pub timestamp: u64,
    
    /// Detailed system call statistics (optional)
    pub syscall_details: Option<Vec<SyscallStat>>,
    
    /// Detailed network activity statistics (optional)
    pub network_details: Option<Vec<NetworkStat>>,
    
    /// Detailed network connection statistics (optional)
    pub connection_details: Option<Vec<ConnectionStat>>,
    
    /// Detailed GPU performance statistics (optional)
    pub gpu_details: Option<Vec<GpuStat>>,
    
    /// Detailed filesystem operation statistics (optional)
    pub filesystem_details: Option<Vec<FilesystemStat>>,
    
    /// Detailed process-specific statistics (optional)
    pub process_details: Option<Vec<ProcessStat>>,
}
```

### Supporting Structures

#### GpuStat

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GpuStat {
    /// GPU device identifier
    pub gpu_id: u32,
    
    /// GPU usage percentage
    pub gpu_usage: f64,
    
    /// GPU memory usage in bytes
    pub memory_usage: u64,
    
    /// Number of active compute units
    pub compute_units_active: u32,
    
    /// Power usage in microwatts
    pub power_usage_uw: u64,
}
```

#### FilesystemStat

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FilesystemStat {
    /// File identifier
    pub file_id: u32,
    
    /// Read operations count
    pub read_count: u64,
    
    /// Write operations count
    pub write_count: u64,
    
    /// Open operations count
    pub open_count: u64,
    
    /// Close operations count
    pub close_count: u64,
    
    /// Bytes read
    pub bytes_read: u64,
    
    /// Bytes written
    pub bytes_written: u64,
}
```

#### ConnectionStat

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ConnectionStat {
    /// Source IP address
    pub src_ip: u32,
    
    /// Destination IP address
    pub dst_ip: u32,
    
    /// Source port
    pub src_port: u16,
    
    /// Destination port
    pub dst_port: u16,
    
    /// Protocol (TCP/UDP)
    pub protocol: u8,
    
    /// Connection state
    pub state: u8,
    
    /// Packets count
    pub packets: u64,
    
    /// Bytes count
    pub bytes: u64,
    
    /// Connection start time
    pub start_time: u64,
    
    /// Last activity time
    pub last_activity: u64,
}
```

#### ProcessStat

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProcessStat {
    /// Process identifier
    pub pid: u32,
    
    /// Thread group identifier
    pub tgid: u32,
    
    /// Parent process identifier
    pub ppid: u32,
    
    /// CPU time in nanoseconds
    pub cpu_time: u64,
    
    /// Memory usage in bytes
    pub memory_usage: u64,
    
    /// System call count
    pub syscall_count: u64,
    
    /// I/O bytes count
    pub io_bytes: u64,
    
    /// Process start time
    pub start_time: u64,
    
    /// Last activity time
    pub last_activity: u64,
    
    /// Process name
    pub name: String,
}
```

#### NetworkStat

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NetworkStat {
    /// IP address (simplified)
    pub ip_address: u32,
    
    /// Packets sent
    pub packets_sent: u64,
    
    /// Packets received
    pub packets_received: u64,
    
    /// Bytes sent
    pub bytes_sent: u64,
    
    /// Bytes received
    pub bytes_received: u64,
}
```

#### SyscallStat

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SyscallStat {
    /// System call identifier
    pub syscall_id: u32,
    
    /// Call count
    pub count: u64,
    
    /// Total execution time in nanoseconds
    pub total_time_ns: u64,
    
    /// Average execution time in nanoseconds
    pub avg_time_ns: u64,
}
```

## EbpfMetricsCollector API

The main interface for collecting eBPF metrics.

### Constructor

```rust
/// Create a new eBPF metrics collector
pub fn new(config: EbpfConfig) -> Self
```

## Filtering and Aggregation

The eBPF module provides advanced filtering and aggregation capabilities to optimize data collection and reduce overhead.

### EbpfFilterConfig

The `EbpfFilterConfig` structure defines all configurable parameters for filtering and aggregation.

```rust
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EbpfFilterConfig {
    /// Enable kernel-level filtering
    pub enable_kernel_filtering: bool,
    
    /// Minimum CPU usage threshold (in percentage)
    pub cpu_usage_threshold: f64,
    
    /// Minimum memory usage threshold (in bytes)
    pub memory_usage_threshold: u64,
    
    /// Minimum system call count threshold
    pub syscall_count_threshold: u64,
    
    /// Minimum network traffic threshold (in bytes)
    pub network_traffic_threshold: u64,
    
    /// Minimum active connections threshold
    pub active_connections_threshold: u64,
    
    /// Minimum GPU usage threshold (in percentage)
    pub gpu_usage_threshold: f64,
    
    /// Minimum GPU memory threshold (in bytes)
    pub gpu_memory_threshold: u64,
    
    /// Enable kernel-level aggregation
    pub enable_kernel_aggregation: bool,
    
    /// Aggregation interval (in milliseconds)
    pub aggregation_interval_ms: u64,
    
    /// Maximum number of aggregated entries
    pub max_aggregated_entries: usize,
    
    /// Enable PID filtering
    pub enable_pid_filtering: bool,
    
    /// List of PIDs to filter
    pub filtered_pids: Vec<u32>,
    
    /// Enable syscall type filtering
    pub enable_syscall_type_filtering: bool,
    
    /// List of syscall types to filter
    pub filtered_syscall_types: Vec<u32>,
    
    /// Enable network protocol filtering
    pub enable_network_protocol_filtering: bool,
    
    /// List of network protocols to filter (TCP=6, UDP=17, etc.)
    pub filtered_network_protocols: Vec<u8>,
    
    /// Enable port range filtering
    pub enable_port_range_filtering: bool,
    
    /// Minimum port number for filtering
    pub min_port: u16,
    
    /// Maximum port number for filtering
    pub max_port: u16,
    
    /// Enable process type filtering
    pub enable_process_type_filtering: bool,
    
    /// List of process types to filter
    pub filtered_process_types: Vec<String>,
    
    /// Enable process category filtering
    pub enable_process_category_filtering: bool,
    
    /// List of process categories to filter
    pub filtered_process_categories: Vec<String>,
    
    /// Enable process priority filtering
    pub enable_process_priority_filtering: bool,
    
    /// Minimum process priority for filtering
    pub min_process_priority: i32,
    
    /// Maximum process priority for filtering
    pub max_process_priority: i32,
}
```

### Default Filter Configuration

```rust
impl Default for EbpfFilterConfig {
    fn default() -> Self {
        Self {
            enable_kernel_filtering: false,
            cpu_usage_threshold: 1.0,
            memory_usage_threshold: 1024 * 1024, // 1 MB
            syscall_count_threshold: 10,
            network_traffic_threshold: 1024, // 1 KB
            active_connections_threshold: 5,
            gpu_usage_threshold: 1.0,
            gpu_memory_threshold: 1024 * 1024, // 1 MB
            enable_kernel_aggregation: false,
            aggregation_interval_ms: 1000, // 1 second
            max_aggregated_entries: 1000,
            enable_pid_filtering: false,
            filtered_pids: Vec::new(),
            enable_syscall_type_filtering: false,
            filtered_syscall_types: Vec::new(),
            enable_network_protocol_filtering: false,
            filtered_network_protocols: Vec::new(),
            enable_port_range_filtering: false,
            min_port: 0,
            max_port: 65535,
            enable_process_type_filtering: false,
            filtered_process_types: Vec::new(),
            enable_process_category_filtering: false,
            filtered_process_categories: Vec::new(),
            enable_process_priority_filtering: false,
            min_process_priority: -20, // Minimum priority (highest)
            max_process_priority: 19,  // Maximum priority (lowest)
        }
    }
}
```

### Filtering Functions

#### Set Filter Configuration

```rust
/// Set filter configuration
pub fn set_filter_config(&mut self, filter_config: EbpfFilterConfig)
```

**Parameters:**
- `filter_config`: `EbpfFilterConfig` - Configuration for filtering

**Example:**
```rust
let filter_config = EbpfFilterConfig {
    enable_kernel_filtering: true,
    cpu_usage_threshold: 5.0,
    memory_usage_threshold: 1024 * 1024,
    ..Default::default()
};

collector.set_filter_config(filter_config);
```

#### Apply Filtering

```rust
/// Apply filtering to collected metrics
pub fn apply_filtering(&self, metrics: &mut EbpfMetrics)
```

**Parameters:**
- `metrics`: `&mut EbpfMetrics` - Metrics to filter

**Behavior:**
- Filters metrics based on configured thresholds
- Removes entries below threshold values
- Applies PID, syscall type, and network protocol filtering
- Logs filtering operations

**Example:**
```rust
let mut metrics = collector.collect_metrics()?;
collector.apply_filtering(&mut metrics);
```

#### Set PID Filtering

```rust
/// Set PID filtering
pub fn set_pid_filtering(&mut self, enable: bool, pids: Vec<u32>)
```

**Parameters:**
- `enable`: `bool` - Enable/disable PID filtering
- `pids`: `Vec<u32>` - List of PIDs to filter

**Example:**
```rust
collector.set_pid_filtering(true, vec![100, 200, 300]);
```

#### Set Syscall Type Filtering

```rust
/// Set syscall type filtering
pub fn set_syscall_type_filtering(&mut self, enable: bool, syscall_types: Vec<u32>)
```

**Parameters:**
- `enable`: `bool` - Enable/disable syscall type filtering
- `syscall_types`: `Vec<u32>` - List of syscall types to filter

**Example:**
```rust
collector.set_syscall_type_filtering(true, vec![4, 5, 6]);
```

#### Set Network Protocol Filtering

```rust
/// Set network protocol filtering
pub fn set_network_protocol_filtering(&mut self, enable: bool, protocols: Vec<u8>)
```

**Parameters:**
- `enable`: `bool` - Enable/disable network protocol filtering
- `protocols`: `Vec<u8>` - List of network protocols to filter (TCP=6, UDP=17, etc.)

**Example:**
```rust
collector.set_network_protocol_filtering(true, vec![6, 17]); // TCP and UDP
```

#### Set Port Range Filtering

```rust
/// Set port range filtering
pub fn set_port_range_filtering(&mut self, enable: bool, min_port: u16, max_port: u16)
```

**Parameters:**
- `enable`: `bool` - Enable/disable port range filtering
- `min_port`: `u16` - Minimum port number
- `max_port`: `u16` - Maximum port number

**Example:**
```rust
collector.set_port_range_filtering(true, 1024, 65535);
```

#### Set Aggregation Parameters

```rust
/// Set aggregation parameters
pub fn set_aggregation_parameters(&mut self, enable: bool, interval_ms: u64, max_entries: usize)
```

**Parameters:**
- `enable`: `bool` - Enable/disable aggregation
- `interval_ms`: `u64` - Aggregation interval in milliseconds
- `max_entries`: `usize` - Maximum number of aggregated entries

**Example:**
```rust
collector.set_aggregation_parameters(true, 1000, 500);
```

#### Set Filtering Thresholds

```rust
/// Set filtering thresholds
pub fn set_filtering_thresholds(&mut self, 
    cpu_threshold: f64, 
    memory_threshold: u64, 
    syscall_threshold: u64, 
    network_threshold: u64, 
    connections_threshold: u64, 
    gpu_usage_threshold: f64, 
    gpu_memory_threshold: u64)
```

**Parameters:**
- `cpu_threshold`: `f64` - CPU usage threshold
- `memory_threshold`: `u64` - Memory usage threshold
- `syscall_threshold`: `u64` - Syscall count threshold
- `network_threshold`: `u64` - Network traffic threshold
- `connections_threshold`: `u64` - Active connections threshold
- `gpu_usage_threshold`: `f64` - GPU usage threshold
- `gpu_memory_threshold`: `u64` - GPU memory threshold

**Example:**
```rust
collector.set_filtering_thresholds(5.0, 1024 * 1024, 50, 1024, 2, 5.0, 1024 * 1024);
```

### Aggregation Functions

#### Apply Aggregation

```rust
/// Apply aggregation to collected metrics
pub fn apply_aggregation(&self, metrics: &mut EbpfMetrics)
```

**Parameters:**
- `metrics`: `&mut EbpfMetrics` - Metrics to aggregate

**Behavior:**
- Aggregates detailed statistics by type
- Limits number of entries to configured maximum
- Preserves most significant entries
- Logs aggregation operations

**Example:**
```rust
let mut metrics = collector.collect_metrics()?;
collector.apply_aggregation(&mut metrics);
```

#### Apply Filtering and Aggregation

```rust
/// Apply both filtering and aggregation
pub fn apply_filtering_and_aggregation(&self, metrics: &mut EbpfMetrics)
```

**Parameters:**
- `metrics`: `&mut EbpfMetrics` - Metrics to process

**Behavior:**
- Applies filtering first, then aggregation
- Combines both operations for optimal performance

**Example:**
```rust
let mut metrics = collector.collect_metrics()?;
collector.apply_filtering_and_aggregation(&mut metrics);
```

## Memory Optimization

The eBPF module provides comprehensive memory optimization features to reduce memory footprint and improve performance.

### Memory Optimization Functions

#### Optimize eBPF Memory Usage

```rust
/// Optimize memory usage in eBPF maps
pub fn optimize_ebpf_memory_usage(&mut self) -> Result<()>
```

**Returns:**
- `Result<()>` - Ok if optimization successful, Err with error details

**Behavior:**
- Optimizes memory usage in all eBPF maps
- Cleans up unused entries
- Reduces memory footprint
- Logs optimization results

**Example:**
```rust
collector.optimize_ebpf_memory_usage()?;
```

#### Set Maximum Cached Details

```rust
/// Set maximum number of cached detailed statistics
pub fn set_max_cached_details(&mut self, max_details: usize)
```

**Parameters:**
- `max_details`: `usize` - Maximum number of detailed statistics to cache

**Example:**
```rust
collector.set_max_cached_details(100);
```

#### Get Maximum Cached Details

```rust
/// Get current maximum number of cached detailed statistics
pub fn get_max_cached_details(&self) -> usize
```

**Returns:**
- `usize` - Current maximum number of cached details

**Example:**
```rust
let max_details = collector.get_max_cached_details();
```

#### Enable/Disable Cleanup of Unused Maps

```rust
/// Enable or disable cleanup of unused eBPF maps
pub fn set_cleanup_unused_maps(&mut self, enabled: bool)
```

**Parameters:**
- `enabled`: `bool` - Enable/disable cleanup

**Example:**
```rust
collector.set_cleanup_unused_maps(true);
```

## Temperature Monitoring

The eBPF module provides advanced temperature monitoring capabilities for both CPU and GPU.

### Temperature Monitoring Structures

#### CpuTemperatureStat

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CpuTemperatureStat {
    /// CPU core identifier
    pub cpu_id: u32,
    
    /// Current CPU temperature (in Celsius)
    pub temperature_celsius: u32,
    
    /// Maximum CPU temperature (in Celsius)
    pub max_temperature_celsius: u32,
    
    /// Time of last update
    pub last_update_time: u64,
}
```

### Temperature Monitoring Functions

#### Collect GPU Temperature

```rust
/// Collect GPU temperature from eBPF maps
fn collect_gpu_temperature_from_maps(&self) -> Result<u32>
```

**Returns:**
- `Result<u32>` - GPU temperature in Celsius or error

**Behavior:**
- Collects temperature data from GPU eBPF maps
- Averages temperature across all GPU devices
- Handles errors gracefully

**Example:**
```rust
let gpu_temp = collector.collect_gpu_temperature_from_maps()?;
```

#### Collect CPU Temperature

```rust
/// Collect CPU temperature from eBPF maps
fn collect_cpu_temperature_from_maps(&self) -> Result<Vec<CpuTemperatureStat>>
```

**Returns:**
- `Result<Vec<CpuTemperatureStat>>` - Vector of CPU temperature statistics or error

**Behavior:**
- Collects temperature data from CPU eBPF maps
- Returns detailed statistics for each CPU core
- Handles errors gracefully

**Example:**
```rust
let cpu_temps = collector.collect_cpu_temperature_from_maps()?;
```

#### Collect CPU Temperature Data

```rust
/// Collect comprehensive CPU temperature data
fn collect_cpu_temperature_data(&self) -> Result<(u32, u32, Option<Vec<CpuTemperatureStat>>)>
```

**Returns:**
- `Result<(u32, u32, Option<Vec<CpuTemperatureStat>>)>` - Tuple containing average temperature, max temperature, and detailed statistics

**Behavior:**
- Collects comprehensive CPU temperature data
- Calculates average and maximum temperatures
- Returns detailed statistics if available

**Example:**
```rust
let (avg_temp, max_temp, details) = collector.collect_cpu_temperature_data()?;
```

### Temperature Monitoring Configuration

**Note**: CPU temperature monitoring is now enabled by default in the eBPF configuration. This provides automatic integration with the main system metrics collection.

To customize temperature monitoring, configure the `EbpfConfig`:

```rust
let config = EbpfConfig {
    enable_cpu_temperature_monitoring: true, // Enabled by default
    enable_gpu_monitoring: true,
    ..Default::default()
};
```

### Temperature Monitoring Integration

CPU temperature monitoring is now automatically integrated with the main system metrics collection in `smoothtask-core`. When eBPF temperature data is available, it takes precedence over traditional sysfs/hwmon temperature readings for more accurate and detailed monitoring.

The integration works as follows:

1. **Primary Collection**: Traditional `collect_temperature_metrics()` function gathers temperature data from sysfs/hwmon interfaces
2. **eBPF Enhancement**: If eBPF temperature monitoring is available and provides valid data, it overrides the traditional temperature values
3. **Fallback**: If eBPF is not available or fails, the system gracefully falls back to traditional temperature monitoring

This provides the best of both worlds: high-precision eBPF monitoring when available, with reliable fallback to standard interfaces.

### Temperature Monitoring in Metrics

Temperature data is included in the main `EbpfMetrics` structure:

```rust
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EbpfMetrics {
    // ... other fields ...
    
    /// GPU temperature in Celsius
    pub gpu_temperature: u32,
    
    /// CPU temperature in Celsius
    pub cpu_temperature: u32,
    
    /// Maximum CPU temperature in Celsius
    pub cpu_max_temperature: u32,
    
    /// Detailed CPU temperature statistics (optional)
    pub cpu_temperature_details: Option<Vec<CpuTemperatureStat>>,
    
    // ... other fields ...
}
```

## EbpfMetricsCollector API

The main interface for collecting eBPF metrics.

### Constructor

```rust
/// Create a new eBPF metrics collector
pub fn new(config: EbpfConfig) -> Self
```

**Parameters:**
- `config`: `EbpfConfig` - Configuration for the collector

**Returns:**
- `Self` - New instance of `EbpfMetricsCollector`

**Example:**
```rust
let config = EbpfConfig {
    enable_cpu_metrics: true,
    enable_memory_metrics: true,
    ..Default::default()
};

let mut collector = EbpfMetricsCollector::new(config);
```

### Initialization

```rust
/// Initialize eBPF programs
pub fn initialize(&mut self) -> Result<()>
```

**Returns:**
- `Result<()>` - Ok if initialization successful, Err with error details

**Behavior:**
- Checks eBPF support in the system
- Loads configured eBPF programs
- Sets up eBPF maps for data collection
- Handles errors gracefully with detailed logging

**Example:**
```rust
match collector.initialize() {
    Ok(_) => println!("eBPF initialized successfully"),
    Err(e) => eprintln!("Failed to initialize eBPF: {}", e),
}
```

### Optimized Initialization

```rust
/// Initialize eBPF programs with optimization
#[cfg(feature = "ebpf")]
pub fn initialize_optimized(&mut self) -> Result<()>
```

**Returns:**
- `Result<()>` - Ok if initialization successful, Err with error details

**Behavior:**
- Uses parallel program loading for faster initialization
- Implements program caching to reduce redundant loads
- Provides detailed performance statistics
- Automatically selects optimized eBPF program versions when available

**Example:**
```rust
match collector.initialize_optimized() {
    Ok(_) => println!("eBPF initialized successfully with optimizations"),
    Err(e) => eprintln!("Failed to initialize eBPF: {}", e),
}
```

### Program Management

```rust
/// Save program and load maps
#[cfg(feature = "ebpf")]
fn save_program_and_load_maps(&mut self, program_type: &str, program: Program, program_path: &str, map_name: &str) -> Result<()>
```

**Parameters:**
- `program_type`: Type of program ("cpu", "memory", etc.)
- `program`: Loaded eBPF program
- `program_path`: Path to the program file
- `map_name`: Name of the map to load

**Returns:**
- `Result<()>` - Ok if program and maps loaded successfully

**Behavior:**
- Stores program in appropriate field based on type
- Loads and stores associated eBPF maps
- Provides detailed logging of loaded maps

### Metrics Collection

```rust
/// Collect current metrics
pub fn collect_metrics(&mut self) -> Result<EbpfMetrics>
```

**Returns:**
- `Result<EbpfMetrics>` - Collected metrics or error

**Behavior:**
- Returns cached metrics if caching is enabled
- Collects fresh metrics if caching is disabled or cache expired
- Handles both initialized and uninitialized states gracefully
- Applies memory optimization techniques

**Example:**
```rust
let metrics = collector.collect_metrics()?;
println!("CPU Usage: {}%", metrics.cpu_usage);
println!("Memory Usage: {} MB", metrics.memory_usage / 1024 / 1024);
```

### Status and Information

```rust
/// Check if eBPF is initialized
pub fn is_initialized(&self) -> bool
```

```rust
/// Check if eBPF support is enabled
pub fn is_ebpf_enabled() -> bool
```

```rust
/// Get last initialization error
pub fn get_last_error(&self) -> Option<&str>
```

```rust
/// Get detailed error information
pub fn get_detailed_error_info(&self) -> Option<String>
```

```rust
/// Check if there are active errors
pub fn has_errors(&self) -> bool
```

```rust
/// Get maps information
pub fn get_maps_info(&self) -> String
```

```rust
/// Check maps availability
pub fn check_maps_availability(&self) -> bool
```

```rust
/// Get initialization statistics
pub fn get_initialization_stats(&self) -> (usize, usize)
```

### Configuration Management

```rust
/// Set maximum cached details
pub fn set_max_cached_details(&mut self, max_details: usize)
```

```rust
/// Enable/disable cleanup of unused maps
pub fn set_cleanup_unused_maps(&mut self, enabled: bool)
```

### Program Cache Management

```rust
/// Get program cache statistics
#[cfg(feature = "ebpf")]
pub fn get_program_cache_stats(&self) -> (u64, u64, f64)
```

**Returns:**
- `(hit_count, miss_count, hit_rate)` - Cache performance metrics

**Example:**
```rust
let (hits, misses, hit_rate) = collector.get_program_cache_stats();
println!("Cache stats: {} hits, {} misses, {:.1}% hit rate", hits, misses, hit_rate);
```

```rust
/// Clear program cache
#[cfg(feature = "ebpf")]
pub fn clear_program_cache(&mut self)
```

**Behavior:**
- Clears all cached eBPF programs
- Resets cache statistics
- Forces reloading of programs on next access

### Recovery and Reset

```rust
/// Attempt recovery after errors
pub fn attempt_recovery(&mut self) -> Result<()>
```

```rust
/// Reset collector state (for testing)
pub fn reset(&mut self)
```

### Performance Optimization

```rust
/// Optimize memory usage
pub fn optimize_memory_usage(&mut self)
```

```rust
/// Get memory usage estimate
pub fn get_memory_usage_estimate(&self) -> usize
```

## Utility Functions

### eBPF Program Loading

```rust
/// Load eBPF program from file
#[cfg(feature = "ebpf")]
fn load_ebpf_program_from_file(program_path: &str) -> Result<Program>
```

**Parameters:**
- `program_path`: Path to the eBPF program file

**Returns:**
- `Result<Program>` - Loaded eBPF program or error

**Behavior:**
- Checks if file exists
- Loads program using libbpf-rs
- Provides detailed error logging

### eBPF Program Loading with Timeout

```rust
/// Load eBPF program from file with timeout
#[cfg(feature = "ebpf")]
fn load_ebpf_program_from_file_with_timeout(program_path: &str, timeout_ms: u64) -> Result<Program>
```

**Parameters:**
- `program_path`: Path to the eBPF program file
- `timeout_ms`: Timeout in milliseconds for the loading operation

**Returns:**
- `Result<Program>` - Loaded eBPF program or error

**Behavior:**
- Checks if file exists
- Loads program using libbpf-rs with timeout tracking
- Logs performance metrics and warnings if timeout is exceeded

### Parallel eBPF Program Loading

```rust
/// Parallel loading of multiple eBPF programs
#[cfg(feature = "ebpf")]
fn load_ebpf_programs_parallel(program_paths: Vec<&str>, timeout_ms: u64) -> Result<Vec<Option<Program>>>
```

**Parameters:**
- `program_paths`: Vector of paths to eBPF program files
- `timeout_ms`: Timeout in milliseconds for each program load

**Returns:**
- `Result<Vec<Option<Program>>>` - Vector of loaded programs (Some) or errors (None)

**Behavior:**
- Loads multiple programs simultaneously using separate threads
- Provides detailed statistics on success/failure rates
- Optimizes initialization time significantly

### eBPF Map Loading

```rust
/// Load eBPF maps from program
#[cfg(feature = "ebpf")]
fn load_maps_from_program(program_path: &str, expected_map_name: &str) -> Result<Vec<Map>>
```

**Parameters:**
- `program_path`: Path to the eBPF program file
- `expected_map_name`: Name of the map to load

**Returns:**
- `Result<Vec<Map>>` - Vector of loaded maps or error

### eBPF Map Iteration

```rust
/// Iterate over all keys in eBPF map and collect data
#[cfg(feature = "ebpf")]
fn iterate_ebpf_map_keys<T: Default + Copy>(map: &Map, value_size: usize) -> Result<Vec<T>>
```

**Parameters:**
- `map`: Reference to the eBPF map
- `value_size`: Expected value size in bytes for validation

**Returns:**
- `Result<Vec<T>>` - Vector of values of type T extracted from the map

### eBPF Program Cache

```rust
/// eBPF Program Cache for performance optimization
struct EbpfProgramCache {
    cache: HashMap<String, Program>,
    hit_count: u64,
    miss_count: u64,
}
```

**Methods:**

```rust
/// Get program from cache or load if not present
fn get_or_load(&mut self, program_path: &str, timeout_ms: u64) -> Result<Program>
```

**Parameters:**
- `program_path`: Path to the eBPF program file
- `timeout_ms`: Timeout in milliseconds for loading

**Returns:**
- `Result<Program>` - Loaded program from cache or newly loaded

```rust
/// Clear the program cache
fn clear(&mut self)
```

**Behavior:**
- Clears all cached programs
- Resets cache statistics

```rust
/// Get cache statistics
fn get_stats(&self) -> (u64, u64, f64)
```

**Returns:**
- `(hit_count, miss_count, hit_rate)` - Cache performance metrics

**Behavior:**
- Iterates through all keys in the map
- Extracts and validates values
- Handles errors gracefully
- Provides detailed error logging

### eBPF Support Check

```rust
/// Check eBPF support in the system
pub fn check_ebpf_support() -> Result<bool>
```

**Returns:**
- `Result<bool>` - True if eBPF is supported, false otherwise

**Behavior:**
- Checks Linux kernel version (requires 4.4+)
- Verifies eBPF subsystem availability
- Provides detailed logging

## Error Handling

The eBPF module implements comprehensive error handling:

### Error Types

- **Initialization Errors**: Failed to load eBPF programs or maps
- **Support Errors**: eBPF not supported by the system
- **Permission Errors**: Insufficient privileges for eBPF operations
- **Configuration Errors**: Invalid configuration parameters
- **Runtime Errors**: Errors during metrics collection

### Error Recovery

The module provides automatic error recovery mechanisms:

```rust
/// Attempt recovery after errors
pub fn attempt_recovery(&mut self) -> Result<()>
```

This method:
1. Resets the collector state
2. Attempts re-initialization
3. Logs recovery attempts
4. Returns success/failure status

### Error Information

Detailed error information is available through:

```rust
/// Get detailed error information
pub fn get_detailed_error_info(&self) -> Option<String>
```

```rust
/// Check if there are active errors
pub fn has_errors(&self) -> bool
```

## Performance Optimization

### Caching Strategies

The module implements multiple caching strategies:

1. **Basic Caching**: Enabled by `enable_caching` flag
   - Caches metrics for the duration of `batch_size` collections
   - Reduces overhead for frequent polling

2. **Aggressive Caching**: Enabled by `enable_aggressive_caching` flag
   - Caches metrics for `aggressive_cache_interval_ms` milliseconds
   - Significantly reduces overhead but may impact accuracy
   - Ideal for high-frequency monitoring scenarios

### Memory Optimization

```rust
/// Optimize memory usage
pub fn optimize_memory_usage(&mut self)
```

This method:
1. Limits cached detailed statistics to `max_cached_details`
2. Cleans up unused eBPF maps when `cleanup_unused_maps` is enabled
3. Optimizes internal data structures
4. Provides memory usage estimates

### Memory Cleanup

```rust
/// Perform actual memory cleanup
fn perform_memory_cleanup(&mut self)
```

**Behavior:**
- Cleans up unused eBPF maps
- Optimizes cached metrics
- Limits detailed statistics to configured maximum
- Logs cleanup operations

### Memory Usage Estimation

```rust
/// Get memory usage estimate
pub fn get_memory_usage_estimate(&self) -> usize
```

**Returns:**
- `usize` - Estimated memory usage in bytes

**Behavior:**
- Calculates memory usage based on cached metrics
- Includes detailed statistics in calculation
- Provides approximate memory footprint

### Parallel Collection

The module uses parallel collection for detailed statistics:

```rust
/// Collect detailed statistics in parallel
fn collect_detailed_stats_parallel(&self) -> (
    Option<Vec<SyscallStat>>,
    Option<Vec<NetworkStat>>,
    Option<Vec<ConnectionStat>>,
    Option<Vec<GpuStat>>,
    Option<Vec<ProcessStat>>,
    Option<Vec<FilesystemStat>>
)
```

This method uses separate threads for each metric type to:
- Reduce overall collection time
- Improve responsiveness
- Better utilize multi-core systems

## Integration Examples

### Basic Integration

```rust
use smoothtask_core::metrics::ebpf::{EbpfMetricsCollector, EbpfConfig};

fn main() -> Result<()> {
    // Create configuration
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_syscall_monitoring: true,
        ..Default::default()
    };

    // Create collector
    let mut collector = EbpfMetricsCollector::new(config);

    // Initialize
    collector.initialize()?;

    // Collect metrics
    let metrics = collector.collect_metrics()?;

    println!("System Metrics:");
    println!("  CPU Usage: {}%", metrics.cpu_usage);
    println!("  Memory Usage: {} MB", metrics.memory_usage / 1024 / 1024);
    println!("  System Calls: {}", metrics.syscall_count);

    Ok(())
}
```

### Optimized Integration with Parallel Loading

```rust
use smoothtask_core::metrics::ebpf::{EbpfMetricsCollector, EbpfConfig};

fn main() -> Result<()> {
    // Create configuration with performance optimizations
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_network_monitoring: true,
        enable_network_connections: true,
        enable_high_performance_mode: true,
        enable_aggressive_caching: true,
        aggressive_cache_interval_ms: 5000,
        batch_size: 100,
        ..Default::default()
    };

    // Create collector
    let mut collector = EbpfMetricsCollector::new(config);

    // Use optimized initialization with parallel loading
    collector.initialize_optimized()?;

    // Collect metrics with caching
    let metrics = collector.collect_metrics()?;

    // Get cache performance statistics
    let (hits, misses, hit_rate) = collector.get_program_cache_stats();
    println!("Cache performance: {} hits, {} misses, {:.1}% hit rate", hits, misses, hit_rate);

    Ok(())
}
```

### Advanced Integration with Error Handling

```rust
use smoothtask_core::metrics::ebpf::{EbpfMetricsCollector, EbpfConfig};

fn main() -> Result<()> {
    // Create configuration with more features
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_network_monitoring: true,
        enable_network_connections: true,
        enable_caching: true,
        batch_size: 50,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);

    // Check eBPF support first
    if !EbpfMetricsCollector::check_ebpf_support()? {
        println!("eBPF not supported on this system, falling back to basic metrics");
        // Fallback to non-eBPF metrics collection
        return Ok(());
    }

    // Initialize with error handling
    match collector.initialize() {
        Ok(_) => println!("eBPF initialized successfully"),
        Err(e) => {
            println!("Failed to initialize eBPF: {}", e);
            if let Some(error_info) = collector.get_detailed_error_info() {
                println!("Detailed error: {}", error_info);
            }
            return Ok(()); // Continue with fallback
        }
    }

    // Main monitoring loop
    loop {
        match collector.collect_metrics() {
            Ok(metrics) => {
                println!("Metrics collected successfully");
                println!("  CPU: {}%, Memory: {} MB", 
                    metrics.cpu_usage, 
                    metrics.memory_usage / 1024 / 1024);
                    
                if let Some(details) = metrics.syscall_details {
                    println!("  Top system calls:");
                    for stat in details.iter().take(5) {
                        println!("    Syscall {}: {} calls, avg {} ns",
                            stat.syscall_id, stat.count, stat.avg_time_ns);
                    }
                }
            }
            Err(e) => {
                println!("Error collecting metrics: {}", e);
                // Attempt recovery
                if let Err(recovery_err) = collector.attempt_recovery() {
                    println!("Recovery failed: {}", recovery_err);
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
```

### Integration with Configuration Reloading

```rust
use smoothtask_core::metrics::ebpf::{EbpfMetricsCollector, EbpfConfig};

fn main() -> Result<()> {
    // Initial configuration
    let mut config = EbpfConfig::default();
    config.enable_cpu_metrics = true;
    config.enable_memory_metrics = true;

    let mut collector = EbpfMetricsCollector::new(config);
    collector.initialize()?;

    // Configuration reload simulation
    fn reload_config(collector: &mut EbpfMetricsCollector) -> Result<()> {
        // New configuration (could come from file, API, etc.)
        let new_config = EbpfConfig {
            enable_cpu_metrics: true,
            enable_memory_metrics: true,
            enable_network_monitoring: true, // New feature enabled
            enable_caching: true,
            aggressive_cache_interval_ms: 10000, // More aggressive caching
            ..Default::default()
        };

        // Create new collector with updated configuration
        let mut new_collector = EbpfMetricsCollector::new(new_config);
        new_collector.initialize()?;

        // Replace old collector (in real code, use Arc<Mutex<>> for thread safety)
        *collector = new_collector;

        println!("Configuration reloaded successfully");
        Ok(())
    }

    // Main loop with periodic config reload
    let mut iteration = 0;
    loop {
        let metrics = collector.collect_metrics()?;
        println!("Iteration {}: CPU {}%, Memory {} MB",
            iteration, metrics.cpu_usage, metrics.memory_usage / 1024 / 1024);

        // Reload config every 10 iterations
        if iteration % 10 == 0 {
            if let Err(e) = reload_config(&mut collector) {
                println!("Config reload failed: {}", e);
            }
        }

        iteration += 1;
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
```

### Integration with Filtering and Aggregation

```rust
use smoothtask_core::metrics::ebpf::{EbpfMetricsCollector, EbpfConfig, EbpfFilterConfig};

fn main() -> Result<()> {
    // Create configuration with filtering enabled
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_syscall_monitoring: true,
        enable_network_monitoring: true,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    collector.initialize()?;

    // Configure filtering
    let filter_config = EbpfFilterConfig {
        enable_kernel_filtering: true,
        cpu_usage_threshold: 5.0,
        memory_usage_threshold: 1024 * 1024,
        syscall_count_threshold: 50,
        network_traffic_threshold: 1024 * 1024,
        ..Default::default()
    };

    collector.set_filter_config(filter_config);

    // Configure PID filtering
    collector.set_pid_filtering(true, vec![100, 200, 300]);

    // Configure network protocol filtering
    collector.set_network_protocol_filtering(true, vec![6, 17]); // TCP and UDP

    // Configure aggregation
    collector.set_aggregation_parameters(true, 1000, 500);

    // Main monitoring loop with filtering and aggregation
    loop {
        let mut metrics = collector.collect_metrics()?;
        
        // Apply filtering and aggregation
        collector.apply_filtering_and_aggregation(&mut metrics);
        
        println!("Filtered Metrics:");
        println!("  CPU Usage: {}%", metrics.cpu_usage);
        println!("  Memory Usage: {} MB", metrics.memory_usage / 1024 / 1024);
        println!("  System Calls: {}", metrics.syscall_count);
        
        if let Some(details) = &metrics.syscall_details {
            println!("  Top system calls after filtering:");
            for stat in details.iter().take(3) {
                println!("    Syscall {}: {} calls", stat.syscall_id, stat.count);
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
```

### Integration with Memory Optimization

```rust
use smoothtask_core::metrics::ebpf::{EbpfMetricsCollector, EbpfConfig};

fn main() -> Result<()> {
    // Create configuration with memory optimization
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_syscall_monitoring: true,
        enable_network_monitoring: true,
        enable_gpu_monitoring: true,
        enable_filesystem_monitoring: true,
        enable_high_performance_mode: true,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    collector.initialize()?;

    // Configure memory optimization
    collector.set_max_cached_details(100);
    collector.set_cleanup_unused_maps(true);

    // Main monitoring loop with periodic memory optimization
    let mut iteration = 0;
    loop {
        let metrics = collector.collect_metrics()?;
        
        println!("Iteration {}: CPU {}%, Memory {} MB",
            iteration, metrics.cpu_usage, metrics.memory_usage / 1024 / 1024);
        
        // Optimize memory every 10 iterations
        if iteration % 10 == 0 {
            collector.optimize_ebpf_memory_usage()?;
            let memory_usage = collector.get_memory_usage_estimate();
            println!("Memory optimized. Current usage: {} bytes", memory_usage);
        }

        iteration += 1;
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
```

### Integration with Temperature Monitoring

```rust
use smoothtask_core::metrics::ebpf::{EbpfMetricsCollector, EbpfConfig};

fn main() -> Result<()> {
    // Create configuration with temperature monitoring
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_cpu_temperature_monitoring: true,
        enable_gpu_monitoring: true,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    collector.initialize()?;

    // Main monitoring loop with temperature monitoring
    loop {
        let metrics = collector.collect_metrics()?;
        
        println!("System Metrics:");
        println!("  CPU Usage: {}%", metrics.cpu_usage);
        println!("  Memory Usage: {} MB", metrics.memory_usage / 1024 / 1024);
        
        println!("Temperature Metrics:");
        println!("  CPU Temperature: {}°C", metrics.cpu_temperature);
        println!("  CPU Max Temperature: {}°C", metrics.cpu_max_temperature);
        println!("  GPU Temperature: {}°C", metrics.gpu_temperature);
        
        if let Some(details) = &metrics.cpu_temperature_details {
            println!("  CPU Core Temperatures:");
            for stat in details {
                println!("    Core {}: {}°C (Max: {}°C)", 
                    stat.cpu_id, stat.temperature_celsius, stat.max_temperature_celsius);
            }
        }

        // Check for overheating
        if metrics.cpu_temperature > 80 {
            println!("WARNING: CPU temperature is high!");
        }
        
        if metrics.gpu_temperature > 85 {
            println!("WARNING: GPU temperature is high!");
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
```

## Best Practices

### Configuration

1. **Enable Only Necessary Features**: Each enabled feature adds overhead
2. **Use Appropriate Caching**: Balance between accuracy and performance
3. **Set Reasonable Timeouts**: Avoid hanging on eBPF operations
4. **Limit Detailed Statistics**: Detailed stats consume significant memory

### Error Handling

1. **Check eBPF Support Early**: Before attempting initialization
2. **Implement Fallback Mechanisms**: For systems without eBPF support
3. **Monitor Error State**: Regularly check `has_errors()` and `get_last_error()`
4. **Attempt Recovery**: Use `attempt_recovery()` when errors occur

### Performance

1. **Use Caching**: For high-frequency monitoring scenarios
2. **Limit Parallel Collection**: For systems with many CPU cores
3. **Optimize Memory Usage**: Regularly call `optimize_memory_usage()`
4. **Monitor Memory Consumption**: Use `get_memory_usage_estimate()`
5. **Use Optimized Initialization**: Prefer `initialize_optimized()` for better performance
6. **Enable Program Caching**: Use `enable_high_performance_mode` for faster program loading
7. **Configure Aggressive Caching**: Use `enable_aggressive_caching` for high-frequency monitoring
8. **Monitor Cache Performance**: Regularly check `get_program_cache_stats()`

### Integration

1. **Thread Safety**: Use appropriate synchronization (Arc<Mutex<>>)
2. **Graceful Degradation**: Handle eBPF failures gracefully
3. **Configuration Reloading**: Support dynamic configuration changes
4. **Monitor Initialization**: Check `is_initialized()` before collecting metrics
5. **Use Filtering**: Apply filtering to reduce data volume and improve performance
6. **Apply Aggregation**: Use aggregation for high-volume metrics to reduce memory usage

### Security

1. **Run with Minimum Privileges**: Use CAP_BPF instead of root when possible
2. **Validate eBPF Programs**: Ensure programs are from trusted sources
3. **Monitor Resource Usage**: Prevent excessive memory/CPU consumption
4. **Limit Collection Frequency**: Avoid overwhelming the system
5. **Monitor Temperature**: Use temperature monitoring to prevent hardware damage

### Filtering and Aggregation

1. **Set Appropriate Thresholds**: Configure thresholds based on your monitoring needs
2. **Use PID Filtering**: Filter by specific processes to reduce noise
3. **Apply Network Filtering**: Filter by protocols and ports for network-focused monitoring
4. **Limit Aggregated Entries**: Set reasonable limits for aggregated data
5. **Balance Accuracy vs Performance**: Adjust filtering thresholds based on system load

### Memory Optimization

1. **Regular Optimization**: Call `optimize_ebpf_memory_usage()` periodically
2. **Limit Cached Details**: Set appropriate limits for detailed statistics
3. **Enable Map Cleanup**: Use `set_cleanup_unused_maps(true)` to clean up unused resources
4. **Monitor Memory Usage**: Use `get_memory_usage_estimate()` to track memory consumption
5. **Adjust Based on Load**: Increase memory limits for high-load systems

### Temperature Monitoring

1. **Enable When Needed**: Temperature monitoring adds overhead, enable only when necessary
2. **Set Alert Thresholds**: Monitor for overheating conditions (typically >80°C for CPU, >85°C for GPU)
   - Configure `cpu_temperature_warning_threshold` (default: 75°C)
   - Configure `cpu_temperature_critical_threshold` (default: 90°C)
   - Configure `cpu_max_temperature_warning_threshold` (default: 85°C)
   - Configure `cpu_max_temperature_critical_threshold` (default: 95°C)
3. **Use Detailed Monitoring**: Enable detailed temperature monitoring for troubleshooting
4. **Integrate with Alerting**: Combine temperature data with alerting systems
5. **Monitor Trends**: Track temperature changes over time to detect cooling issues

## Troubleshooting

### Common Issues

1. **eBPF Not Supported**: Check kernel version and configuration
2. **Permission Denied**: Ensure CAP_BPF capability or root privileges
3. **Program Load Failures**: Verify eBPF program files exist and are valid
4. **Map Access Errors**: Check if maps are properly initialized
5. **Performance Issues**: Review caching configuration and enabled features
6. **Filtering Not Working**: Verify filter configuration and thresholds
7. **Memory Optimization Issues**: Check map cleanup settings and cached details limits
8. **Temperature Monitoring Failures**: Ensure temperature monitoring is enabled in configuration

### Debugging

1. **Enable Debug Logging**: Set appropriate log level
2. **Check Error Information**: Use `get_detailed_error_info()`
3. **Monitor Initialization**: Check `get_initialization_stats()`
4. **Verify Maps**: Use `get_maps_info()` and `check_maps_availability()`
5. **Monitor Memory**: Use `get_memory_usage_estimate()`
6. **Check Filter Configuration**: Verify `EbpfFilterConfig` settings
7. **Test Temperature Collection**: Use `collect_cpu_temperature_from_maps()` and `collect_gpu_temperature_from_maps()`

### Performance Tuning

1. **Adjust Caching**: Modify `batch_size` and caching intervals
2. **Limit Features**: Disable unnecessary monitoring features
3. **Optimize Memory**: Adjust `max_cached_details`
4. **Tune Collection Interval**: Balance frequency with overhead
5. **Monitor Overhead**: Measure collection time and resource usage
6. **Adjust Filtering**: Fine-tune filtering thresholds for optimal performance
7. **Optimize Aggregation**: Set appropriate aggregation intervals and entry limits
8. **Balance Temperature Monitoring**: Adjust temperature monitoring frequency based on needs

## Conclusion

The SmoothTask eBPF module provides a powerful and flexible interface for collecting detailed system metrics with minimal overhead. By following the API documentation and best practices, you can effectively integrate eBPF-based monitoring into your applications while maintaining performance and reliability.

For more information about eBPF technology and its capabilities, refer to the official Linux kernel documentation and libbpf resources.