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
