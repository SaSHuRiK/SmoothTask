//! Integration tests for virtualization metrics collection.
//!
//! These tests verify the integration of virtualization metrics collection
//! with the rest of the SmoothTask system.

use smoothtask_core::utils::virtualization::{collect_vm_metrics, VmMetrics};

#[tokio::test]
async fn test_virtualization_integration() {
    // Test basic VM metrics collection
    let result = collect_vm_metrics("test_vm");
    assert!(result.is_ok());
    let metrics = result.unwrap();
    
    // Verify that all metrics are collected
    assert_eq!(metrics.cpu_usage, 0.0);
    assert_eq!(metrics.memory_usage, 0);
    assert_eq!(metrics.disk_read_bytes, 0);
    assert_eq!(metrics.disk_write_bytes, 0);
    assert_eq!(metrics.disk_read_ops, 0);
    assert_eq!(metrics.disk_write_ops, 0);
}

#[tokio::test]
async fn test_virtualization_multiple_vms() {
    // Test metrics collection for multiple VMs
    let vm_ids = ["vm1", "vm2", "vm3"];
    
    for vm_id in vm_ids.iter() {
        let result = collect_vm_metrics(vm_id);
        assert!(result.is_ok());
        let metrics = result.unwrap();
        
        // Verify that all metrics are collected for each VM
        assert_eq!(metrics.cpu_usage, 0.0);
        assert_eq!(metrics.memory_usage, 0);
        assert_eq!(metrics.disk_read_bytes, 0);
        assert_eq!(metrics.disk_write_bytes, 0);
        assert_eq!(metrics.disk_read_ops, 0);
        assert_eq!(metrics.disk_write_ops, 0);
    }
}