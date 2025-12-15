// Integration tests for Thunderbolt device detection functionality
use smoothtask_core::metrics::system::*;

#[test]
fn test_thunderbolt_device_metrics_structures() {
    // Test ThunderboltDeviceMetrics structure
    let device = ThunderboltDeviceMetrics {
        device_id: "test-device-1".to_string(),
        device_name: "Test Thunderbolt Device".to_string(),
        connection_speed_gbps: 40.0,
        status: "active".to_string(),
        temperature_c: Some(45.0),
        power_w: Some(15.0),
        device_classification: Some(DeviceClassification::ThunderboltDevice),
        performance_category: Some(PerformanceCategory::Normal),
    };

    assert_eq!(device.device_id, "test-device-1");
    assert_eq!(device.device_name, "Test Thunderbolt Device");
    assert_eq!(device.connection_speed_gbps, 40.0);
    assert_eq!(device.status, "active");
    assert_eq!(device.temperature_c, Some(45.0));
    assert_eq!(device.power_w, Some(15.0));
    assert_eq!(device.device_classification, Some(DeviceClassification::ThunderboltDevice));
    assert_eq!(device.performance_category, Some(PerformanceCategory::Normal));

    println!("✅ Thunderbolt device metrics structure test passed");
}

#[test]
fn test_thunderbolt_device_classification() {
    // Create a hardware device manager
    let manager = HardwareDeviceManager::default();

    // Test classification of different Thunderbolt device types
    
    // Test External GPU classification
    let gpu_device = manager.classify_thunderbolt_device("External Graphics Card", &40.0);
    assert_eq!(gpu_device, DeviceClassification::ExternalGpu);

    // Test Docking Station classification
    let dock_device = manager.classify_thunderbolt_device("Thunderbolt Dock", &40.0);
    assert_eq!(dock_device, DeviceClassification::DockingStation);

    // Test Storage Controller classification
    let storage_device = manager.classify_thunderbolt_device("NVMe SSD Enclosure", &40.0);
    assert_eq!(storage_device, DeviceClassification::StorageController);

    // Test Network device classification
    let network_device = manager.classify_thunderbolt_device("Thunderbolt Ethernet Adapter", &40.0);
    assert_eq!(network_device, DeviceClassification::Network);

    // Test Virtualization device classification
    let virt_device = manager.classify_thunderbolt_device("Virtualization Accelerator", &40.0);
    assert_eq!(virt_device, DeviceClassification::VirtualizationDevice);

    // Test Security device classification
    let security_device = manager.classify_thunderbolt_device("Security Module", &40.0);
    assert_eq!(security_device, DeviceClassification::SecurityDevice);

    // Test Thunderbolt device classification
    let tb_device = manager.classify_thunderbolt_device("Thunderbolt Controller", &40.0);
    assert_eq!(tb_device, DeviceClassification::ThunderboltDevice);

    // Test High-speed device classification
    let high_speed_device = manager.classify_thunderbolt_device("High Speed Device", &40.0);
    assert_eq!(high_speed_device, DeviceClassification::HighSpeedDevice);

    // Test default classification
    let other_device = manager.classify_thunderbolt_device("Unknown Device", &10.0);
    assert_eq!(other_device, DeviceClassification::Other);

    println!("✅ Thunderbolt device classification test passed");
}

#[test]
fn test_thunderbolt_performance_category() {
    // Create a hardware device manager
    let manager = HardwareDeviceManager::default();

    // Test performance category determination
    
    // Test normal performance
    let mut normal_device = ThunderboltDeviceMetrics {
        device_id: "normal-device".to_string(),
        device_name: "Normal Device".to_string(),
        connection_speed_gbps: 40.0,
        status: "active".to_string(),
        temperature_c: Some(45.0), // Normal temperature
        power_w: Some(15.0), // Normal power
        device_classification: None,
        performance_category: None,
    };
    
    let normal_category = manager.determine_thunderbolt_performance_category(&normal_device);
    assert_eq!(normal_category, PerformanceCategory::Normal);

    // Test high temperature
    let mut high_temp_device = normal_device.clone();
    high_temp_device.temperature_c = Some(80.0); // High temperature
    
    let high_temp_category = manager.determine_thunderbolt_performance_category(&high_temp_device);
    assert_eq!(high_temp_category, PerformanceCategory::HighTemperature);

    // Test high power
    let mut high_power_device = normal_device.clone();
    high_power_device.power_w = Some(60.0); // High power
    
    let high_power_category = manager.determine_thunderbolt_performance_category(&high_power_device);
    assert_eq!(high_power_category, PerformanceCategory::HighPower);

    // Test low performance (low connection speed)
    let mut low_perf_device = normal_device.clone();
    low_perf_device.connection_speed_gbps = 10.0; // Low speed
    
    let low_perf_category = manager.determine_thunderbolt_performance_category(&low_perf_device);
    assert_eq!(low_perf_category, PerformanceCategory::LowPerformance);

    println!("✅ Thunderbolt performance category test passed");
}

#[test]
fn test_thunderbolt_device_with_classification() {
    // Test ThunderboltDeviceMetricsWithClassification structure
    let device_with_class = ThunderboltDeviceMetricsWithClassification {
        device_metrics: ThunderboltDeviceMetrics {
            device_id: "test-device-1".to_string(),
            device_name: "Test Thunderbolt Device".to_string(),
            connection_speed_gbps: 40.0,
            status: "active".to_string(),
            temperature_c: Some(45.0),
            power_w: Some(15.0),
            device_classification: None,
            performance_category: None,
        },
        device_classification: DeviceClassification::ThunderboltDevice,
        performance_category: PerformanceCategory::Normal,
    };

    assert_eq!(device_with_class.device_metrics.device_id, "test-device-1");
    assert_eq!(device_with_class.device_classification, DeviceClassification::ThunderboltDevice);
    assert_eq!(device_with_class.performance_category, PerformanceCategory::Normal);

    println!("✅ Thunderbolt device with classification test passed");
}

#[test]
fn test_hardware_metrics_integration() {
    // Test that HardwareMetrics includes Thunderbolt devices
    let mut hardware = HardwareMetrics::default();
    
    // Add some Thunderbolt devices
    let thunderbolt_device = ThunderboltDeviceMetrics {
        device_id: "tb-device-1".to_string(),
        device_name: "Test Thunderbolt Device".to_string(),
        connection_speed_gbps: 40.0,
        status: "active".to_string(),
        temperature_c: Some(45.0),
        power_w: Some(15.0),
        device_classification: Some(DeviceClassification::ThunderboltDevice),
        performance_category: Some(PerformanceCategory::Normal),
    };
    
    hardware.thunderbolt_devices.push(thunderbolt_device);
    
    // Test that devices are included in the metrics
    assert_eq!(hardware.thunderbolt_devices.len(), 1);
    assert_eq!(hardware.thunderbolt_devices[0].device_id, "tb-device-1");

    // Test sanitized version
    let sanitized = hardware.sanitized();
    assert_eq!(sanitized.thunderbolt_devices.len(), 1);
    assert_eq!(sanitized.thunderbolt_devices[0].device_id, "tb-device-1");

    println!("✅ Hardware metrics integration test passed");
}

#[test]
fn test_device_classification_enum() {
    // Test that new device classifications are available
    
    // Test Thunderbolt-specific classifications
    assert_eq!(DeviceClassification::ThunderboltDevice, DeviceClassification::ThunderboltDevice);
    assert_eq!(DeviceClassification::ExternalGpu, DeviceClassification::ExternalGpu);
    assert_eq!(DeviceClassification::DockingStation, DeviceClassification::DockingStation);
    assert_eq!(DeviceClassification::VirtualizationDevice, DeviceClassification::VirtualizationDevice);
    assert_eq!(DeviceClassification::SecurityDevice, DeviceClassification::SecurityDevice);

    // Test that existing classifications still work
    assert_eq!(DeviceClassification::NvmeStorage, DeviceClassification::NvmeStorage);
    assert_eq!(DeviceClassification::SsdStorage, DeviceClassification::SsdStorage);
    assert_eq!(DeviceClassification::HddStorage, DeviceClassification::HddStorage);

    println!("✅ Device classification enum test passed");
}

#[test]
fn test_performance_category_enum() {
    // Test that performance categories work correctly
    
    assert_eq!(PerformanceCategory::Normal, PerformanceCategory::Normal);
    assert_eq!(PerformanceCategory::HighTemperature, PerformanceCategory::HighTemperature);
    assert_eq!(PerformanceCategory::HighPower, PerformanceCategory::HighPower);
    assert_eq!(PerformanceCategory::LowPerformance, PerformanceCategory::LowPerformance);

    println!("✅ Performance category enum test passed");
}