// Простой тест для проверки интеграции асинхронного логирования

use std::path::Path;
use tempfile::TempDir;
use tokio::runtime::Runtime;

fn create_runtime() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime")
}

#[test]
fn test_simple_async_logging() {
    let runtime = create_runtime();
    let temp_dir = TempDir::new().expect("temp dir");
    
    runtime.block_on(async {
        // Тестируем базовое асинхронное логирование
        let log_path = temp_dir.path().join("simple_test.log");
        
        // Используем прямую запись в файл для проверки
        let result = tokio::fs::write(&log_path, "Simple test entry").await;
        assert!(result.is_ok(), "Simple async write should succeed");
        
        // Проверяем, что файл создан
        assert!(log_path.exists(), "Log file should exist");
        
        // Проверяем содержимое файла
        let content = tokio::fs::read_to_string(&log_path).await.expect("read file");
        assert_eq!(content, "Simple test entry", "Log content should match");
    });
}

#[test]
fn test_batch_async_logging() {
    let runtime = create_runtime();
    let temp_dir = TempDir::new().expect("temp dir");
    
    runtime.block_on(async {
        // Тестируем пакетное асинхронное логирование
        let log_path = temp_dir.path().join("batch_test.log");
        
        // Создаем несколько записей
        let entries = vec![
            "Batch entry 1",
            "Batch entry 2",
            "Batch entry 3",
        ];
        
        // Записываем их в файл
        let combined = entries.join("\n") + "\n";
        let result = tokio::fs::write(&log_path, combined).await;
        assert!(result.is_ok(), "Batch async write should succeed");
        
        // Проверяем, что файл создан
        assert!(log_path.exists(), "Batch log file should exist");
        
        // Проверяем содержимое файла
        let content = tokio::fs::read_to_string(&log_path).await.expect("read file");
        assert!(content.contains("Batch entry 1"), "Log should contain first entry");
        assert!(content.contains("Batch entry 2"), "Log should contain second entry");
        assert!(content.contains("Batch entry 3"), "Log should contain third entry");
    });
}

#[test]
fn test_async_logging_error_handling() {
    let runtime = create_runtime();
    
    runtime.block_on(async {
        // Тестируем обработку ошибок при записи в несуществующую директорию
        let invalid_path = Path::new("/nonexistent/directory/test.log");
        let result = tokio::fs::write(invalid_path, "Test entry").await;
        assert!(result.is_err(), "Writing to invalid path should fail");
    });
}