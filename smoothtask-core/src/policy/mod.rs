//! Модуль для политики приоритетов процессов.
//!
//! Этот модуль предоставляет функциональность для определения приоритетов
//! процессов на основе правил и метрик системы.
//!
//! # Компоненты
//!
//! - **classes**: Классы приоритетов (CRIT_INTERACTIVE, INTERACTIVE, NORMAL, BACKGROUND, IDLE)
//!   и их маппинг на системные параметры (nice, ionice, cpu.weight, latency_nice)
//! - **engine**: Движок политики, применяющий жёсткие и семантические правила
//! - **dynamic**: Динамическое масштабирование приоритетов на основе нагрузки системы

pub mod classes;
pub mod dynamic;
pub mod engine;

/// Интеграция асинхронного логирования в модуль политик
use crate::logging::async_logging::{write_log_entry_async, write_log_batch_async};
use std::path::Path;
use anyhow::Result;

/// Асинхронное логирование политик
pub async fn log_policy_async(log_path: &Path, policy_data: &str) -> Result<()> {
    write_log_entry_async(log_path, policy_data).await
}

/// Асинхронное пакетное логирование политик
pub async fn log_policy_batch_async(log_path: &Path, policy_batch: &[String]) -> Result<()> {
    write_log_batch_async(log_path, policy_batch).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{NamedTempFile, TempDir};
    use tokio::runtime::Runtime;

    fn create_runtime() -> Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime")
    }

    #[test]
    fn test_log_policy_async() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("policy_test.log");

        runtime.block_on(async {
            let result = log_policy_async(&log_path, "Test policy data").await;
            assert!(result.is_ok(), "Policy logging should succeed");
        });
    }

    #[test]
    fn test_log_policy_batch_async() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("policy_batch_test.log");

        runtime.block_on(async {
            let policy_batch = vec![
                "Policy entry 1".to_string(),
                "Policy entry 2".to_string(),
                "Policy entry 3".to_string(),
            ];

            let result = log_policy_batch_async(&log_path, &policy_batch).await;
            assert!(result.is_ok(), "Batch policy logging should succeed");
        });
    }
}
