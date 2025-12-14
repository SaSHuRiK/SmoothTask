// Интеграционные тесты для асинхронного логирования

use smoothtask_core::logging::async_logging::{AsyncLogRotator, write_log_entry_async, write_log_batch_async};
use smoothtask_core::metrics::{log_metrics_async, log_metrics_batch_async};
use smoothtask_core::classify::{log_classification_async, log_classification_batch_async};
use smoothtask_core::policy::{log_policy_async, log_policy_batch_async};
use std::path::Path;
use tempfile::{TempDir, NamedTempFile};
use tokio::runtime::Runtime;

fn create_runtime() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime")
}

#[test]
fn test_async_logging_integration() {
    let runtime = create_runtime();
    let temp_dir = TempDir::new().expect("temp dir");
    
    runtime.block_on(async {
        // Тестируем базовое асинхронное логирование
        let log_path = temp_dir.path().join("basic_test.log");
        let result = write_log_entry_async(&log_path, "Basic test entry").await;
        assert!(result.is_ok(), "Basic async logging should succeed");
        
        // Тестируем пакетное асинхронное логирование
        let batch_path = temp_dir.path().join("batch_test.log");
        let batch = vec![
            "Batch entry 1".to_string(),
            "Batch entry 2".to_string(),
            "Batch entry 3".to_string(),
        ];
        let batch_result = write_log_batch_async(&batch_path, &batch).await;
        assert!(batch_result.is_ok(), "Batch async logging should succeed");
        
        // Тестируем интеграцию с модулем метрик
        let metrics_path = temp_dir.path().join("metrics_test.log");
        let metrics_result = log_metrics_async(&metrics_path, "Test metrics data").await;
        assert!(metrics_result.is_ok(), "Metrics async logging should succeed");
        
        // Тестируем пакетное логирование метрик
        let metrics_batch_path = temp_dir.path().join("metrics_batch_test.log");
        let metrics_batch = vec![
            "Metrics batch entry 1".to_string(),
            "Metrics batch entry 2".to_string(),
        ];
        let metrics_batch_result = log_metrics_batch_async(&metrics_batch_path, &metrics_batch).await;
        assert!(metrics_batch_result.is_ok(), "Metrics batch async logging should succeed");
        
        // Тестируем интеграцию с модулем классификации
        let classification_path = temp_dir.path().join("classification_test.log");
        let classification_result = log_classification_async(&classification_path, "Test classification data").await;
        assert!(classification_result.is_ok(), "Classification async logging should succeed");
        
        // Тестируем пакетное логирование классификации
        let classification_batch_path = temp_dir.path().join("classification_batch_test.log");
        let classification_batch = vec![
            "Classification batch entry 1".to_string(),
            "Classification batch entry 2".to_string(),
        ];
        let classification_batch_result = log_classification_batch_async(&classification_batch_path, &classification_batch).await;
        assert!(classification_batch_result.is_ok(), "Classification batch async logging should succeed");
        
        // Тестируем интеграцию с модулем политик
        let policy_path = temp_dir.path().join("policy_test.log");
        let policy_result = log_policy_async(&policy_path, "Test policy data").await;
        assert!(policy_result.is_ok(), "Policy async logging should succeed");
        
        // Тестируем пакетное логирование политик
        let policy_batch_path = temp_dir.path().join("policy_batch_test.log");
        let policy_batch = vec![
            "Policy batch entry 1".to_string(),
            "Policy batch entry 2".to_string(),
        ];
        let policy_batch_result = log_policy_batch_async(&policy_batch_path, &policy_batch).await;
        assert!(policy_batch_result.is_ok(), "Policy batch async logging should succeed");
    });
}

#[test]
fn test_async_logging_with_rotation() {
    let runtime = create_runtime();
    let temp_dir = TempDir::new().expect("temp dir");
    
    runtime.block_on(async {
        let log_path = temp_dir.path().join("rotation_test.log");
        let rotator = AsyncLogRotator::new(100, 3, true, 0, 0, 0);
        
        // Записываем достаточно данных для ротации
        for i in 0..20 {
            let entry = format!("Rotation test entry {}", i);
            let result = write_log_entry_async(&log_path, &entry).await;
            assert!(result.is_ok(), "Log entry {} should succeed", i);
        }
        
        // Проверяем, что ротация произошла
        let result = rotator.rotate_log(&log_path).await;
        assert!(result.is_ok(), "Rotation should succeed");
        
        // Проверяем, что оригинальный файл удален
        assert!(!log_path.exists(), "Original log file should be removed after rotation");
        
        // Проверяем, что создан ротированный файл
        let rotated_files: Vec<_> = std::fs::read_dir(temp_dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "gz"))
            .collect();
        
        assert_eq!(rotated_files.len(), 1, "Should have one compressed rotated log file");
    });
}

#[test]
fn test_async_logging_error_handling() {
    let runtime = create_runtime();
    let temp_dir = TempDir::new().expect("temp dir");
    
    runtime.block_on(async {
        // Тестируем обработку ошибок при записи в несуществующую директорию
        let invalid_path = Path::new("/nonexistent/directory/test.log");
        let result = write_log_entry_async(invalid_path, "Test entry").await;
        assert!(result.is_err(), "Writing to invalid path should fail");
        
        // Тестируем обработку ошибок при пакетной записи
        let batch_result = write_log_batch_async(invalid_path, &["Test entry".to_string()]).await;
        assert!(batch_result.is_err(), "Batch writing to invalid path should fail");
    });
}

#[test]
fn test_async_logging_performance() {
    let runtime = create_runtime();
    let temp_dir = TempDir::new().expect("temp dir");
    
    runtime.block_on(async {
        let log_path = temp_dir.path().join("performance_test.log");
        let start_time = std::time::Instant::now();
        
        // Записываем большое количество логов
        let batch_size = 1000;
        let mut batch = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            batch.push(format!("Performance test entry {}", i));
        }
        
        let result = write_log_batch_async(&log_path, &batch).await;
        assert!(result.is_ok(), "Performance test should succeed");
        
        let duration = start_time.elapsed();
        println!("Performance test completed in {:?} for {} entries", duration, batch_size);
        
        // Проверяем, что файл создан и содержит данные
        assert!(log_path.exists(), "Performance test log file should exist");
        let metadata = std::fs::metadata(&log_path).expect("get metadata");
        assert!(metadata.len() > 0, "Performance test log file should not be empty");
    });
}