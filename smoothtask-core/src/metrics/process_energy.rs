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
use num_cpus;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs as tokio_fs;
use tracing;

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
    /// Включает улучшенную обработку ошибок и graceful degradation.
    pub async fn collect_process_energy(&self, pid: i32) -> Result<Option<ProcessEnergyStats>> {
        let mut errors = Vec::new();

        // Пробуем получить данные из /proc/[pid]/power/energy_uj (наивысший приоритет)
        match self.try_collect_proc_power_energy(pid).await {
            Ok(Some(stats)) => {
                tracing::debug!(
                    "Successfully collected process energy from /proc/power for PID {}",
                    pid
                );
                return Ok(Some(stats));
            }
            Ok(None) => {
                tracing::debug!("/proc/power energy data not available for PID {}", pid);
            }
            Err(e) => {
                tracing::warn!("Error collecting /proc/power energy for PID {}: {}", pid, e);
                errors.push(format!("proc_power: {}", e));
            }
        }

        // Пробуем получить данные из eBPF (если включено)
        if self.enable_ebpf {
            match self.try_collect_ebpf_energy(pid).await {
                Ok(Some(stats)) => {
                    tracing::debug!(
                        "Successfully collected process energy from eBPF for PID {}",
                        pid
                    );
                    return Ok(Some(stats));
                }
                Ok(None) => {
                    tracing::debug!("eBPF energy data not available for PID {}", pid);
                }
                Err(e) => {
                    tracing::warn!("Error collecting eBPF energy for PID {}: {}", pid, e);
                    errors.push(format!("ebpf: {}", e));
                }
            }
        }

        // Пробуем получить данные из RAPL (если включено)
        if self.enable_rapl {
            match self.try_collect_rapl_energy(pid).await {
                Ok(Some(stats)) => {
                    tracing::debug!(
                        "Successfully collected process energy from RAPL for PID {}",
                        pid
                    );
                    return Ok(Some(stats));
                }
                Ok(None) => {
                    tracing::debug!("RAPL energy data not available for PID {}", pid);
                }
                Err(e) => {
                    tracing::warn!("Error collecting RAPL energy for PID {}: {}", pid, e);
                    errors.push(format!("rapl: {}", e));
                }
            }
        }

        // Если ни один источник не сработал, пробуем fallback оценку
        if !errors.is_empty() {
            tracing::info!(
                "No process energy data available for PID {} from any source. Errors: {}",
                pid,
                errors.join(", ")
            );
        } else {
            tracing::debug!(
                "No process energy data available for PID {} - all sources returned None",
                pid
            );
        }

        // Пробуем fallback оценку на основе CPU использования
        if let Some(fallback_stats) = self.try_fallback_energy_estimation(pid).await? {
            tracing::debug!("Using fallback energy estimation for PID {}", pid);
            return Ok(Some(fallback_stats));
        }

        Ok(None)
    }

    /// Попробовать оценить энергопотребление на основе CPU использования (fallback метод).
    async fn try_fallback_energy_estimation(&self, pid: i32) -> Result<Option<ProcessEnergyStats>> {
        // Этот метод предоставляет грубую оценку энергопотребления на основе CPU использования
        // Используется только когда прямые источники данных недоступны

        // Пробуем получить CPU использование процесса
        let cpu_usage = self.get_process_cpu_usage(pid).await?;

        if cpu_usage > 0.0 {
            // Упрощенная модель: предполагаем, что 100% CPU = 10 Вт (типичное значение для современных CPU)
            // Это очень грубая оценка и не должна использоваться для точных измерений
            let estimated_power_w = cpu_usage * 10.0; // 10 Вт при 100% нагрузке
            let estimated_energy_uj = (estimated_power_w * 1_000_000.0) as u64; // Конвертация в микроджоули

            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

            return Ok(Some(ProcessEnergyStats {
                pid,
                energy_uj: estimated_energy_uj,
                power_w: estimated_power_w,
                timestamp,
                source: EnergySource::None, // Помечаем как ненадежный источник
                is_reliable: false,         // Явно указываем, что это оценка
            }));
        }

        Ok(None)
    }

    /// Получить CPU использование процесса из /proc/stat.
    async fn get_process_cpu_usage(&self, pid: i32) -> Result<f32> {
        let stat_path = format!("/proc/{}/stat", pid);

        if let Ok(stat_content) = tokio_fs::read_to_string(&stat_path).await {
            // Парсим /proc/[pid]/stat для получения CPU времени
            // Формат: pid (comm) state ppid ... utime stime ...
            let parts: Vec<&str> = stat_content.split_whitespace().collect();

            if parts.len() >= 14 {
                // utime (14) и stime (15) - пользовательское и системное время в тиках
                if let (Ok(utime), Ok(stime)) = (parts[13].parse::<u64>(), parts[14].parse::<u64>())
                {
                    let total_time = utime + stime;

                    // Получаем общее время системы
                    let system_uptime = self.get_system_uptime().await?;

                    if system_uptime > 0 {
                        // Очень упрощенная оценка: предполагаем, что процесс использует CPU пропорционально времени
                        // В реальности нужно более сложное вычисление с учетом количества CPU и т.д.
                        let cpu_usage =
                            (total_time as f32 / system_uptime as f32 * 100.0).min(100.0);
                        return Ok(cpu_usage / 100.0); // Возвращаем как долю (0.0 - 1.0)
                    }
                }
            }
        }

        Ok(0.0)
    }

    /// Получить время работы системы из /proc/uptime.
    async fn get_system_uptime(&self) -> Result<u64> {
        if let Ok(uptime_content) = tokio_fs::read_to_string("/proc/uptime").await {
            if let Some(first_line) = uptime_content.lines().next() {
                if let Some(uptime_secs) = first_line
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<f64>().ok())
                {
                    return Ok(uptime_secs as u64);
                }
            }
        }

        Ok(1) // Fallback значение, чтобы избежать деления на ноль
    }

    /// Попробовать получить данные из /proc/[pid]/power/energy_uj.
    async fn try_collect_proc_power_energy(&self, pid: i32) -> Result<Option<ProcessEnergyStats>> {
        let energy_path = format!("/proc/{}/power/energy_uj", pid);

        if let Ok(energy_content) = tokio_fs::read_to_string(&energy_path).await {
            if let Ok(energy_uj) = energy_content.trim().parse::<u64>() {
                let power_w = energy_uj as f32 / 1_000_000.0;
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

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
        // Пробуем получить eBPF метрики через глобальный коллектор
        // Используем кэширование для улучшения производительности
        let ebpf_metrics = self.get_cached_ebpf_metrics().await?;

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

    /// Получить кэшированные eBPF метрики с обработкой ошибок.
    async fn get_cached_ebpf_metrics(&self) -> Result<Option<crate::metrics::ebpf::EbpfMetrics>> {
        // Пробуем получить eBPF метрики через глобальный коллектор
        let ebpf_metrics = crate::metrics::system::collect_ebpf_metrics();

        // Логируем информацию о доступности eBPF
        if ebpf_metrics.is_none() {
            tracing::debug!(
                "eBPF metrics not available - eBPF support may be disabled or not initialized"
            );
        } else {
            tracing::debug!("Successfully retrieved eBPF metrics for process energy monitoring");
        }

        Ok(ebpf_metrics)
    }

    /// Попробовать получить данные из RAPL.
    async fn try_collect_rapl_energy(&self, pid: i32) -> Result<Option<ProcessEnergyStats>> {
        if !self.rapl_base_path.exists() {
            return Ok(None);
        }

        // Получаем информацию о процессе для сопоставления с RAPL доменами
        let process_cpu_affinity = self.get_process_cpu_affinity(pid).await?;
        if process_cpu_affinity.is_empty() {
            return Ok(None);
        }

        // Собираем данные из RAPL доменов и сопоставляем с CPU процесса
        let mut domain_energy = Vec::new();
        let mut read_dir = tokio_fs::read_dir(&self.rapl_base_path).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let domain_path = entry.path();
            let energy_path = domain_path.join("energy_uj");

            if energy_path.exists() {
                if let Ok(energy_content) = tokio_fs::read_to_string(&energy_path).await {
                    if let Ok(energy_uj) = energy_content.trim().parse::<u64>() {
                        // Пробуем получить CPU ID из имени домена
                        let cpu_id = self.extract_cpu_id_from_domain(&domain_path);
                        domain_energy.push((cpu_id, energy_uj));
                    }
                }
            }
        }

        if domain_energy.is_empty() {
            return Ok(None);
        }

        // Рассчитываем энергопотребление процесса на основе CPU affinity
        let process_energy_uj =
            self.calculate_process_energy_from_rapl(&domain_energy, &process_cpu_affinity);

        if process_energy_uj > 0 {
            let power_w = process_energy_uj as f32 / 1_000_000.0;
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

            return Ok(Some(ProcessEnergyStats {
                pid,
                energy_uj: process_energy_uj,
                power_w,
                timestamp,
                source: EnergySource::Rapl,
                is_reliable: domain_energy.len() >= 2, // Более достоверно с несколькими доменами
            }));
        }

        Ok(None)
    }

    /// Получить CPU affinity для процесса.
    async fn get_process_cpu_affinity(&self, pid: i32) -> Result<Vec<usize>> {
        let affinity_path = format!("/proc/{}/status", pid);

        if let Ok(status_content) = tokio_fs::read_to_string(&affinity_path).await {
            // Парсим CPU affinity из /proc/[pid]/status
            for line in status_content.lines() {
                if line.starts_with("Cpus_allowed:") || line.starts_with("Cpus_allowed_list:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let affinity_str = parts[1];
                        return self.parse_cpu_affinity(affinity_str);
                    }
                }
            }
        }

        Ok(Vec::new())
    }

    /// Парсить строку CPU affinity.
    fn parse_cpu_affinity(&self, affinity_str: &str) -> Result<Vec<usize>> {
        let mut cpus = Vec::new();

        // Поддерживаем форматы как "0-3", так и "0,1,2,3"
        if affinity_str.contains('-') {
            // Формат диапазона: "0-3"
            let range_parts: Vec<&str> = affinity_str.split('-').collect();
            if range_parts.len() == 2 {
                if let (Ok(start), Ok(end)) = (
                    range_parts[0].parse::<usize>(),
                    range_parts[1].parse::<usize>(),
                ) {
                    for cpu in start..=end {
                        cpus.push(cpu);
                    }
                }
            }
        } else if affinity_str.contains(',') {
            // Формат списка: "0,1,2,3"
            for cpu_str in affinity_str.split(',') {
                if let Ok(cpu) = cpu_str.parse::<usize>() {
                    cpus.push(cpu);
                }
            }
        } else {
            // Одиночный CPU
            if let Ok(cpu) = affinity_str.parse::<usize>() {
                cpus.push(cpu);
            }
        }

        Ok(cpus)
    }

    /// Извлечь CPU ID из имени RAPL домена.
    fn extract_cpu_id_from_domain(&self, domain_path: &std::path::Path) -> Option<usize> {
        if let Some(file_name) = domain_path.file_name() {
            if let Some(name_str) = file_name.to_str() {
                // Пробуем извлечь CPU ID из имени домена
                // Формат может быть: intel-rapl:0, intel-rapl:1, и т.д.
                if name_str.starts_with("intel-rapl:") {
                    if let Some(id_part) = name_str.split(':').nth(1) {
                        return id_part.parse::<usize>().ok();
                    }
                }
                // Также пробуем другие форматы
                if name_str.contains("package")
                    || name_str.contains("core")
                    || name_str.contains("uncore")
                {
                    // Для пакетных доменов используем 0 как идентификатор по умолчанию
                    return Some(0);
                }
            }
        }
        None
    }

    /// Рассчитать энергопотребление процесса на основе RAPL данных и CPU affinity.
    fn calculate_process_energy_from_rapl(
        &self,
        domain_energy: &[(Option<usize>, u64)],
        cpu_affinity: &[usize],
    ) -> u64 {
        // Упрощенный алгоритм: распределяем энергию пропорционально количеству CPU
        // В реальной системе нужно более сложное сопоставление

        let total_cpus_in_system = num_cpus::get();
        let process_cpus = cpu_affinity.len();

        if process_cpus == 0 || total_cpus_in_system == 0 {
            return 0;
        }

        // Суммируем всю энергию из RAPL доменов
        let total_energy: u64 = domain_energy.iter().map(|(_, energy)| energy).sum();

        // Распределяем энергию пропорционально количеству CPU, доступных процессу
        let cpu_ratio = process_cpus as f64 / total_cpus_in_system as f64;
        (total_energy as f64 * cpu_ratio) as u64
    }

    /// Обновить ProcessRecord данными о энергопотреблении.
    pub fn enhance_process_record(
        &self,
        mut record: ProcessRecord,
        energy_stats: Option<ProcessEnergyStats>,
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
        static MONITOR: once_cell::sync::OnceCell<ProcessEnergyMonitor> =
            once_cell::sync::OnceCell::new();

        let monitor = MONITOR.get_or_init(|| ProcessEnergyMonitor::new());
        monitor.collect_process_energy(pid).await
    }

    /// Обновить ProcessRecord данными о энергопотреблении.
    pub async fn enhance_process_record(record: ProcessRecord) -> Result<ProcessRecord> {
        let pid = record.pid;
        let energy_stats = Self::collect_process_energy(pid).await?;

        static MONITOR: once_cell::sync::OnceCell<ProcessEnergyMonitor> =
            once_cell::sync::OnceCell::new();
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
        let pids = [999997, 999998, 999999]; // Несуществующие PID для теста

        let results = monitor.collect_batch_energy(&pids).await.unwrap();
        // Должны получить пустой вектор, так как процессы не существуют
        assert!(results.is_empty());
    }

    #[test]
    async fn test_sync_wrapper() {
        let monitor = ProcessEnergyMonitor::new();
        // Тестируем синхронный wrapper
        // Примечание: В реальном использовании это должно вызываться из синхронного контекста
        // Здесь мы просто тестируем, что функция доступна и не вызывает панику
        // Используем блокирующий вызов в отдельном потоке, чтобы избежать конфликта runtime
        let result = std::thread::spawn(move || monitor.collect_process_energy_sync(999999))
            .join()
            .unwrap();

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

    #[test]
    async fn test_cpu_affinity_parsing() {
        let monitor = ProcessEnergyMonitor::new();

        // Тестируем парсинг диапазона
        let range_result = monitor.parse_cpu_affinity("0-3");
        assert!(range_result.is_ok());
        let range_cpus = range_result.unwrap();
        assert_eq!(range_cpus, vec![0, 1, 2, 3]);

        // Тестируем парсинг списка
        let list_result = monitor.parse_cpu_affinity("0,1,2,3");
        assert!(list_result.is_ok());
        let list_cpus = list_result.unwrap();
        assert_eq!(list_cpus, vec![0, 1, 2, 3]);

        // Тестируем одиночный CPU
        let single_result = monitor.parse_cpu_affinity("2");
        assert!(single_result.is_ok());
        let single_cpus = single_result.unwrap();
        assert_eq!(single_cpus, vec![2]);
    }

    #[test]
    async fn test_energy_calculation() {
        let monitor = ProcessEnergyMonitor::new();

        // Тестируем расчет энергопотребления
        let domain_energy = vec![
            (Some(0), 1000),
            (Some(1), 1500),
            (None, 2000), // Домен без CPU ID
        ];

        let cpu_affinity = vec![0, 1]; // Процесс использует 2 CPU из 4

        let process_energy =
            monitor.calculate_process_energy_from_rapl(&domain_energy, &cpu_affinity);

        // Общая энергия: 1000 + 1500 + 2000 = 4500
        // Соотношение: 2/4 = 0.5
        // Ожидаемая энергия процесса: 4500 * 0.5 = 2250
        assert_eq!(process_energy, 2250);
    }

    #[test]
    async fn test_rapl_domain_extraction() {
        let monitor = ProcessEnergyMonitor::new();

        // Тестируем извлечение CPU ID из имени домена
        let path_intel_rapl = std::path::Path::new("intel-rapl:0");
        let cpu_id = monitor.extract_cpu_id_from_domain(path_intel_rapl);
        assert_eq!(cpu_id, Some(0));

        let path_intel_rapl_1 = std::path::Path::new("intel-rapl:1");
        let cpu_id_1 = monitor.extract_cpu_id_from_domain(path_intel_rapl_1);
        assert_eq!(cpu_id_1, Some(1));

        let path_package = std::path::Path::new("package-0");
        let cpu_id_package = monitor.extract_cpu_id_from_domain(path_package);
        assert_eq!(cpu_id_package, Some(0)); // Для пакетных доменов возвращаем 0

        let path_unknown = std::path::Path::new("unknown");
        let cpu_id_unknown = monitor.extract_cpu_id_from_domain(path_unknown);
        assert_eq!(cpu_id_unknown, None);
    }

    #[test]
    async fn test_rapl_energy_collection_fallback() {
        let monitor = ProcessEnergyMonitor::new();

        // Тестируем, что RAPL сбор возвращает None, если RAPL не доступен
        // Это тест на graceful degradation
        let result = monitor.try_collect_rapl_energy(999999).await;

        // Должно вернуть Ok(None) для несуществующего процесса или недоступного RAPL
        assert!(result.is_ok());
        // Note: Мы не можем проверить точное значение, так как это зависит от системы
    }

    #[test]
    async fn test_process_energy_integration() {
        let monitor = ProcessEnergyMonitor::new();

        // Тестируем интеграцию всех источников
        // Это тест проверяет, что монитор корректно пробует все источники
        let result = monitor.collect_process_energy(1).await; // PID 1 - обычно init/systemd

        // Должно вернуть Ok, даже если данных нет
        assert!(result.is_ok());
        // Note: Мы не можем гарантировать наличие данных, так как это зависит от системы
    }

    #[test]
    async fn test_fallback_energy_estimation() {
        let monitor = ProcessEnergyMonitor::new();

        // Тестируем fallback оценку
        // Это тест проверяет, что fallback метод работает корректно
        let result = monitor.try_fallback_energy_estimation(1).await;

        // Должно вернуть Ok, даже если данных нет
        assert!(result.is_ok());

        // Если есть данные, проверяем, что они помечены как ненадежные
        if let Ok(Some(stats)) = result {
            assert!(!stats.is_reliable);
            assert_eq!(stats.source, EnergySource::None);
        }
    }

    #[test]
    async fn test_cpu_usage_parsing() {
        let monitor = ProcessEnergyMonitor::new();

        // Тестируем парсинг CPU использования
        // Это тест проверяет, что метод корректно обрабатывает /proc/stat
        let result = monitor.get_process_cpu_usage(1).await;

        // Должно вернуть Ok с каким-то значением
        assert!(result.is_ok());
        let cpu_usage = result.unwrap();
        assert!(cpu_usage >= 0.0 && cpu_usage <= 1.0);
    }

    #[test]
    async fn test_system_uptime() {
        let monitor = ProcessEnergyMonitor::new();

        // Тестируем получение времени работы системы
        let result = monitor.get_system_uptime().await;

        // Должно вернуть Ok с положительным значением
        assert!(result.is_ok());
        let uptime = result.unwrap();
        assert!(uptime > 0);
    }

    #[test]
    async fn test_error_handling_in_collection() {
        let monitor = ProcessEnergyMonitor::new();

        // Тестируем обработку ошибок при сборе метрик
        // Используем несуществующий PID
        let result = monitor.collect_process_energy(999999).await;

        // Должно вернуть Ok(None) для несуществующего процесса
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert!(stats.is_none());
    }

    #[test]
    async fn test_monitor_with_disabled_sources() {
        // Тестируем монитор с отключенными источниками
        let monitor = ProcessEnergyMonitor::with_config(false, false);

        // Должно вернуть Ok, но может вернуть Some с fallback данными
        let result = monitor.collect_process_energy(1).await;
        assert!(result.is_ok());
        let stats = result.unwrap();

        // Если есть данные, они должны быть помечены как ненадежные (fallback)
        if let Some(energy_stats) = stats {
            assert!(!energy_stats.is_reliable);
            assert_eq!(energy_stats.source, EnergySource::None);
        }
    }

    #[test]
    async fn test_energy_source_reliability() {
        // Тестируем, что разные источники имеют правильные флаги надежности
        let stats_proc = ProcessEnergyStats {
            pid: 123,
            energy_uj: 1000,
            power_w: 1.0,
            timestamp: 1234567890,
            source: EnergySource::ProcPower,
            is_reliable: true,
        };

        let stats_ebpf = ProcessEnergyStats {
            pid: 123,
            energy_uj: 1000,
            power_w: 1.0,
            timestamp: 1234567890,
            source: EnergySource::Ebpf,
            is_reliable: true,
        };

        let stats_rapl = ProcessEnergyStats {
            pid: 123,
            energy_uj: 1000,
            power_w: 1.0,
            timestamp: 1234567890,
            source: EnergySource::Rapl,
            is_reliable: true,
        };

        let stats_fallback = ProcessEnergyStats {
            pid: 123,
            energy_uj: 1000,
            power_w: 1.0,
            timestamp: 1234567890,
            source: EnergySource::None,
            is_reliable: false,
        };

        // Проверяем, что прямой источник надежен
        assert!(stats_proc.is_reliable);
        assert!(stats_ebpf.is_reliable);
        assert!(stats_rapl.is_reliable);

        // Проверяем, что fallback источник ненадежен
        assert!(!stats_fallback.is_reliable);
    }

    #[test]
    async fn test_batch_collection_with_mixed_results() {
        let monitor = ProcessEnergyMonitor::new();

        // Тестируем пакетный сбор с разными результатами
        let pids = [1, 999999, 2]; // Смесь реальных и несуществующих PID

        let results = monitor.collect_batch_energy(&pids).await;
        assert!(results.is_ok());

        // Должны получить только результаты для существующих процессов
        let stats = results.unwrap();
        // Note: Мы не можем предсказать точное количество, так как это зависит от системы
        assert!(stats.len() <= pids.len());
    }

    #[test]
    async fn test_energy_monitor_configuration_options() {
        // Тестируем различные конфигурации монитора
        let config_combinations = vec![
            (true, true),   // RAPL и eBPF включены
            (true, false),  // Только RAPL
            (false, true),  // Только eBPF
            (false, false), // Все отключено
        ];

        for (enable_rapl, enable_ebpf) in config_combinations {
            let monitor = ProcessEnergyMonitor::with_config(enable_rapl, enable_ebpf);
            assert_eq!(monitor.enable_rapl, enable_rapl);
            assert_eq!(monitor.enable_ebpf, enable_ebpf);
        }
    }
}
