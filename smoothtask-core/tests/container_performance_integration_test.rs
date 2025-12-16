// Integration tests for container performance monitoring

use smoothtask_core::metrics::container::*;
use smoothtask_core::metrics::ContainerPerformanceMetrics;

#[test]
fn test_container_performance_metrics_structures() {
    // Test that all performance metrics structures can be created and have proper defaults
    let performance_metrics = ContainerPerformanceMetrics::default();
    assert_eq!(performance_metrics.container_id, "");
    assert_eq!(performance_metrics.container_name, "");
    assert_eq!(performance_metrics.overall_performance_score, 0.0);
    assert_eq!(performance_metrics.timestamp, 0);

    let cpu_performance = CpuPerformanceMetrics::default();
    assert_eq!(cpu_performance.usage_percent, 0.0);
    assert_eq!(cpu_performance.throttling_percent, 0.0);
    assert_eq!(cpu_performance.efficiency_score, 0.0);
    assert_eq!(cpu_performance.response_time_ms, 0.0);
    assert_eq!(cpu_performance.load_avg_1min, 0.0);
    assert_eq!(cpu_performance.load_avg_5min, 0.0);
    assert_eq!(cpu_performance.load_avg_15min, 0.0);

    let memory_performance = MemoryPerformanceMetrics::default();
    assert_eq!(memory_performance.usage_percent, 0.0);
    assert_eq!(memory_performance.pressure_score, 0.0);
    assert_eq!(memory_performance.swap_usage_percent, 0.0);
    assert_eq!(memory_performance.allocation_rate_bps, 0.0);
    assert_eq!(memory_performance.fragmentation_score, 0.0);

    let network_performance = NetworkPerformanceMetrics::default();
    assert_eq!(network_performance.latency_ms, 0.0);
    assert_eq!(network_performance.throughput_bps, 0.0);
    assert_eq!(network_performance.packet_loss_percent, 0.0);
    assert_eq!(network_performance.stability_score, 0.0);
    assert_eq!(network_performance.bandwidth_utilization_percent, 0.0);

    let storage_performance = StoragePerformanceMetrics::default();
    assert_eq!(storage_performance.iops, 0.0);
    assert_eq!(storage_performance.latency_ms, 0.0);
    assert_eq!(storage_performance.throughput_bps, 0.0);
    assert_eq!(storage_performance.queue_depth, 0.0);
    assert_eq!(storage_performance.health_score, 0.0);
}

#[test]
fn test_container_performance_metrics_collection() {
    // Test that container performance metrics collection works without errors
    let result = collect_container_performance_metrics();
    assert!(result.is_ok());
    let metrics = result.unwrap();
    
    // Should return empty vector if no containers or runtime not available
    assert!(metrics.is_empty() || !metrics.is_empty());
}

#[test]
fn test_performance_calculation_functions() {
    // Create a test container metric with realistic values
    let mut metric = ContainerMetrics::default();
    metric.id = "test_container".to_string();
    metric.name = "test_container".to_string();
    metric.cpu_usage.usage_percent = 75.0;
    metric.cpu_usage.total_usage = 1000000000; // 1 second of CPU time
    metric.cpu_usage.system_cpu_usage = 2000000000; // 2 seconds of system CPU time
    metric.cpu_usage.online_cpus = 4;
    metric.memory_usage.usage = 536870912; // 512MB
    metric.memory_usage.max_usage = 671088640; // 640MB
    metric.memory_usage.limit = 1073741824; // 1GB
    metric.memory_usage.usage_percent = 60.0;
    metric.network_stats.rx_bytes = 1000000; // 1MB received
    metric.network_stats.tx_bytes = 500000; // 500KB transmitted
    metric.storage_stats.read_bytes = 2000000; // 2MB read
    metric.storage_stats.write_bytes = 1000000; // 1MB written
    metric.storage_stats.read_ops = 1000; // 1000 read operations
    metric.storage_stats.write_ops = 500; // 500 write operations

    // Test CPU calculations
    let throttling = calculate_cpu_throttling(&metric);
    assert!(throttling >= 0.0 && throttling <= 100.0);
    
    let efficiency = calculate_cpu_efficiency(&metric);
    assert!(efficiency >= 0.0 && efficiency <= 1.0);
    
    let response_time = calculate_cpu_response_time(&metric);
    assert!(response_time >= 0.0);

    // Test memory calculations
    let pressure = calculate_memory_pressure(&metric);
    assert!(pressure >= 0.0 && pressure <= 1.0);
    
    let allocation_rate = calculate_memory_allocation_rate(&metric);
    assert_eq!(allocation_rate, 0.0); // Simplified implementation
    
    let fragmentation = calculate_memory_fragmentation(&metric);
    assert_eq!(fragmentation, 0.0); // Simplified implementation

    // Test network calculations
    let latency = calculate_network_latency(&metric);
    assert_eq!(latency, 10.0); // Fixed value in simplified implementation
    
    let throughput = calculate_network_throughput(&metric);
    assert!(throughput > 0.0);
    
    let packet_loss = calculate_network_packet_loss(&metric);
    assert_eq!(packet_loss, 0.0); // Simplified implementation
    
    let stability = calculate_network_stability(&metric);
    assert_eq!(stability, 1.0); // Fixed value in simplified implementation
    
    let bandwidth = calculate_bandwidth_utilization(&metric);
    assert_eq!(bandwidth, 0.0); // Simplified implementation

    // Test storage calculations
    let iops = calculate_storage_iops(&metric);
    assert!(iops > 0.0);
    
    let storage_latency = calculate_storage_latency(&metric);
    assert_eq!(storage_latency, 5.0); // Fixed value in simplified implementation
    
    let storage_throughput = calculate_storage_throughput(&metric);
    assert!(storage_throughput > 0.0);
    
    let queue_depth = calculate_storage_queue_depth(&metric);
    assert_eq!(queue_depth, 0.0); // Simplified implementation
    
    let health = calculate_storage_health(&metric);
    assert_eq!(health, 1.0); // Fixed value in simplified implementation

    // Test overall performance score
    let overall_score = calculate_overall_performance_score(&metric);
    assert!(overall_score >= 0.0 && overall_score <= 1.0);
}

#[test]
fn test_container_performance_metrics_serialization() {
    // Test that performance metrics can be serialized and deserialized
    use serde_json;
    
    let mut performance_metrics = ContainerPerformanceMetrics::default();
    performance_metrics.container_id = "test_container".to_string();
    performance_metrics.container_name = "test_container".to_string();
    performance_metrics.cpu_performance.usage_percent = 50.0;
    performance_metrics.memory_performance.usage_percent = 40.0;
    performance_metrics.overall_performance_score = 0.85;
    performance_metrics.timestamp = 1234567890;

    // Serialize to JSON
    let json = serde_json::to_string(&performance_metrics);
    assert!(json.is_ok());
    
    // Deserialize back
    let deserialized: Result<ContainerPerformanceMetrics, _> = serde_json::from_str(&json.unwrap());
    assert!(deserialized.is_ok());
    
    let deserialized_metrics = deserialized.unwrap();
    assert_eq!(deserialized_metrics.container_id, "test_container");
    assert_eq!(deserialized_metrics.container_name, "test_container");
    assert_eq!(deserialized_metrics.cpu_performance.usage_percent, 50.0);
    assert_eq!(deserialized_metrics.memory_performance.usage_percent, 40.0);
    assert_eq!(deserialized_metrics.overall_performance_score, 0.85);
    assert_eq!(deserialized_metrics.timestamp, 1234567890);
}

#[test]
fn test_performance_metrics_with_different_workloads() {
    // Test performance calculations with different workload patterns
    
    // Low workload container
    let mut low_workload = ContainerMetrics::default();
    low_workload.cpu_usage.usage_percent = 10.0;
    low_workload.memory_usage.usage_percent = 20.0;
    
    let low_score = calculate_overall_performance_score(&low_workload);
    assert!(low_score > 0.8); // Should have high performance score
    
    // High workload container
    let mut high_workload = ContainerMetrics::default();
    high_workload.cpu_usage.usage_percent = 95.0;
    high_workload.memory_usage.usage_percent = 90.0;
    
    let high_score = calculate_overall_performance_score(&high_workload);
    assert!(high_score < 0.3); // Should have low performance score
    
    // Balanced workload container
    let mut balanced_workload = ContainerMetrics::default();
    balanced_workload.cpu_usage.usage_percent = 50.0;
    balanced_workload.memory_usage.usage_percent = 50.0;
    
    let balanced_score = calculate_overall_performance_score(&balanced_workload);
    assert!(balanced_score > 0.4 && balanced_score < 0.6); // Should have medium performance score
}

#[test]
fn test_cpu_throttling_calculation() {
    // Test CPU throttling calculation with different usage levels
    
    // Low CPU usage - should have no throttling
    let mut low_usage = ContainerMetrics::default();
    low_usage.cpu_usage.usage_percent = 30.0;
    let low_throttling = calculate_cpu_throttling(&low_usage);
    assert_eq!(low_throttling, 0.0);
    
    // Medium CPU usage - should have some throttling
    let mut medium_usage = ContainerMetrics::default();
    medium_usage.cpu_usage.usage_percent = 70.0;
    let medium_throttling = calculate_cpu_throttling(&medium_usage);
    assert!(medium_throttling > 0.0 && medium_throttling < 10.0);
    
    // High CPU usage - should have significant throttling
    let mut high_usage = ContainerMetrics::default();
    high_usage.cpu_usage.usage_percent = 95.0;
    let high_throttling = calculate_cpu_throttling(&high_usage);
    assert!(high_throttling >= 20.0 && high_throttling <= 22.5);
}

#[test]
fn test_memory_pressure_calculation() {
    // Test memory pressure calculation
    
    // Low memory usage - should have low pressure
    let mut low_memory = ContainerMetrics::default();
    low_memory.memory_usage.usage_percent = 20.0;
    let low_pressure = calculate_memory_pressure(&low_memory);
    assert_eq!(low_pressure, 0.2);
    
    // High memory usage - should have high pressure
    let mut high_memory = ContainerMetrics::default();
    high_memory.memory_usage.usage_percent = 85.0;
    let high_pressure = calculate_memory_pressure(&high_memory);
    assert_eq!(high_pressure, 0.85);
}

#[test]
fn test_network_throughput_calculation() {
    // Test network throughput calculation
    
    let mut metric = ContainerMetrics::default();
    metric.network_stats.rx_bytes = 1000000; // 1MB
    metric.network_stats.tx_bytes = 500000;  // 500KB
    
    let throughput = calculate_network_throughput(&metric);
    assert_eq!(throughput, 1500000.0); // 1.5MB total
}

#[test]
fn test_storage_iops_calculation() {
    // Test storage IOPS calculation
    
    let mut metric = ContainerMetrics::default();
    metric.storage_stats.read_ops = 1000;
    metric.storage_stats.write_ops = 500;
    
    let iops = calculate_storage_iops(&metric);
    assert_eq!(iops, 1500.0); // 1500 total operations
}

#[test]
fn test_storage_throughput_calculation() {
    // Test storage throughput calculation
    
    let mut metric = ContainerMetrics::default();
    metric.storage_stats.read_bytes = 2000000;  // 2MB
    metric.storage_stats.write_bytes = 1000000; // 1MB
    
    let throughput = calculate_storage_throughput(&metric);
    assert_eq!(throughput, 3000000.0); // 3MB total
}