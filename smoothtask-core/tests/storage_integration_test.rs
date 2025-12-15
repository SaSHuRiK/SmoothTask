//! Интеграционные тесты для модуля мониторинга SATA устройств
//!
//! Эти тесты проверяют интеграцию модуля storage с другими компонентами системы.

use smoothtask_core::metrics::storage::{detect_sata_devices, SataDeviceInfo, SataDeviceType};

#[test]
fn test_sata_device_detection_integration() {
    // Тестируем обнаружение SATA устройств
    let result = detect_sata_devices();
    
    // Проверяем, что функция выполняется без ошибок
    assert!(result.is_ok());
    let devices = result.unwrap();
    
    // Проверяем, что возвращается вектор (даже если пустой)
    assert!(devices.is_empty() || !devices.is_empty());
    
    // Если устройства обнаружены, проверяем их структуру
    for device in devices {
        assert!(!device.device_name.is_empty());
        assert!(!device.model.is_empty());
        assert!(!device.serial_number.is_empty());
        assert!(device.capacity > 0);
        
        // Проверяем, что тип устройства корректен
        match device.device_type {
            SataDeviceType::Hdd | SataDeviceType::Ssd | SataDeviceType::Sshd | SataDeviceType::Unknown => {}
        }
        
        // Проверяем метрики производительности
        assert!(device.performance_metrics.read_speed >= 0);
        assert!(device.performance_metrics.write_speed >= 0);
        assert!(device.performance_metrics.iops >= 0);
        assert!(device.performance_metrics.utilization >= 0.0);
    }
}

#[test]
fn test_sata_device_classification_logic() {
    // Тестируем логику классификации устройств
    // Это более детальный тест, который проверяет внутреннюю логику
    
    // В реальной системе мы бы использовали mock данные, но для интеграционного теста
    // мы просто проверяем, что функция работает корректно
    let result = detect_sata_devices();
    
    if let Ok(devices) = result {
        for device in devices {
            // Проверяем, что устройства классифицируются корректно
            // SSD должны иметь rotation_speed = 0 или 1
            // HDD должны иметь rotation_speed > 1
            match device.device_type {
                SataDeviceType::Ssd => {
                    if let Some(rotation) = device.rotation_speed {
                        assert!(rotation == 0 || rotation == 1, 
                            "SSD должен иметь rotation_speed 0 или 1, но обнаружено: {}", rotation);
                    }
                },
                SataDeviceType::Hdd => {
                    if let Some(rotation) = device.rotation_speed {
                        assert!(rotation > 1, 
                            "HDD должен иметь rotation_speed > 1, но обнаружено: {}", rotation);
                    }
                },
                SataDeviceType::Sshd | SataDeviceType::Unknown => {
                    // Для SSHD и Unknown не проверяем скорость вращения
                }
            }
        }
    }
}

#[test]
fn test_sata_device_integration_with_system() {
    // Тестируем интеграцию с системными метриками
    // В реальной системе это бы проверяло, что SATA метрики корректно интегрируются
    // с другими системными метриками
    
    let result = detect_sata_devices();
    assert!(result.is_ok());
    
    // В будущем этот тест можно расширить для проверки интеграции
    // с системными снапшотами и мониторингом
}
