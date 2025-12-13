//! Мониторинг энергопотребления процессов.
//!
//! Этот модуль предоставляет функциональность для сбора метрик энергопотребления
//! на уровне отдельных процессов. Поддерживаются различные источники данных:
//! - /proc/[pid]/power/energy_uj (экспериментальный интерфейс)
//! - RAPL (Running Average Power Limit) через /sys/class/powercap
//! - eBPF мониторинг (через интеграцию с ebpf модулем)
//!
//! # Основные компоненты
//!
//! - **ProcessEnergyMonitor**: Основной монитор энергопотребления процессов
//! - **ProcessEnergyStats**: Структура для хранения статистики энергопотребления
//! - **EnergySource**: Перечисление доступных источников данных
//!
//! # Пример использования
//!
//! ```no_run
//! use smoothtask_core::metrics::process_energy;
//!
//! // Создать монитор энергопотребления
//! let monitor = ProcessEnergyMonitor::new();
//!
//! // Собрать метрики для конкретного процесса
//! let stats = monitor.collect_process_energy(1234).await?;
//!
//! if let Some(stats) = stats {
//!     println!("Process {} energy: {} µJ ({} W)", 
//!              stats.pid, stats.energy_uj, stats.power_w);
//! }
//! ```

use crate::logging::snapshots::ProcessRecord;
use anyhow::Result;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs as tokio_fs;

/// Источник данных для метрик энергопотребления.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EnergySource {
    /// Данные из /proc/[pid]/power/energy_uj
    ProcPower,
    /// Данные из RAPL (Running Average Power Limit)
    Rapl,
    /// Данные из eBPF мониторинга
    Ebpf,
    /// Данные недоступны
    None,
}

impl Default for EnergySource {
    fn default() -> Self {
        Self::None
    }
}

/// Статистика энергопотребления процесса.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProcessEnergyStats {
    /// Идентификатор процесса
    pub pid: i32,
    /// Потребление энергии в микроджоулях
    pub energy_uj: u64,
    /// Мгновенная мощность в ваттах
    pub power_w: f32,
    /// Время последнего измерения (timestamp в секундах)
    pub timestamp: u64,
    /// Источник данных
    pub source: EnergySource,
    /// Признак достоверности данных
    pub is_reliable: bool,
}

impl Default for ProcessEnergyStats {
    fn default() -> Self {
        Self {
            pid: 0,
            energy_uj: 0,
            power_w: 0.0,
            timestamp: 0,
            source: EnergySource::None,
            is_reliable: false,
        }
    }
}

/// Основной монитор энергопотребления процессов.
#[derive(Debug)]
pub struct ProcessEnergyMonitor {
    /// Включить использование RAPL
    enable_rapl: bool,
    /// Включить интеграцию с eBPF
    enable_ebpf: bool,
    /// Базовый путь к RAPL интерфейсам
    rapl_base_path: PathBuf,
    // Кэш последних измерений
    // В будущем можно добавить кэширование
}

impl ProcessEnergyMonitor {
    /// Создать новый монитор энергопотребления.
    pub fn new() -> Self {
        Self {
            enable_rapl: true,
            enable_ebpf: true,
            rapl_base_path: PathBuf::from("/sys/class/powercap/intel-rapl"),
        }
    }

    /// Создать монитор с кастомной конфигурацией.
    pub fn with_config(enable_rapl: bool, enable_ebpf: bool) -> Self {
        Self {
            enable_rapl,
            enable_ebpf,
            rapl_base_path: PathBuf::from("/sys/class/powercap/intel-rapl"),
        }
    }

    /// Собрать метрики энергопотребления для процесса.
    ///
    /// Пробует получить данные из доступных источников в порядке приоритета:
    /// 1. /proc/[pid]/power/energy_uj (наиболее точный)
    /// 2. eBPF мониторинг (если включен)
    /// 3. RAPL (если доступен)
    ///
    /// Возвращает `None`, если ни один источник не доступен.
    pub async fn collect_process_energy(&self, pid: i32) -> Result<Option<ProcessEnergyStats>> {
        // Пробуем получить данные из /proc/[pid]/power/energy_uj
        if let Some(stats) = self.try_collect_proc_power_energy(pid).await? {
            return Ok(Some(stats));
        }

        // Пробуем получить данные из eBPF (если включено)
        if self.enable_ebpf {
            if let Some(stats) = self.try_collect_ebpf_energy(pid).await? {
                return Ok(Some(stats));
            }
        }

        // Пробуем получить данные из RAPL (если включено)
        if self.enable_rapl {
            if let Some(stats) = self.try_collect_rapl_energy(pid).await? {
                return Ok(Some(stats));
            }
        }

        Ok(None)
    }

    /// Попробовать получить данные из /proc/[pid]/power/energy_uj.
    async fn try_collect_proc_power_energy(&self, pid: i32) -> Result<Option<ProcessEnergyStats>> {
        let energy_path = format!("/proc/{}/power/energy_uj", pid);
        
        if let Ok(energy_content) = tokio_fs::read_to_string(&energy_path).await {
            if let Ok(energy_uj) = energy_content.trim().parse::<u64>() {
                let power_w = energy_uj as f32 / 1_000_000.0;
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)?
                    .as_secs();

                return Ok(Some(ProcessEnergyStats {
                    pid,
                    energy_uj,
                    power_w,
                    timestamp,
                    source: EnergySource::ProcPower,
                    is_reliable: true,
                }));
            }
        }

        Ok(None)
    }

    /// Попробовать получить данные из eBPF мониторинга.
    async fn try_collect_ebpf_energy(&self, pid: i32) -> Result<Option<ProcessEnergyStats>> {
        // В реальной реализации нужно получить доступ к eBPF метрикам
        // Это временная заглушка - в будущем нужно интегрировать с EbpfMetrics
        
        // Пробуем получить eBPF метрики через глобальный коллектор
        // Это временное решение - в будущем нужно более тесную интеграцию
        let ebpf_metrics = crate::metrics::system::collect_ebpf_metrics();
        
        if let Some(ebpf_metrics) = ebpf_metrics {
            if let Some(process_energy_details) = &ebpf_metrics.process_energy_details {
                for detail in process_energy_details {
                    if detail.pid == pid as u32 {
                        return Ok(Some(ProcessEnergyStats {
                            pid,
                            energy_uj: detail.energy_uj,
                            power_w: detail.energy_w,
                            timestamp: detail.last_update_ns / 1_000_000_000, // Convert ns to s
                            source: EnergySource::Ebpf,
                            is_reliable: true,
                        }));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Попробовать получить данные из RAPL.
    async fn try_collect_rapl_energy(&self, pid: i32) -> Result<Option<ProcessEnergyStats>> {
        if !self.rapl_base_path.exists() {
            return Ok(None);
        }

        let mut total_energy_uj = 0;
        let mut domain_count = 0;

        // Чтение RAPL доменов
        let mut read_dir = tokio_fs::read_dir(&self.rapl_base_path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let domain_path = entry.path();
            let energy_path = domain_path.join("energy_uj");

            if energy_path.exists() {
                if let Ok(energy_content) = tokio_fs::read_to_string(&energy_path).await {
                    if let Ok(energy_uj) = energy_content.trim().parse::<u64>() {
                        total_energy_uj += energy_uj;
                        domain_count += 1;
                    }
                }
            }
        }

        if domain_count > 0 && total_energy_uj > 0 {
            // Для RAPL нужно более сложное сопоставление процессов с доменами
            // Это упрощенная версия - в реальности нужно учитывать топологию системы
            let power_w = total_energy_uj as f32 / 1_000_000.0;
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs();

            return Ok(Some(ProcessEnergyStats {
                pid,
                energy_uj: total_energy_uj,
                power_w,
                timestamp,
                source: EnergySource::Rapl,
                is_reliable: domain_count >= 2, // Более достоверно с несколькими доменами
            }));
        }

        Ok(None)
    }

    /// Обновить ProcessRecord данными о энергопотреблении.
    pub fn enhance_process_record(
        &self,
        mut record: ProcessRecord,
        energy_stats: Option<ProcessEnergyStats>
    ) -> ProcessRecord {
        if let Some(stats) = energy_stats {
            record.energy_uj = Some(stats.energy_uj);
            record.power_w = Some(stats.power_w);
            record.energy_timestamp = Some(stats.timestamp);
        }
        record
    }

    /// Собрать метрики для нескольких процессов.
    pub async fn collect_batch_energy(&self, pids: &[i32]) -> Result<Vec<ProcessEnergyStats>> {
        let mut results = Vec::new();

        for &pid in pids {
            if let Some(stats) = self.collect_process_energy(pid).await? {
                results.push(stats);
            }
        }

        Ok(results)
    }

    /// Синхронная версия сбора метрик энергопотребления.
    /// Используется для интеграции с синхронным кодом.
    pub fn collect_process_energy_sync(&self, pid: i32) -> Result<Option<ProcessEnergyStats>> {
        // Используем блокирующий runtime для синхронного выполнения
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(self.collect_process_energy(pid))
    }
}

/// Глобальный экземпляр монитора энергопотребления.
///
/// Предоставляет удобный доступ к функциональности мониторинга энергопотребления
/// из других частей системы.
#[derive(Debug)]
pub struct GlobalProcessEnergyMonitor;

impl GlobalProcessEnergyMonitor {
    /// Собрать метрики энергопотребления для процесса.
    pub async fn collect_process_energy(pid: i32) -> Result<Option<ProcessEnergyStats>> {
        static MONITOR: once_cell::sync::OnceCell<ProcessEnergyMonitor> = once_cell::sync::OnceCell::new();
        
        let monitor = MONITOR.get_or_init(|| ProcessEnergyMonitor::new());
        monitor.collect_process_energy(pid).await
    }

    /// Обновить ProcessRecord данными о энергопотреблении.
    pub async fn enhance_process_record(record: ProcessRecord) -> Result<ProcessRecord> {
        let pid = record.pid;
        let energy_stats = Self::collect_process_energy(pid).await?;
        
        static MONITOR: once_cell::sync::OnceCell<ProcessEnergyMonitor> = once_cell::sync::OnceCell::new();
        let monitor = MONITOR.get_or_init(|| ProcessEnergyMonitor::new());
        Ok(monitor.enhance_process_record(record, energy_stats))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_process_energy_stats_default() {
        let stats = ProcessEnergyStats::default();
        assert_eq!(stats.pid, 0);
        assert_eq!(stats.energy_uj, 0);
        assert_eq!(stats.power_w, 0.0);
        assert_eq!(stats.timestamp, 0);
        assert_eq!(stats.source, EnergySource::None);
        assert!(!stats.is_reliable);
    }

    #[test]
    async fn test_energy_source_serialization() {
        let source = EnergySource::ProcPower;
        let serialized = serde_json::to_string(&source).unwrap();
        let deserialized: EnergySource = serde_json::from_str(&serialized).unwrap();
        assert_eq!(source, deserialized);
    }

    #[test]
    async fn test_process_energy_monitor_creation() {
        let monitor = ProcessEnergyMonitor::new();
        assert!(monitor.enable_rapl);
        assert!(monitor.enable_ebpf);
        
        let monitor_custom = ProcessEnergyMonitor::with_config(false, false);
        assert!(!monitor_custom.enable_rapl);
        assert!(!monitor_custom.enable_ebpf);
    }

    #[test]
    async fn test_enhance_process_record() {
        let monitor = ProcessEnergyMonitor::new();
        let mut record = ProcessRecord::default();
        record.pid = 123;
        
        let stats = ProcessEnergyStats {
            pid: 123,
            energy_uj: 1000,
            power_w: 1.5,
            timestamp: 1234567890,
            source: EnergySource::ProcPower,
            is_reliable: true,
        };
        
        let enhanced = monitor.enhance_process_record(record, Some(stats));
        assert_eq!(enhanced.energy_uj, Some(1000));
        assert_eq!(enhanced.power_w, Some(1.5));
        assert_eq!(enhanced.energy_timestamp, Some(1234567890));
    }

    #[test]
    async fn test_batch_collection() {
        let monitor = ProcessEnergyMonitor::new();
        let pids = [1, 2, 3]; // Несуществующие PID для теста
        
        let results = monitor.collect_batch_energy(&pids).await.unwrap();
        // Должны получить пустой вектор, так как процессы не существуют
        assert!(results.is_empty());
    }

    #[test]
    async fn test_sync_wrapper() {
        let monitor = ProcessEnergyMonitor::new();
        // Тестируем синхронный wrapper
        let result = monitor.collect_process_energy_sync(999999);
        // Должны получить Ok(None) для несуществующего процесса
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    async fn test_energy_source_variants() {
        // Тестируем все варианты EnergySource
        let sources = vec![
            EnergySource::ProcPower,
            EnergySource::Rapl,
            EnergySource::Ebpf,
            EnergySource::None,
        ];
        
        for source in sources {
            let stats = ProcessEnergyStats {
                pid: 123,
                energy_uj: 1000,
                power_w: 1.0,
                timestamp: 1234567890,
                source,
                is_reliable: true,
            };
            
            // Проверяем, что статистика создается корректно
            assert_eq!(stats.pid, 123);
            assert_eq!(stats.energy_uj, 1000);
            assert_eq!(stats.power_w, 1.0);
            assert_eq!(stats.timestamp, 1234567890);
            assert!(stats.is_reliable);
        }
    }

    #[test]
    async fn test_monitor_configuration() {
        // Тестируем различные конфигурации монитора
        let monitor_all_enabled = ProcessEnergyMonitor::new();
        assert!(monitor_all_enabled.enable_rapl);
        assert!(monitor_all_enabled.enable_ebpf);
        
        let monitor_rapl_only = ProcessEnergyMonitor::with_config(true, false);
        assert!(monitor_rapl_only.enable_rapl);
        assert!(!monitor_rapl_only.enable_ebpf);
        
        let monitor_ebpf_only = ProcessEnergyMonitor::with_config(false, true);
        assert!(!monitor_ebpf_only.enable_rapl);
        assert!(monitor_ebpf_only.enable_ebpf);
        
        let monitor_all_disabled = ProcessEnergyMonitor::with_config(false, false);
        assert!(!monitor_all_disabled.enable_rapl);
        assert!(!monitor_all_disabled.enable_ebpf);
    }

    #[test]
    async fn test_enhance_process_record_with_none() {
        let monitor = ProcessEnergyMonitor::new();
        let mut record = ProcessRecord::default();
        record.pid = 123;
        
        // Тестируем с None статистикой
        let enhanced = monitor.enhance_process_record(record, None);
        assert_eq!(enhanced.energy_uj, None);
        assert_eq!(enhanced.power_w, None);
        assert_eq!(enhanced.energy_timestamp, None);
    }

    #[test]
    async fn test_serialization_deserialization() {
        let stats = ProcessEnergyStats {
            pid: 123,
            energy_uj: 1000,
            power_w: 1.5,
            timestamp: 1234567890,
            source: EnergySource::ProcPower,
            is_reliable: true,
        };
        
        // Тестируем сериализацию
        let serialized = serde_json::to_string(&stats).unwrap();
        let deserialized: ProcessEnergyStats = serde_json::from_str(&serialized).unwrap();
        
        // Проверяем, что десериализованные данные совпадают с оригиналом
        assert_eq!(stats.pid, deserialized.pid);
        assert_eq!(stats.energy_uj, deserialized.energy_uj);
        assert_eq!(stats.power_w, deserialized.power_w);
        assert_eq!(stats.timestamp, deserialized.timestamp);
        assert_eq!(stats.source, deserialized.source);
        assert_eq!(stats.is_reliable, deserialized.is_reliable);
    }
}
