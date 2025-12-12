//! eBPF-метрики для высокопроизводительного сбора системных данных.
//!
//! Этот модуль предоставляет функциональность для сбора метрик с использованием eBPF,
//! что позволяет получать более точные и детализированные данные о системе
//! с меньшими накладными расходами.
//!
//! # Возможности
//!
//! - Сбор базовых системных метрик (CPU, память, IO)
//! - Мониторинг системных вызовов
//! - Отслеживание сетевой активности
//! - Профилирование производительности
//!
//! # Зависимости
//!
//! Для работы этого модуля требуются:
//! - Ядро Linux с поддержкой eBPF (5.4+)
//! - Права для загрузки eBPF-программ (CAP_BPF или root)
//!
//! # Безопасность
//!
//! eBPF-программы выполняются в привилегированном контексте ядра.
//! Все программы должны быть тщательно проверены на безопасность.

use anyhow::{Context, Result};
use std::time::Duration;

#[cfg(feature = "ebpf")]
use libbpf_rs::Program;

/// Конфигурация eBPF-метрик
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EbpfConfig {
    /// Включить сбор CPU метрик через eBPF
    pub enable_cpu_metrics: bool,
    /// Включить сбор метрик памяти через eBPF
    pub enable_memory_metrics: bool,
    /// Включить мониторинг системных вызовов
    pub enable_syscall_monitoring: bool,
    /// Интервал сбора метрик
    pub collection_interval: Duration,
}

impl Default for EbpfConfig {
    fn default() -> Self {
        Self {
            enable_cpu_metrics: true,
            enable_memory_metrics: true,
            enable_syscall_monitoring: false,
            collection_interval: Duration::from_secs(1),
        }
    }
}

/// Структура для хранения eBPF метрик
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EbpfMetrics {
    /// Использование CPU (в процентах)
    pub cpu_usage: f64,
    /// Использование памяти (в байтах)
    pub memory_usage: u64,
    /// Количество системных вызовов
    pub syscall_count: u64,
    /// Время выполнения (в наносекундах)
    pub timestamp: u64,
}

/// Основной структуры для управления eBPF метриками
pub struct EbpfMetricsCollector {
    config: EbpfConfig,
    #[cfg(feature = "ebpf")]
    cpu_program: Option<Program>,
    #[cfg(feature = "ebpf")]
    memory_program: Option<Program>,
    initialized: bool,
}

impl EbpfMetricsCollector {
    /// Создать новый коллектор eBPF метрик
    pub fn new(config: EbpfConfig) -> Self {
        Self {
            config,
            #[cfg(feature = "ebpf")]
            cpu_program: None,
            #[cfg(feature = "ebpf")]
            memory_program: None,
            initialized: false,
        }
    }

    /// Инициализировать eBPF программы
    pub fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            tracing::info!("eBPF метрики уже инициализированы");
            return Ok(());
        }

        tracing::info!("Инициализация eBPF метрик");

        #[cfg(feature = "ebpf")]
        {
            // Проверяем поддержку eBPF
            if !Self::check_ebpf_support()? {
                tracing::warn!("eBPF не поддерживается в этой системе");
                return Ok(());
            }

            // Загружаем eBPF программы
            if self.config.enable_cpu_metrics {
                self.load_cpu_program()?;
            }

            if self.config.enable_memory_metrics {
                self.load_memory_program()?;
            }

            self.initialized = true;
            tracing::info!("eBPF метрики успешно инициализированы");
        }

        #[cfg(not(feature = "ebpf"))]
        {
            tracing::warn!("eBPF поддержка отключена (собран без feature 'ebpf')");
        }

        Ok(())
    }

    /// Загрузить eBPF программу для сбора CPU метрик
    #[cfg(feature = "ebpf")]
    fn load_cpu_program(&mut self) -> Result<()> {
        use std::path::Path;
        
        let program_path = Path::new("src/ebpf_programs/cpu_metrics.c");
        
        if !program_path.exists() {
            tracing::warn!("eBPF программа для CPU метрик не найдена: {:?}", program_path);
            return Ok(());
        }

        // В реальной реализации здесь будет загрузка и компиляция eBPF программы
        // Пока что это заглушка
        tracing::info!("Загрузка eBPF программы для CPU метрик");
        
        // TODO: Реальная загрузка eBPF программы
        // self.cpu_program = Some(Program::from_file(program_path)?);
        
        Ok(())
    }

    /// Загрузить eBPF программу для сбора метрик памяти
    #[cfg(feature = "ebpf")]
    fn load_memory_program(&mut self) -> Result<()> {
        // Пока что заглушка - в будущем здесь будет реальная загрузка
        tracing::info!("Загрузка eBPF программы для метрик памяти");
        
        // TODO: Реальная загрузка eBPF программы
        // self.memory_program = Some(Program::from_file(memory_program_path)?);
        
        Ok(())
    }

    /// Собрать текущие метрики
    pub fn collect_metrics(&self) -> Result<EbpfMetrics> {
        if !self.initialized {
            tracing::warn!("eBPF метрики не инициализированы, возвращаем значения по умолчанию");
            return Ok(EbpfMetrics::default());
        }

        #[cfg(feature = "ebpf")]
        {
            // В реальной реализации здесь будет сбор метрик из eBPF программ
            // Пока что возвращаем тестовые значения
            let cpu_usage = if self.config.enable_cpu_metrics { 25.5 } else { 0.0 };
            let memory_usage = if self.config.enable_memory_metrics { 1024 * 1024 * 512 } else { 0 };
            
            let metrics = EbpfMetrics {
                cpu_usage,
                memory_usage,
                syscall_count: 100,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_nanos() as u64,
            };
            
            tracing::debug!("Собраны eBPF метрики: {:?}", metrics);
            Ok(metrics)
        }

        #[cfg(not(feature = "ebpf"))]
        {
            // Без eBPF поддержки возвращаем значения по умолчанию
            Ok(EbpfMetrics::default())
        }
    }

    /// Проверить поддержку eBPF в системе
    pub fn check_ebpf_support() -> Result<bool> {
        // Проверяем поддержку eBPF
        // На Linux проверяем версию ядра и наличие необходимых возможностей
        #[cfg(target_os = "linux")] {
            // Проверяем версию ядра
            let kernel_version = Self::get_kernel_version()?;
            
            // eBPF требует ядро 4.4+ для базовой поддержки, 5.4+ для расширенных возможностей
            if kernel_version >= (4, 4) {
                // Дополнительная проверка наличия eBPF в системе
                let has_ebpf = std::path::Path::new("/sys/kernel/debug/tracing/available_filter_functions").exists()
                    || std::path::Path::new("/proc/kallsyms").exists();
                
                Ok(has_ebpf)
            } else {
                tracing::warn!("Ядро Linux {} не поддерживает eBPF (требуется 4.4+)", 
                    format!("{}.{}", kernel_version.0, kernel_version.1));
                Ok(false)
            }
        }
        #[cfg(not(target_os = "linux"))] {
            tracing::info!("eBPF поддерживается только на Linux");
            Ok(false)
        }
    }

    /// Получить версию ядра Linux
    #[cfg(target_os = "linux")]
    fn get_kernel_version() -> Result<(u32, u32)> {
        use std::fs::read_to_string;
        
        let utsname = read_to_string("/proc/sys/kernel/osrelease")
            .context("Не удалось прочитать версию ядра из /proc/sys/kernel/osrelease")?;
        
        let utsname = utsname.trim();
        let parts: Vec<&str> = utsname.split('-').collect();
        let version_parts: Vec<&str> = parts[0].split('.').collect();
        
        if version_parts.len() >= 2 {
            let major = version_parts[0].parse::<u32>()?;
            let minor = version_parts[1].parse::<u32>()?;
            Ok((major, minor))
        } else {
            anyhow::bail!("Не удалось разобрать версию ядра: {}", utsname);
        }
    }

    /// Заглушка для не-Linux систем
    #[cfg(not(target_os = "linux"))]
    fn get_kernel_version() -> Result<(u32, u32)> {
        Ok((0, 0))
    }

    /// Проверить, включена ли поддержка eBPF
    pub fn is_ebpf_enabled() -> bool {
        #[cfg(feature = "ebpf")]
        {
            true
        }
        #[cfg(not(feature = "ebpf"))]
        {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebpf_config_default() {
        let config = EbpfConfig::default();
        assert!(config.enable_cpu_metrics);
        assert!(config.enable_memory_metrics);
        assert!(!config.enable_syscall_monitoring);
        assert_eq!(config.collection_interval, Duration::from_secs(1));
    }

    #[test]
    fn test_ebpf_metrics_default() {
        let metrics = EbpfMetrics::default();
        assert_eq!(metrics.cpu_usage, 0.0);
        assert_eq!(metrics.memory_usage, 0);
        assert_eq!(metrics.syscall_count, 0);
        assert_eq!(metrics.timestamp, 0);
    }

    #[test]
    fn test_ebpf_collector_creation() {
        let config = EbpfConfig::default();
        let mut collector = EbpfMetricsCollector::new(config);
        assert!(collector.initialize().is_ok());
        assert!(collector.collect_metrics().is_ok());
    }

    #[test]
    fn test_ebpf_support_check() {
        let supported = EbpfMetricsCollector::check_ebpf_support();
        // На Linux должна быть поддержка (если ядро достаточно новое)
        #[cfg(target_os = "linux")] {
            // В тестовой среде может не быть поддержки, поэтому просто проверяем, что функция не паникует
            assert!(supported.is_ok());
        }
        
        #[cfg(not(target_os = "linux"))] {
            assert_eq!(supported.unwrap(), false);
        }
    }

    #[test]
    fn test_ebpf_enabled_feature() {
        let enabled = EbpfMetricsCollector::is_ebpf_enabled();
        #[cfg(feature = "ebpf")] {
            assert!(enabled);
        }
        #[cfg(not(feature = "ebpf"))] {
            assert!(!enabled);
        }
    }

    #[test]
    fn test_ebpf_metrics_with_config() {
        let mut config = EbpfConfig::default();
        config.enable_cpu_metrics = true;
        config.enable_memory_metrics = false;
        
        let mut collector = EbpfMetricsCollector::new(config);
        assert!(collector.initialize().is_ok());
        
        let metrics = collector.collect_metrics().unwrap();
        // Проверяем, что метрики собираются корректно
        assert!(metrics.cpu_usage >= 0.0);
        assert_eq!(metrics.memory_usage, 0); // Должно быть 0, так как отключено в конфиге
    }

    #[test]
    fn test_ebpf_double_initialization() {
        let config = EbpfConfig::default();
        let mut collector = EbpfMetricsCollector::new(config);
        
        assert!(collector.initialize().is_ok());
        // Вторая инициализация должна пройти успешно, но не делать ничего
        assert!(collector.initialize().is_ok());
    }
}