//! Container detection and environment utilities.
//!
//! This module provides functionality for detecting containerized environments
//! (Docker, Podman, etc.) and adapting SmoothTask behavior accordingly.

use std::fs;
use std::path::Path;
use std::env;
use tracing::{debug, info};

/// Container runtime types that SmoothTask can detect
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerRuntime {
    Docker,
    Podman,
    Containerd,
    Lxc,
    Unknown(String),
    None,
}

/// Container environment information
#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub runtime: ContainerRuntime,
    pub is_containerized: bool,
    pub container_id: Option<String>,
    pub cgroup_path: Option<String>,
}

impl ContainerInfo {
    /// Create a new ContainerInfo instance
    pub fn new(runtime: ContainerRuntime, container_id: Option<String>, cgroup_path: Option<String>) -> Self {
        Self {
            runtime: runtime.clone(),
            is_containerized: runtime != ContainerRuntime::None,
            container_id,
            cgroup_path,
        }
    }
}

/// Detect container runtime by checking various indicators
pub fn detect_container_runtime() -> ContainerRuntime {
    // Check environment variables first (most reliable for Docker/Podman)
    if let Ok(container_type) = env::var("CONTAINER_TYPE") {
        match container_type.as_str() {
            "docker" => return ContainerRuntime::Docker,
            "podman" => return ContainerRuntime::Podman,
            _ => debug!("Unknown CONTAINER_TYPE: {}", container_type),
        }
    }

    // Check for Docker-specific environment variables
    if env::var("DOCKER_CONTAINER").is_ok() {
        return ContainerRuntime::Docker;
    }

    // Check for Podman-specific environment variables
    if env::var("PODMAN_CONTAINER").is_ok() {
        return ContainerRuntime::Podman;
    }

    // Check cgroup information (more reliable for detection)
    if let Ok(cgroup_content) = fs::read_to_string("/proc/1/cgroup") {
        for line in cgroup_content.lines() {
            if line.contains("docker") || line.contains("/docker-") {
                return ContainerRuntime::Docker;
            }
            if line.contains("podman") || line.contains("/podman-") {
                return ContainerRuntime::Podman;
            }
            if line.contains("containerd") || line.contains("/containerd-") {
                return ContainerRuntime::Containerd;
            }
            if line.contains("lxc") || line.contains("/lxc-") {
                return ContainerRuntime::Lxc;
            }
        }
    }

    // Check for .dockerenv file (Docker specific)
    if Path::new("/.dockerenv").exists() {
        return ContainerRuntime::Docker;
    }

    // Check for .containerenv file (Podman specific)
    if Path::new("/.containerenv").exists() {
        return ContainerRuntime::Podman;
    }

    ContainerRuntime::None
}

/// Get detailed container information including ID and cgroup path
pub fn get_container_info() -> ContainerInfo {
    let runtime = detect_container_runtime();
    
    if runtime == ContainerRuntime::None {
        return ContainerInfo::new(runtime, None, None);
    }

    // Try to extract container ID from environment variables
    let container_id = 
        env::var("HOSTNAME").ok()
        .or_else(|| env::var("CONTAINER_ID").ok())
        .or_else(|| env::var("NAME").ok());

    // Try to extract cgroup path
    let cgroup_path = fs::read_to_string("/proc/self/cgroup")
        .ok()
        .and_then(|content| {
            content.lines()
                .find(|line| line.contains("0::"))
                .map(|line| line.split("0::").nth(1).unwrap_or("").to_string())
        });

    ContainerInfo::new(runtime, container_id, cgroup_path)
}

/// Check if we're running in a containerized environment
pub fn is_containerized() -> bool {
    detect_container_runtime() != ContainerRuntime::None
}

/// Container metrics structure
#[derive(Debug, Clone)]
pub struct ContainerMetrics {
    pub runtime: ContainerRuntime,
    pub container_id: Option<String>,
    pub memory_limit_bytes: Option<u64>,
    pub memory_usage_bytes: Option<u64>,
    pub cpu_shares: Option<u64>,
    pub cpu_quota: Option<i64>,
    pub cpu_period: Option<u64>,
    pub network_interfaces: Vec<String>,
}

impl Default for ContainerMetrics {
    fn default() -> Self {
        Self {
            runtime: ContainerRuntime::None,
            container_id: None,
            memory_limit_bytes: None,
            memory_usage_bytes: None,
            cpu_shares: None,
            cpu_quota: None,
            cpu_period: None,
            network_interfaces: Vec::new(),
        }
    }
}

/// Collect container-specific metrics
pub fn collect_container_metrics() -> ContainerMetrics {
    if !is_containerized() {
        return ContainerMetrics::default();
    }

    let container_info = get_container_info();
    let mut metrics = ContainerMetrics {
        runtime: container_info.runtime.clone(),
        container_id: container_info.container_id.clone(),
        ..Default::default()
    };

    // Read memory limits from cgroup
    if let Some(cgroup_path) = &container_info.cgroup_path {
        let memory_limit_path = format!("/sys/fs/cgroup/memory{}/memory.limit_in_bytes", cgroup_path);
        let memory_usage_path = format!("/sys/fs/cgroup/memory{}/memory.usage_in_bytes", cgroup_path);
        
        if let Ok(limit_content) = fs::read_to_string(&memory_limit_path) {
            if let Ok(limit) = limit_content.trim().parse::<u64>() {
                metrics.memory_limit_bytes = Some(limit);
            }
        }

        if let Ok(usage_content) = fs::read_to_string(&memory_usage_path) {
            if let Ok(usage) = usage_content.trim().parse::<u64>() {
                metrics.memory_usage_bytes = Some(usage);
            }
        }
    }

    // Read CPU constraints from cgroup
    if let Some(cgroup_path) = &container_info.cgroup_path {
        let cpu_shares_path = format!("/sys/fs/cgroup/cpu{}/cpu.shares", cgroup_path);
        let cpu_quota_path = format!("/sys/fs/cgroup/cpu{}/cpu.cfs_quota_us", cgroup_path);
        let cpu_period_path = format!("/sys/fs/cgroup/cpu{}/cpu.cfs_period_us", cgroup_path);
        
        if let Ok(shares_content) = fs::read_to_string(&cpu_shares_path) {
            if let Ok(shares) = shares_content.trim().parse::<u64>() {
                metrics.cpu_shares = Some(shares);
            }
        }

        if let Ok(quota_content) = fs::read_to_string(&cpu_quota_path) {
            if let Ok(quota) = quota_content.trim().parse::<i64>() {
                metrics.cpu_quota = Some(quota);
            }
        }

        if let Ok(period_content) = fs::read_to_string(&cpu_period_path) {
            if let Ok(period) = period_content.trim().parse::<u64>() {
                metrics.cpu_period = Some(period);
            }
        }
    }

    // Detect network interfaces (container-specific ones)
    if let Ok(interfaces) = fs::read_dir("/sys/class/net/") {
        for interface in interfaces.filter_map(Result::ok) {
            let interface_name = interface.file_name();
            let interface_name_str = interface_name.to_string_lossy();
            
            // Skip loopback and host interfaces
            if interface_name_str != "lo" && !interface_name_str.starts_with("eth") && !interface_name_str.starts_with("en") {
                metrics.network_interfaces.push(interface_name_str.into_owned());
            }
        }
    }

    metrics
}

/// Adapt SmoothTask configuration for container environment
pub fn adapt_for_container() -> bool {
    if !is_containerized() {
        debug!("Not running in a container environment");
        return false;
    }

    let container_info = get_container_info();
    info!("Detected container environment: {:?}", container_info.runtime);
    
    if let Some(container_id) = &container_info.container_id {
        info!("Container ID: {}", container_id);
    }
    
    if let Some(cgroup_path) = &container_info.cgroup_path {
        info!("Container cgroup path: {}", cgroup_path);
    }

    // Collect and log container metrics
    let metrics = collect_container_metrics();
    if metrics.memory_limit_bytes.is_some() {
        info!("Container memory limit: {} bytes", metrics.memory_limit_bytes.unwrap());
    }
    if metrics.cpu_shares.is_some() {
        info!("Container CPU shares: {}", metrics.cpu_shares.unwrap());
    }

    // Container-specific adaptations would go here
    // For now, just return true to indicate we detected a container
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_container_detection_no_container() {
        // In a normal environment, should detect no container
        let runtime = detect_container_runtime();
        assert_eq!(runtime, ContainerRuntime::None);
        assert!(!is_containerized());
    }

    #[test]
    fn test_container_info_no_container() {
        let info = get_container_info();
        assert_eq!(info.runtime, ContainerRuntime::None);
        assert!(!info.is_containerized);
        assert!(info.container_id.is_none());
        assert!(info.cgroup_path.is_none());
    }

    #[test]
    fn test_container_runtime_enum() {
        // Test that our enum variants work correctly
        assert_eq!(ContainerRuntime::Docker, ContainerRuntime::Docker);
        assert_eq!(ContainerRuntime::Podman, ContainerRuntime::Podman);
        assert_eq!(ContainerRuntime::None, ContainerRuntime::None);
        assert_ne!(ContainerRuntime::Docker, ContainerRuntime::Podman);
    }

    #[test]
    fn test_container_info_creation() {
        let info = ContainerInfo::new(ContainerRuntime::Docker, Some("test123".to_string()), Some("/docker/test".to_string()));
        assert_eq!(info.runtime, ContainerRuntime::Docker);
        assert!(info.is_containerized);
        assert_eq!(info.container_id, Some("test123".to_string()));
        assert_eq!(info.cgroup_path, Some("/docker/test".to_string()));
    }

    #[test]
    fn test_container_metrics_default() {
        let metrics = ContainerMetrics::default();
        assert_eq!(metrics.runtime, ContainerRuntime::None);
        assert!(metrics.container_id.is_none());
        assert!(metrics.memory_limit_bytes.is_none());
        assert!(metrics.memory_usage_bytes.is_none());
        assert!(metrics.cpu_shares.is_none());
        assert!(metrics.cpu_quota.is_none());
        assert!(metrics.cpu_period.is_none());
        assert!(metrics.network_interfaces.is_empty());
    }

    #[test]
    fn test_container_metrics_no_container() {
        // In a non-container environment, should return default metrics
        let metrics = collect_container_metrics();
        assert_eq!(metrics.runtime, ContainerRuntime::None);
        // Other fields may have values if cgroups exist, but runtime should be None
    }

    #[test]
    fn test_container_adaptation_no_container() {
        // Should return false when not in container
        let result = adapt_for_container();
        assert!(!result);
    }
}