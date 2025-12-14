//! Интеграционные тесты для асинхронной системы логирования

use smoothtask_core::logging::{create_async_log_rotator, get_log_file_size_async};
use std::io::Write;
use tempfile::TempDir;
use tokio::runtime::Runtime;

fn create_runtime() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime")
}

#[test]
fn test_async_log_rotator_integration() {
    let runtime = create_runtime();
    
    runtime.block_on(async {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога с достаточным размером для ротации
        let mut file = std::fs::File::create(&log_path).expect("create log file");
        for i in 0..200 {
            writeln!(file, "Test log entry {}", i).expect("write to log");
        }
        drop(file);

        // Создаём асинхронный ротатор
        let rotator = create_async_log_rotator(100, 3, false, 0, 0, 0);

        // Проверяем, что ротация необходима
        let current_size = get_log_file_size_async(&log_path).await.expect("get size");
        let needs_rotation = rotator.needs_rotation(&log_path, current_size).await.expect("check rotation");
        assert!(needs_rotation, "Rotation should be needed for large file");

        // Выполняем ротацию
        rotator.rotate_log(&log_path).await.expect("rotation should succeed");

        // Проверяем, что оригинальный файл удалён
        assert!(!log_path.exists(), "Original log file should be removed");

        // Проверяем, что создан ротированный файл
        let rotated_files: Vec<_> = std::fs::read_dir(temp_dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "log"))
            .collect();

        assert_eq!(rotated_files.len(), 1, "Should have one rotated log file");
    });
}

#[test]
fn test_async_log_rotator_compression() {
    let runtime = create_runtime();
    
    runtime.block_on(async {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога с достаточным размером для ротации
        let mut file = std::fs::File::create(&log_path).expect("create log file");
        for i in 0..200 {
            writeln!(file, "Test log entry for compression {}", i).expect("write to log");
        }
        drop(file);

        // Создаём асинхронный ротатор с включенным сжатием
        let rotator = create_async_log_rotator(100, 3, true, 0, 0, 0);

        // Выполняем ротацию
        rotator.rotate_log(&log_path).await.expect("rotation with compression should succeed");

        // Проверяем, что оригинальный файл удалён
        assert!(!log_path.exists(), "Original log file should be removed");

        // Проверяем, что создан сжатый файл
        let compressed_files: Vec<_> = std::fs::read_dir(temp_dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "gz"))
            .collect();

        assert_eq!(compressed_files.len(), 1, "Should have one compressed log file");
    });
}

#[test]
fn test_async_log_rotator_cleanup() {
    let runtime = create_runtime();
    
    runtime.block_on(async {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога
        let mut file = std::fs::File::create(&log_path).expect("create log file");
        writeln!(file, "Test log entry").expect("write to log");
        drop(file);

        // Создаём асинхронный ротатор с ограничением на количество файлов
        let rotator = create_async_log_rotator(100, 2, false, 0, 0, 0);

        // Выполняем несколько ротаций
        for i in 0..5 {
            rotator.rotate_log(&log_path).await.expect("rotation should succeed");
            // Воссоздаём файл для следующей ротации
            let mut file = std::fs::File::create(&log_path).expect("recreate log file");
            writeln!(file, "Test log entry {}", i).expect("write to log");
            drop(file);
        }

        // Выполняем очистку
        rotator.cleanup_logs(&log_path).await.expect("cleanup should succeed");

        // Проверяем, что сохранено не более 2 ротированных файлов
        let rotated_files: Vec<_> = std::fs::read_dir(temp_dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "log"))
            .collect();

        assert!(rotated_files.len() <= 2, "Should have at most 2 rotated log files");
    });
}

#[test]
fn test_async_log_file_size() {
    let runtime = create_runtime();
    
    runtime.block_on(async {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога
        let mut file = std::fs::File::create(&log_path).expect("create log file");
        writeln!(file, "Test log entry").expect("write to log");
        drop(file);

        // Проверяем размер файла асинхронно
        let size = get_log_file_size_async(&log_path).await.expect("get size");
        assert!(size > 0, "File should have non-zero size");
    });
}

#[test]
fn test_async_rotator_config() {
    let runtime = create_runtime();
    
    runtime.block_on(async {
        // Создаём асинхронный ротатор
        let rotator = create_async_log_rotator(1000, 3, false, 0, 0, 0);

        // Проверяем конфигурацию
        let (max_size, max_files, compression, interval, max_age, max_total_size) = rotator.get_config();
        assert_eq!(max_size, 1000);
        assert_eq!(max_files, 3);
        assert!(!compression);
        assert_eq!(interval, 0);
        assert_eq!(max_age, 0);
        assert_eq!(max_total_size, 0);
    });
}
