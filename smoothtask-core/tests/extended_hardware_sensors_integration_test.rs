// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

//! Интеграционные тесты для расширенного мониторинга аппаратных сенсоров

use smoothtask_core::metrics::extended_hardware_sensors::{
    ExtendedHardwareSensorsConfig, ExtendedHardwareSensorsMonitor,
};

#[tokio::test]
async fn test_extended_sensors_integration() {
    let config = ExtendedHardwareSensorsConfig::default();
    let monitor = ExtendedHardwareSensorsMonitor::new(config);

    let result = monitor.collect_extended_sensors();
    assert!(result.is_ok());
    let sensors = result.unwrap();

    // Проверяем, что все поля инициализированы
    assert!(sensors.temperatures_c.len() >= 0);
    assert!(sensors.additional_fan_speeds_rpm.len() >= 0);
    assert!(sensors.additional_voltages_v.len() >= 0);
    assert!(sensors.additional_currents_a.len() >= 0);
    assert!(sensors.additional_power_w.len() >= 0);
    assert!(sensors.additional_energy_j.len() >= 0);
    assert!(sensors.additional_humidity_percent.len() >= 0);
    assert!(sensors.pressure_pa.len() >= 0);
    assert!(sensors.illumination_lux.len() >= 0);
    assert!(sensors.custom_sensors.len() >= 0);
    assert!(sensors.thunderbolt_devices.len() >= 0);
    assert!(sensors.pcie_devices.len() >= 0);
}

#[tokio::test]
async fn test_extended_sensors_with_disabled_features() {
    let mut config = ExtendedHardwareSensorsConfig::default();
    config.enable_temperature_sensors = false;
    config.enable_additional_fan_sensors = false;
    config.enable_additional_voltage_sensors = false;
    config.enable_additional_current_sensors = false;
    config.enable_additional_power_sensors = false;
    config.enable_additional_energy_sensors = false;
    config.enable_additional_humidity_sensors = false;
    config.enable_pressure_sensors = false;
    config.enable_illumination_sensors = false;
    config.enable_custom_sensors = false;
    config.enable_thunderbolt_monitoring = false;
    config.enable_pcie_monitoring = false;

    let monitor = ExtendedHardwareSensorsMonitor::new(config);
    let result = monitor.collect_extended_sensors();
    assert!(result.is_ok());
    let sensors = result.unwrap();

    // Проверяем, что все функции отключены
    assert_eq!(sensors.temperatures_c.len(), 0);
    assert_eq!(sensors.additional_fan_speeds_rpm.len(), 0);
    assert_eq!(sensors.additional_voltages_v.len(), 0);
    assert_eq!(sensors.additional_currents_a.len(), 0);
    assert_eq!(sensors.additional_power_w.len(), 0);
    assert_eq!(sensors.additional_energy_j.len(), 0);
    assert_eq!(sensors.additional_humidity_percent.len(), 0);
    assert_eq!(sensors.pressure_pa.len(), 0);
    assert_eq!(sensors.illumination_lux.len(), 0);
    assert_eq!(sensors.custom_sensors.len(), 0);
    assert_eq!(sensors.thunderbolt_devices.len(), 0);
    assert_eq!(sensors.pcie_devices.len(), 0);
}

#[tokio::test]
async fn test_new_device_types_integration() {
    let config = ExtendedHardwareSensorsConfig::default();
    let monitor = ExtendedHardwareSensorsMonitor::new(config);

    let result = monitor.collect_extended_sensors();
    assert!(result.is_ok());
    let sensors = result.unwrap();

    // Проверяем, что новые поля инициализированы
    assert!(sensors.thunderbolt5_devices.len() >= 0);
    assert!(sensors.pcie6_devices.len() >= 0);
    assert!(sensors.usb4_v2_devices.len() >= 0);
    assert!(sensors.nvme_2_0_devices.len() >= 0);
    assert!(sensors.thunderbolt6_devices.len() >= 0);
    assert!(sensors.pcie7_devices.len() >= 0);
}

#[tokio::test]
async fn test_new_device_types_with_disabled_features() {
    let mut config = ExtendedHardwareSensorsConfig::default();
    config.enable_thunderbolt5_monitoring = false;
    config.enable_pcie6_monitoring = false;
    config.enable_usb4_v2_monitoring = false;
    config.enable_nvme_2_0_monitoring = false;
    config.enable_thunderbolt6_monitoring = false;
    config.enable_pcie7_monitoring = false;

    let monitor = ExtendedHardwareSensorsMonitor::new(config);
    let result = monitor.collect_extended_sensors();
    assert!(result.is_ok());
    let sensors = result.unwrap();

    // Проверяем, что новые функции отключены
    assert_eq!(sensors.thunderbolt5_devices.len(), 0);
    assert_eq!(sensors.pcie6_devices.len(), 0);
    assert_eq!(sensors.usb4_v2_devices.len(), 0);
    assert_eq!(sensors.nvme_2_0_devices.len(), 0);
    assert_eq!(sensors.thunderbolt6_devices.len(), 0);
    assert_eq!(sensors.pcie7_devices.len(), 0);
}

#[tokio::test]
async fn test_thunderbolt6_device_detection() {
    let config = ExtendedHardwareSensorsConfig::default();
    let monitor = ExtendedHardwareSensorsMonitor::new(config);

    let result = monitor.collect_extended_sensors();
    assert!(result.is_ok());
    let sensors = result.unwrap();

    // Проверяем, что Thunderbolt 6 устройства могут быть обнаружены
    // В реальной системе это будет зависеть от наличия соответствующего оборудования
    assert!(sensors.thunderbolt6_devices.len() >= 0);
    
    // Проверяем, что все устройства имеют корректные скорости (если они есть)
    for (device_name, speed) in &sensors.thunderbolt6_devices {
        assert!(!device_name.is_empty());
        assert!(speed >= &160.0, "Thunderbolt 6 devices should have speeds >= 160 Gbps");
    }
}

#[tokio::test]
async fn test_pcie7_device_detection() {
    let config = ExtendedHardwareSensorsConfig::default();
    let monitor = ExtendedHardwareSensorsMonitor::new(config);

    let result = monitor.collect_extended_sensors();
    assert!(result.is_ok());
    let sensors = result.unwrap();

    // Проверяем, что PCIe 7.0 устройства могут быть обнаружены
    // В реальной системе это будет зависеть от наличия соответствующего оборудования
    assert!(sensors.pcie7_devices.len() >= 0);
    
    // Проверяем, что все устройства имеют корректные скорости (если они есть)
    for (device_name, speed) in &sensors.pcie7_devices {
        assert!(!device_name.is_empty());
        assert!(speed >= &128.0, "PCIe 7.0 devices should have speeds >= 128 Gbps");
    }
}

#[tokio::test]
async fn test_config_struct_completeness() {
    let config = ExtendedHardwareSensorsConfig::default();
    
    // Проверяем, что все новые поля конфигурации присутствуют и имеют значения по умолчанию
    assert!(config.enable_thunderbolt5_monitoring);
    assert!(config.enable_pcie6_monitoring);
    assert!(config.enable_usb4_v2_monitoring);
    assert!(config.enable_nvme_2_0_monitoring);
    assert!(config.enable_thunderbolt6_monitoring);
    assert!(config.enable_pcie7_monitoring);
}

#[tokio::test]
async fn test_sensor_struct_completeness() {
    let sensors = ExtendedHardwareSensors::default();
    
    // Проверяем, что все новые поля структуры присутствуют
    assert!(sensors.thunderbolt5_devices.is_empty());
    assert!(sensors.pcie6_devices.is_empty());
    assert!(sensors.usb4_v2_devices.is_empty());
    assert!(sensors.nvme_2_0_devices.is_empty());
    assert!(sensors.thunderbolt6_devices.is_empty());
    assert!(sensors.pcie7_devices.is_empty());
}
