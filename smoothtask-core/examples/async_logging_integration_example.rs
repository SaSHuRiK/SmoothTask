//! Пример интеграции асинхронного логирования в основные компоненты SmoothTask.
//!
//! Этот пример демонстрирует, как интегрировать асинхронное логирование в:
//! - Модуль метрик
//! - Модуль классификации
//! - Модуль политик
//!
//! Запуск примера:
//! ```bash
//! cargo run --example async_logging_integration_example
//! ```

use anyhow::Result;
use smoothtask_core::logging::{
    AsyncLoggingIntegration, ClassifyAsyncLogger, MetricsAsyncLogger, PolicyAsyncLogger,
};
use std::path::Path;
use tempfile::TempDir;
use tokio::runtime::Runtime;

fn main() -> Result<()> {
    // Создаём временную директорию для логов
    let temp_dir = TempDir::new()?;
    let log_dir = temp_dir.path();

    println!("Async Logging Integration Example");
    println!("Log directory: {}", log_dir.display());
    println!();

    // Создаём runtime для асинхронных операций
    let runtime = Runtime::new()?;

    // Пример 1: Создание интеграции асинхронного логирования
    println!("=== Example 1: Creating Async Logging Integration ===");
    let integration = runtime.block_on(async {
        AsyncLoggingIntegration::new_default(log_dir)
            .expect("Failed to create async logging integration")
    });

    println!("Created async logging integration:");
    println!(
        "  Metrics log path: {}",
        integration.metrics_log_path().display()
    );
    println!(
        "  Classify log path: {}",
        integration.classify_log_path().display()
    );
    println!(
        "  Policy log path: {}",
        integration.policy_log_path().display()
    );
    println!();

    // Пример 2: Использование логгера метрик
    println!("=== Example 2: Using Metrics Logger ===");
    let metrics_logger = MetricsAsyncLogger::new(integration.clone());

    runtime.block_on(async {
        // Запись одиночного лога метрик
        metrics_logger
            .log_metrics("System metrics collected: CPU=45%, Memory=60%, Disk=30%")
            .await
            .expect("Failed to log metrics");

        // Запись пакета логов метрик
        let batch_logs = vec![
            "CPU metrics: user=30%, system=15%, idle=55%",
            "Memory metrics: total=16GB, used=9.6GB, free=6.4GB",
            "Disk metrics: read=10MB/s, write=5MB/s",
        ];

        metrics_logger
            .log_metrics_batch(&batch_logs)
            .await
            .expect("Failed to log metrics batch");

        println!("Successfully logged metrics entries");
    });
    println!();

    // Пример 3: Использование логгера классификации
    println!("=== Example 3: Using Classify Logger ===");
    let classify_logger = ClassifyAsyncLogger::new(integration.clone());

    runtime.block_on(async {
        // Запись одиночного лога классификации
        classify_logger
            .log_classify("Process classified: firefox -> INTERACTIVE")
            .await
            .expect("Failed to log classification");

        // Запись пакета логов классификации
        let batch_logs = vec![
            "Process classified: chrome -> INTERACTIVE",
            "Process classified: systemd -> BACKGROUND",
            "Process classified: gnome-shell -> CRIT_INTERACTIVE",
        ];

        classify_logger
            .log_classify_batch(&batch_logs)
            .await
            .expect("Failed to log classification batch");

        println!("Successfully logged classification entries");
    });
    println!();

    // Пример 4: Использование логгера политик
    println!("=== Example 4: Using Policy Logger ===");
    let policy_logger = PolicyAsyncLogger::new(integration.clone());

    runtime.block_on(async {
        // Запись одиночного лога политик
        policy_logger
            .log_policy("Applied priority: firefox -> nice=-10, ionice=1")
            .await
            .expect("Failed to log policy");

        // Запись пакета логов политик
        let batch_logs = vec![
            "Applied priority: chrome -> nice=-5, ionice=2",
            "Applied priority: systemd -> nice=10, ionice=7",
            "Applied priority: gnome-shell -> nice=-15, ionice=0",
        ];

        policy_logger
            .log_policy_batch(&batch_logs)
            .await
            .expect("Failed to log policy batch");

        println!("Successfully logged policy entries");
    });
    println!();

    // Пример 5: Оптимизация производительности логирования
    println!("=== Example 5: Optimizing Logging Performance ===");
    runtime.block_on(async {
        // Оптимизация для высокого давления памяти
        metrics_logger
            .optimize_logging(true, false, false)
            .await
            .expect("Failed to optimize logging");

        println!("Optimized logging for high memory pressure");
    });
    println!();

    // Пример 6: Очистка логов
    println!("=== Example 6: Cleaning Up Logs ===");
    runtime.block_on(async {
        // Выполняем очистку логов
        integration
            .cleanup_all_logs(false)
            .await
            .expect("Failed to cleanup logs");

        println!("Successfully cleaned up all logs");
    });
    println!();

    // Пример 7: Использование утилит для создания логгеров
    println!("=== Example 7: Using Utility Functions ===");
    runtime.block_on(async {
        // Создание логгера метрик с помощью утилиты
        let metrics_logger =
            create_default_metrics_logger(log_dir).expect("Failed to create metrics logger");

        // Создание логгера классификации с помощью утилиты
        let classify_logger =
            create_default_classify_logger(log_dir).expect("Failed to create classify logger");

        // Создание логгера политик с помощью утилиты
        let policy_logger =
            create_default_policy_logger(log_dir).expect("Failed to create policy logger");

        println!("Successfully created loggers using utility functions");

        // Использование созданных логгеров
        metrics_logger
            .log_metrics("Utility function test: metrics logger")
            .await
            .expect("Failed to log with utility metrics logger");

        classify_logger
            .log_classify("Utility function test: classify logger")
            .await
            .expect("Failed to log with utility classify logger");

        policy_logger
            .log_policy("Utility function test: policy logger")
            .await
            .expect("Failed to log with utility policy logger");

        println!("Successfully logged using utility-created loggers");
    });
    println!();

    println!("=== Example Completed Successfully ===");
    println!("All async logging integration examples have been executed successfully.");
    println!("Check the log directory for generated log files:");
    println!("  {}", log_dir.display());

    Ok(())
}
