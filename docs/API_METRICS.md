# SmoothTask Metrics API Documentation

This document provides comprehensive API documentation for the SmoothTask metrics module, covering system metrics, process metrics, and other monitoring capabilities.

## Table of Contents

- [System Metrics API](#system-metrics-api)
  - [Structures](#structures)
  - [Functions](#functions)
  - [Usage Examples](#usage-examples)
- [Process Metrics API](#process-metrics-api)
- [GPU Metrics API](#gpu-metrics-api)
- [eBPF Metrics API](#ebpf-metrics-api)
- [Integration Patterns](#integration-patterns)

## System Metrics API

The system metrics module provides comprehensive monitoring of system-level resources including CPU, memory, load averages, PSI (Pressure Stall Information), and more.

### Structures

#### `CpuTimes`

Represents raw CPU counters from `/proc/stat`.

```rust
pub struct CpuTimes {
    pub user: u64,      // User CPU time
    pub nice: u64,      // Nice CPU time
    pub system: u64,    // System CPU time
    pub idle: u64,      // Idle CPU time
    pub iowait: u64,    // I/O wait CPU time
    pub irq: u64,       // IRQ CPU time
    pub softirq: u64,   // Soft IRQ CPU time
    pub steal: u64,     // Stolen CPU time (virtualization)
    pub guest: u64,     // Guest CPU time (virtualization)
    pub guest_nice: u64,// Guest nice CPU time (virtualization)
}
```

#### `CpuUsage`

Normalized CPU usage percentages calculated from `CpuTimes` deltas.

```rust
pub struct CpuUsage {
    pub user: f64,      // User + nice CPU usage percentage
    pub system: f64,    // System + IRQ + softirq CPU usage percentage
    pub idle: f64,      // Idle CPU usage percentage
    pub iowait: f64,    // I/O wait CPU usage percentage
}
```

#### `MemoryInfo`

Memory information from `/proc/meminfo`.

```rust
pub struct MemoryInfo {
    pub mem_total_kb: u64,      // Total memory in KB
    pub mem_available_kb: u64,  // Available memory in KB
    pub mem_free_kb: u64,       // Free memory in KB
    pub buffers_kb: u64,        // Buffers memory in KB
    pub cached_kb: u64,         // Cached memory in KB
    pub swap_total_kb: u64,     // Total swap in KB
    pub swap_free_kb: u64,      // Free swap in KB
}
```

#### `SystemMetrics`

Comprehensive system metrics structure containing all collected data.

```rust
pub struct SystemMetrics {
    pub cpu_times: CpuTimes,                    // Current CPU times
    pub memory: MemoryInfo,                     // Memory information
    pub load_avg: LoadAvg,                      // Load averages
    pub pressure: PressureMetrics,              // PSI metrics
    pub temperature: TemperatureMetrics,        // Temperature metrics
    pub power: PowerMetrics,                    // Power consumption metrics
    pub network: NetworkMetrics,                // Network metrics
    pub disk: DiskMetrics,                      // Disk metrics
    pub ebpf: Option<EbpfMetrics>,              // eBPF metrics (if available)
    pub timestamp: SystemTime,                  // Collection timestamp
}
```

### Functions

#### `collect_system_metrics(paths: &ProcPaths) -> Result<SystemMetrics>`

Collects comprehensive system metrics from `/proc` and other sources.

**Parameters:**
- `paths`: `ProcPaths` structure containing paths to `/proc` files

**Returns:**
- `Result<SystemMetrics>`: Collected system metrics or error

**Error Handling:**
- Returns error if critical files (`/proc/stat`, `/proc/meminfo`, `/proc/loadavg`) cannot be read
- Gracefully handles PSI file errors (returns empty metrics if PSI not supported)
- Gracefully handles eBPF errors (returns `None` for eBPF metrics if not available)

**Example:**
```rust
use smoothtask_core::metrics::system::{collect_system_metrics, ProcPaths};

let paths = ProcPaths::default();
let metrics = collect_system_metrics(&paths)?;

println!("CPU usage: {:.2}%", metrics.cpu_usage_since(&prev_metrics).map_or(0.0, |u| u.user * 100.0));
```

#### `collect_system_metrics_parallel(paths: &ProcPaths) -> Result<SystemMetrics>`

Collects system metrics using parallel processing for improved performance.

**Parameters:**
- `paths`: `ProcPaths` structure containing paths to `/proc` files

**Returns:**
- `Result<SystemMetrics>`: Collected system metrics or error

**Example:**
```rust
use smoothtask_core::metrics::system::{collect_system_metrics_parallel, ProcPaths};

let paths = ProcPaths::default();
let metrics = collect_system_metrics_parallel(&paths)?;
```

#### `collect_system_metrics_cached(paths: &ProcPaths, cache: &mut SystemMetricsCache) -> Result<SystemMetrics>`

Collects system metrics with caching to reduce I/O operations.

**Parameters:**
- `paths`: `ProcPaths` structure containing paths to `/proc` files
- `cache`: Mutable reference to cache structure

**Returns:**
- `Result<SystemMetrics>`: Collected system metrics or error

**Example:**
```rust
use smoothtask_core::metrics::system::{collect_system_metrics_cached, ProcPaths, SystemMetricsCache};

let paths = ProcPaths::default();
let mut cache = SystemMetricsCache::new();
let metrics = collect_system_metrics_cached(&paths, &mut cache)?;
```

#### `CpuTimes::delta(&self, prev: &CpuTimes) -> Option<CpuUsage>`

Calculates CPU usage percentages from two `CpuTimes` snapshots.

**Parameters:**
- `prev`: Previous CPU times snapshot

**Returns:**
- `Option<CpuUsage>`: CPU usage percentages or `None` if calculation fails

**Example:**
```rust
let prev = CpuTimes { user: 100, nice: 20, system: 50, idle: 200, iowait: 10, irq: 5, softirq: 5, steal: 0, guest: 0, guest_nice: 0 };
let cur = CpuTimes { user: 150, nice: 30, system: 80, idle: 260, iowait: 20, irq: 10, softirq: 10, steal: 0, guest: 0, guest_nice: 0 };

let usage = cur.delta(&prev).expect("Should be Some");
assert!(usage.user > 0.0);
```

#### `SystemMetrics::cpu_usage_since(&self, prev: &SystemMetrics) -> Option<CpuUsage>`

Calculates CPU usage since previous system metrics snapshot.

**Parameters:**
- `prev`: Previous system metrics snapshot

**Returns:**
- `Option<CpuUsage>`: CPU usage percentages or `None` if calculation fails

**Example:**
```rust
let usage = current_metrics.cpu_usage_since(&previous_metrics);
if let Some(usage) = usage {
    println!("CPU usage: user={:.2}%, system={:.2}%", usage.user * 100.0, usage.system * 100.0);
}
```

#### `SystemMetrics::optimize_memory_usage(mut self) -> Self`

Optimizes memory usage of the `SystemMetrics` structure.

**Returns:**
- Optimized `SystemMetrics` with reduced memory footprint

**Example:**
```rust
let optimized_metrics = metrics.optimize_memory_usage();
```

### Usage Examples

#### Basic Metrics Collection

```rust
use smoothtask_core::metrics::system::{collect_system_metrics, ProcPaths};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = ProcPaths::default();
    let metrics = collect_system_metrics(&paths)?;
    
    println!("System Metrics:");
    println!("  CPU: {:?}", metrics.cpu_times);
    println!("  Memory: {:?}", metrics.memory);
    println!("  Load Avg: {:?}", metrics.load_avg);
    
    Ok(())
}
```

#### Monitoring Loop with Delta Calculation

```rust
use smoothtask_core::metrics::system::{collect_system_metrics, ProcPaths};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = ProcPaths::default();
    let mut prev_metrics = collect_system_metrics(&paths)?;
    
    loop {
        let current_metrics = collect_system_metrics(&paths)?;
        
        if let Some(cpu_usage) = current_metrics.cpu_usage_since(&prev_metrics) {
            println!("CPU Usage - User: {:.2}%, System: {:.2}%", 
                     cpu_usage.user * 100.0, cpu_usage.system * 100.0);
        }
        
        prev_metrics = current_metrics;
        thread::sleep(Duration::from_secs(1));
    }
}
```

#### Parallel Metrics Collection

```rust
use smoothtask_core::metrics::system::{collect_system_metrics_parallel, ProcPaths};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = ProcPaths::default();
    let metrics = collect_system_metrics_parallel(&paths)?;
    
    println!("Parallel collection completed successfully");
    println!("CPU cores: {}", metrics.cpu_times.user);
    
    Ok(())
}
```

#### Cached Metrics Collection

```rust
use smoothtask_core::metrics::system::{collect_system_metrics_cached, ProcPaths, SystemMetricsCache};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = ProcPaths::default();
    let mut cache = SystemMetricsCache::new();
    
    // First collection (populates cache)
    let metrics1 = collect_system_metrics_cached(&paths, &mut cache)?;
    
    // Second collection (uses cache where possible)
    let metrics2 = collect_system_metrics_cached(&paths, &mut cache)?;
    
    println!("Cached collection reduces I/O operations");
    
    Ok(())
}
```

## Process Metrics API

The process metrics module provides detailed monitoring of individual processes.

### Key Structures

#### `ProcessRecord`

Comprehensive process information including CPU, memory, I/O usage, and classification data.

```rust
pub struct ProcessRecord {
    pub pid: i32,                        // Process ID
    pub ppid: i32,                       // Parent Process ID
    pub uid: u32,                        // User ID
    pub gid: u32,                        // Group ID
    pub exe: Option<String>,             // Executable name
    pub cmdline: Option<String>,         // Command line
    pub cgroup_path: Option<String>,     // Cgroup path
    pub cpu_share_10s: Option<f64>,      // CPU usage over 10 seconds
    pub mem_rss_kb: Option<u64>,         // Resident memory in KB
    pub io_read_bytes: Option<u64>,      // I/O read bytes
    pub io_write_bytes: Option<u64>,     // I/O write bytes
    pub process_type: Option<String>,    // Process type (from classification)
    pub tags: Vec<String>,               // Process tags
    pub priority_class: PriorityClass,   // Priority class
}
```

### Key Functions

#### `collect_process_metrics() -> Result<Vec<ProcessRecord>>`

Collects metrics for all processes in the system.

**Returns:**
- `Result<Vec<ProcessRecord>>`: Vector of process records or error

**Example:**
```rust
use smoothtask_core::metrics::process::collect_process_metrics;

let processes = collect_process_metrics()?;
for process in processes {
    println!("PID {}: {} - CPU: {:.2}%", 
             process.pid, 
             process.exe.unwrap_or("unknown".to_string()),
             process.cpu_share_10s.unwrap_or(0.0) * 100.0);
}
```

## GPU Metrics API

The GPU metrics module provides monitoring of GPU devices and their utilization.

### Key Structures

#### `GpuDevice`

GPU device information and metrics.

```rust
pub struct GpuDevice {
    pub id: String,                     // GPU ID
    pub name: String,                   // GPU name
    pub vendor: String,                 // Vendor
    pub utilization: f32,               // GPU utilization percentage
    pub memory_used_mb: u64,            // Memory used in MB
    pub memory_total_mb: u64,           // Total memory in MB
    pub temperature_c: Option<f32>,     // Temperature in Celsius
    pub power_w: Option<f32>,           // Power consumption in Watts
}
```

### Key Functions

#### `collect_gpu_metrics() -> Result<Vec<GpuDevice>>`

Collects metrics for all GPU devices in the system.

**Returns:**
- `Result<Vec<GpuDevice>>`: Vector of GPU devices or error

**Example:**
```rust
use smoothtask_core::metrics::gpu::collect_gpu_metrics;

let gpus = collect_gpu_metrics()?;
for gpu in gpus {
    println!("GPU {}: {}% utilization, {}°C", 
             gpu.name, gpu.utilization, 
             gpu.temperature_c.unwrap_or(0.0));
}
```

## eBPF Metrics API

The eBPF metrics module provides high-performance system monitoring using eBPF technology.

### Key Structures

#### `EbpfMetrics`

Comprehensive eBPF-collected metrics.

```rust
pub struct EbpfMetrics {
    pub cpu_usage: f64,                 // CPU usage percentage
    pub process_stats: Vec<ProcessStat>, // Per-process statistics
    pub network_stats: NetworkStats,     // Network statistics
    pub disk_stats: DiskStats,           // Disk statistics
    pub system_stats: SystemStats,       // System-wide statistics
}
```

### Key Functions

#### `initialize_ebpf_monitor() -> Result<EbpfMonitor>`

Initializes the eBPF monitoring system.

**Returns:**
- `Result<EbpfMonitor>`: Initialized eBPF monitor or error

**Example:**
```rust
use smoothtask_core::metrics::ebpf::initialize_ebpf_monitor;

let mut monitor = initialize_ebpf_monitor()?;
let metrics = monitor.collect_metrics()?;

println!("eBPF CPU usage: {:.2}%", metrics.cpu_usage);
```

## Integration Patterns

### Main Daemon Integration

```rust
use smoothtask_core::metrics::{system, process, gpu};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let system_paths = system::ProcPaths::default();
    let mut system_cache = system::SystemMetricsCache::new();
    
    loop {
        // Collect system metrics
        let system_metrics = system::collect_system_metrics_cached(&system_paths, &mut system_cache)?;
        
        // Collect process metrics
        let processes = process::collect_process_metrics()?;
        
        // Collect GPU metrics
        let gpus = gpu::collect_gpu_metrics()?;
        
        // Process and analyze metrics
        analyze_metrics(&system_metrics, &processes, &gpus);
        
        // Sleep for monitoring interval
        std::thread::sleep(Duration::from_secs(1));
    }
}
```

### Error Handling and Graceful Degradation

```rust
use smoothtask_core::metrics::system::{collect_system_metrics, ProcPaths};

fn collect_metrics_with_fallback() -> SystemMetrics {
    let paths = ProcPaths::default();
    
    match collect_system_metrics(&paths) {
        Ok(metrics) => metrics,
        Err(e) => {
            eprintln!("Error collecting metrics: {}, using fallback", e);
            SystemMetrics::default() // Return default metrics as fallback
        }
    }
}
```

### Multi-threaded Monitoring

```rust
use smoothtask_core::metrics::system::{collect_system_metrics_parallel, ProcPaths};
use std::sync::{Arc, Mutex};
use std::thread;

fn start_monitoring_thread(metrics: Arc<Mutex<SystemMetrics>>) {
    let paths = ProcPaths::default();
    
    thread::spawn(move || {
        loop {
            match collect_system_metrics_parallel(&paths) {
                Ok(new_metrics) => {
                    let mut metrics_guard = metrics.lock().unwrap();
                    *metrics_guard = new_metrics;
                }
                Err(e) => eprintln!("Monitoring error: {}", e),
            }
            thread::sleep(std::time::Duration::from_secs(1));
        }
    });
}
```

## Best Practices

1. **Use Caching**: For frequent metrics collection, use cached versions to reduce I/O operations.
2. **Handle Errors Gracefully**: Always handle potential errors and provide fallback behavior.
3. **Use Parallel Collection**: For performance-critical applications, use parallel collection methods.
4. **Optimize Memory**: Use `optimize_memory_usage()` for long-term storage of metrics.
5. **Monitor Delta Changes**: Calculate deltas between snapshots for accurate usage measurements.

## Troubleshooting

### Common Issues

1. **Permission Errors**: Ensure the application has read access to `/proc` files.
2. **Missing PSI Support**: PSI metrics require Linux kernel 4.20+ with PSI enabled.
3. **eBPF Not Available**: eBPF requires Linux kernel 5.4+ and appropriate capabilities.
4. **GPU Monitoring Issues**: Ensure proper GPU drivers and permissions are installed.

### Debugging Tips

1. Check kernel version: `uname -r`
2. Verify `/proc` access: `ls -la /proc/stat /proc/meminfo`
3. Check PSI availability: `ls -la /proc/pressure/`
4. Test eBPF support: `ls -la /sys/fs/bpf/`
5. Check GPU drivers: `nvidia-smi` or `amdgpu_info`

## Performance Considerations

1. **I/O Operations**: Reading `/proc` files can be expensive - use caching for frequent collections.
2. **Memory Usage**: Large metrics structures can consume significant memory - use optimization methods.
3. **CPU Usage**: Complex calculations should be done in background threads.
4. **Parallelism**: Use parallel collection methods for multi-core systems.
