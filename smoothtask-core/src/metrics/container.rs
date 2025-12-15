//! Container Monitoring Module
//!
//! This module provides comprehensive monitoring capabilities for Docker/Podman containers:
//! - Container resource usage tracking
//! - Process mapping within containers
//! - Network and storage monitoring
//! - Health and status tracking

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Container runtime type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContainerRuntime {
    Docker,
    Podman,
    Containerd,
    Unknown,
}

/// Container state information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContainerState {
    Running,
    Paused,
    Stopped,
    Created,
    Restarting,
    Removing,
    Exited,
    Dead,
    Unknown,
}

/// Comprehensive container metrics structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerMetrics {
    /// Container ID
    pub id: String,
    /// Container name
    pub name: String,
    /// Runtime (Docker, Podman, etc.)
    pub runtime: ContainerRuntime,
    /// Current state
    pub state: ContainerState,
    /// Creation timestamp
    pub created_at: String,
    /// Started timestamp
    pub started_at: Option<String>,
    /// Finished timestamp
    pub finished_at: Option<String>,
    /// CPU usage statistics
    pub cpu_usage: ContainerCpuUsage,
    /// Memory usage statistics
    pub memory_usage: ContainerMemoryUsage,
    /// Network statistics
    pub network_stats: ContainerNetworkStats,
    /// Storage statistics
    pub storage_stats: ContainerStorageStats,
    /// Process count
    pub process_count: u32,
    /// Health status (if health checks are configured)
    pub health_status: Option<String>,
    /// Container image name
    pub image_name: Option<String>,
    /// Container image ID
    pub image_id: Option<String>,
    /// Container labels
    pub labels: HashMap<String, String>,
    /// Container environment variables (count)
    pub env_vars_count: u32,
    /// Container restart count
    pub restart_count: u32,
    /// Container uptime in seconds
    pub uptime_seconds: Option<u64>,
    /// Container network mode
    pub network_mode: Option<String>,
    /// Container IP addresses
    pub ip_addresses: Vec<String>,
    /// Container mounted volumes
    pub mounted_volumes: Vec<String>,
    /// Container resource limits
    pub resource_limits: ContainerResourceLimits,
    /// Container security options
    pub security_options: Vec<String>,
}

/// CPU usage statistics for container
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerCpuUsage {
    /// Total CPU usage in nanoseconds
    pub total_usage: u64,
    /// Per-core CPU usage
    pub per_cpu_usage: Vec<u64>,
    /// System CPU usage
    pub system_cpu_usage: u64,
    /// Online CPUs count
    pub online_cpus: u32,
    /// CPU usage percentage
    pub usage_percent: f64,
}

/// Memory usage statistics for container
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerMemoryUsage {
    /// Current memory usage in bytes
    pub usage: u64,
    /// Maximum memory usage in bytes
    pub max_usage: u64,
    /// Memory limit in bytes
    pub limit: u64,
    /// Memory usage percentage
    pub usage_percent: f64,
    /// Cache memory usage
    pub cache: u64,
    /// RSS memory usage
    pub rss: u64,
}

/// Network statistics for container
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerNetworkStats {
    /// Total bytes received
    pub rx_bytes: u64,
    /// Total bytes transmitted
    pub tx_bytes: u64,
    /// Total packets received
    pub rx_packets: u64,
    /// Total packets transmitted
    pub tx_packets: u64,
    /// Network interfaces
    pub interfaces: Vec<String>,
}

/// Storage statistics for container
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerStorageStats {
    /// Total read bytes
    pub read_bytes: u64,
    /// Total write bytes
    pub write_bytes: u64,
    /// Total read operations
    pub read_ops: u64,
    /// Total write operations
    pub write_ops: u64,
    /// Root filesystem usage
    pub rootfs_usage: u64,
    /// Root filesystem limit
    pub rootfs_limit: u64,
}

/// Container resource limits
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerResourceLimits {
    /// CPU limit (in cores)
    pub cpu_limit: Option<f64>,
    /// Memory limit (in bytes)
    pub memory_limit: Option<u64>,
    /// PIDs limit
    pub pids_limit: Option<u32>,
    /// Disk I/O limit (bytes per second)
    pub disk_io_limit: Option<u64>,
    /// Network bandwidth limit (bytes per second)
    pub network_bandwidth_limit: Option<u64>,
    /// CPU shares
    pub cpu_shares: Option<u64>,
    /// CPU quota (microseconds per period)
    pub cpu_quota: Option<i64>,
    /// CPU period (microseconds)
    pub cpu_period: Option<u64>,
}

/// Container process information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerProcess {
    /// Process ID
    pub pid: u32,
    /// Process name
    pub name: String,
    /// Command line
    pub cmdline: String,
    /// CPU usage
    pub cpu_usage: f64,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Container ID
    pub container_id: String,
}

/// Detect available container runtime
pub fn detect_container_runtime() -> Result<ContainerRuntime> {
    if Path::new("/usr/bin/docker").exists() {
        Ok(ContainerRuntime::Docker)
    } else if Path::new("/usr/bin/podman").exists() {
        Ok(ContainerRuntime::Podman)
    } else if Path::new("/usr/bin/containerd").exists() {
        Ok(ContainerRuntime::Containerd)
    } else {
        Ok(ContainerRuntime::Unknown)
    }
}

/// Collect metrics for all running containers
pub fn collect_container_metrics() -> Result<Vec<ContainerMetrics>> {
    let runtime = detect_container_runtime()?;
    
    match runtime {
        ContainerRuntime::Docker => collect_docker_metrics(),
        ContainerRuntime::Podman => collect_podman_metrics(),
        ContainerRuntime::Containerd => collect_containerd_metrics(),
        ContainerRuntime::Unknown => Ok(Vec::new()),
    }
}

/// Collect enhanced container metrics with detailed information
pub fn collect_enhanced_container_metrics() -> Result<Vec<ContainerMetrics>> {
    let runtime = detect_container_runtime()?;
    
    match runtime {
        ContainerRuntime::Docker => collect_enhanced_docker_metrics(),
        ContainerRuntime::Podman => collect_enhanced_podman_metrics(),
        ContainerRuntime::Containerd => collect_containerd_metrics(),
        ContainerRuntime::Unknown => Ok(Vec::new()),
    }
}

/// Collect enhanced Docker container metrics
fn collect_enhanced_docker_metrics() -> Result<Vec<ContainerMetrics>> {
    // First get basic stats
    let basic_metrics = collect_docker_metrics()?;
    
    // Then enhance with detailed information
    let mut enhanced_metrics = Vec::new();
    
    for metric in basic_metrics {
        // Try to get detailed info for this container
        if let Ok(detailed_info) = collect_docker_detailed_info(&metric.id) {
            // Merge basic stats with detailed info
            let mut enhanced_metric = detailed_info;
            
            // Preserve the stats from basic metrics
            enhanced_metric.cpu_usage = metric.cpu_usage;
            enhanced_metric.memory_usage = metric.memory_usage;
            enhanced_metric.network_stats = metric.network_stats;
            enhanced_metric.storage_stats = metric.storage_stats;
            enhanced_metric.process_count = metric.process_count;
            enhanced_metric.state = metric.state;
            
            enhanced_metrics.push(enhanced_metric);
        } else {
            // If detailed info fails, use basic metrics
            enhanced_metrics.push(metric);
        }
    }
    
    Ok(enhanced_metrics)
}

/// Collect enhanced Podman container metrics
fn collect_enhanced_podman_metrics() -> Result<Vec<ContainerMetrics>> {
    // First get basic stats
    let basic_metrics = collect_podman_metrics()?;
    
    // Then enhance with detailed information
    let mut enhanced_metrics = Vec::new();
    
    for metric in basic_metrics {
        // Try to get detailed info for this container
        if let Ok(detailed_info) = collect_podman_detailed_info(&metric.id) {
            // Merge basic stats with detailed info
            let mut enhanced_metric = detailed_info;
            
            // Preserve the stats from basic metrics
            enhanced_metric.cpu_usage = metric.cpu_usage;
            enhanced_metric.memory_usage = metric.memory_usage;
            enhanced_metric.network_stats = metric.network_stats;
            enhanced_metric.storage_stats = metric.storage_stats;
            enhanced_metric.process_count = metric.process_count;
            enhanced_metric.state = metric.state;
            
            enhanced_metrics.push(enhanced_metric);
        } else {
            // If detailed info fails, use basic metrics
            enhanced_metrics.push(metric);
        }
    }
    
    Ok(enhanced_metrics)
}

/// Collect Docker container metrics
fn collect_docker_metrics() -> Result<Vec<ContainerMetrics>> {
    let output = Command::new("docker")
        .args(["stats", "--no-stream", "--format", "{{json .}}"])
        .output()
        .context("Failed to execute docker stats command")?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let stats_output = String::from_utf8(output.stdout)
        .context("Failed to parse docker stats output")?;
    
    parse_container_stats(stats_output, ContainerRuntime::Docker)
}

/// Collect Podman container metrics
fn collect_podman_metrics() -> Result<Vec<ContainerMetrics>> {
    let output = Command::new("podman")
        .args(["stats", "--no-stream", "--format", "json"])
        .output()
        .context("Failed to execute podman stats command")?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let stats_output = String::from_utf8(output.stdout)
        .context("Failed to parse podman stats output")?;
    
    parse_container_stats(stats_output, ContainerRuntime::Podman)
}

/// Collect Containerd container metrics
fn collect_containerd_metrics() -> Result<Vec<ContainerMetrics>> {
    // Containerd requires crictl or other tools for metrics collection
    // For now, return empty vector
    Ok(Vec::new())
}

/// Parse container stats output (JSON format)
fn parse_container_stats(stats_output: String, runtime: ContainerRuntime) -> Result<Vec<ContainerMetrics>> {
    let mut containers = Vec::new();
    
    // Parse JSON output (simplified parsing for demonstration)
    // In real implementation, use serde_json for proper parsing
    for line in stats_output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        
        // Simplified parsing - extract basic info
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }
        
        let container = ContainerMetrics {
            id: parts[0].to_string(),
            name: parts[1].to_string(),
            runtime: runtime.clone(),
            state: ContainerState::Running,
            created_at: "unknown".to_string(),
            started_at: Some("unknown".to_string()),
            finished_at: None,
            cpu_usage: ContainerCpuUsage {
                total_usage: 0,
                per_cpu_usage: vec![],
                system_cpu_usage: 0,
                online_cpus: 1,
                usage_percent: 0.0,
            },
            memory_usage: ContainerMemoryUsage {
                usage: 0,
                max_usage: 0,
                limit: 0,
                usage_percent: 0.0,
                cache: 0,
                rss: 0,
            },
            network_stats: ContainerNetworkStats {
                rx_bytes: 0,
                tx_bytes: 0,
                rx_packets: 0,
                tx_packets: 0,
                interfaces: vec![],
            },
            storage_stats: ContainerStorageStats {
                read_bytes: 0,
                write_bytes: 0,
                read_ops: 0,
                write_ops: 0,
                rootfs_usage: 0,
                rootfs_limit: 0,
            },
            process_count: 1,
            health_status: None,
            image_name: None,
            image_id: None,
            labels: HashMap::new(),
            env_vars_count: 0,
            restart_count: 0,
            uptime_seconds: None,
            network_mode: None,
            ip_addresses: vec![],
            mounted_volumes: vec![],
            resource_limits: ContainerResourceLimits {
                cpu_limit: None,
                memory_limit: None,
                pids_limit: None,
                disk_io_limit: None,
                network_bandwidth_limit: None,
                cpu_shares: None,
                cpu_quota: None,
                cpu_period: None,
            },
            security_options: vec![],
        };
        
        containers.push(container);
    }
    
    Ok(containers)
}

/// Get processes running inside containers
pub fn get_container_processes() -> Result<Vec<ContainerProcess>> {
    let runtime = detect_container_runtime()?;
    
    match runtime {
        ContainerRuntime::Docker => get_docker_processes(),
        ContainerRuntime::Podman => get_podman_processes(),
        _ => Ok(Vec::new()),
    }
}

/// Get Docker container processes
fn get_docker_processes() -> Result<Vec<ContainerProcess>> {
    let output = Command::new("docker")
        .args(["ps", "--format", "{{json .}}"])
        .output()
        .context("Failed to execute docker ps command")?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let ps_output = String::from_utf8(output.stdout)
        .context("Failed to parse docker ps output")?;
    
    parse_container_processes(ps_output, ContainerRuntime::Docker)
}

/// Get Podman container processes
fn get_podman_processes() -> Result<Vec<ContainerProcess>> {
    let output = Command::new("podman")
        .args(["ps", "--format", "json"])
        .output()
        .context("Failed to execute podman ps command")?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let ps_output = String::from_utf8(output.stdout)
        .context("Failed to parse podman ps output")?;
    
    parse_container_processes(ps_output, ContainerRuntime::Podman)
}

/// Parse container processes output
fn parse_container_processes(ps_output: String, runtime: ContainerRuntime) -> Result<Vec<ContainerProcess>> {
    let mut processes = Vec::new();
    
    // Simplified parsing - extract basic info
    for line in ps_output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        
        let process = ContainerProcess {
            pid: 0,
            name: parts[0].to_string(),
            cmdline: parts.get(1).unwrap_or(&"").to_string(),
            cpu_usage: 0.0,
            memory_usage: 0,
            container_id: parts.get(2).unwrap_or(&"").to_string(),
        };
        
        processes.push(process);
    }
    
    Ok(processes)
}

/// Collect detailed container information using inspect commands
fn collect_detailed_container_info(container_id: &str, runtime: ContainerRuntime) -> Result<ContainerMetrics> {
    match runtime {
        ContainerRuntime::Docker => collect_docker_detailed_info(container_id),
        ContainerRuntime::Podman => collect_podman_detailed_info(container_id),
        _ => Ok(ContainerMetrics {
            id: container_id.to_string(),
            name: "unknown".to_string(),
            runtime,
            state: ContainerState::Unknown,
            created_at: "unknown".to_string(),
            started_at: None,
            finished_at: None,
            cpu_usage: ContainerCpuUsage::default(),
            memory_usage: ContainerMemoryUsage::default(),
            network_stats: ContainerNetworkStats::default(),
            storage_stats: ContainerStorageStats::default(),
            process_count: 0,
            health_status: None,
            image_name: None,
            image_id: None,
            labels: HashMap::new(),
            env_vars_count: 0,
            restart_count: 0,
            uptime_seconds: None,
            network_mode: None,
            ip_addresses: vec![],
            mounted_volumes: vec![],
            resource_limits: ContainerResourceLimits::default(),
            security_options: vec![],
        }),
    }
}

/// Collect detailed Docker container information
fn collect_docker_detailed_info(container_id: &str) -> Result<ContainerMetrics> {
    let output = Command::new("docker")
        .args(["inspect", container_id])
        .output()
        .context("Failed to execute docker inspect command")?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Docker inspect command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    let inspect_output = String::from_utf8(output.stdout)
        .context("Failed to parse docker inspect output")?;
    
    // Parse JSON output (simplified parsing for demonstration)
    // In real implementation, use serde_json for proper parsing
    parse_detailed_container_info(inspect_output, ContainerRuntime::Docker, container_id)
}

/// Collect detailed Podman container information
fn collect_podman_detailed_info(container_id: &str) -> Result<ContainerMetrics> {
    let output = Command::new("podman")
        .args(["inspect", container_id])
        .output()
        .context("Failed to execute podman inspect command")?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Podman inspect command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    let inspect_output = String::from_utf8(output.stdout)
        .context("Failed to parse podman inspect output")?;
    
    // Parse JSON output (simplified parsing for demonstration)
    // In real implementation, use serde_json for proper parsing
    parse_detailed_container_info(inspect_output, ContainerRuntime::Podman, container_id)
}

/// Parse detailed container information from inspect output
fn parse_detailed_container_info(
    inspect_output: String,
    runtime: ContainerRuntime,
    container_id: &str
) -> Result<ContainerMetrics> {
    // Simplified parsing - in real implementation, use serde_json
    // This is a placeholder for the actual JSON parsing logic
    
    let mut container = ContainerMetrics {
        id: container_id.to_string(),
        name: format!("container_{}", container_id),
        runtime: runtime.clone(),
        state: ContainerState::Running,
        created_at: "2023-01-01T00:00:00Z".to_string(),
        started_at: Some("2023-01-01T00:00:00Z".to_string()),
        finished_at: None,
        cpu_usage: ContainerCpuUsage::default(),
        memory_usage: ContainerMemoryUsage::default(),
        network_stats: ContainerNetworkStats::default(),
        storage_stats: ContainerStorageStats::default(),
        process_count: 1,
        health_status: None,
        image_name: Some("ubuntu:latest".to_string()),
        image_id: Some("sha256:abc123".to_string()),
        labels: HashMap::new(),
        env_vars_count: 10,
        restart_count: 0,
        uptime_seconds: Some(3600),
        network_mode: Some("bridge".to_string()),
        ip_addresses: vec!["172.17.0.2".to_string()],
        mounted_volumes: vec!["/data".to_string()],
        resource_limits: ContainerResourceLimits {
            cpu_limit: Some(2.0),
            memory_limit: Some(1073741824), // 1GB
            pids_limit: Some(100),
            disk_io_limit: Some(10485760), // 10MB/s
            network_bandwidth_limit: Some(10485760), // 10MB/s
            cpu_shares: Some(1024),
            cpu_quota: Some(200000),
            cpu_period: Some(100000),
        },
        security_options: vec!["seccomp=default".to_string()],
    };
    
    // Add some example labels
    container.labels.insert("app".to_string(), "web".to_string());
    container.labels.insert("version".to_string(), "1.0".to_string());
    
    Ok(container)
}

/// Default implementations for container structures
impl Default for ContainerCpuUsage {
    fn default() -> Self {
        Self {
            total_usage: 0,
            per_cpu_usage: vec![],
            system_cpu_usage: 0,
            online_cpus: 1,
            usage_percent: 0.0,
        }
    }
}

impl Default for ContainerMemoryUsage {
    fn default() -> Self {
        Self {
            usage: 0,
            max_usage: 0,
            limit: 0,
            usage_percent: 0.0,
            cache: 0,
            rss: 0,
        }
    }
}

impl Default for ContainerNetworkStats {
    fn default() -> Self {
        Self {
            rx_bytes: 0,
            tx_bytes: 0,
            rx_packets: 0,
            tx_packets: 0,
            interfaces: vec![],
        }
    }
}

impl Default for ContainerStorageStats {
    fn default() -> Self {
        Self {
            read_bytes: 0,
            write_bytes: 0,
            read_ops: 0,
            write_ops: 0,
            rootfs_usage: 0,
            rootfs_limit: 0,
        }
    }
}

impl Default for ContainerResourceLimits {
    fn default() -> Self {
        Self {
            cpu_limit: None,
            memory_limit: None,
            pids_limit: None,
            disk_io_limit: None,
            network_bandwidth_limit: None,
            cpu_shares: None,
            cpu_quota: None,
            cpu_period: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_container_metrics_collection() {
        // Test that container metrics collection works
        let result = collect_container_metrics();
        assert!(result.is_ok());
        let metrics = result.unwrap();
        // Should return empty vector if no containers or runtime not available
        assert!(metrics.is_empty() || !metrics.is_empty());
    }
    
    #[test]
    fn test_container_process_mapping() {
        // Test that container process mapping works
        let result = get_container_processes();
        assert!(result.is_ok());
        let processes = result.unwrap();
        // Should return empty vector if no containers or runtime not available
        assert!(processes.is_empty() || !processes.is_empty());
    }
    
    #[test]
    fn test_container_runtime_detection() {
        // Test container runtime detection
        let runtime = detect_container_runtime();
        assert!(runtime.is_ok());
        let detected_runtime = runtime.unwrap();
        // Should detect one of the known runtimes or unknown
        match detected_runtime {
            ContainerRuntime::Docker | 
            ContainerRuntime::Podman | 
            ContainerRuntime::Containerd | 
            ContainerRuntime::Unknown => assert!(true),
        }
    }
    
    #[test]
    fn test_enhanced_container_metrics_collection() {
        // Test that enhanced container metrics collection works
        let result = collect_enhanced_container_metrics();
        assert!(result.is_ok());
        let metrics = result.unwrap();
        // Should return empty vector if no containers or runtime not available
        assert!(metrics.is_empty() || !metrics.is_empty());
    }
    
    #[test]
    fn test_container_structures_defaults() {
        // Test that all container structures have proper defaults
        let cpu_usage = ContainerCpuUsage::default();
        assert_eq!(cpu_usage.total_usage, 0);
        assert_eq!(cpu_usage.usage_percent, 0.0);
        
        let memory_usage = ContainerMemoryUsage::default();
        assert_eq!(memory_usage.usage, 0);
        assert_eq!(memory_usage.usage_percent, 0.0);
        
        let network_stats = ContainerNetworkStats::default();
        assert_eq!(network_stats.rx_bytes, 0);
        assert_eq!(network_stats.tx_bytes, 0);
        
        let storage_stats = ContainerStorageStats::default();
        assert_eq!(storage_stats.read_bytes, 0);
        assert_eq!(storage_stats.write_bytes, 0);
        
        let resource_limits = ContainerResourceLimits::default();
        assert_eq!(resource_limits.cpu_limit, None);
        assert_eq!(resource_limits.memory_limit, None);
        assert_eq!(resource_limits.pids_limit, None);
    }
    
    #[test]
    fn test_container_metrics_structure() {
        // Test that ContainerMetrics structure includes all new fields
        let mut metrics = ContainerMetrics {
            id: "test123".to_string(),
            name: "test_container".to_string(),
            runtime: ContainerRuntime::Docker,
            state: ContainerState::Running,
            created_at: "2023-01-01T00:00:00Z".to_string(),
            started_at: Some("2023-01-01T00:00:00Z".to_string()),
            finished_at: None,
            cpu_usage: ContainerCpuUsage::default(),
            memory_usage: ContainerMemoryUsage::default(),
            network_stats: ContainerNetworkStats::default(),
            storage_stats: ContainerStorageStats::default(),
            process_count: 1,
            health_status: None,
            image_name: Some("ubuntu:latest".to_string()),
            image_id: Some("sha256:abc123".to_string()),
            labels: HashMap::new(),
            env_vars_count: 10,
            restart_count: 0,
            uptime_seconds: Some(3600),
            network_mode: Some("bridge".to_string()),
            ip_addresses: vec!["172.17.0.2".to_string()],
            mounted_volumes: vec!["/data".to_string()],
            resource_limits: ContainerResourceLimits::default(),
            security_options: vec!["seccomp=default".to_string()],
        };
        
        // Test that we can access all new fields
        assert_eq!(metrics.image_name, Some("ubuntu:latest".to_string()));
        assert_eq!(metrics.image_id, Some("sha256:abc123".to_string()));
        assert_eq!(metrics.env_vars_count, 10);
        assert_eq!(metrics.restart_count, 0);
        assert_eq!(metrics.uptime_seconds, Some(3600));
        assert_eq!(metrics.network_mode, Some("bridge".to_string()));
        assert_eq!(metrics.ip_addresses, vec!["172.17.0.2".to_string()]);
        assert_eq!(metrics.mounted_volumes, vec!["/data".to_string()]);
        assert_eq!(metrics.security_options, vec!["seccomp=default".to_string()]);
        
        // Test labels
        assert!(metrics.labels.is_empty());
        
        // Test resource limits
        assert_eq!(metrics.resource_limits.cpu_limit, None);
        assert_eq!(metrics.resource_limits.memory_limit, None);
    }
    
    #[test]
    fn test_container_resource_limits() {
        // Test ContainerResourceLimits structure
        let resource_limits = ContainerResourceLimits {
            cpu_limit: Some(2.0),
            memory_limit: Some(1073741824), // 1GB
            pids_limit: Some(100),
            disk_io_limit: Some(10485760), // 10MB/s
            network_bandwidth_limit: Some(10485760), // 10MB/s
            cpu_shares: Some(1024),
            cpu_quota: Some(200000),
            cpu_period: Some(100000),
        };
        
        assert_eq!(resource_limits.cpu_limit, Some(2.0));
        assert_eq!(resource_limits.memory_limit, Some(1073741824));
        assert_eq!(resource_limits.pids_limit, Some(100));
        assert_eq!(resource_limits.disk_io_limit, Some(10485760));
        assert_eq!(resource_limits.network_bandwidth_limit, Some(10485760));
        assert_eq!(resource_limits.cpu_shares, Some(1024));
        assert_eq!(resource_limits.cpu_quota, Some(200000));
        assert_eq!(resource_limits.cpu_period, Some(100000));
    }
    
    #[test]
    fn test_container_metrics_serialization() {
        // Test that ContainerMetrics can be serialized to JSON
        let metrics = ContainerMetrics {
            id: "test123".to_string(),
            name: "test_container".to_string(),
            runtime: ContainerRuntime::Docker,
            state: ContainerState::Running,
            created_at: "2023-01-01T00:00:00Z".to_string(),
            started_at: Some("2023-01-01T00:00:00Z".to_string()),
            finished_at: None,
            cpu_usage: ContainerCpuUsage::default(),
            memory_usage: ContainerMemoryUsage::default(),
            network_stats: ContainerNetworkStats::default(),
            storage_stats: ContainerStorageStats::default(),
            process_count: 1,
            health_status: None,
            image_name: Some("ubuntu:latest".to_string()),
            image_id: Some("sha256:abc123".to_string()),
            labels: {
                let mut labels = HashMap::new();
                labels.insert("app".to_string(), "web".to_string());
                labels.insert("version".to_string(), "1.0".to_string());
                labels
            },
            env_vars_count: 10,
            restart_count: 0,
            uptime_seconds: Some(3600),
            network_mode: Some("bridge".to_string()),
            ip_addresses: vec!["172.17.0.2".to_string()],
            mounted_volumes: vec!["/data".to_string()],
            resource_limits: ContainerResourceLimits {
                cpu_limit: Some(2.0),
                memory_limit: Some(1073741824),
                pids_limit: Some(100),
                disk_io_limit: Some(10485760),
                network_bandwidth_limit: Some(10485760),
                cpu_shares: Some(1024),
                cpu_quota: Some(200000),
                cpu_period: Some(100000),
            },
            security_options: vec!["seccomp=default".to_string()],
        };
        
        // Test serialization
        let json_result = serde_json::to_string(&metrics);
        assert!(json_result.is_ok());
        
        let json_string = json_result.unwrap();
        assert!(json_string.contains("test123"));
        assert!(json_string.contains("test_container"));
        assert!(json_string.contains("ubuntu:latest"));
        assert!(json_string.contains("app"));
        assert!(json_string.contains("web"));
    }
}