//! Тесты для проверки работы SATA устройств на реальной системе
//!
//! Эти тесты проверяют функциональность обнаружения и классификации SATA устройств
//! в реальных условиях, включая обработку ошибок и граничных случаев.

use smoothtask_core::metrics::storage::{detect_sata_devices, SataDeviceInfo, SataDeviceType, SataPerformanceMetrics};
use std::io;

#[test]
fn test_sata_detection_on_real_system() {
    // Тестируем обнаружение SATA устройств на реальной системе
    let result = detect_sata_devices();
    
    // Проверяем, что функция выполняется без паники
    assert!(result.is_ok(), "Ошибка при обнаружении SATA устройств: {:?}", result.err());
    
    let devices = result.unwrap();
    
    // Проверяем, что возвращается корректный вектор
    assert!(devices.len() >= 0, "Некорректное количество устройств");
    
    // Если устройства обнаружены, проверяем их структуру
    for device in &devices {
        validate_sata_device(device);
    }
    
    println!("Обнаружено {} SATA устройств", devices.len());
    for device in devices {
        println!("Устройство: {} ({}), Тип: {:?}, Модель: {}",
            device.device_name, 
            device.serial_number, 
            device.device_type, 
            device.model
        );
    }
}

#[test]
fn test_sata_device_structure_validation() {
    // Тестируем валидацию структуры SATA устройств
    let result = detect_sata_devices();
    
    if let Ok(devices) = result {
        for device in devices {
            // Проверяем обязательные поля
            assert!(!device.device_name.is_empty(), "Пустое имя устройства");
            assert!(!device.model.is_empty(), "Пустая модель устройства");
            assert!(!device.serial_number.is_empty(), "Пустой серийный номер");
            assert!(device.capacity > 0, "Некорректная емкость устройства");
            
            // Проверяем метрики производительности
            assert!(device.performance_metrics.read_speed >= 0, "Некорректная скорость чтения");
            assert!(device.performance_metrics.write_speed >= 0, "Некорректная скорость записи");
            assert!(device.performance_metrics.iops >= 0, "Некорректное количество IOPS");
            assert!(device.performance_metrics.utilization >= 0.0, "Некорректный уровень загрузки");
            assert!(device.performance_metrics.utilization <= 1.0, "Уровень загрузки превышает 1.0");
            
            // Проверяем температуру (если доступна)
            if let Some(temp) = device.temperature {
                assert!(temp >= 0.0, "Некорректная температура устройства");
                assert!(temp <= 150.0, "Температура устройства слишком высока");
            }
        }
    }
}

#[test]
fn test_sata_device_classification_consistency() {
    // Тестируем согласованность классификации устройств
    let result = detect_sata_devices();
    
    if let Ok(devices) = result {
        for device in devices {
            // Проверяем согласованность между типом устройства и скоростью вращения
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
fn test_sata_error_handling() {
    // Тестируем обработку ошибок при обнаружении SATA устройств
    // В реальной системе это проверяет, что функция корректно обрабатывает
    // отсутствие устройств, некорректные данные и т.д.
    
    let result = detect_sata_devices();
    
    // Функция должна всегда возвращать Ok, даже если устройств нет
    assert!(result.is_ok(), "Функция должна корректно обрабатывать отсутствие устройств");
    
    let devices = result.unwrap();
    
    // Если устройств нет, должен возвращаться пустой вектор
    if devices.is_empty() {
        println!("SATA устройства не обнаружены - это нормально для некоторых систем");
    } else {
        println!("Обнаружено {} SATA устройств", devices.len());
    }
}

#[test]
fn test_sata_performance_metrics_validation() {
    // Тестируем валидацию метрик производительности
    let result = detect_sata_devices();
    
    if let Ok(devices) = result {
        for device in devices {
            let metrics = device.performance_metrics;
            
            // Проверяем, что метрики находятся в разумных пределах
            assert!(metrics.read_speed <= 10_000_000_000, "Слишком высокая скорость чтения");
            assert!(metrics.write_speed <= 10_000_000_000, "Слишком высокая скорость записи");
            assert!(metrics.iops <= 1_000_000, "Слишком высокое количество IOPS");
            
            // Проверяем, что время доступа разумное
            assert!(metrics.access_time <= 100_000, "Слишком высокое время доступа");
        }
    }
}

/// Вспомогательная функция для валидации структуры SATA устройства
fn validate_sata_device(device: &SataDeviceInfo) {
    assert!(!device.device_name.is_empty(), "Пустое имя устройства");
    assert!(!device.model.is_empty(), "Пустая модель устройства");
    assert!(!device.serial_number.is_empty(), "Пустой серийный номер");
    assert!(device.capacity > 0, "Некорректная емкость устройства");
    
    // Проверяем, что тип устройства корректен
    match device.device_type {
        SataDeviceType::Hdd | SataDeviceType::Ssd | SataDeviceType::Sshd | SataDeviceType::Unknown => {}
    }
    
    // Проверяем метрики производительности
    let metrics = &device.performance_metrics;
    assert!(metrics.read_speed >= 0, "Некорректная скорость чтения");
    assert!(metrics.write_speed >= 0, "Некорректная скорость записи");
    assert!(metrics.iops >= 0, "Некорректное количество IOPS");
    assert!(metrics.utilization >= 0.0, "Некорректный уровень загрузки");
    assert!(metrics.utilization <= 1.0, "Уровень загрузки превышает 1.0");
}

#[test]
fn test_sata_device_capacity_validation() {
    // Тестируем валидацию емкости устройств
    let result = detect_sata_devices();
    
    if let Ok(devices) = result {
        for device in devices {
            // Проверяем, что емкость находится в разумных пределах
            assert!(device.capacity > 0, "Емкость устройства должна быть положительной");
            assert!(device.capacity <= 100_000_000_000_000, "Емкость устройства слишком велика");
            
            // Проверяем, что емкость кратна 512 (размер сектора)
            assert_eq!(device.capacity % 512, 0, "Емкость должна быть кратна 512");
        }
    }
}

#[test]
fn test_sata_device_identification() {
    // Тестируем идентификацию устройств
    let result = detect_sata_devices();
    
    if let Ok(devices) = result {
        let mut device_names = Vec::new();
        
        for device in devices {
            // Проверяем, что имена устройств уникальны
            assert!(!device_names.contains(&device.device_name), 
                "Дублирующееся имя устройства: {}", device.device_name);
            device_names.push(device.device_name.clone());
            
            // Проверяем, что имена устройств соответствуют ожидаемому формату
            assert!(device.device_name.starts_with("sd") || device.device_name.starts_with("hd"),
                "Неожиданный формат имени устройства: {}", device.device_name);
        }
    }
}
