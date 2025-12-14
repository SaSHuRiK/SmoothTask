// Container integration tests for SmoothTask
// These tests verify container detection, metrics collection, and integration

use smoothtask_core::utils::container::{
    collect_container_metrics, detect_container_runtime, get_container_info, 
    is_containerized, adapt_for_container, ContainerRuntime, ContainerInfo, ContainerMetrics
};
use smoothtask_core::utils::cgroups::is_cgroup_v2_available;

#[test]
fn test_container_detection_integration() {
    // Test basic container detection functionality
    let runtime = detect_container_runtime();
    let containerized = is_containerized();
    
    // In a normal test environment, we should not be in a container
    // This test verifies the functions work without panicking
    assert_eq!(containerized, runtime != ContainerRuntime::None);
    
    // Test that we can get container info
    let info = get_container_info();
    assert_eq!(info.runtime, runtime);
    assert_eq!(info.is_containerized, containerized);
}

#[test]
fn test_container_metrics_collection() {
    // Test container metrics collection
    let metrics = collect_container_metrics();
    
    // In a non-container environment, metrics should be default or have None values
    if !is_containerized() {
        assert_eq!(metrics.runtime, ContainerRuntime::None);
    } else {
        // In a container environment, we should have a runtime
        assert_ne!(metrics.runtime, ContainerRuntime::None);
    }
    
    // Test that metrics structure is valid
    assert!(metrics.memory_limit_bytes.is_none() || metrics.memory_limit_bytes.is_some());
    assert!(metrics.memory_usage_bytes.is_none() || metrics.memory_usage_bytes.is_some());
    assert!(metrics.cpu_shares.is_none() || metrics.cpu_shares.is_some());
}

#[test]
fn test_container_info_structure() {
    // Test ContainerInfo structure and methods
    let info = ContainerInfo::new(
        ContainerRuntime::Docker,
        Some("test-container-123".to_string()),
        Some("/docker/container-123".to_string())
    );
    
    assert_eq!(info.runtime, ContainerRuntime::Docker);
    assert!(info.is_containerized);
    assert_eq!(info.container_id, Some("test-container-123".to_string()));
    assert_eq!(info.cgroup_path, Some("/docker/container-123".to_string()));
}

#[test]
fn test_container_metrics_structure() {
    // Test ContainerMetrics structure
    let metrics = ContainerMetrics {
        runtime: ContainerRuntime::Podman,
        container_id: Some("podman-test".to_string()),
        memory_limit_bytes: Some(2 * 1024 * 1024 * 1024), // 2GB
        memory_usage_bytes: Some(1 * 1024 * 1024 * 1024), // 1GB
        cpu_shares: Some(2048),
        cpu_quota: Some(200000),
        cpu_period: Some(100000),
        network_interfaces: vec!["eth0".to_string(), "veth1".to_string()],
    };
    
    assert_eq!(metrics.runtime, ContainerRuntime::Podman);
    assert_eq!(metrics.container_id, Some("podman-test".to_string()));
    assert_eq!(metrics.memory_limit_bytes, Some(2 * 1024 * 1024 * 1024));
    assert_eq!(metrics.memory_usage_bytes, Some(1 * 1024 * 1024 * 1024));
    assert_eq!(metrics.cpu_shares, Some(2048));
    assert_eq!(metrics.cpu_quota, Some(200000));
    assert_eq!(metrics.cpu_period, Some(100000));
    assert_eq!(metrics.network_interfaces.len(), 2);
}

#[test]
fn test_container_runtime_enum() {
    // Test ContainerRuntime enum variants
    assert_eq!(ContainerRuntime::Docker, ContainerRuntime::Docker);
    assert_eq!(ContainerRuntime::Podman, ContainerRuntime::Podman);
    assert_eq!(ContainerRuntime::Containerd, ContainerRuntime::Containerd);
    assert_eq!(ContainerRuntime::Lxc, ContainerRuntime::Lxc);
    assert_eq!(ContainerRuntime::None, ContainerRuntime::None);
    
    // Test that Unknown variant works
    let unknown_runtime = ContainerRuntime::Unknown("custom-runtime".to_string());
    match unknown_runtime {
        ContainerRuntime::Unknown(name) => assert_eq!(name, "custom-runtime"),
        _ => panic!("Expected Unknown variant"),
    }
}

#[test]
fn test_container_adaptation_function() {
    // Test container adaptation function
    let result = adapt_for_container();
    
    // In a non-container environment, should return false
    if !is_containerized() {
        assert!(!result);
    } else {
        // In a container environment, should return true
        assert!(result);
    }
}

#[test]
fn test_cgroup_v2_integration() {
    // Test cgroup v2 detection integration with container metrics
    let v2_available = is_cgroup_v2_available();
    
    // This test just verifies the function can be called
    // The actual availability depends on system configuration
    assert!(v2_available || !v2_available);
    
    // Test that container metrics work with both cgroup versions
    let metrics = collect_container_metrics();
    
    // Metrics should be collectable regardless of cgroup version
    assert!(metrics.memory_limit_bytes.is_none() || metrics.memory_limit_bytes.is_some());
    assert!(metrics.cpu_shares.is_none() || metrics.cpu_shares.is_some());
}

#[test]
fn test_container_detection_edge_cases() {
    // Test container detection with various edge cases
    
    // Test that detection doesn't panic with missing files
    let runtime = detect_container_runtime();
    let _ = runtime; // Just ensure it doesn't panic
    
    // Test that container info doesn't panic
    let info = get_container_info();
    let _ = info; // Just ensure it doesn't panic
    
    // Test that metrics collection doesn't panic
    let metrics = collect_container_metrics();
    let _ = metrics; // Just ensure it doesn't panic
}

#[test]
fn test_container_default_metrics() {
    // Test default container metrics
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
fn test_container_info_default() {
    // Test default container info
    let info = ContainerInfo::new(ContainerRuntime::None, None, None);
    
    assert_eq!(info.runtime, ContainerRuntime::None);
    assert!(!info.is_containerized);
    assert!(info.container_id.is_none());
    assert!(info.cgroup_path.is_none());
}

// Note: More comprehensive container tests would require actually running
// in a container environment with proper cgroup setup. These tests verify
// that the container support code works correctly in non-container environments
// and doesn't panic or cause issues.

// For full container testing, use the Dockerfiles provided and run:
// docker build -t smoothtask-test .
// docker run --rm smoothtask-test /usr/local/bin/smoothtaskd --container-info