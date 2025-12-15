//! Модуль для сбора метрик системы и процессов.
//!
//! Этот модуль предоставляет функциональность для сбора различных метрик:
//! - Системные метрики (CPU, память, PSI)
//! - Метрики процессов (CPU, IO, память)
//! - Метрики энергопотребления процессов (RAPL, /proc/power, eBPF)
//! - Метрики окон и фокуса (X11/Wayland)
//! - Метрики аудио (PipeWire/PulseAudio, XRUN)
//! - Метрики ввода пользователя (evdev)
//! - Метрики scheduling latency
//! - Метрики GPU (использование, память, температура, энергопотребление)
//! - Метрики производительности приложений (задержки, FPS, использование ресурсов)
//!
//! # Компоненты
//!
//! - **system**: Глобальные метрики системы из /proc и PSI
//! - **process**: Метрики отдельных процессов
//! - **process_energy**: Мониторинг энергопотребления процессов
//! - **windows**: Интроспекция окон через X11/Wayland
//! - **audio**: Метрики аудио-системы (PipeWire/PulseAudio)
//! - **input**: Отслеживание активности пользователя
//! - **scheduling_latency**: Измерение задержек планировщика
//! - **nvml_wrapper**: Расширенный мониторинг NVIDIA GPU через NVML
//! - **amdgpu_wrapper**: Расширенный мониторинг AMD GPU через AMDGPU
//! - **gpu**: Мониторинг GPU устройств и их метрик
//! - **ebpf**: Высокопроизводительный сбор метрик через eBPF
//! - **filesystem_monitor**: Мониторинг файловой системы в реальном времени
//! - **extended_hardware_sensors**: Расширенный мониторинг аппаратных сенсоров
//! - **app_performance**: Метрики производительности приложений
//! - **ml_performance**: Метрики производительности ML-моделей и экспорт в Prometheus
//! - **performance_profiler**: Профилирование производительности и анализ узких мест
//! - **hardware_acceleration**: Мониторинг аппаратного ускорения (VA-API, VDPAU, CUDA)
//! - **container**: Мониторинг контейнеров Docker/Podman
//! - **vm**: Мониторинг и управление виртуальными машинами (QEMU/KVM, VirtualBox)
//! - **thunderbolt**: Мониторинг Thunderbolt устройств
//! - **pcie**: Мониторинг PCIe устройств

pub mod amdgpu_wrapper;
pub mod app_performance;
pub mod audio;
pub mod audio_pipewire;
pub mod batch_processor;
pub mod cache;
pub mod container;
pub mod custom;
pub mod ebpf;
pub mod energy_monitoring;
pub mod extended_hardware_sensors;
pub mod filesystem_monitor;
pub mod gpu;
pub mod hardware_acceleration;
pub mod input;
pub mod ml_performance;
pub mod network;
pub mod nvml_wrapper;
pub mod performance_profiler;
pub mod process;
pub mod process_energy;
pub mod process_gpu;
pub mod process_network;
pub mod prometheus_exporter;
pub mod scheduling_latency;
pub mod system;
pub mod vm;
pub mod windows;
pub mod windows_wayland;
pub mod windows_x11;

/// Интеграция асинхронного логирования в модуль метрик
use crate::logging::async_logging::{write_log_entry_async, write_log_batch_async};
use std::path::Path;
use anyhow::Result;

/// Асинхронное логирование метрик
pub async fn log_metrics_async(log_path: &Path, metrics_data: &str) -> Result<()> {
    write_log_entry_async(log_path, metrics_data).await
}

/// Асинхронное пакетное логирование метрик
pub async fn log_metrics_batch_async(log_path: &Path, metrics_batch: &[String]) -> Result<()> {
    write_log_batch_async(log_path, metrics_batch).await
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
    fn test_log_metrics_async() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("metrics_test.log");

        runtime.block_on(async {
            let result = log_metrics_async(&log_path, "Test metrics data").await;
            assert!(result.is_ok(), "Metrics logging should succeed");
        });
    }

    #[test]
    fn test_log_metrics_batch_async() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("metrics_batch_test.log");

        runtime.block_on(async {
            let metrics_batch = vec![
                "Metrics entry 1".to_string(),
                "Metrics entry 2".to_string(),
                "Metrics entry 3".to_string(),
            ];

            let result = log_metrics_batch_async(&log_path, &metrics_batch).await;
            assert!(result.is_ok(), "Batch metrics logging should succeed");
        });
    }
}
