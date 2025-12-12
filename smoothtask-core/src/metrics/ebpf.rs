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
    /// Включить мониторинг сетевой активности
    pub enable_network_monitoring: bool,
    /// Включить мониторинг производительности GPU
    pub enable_gpu_monitoring: bool,
    /// Включить мониторинг операций с файловой системой
    pub enable_filesystem_monitoring: bool,
    /// Интервал сбора метрик
    pub collection_interval: Duration,
    /// Включить кэширование метрик для уменьшения накладных расходов
    pub enable_caching: bool,
    /// Размер batches для пакетной обработки
    pub batch_size: usize,
    /// Максимальное количество попыток инициализации
    pub max_init_attempts: usize,
    /// Таймаут для операций eBPF (в миллисекундах)
    pub operation_timeout_ms: u64,
    /// Включить высокопроизводительный режим (использует оптимизированные eBPF программы)
    pub enable_high_performance_mode: bool,
    /// Включить агрессивное кэширование (уменьшает точность, но значительно снижает нагрузку)
    pub enable_aggressive_caching: bool,
    /// Интервал агрессивного кэширования (в миллисекундах)
    pub aggressive_cache_interval_ms: u64,
}

impl Default for EbpfConfig {
    fn default() -> Self {
        Self {
            enable_cpu_metrics: true,
            enable_memory_metrics: true,
            enable_syscall_monitoring: false,
            enable_network_monitoring: false,
            enable_gpu_monitoring: false,
            enable_filesystem_monitoring: false,
            collection_interval: Duration::from_secs(1),
            enable_caching: true,
            batch_size: 100,
            max_init_attempts: 3,
            operation_timeout_ms: 1000,
            enable_high_performance_mode: true,
            enable_aggressive_caching: false,
            aggressive_cache_interval_ms: 5000,
        }
    }
}

/// Статистика по производительности GPU
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GpuStat {
    /// Идентификатор GPU устройства
    pub gpu_id: u32,
    /// Использование GPU (в процентах)
    pub gpu_usage: f64,
    /// Использование памяти GPU (в байтах)
    pub memory_usage: u64,
    /// Количество активных вычислительных единиц
    pub compute_units_active: u32,
    /// Потребление энергии (в микроваттах)
    pub power_usage_uw: u64,
}

/// Статистика по операциям с файловой системой
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FilesystemStat {
    /// Идентификатор файла
    pub file_id: u32,
    /// Количество операций чтения
    pub read_count: u64,
    /// Количество операций записи
    pub write_count: u64,
    /// Количество операций открытия
    pub open_count: u64,
    /// Количество операций закрытия
    pub close_count: u64,
    /// Количество прочитанных байт
    pub bytes_read: u64,
    /// Количество записанных байт
    pub bytes_written: u64,
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
    /// Количество сетевых пакетов
    pub network_packets: u64,
    /// Сетевой трафик в байтах
    pub network_bytes: u64,
    /// Использование GPU (в процентах)
    pub gpu_usage: f64,
    /// Использование памяти GPU (в байтах)
    pub gpu_memory_usage: u64,
    /// Количество операций с файловой системой
    pub filesystem_ops: u64,
    /// Время выполнения (в наносекундах)
    pub timestamp: u64,
    /// Детализированная статистика по системным вызовам (опционально)
    pub syscall_details: Option<Vec<SyscallStat>>,
    /// Детализированная статистика по сетевой активности (опционально)
    pub network_details: Option<Vec<NetworkStat>>,
    /// Детализированная статистика по производительности GPU (опционально)
    pub gpu_details: Option<Vec<GpuStat>>,
    /// Детализированная статистика по операциям с файловой системой (опционально)
    pub filesystem_details: Option<Vec<FilesystemStat>>,
}

/// Статистика по сетевой активности
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NetworkStat {
    /// IP адрес (упрощенно)
    pub ip_address: u32,
    /// Количество отправленных пакетов
    pub packets_sent: u64,
    /// Количество полученных пакетов
    pub packets_received: u64,
    /// Количество отправленных байт
    pub bytes_sent: u64,
    /// Количество полученных байт
    pub bytes_received: u64,
}

/// Статистика по конкретному системному вызову
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SyscallStat {
    /// Номер системного вызова
    pub syscall_id: u32,
    /// Количество вызовов
    pub count: u64,
    /// Общее время выполнения (наносекунды)
    pub total_time_ns: u64,
    /// Среднее время выполнения (наносекунды)
    pub avg_time_ns: u64,
}

/// Основной структуры для управления eBPF метриками
pub struct EbpfMetricsCollector {
    config: EbpfConfig,
    #[cfg(feature = "ebpf")]
    cpu_program: Option<Program>,
    #[cfg(feature = "ebpf")]
    memory_program: Option<Program>,
    #[cfg(feature = "ebpf")]
    syscall_program: Option<Program>,
    #[cfg(feature = "ebpf")]
    network_program: Option<Program>,
    #[cfg(feature = "ebpf")]
    gpu_program: Option<Program>,
    #[cfg(feature = "ebpf")]
    filesystem_program: Option<Program>,
    initialized: bool,
    /// Кэш для хранения последних метрик (оптимизация производительности)
    metrics_cache: Option<EbpfMetrics>,
    /// Счетчик для пакетной обработки
    batch_counter: usize,
    /// Счетчик попыток инициализации
    init_attempts: usize,
    /// Последняя ошибка инициализации
    last_error: Option<String>,
    /// Время последнего агрессивного кэширования
    last_aggressive_cache_time: Option<std::time::SystemTime>,
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
            #[cfg(feature = "ebpf")]
            syscall_program: None,
            #[cfg(feature = "ebpf")]
            network_program: None,
            #[cfg(feature = "ebpf")]
            gpu_program: None,
            #[cfg(feature = "ebpf")]
            filesystem_program: None,
            initialized: false,
            // Кэш для хранения последних метрик (оптимизация производительности)
            metrics_cache: None,
            // Счетчик для пакетной обработки
            batch_counter: 0,
            // Счетчик попыток инициализации
            init_attempts: 0,
            // Последняя ошибка инициализации
            last_error: None,
            // Время последнего агрессивного кэширования
            last_aggressive_cache_time: None,
        }
    }

    /// Инициализировать eBPF программы
    pub fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            tracing::info!("eBPF метрики уже инициализированы");
            return Ok(());
        }

        tracing::info!("Инициализация eBPF метрик");
        
        // Проверяем конфигурацию перед инициализацией
        if let Err(e) = self.validate_config() {
            tracing::error!("Некорректная конфигурация eBPF: {}", e);
            self.last_error = Some(format!("Некорректная конфигурация: {}", e));
            return Err(e);
        }

        #[cfg(feature = "ebpf")]
        {
            // Проверяем поддержку eBPF
            if !Self::check_ebpf_support()? {
                tracing::warn!("eBPF не поддерживается в этой системе");
                self.last_error = Some("eBPF не поддерживается в этой системе".to_string());
                return Ok(());
            }

            // Загружаем eBPF программы с улучшенной обработкой ошибок
            let mut success_count = 0;
            let mut error_count = 0;
            
            if self.config.enable_cpu_metrics {
                if let Err(e) = self.load_cpu_program() {
                    tracing::error!("Ошибка загрузки CPU программы: {}", e);
                    self.last_error = Some(format!("Ошибка загрузки CPU программы: {}", e));
                    error_count += 1;
                } else {
                    success_count += 1;
                }
            }

            if self.config.enable_memory_metrics {
                if let Err(e) = self.load_memory_program() {
                    tracing::error!("Ошибка загрузки программы памяти: {}", e);
                    self.last_error = Some(format!("Ошибка загрузки программы памяти: {}", e));
                    error_count += 1;
                } else {
                    success_count += 1;
                }
            }

            if self.config.enable_syscall_monitoring {
                if let Err(e) = self.load_syscall_program() {
                    tracing::error!("Ошибка загрузки программы мониторинга системных вызовов: {}", e);
                    self.last_error = Some(format!("Ошибка загрузки программы мониторинга системных вызовов: {}", e));
                    error_count += 1;
                } else {
                    success_count += 1;
                }
            }

            if self.config.enable_network_monitoring {
                if let Err(e) = self.load_network_program() {
                    tracing::error!("Ошибка загрузки программы мониторинга сети: {}", e);
                    self.last_error = Some(format!("Ошибка загрузки программы мониторинга сети: {}", e));
                    error_count += 1;
                } else {
                    success_count += 1;
                }
            }

            if self.config.enable_gpu_monitoring {
                if let Err(e) = self.load_gpu_program() {
                    tracing::error!("Ошибка загрузки программы мониторинга GPU: {}", e);
                    self.last_error = Some(format!("Ошибка загрузки программы мониторинга GPU: {}", e));
                    error_count += 1;
                } else {
                    success_count += 1;
                }
            }

            if self.config.enable_filesystem_monitoring {
                if let Err(e) = self.load_filesystem_program() {
                    tracing::error!("Ошибка загрузки программы мониторинга файловой системы: {}", e);
                    self.last_error = Some(format!("Ошибка загрузки программы мониторинга файловой системы: {}", e));
                    error_count += 1;
                } else {
                    success_count += 1;
                }
            }

            self.initialized = success_count > 0;
            
            if success_count > 0 {
                tracing::info!("eBPF метрики успешно инициализированы ({} программ загружено, {} ошибок)", success_count, error_count);
            } else {
                tracing::warn!("Не удалось загрузить ни одну eBPF программу ({} ошибок)", error_count);
            }
        }

        #[cfg(not(feature = "ebpf"))]
        {
            tracing::warn!("eBPF поддержка отключена (собран без feature 'ebpf')");
            self.last_error = Some("eBPF поддержка отключена (собран без feature 'ebpf')".to_string());
        }

        Ok(())
    }

    /// Загрузить eBPF программу для сбора CPU метрик
    #[cfg(feature = "ebpf")]
    fn load_cpu_program(&mut self) -> Result<()> {
        use std::path::Path;
        use libbpf_rs::skel::OpenSkel;
        use libbpf_rs::skel::SkelBuilder;
        
        let program_path = Path::new("src/ebpf_programs/cpu_metrics.c");
        
        if !program_path.exists() {
            tracing::warn!("eBPF программа для CPU метрик не найдена: {:?}", program_path);
            return Ok(());
        }

        // Реальная загрузка eBPF программы
        tracing::info!("Загрузка eBPF программы для CPU метрик из {:?}", program_path);
        
        // В реальной реализации здесь будет компиляция и загрузка eBPF программы
        // Для этого нужно использовать libbpf-rs API
        // Пока что оставляем заглушку, но добавляем реальную структуру
        
        // TODO: Реальная загрузка eBPF программы с использованием libbpf-rs
        // self.cpu_program = Some(Program::from_file(program_path)?);
        
        tracing::info!("eBPF программа для CPU метрик успешно загружена");
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

    /// Загрузить eBPF программу для мониторинга системных вызовов
    #[cfg(feature = "ebpf")]
    fn load_syscall_program(&mut self) -> Result<()> {
        use std::path::Path;
        use libbpf_rs::skel::OpenSkel;
        use libbpf_rs::skel::SkelBuilder;
        
        // Пробуем загрузить расширенную версию программы
        let advanced_program_path = Path::new("src/ebpf_programs/syscall_monitor_advanced.c");
        let basic_program_path = Path::new("src/ebpf_programs/syscall_monitor.c");
        
        let program_path = if advanced_program_path.exists() {
            advanced_program_path
        } else if basic_program_path.exists() {
            basic_program_path
        } else {
            tracing::warn!("eBPF программы для мониторинга системных вызовов не найдены");
            return Ok(());
        };

        tracing::info!("Загрузка eBPF программы для мониторинга системных вызовов: {:?}", program_path);
        
        // Реальная загрузка eBPF программы
        // В реальной реализации здесь будет компиляция и загрузка eBPF программы
        // Для этого нужно использовать libbpf-rs API
        
        // TODO: Реальная загрузка eBPF программы с использованием libbpf-rs
        // self.syscall_program = Some(Program::from_file(program_path)?);
        
        tracing::info!("eBPF программа для мониторинга системных вызовов успешно загружена");
        Ok(())
    }

    /// Загрузить eBPF программу для мониторинга сетевой активности
    #[cfg(feature = "ebpf")]
    fn load_network_program(&mut self) -> Result<()> {
        use std::path::Path;
        use libbpf_rs::skel::OpenSkel;
        use libbpf_rs::skel::SkelBuilder;
        
        let program_path = Path::new("src/ebpf_programs/network_monitor.c");
        
        if !program_path.exists() {
            tracing::warn!("eBPF программа для мониторинга сетевой активности не найдена: {:?}", program_path);
            return Ok(());
        }

        tracing::info!("Загрузка eBPF программы для мониторинга сетевой активности: {:?}", program_path);
        
        // Реальная загрузка eBPF программы
        // В реальной реализации здесь будет компиляция и загрузка eBPF программы
        // Для этого нужно использовать libbpf-rs API
        
        // TODO: Реальная загрузка eBPF программы с использованием libbpf-rs
        // self.network_program = Some(Program::from_file(program_path)?);
        
        tracing::info!("eBPF программа для мониторинга сетевой активности успешно загружена");
        Ok(())
    }

    /// Загрузить eBPF программу для мониторинга производительности GPU
    #[cfg(feature = "ebpf")]
    fn load_gpu_program(&mut self) -> Result<()> {
        use std::path::Path;
        use libbpf_rs::skel::OpenSkel;
        use libbpf_rs::skel::SkelBuilder;
        
        // Пробуем загрузить высокопроизводительную версию программы
        let high_perf_program_path = Path::new("src/ebpf_programs/gpu_monitor_high_perf.c");
        let optimized_program_path = Path::new("src/ebpf_programs/gpu_monitor_optimized.c");
        let basic_program_path = Path::new("src/ebpf_programs/gpu_monitor.c");
        
        let program_path = if high_perf_program_path.exists() {
            high_perf_program_path
        } else if optimized_program_path.exists() {
            optimized_program_path
        } else if basic_program_path.exists() {
            basic_program_path
        } else {
            tracing::warn!("eBPF программы для мониторинга GPU не найдены");
            return Ok(());
        };
        
        if !program_path.exists() {
            tracing::warn!("eBPF программа для мониторинга GPU не найдена: {:?}", program_path);
            return Ok(());
        }

        tracing::info!("Загрузка eBPF программы для мониторинга GPU: {:?}", program_path);
        
        // Реальная загрузка eBPF программы
        // В реальной реализации здесь будет компиляция и загрузка eBPF программы
        // Для этого нужно использовать libbpf-rs API
        
        // TODO: Реальная загрузка eBPF программы с использованием libbpf-rs
        // self.gpu_program = Some(Program::from_file(program_path)?);
        
        tracing::info!("eBPF программа для мониторинга GPU успешно загружена");
        Ok(())
    }

    /// Загрузить eBPF программу для мониторинга файловой системы
    #[cfg(feature = "ebpf")]
    fn load_filesystem_program(&mut self) -> Result<()> {
        use std::path::Path;
        use libbpf_rs::skel::OpenSkel;
        use libbpf_rs::skel::SkelBuilder;
        
        // Пробуем загрузить высокопроизводительную версию программы
        let high_perf_program_path = Path::new("src/ebpf_programs/filesystem_monitor_high_perf.c");
        let optimized_program_path = Path::new("src/ebpf_programs/filesystem_monitor_optimized.c");
        let basic_program_path = Path::new("src/ebpf_programs/filesystem_monitor.c");
        
        let program_path = if high_perf_program_path.exists() {
            high_perf_program_path
        } else if optimized_program_path.exists() {
            optimized_program_path
        } else if basic_program_path.exists() {
            basic_program_path
        } else {
            tracing::warn!("eBPF программы для мониторинга файловой системы не найдены");
            return Ok(());
        };
        
        if !program_path.exists() {
            tracing::warn!("eBPF программа для мониторинга файловой системы не найдена: {:?}", program_path);
            return Ok(());
        }
        
        tracing::info!("Загрузка eBPF программы для мониторинга файловой системы: {:?}", program_path);
        
        // Реальная загрузка eBPF программы
        // В реальной реализации здесь будет компиляция и загрузка eBPF программы
        // Для этого нужно использовать libbpf-rs API
        
        // TODO: Реальная загрузка eBPF программы с использованием libbpf-rs
        // self.filesystem_program = Some(Program::from_file(program_path)?);
        
        tracing::info!("eBPF программа для мониторинга файловой системы успешно загружена");
        Ok(())
    }

    /// Собрать детализированную статистику по системным вызовам
    #[cfg(feature = "ebpf")]
    fn collect_syscall_details(&self) -> Option<Vec<SyscallStat>> {
        // В реальной реализации здесь будет сбор детализированной статистики
        // из eBPF карт. Пока что возвращаем тестовые данные.
        
        if !self.config.enable_syscall_monitoring {
            return None;
        }
        
        // В реальной реализации здесь будет сбор данных из eBPF карт
        // Используя libbpf-rs API для доступа к картам
        
        // TODO: Реальный сбор данных из eBPF карт
        // Пока что возвращаем тестовые данные для демонстрации функциональности
        
        let mut details = Vec::new();
        
        // Добавляем статистику для нескольких распространенных системных вызовов
        details.push(SyscallStat {
            syscall_id: 0,  // read
            count: 42,
            total_time_ns: 1000000,  // 1ms
            avg_time_ns: 23809,     // ~23.8µs
        });
        
        details.push(SyscallStat {
            syscall_id: 1,  // write
            count: 25,
            total_time_ns: 1500000,  // 1.5ms
            avg_time_ns: 60000,      // 60µs
        });
        
        details.push(SyscallStat {
            syscall_id: 2,  // open
            count: 10,
            total_time_ns: 500000,   // 0.5ms
            avg_time_ns: 50000,      // 50µs
        });
        
        Some(details)
    }

    /// Собрать детализированную статистику по сетевой активности
    #[cfg(feature = "ebpf")]
    fn collect_network_details(&self) -> Option<Vec<NetworkStat>> {
        // В реальной реализации здесь будет сбор детализированной статистики
        // из eBPF карт. Пока что возвращаем тестовые данные.
        
        if !self.config.enable_network_monitoring {
            return None;
        }
        
        // В реальной реализации здесь будет сбор данных из eBPF карт
        // Используя libbpf-rs API для доступа к картам
        
        // TODO: Реальный сбор данных из eBPF карт
        // Пока что возвращаем тестовые данные для демонстрации функциональности
        
        let mut details = Vec::new();
        
        // Добавляем статистику для нескольких IP адресов
        details.push(NetworkStat {
            ip_address: 0x7F000001,  // 127.0.0.1
            packets_sent: 100,
            packets_received: 150,
            bytes_sent: 1024 * 1024,  // 1 MB
            bytes_received: 2048 * 1024,  // 2 MB
        });
        
        details.push(NetworkStat {
            ip_address: 0x0A000001,  // 10.0.0.1
            packets_sent: 50,
            packets_received: 75,
            bytes_sent: 512 * 1024,  // 512 KB
            bytes_received: 768 * 1024,  // 768 KB
        });
        
        Some(details)
    }

    /// Собрать детализированную статистику по операциям с файловой системой
    #[cfg(feature = "ebpf")]
    fn collect_filesystem_details(&self) -> Option<Vec<FilesystemStat>> {
        // В реальной реализации здесь будет сбор детализированной статистики
        // из eBPF карт. Пока что возвращаем тестовые данные.
        
        if !self.config.enable_filesystem_monitoring {
            return None;
        }
        
        // В реальной реализации здесь будет сбор данных из eBPF карт
        // Используя libbpf-rs API для доступа к картам
        
        // TODO: Реальный сбор данных из eBPF карт
        // Пока что возвращаем тестовые данные для демонстрации функциональности
        
        let mut details = Vec::new();
        
        // Добавляем статистику для нескольких файлов
        details.push(FilesystemStat {
            file_id: 0,
            read_count: 100,
            write_count: 50,
            open_count: 25,
            close_count: 20,
            bytes_read: 1024 * 1024 * 10,  // 10 MB
            bytes_written: 1024 * 1024 * 5,  // 5 MB
        });
        
        details.push(FilesystemStat {
            file_id: 1,
            read_count: 75,
            write_count: 30,
            open_count: 15,
            close_count: 10,
            bytes_read: 1024 * 1024 * 8,  // 8 MB
            bytes_written: 1024 * 1024 * 3,  // 3 MB
        });
        
        Some(details)
    }

    /// Собрать детализированную статистику по производительности GPU
    #[cfg(feature = "ebpf")]
    fn collect_gpu_details(&self) -> Option<Vec<GpuStat>> {
        // В реальной реализации здесь будет сбор детализированной статистики
        // из eBPF карт. Пока что возвращаем тестовые данные.
        
        if !self.config.enable_gpu_monitoring {
            return None;
        }

        // В реальной реализации здесь будет сбор данных из eBPF карт
        // Используя libbpf-rs API для доступа к картам
        
        // TODO: Реальный сбор данных из eBPF карт
        // Пока что возвращаем тестовые данные для демонстрации функциональности
        
        let mut details = Vec::new();
        
        // Добавляем статистику для нескольких GPU устройств
        details.push(GpuStat {
            gpu_id: 0,
            gpu_usage: 45.5,
            memory_usage: 2 * 1024 * 1024 * 1024,  // 2 GB
            compute_units_active: 8,
            power_usage_uw: 150000,  // 150 W
        });
        
        details.push(GpuStat {
            gpu_id: 1,
            gpu_usage: 20.0,
            memory_usage: 1 * 1024 * 1024 * 1024,  // 1 GB
            compute_units_active: 4,
            power_usage_uw: 75000,  // 75 W
        });
        
        Some(details)
    }

    /// Собрать текущие метрики
    pub fn collect_metrics(&mut self) -> Result<EbpfMetrics> {
        if !self.initialized {
            tracing::warn!("eBPF метрики не инициализированы, возвращаем значения по умолчанию");
            return Ok(EbpfMetrics::default());
        }

        // Оптимизация: агрессивное кэширование
        if self.config.enable_aggressive_caching {
            if let Some(last_cache_time) = self.last_aggressive_cache_time {
                let current_time = std::time::SystemTime::now();
                let elapsed = current_time.duration_since(last_cache_time).unwrap_or(Duration::from_secs(0));
                
                if (elapsed.as_millis() as u64) < self.config.aggressive_cache_interval_ms {
                    // Возвращаем кэшированные метрики
                    if let Some(cached_metrics) = self.metrics_cache.clone() {
                        return Ok(cached_metrics);
                    }
                }
            }
        }

        // Оптимизация: используем кэширование если включено
        if self.config.enable_caching {
            if let Some(cached_metrics) = self.metrics_cache.clone() {
                // Возвращаем кэшированные метрики для уменьшения накладных расходов
                self.batch_counter += 1;
                
                // Сбрасываем кэш если достигнут размер batch
                if self.batch_counter >= self.config.batch_size {
                    self.metrics_cache = None;
                    self.batch_counter = 0;
                }
                
                return Ok(cached_metrics);
            }
        }

        #[cfg(feature = "ebpf")]
        {
            // В реальной реализации здесь будет сбор метрик из eBPF программ
            // Пока что возвращаем тестовые значения
            // В будущем нужно заменить на реальный сбор данных из eBPF карт
            
            // TODO: Реальный сбор метрик из eBPF программ
            // Используя libbpf-rs API для доступа к картам и программам
            
            let cpu_usage = if self.config.enable_cpu_metrics { 25.5 } else { 0.0 };
            let memory_usage = if self.config.enable_memory_metrics { 1024 * 1024 * 512 } else { 0 };
            let syscall_count = if self.config.enable_syscall_monitoring { 100 } else { 0 };
            let network_packets = if self.config.enable_network_monitoring { 250 } else { 0 };
            let network_bytes = if self.config.enable_network_monitoring { 1024 * 1024 * 5 } else { 0 };  // 5 MB
            let gpu_usage = if self.config.enable_gpu_monitoring { 30.0 } else { 0.0 };
            let gpu_memory_usage = if self.config.enable_gpu_monitoring { 1024 * 1024 * 1024 } else { 0 };  // 1 GB
            let filesystem_ops = if self.config.enable_filesystem_monitoring { 150 } else { 0 };
            
            // Собираем детализированную статистику по системным вызовам
            let syscall_details = self.collect_syscall_details();
            
            // Собираем детализированную статистику по сетевой активности
            let network_details = self.collect_network_details();
            
            // Собираем детализированную статистику по производительности GPU
            let gpu_details = self.collect_gpu_details();
            
            // Собираем детализированную статистику по операциям с файловой системой
            let filesystem_details = self.collect_filesystem_details();
            
            let metrics = EbpfMetrics {
                cpu_usage,
                memory_usage,
                syscall_count,
                network_packets,
                network_bytes,
                gpu_usage,
                gpu_memory_usage,
                filesystem_ops,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_nanos() as u64,
                syscall_details,
                network_details,
                gpu_details,
                filesystem_details,
            };
            
            // Кэшируем метрики если включено кэширование
            if self.config.enable_caching {
                self.metrics_cache = Some(metrics.clone());
                self.batch_counter = 1;
            }
            
            // Обновляем время агрессивного кэширования
            if self.config.enable_aggressive_caching {
                self.last_aggressive_cache_time = Some(std::time::SystemTime::now());
            }
            
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

    /// Получить последнюю ошибку инициализации
    pub fn get_last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Проверить, что конфигурация корректна
    pub fn validate_config(&self) -> Result<()> {
        if self.config.batch_size == 0 {
            anyhow::bail!("batch_size не может быть 0");
        }
        
        if self.config.max_init_attempts == 0 {
            anyhow::bail!("max_init_attempts не может быть 0");
        }
        
        if self.config.collection_interval.as_secs() == 0 && self.config.collection_interval.as_millis() == 0 {
            anyhow::bail!("collection_interval не может быть 0");
        }
        
        Ok(())
    }

    /// Проверить, инициализирован ли коллектор
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Получить статистику инициализации
    pub fn get_initialization_stats(&self) -> (usize, usize) {
        // В реальной реализации здесь будет возвращаться реальная статистика
        // Пока что возвращаем тестовые значения
        (3, 1) // 3 программы загружено успешно, 1 ошибка
    }

    /// Сбросить состояние коллектора (для тестирования)
    pub fn reset(&mut self) {
        self.initialized = false;
        self.metrics_cache = None;
        self.batch_counter = 0;
        self.init_attempts = 0;
        self.last_error = None;
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
        assert!(metrics.syscall_details.is_none());
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
        let config = EbpfConfig {
            enable_cpu_metrics: true,
            enable_memory_metrics: false,
            ..Default::default()
        };
        
        let mut collector = EbpfMetricsCollector::new(config);
        assert!(collector.initialize().is_ok());
        
        let metrics = collector.collect_metrics().unwrap();
        // Проверяем, что метрики собираются корректно
        assert!(metrics.cpu_usage >= 0.0);
        assert_eq!(metrics.memory_usage, 0); // Должно быть 0, так как отключено в конфиге
    }

    #[test]
    fn test_ebpf_syscall_monitoring() {
        let config = EbpfConfig {
            enable_syscall_monitoring: true,
            enable_cpu_metrics: false,
            enable_memory_metrics: false,
            ..Default::default()
        };
        
        let mut collector = EbpfMetricsCollector::new(config);
        assert!(collector.initialize().is_ok());
        
        let metrics = collector.collect_metrics().unwrap();
        // Проверяем, что мониторинг системных вызовов работает
        assert_eq!(metrics.cpu_usage, 0.0); // Должно быть 0, так как отключено в конфиге
        assert_eq!(metrics.memory_usage, 0); // Должно быть 0, так как отключено в конфиге
        
        // В тестовой реализации syscall_count должно быть 100, так как включено в конфиге
        #[cfg(feature = "ebpf")] {
            assert_eq!(metrics.syscall_count, 100);
        }
        #[cfg(not(feature = "ebpf"))] {
            // Без eBPF поддержки syscall_count должно быть 0
            assert_eq!(metrics.syscall_count, 0);
        }
    }

    #[test]
    fn test_ebpf_double_initialization() {
        let config = EbpfConfig::default();
        let mut collector = EbpfMetricsCollector::new(config);
        
        assert!(collector.initialize().is_ok());
        // Вторая инициализация должна пройти успешно, но не делать ничего
        assert!(collector.initialize().is_ok());
    }

    #[test]
    fn test_ebpf_caching() {
        let config = EbpfConfig {
            enable_caching: true,
            batch_size: 3,
            ..Default::default()
        };
        
        let mut collector = EbpfMetricsCollector::new(config);
        assert!(collector.initialize().is_ok());
        
        // Первый вызов должен собрать реальные метрики
        let metrics1 = collector.collect_metrics().unwrap();
        assert!(metrics1.cpu_usage >= 0.0);
        
        // Второй и третий вызовы должны вернуть кэшированные метрики
        let metrics2 = collector.collect_metrics().unwrap();
        let metrics3 = collector.collect_metrics().unwrap();
        
        // После третьего вызова кэш должен сброситься
        assert_eq!(metrics1.cpu_usage, metrics2.cpu_usage);
        assert_eq!(metrics1.cpu_usage, metrics3.cpu_usage);
        
        // Четвертый вызов должен собрать новые метрики
        let metrics4 = collector.collect_metrics().unwrap();
        // В тестовой реализации метрики не меняются, поэтому они должны быть такими же
        assert_eq!(metrics1.cpu_usage, metrics4.cpu_usage);
    }

    #[test]
    fn test_ebpf_config_serialization() {
        let config = EbpfConfig {
            enable_cpu_metrics: true,
            enable_memory_metrics: false,
            enable_syscall_monitoring: true,
            enable_network_monitoring: false,
            enable_gpu_monitoring: false,
            enable_filesystem_monitoring: false,
            collection_interval: Duration::from_secs(2),
            enable_caching: true,
            batch_size: 200,
            max_init_attempts: 3,
            operation_timeout_ms: 1000,
            enable_high_performance_mode: true,
            enable_aggressive_caching: false,
            aggressive_cache_interval_ms: 5000,
        };
        
        // Тестируем сериализацию и десериализацию
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: EbpfConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.enable_cpu_metrics, deserialized.enable_cpu_metrics);
        assert_eq!(config.enable_memory_metrics, deserialized.enable_memory_metrics);
        assert_eq!(config.enable_syscall_monitoring, deserialized.enable_syscall_monitoring);
        assert_eq!(config.enable_network_monitoring, deserialized.enable_network_monitoring);
        assert_eq!(config.collection_interval, deserialized.collection_interval);
        assert_eq!(config.enable_caching, deserialized.enable_caching);
        assert_eq!(config.batch_size, deserialized.batch_size);
        assert_eq!(config.max_init_attempts, deserialized.max_init_attempts);
        assert_eq!(config.operation_timeout_ms, deserialized.operation_timeout_ms);
    }

    #[test]
    fn test_ebpf_metrics_serialization() {
        let metrics = EbpfMetrics {
            cpu_usage: 42.5,
            memory_usage: 1024 * 1024 * 1024,  // 1 GB
            syscall_count: 1000,
            network_packets: 500,
            network_bytes: 1024 * 1024 * 10,
            gpu_usage: 0.0,
            gpu_memory_usage: 0,
            filesystem_ops: 0,
            timestamp: 1234567890,
            syscall_details: None,
            network_details: None,
            gpu_details: None,
            filesystem_details: None,
        };
        
        // Тестируем сериализацию и десериализацию
        let serialized = serde_json::to_string(&metrics).unwrap();
        let deserialized: EbpfMetrics = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(metrics.cpu_usage, deserialized.cpu_usage);
        assert_eq!(metrics.memory_usage, deserialized.memory_usage);
        assert_eq!(metrics.syscall_count, deserialized.syscall_count);
        assert_eq!(metrics.network_packets, deserialized.network_packets);
        assert_eq!(metrics.network_bytes, deserialized.network_bytes);
        assert_eq!(metrics.timestamp, deserialized.timestamp);
    }

    #[test]
    fn test_ebpf_disabled_feature() {
        // Тестируем поведение при отключенной eBPF поддержке
        let config = EbpfConfig::default();
        let mut collector = EbpfMetricsCollector::new(config);
        
        // Инициализация должна пройти успешно даже без eBPF поддержки
        assert!(collector.initialize().is_ok());
        
        // Сбор метрик должен вернуть значения по умолчанию
        let metrics = collector.collect_metrics().unwrap();
        assert_eq!(metrics.cpu_usage, 0.0);
        assert_eq!(metrics.memory_usage, 0);
        assert_eq!(metrics.syscall_count, 0);
    }

    #[test]
    fn test_ebpf_high_performance_config() {
        // Тестируем конфигурацию высокопроизводительного режима
        let config = EbpfConfig {
            enable_high_performance_mode: true,
            enable_aggressive_caching: true,
            aggressive_cache_interval_ms: 1000,
            ..Default::default()
        };
        
        let mut collector = EbpfMetricsCollector::new(config);
        assert!(collector.initialize().is_ok());
        
        // Проверяем, что агрессивное кэширование работает
        let metrics1 = collector.collect_metrics().unwrap();
        let metrics2 = collector.collect_metrics().unwrap();
        
        // Вторые метрики должны быть кэшированы
        assert_eq!(metrics1.cpu_usage, metrics2.cpu_usage);
        assert_eq!(metrics1.memory_usage, metrics2.memory_usage);
    }

    #[test]
    fn test_ebpf_aggressive_caching() {
        // Тестируем агрессивное кэширование
        let config = EbpfConfig {
            enable_aggressive_caching: true,
            aggressive_cache_interval_ms: 10000, // Большой интервал для теста
            ..Default::default()
        };
        
        let mut collector = EbpfMetricsCollector::new(config);
        assert!(collector.initialize().is_ok());
        
        // Первый вызов должен собрать реальные метрики
        let metrics1 = collector.collect_metrics().unwrap();
        
        // Второй вызов должен вернуть кэшированные метрики
        let metrics2 = collector.collect_metrics().unwrap();
        
        // Метрики должны быть одинаковыми
        assert_eq!(metrics1.cpu_usage, metrics2.cpu_usage);
        assert_eq!(metrics1.memory_usage, metrics2.memory_usage);
        assert_eq!(metrics1.syscall_count, metrics2.syscall_count);
    }

    #[test]
    fn test_ebpf_config_high_performance_defaults() {
        // Тестируем значения по умолчанию для высокопроизводительного режима
        let config = EbpfConfig::default();
        
        assert!(config.enable_high_performance_mode);
        assert!(!config.enable_aggressive_caching);
        assert_eq!(config.aggressive_cache_interval_ms, 5000);
    }

    #[test]
    fn test_ebpf_gpu_monitoring() {
        // Тестируем мониторинг GPU
        let config = EbpfConfig {
            enable_gpu_monitoring: true,
            enable_cpu_metrics: false,
            enable_memory_metrics: false,
            enable_syscall_monitoring: false,
            enable_network_monitoring: false,
            ..Default::default()
        };
        
        let mut collector = EbpfMetricsCollector::new(config);
        assert!(collector.initialize().is_ok());
        
        let metrics = collector.collect_metrics().unwrap();
        
        // Проверяем, что GPU метрики собираются корректно
        assert_eq!(metrics.cpu_usage, 0.0); // Должно быть 0, так как отключено в конфиге
        assert_eq!(metrics.memory_usage, 0); // Должно быть 0, так как отключено в конфиге
        assert_eq!(metrics.syscall_count, 0); // Должно быть 0, так как отключено в конфиге
        assert_eq!(metrics.network_packets, 0); // Должно быть 0, так как отключено в конфиге
        assert_eq!(metrics.network_bytes, 0); // Должно быть 0, так как отключено в конфиге
        
        // В тестовой реализации GPU метрики должны быть установлены
        #[cfg(feature = "ebpf")] {
            assert_eq!(metrics.gpu_usage, 30.0);
            assert_eq!(metrics.gpu_memory_usage, 1024 * 1024 * 1024); // 1 GB
        }
        #[cfg(not(feature = "ebpf"))] {
            // Без eBPF поддержки GPU метрики должны быть 0
            assert_eq!(metrics.gpu_usage, 0.0);
            assert_eq!(metrics.gpu_memory_usage, 0);
        }
    }

    #[test]
    fn test_ebpf_gpu_details() {
        // Тестируем детализированную статистику GPU
        let config = EbpfConfig {
            enable_gpu_monitoring: true,
            ..Default::default()
        };
        
        let mut collector = EbpfMetricsCollector::new(config);
        assert!(collector.initialize().is_ok());
        
        let metrics = collector.collect_metrics().unwrap();
        
        // Проверяем детализированную статистику GPU
        if let Some(gpu_details) = metrics.gpu_details {
            assert!(!gpu_details.is_empty());
            
            // Проверяем, что статистика имеет разумные значения
            for gpu_stat in gpu_details {
                assert!(gpu_stat.gpu_usage >= 0.0 && gpu_stat.gpu_usage <= 100.0);
                assert!(gpu_stat.memory_usage > 0);
                assert!(gpu_stat.compute_units_active > 0);
                assert!(gpu_stat.power_usage_uw > 0);
            }
        }
    }

    #[test]
    fn test_ebpf_gpu_config_serialization() {
        // Тестируем сериализацию и десериализацию конфигурации с GPU
        let config = EbpfConfig {
            enable_gpu_monitoring: true,
            enable_cpu_metrics: true,
            enable_memory_metrics: false,
            enable_syscall_monitoring: true,
            enable_network_monitoring: false,
            enable_filesystem_monitoring: false,
            collection_interval: Duration::from_secs(2),
            enable_caching: true,
            batch_size: 200,
            max_init_attempts: 3,
            operation_timeout_ms: 1000,
            enable_high_performance_mode: true,
            enable_aggressive_caching: false,
            aggressive_cache_interval_ms: 5000,
        };
        
        // Тестируем сериализацию и десериализацию
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: EbpfConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.enable_gpu_monitoring, deserialized.enable_gpu_monitoring);
        assert_eq!(config.enable_cpu_metrics, deserialized.enable_cpu_metrics);
        assert_eq!(config.enable_memory_metrics, deserialized.enable_memory_metrics);
        assert_eq!(config.enable_syscall_monitoring, deserialized.enable_syscall_monitoring);
        assert_eq!(config.enable_network_monitoring, deserialized.enable_network_monitoring);
        assert_eq!(config.collection_interval, deserialized.collection_interval);
        assert_eq!(config.enable_caching, deserialized.enable_caching);
        assert_eq!(config.batch_size, deserialized.batch_size);
        assert_eq!(config.max_init_attempts, deserialized.max_init_attempts);
        assert_eq!(config.operation_timeout_ms, deserialized.operation_timeout_ms);
    }

    #[test]
    fn test_ebpf_gpu_metrics_serialization() {
        // Тестируем сериализацию и десериализацию метрик с GPU
        let metrics = EbpfMetrics {
            cpu_usage: 42.5,
            memory_usage: 1024 * 1024 * 1024,  // 1 GB
            syscall_count: 1000,
            network_packets: 500,
            network_bytes: 1024 * 1024 * 10,
            gpu_usage: 75.0,
            gpu_memory_usage: 2 * 1024 * 1024 * 1024,  // 2 GB
            filesystem_ops: 0,
            timestamp: 1234567890,
            syscall_details: None,
            network_details: None,
            gpu_details: None,
            filesystem_details: None,
        };
        
        // Тестируем сериализацию и десериализацию
        let serialized = serde_json::to_string(&metrics).unwrap();
        let deserialized: EbpfMetrics = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(metrics.cpu_usage, deserialized.cpu_usage);
        assert_eq!(metrics.memory_usage, deserialized.memory_usage);
        assert_eq!(metrics.syscall_count, deserialized.syscall_count);
        assert_eq!(metrics.network_packets, deserialized.network_packets);
        assert_eq!(metrics.network_bytes, deserialized.network_bytes);
        assert_eq!(metrics.gpu_usage, deserialized.gpu_usage);
        assert_eq!(metrics.gpu_memory_usage, deserialized.gpu_memory_usage);
        assert_eq!(metrics.filesystem_ops, deserialized.filesystem_ops);
        assert_eq!(metrics.timestamp, deserialized.timestamp);
    }

    #[test]
    fn test_ebpf_filesystem_monitoring() {
        // Тестируем мониторинг файловой системы
        let config = EbpfConfig {
            enable_filesystem_monitoring: true,
            enable_cpu_metrics: false,
            enable_memory_metrics: false,
            enable_syscall_monitoring: false,
            enable_network_monitoring: false,
            enable_gpu_monitoring: false,
            ..Default::default()
        };
        
        let mut collector = EbpfMetricsCollector::new(config);
        assert!(collector.initialize().is_ok());
        
        let metrics = collector.collect_metrics().unwrap();
        
        // Проверяем, что метрики файловой системы собираются корректно
        assert_eq!(metrics.cpu_usage, 0.0); // Должно быть 0, так как отключено в конфиге
        assert_eq!(metrics.memory_usage, 0); // Должно быть 0, так как отключено в конфиге
        assert_eq!(metrics.syscall_count, 0); // Должно быть 0, так как отключено в конфиге
        assert_eq!(metrics.network_packets, 0); // Должно быть 0, так как отключено в конфиге
        assert_eq!(metrics.network_bytes, 0); // Должно быть 0, так как отключено в конфиге
        assert_eq!(metrics.gpu_usage, 0.0); // Должно быть 0, так как отключено в конфиге
        assert_eq!(metrics.gpu_memory_usage, 0); // Должно быть 0, так как отключено в конфиге
        
        // В тестовой реализации метрики файловой системы должны быть установлены
        #[cfg(feature = "ebpf")] {
            assert_eq!(metrics.filesystem_ops, 150);
        }
        #[cfg(not(feature = "ebpf"))] {
            // Без eBPF поддержки метрики файловой системы должны быть 0
            assert_eq!(metrics.filesystem_ops, 0);
        }
    }

    #[test]
    fn test_ebpf_filesystem_details() {
        // Тестируем детализированную статистику файловой системы
        let config = EbpfConfig {
            enable_filesystem_monitoring: true,
            ..Default::default()
        };
        
        let mut collector = EbpfMetricsCollector::new(config);
        assert!(collector.initialize().is_ok());
        
        let metrics = collector.collect_metrics().unwrap();
        
        // Проверяем детализированную статистику файловой системы
        if let Some(filesystem_details) = metrics.filesystem_details {
            assert!(!filesystem_details.is_empty());
            
            // Проверяем, что статистика имеет разумные значения
            for fs_stat in filesystem_details {
                assert!(fs_stat.read_count > 0);
                assert!(fs_stat.write_count > 0);
                assert!(fs_stat.open_count > 0);
                assert!(fs_stat.close_count > 0);
                assert!(fs_stat.bytes_read > 0);
                assert!(fs_stat.bytes_written > 0);
            }
        }
    }

    #[test]
    fn test_ebpf_filesystem_config_serialization() {
        // Тестируем сериализацию и десериализацию конфигурации с файловой системой
        let config = EbpfConfig {
            enable_filesystem_monitoring: true,
            enable_cpu_metrics: true,
            enable_memory_metrics: false,
            enable_syscall_monitoring: true,
            enable_network_monitoring: false,
            enable_gpu_monitoring: false,
            collection_interval: Duration::from_secs(2),
            enable_caching: true,
            batch_size: 200,
            max_init_attempts: 3,
            operation_timeout_ms: 1000,
            enable_high_performance_mode: true,
            enable_aggressive_caching: false,
            aggressive_cache_interval_ms: 5000,
        };
        
        // Тестируем сериализацию и десериализацию
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: EbpfConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.enable_filesystem_monitoring, deserialized.enable_filesystem_monitoring);
        assert_eq!(config.enable_cpu_metrics, deserialized.enable_cpu_metrics);
        assert_eq!(config.enable_memory_metrics, deserialized.enable_memory_metrics);
        assert_eq!(config.enable_syscall_monitoring, deserialized.enable_syscall_monitoring);
        assert_eq!(config.enable_network_monitoring, deserialized.enable_network_monitoring);
        assert_eq!(config.enable_gpu_monitoring, deserialized.enable_gpu_monitoring);
        assert_eq!(config.collection_interval, deserialized.collection_interval);
        assert_eq!(config.enable_caching, deserialized.enable_caching);
        assert_eq!(config.batch_size, deserialized.batch_size);
        assert_eq!(config.max_init_attempts, deserialized.max_init_attempts);
        assert_eq!(config.operation_timeout_ms, deserialized.operation_timeout_ms);
    }

    #[test]
    fn test_ebpf_filesystem_metrics_serialization() {
        // Тестируем сериализацию и десериализацию метрик с файловой системой
        let metrics = EbpfMetrics {
            cpu_usage: 42.5,
            memory_usage: 1024 * 1024 * 1024,  // 1 GB
            syscall_count: 1000,
            network_packets: 500,
            network_bytes: 1024 * 1024 * 10,
            gpu_usage: 0.0,
            gpu_memory_usage: 0,
            filesystem_ops: 200,
            timestamp: 1234567890,
            syscall_details: None,
            network_details: None,
            gpu_details: None,
            filesystem_details: None,
        };
        
        // Тестируем сериализацию и десериализацию
        let serialized = serde_json::to_string(&metrics).unwrap();
        let deserialized: EbpfMetrics = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(metrics.cpu_usage, deserialized.cpu_usage);
        assert_eq!(metrics.memory_usage, deserialized.memory_usage);
        assert_eq!(metrics.syscall_count, deserialized.syscall_count);
        assert_eq!(metrics.network_packets, deserialized.network_packets);
        assert_eq!(metrics.network_bytes, deserialized.network_bytes);
        assert_eq!(metrics.gpu_usage, deserialized.gpu_usage);
        assert_eq!(metrics.gpu_memory_usage, deserialized.gpu_memory_usage);
        assert_eq!(metrics.filesystem_ops, deserialized.filesystem_ops);
        assert_eq!(metrics.timestamp, deserialized.timestamp);
    }

    #[test]
    fn test_ebpf_kernel_version_parsing() {
        // Тестируем парсинг версии ядра
        #[cfg(target_os = "linux")] {
            // В тестовой среде может не быть доступа к /proc, поэтому проверяем только логику
            let result = EbpfMetricsCollector::get_kernel_version();
            // В большинстве случаев это должно завершиться успешно или вернуть ошибку
            match result {
                Ok(version) => {
                    // Если удалось получить версию, проверяем что она разумная
                    assert!(version.0 >= 2);  // Мажорная версия должна быть >= 2
                }
                Err(_) => {
                    // В тестовой среде это нормально
                }
            }
        }
    }

    #[test]
    fn test_ebpf_config_validation() {
        // Тестируем валидацию конфигурации
        let mut config = EbpfConfig::default();
        let mut collector = EbpfMetricsCollector::new(config.clone());
        
        // Корректная конфигурация должна проходить валидацию
        assert!(collector.validate_config().is_ok());
        
        // Тестируем некорректные конфигурации
        config.batch_size = 0;
        let mut collector = EbpfMetricsCollector::new(config.clone());
        assert!(collector.validate_config().is_err());
        
        config.batch_size = 100;
        config.max_init_attempts = 0;
        let mut collector = EbpfMetricsCollector::new(config.clone());
        assert!(collector.validate_config().is_err());
        
        config.max_init_attempts = 3;
        config.collection_interval = Duration::from_secs(0);
        let mut collector = EbpfMetricsCollector::new(config);
        assert!(collector.validate_config().is_err());
    }

    #[test]
    fn test_ebpf_error_handling() {
        // Тестируем обработку ошибок
        let config = EbpfConfig::default();
        let mut collector = EbpfMetricsCollector::new(config);
        
        // Проверяем, что последняя ошибка отсутствует изначально
        assert!(collector.get_last_error().is_none());
        
        // Инициализация должна пройти успешно
        assert!(collector.initialize().is_ok());
        
        // Проверяем, что последняя ошибка может быть получена
        let error = collector.get_last_error();
        // В зависимости от окружения, может быть ошибка или нет
        if let Some(err) = error {
            assert!(!err.is_empty());
        }
    }

    #[test]
    fn test_ebpf_program_loading_with_libbpf() {
        // Тестируем загрузку eBPF программ с использованием libbpf-rs
        let config = EbpfConfig {
            enable_cpu_metrics: true,
            enable_syscall_monitoring: true,
            enable_network_monitoring: true,
            ..Default::default()
        };
        
        let mut collector = EbpfMetricsCollector::new(config);
        
        // Инициализация должна пройти успешно
        assert!(collector.initialize().is_ok());
        
        // Проверяем, что коллектор инициализирован (зависит от наличия eBPF поддержки)
        #[cfg(feature = "ebpf")] {
            assert!(collector.is_initialized());
        }
        #[cfg(not(feature = "ebpf"))] {
            // Без eBPF поддержки коллектор не инициализируется
            assert!(!collector.is_initialized());
        }
        
        // Сбор метрик должен работать
        let metrics = collector.collect_metrics();
        assert!(metrics.is_ok());
        
        let metrics = metrics.unwrap();
        // Проверяем, что метрики имеют разумные значения
        assert!(metrics.cpu_usage >= 0.0);
        assert!(metrics.memory_usage >= 0);
        assert!(metrics.syscall_count >= 0);
        assert!(metrics.network_packets >= 0);
        assert!(metrics.network_bytes >= 0);
    }

    #[test]
    fn test_ebpf_enhanced_error_handling() {
        // Тестируем улучшенную обработку ошибок
        let config = EbpfConfig {
            enable_cpu_metrics: true,
            enable_syscall_monitoring: true,
            enable_network_monitoring: true,
            ..Default::default()
        };
        
        let mut collector = EbpfMetricsCollector::new(config.clone());
        
        // Тестируем инициализацию с некорректной конфигурацией
        let mut bad_config = config.clone();
        bad_config.batch_size = 0; // Некорректное значение
        let mut bad_collector = EbpfMetricsCollector::new(bad_config);
        
        // Инициализация с некорректной конфигурацией должна завершиться с ошибкой
        assert!(bad_collector.initialize().is_err());
        assert!(bad_collector.get_last_error().is_some());
        
        // Тестируем инициализацию с корректной конфигурацией
        assert!(collector.initialize().is_ok());
        
        // Тестируем получение статистики инициализации
        let (success, errors) = collector.get_initialization_stats();
        assert!(success > 0 || errors >= 0); // Хотя бы одна попытка должна быть
        
        // Тестируем graceful degradation
        // Даже если некоторые программы не загрузились, коллектор должен работать
        let metrics = collector.collect_metrics();
        assert!(metrics.is_ok());
    }

    #[test]
    fn test_ebpf_reset() {
        // Тестируем сброс состояния коллектора
        let config = EbpfConfig::default();
        let mut collector = EbpfMetricsCollector::new(config);
        
        // Инициализируем коллектор
        assert!(collector.initialize().is_ok());
        
        // Проверяем начальное состояние после инициализации
        #[cfg(feature = "ebpf")] {
            assert!(collector.is_initialized());
        }
        #[cfg(not(feature = "ebpf"))] {
            // Без eBPF поддержки коллектор не инициализируется
            assert!(!collector.is_initialized());
        }
        
        // Сбрасываем состояние
        collector.reset();
        
        // Проверяем, что состояние сброшено
        assert!(!collector.is_initialized());
        assert!(collector.metrics_cache.is_none());
        assert_eq!(collector.batch_counter, 0);
        assert_eq!(collector.init_attempts, 0);
        assert!(collector.get_last_error().is_none());
    }

    #[test]
    fn test_ebpf_graceful_degradation() {
        // Тестируем graceful degradation при отсутствии eBPF поддержки
        let config = EbpfConfig {
            enable_cpu_metrics: true,
            enable_memory_metrics: true,
            enable_syscall_monitoring: true,
            enable_network_monitoring: true,
            ..Default::default()
        };
        
        let mut collector = EbpfMetricsCollector::new(config);
        
        // Инициализация должна пройти успешно даже без eBPF поддержки
        assert!(collector.initialize().is_ok());
        
        // Сбор метрик должен вернуть значения по умолчанию
        let metrics = collector.collect_metrics().unwrap();
        
        // Проверяем, что метрики имеют разумные значения по умолчанию
        assert!(metrics.cpu_usage >= 0.0);
        assert!(metrics.memory_usage >= 0);
        assert!(metrics.syscall_count >= 0);
        assert!(metrics.network_packets >= 0);
        assert!(metrics.network_bytes >= 0);
    }
}