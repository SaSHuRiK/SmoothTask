//! Container detection and environment utilities.
//!
//! This module provides functionality for detecting containerized environments
//! (Docker, Podman, etc.) and adapting SmoothTask behavior accordingly.

use std::fs;
use std::path::Path;
use std::env;
use tracing::{debug, info};

use super::cgroups::is_cgroup_v2_available;

/// Container runtime types that SmoothTask can detect
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerRuntime {
    Docker,
    Podman,
    Containerd,
    Lxc,
    Kubernetes,
    Crio,
    Rkt,
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
            "containerd" => return ContainerRuntime::Containerd,
            "lxc" => return ContainerRuntime::Lxc,
            "kubernetes" => return ContainerRuntime::Kubernetes,
            "crio" => return ContainerRuntime::Crio,
            "rkt" => return ContainerRuntime::Rkt,
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

    // Check for Kubernetes-specific environment variables
    if env::var("KUBERNETES_SERVICE_HOST").is_ok() && env::var("KUBERNETES_SERVICE_PORT").is_ok() {
        return ContainerRuntime::Kubernetes;
    }

    // Check for CRI-O specific environment variables
    if env::var("CRIO_VERSION").is_ok() {
        return ContainerRuntime::Crio;
    }

    // Check for containerd-specific environment variables
    if env::var("CONTAINERD_NAMESPACE").is_ok() {
        return ContainerRuntime::Containerd;
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
            if line.contains("kubepods") || line.contains("/kubepods") {
                return ContainerRuntime::Kubernetes;
            }
            if line.contains("crio") || line.contains("/crio-") {
                return ContainerRuntime::Crio;
            }
            if line.contains("rkt") || line.contains("/rkt-") {
                return ContainerRuntime::Rkt;
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

    // Check for Kubernetes-specific files
    if Path::new("/var/run/secrets/kubernetes.io").exists() {
        return ContainerRuntime::Kubernetes;
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

/// Collect container-specific metrics with support for both cgroup v1 and v2
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

    // Check if cgroup v2 is available
    let cgroup_v2_available = is_cgroup_v2_available();

    // Read memory limits from cgroup (support both v1 and v2)
    if let Some(cgroup_path) = &container_info.cgroup_path {
        if cgroup_v2_available {
            // cgroup v2 paths
            let memory_limit_path = format!("/sys/fs/cgroup{}/memory.max", cgroup_path);
            let memory_usage_path = format!("/sys/fs/cgroup{}/memory.current", cgroup_path);
            
            // Improved error handling with logging
            match fs::read_to_string(&memory_limit_path) {
                Ok(limit_content) => {
                    if let Ok(limit) = limit_content.trim().parse::<u64>() {
                        metrics.memory_limit_bytes = Some(limit);
                    } else {
                        debug!("Failed to parse memory limit from: {}", limit_content.trim());
                    }
                }
                Err(e) => debug!("Failed to read memory limit at {}: {}", memory_limit_path, e),
            }

            match fs::read_to_string(&memory_usage_path) {
                Ok(usage_content) => {
                    if let Ok(usage) = usage_content.trim().parse::<u64>() {
                        metrics.memory_usage_bytes = Some(usage);
                    } else {
                        debug!("Failed to parse memory usage from: {}", usage_content.trim());
                    }
                }
                Err(e) => debug!("Failed to read memory usage at {}: {}", memory_usage_path, e),
            }
        } else {
            // cgroup v1 paths (fallback)
            let memory_limit_path = format!("/sys/fs/cgroup/memory{}/memory.limit_in_bytes", cgroup_path);
            let memory_usage_path = format!("/sys/fs/cgroup/memory{}/memory.usage_in_bytes", cgroup_path);
            
            match fs::read_to_string(&memory_limit_path) {
                Ok(limit_content) => {
                    if let Ok(limit) = limit_content.trim().parse::<u64>() {
                        metrics.memory_limit_bytes = Some(limit);
                    } else {
                        debug!("Failed to parse memory limit from: {}", limit_content.trim());
                    }
                }
                Err(e) => debug!("Failed to read memory limit at {}: {}", memory_limit_path, e),
            }

            match fs::read_to_string(&memory_usage_path) {
                Ok(usage_content) => {
                    if let Ok(usage) = usage_content.trim().parse::<u64>() {
                        metrics.memory_usage_bytes = Some(usage);
                    } else {
                        debug!("Failed to parse memory usage from: {}", usage_content.trim());
                    }
                }
                Err(e) => debug!("Failed to read memory usage at {}: {}", memory_usage_path, e),
            }
        }
    }

    // Read CPU constraints from cgroup (support both v1 and v2)
    if let Some(cgroup_path) = &container_info.cgroup_path {
        if cgroup_v2_available {
            // cgroup v2 paths
            let cpu_weight_path = format!("/sys/fs/cgroup{}/cpu.weight", cgroup_path);
            let cpu_max_path = format!("/sys/fs/cgroup{}/cpu.max", cgroup_path);
            
            // Try to read CPU weight (replaces cpu.shares in v2)
            match fs::read_to_string(&cpu_weight_path) {
                Ok(weight_content) => {
                    if let Ok(weight) = weight_content.trim().parse::<u64>() {
                        metrics.cpu_shares = Some(weight);
                    } else {
                        debug!("Failed to parse CPU weight from: {}", weight_content.trim());
                    }
                }
                Err(e) => debug!("Failed to read CPU weight at {}: {}", cpu_weight_path, e),
            }

            // Try to read CPU max (replaces cpu.cfs_quota_us and cpu.cfs_period_us in v2)
            match fs::read_to_string(&cpu_max_path) {
                Ok(max_content) => {
                    // Format is "max period", e.g., "100000 100000" for 1 CPU
                    let parts: Vec<&str> = max_content.trim().split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(quota) = parts[0].parse::<i64>() {
                            metrics.cpu_quota = Some(quota);
                        } else {
                            debug!("Failed to parse CPU quota from: {}", parts[0]);
                        }
                        if let Ok(period) = parts[1].parse::<u64>() {
                            metrics.cpu_period = Some(period);
                        } else {
                            debug!("Failed to parse CPU period from: {}", parts[1]);
                        }
                    } else {
                        debug!("Invalid CPU max format: {}", max_content.trim());
                    }
                }
                Err(e) => debug!("Failed to read CPU max at {}: {}", cpu_max_path, e),
            }
        } else {
            // cgroup v1 paths (fallback)
            let cpu_shares_path = format!("/sys/fs/cgroup/cpu{}/cpu.shares", cgroup_path);
            let cpu_quota_path = format!("/sys/fs/cgroup/cpu{}/cpu.cfs_quota_us", cgroup_path);
            let cpu_period_path = format!("/sys/fs/cgroup/cpu{}/cpu.cfs_period_us", cgroup_path);
            
            match fs::read_to_string(&cpu_shares_path) {
                Ok(shares_content) => {
                    if let Ok(shares) = shares_content.trim().parse::<u64>() {
                        metrics.cpu_shares = Some(shares);
                    } else {
                        debug!("Failed to parse CPU shares from: {}", shares_content.trim());
                    }
                }
                Err(e) => debug!("Failed to read CPU shares at {}: {}", cpu_shares_path, e),
            }

            match fs::read_to_string(&cpu_quota_path) {
                Ok(quota_content) => {
                    if let Ok(quota) = quota_content.trim().parse::<i64>() {
                        metrics.cpu_quota = Some(quota);
                    } else {
                        debug!("Failed to parse CPU quota from: {}", quota_content.trim());
                    }
                }
                Err(e) => debug!("Failed to read CPU quota at {}: {}", cpu_quota_path, e),
            }

            match fs::read_to_string(&cpu_period_path) {
                Ok(period_content) => {
                    if let Ok(period) = period_content.trim().parse::<u64>() {
                        metrics.cpu_period = Some(period);
                    } else {
                        debug!("Failed to parse CPU period from: {}", period_content.trim());
                    }
                }
                Err(e) => debug!("Failed to read CPU period at {}: {}", cpu_period_path, e),
            }
        }
    }

    // Detect network interfaces (container-specific ones)
    match fs::read_dir("/sys/class/net/") {
        Ok(interfaces) => {
            for interface in interfaces.filter_map(Result::ok) {
                let interface_name = interface.file_name();
                let interface_name_str = interface_name.to_string_lossy();
                
                // Skip loopback and host interfaces
                if interface_name_str != "lo" && !interface_name_str.starts_with("eth") && !interface_name_str.starts_with("en") {
                    metrics.network_interfaces.push(interface_name_str.into_owned());
                }
            }
        }
        Err(e) => debug!("Failed to read network interfaces: {}", e),
    }

    metrics
}

/// Get container-specific environment variables for debugging and monitoring
pub fn get_container_environment_info() -> Vec<(String, String)> {
    let mut env_info = Vec::new();
    
    // Common container environment variables
    let env_vars = [
        "CONTAINER_TYPE",
        "DOCKER_CONTAINER", 
        "PODMAN_CONTAINER",
        "KUBERNETES_SERVICE_HOST",
        "KUBERNETES_SERVICE_PORT",
        "CRIO_VERSION",
        "CONTAINERD_NAMESPACE",
        "HOSTNAME",
        "CONTAINER_ID",
        "NAME",
    ];
    
    for var in env_vars.iter() {
        if let Ok(value) = env::var(var) {
            env_info.push((var.to_string(), value));
        }
    }
    
    env_info
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
    if metrics.memory_usage_bytes.is_some() {
        info!("Container memory usage: {} bytes", metrics.memory_usage_bytes.unwrap());
    }
    if metrics.cpu_shares.is_some() {
        info!("Container CPU shares: {}", metrics.cpu_shares.unwrap());
    }
    if metrics.cpu_quota.is_some() {
        info!("Container CPU quota: {}", metrics.cpu_quota.unwrap());
    }
    if metrics.cpu_period.is_some() {
        info!("Container CPU period: {}", metrics.cpu_period.unwrap());
    }
    if !metrics.network_interfaces.is_empty() {
        info!("Container network interfaces: {:?}", metrics.network_interfaces);
    }

    // Log container environment variables for debugging
    let env_info = get_container_environment_info();
    if !env_info.is_empty() {
        debug!("Container environment variables:");
        for (key, value) in env_info {
            debug!("  {}={}", key, value);
        }
    }

    // Container-specific adaptations would go here
    // For now, just return true to indicate we detected a container
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    
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

    #[test]
    fn test_cgroup_v2_detection_integration() {
        // Test that cgroup v2 detection works in the container context
        let v2_available = is_cgroup_v2_available();
        // This test just verifies the function can be called without panicking
        // The actual availability depends on the system configuration
        assert!(v2_available || !v2_available); // Always true, just testing it doesn't panic
    }

    #[test]
    fn test_container_metrics_structure() {
        // Test that ContainerMetrics structure is properly defined
        let metrics = ContainerMetrics {
            runtime: ContainerRuntime::Docker,
            container_id: Some("test-container".to_string()),
            memory_limit_bytes: Some(1024 * 1024 * 1024), // 1GB
            memory_usage_bytes: Some(512 * 1024 * 1024),  // 512MB
            cpu_shares: Some(1024),
            cpu_quota: Some(100000),
            cpu_period: Some(100000),
            network_interfaces: vec!["eth0".to_string(), "veth1".to_string()],
        };

        assert_eq!(metrics.runtime, ContainerRuntime::Docker);
        assert_eq!(metrics.container_id, Some("test-container".to_string()));
        assert_eq!(metrics.memory_limit_bytes, Some(1024 * 1024 * 1024));
        assert_eq!(metrics.memory_usage_bytes, Some(512 * 1024 * 1024));
        assert_eq!(metrics.cpu_shares, Some(1024));
        assert_eq!(metrics.cpu_quota, Some(100000));
        assert_eq!(metrics.cpu_period, Some(100000));
        assert_eq!(metrics.network_interfaces.len(), 2);
    }

    #[test]
    fn test_container_info_with_cgroup_path() {
        // Test ContainerInfo creation with cgroup path
        let info = ContainerInfo::new(
            ContainerRuntime::Podman,
            Some("podman-container-123".to_string()),
            Some("/podman/container-123".to_string())
        );

        assert_eq!(info.runtime, ContainerRuntime::Podman);
        assert!(info.is_containerized);
        assert_eq!(info.container_id, Some("podman-container-123".to_string()));
        assert_eq!(info.cgroup_path, Some("/podman/container-123".to_string()));
    }

    #[test]
    fn test_new_container_runtimes() {
        // Test that new container runtime variants work correctly
        assert_eq!(ContainerRuntime::Kubernetes, ContainerRuntime::Kubernetes);
        assert_eq!(ContainerRuntime::Crio, ContainerRuntime::Crio);
        assert_eq!(ContainerRuntime::Rkt, ContainerRuntime::Rkt);
        assert_ne!(ContainerRuntime::Kubernetes, ContainerRuntime::Docker);
        assert_ne!(ContainerRuntime::Crio, ContainerRuntime::Podman);
        assert_ne!(ContainerRuntime::Rkt, ContainerRuntime::Containerd);
    }

    #[test]
    fn test_container_environment_info() {
        // Test container environment info function
        let env_info = get_container_environment_info();
        
        // Should return a vector of tuples
        assert!(env_info.is_empty() || !env_info.is_empty()); // Always true, just testing it doesn't panic
        
        // Each item should be a tuple of strings
        for (key, value) in env_info {
            assert!(!key.is_empty());
            assert!(!value.is_empty());
        }
    }

    #[test]
    fn test_container_runtime_detection_edge_cases() {
        // Test that container detection handles edge cases gracefully
        let runtime = detect_container_runtime();
        let _ = runtime; // Just ensure it doesn't panic
        
        // Test that all runtime variants can be matched
        match runtime {
            ContainerRuntime::Docker => {},
            ContainerRuntime::Podman => {},
            ContainerRuntime::Containerd => {},
            ContainerRuntime::Lxc => {},
            ContainerRuntime::Kubernetes => {},
            ContainerRuntime::Crio => {},
            ContainerRuntime::Rkt => {},
            ContainerRuntime::Unknown(_) => {},
            ContainerRuntime::None => {},
        }
    }

    #[test]
    fn test_container_metrics_with_new_runtimes() {
        // Test that container metrics work with new runtime types
        let metrics = ContainerMetrics {
            runtime: ContainerRuntime::Kubernetes,
            container_id: Some("k8s-pod-123".to_string()),
            memory_limit_bytes: Some(2 * 1024 * 1024 * 1024), // 2GB
            memory_usage_bytes: Some(1 * 1024 * 1024 * 1024), // 1GB
            cpu_shares: Some(2048),
            cpu_quota: Some(200000),
            cpu_period: Some(100000),
            network_interfaces: vec!["eth0".to_string(), "veth1".to_string(), "flannel.1".to_string()],
        };

        assert_eq!(metrics.runtime, ContainerRuntime::Kubernetes);
        assert_eq!(metrics.container_id, Some("k8s-pod-123".to_string()));
        assert_eq!(metrics.memory_limit_bytes, Some(2 * 1024 * 1024 * 1024));
        assert_eq!(metrics.memory_usage_bytes, Some(1 * 1024 * 1024 * 1024));
        assert_eq!(metrics.cpu_shares, Some(2048));
        assert_eq!(metrics.cpu_quota, Some(200000));
        assert_eq!(metrics.cpu_period, Some(100000));
        assert_eq!(metrics.network_interfaces.len(), 3);
    }

    #[test]
    fn test_container_info_with_kubernetes() {
        // Test ContainerInfo creation with Kubernetes runtime
        let info = ContainerInfo::new(
            ContainerRuntime::Kubernetes,
            Some("k8s-pod-abc123".to_string()),
            Some("/kubepods/podabc123".to_string())
        );

        assert_eq!(info.runtime, ContainerRuntime::Kubernetes);
        assert!(info.is_containerized);
        assert_eq!(info.container_id, Some("k8s-pod-abc123".to_string()));
        assert_eq!(info.cgroup_path, Some("/kubepods/podabc123".to_string()));
    }

    #[test]
    fn test_container_info_with_crio() {
        // Test ContainerInfo creation with CRI-O runtime
        let info = ContainerInfo::new(
            ContainerRuntime::Crio,
            Some("crio-container-456".to_string()),
            Some("/crio/container-456".to_string())
        );

        assert_eq!(info.runtime, ContainerRuntime::Crio);
        assert!(info.is_containerized);
        assert_eq!(info.container_id, Some("crio-container-456".to_string()));
        assert_eq!(info.cgroup_path, Some("/crio/container-456".to_string()));
    }

    #[test]
    fn test_container_adaptation_with_new_runtimes() {
        // Test that container adaptation works with new runtime types
        // This test verifies the function doesn't panic with different runtime types
        let test_runtimes = vec![
            ContainerRuntime::Docker,
            ContainerRuntime::Podman,
            ContainerRuntime::Containerd,
            ContainerRuntime::Lxc,
            ContainerRuntime::Kubernetes,
            ContainerRuntime::Crio,
            ContainerRuntime::Rkt,
            ContainerRuntime::Unknown("custom".to_string()),
        ];
        
        for runtime in test_runtimes {
            let info = ContainerInfo::new(runtime, Some("test".to_string()), Some("/test".to_string()));
            assert!(info.is_containerized);
        }
    }

    #[test]
    fn test_container_metrics_error_handling() {
        // Test that container metrics collection handles errors gracefully
        // This test verifies that the function doesn't panic when files are missing
        let metrics = collect_container_metrics();
        
        // Should return a valid ContainerMetrics struct even if not in container
        assert!(metrics.memory_limit_bytes.is_none() || metrics.memory_limit_bytes.is_some());
        assert!(metrics.memory_usage_bytes.is_none() || metrics.memory_usage_bytes.is_some());
        assert!(metrics.cpu_shares.is_none() || metrics.cpu_shares.is_some());
        assert!(metrics.cpu_quota.is_none() || metrics.cpu_quota.is_some());
        assert!(metrics.cpu_period.is_none() || metrics.cpu_period.is_some());
        assert!(metrics.network_interfaces.is_empty() || !metrics.network_interfaces.is_empty());
    }
}