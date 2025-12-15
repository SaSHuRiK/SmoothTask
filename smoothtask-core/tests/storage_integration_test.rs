// Интеграционный тест для модуля storage
// Проверяет, что функции обнаружения устройств хранения работают корректно

use smoothtask_core::metrics::storage::{detect_all_storage_devices, detect_sata_devices, detect_nvme_devices};
use std::io::Result;

#[test]
fn test_storage_module_compiles() {
    // Этот тест просто проверяет, что модуль компилируется
    // и функции доступны
    
    // Проверяем, что функции обнаружения устройств доступны
    let _: fn() -> Result<_> = detect_sata_devices;
    let _: fn() -> Result<_> = detect_nvme_devices;
    let _: fn() -> Result<_> = detect_all_storage_devices;
    
    // Если мы дошли до этого места, значит модуль компилируется корректно
    assert!(true);
}

#[test]
fn test_storage_detection_functions_return_results() {
    // Этот тест проверяет, что функции обнаружения возвращают результаты
    // даже если устройства не найдены
    
    // Тестируем обнаружение SATA устройств
    let sata_result = detect_sata_devices();
    assert!(sata_result.is_ok(), "SATA detection should return Ok result");
    
    // Тестируем обнаружение NVMe устройств
    let nvme_result = detect_nvme_devices();
    assert!(nvme_result.is_ok(), "NVMe detection should return Ok result");
    
    // Тестируем комплексное обнаружение
    let comprehensive_result = detect_all_storage_devices();
    assert!(comprehensive_result.is_ok(), "Comprehensive storage detection should return Ok result");
    
    // В тестовой среде без реальных устройств результаты могут быть пустыми
    let comprehensive_data = comprehensive_result.unwrap();
    println!("Found {} SATA devices and {} NVMe devices", 
             comprehensive_data.sata_devices.len(), 
             comprehensive_data.nvme_devices.len());
    
    // Главное, что функции не падают и возвращают корректные структуры данных
    assert!(true);
}

#[test]
fn test_storage_module_structures() {
    // Этот тест проверяет, что структуры данных модуля storage корректны
    use smoothtask_core::metrics::storage::{
        SataDeviceInfo, NvmeDeviceInfo, SataDeviceType, NvmeDeviceType,
        SataPerformanceMetrics, NvmePerformanceMetrics, StorageDetectionResult
    };
    
    // Проверяем, что структуры можно создать
    let sata_device = SataDeviceInfo {
        device_name: "sda".to_string(),
        model: "Test Model".to_string(),
        serial_number: "TEST123456".to_string(),
        device_type: SataDeviceType::Ssd,
        rotation_speed: Some(0),
        capacity: 1024 * 1024 * 1024, // 1 GB
        temperature: Some(45.0),
        performance_metrics: SataPerformanceMetrics::default(),
    };
    
    let nvme_device = NvmeDeviceInfo {
        device_name: "nvme0n1".to_string(),
        model: "Test NVMe".to_string(),
        serial_number: "NVME123456".to_string(),
        device_type: NvmeDeviceType::Nvme4_0,
        capacity: 1024 * 1024 * 1024, // 1 GB
        temperature: Some(50.0),
        pcie_generation: Some(4.0),
        pcie_lanes: Some(4),
        performance_metrics: NvmePerformanceMetrics::default(),
    };
    
    let detection_result = StorageDetectionResult {
        sata_devices: vec![sata_device],
        nvme_devices: vec![nvme_device],
    };
    
    // Проверяем, что структуры содержат ожидаемые данные
    assert_eq!(detection_result.sata_devices.len(), 1);
    assert_eq!(detection_result.nvme_devices.len(), 1);
    assert_eq!(detection_result.sata_devices[0].device_name, "sda");
    assert_eq!(detection_result.nvme_devices[0].device_name, "nvme0n1");
}

#[test]
fn test_nvme_device_type_enum() {
    // Этот тест проверяет, что перечисление типов NVMe устройств работает корректно
    use smoothtask_core::metrics::storage::NvmeDeviceType;
    
    // Проверяем все варианты перечисления
    let types = vec![
        NvmeDeviceType::Nvme1x,
        NvmeDeviceType::Nvme2_0,
        NvmeDeviceType::Nvme3_0,
        NvmeDeviceType::Nvme4_0,
        NvmeDeviceType::Nvme5_0,
        NvmeDeviceType::Unknown,
    ];
    
    // Проверяем, что все варианты доступны
    assert_eq!(types.len(), 6);
    
    // Проверяем, что варианты можно сравнивать
    assert!(matches!(types[0], NvmeDeviceType::Nvme1x));
    assert!(matches!(types[1], NvmeDeviceType::Nvme2_0));
    assert!(matches!(types[5], NvmeDeviceType::Unknown));
}
