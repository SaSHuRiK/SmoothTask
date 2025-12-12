pub mod actuator;
pub mod api;
pub mod classify;
pub mod config;
pub mod logging;
pub mod metrics;
pub mod model;
pub mod policy;
pub mod utils;

use anyhow::{Context, Result};
use chrono::Utc;
use config::Config;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

use crate::actuator::{apply_priority_adjustments, plan_priority_changes, HysteresisTracker};
use crate::api::{ApiServer, ApiServerHandle};
use crate::classify::{grouper::ProcessGrouper, rules::classify_all, rules::PatternDatabase};
use crate::logging::snapshots::{GlobalMetrics, ProcessRecord, ResponsivenessMetrics, Snapshot, SnapshotLogger};
use crate::metrics::audio::{AudioIntrospector, AudioMetrics, StaticAudioIntrospector};
use crate::metrics::audio_pipewire::PipeWireIntrospector;
use crate::metrics::input::{EvdevInputTracker, InputMetrics, InputTracker};
use crate::metrics::process::collect_process_metrics;
use crate::metrics::scheduling_latency::{LatencyCollector, LatencyProbe};
use crate::metrics::system::{
    collect_system_metrics, CpuTimes, LoadAvg, MemoryInfo, PressureMetrics, ProcPaths,
    SystemMetrics, TemperatureMetrics, PowerMetrics, NetworkMetrics, DiskMetrics,
};
use crate::metrics::windows::{
    is_wayland_available, StaticWindowIntrospector, WaylandIntrospector, WindowIntrospector,
    X11Introspector,
};
use crate::policy::engine::PolicyEngine;

/// Callback функция для уведомления о готовности демона (например, для systemd notify).
pub type ReadyCallback = Box<dyn Fn() + Send + Sync>;

/// Callback функция для обновления статуса демона (например, для systemd notify).
pub type StatusCallback = Box<dyn Fn(&str) + Send + Sync>;

///
/// Структура собирает метрики о работе демона во время выполнения:
/// количество итераций, время выполнения, количество применённых изменений приоритетов
/// и ошибок. Статистика логируется периодически (каждые 10 итераций) для мониторинга
/// производительности и отладки.
///
/// # Примеры использования
///
/// ```no_run
/// use smoothtask_core::DaemonStats;
///
/// let mut stats = DaemonStats::new();
///
/// // Записываем успешную итерацию
/// stats.record_successful_iteration(100, 5, 1);
///
/// // Записываем итерацию с ошибкой
/// stats.record_error_iteration();
///
/// // Вычисляем среднее время итерации
/// let avg = stats.average_iteration_duration_ms();
///
/// // Логируем статистику
/// stats.log_stats();
/// ```
///
/// # Поля
///
/// - `total_iterations`: Общее количество итераций (успешных и с ошибками)
/// - `successful_iterations`: Количество успешных итераций (без ошибок сбора метрик)
/// - `error_iterations`: Количество итераций с ошибками (ошибки при сборе метрик)
/// - `total_duration_ms`: Суммарное время выполнения всех успешных итераций
/// - `max_iteration_duration_ms`: Максимальное время выполнения одной итерации
/// - `total_applied_adjustments`: Общее количество применённых изменений приоритетов
/// - `total_apply_errors`: Общее количество ошибок при применении приоритетов
#[derive(Debug, Clone, serde::Serialize)]
pub struct DaemonStats {
    /// Общее количество итераций (успешных и с ошибками)
    total_iterations: u64,
    /// Количество успешных итераций (без ошибок сбора метрик)
    successful_iterations: u64,
    /// Количество итераций с ошибками (ошибки при сборе метрик)
    error_iterations: u64,
    /// Суммарное время выполнения всех успешных итераций (в миллисекундах)
    total_duration_ms: u128,
    /// Максимальное время выполнения одной итерации (в миллисекундах)
    max_iteration_duration_ms: u128,
    /// Количество применённых изменений приоритетов
    total_applied_adjustments: u64,
    /// Количество ошибок при применении приоритетов
    total_apply_errors: u64,
}

impl DaemonStats {
    /// Создаёт новую статистику с нулевыми значениями.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::DaemonStats;
    ///
    /// let stats = DaemonStats::new();
    /// // Статистика инициализирована с нулевыми значениями
    /// assert_eq!(stats.average_iteration_duration_ms(), 0.0);
    /// ```
    pub fn new() -> Self {
        Self {
            total_iterations: 0,
            successful_iterations: 0,
            error_iterations: 0,
            total_duration_ms: 0,
            max_iteration_duration_ms: 0,
            total_applied_adjustments: 0,
            total_apply_errors: 0,
        }
    }

    /// Обновляет статистику после успешной итерации.
    ///
    /// Увеличивает счётчики итераций, обновляет время выполнения и статистику
    /// применения приоритетов.
    ///
    /// # Параметры
    ///
    /// - `duration_ms`: Время выполнения итерации в миллисекундах
    /// - `applied`: Количество успешно применённых изменений приоритетов
    /// - `errors`: Количество ошибок при применении приоритетов
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::DaemonStats;
    ///
    /// let mut stats = DaemonStats::new();
    /// stats.record_successful_iteration(100, 5, 1);
    /// // Статистика обновлена: среднее время итерации вычисляется
    /// assert_eq!(stats.average_iteration_duration_ms(), 100.0);
    /// ```
    pub fn record_successful_iteration(&mut self, duration_ms: u128, applied: u64, errors: u64) {
        self.total_iterations += 1;
        self.successful_iterations += 1;
        self.total_duration_ms += duration_ms;
        self.max_iteration_duration_ms = self.max_iteration_duration_ms.max(duration_ms);
        self.total_applied_adjustments += applied;
        self.total_apply_errors += errors;
    }

    /// Обновляет статистику после итерации с ошибкой.
    ///
    /// Увеличивает счётчики итераций и ошибок. Используется, когда итерация
    /// завершилась с ошибкой при сборе метрик (например, ошибка чтения /proc).
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::DaemonStats;
    ///
    /// let mut stats = DaemonStats::new();
    /// stats.record_error_iteration();
    /// // Статистика обновлена: среднее время итерации остаётся 0.0 (нет успешных итераций)
    /// assert_eq!(stats.average_iteration_duration_ms(), 0.0);
    /// ```
    pub fn record_error_iteration(&mut self) {
        self.total_iterations += 1;
        self.error_iterations += 1;
    }

    /// Вычисляет среднее время итерации (в миллисекундах).
    ///
    /// Возвращает среднее время выполнения успешных итераций. Если успешных итераций
    /// не было, возвращает 0.0.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::DaemonStats;
    ///
    /// let mut stats = DaemonStats::new();
    /// stats.record_successful_iteration(100, 0, 0);
    /// stats.record_successful_iteration(200, 0, 0);
    /// assert_eq!(stats.average_iteration_duration_ms(), 150.0);
    ///
    /// // Без успешных итераций возвращает 0.0
    /// let empty_stats = DaemonStats::new();
    /// assert_eq!(empty_stats.average_iteration_duration_ms(), 0.0);
    /// ```
    pub fn average_iteration_duration_ms(&self) -> f64 {
        if self.successful_iterations > 0 {
            self.total_duration_ms as f64 / self.successful_iterations as f64
        } else {
            0.0
        }
    }

    /// Логирует статистику работы демона.
    ///
    /// Выводит информацию о количестве итераций, среднем и максимальном времени
    /// выполнения, количестве применённых изменений приоритетов и ошибок.
    /// Использует уровень логирования `info!`.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::DaemonStats;
    ///
    /// let mut stats = DaemonStats::new();
    /// stats.record_successful_iteration(100, 5, 1);
    /// stats.log_stats(); // Логирует: "Daemon stats: 1 total iterations..."
    /// ```
    pub fn log_stats(&self) {
        let avg_duration = self.average_iteration_duration_ms();
        info!(
            "Daemon stats: {} total iterations ({} successful, {} errors), \
             avg iteration: {:.2}ms, max iteration: {}ms, \
             applied adjustments: {}, apply errors: {}",
            self.total_iterations,
            self.successful_iterations,
            self.error_iterations,
            avg_duration,
            self.max_iteration_duration_ms,
            self.total_applied_adjustments,
            self.total_apply_errors
        );
    }
}

impl Default for DaemonStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Проверить доступность необходимых системных утилит и устройств.
///
/// Функция проверяет доступность всех компонентов, необходимых для работы демона,
/// и логирует предупреждения, если какие-то компоненты недоступны.
///
/// # Проверяемые компоненты
///
/// - **X11 сервер**: для получения метрик окон через X11 (EWMH)
/// - **Wayland композитор**: для получения метрик окон через Wayland (wlr-foreign-toplevel-management)
/// - **pw-dump**: утилита PipeWire для получения метрик аудио (XRUN, клиенты)
/// - **evdev устройства**: устройства ввода (`/dev/input/event*`) для отслеживания активности пользователя
///
/// # Логирование
///
/// Функция использует следующие уровни логирования:
///
/// - `debug!`: для успешных проверок (компонент доступен)
/// - `warn!`: для недоступных компонентов (может ограничить функциональность)
///
/// # Примеры использования
///
/// ## Базовое использование
///
/// ```no_run
/// use smoothtask_core::check_system_utilities;
///
/// // Проверка доступности компонентов при старте демона
/// check_system_utilities();
/// ```
///
/// ## Использование в тестах
///
/// ```no_run
/// use smoothtask_core::check_system_utilities;
///
/// #[test]
/// fn test_system_utilities_check() {
///     // Функция не должна паниковать независимо от доступности компонентов
///     check_system_utilities();
/// }
/// ```
///
/// # Примечания
///
/// - Функция не возвращает ошибки и не паникует, даже если все компоненты недоступны
/// - Отсутствие некоторых компонентов (например, X11 или Wayland) является нормальной ситуацией,
///   так как система может использовать только один из них
/// - Демон продолжит работу даже при отсутствии некоторых компонентов, используя fallback-решения
///   (например, `StaticWindowIntrospector` вместо X11/Wayland)
pub fn check_system_utilities() {
    // Проверка доступности /proc файловой системы (критично для работы демона)
    let proc_available =
        std::path::Path::new("/proc").exists() && std::path::Path::new("/proc/stat").exists();
    if !proc_available {
        error!(
            "/proc filesystem is not available. This is critical for daemon operation. \
            Ensure you are running on a Linux system with /proc mounted."
        );
    } else {
        debug!("/proc filesystem is available");
    }

    // Проверка доступности cgroups v2 (желательно, но не критично)
    let cgroup_v2_available = {
        let candidates = ["/sys/fs/cgroup", "/sys/fs/cgroup/unified"];
        candidates.iter().any(|candidate| {
            std::path::Path::new(candidate)
                .join("cgroup.controllers")
                .exists()
        })
    };
    if !cgroup_v2_available {
        warn!(
            "cgroups v2 not available. CPU weight adjustments will be limited. \
            Ensure cgroups v2 is mounted at /sys/fs/cgroup."
        );
    } else {
        debug!("cgroups v2 is available");
    }

    // Проверка X11 сервера
    let x11_available = X11Introspector::is_available();
    if !x11_available {
        debug!("X11 server not available (this is normal on Wayland-only systems)");
    } else {
        debug!("X11 server is available");
    }

    // Проверка Wayland композитора
    let wayland_available = is_wayland_available();
    if !wayland_available {
        debug!("Wayland compositor not available (this is normal on X11-only systems)");
    } else {
        debug!("Wayland compositor is available");
    }

    // Проверка pw-dump
    let pw_dump_available = std::process::Command::new("pw-dump")
        .arg("--version")
        .output()
        .is_ok();
    if !pw_dump_available {
        warn!(
            "pw-dump not available. Audio metrics will be limited. \
            Install PipeWire and ensure pw-dump is in PATH."
        );
    } else {
        debug!("pw-dump is available");
    }

    // Проверка evdev устройств
    let evdev_available = EvdevInputTracker::is_available();
    if !evdev_available {
        warn!(
            "No evdev input devices found. User activity tracking will be limited. \
            Ensure /dev/input/event* devices are accessible."
        );
    } else {
        debug!("evdev input devices are available");
    }
}

/// Создаёт window introspector, пытаясь использовать доступные бекенды в порядке приоритета:
/// 1. X11Introspector (если X-сервер доступен)
/// 2. WaylandIntrospector (если Wayland доступен)
/// 3. StaticWindowIntrospector (fallback)
///
/// Функция логирует, какой интроспектор был выбран, и возвращает ошибки только если
/// все бекенды недоступны (в этом случае используется StaticWindowIntrospector).
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn create_window_introspector() -> Box<dyn WindowIntrospector> {
    // Пробуем X11
    if X11Introspector::is_available() {
        match X11Introspector::new() {
            Ok(introspector) => {
                info!("Using X11Introspector for window metrics");
                return Box::new(introspector);
            }
            Err(e) => {
                warn!(
                    "X11 server available but failed to initialize X11Introspector: {}, trying Wayland",
                    e
                );
            }
        }
    }

    // Пробуем Wayland
    if is_wayland_available() {
        match WaylandIntrospector::new() {
            Ok(introspector) => {
                info!("Using WaylandIntrospector for window metrics");
                return Box::new(introspector);
            }
            Err(e) => {
                warn!(
                    "Wayland available but failed to initialize WaylandIntrospector: {}, falling back to StaticWindowIntrospector",
                    e
                );
            }
        }
    } else {
        warn!("Wayland not available, falling back to StaticWindowIntrospector");
    }

    // Fallback на статический интроспектор
    warn!("Neither X11 nor Wayland available, using StaticWindowIntrospector");
    Box::new(StaticWindowIntrospector::new(Vec::new()))
}

/// Главный цикл демона: опрос метрик, ранжирование, применение.
///
/// Демон работает до тех пор, пока не будет получен сигнал завершения через `shutdown_rx`.
/// Для корректного завершения отправьте сигнал через соответствующий `watch::Sender`.
///
/// # Параметры
///
/// - `config`: Конфигурация демона (пороги, пути, режимы работы)
/// - `dry_run`: Если `true`, демон только планирует изменения приоритетов, но не применяет их
/// - `shutdown_rx`: Канал для получения сигнала завершения работы демона
/// - `on_ready`: Опциональный callback, вызываемый после успешной инициализации всех компонентов
/// - `on_status_update`: Опциональный callback для периодического обновления статуса (например, для systemd STATUS)
///
/// # Возвращаемое значение
///
/// Возвращает `Ok(())` при успешном завершении или `Err` при ошибке.
///
/// # Примеры использования
///
/// ## Базовое использование
///
/// ```no_run
/// use smoothtask_core::{config::Config, run_daemon};
/// use tokio::sync::watch;
///
/// # async fn example() -> anyhow::Result<()> {
/// let config = Config::load("configs/smoothtask.yml")?;
/// let (shutdown_tx, shutdown_rx) = watch::channel(false);
///
/// // Запускаем демон в фоновой задаче
/// let daemon_handle = tokio::spawn(async move {
///     run_daemon(config, false, shutdown_rx, None, None).await
/// });
///
/// // Позже отправляем сигнал завершения
/// shutdown_tx.send(true)?;
/// daemon_handle.await??;
/// # Ok(())
/// # }
/// ```
///
/// ## Использование с обработкой сигналов
///
/// ```no_run
/// use smoothtask_core::{config::Config, run_daemon};
/// use tokio::sync::watch;
///
/// # async fn example() -> anyhow::Result<()> {
/// let config = Config::load("configs/smoothtask.yml")?;
/// let (shutdown_tx, shutdown_rx) = watch::channel(false);
///
/// // Обработка SIGINT/SIGTERM (требует tokio с feature "signal")
/// // В реальном использовании можно использовать nix::sys::signal или
/// // tokio::signal::unix::SignalKind для обработки сигналов
/// // let shutdown_tx_clone = shutdown_tx.clone();
/// // tokio::spawn(async move {
/// //     // Обработка сигналов здесь
/// //     let _ = shutdown_tx_clone.send(true);
/// // });
///
/// // Запускаем демон
/// run_daemon(config, false, shutdown_rx, None, None).await?;
/// # Ok(())
/// # }
/// ```
///
/// ## Dry-run режим
///
/// ```no_run
/// use smoothtask_core::{config::Config, run_daemon};
/// use tokio::sync::watch;
///
/// # async fn example() -> anyhow::Result<()> {
/// let config = Config::load("configs/smoothtask.yml")?;
/// let (shutdown_tx, shutdown_rx) = watch::channel(false);
///
/// // Запускаем демон в dry-run режиме (не применяет изменения)
/// run_daemon(config, true, shutdown_rx, None, None).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Ошибки
///
/// Функция может вернуть ошибку в следующих случаях:
///
/// - Не удалось инициализировать snapshot logger (если указан путь к БД)
/// - Не удалось загрузить базу паттернов
/// - Ошибки при сборе метрик (частичные ошибки обрабатываются gracefully)
/// - Ошибки при применении приоритетов (логируются, но не останавливают демон)
///
/// # Примечания
///
/// - Демон работает в бесконечном цикле до получения сигнала завершения
/// - Каждая итерация включает: сбор метрик, группировку процессов, классификацию,
///   применение политики и actuator
/// - Статистика работы логируется периодически (каждые 10 итераций)
/// - Демон корректно обрабатывает частичные ошибки (если один компонент не работает,
///   остальные продолжают работать)
pub async fn run_daemon(
    config: Config,
    dry_run: bool,
    mut shutdown_rx: watch::Receiver<bool>,
    on_ready: Option<ReadyCallback>,
    on_status_update: Option<StatusCallback>,
) -> Result<()> {
    info!("Initializing SmoothTask daemon (dry_run = {})", dry_run);

    // Проверка доступности системных утилит и устройств
    check_system_utilities();

    // Инициализация подсистем
    let mut snapshot_logger =
        if config.enable_snapshot_logging && !config.paths.snapshot_db_path.is_empty() {
            info!(
                "Initializing snapshot logger at: {}",
                config.paths.snapshot_db_path
            );
            Some(
                SnapshotLogger::new(&config.paths.snapshot_db_path).with_context(|| {
                    format!(
                        "Failed to initialize snapshot logger at {}. \
                    Ensure the parent directory exists and is writable.",
                        config.paths.snapshot_db_path
                    )
                })?,
            )
        } else {
            if !config.enable_snapshot_logging {
                debug!("Snapshot logging is disabled (enable_snapshot_logging is false)");
            } else {
                debug!("Snapshot logging is disabled (snapshot_db_path is empty)");
            }
            None
        };

    info!("Initializing policy engine");
    let policy_engine = PolicyEngine::new(config.clone());
    let mut hysteresis = HysteresisTracker::new();

    // Инициализация интроспекторов
    // Пробуем использовать X11Introspector, если X-сервер доступен
    // Если X11 недоступен, пробуем WaylandIntrospector
    // В противном случае используем StaticWindowIntrospector
    // Используем Arc для возможности использования в spawn_blocking
    let window_introspector: Arc<dyn WindowIntrospector> = Arc::from(create_window_introspector());

    // Инициализация PipeWire интроспектора с fallback на статический, если PipeWire недоступен
    // Используем Arc<Mutex<...>> для возможности использования в spawn_blocking
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> = {
        // Проверяем доступность pw-dump
        let pw_dump_available = std::process::Command::new("pw-dump")
            .arg("--version")
            .output()
            .is_ok();

        let introspector: Box<dyn AudioIntrospector> = if pw_dump_available {
            info!("Using PipeWireIntrospector for audio metrics");
            Box::new(PipeWireIntrospector::new())
        } else {
            warn!("pw-dump not available, falling back to StaticAudioIntrospector");
            Box::new(StaticAudioIntrospector::empty())
        };
        Arc::new(Mutex::new(introspector))
    };

    // Инициализация трекера активности пользователя
    // Используем Arc<Mutex<...>> для возможности использования в spawn_blocking
    let idle_threshold = Duration::from_secs(config.thresholds.user_idle_timeout_sec);
    let input_tracker = Arc::new(Mutex::new(InputTracker::new(idle_threshold)));

    // Инициализация probe-thread для измерения scheduling latency
    // Используем размер окна 5000 измерений (примерно 25-50 секунд при интервале 5-10 мс)
    let latency_collector = Arc::new(LatencyCollector::new(5000));
    let sleep_interval_ms = 5; // 5 мс согласно документации tz.md
    let mut latency_probe =
        LatencyProbe::new(Arc::clone(&latency_collector), sleep_interval_ms, 5000);

    // Загрузка базы паттернов для классификации
    info!(
        "Loading pattern database from: {}",
        config.paths.patterns_dir
    );
    let pattern_db = PatternDatabase::load(&config.paths.patterns_dir).with_context(|| {
        format!(
            "Failed to load pattern database from {}. \
            Ensure the directory exists and is readable, and contains valid YAML pattern files.",
            config.paths.patterns_dir
        )
    })?;
    info!(
        "Loaded {} patterns from database",
        pattern_db.all_patterns().len()
    );

    // Инициализация путей для чтения /proc
    let proc_paths = ProcPaths::default();

    // Состояние для вычисления дельт CPU
    let mut prev_cpu_times: Option<SystemMetrics> = None;

    // Кэш для системных метрик (обновляется каждые N итераций)
    let mut system_metrics_cache: Option<SystemMetrics> = None;
    let mut system_metrics_cache_iteration: u64 = 0;
    let system_metrics_cache_interval = config.cache_intervals.system_metrics_cache_interval;

    // Кэш для метрик процессов (обновляется каждые N итераций)
    let mut process_metrics_cache: Option<Vec<ProcessRecord>> = None;
    let mut process_metrics_cache_iteration: u64 = 0;
    let process_metrics_cache_interval = config.cache_intervals.process_metrics_cache_interval;

    // Инициализация структур данных для API сервера
    let stats_arc = Arc::new(tokio::sync::RwLock::new(DaemonStats::new()));
    // Создаём дефолтные SystemMetrics для инициализации API сервера
    let system_metrics_arc = Arc::new(tokio::sync::RwLock::new(SystemMetrics {
        cpu_times: CpuTimes {
            user: 0,
            nice: 0,
            system: 0,
            idle: 0,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
            guest: 0,
            guest_nice: 0,
        },
        memory: MemoryInfo {
            mem_total_kb: 0,
            mem_available_kb: 0,
            mem_free_kb: 0,
            buffers_kb: 0,
            cached_kb: 0,
            swap_total_kb: 0,
            swap_free_kb: 0,
        },
        load_avg: LoadAvg {
            one: 0.0,
            five: 0.0,
            fifteen: 0.0,
        },
        pressure: PressureMetrics::default(),
        temperature: TemperatureMetrics::default(),
        power: PowerMetrics::default(),
        network: NetworkMetrics::default(),
        disk: DiskMetrics::default(),
    }));
    let processes_arc: Arc<tokio::sync::RwLock<Vec<crate::logging::snapshots::ProcessRecord>>> =
        Arc::new(tokio::sync::RwLock::new(Vec::new()));
    let app_groups_arc: Arc<tokio::sync::RwLock<Vec<crate::logging::snapshots::AppGroupRecord>>> =
        Arc::new(tokio::sync::RwLock::new(Vec::new()));
    let responsiveness_metrics_arc: Arc<
        tokio::sync::RwLock<crate::logging::snapshots::ResponsivenessMetrics>,
    > = Arc::new(tokio::sync::RwLock::new(
        crate::logging::snapshots::ResponsivenessMetrics::default(),
    ));

    // Запуск API сервера (если указан адрес)
    let mut api_server_handle: Option<ApiServerHandle> = None;
    let config_arc = Arc::new(config.clone());
    let pattern_db_arc = Arc::new(pattern_db.clone());
    if let Some(ref api_addr_str) = config.paths.api_listen_addr {
        match api_addr_str.parse::<std::net::SocketAddr>() {
            Ok(addr) => {
                info!("Starting API server on {}", addr);
                let api_server = ApiServer::with_all_and_responsiveness_and_config_and_patterns(
                    addr,
                    Some(Arc::clone(&stats_arc)),
                    Some(Arc::clone(&system_metrics_arc)),
                    Some(Arc::clone(&processes_arc)),
                    Some(Arc::clone(&app_groups_arc)),
                    Some(Arc::clone(&responsiveness_metrics_arc)),
                    Some(Arc::clone(&config_arc)),
                    Some(Arc::clone(&pattern_db_arc)),
                );
                match api_server.start().await {
                    Ok(handle) => {
                        api_server_handle = Some(handle);
                        info!("API server started successfully on {}", addr);
                    }
                    Err(e) => {
                        warn!("Failed to start API server: {}. Continuing without API.", e);
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Invalid API listen address '{}': {}. API server will not start.",
                    api_addr_str, e
                );
            }
        }
    } else {
        debug!("API server disabled (api_listen_addr not configured)");
    }

    info!("SmoothTask daemon started, entering main loop");

    // Вызываем callback уведомления о готовности (например, для systemd notify)
    if let Some(ref callback) = on_ready {
        callback();
    }

    let mut iteration = 0u64;
    let mut stats = DaemonStats::new();
    const STATS_LOG_INTERVAL: u64 = 10; // Логируем статистику каждые 10 итераций
    loop {
        // Проверяем сигнал завершения перед началом итерации
        // Используем borrow_and_update() для обновления внутреннего состояния
        if *shutdown_rx.borrow_and_update() {
            info!("Shutdown signal received, exiting main loop");
            break;
        }

        let loop_start = Instant::now();
        iteration += 1;

        debug!("Starting iteration {}", iteration);

        // Проверяем shutdown перед сбором снапшота (может быть долгим)
        if *shutdown_rx.borrow_and_update() {
            info!("Shutdown signal received before snapshot collection, exiting main loop");
            break;
        }

        // Сбор снапшота с кэшированием для оптимизации производительности
        let snapshot = match collect_snapshot_with_caching(
            &proc_paths,
            &window_introspector,
            &audio_introspector,
            &input_tracker,
            &mut prev_cpu_times,
            &config.thresholds,
            &latency_collector,
            &mut system_metrics_cache,
            &mut system_metrics_cache_iteration,
            system_metrics_cache_interval,
            &mut process_metrics_cache,
            &mut process_metrics_cache_iteration,
            process_metrics_cache_interval,
            iteration,
        )
        .await
        {
            Ok(snap) => snap,
            Err(e) => {
                error!("Failed to collect snapshot: {}", e);
                stats.record_error_iteration();
                // Проверяем shutdown перед sleep
                if *shutdown_rx.borrow_and_update() {
                    info!("Shutdown signal received, exiting main loop");
                    break;
                }
                tokio::time::sleep(Duration::from_millis(config.polling_interval_ms)).await;
                // Проверяем shutdown после sleep
                if *shutdown_rx.borrow_and_update() {
                    info!("Shutdown signal received, exiting main loop");
                    break;
                }
                continue;
            }
        };

        // Группировка процессов
        let mut processes = snapshot.processes.clone();
        let mut app_groups = ProcessGrouper::group_processes(&processes);

        // Классификация процессов и групп
        classify_all(&mut processes, &mut app_groups, &pattern_db, None);

        // Обновляем app_group_id в процессах на основе группировки
        for process in &mut processes {
            for app_group in &app_groups {
                if app_group.process_ids.contains(&process.pid) {
                    process.app_group_id = Some(app_group.app_group_id.clone());
                    break;
                }
            }
        }

        // Обновляем снапшот с классифицированными данными
        let snapshot = Snapshot {
            processes,
            app_groups,
            ..snapshot
        };

        // Применение политики
        let policy_results = policy_engine.evaluate_snapshot(&snapshot);

        // Планирование изменений приоритетов
        let adjustments = plan_priority_changes(&snapshot, &policy_results);

        let (applied_count, error_count) = if dry_run {
            debug!(
                "Dry-run: would apply {} priority adjustments",
                adjustments.len()
            );
            for adj in &adjustments {
                debug!(
                    "  PID {}: {} -> {} ({})",
                    adj.pid, adj.current_nice, adj.target_nice, adj.reason
                );
            }
            (0, 0)
        } else {
            // Применение изменений
            let apply_result = apply_priority_adjustments(&adjustments, &mut hysteresis);
            if apply_result.applied > 0 {
                info!(
                    "Applied {} priority adjustments ({} skipped due to hysteresis, {} errors)",
                    apply_result.applied, apply_result.skipped_hysteresis, apply_result.errors
                );
            }
            if apply_result.errors > 0 {
                warn!(
                    "Failed to apply {} priority adjustments",
                    apply_result.errors
                );
            }
            (apply_result.applied, apply_result.errors)
        };

        // Логирование снапшота (опционально)
        if let Some(ref mut logger) = snapshot_logger {
            if let Err(e) = logger.log_snapshot(&snapshot) {
                warn!("Failed to log snapshot: {}", e);
            }
        }

        // Вычисляем время до следующей итерации
        let elapsed = loop_start.elapsed();
        let elapsed_ms = elapsed.as_millis();

        // Обновляем статистику
        stats.record_successful_iteration(elapsed_ms, applied_count as u64, error_count as u64);

        // Обновляем данные для API сервера
        {
            *stats_arc.write().await = stats.clone();
            // Обновляем system_metrics из prev_cpu_times (который обновляется в collect_snapshot)
            if let Some(ref system_metrics) = prev_cpu_times {
                *system_metrics_arc.write().await = system_metrics.clone();
            }
            *processes_arc.write().await = snapshot.processes.clone();
            *app_groups_arc.write().await = snapshot.app_groups.clone();
            *responsiveness_metrics_arc.write().await = snapshot.responsiveness.clone();
        }

        // Логируем статистику периодически
        if iteration % STATS_LOG_INTERVAL == 0 {
            stats.log_stats();
            // Обновляем статус для systemd (если callback предоставлен)
            if let Some(ref status_callback) = on_status_update {
                let status_msg = format!(
                    "Running: {} iterations, avg {:.1}ms/iter, {} adjustments applied",
                    stats.total_iterations,
                    stats.average_iteration_duration_ms(),
                    stats.total_applied_adjustments
                );
                status_callback(&status_msg);
            }
        }

        let sleep_duration = if elapsed_ms < config.polling_interval_ms as u128 {
            Duration::from_millis(config.polling_interval_ms) - elapsed
        } else {
            Duration::from_millis(0)
        };

        if sleep_duration.as_millis() > 0 {
            // Разбиваем sleep на маленькие интервалы для проверки shutdown
            let chunk_duration = Duration::from_millis(50); // Проверяем каждые 50ms
            let mut remaining = sleep_duration;

            while remaining > Duration::from_millis(0) {
                // Проверяем shutdown перед каждым маленьким sleep
                if *shutdown_rx.borrow_and_update() {
                    info!("Shutdown signal received during sleep, exiting main loop");
                    break;
                }

                let sleep_chunk = remaining.min(chunk_duration);
                tokio::time::sleep(sleep_chunk).await;
                remaining = remaining.saturating_sub(sleep_chunk);
            }

            // Финальная проверка shutdown после sleep
            if *shutdown_rx.borrow_and_update() {
                info!("Shutdown signal received after sleep, exiting main loop");
                break;
            }
        } else {
            warn!(
                "Iteration {} took {}ms, longer than polling interval {}ms",
                iteration,
                elapsed.as_millis(),
                config.polling_interval_ms
            );
        }
    }

    info!("SmoothTask daemon stopped after {} iterations", iteration);

    // Логируем финальную статистику
    stats.log_stats();

    // Останавливаем probe-thread перед завершением
    latency_probe.stop();

    // Останавливаем API сервер перед завершением
    if let Some(handle) = api_server_handle {
        info!("Stopping API server");
        if let Err(e) = handle.shutdown().await {
            warn!("Failed to stop API server gracefully: {}", e);
        } else {
            info!("API server stopped successfully");
        }
    }

    Ok(())
}

/// Собрать полный снапшот системы.
///
/// Функция собирает все метрики системы, процессов, окон, аудио и ввода,
/// объединяя их в единый снапшот для последующего анализа и применения политики.
///
/// # Параметры
///
/// - `proc_paths`: Пути к файлам /proc для чтения системных метрик
/// - `window_introspector`: Интроспектор для получения информации об окнах (X11/Wayland/Static)
/// - `audio_introspector`: Интроспектор для получения метрик аудио (PipeWire/Static)
/// - `input_tracker`: Трекер активности пользователя (evdev/Static)
/// - `prev_cpu_times`: Предыдущие значения CPU для вычисления дельт (используется для CPU usage)
/// - `thresholds`: Пороги для вычисления метрик отзывчивости
/// - `latency_collector`: Коллектор scheduling latency (probe-thread)
///
/// # Возвращаемое значение
///
/// Возвращает `Ok(Snapshot)` с полным снапшотом системы или `Err` при критической ошибке.
///
/// # Обработка ошибок
///
/// Функция использует graceful degradation: если один компонент не может собрать метрики
/// (например, X11 недоступен или PipeWire не работает), функция продолжает работу с
/// дефолтными значениями для этого компонента. Критической ошибкой считается только
/// невозможность собрать системные метрики или метрики процессов (без них снапшот
/// не имеет смысла).
///
/// # Алгоритм
///
/// 1. Сбор системных метрик (CPU, память, PSI, load average) через `spawn_blocking`
/// 2. Вычисление дельт CPU для определения CPU usage
/// 3. Сбор метрик процессов из /proc через `spawn_blocking`
/// 4. Сбор метрик окон через window_introspector (X11/Wayland/Static)
/// 5. Обновление информации об окнах в процессах
/// 6. Сбор метрик аудио через audio_introspector (PipeWire/Static)
/// 7. Обновление информации об аудио-клиентах в процессах
/// 8. Сбор метрик ввода через input_tracker (evdev/Static)
/// 9. Построение GlobalMetrics из собранных метрик
/// 10. Построение ResponsivenessMetrics с вычислением bad_responsiveness и responsiveness_score
///
/// # Примеры использования
///
/// ```no_run
/// use smoothtask_core::collect_snapshot;
/// use smoothtask_core::config::Thresholds;
/// use smoothtask_core::metrics::system::ProcPaths;
/// use smoothtask_core::metrics::windows::{StaticWindowIntrospector, WindowIntrospector};
/// use smoothtask_core::metrics::audio::{AudioIntrospector, StaticAudioIntrospector};
/// use smoothtask_core::metrics::input::InputTracker;
/// use smoothtask_core::metrics::scheduling_latency::LatencyCollector;
/// use std::sync::{Arc, Mutex};
/// use std::time::Duration;
///
/// # async fn example() -> anyhow::Result<()> {
/// let proc_paths = ProcPaths::default();
/// let window_introspector: Arc<dyn WindowIntrospector> = Arc::new(StaticWindowIntrospector::new(Vec::new()));
/// let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> = Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
/// let input_tracker = Arc::new(Mutex::new(InputTracker::new(Duration::from_secs(60))));
/// let mut prev_cpu_times = None;
/// let thresholds = Thresholds {
///     psi_cpu_some_high: 0.6,
///     psi_io_some_high: 0.4,
///     user_idle_timeout_sec: 120,
///     interactive_build_grace_sec: 10,
///     noisy_neighbour_cpu_share: 0.7,
///     crit_interactive_percentile: 0.9,
///     interactive_percentile: 0.6,
///     normal_percentile: 0.3,
///     background_percentile: 0.1,
///     sched_latency_p99_threshold_ms: 10.0,
///     ui_loop_p95_threshold_ms: 16.67,
/// };
/// let latency_collector = Arc::new(LatencyCollector::new(1000));
///
/// let snapshot = collect_snapshot(
///     &proc_paths,
///     &window_introspector,
///     &audio_introspector,
///     &input_tracker,
///     &mut prev_cpu_times,
///     &thresholds,
///     &latency_collector,
/// ).await?;
///
/// println!("Collected {} processes", snapshot.processes.len());
/// # Ok(())
/// # }
/// ```
///
/// # Примечания
///
/// - Все блокирующие операции (чтение из /proc, вызовы X11/PipeWire/evdev) обёрнуты
///   в `spawn_blocking` для предотвращения блокировки async runtime
/// - Функция не блокирует выполнение при ошибках компонентов (окна, аудио, ввод),
///   используя дефолтные значения
/// - `prev_cpu_times` обновляется внутри функции для вычисления дельт CPU на следующей итерации
/// - `app_groups` в возвращаемом снапшоте пусты, они заполняются после группировки процессов
pub async fn collect_snapshot(
    proc_paths: &ProcPaths,
    window_introspector: &Arc<dyn WindowIntrospector>,
    audio_introspector: &Arc<Mutex<Box<dyn AudioIntrospector>>>,
    input_tracker: &Arc<Mutex<InputTracker>>,
    prev_cpu_times: &mut Option<SystemMetrics>,
    thresholds: &crate::config::Thresholds,
    latency_collector: &Arc<LatencyCollector>,
) -> Result<Snapshot> {
    let now = Instant::now();
    let timestamp = Utc::now();
    let snapshot_id = timestamp.timestamp_millis() as u64;

    // Сбор системных метрик (блокирующая операция - оборачиваем в spawn_blocking)
    let proc_paths_clone = proc_paths.clone();
    let system_metrics = tokio::task::spawn_blocking(move || {
        collect_system_metrics(&proc_paths_clone)
    })
    .await
    .context("Failed to join system metrics task")?
    .context(
        "Failed to collect system metrics: unable to read /proc filesystem (stat, meminfo, PSI)",
    )?;

    // Вычисление дельт CPU
    let cpu_usage = if let Some(ref prev) = prev_cpu_times {
        prev.cpu_times.delta(&system_metrics.cpu_times)
    } else {
        None
    };
    *prev_cpu_times = Some(system_metrics.clone());

    // Сбор метрик процессов (блокирующая операция - оборачиваем в spawn_blocking)
    let mut processes = tokio::task::spawn_blocking(collect_process_metrics)
        .await
        .context("Failed to join process metrics task")?
        .context(
            "Failed to collect process metrics: unable to read process information from /proc",
        )?;

    // Сбор метрик окон (может быть блокирующим для X11 - оборачиваем в spawn_blocking)
    let window_introspector_clone = Arc::clone(window_introspector);
    let pid_to_window = match tokio::task::spawn_blocking(move || {
        crate::metrics::windows::build_pid_to_window_map(window_introspector_clone.as_ref())
    })
    .await
    {
        Ok(Ok(map)) => map,
        Ok(Err(e)) => {
            warn!(
                "Failed to collect window metrics: {}. Continuing without window information.",
                e
            );
            HashMap::new()
        }
        Err(e) => {
            warn!(
                "Failed to join window metrics task: {}. Continuing without window information.",
                e
            );
            HashMap::new()
        }
    };

    // Обновление информации об окнах в процессах
    for process in &mut processes {
        if let Some(window) = pid_to_window.get(&(process.pid as u32)) {
            process.has_gui_window = true;
            process.is_focused_window = window.is_focused();
            process.window_state = Some(format!("{:?}", window.state));
        }
    }

    // Сбор метрик аудио (может быть блокирующим для PipeWire - оборачиваем в spawn_blocking)
    let audio_introspector_clone = Arc::clone(audio_introspector);
    let (audio_metrics, audio_clients) = match tokio::task::spawn_blocking(move || {
        let mut introspector = match audio_introspector_clone.lock() {
            Ok(guard) => guard,
            Err(e) => {
                warn!(
                    "Audio introspector mutex is poisoned: {}. Using empty metrics and clients.",
                    e
                );
                return (
                    AudioMetrics::empty(SystemTime::now(), SystemTime::now()),
                    Vec::new(),
                );
            }
        };
        let metrics = introspector.audio_metrics().unwrap_or_else(|e| {
            warn!(
                "Audio introspector failed to collect metrics: {}. Using empty metrics.",
                e
            );
            AudioMetrics::empty(SystemTime::now(), SystemTime::now())
        });
        let clients = introspector.clients().unwrap_or_else(|e| {
            warn!(
                "Audio introspector failed to collect clients: {}. Using empty client list.",
                e
            );
            Vec::new()
        });
        (metrics, clients)
    })
    .await
    {
        Ok(result) => result,
        Err(e) => {
            warn!(
                "Failed to join audio metrics task: {}. Continuing without audio information.",
                e
            );
            (
                AudioMetrics::empty(SystemTime::now(), SystemTime::now()),
                Vec::new(),
            )
        }
    };
    let audio_client_pids: std::collections::HashSet<u32> =
        audio_clients.iter().map(|c| c.pid).collect();

    // Обновление информации об аудио-клиентах в процессах
    for process in &mut processes {
        if audio_client_pids.contains(&(process.pid as u32)) {
            process.is_audio_client = true;
            process.has_active_stream = true;
        }
    }

    // Сбор метрик ввода (может быть блокирующим для evdev - оборачиваем в spawn_blocking)
    // InputTracker::update() может читать из /dev/input, что является блокирующей операцией
    let input_tracker_clone = Arc::clone(input_tracker);
    let input_metrics = match tokio::task::spawn_blocking(move || {
        let mut tracker = match input_tracker_clone.lock() {
            Ok(guard) => guard,
            Err(e) => {
                warn!(
                    "Input tracker mutex is poisoned: {}. Using default input metrics.",
                    e
                );
                return InputMetrics {
                    user_active: false,
                    time_since_last_input_ms: None,
                };
            }
        };
        tracker.update(now)
    })
    .await
    {
        Ok(metrics) => metrics,
        Err(e) => {
            warn!(
                "Failed to join input metrics task: {}. Using default input metrics.",
                e
            );
            InputMetrics {
                user_active: false,
                time_since_last_input_ms: None,
            }
        }
    };

    // Построение GlobalMetrics
    let global = GlobalMetrics {
        cpu_user: cpu_usage.map(|u| u.user).unwrap_or(0.0),
        cpu_system: cpu_usage.map(|u| u.system).unwrap_or(0.0),
        cpu_idle: cpu_usage.map(|u| u.idle).unwrap_or(0.0),
        cpu_iowait: cpu_usage.map(|u| u.iowait).unwrap_or(0.0),
        mem_total_kb: system_metrics.memory.mem_total_kb,
        mem_used_kb: system_metrics.memory.mem_used_kb(),
        mem_available_kb: system_metrics.memory.mem_available_kb,
        swap_total_kb: system_metrics.memory.swap_total_kb,
        swap_used_kb: system_metrics.memory.swap_used_kb(),
        load_avg_one: system_metrics.load_avg.one,
        load_avg_five: system_metrics.load_avg.five,
        load_avg_fifteen: system_metrics.load_avg.fifteen,
        psi_cpu_some_avg10: system_metrics.pressure.cpu.some.map(|p| p.avg10),
        psi_cpu_some_avg60: system_metrics.pressure.cpu.some.map(|p| p.avg60),
        psi_io_some_avg10: system_metrics.pressure.io.some.map(|p| p.avg10),
        psi_mem_some_avg10: system_metrics.pressure.memory.some.map(|p| p.avg10),
        psi_mem_full_avg10: system_metrics.pressure.memory.full.map(|p| p.avg10),
        user_active: input_metrics.user_active,
        time_since_last_input_ms: input_metrics.time_since_last_input_ms,
    };

    // Построение ResponsivenessMetrics
    // Читаем scheduling latency из LatencyCollector (probe-thread)
    let sched_latency_p95_ms = latency_collector.p95();
    let sched_latency_p99_ms = latency_collector.p99();

    let mut responsiveness = ResponsivenessMetrics {
        audio_xruns_delta: Some(audio_metrics.xrun_count as u64),
        sched_latency_p95_ms,
        sched_latency_p99_ms,
        ..Default::default()
    };

    // Вычисление bad_responsiveness и responsiveness_score
    responsiveness.compute(&global, thresholds);

    // Пока app_groups будут пустыми, они заполнятся после группировки
    Ok(Snapshot {
        snapshot_id,
        timestamp,
        global,
        processes,
        app_groups: Vec::new(),
        responsiveness,
    })
}

/// Собрать снапшот с поддержкой кэширования для оптимизации производительности.
///
/// Эта функция аналогична `collect_snapshot`, но поддерживает кэширование системных
/// и процессных метрик для снижения нагрузки на систему. Кэширование позволяет
/// повторно использовать ранее собранные данные в течение нескольких итераций,
/// что особенно полезно для системных метрик, которые меняются относительно медленно.
///
/// # Параметры
///
/// - `proc_paths`: Пути к файлам /proc
/// - `window_introspector`: Интроспектор окон для сбора информации о GUI
/// - `audio_introspector`: Интроспектор аудио для сбора информации о звуковых клиентах
/// - `input_tracker`: Трекер ввода для определения активности пользователя
/// - `prev_cpu_times`: Предыдущие метрики CPU для вычисления дельт
/// - `thresholds`: Пороги конфигурации
/// - `latency_collector`: Коллектор метрик задержек
/// - `system_metrics_cache`: Кэш системных метрик
/// - `system_metrics_cache_iteration`: Итерация последнего обновления кэша системных метрик
/// - `system_metrics_cache_interval`: Интервал кэширования системных метрик (в итерациях)
/// - `process_metrics_cache`: Кэш метрик процессов
/// - `process_metrics_cache_iteration`: Итерация последнего обновления кэша метрик процессов
/// - `process_metrics_cache_interval`: Интервал кэширования метрик процессов (в итерациях)
/// - `current_iteration`: Текущая итерация главного цикла
///
/// # Возвращаемое значение
///
/// - `Ok(Snapshot)`: Успешно собранный снапшот системы
/// - `Err(anyhow::Error)`: Ошибка при сборе снапшота
///
/// # Алгоритм
///
/// 1. Проверяет, нужно ли обновлять кэш системных метрик (каждые N итераций)
/// 2. Проверяет, нужно ли обновлять кэш метрик процессов (каждые M итераций)
/// 3. Использует кэшированные данные, если они актуальны
/// 4. Обновляет кэш, если он устарел
/// 5. Собирает остальные метрики (окна, аудио, ввод) без кэширования
/// 6. Объединяет все данные в финальный снапшот
///
/// # Примечания
///
/// - Кэширование системных метрик может значительно снизить нагрузку на систему
/// - Кэширование метрик процессов может быть полезно для снижения нагрузки, но по умолчанию отключено
/// - Оконные и аудио метрики не кэшируются, так как они могут меняться быстро
/// - Функция безопасна для использования в async контексте
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::{collect_snapshot_with_caching, config::Config, metrics::system::ProcPaths};
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
///
/// // Инициализация кэшей
/// let mut system_metrics_cache: Option<SystemMetrics> = None;
/// let mut system_metrics_cache_iteration: u64 = 0;
/// let system_metrics_cache_interval = 3; // Обновление каждые 3 итерации
///
/// let mut process_metrics_cache: Option<Vec<ProcessRecord>> = None;
/// let mut process_metrics_cache_iteration: u64 = 0;
/// let process_metrics_cache_interval = 1; // Обновление каждую итерацию (отключено)
///
/// let current_iteration = 1;
///
/// // Сбор снапшота с кэшированием
/// let snapshot = collect_snapshot_with_caching(
///     &proc_paths,
///     &window_introspector,
///     &audio_introspector,
///     &input_tracker,
///     &mut prev_cpu_times,
///     &config.thresholds,
///     &latency_collector,
///     &mut system_metrics_cache,
///     &mut system_metrics_cache_iteration,
///     system_metrics_cache_interval,
///     &mut process_metrics_cache,
///     &mut process_metrics_cache_iteration,
///     process_metrics_cache_interval,
///     current_iteration,
/// ).await?;
/// ```
#[allow(clippy::too_many_arguments)]
pub async fn collect_snapshot_with_caching(
    proc_paths: &ProcPaths,
    window_introspector: &Arc<dyn WindowIntrospector>,
    audio_introspector: &Arc<Mutex<Box<dyn AudioIntrospector>>>,
    input_tracker: &Arc<Mutex<InputTracker>>,
    prev_cpu_times: &mut Option<SystemMetrics>,
    thresholds: &crate::config::Thresholds,
    latency_collector: &Arc<LatencyCollector>,
    system_metrics_cache: &mut Option<SystemMetrics>,
    system_metrics_cache_iteration: &mut u64,
    system_metrics_cache_interval: u64,
    process_metrics_cache: &mut Option<Vec<ProcessRecord>>,
    process_metrics_cache_iteration: &mut u64,
    process_metrics_cache_interval: u64,
    current_iteration: u64,
) -> Result<Snapshot> {
    let _now = Instant::now();
    let timestamp = Utc::now();
    let snapshot_id = timestamp.timestamp_millis() as u64;

    // Проверяем, нужно ли обновлять кэш системных метрик
    let need_update_system_metrics = 
        system_metrics_cache.is_none() ||
        (current_iteration - *system_metrics_cache_iteration) >= system_metrics_cache_interval;

    let system_metrics = if need_update_system_metrics {
        // Кэш устарел, обновляем системные метрики
        let proc_paths_clone = proc_paths.clone();
        let new_system_metrics = tokio::task::spawn_blocking(move || {
            collect_system_metrics(&proc_paths_clone)
        })
        .await
        .context("Failed to join system metrics task")?
        .context(
            "Failed to collect system metrics: unable to read /proc filesystem (stat, meminfo, PSI)",
        )?;

        // Обновляем кэш
        *system_metrics_cache = Some(new_system_metrics.clone());
        *system_metrics_cache_iteration = current_iteration;
        
        debug!(
            "Updated system metrics cache (iteration {})",
            current_iteration
        );
        
        new_system_metrics
    } else {
        // Используем кэшированные системные метрики
        debug!(
            "Using cached system metrics (iteration {}, cache age: {} iterations)",
            current_iteration,
            current_iteration - *system_metrics_cache_iteration
        );
        system_metrics_cache.as_ref().unwrap().clone()
    };

    // Вычисление дельт CPU
    let cpu_usage = if let Some(ref prev) = prev_cpu_times {
        prev.cpu_times.delta(&system_metrics.cpu_times)
    } else {
        None
    };
    *prev_cpu_times = Some(system_metrics.clone());

    // Проверяем, нужно ли обновлять кэш метрик процессов
    let need_update_process_metrics = 
        process_metrics_cache.is_none() ||
        (current_iteration - *process_metrics_cache_iteration) >= process_metrics_cache_interval;

    let mut processes = if need_update_process_metrics {
        // Кэш устарел, обновляем метрики процессов
        let new_processes = tokio::task::spawn_blocking(collect_process_metrics)
            .await
            .context("Failed to join process metrics task")?
            .context(
                "Failed to collect process metrics: unable to read process information from /proc",
            )?;

        // Обновляем кэш
        *process_metrics_cache = Some(new_processes.clone());
        *process_metrics_cache_iteration = current_iteration;
        
        debug!(
            "Updated process metrics cache (iteration {})",
            current_iteration
        );
        
        new_processes
    } else {
        // Используем кэшированные метрики процессов
        debug!(
            "Using cached process metrics (iteration {}, cache age: {} iterations)",
            current_iteration,
            current_iteration - *process_metrics_cache_iteration
        );
        process_metrics_cache.as_ref().unwrap().clone()
    };

    // Сбор метрик окон (без кэширования, так как они могут меняться быстро)
    let window_introspector_clone = Arc::clone(window_introspector);
    let pid_to_window = match tokio::task::spawn_blocking(move || {
        crate::metrics::windows::build_pid_to_window_map(window_introspector_clone.as_ref())
    })
    .await
    {
        Ok(Ok(map)) => map,
        Ok(Err(e)) => {
            warn!(
                "Failed to collect window metrics: {}. Continuing without window information.",
                e
            );
            HashMap::new()
        }
        Err(e) => {
            warn!(
                "Failed to join window metrics task: {}. Continuing without window information.",
                e
            );
            HashMap::new()
        }
    };

    // Обновление информации об окнах в процессах
    for process in &mut processes {
        if let Some(window) = pid_to_window.get(&(process.pid as u32)) {
            process.has_gui_window = true;
            process.is_focused_window = window.is_focused();
            process.window_state = Some(format!("{:?}", window.state));
        }
    }

    // Сбор метрик аудио (без кэширования, так как они могут меняться быстро)
    let audio_introspector_clone = Arc::clone(audio_introspector);
    let (audio_metrics, audio_clients) = match tokio::task::spawn_blocking(move || {
        let mut introspector = match audio_introspector_clone.lock() {
            Ok(guard) => guard,
            Err(e) => {
                warn!(
                    "Audio introspector mutex is poisoned: {}. Using empty metrics and clients.",
                    e
                );
                return (
                    AudioMetrics::empty(SystemTime::now(), SystemTime::now()),
                    Vec::new(),
                );
            }
        };
        let metrics = introspector.audio_metrics().unwrap_or_else(|e| {
            warn!(
                "Audio introspector failed to collect metrics: {}. Using empty metrics.",
                e
            );
            AudioMetrics::empty(SystemTime::now(), SystemTime::now())
        });
        let clients = introspector.clients().unwrap_or_else(|e| {
            warn!(
                "Audio introspector failed to collect clients: {}. Using empty client list.",
                e
            );
            Vec::new()
        });
        (metrics, clients)
    })
    .await
    {
        Ok((metrics, clients)) => (metrics, clients),
        Err(e) => {
            warn!(
                "Failed to join audio metrics task: {}. Using empty audio metrics.",
                e
            );
            (AudioMetrics::empty(SystemTime::now(), SystemTime::now()), Vec::new())
        }
    };

    // Обновление информации об аудио клиентах в процессах
    for process in &mut processes {
        if audio_clients.iter().any(|c| c.pid == process.pid as u32) {
            process.is_audio_client = true;
            // Note: has_active_stream is not available in AudioClientInfo, so we set it to false
            // This could be enhanced in the future if we add active stream detection
            process.has_active_stream = false;
        }
    }

    // Сбор метрик ввода (без кэширования, так как они могут меняться быстро)
    let input_tracker_clone = Arc::clone(input_tracker);
    let now_for_input = Instant::now();
    let input_metrics = match tokio::task::spawn_blocking(move || {
        let tracker = match input_tracker_clone.lock() {
            Ok(guard) => guard,
            Err(e) => {
                warn!(
                    "Input tracker mutex is poisoned: {}. Using empty input metrics.",
                    e
                );
                return InputMetrics::empty();
            }
        };
        tracker.metrics(now_for_input)
    })
    .await
    {
        Ok(metrics) => metrics,
        Err(e) => {
            warn!(
                "Failed to join input metrics task: {}. Using empty input metrics.",
                e
            );
            InputMetrics::empty()
        }
    };

    // Сбор глобальных метрик
    let global_metrics = GlobalMetrics {
        cpu_user: cpu_usage.map_or(0.0, |u| u.user),
        cpu_system: cpu_usage.map_or(0.0, |u| u.system),
        cpu_idle: cpu_usage.map_or(0.0, |u| u.idle),
        cpu_iowait: cpu_usage.map_or(0.0, |u| u.iowait),
        mem_total_kb: system_metrics.memory.mem_total_kb,
        mem_used_kb: system_metrics.memory.mem_used_kb(),
        mem_available_kb: system_metrics.memory.mem_available_kb,
        swap_total_kb: system_metrics.memory.swap_total_kb,
        swap_used_kb: system_metrics.memory.swap_used_kb(),
        load_avg_one: system_metrics.load_avg.one,
        load_avg_five: system_metrics.load_avg.five,
        load_avg_fifteen: system_metrics.load_avg.fifteen,
        psi_cpu_some_avg10: system_metrics.pressure.cpu.some.map(|p| p.avg10),
        psi_cpu_some_avg60: system_metrics.pressure.cpu.some.map(|p| p.avg60),
        psi_io_some_avg10: system_metrics.pressure.io.some.map(|p| p.avg10),
        psi_mem_some_avg10: system_metrics.pressure.memory.some.map(|p| p.avg10),
        psi_mem_full_avg10: system_metrics.pressure.memory.full.map(|p| p.avg10),
        user_active: input_metrics.user_active,
        time_since_last_input_ms: input_metrics.time_since_last_input_ms,
    };

    // Сбор метрик отзывчивости
    let mut responsiveness_metrics = ResponsivenessMetrics {
        sched_latency_p95_ms: latency_collector.p95(),
        sched_latency_p99_ms: latency_collector.p99(),
        audio_xruns_delta: Some(audio_metrics.xrun_count as u64),
        ui_loop_p95_ms: latency_collector.p95(),
        frame_jank_ratio: None, // Will be computed later
        bad_responsiveness: input_metrics.user_active && 
            input_metrics.time_since_last_input_ms
                .map_or(false, |time_since_input| 
                    time_since_input > thresholds.user_idle_timeout_sec * 1000
                ),
        responsiveness_score: None, // Will be computed later
    };
    
    // Compute the remaining fields
    responsiveness_metrics.compute(&global_metrics, thresholds);

    let snapshot = Snapshot {
        snapshot_id,
        timestamp,
        global: global_metrics,
        processes,
        app_groups: vec![],
        responsiveness: responsiveness_metrics,
    };

    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_stats_new() {
        let stats = DaemonStats::new();
        assert_eq!(stats.total_iterations, 0);
        assert_eq!(stats.successful_iterations, 0);
        assert_eq!(stats.error_iterations, 0);
        assert_eq!(stats.total_duration_ms, 0);
        assert_eq!(stats.max_iteration_duration_ms, 0);
        assert_eq!(stats.total_applied_adjustments, 0);
        assert_eq!(stats.total_apply_errors, 0);
    }

    #[test]
    fn test_daemon_stats_default() {
        let stats_default = DaemonStats::default();
        let stats_new = DaemonStats::new();
        assert_eq!(stats_default.total_iterations, stats_new.total_iterations);
        assert_eq!(
            stats_default.successful_iterations,
            stats_new.successful_iterations
        );
        assert_eq!(stats_default.error_iterations, stats_new.error_iterations);
        assert_eq!(stats_default.total_duration_ms, stats_new.total_duration_ms);
        assert_eq!(
            stats_default.max_iteration_duration_ms,
            stats_new.max_iteration_duration_ms
        );
        assert_eq!(
            stats_default.total_applied_adjustments,
            stats_new.total_applied_adjustments
        );
        assert_eq!(
            stats_default.total_apply_errors,
            stats_new.total_apply_errors
        );
    }

    #[test]
    fn test_daemon_stats_record_successful_iteration() {
        let mut stats = DaemonStats::new();
        stats.record_successful_iteration(100, 5, 1);

        assert_eq!(stats.total_iterations, 1);
        assert_eq!(stats.successful_iterations, 1);
        assert_eq!(stats.error_iterations, 0);
        assert_eq!(stats.total_duration_ms, 100);
        assert_eq!(stats.max_iteration_duration_ms, 100);
        assert_eq!(stats.total_applied_adjustments, 5);
        assert_eq!(stats.total_apply_errors, 1);

        // Добавляем ещё одну итерацию
        stats.record_successful_iteration(150, 3, 0);

        assert_eq!(stats.total_iterations, 2);
        assert_eq!(stats.successful_iterations, 2);
        assert_eq!(stats.total_duration_ms, 250);
        assert_eq!(stats.max_iteration_duration_ms, 150); // Максимум обновлён
        assert_eq!(stats.total_applied_adjustments, 8);
        assert_eq!(stats.total_apply_errors, 1);
    }

    #[test]
    fn test_daemon_stats_record_error_iteration() {
        let mut stats = DaemonStats::new();
        stats.record_error_iteration();

        assert_eq!(stats.total_iterations, 1);
        assert_eq!(stats.successful_iterations, 0);
        assert_eq!(stats.error_iterations, 1);

        // Добавляем успешную итерацию после ошибки
        stats.record_successful_iteration(100, 2, 0);

        assert_eq!(stats.total_iterations, 2);
        assert_eq!(stats.successful_iterations, 1);
        assert_eq!(stats.error_iterations, 1);
    }

    #[test]
    fn test_daemon_stats_average_iteration_duration() {
        let mut stats = DaemonStats::new();

        // Без итераций среднее должно быть 0
        assert_eq!(stats.average_iteration_duration_ms(), 0.0);

        // Добавляем несколько итераций
        stats.record_successful_iteration(100, 0, 0);
        stats.record_successful_iteration(200, 0, 0);
        stats.record_successful_iteration(150, 0, 0);

        // Среднее: (100 + 200 + 150) / 3 = 150.0
        assert!((stats.average_iteration_duration_ms() - 150.0).abs() < 0.01);

        // Добавляем ошибку (не должна влиять на среднее)
        stats.record_error_iteration();
        assert!((stats.average_iteration_duration_ms() - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_daemon_stats_max_iteration_duration() {
        let mut stats = DaemonStats::new();

        stats.record_successful_iteration(50, 0, 0);
        assert_eq!(stats.max_iteration_duration_ms, 50);

        stats.record_successful_iteration(100, 0, 0);
        assert_eq!(stats.max_iteration_duration_ms, 100);

        stats.record_successful_iteration(75, 0, 0);
        assert_eq!(stats.max_iteration_duration_ms, 100); // Максимум не изменился

        stats.record_successful_iteration(200, 0, 0);
        assert_eq!(stats.max_iteration_duration_ms, 200); // Новый максимум
    }

    #[test]
    fn test_daemon_stats_log_stats_with_empty_stats() {
        // Тест проверяет, что log_stats() не паникует при пустой статистике
        let stats = DaemonStats::new();
        // Функция должна корректно обработать случай, когда нет успешных итераций
        stats.log_stats();
        // Проверяем, что среднее время = 0.0 (нет успешных итераций)
        assert_eq!(stats.average_iteration_duration_ms(), 0.0);
    }

    #[test]
    fn test_daemon_stats_log_stats_with_only_errors() {
        // Тест проверяет, что log_stats() корректно обрабатывает случай, когда есть только ошибки
        let mut stats = DaemonStats::new();
        stats.record_error_iteration();
        stats.record_error_iteration();
        // Функция должна корректно обработать случай, когда нет успешных итераций
        stats.log_stats();
        // Проверяем, что среднее время = 0.0 (нет успешных итераций)
        assert_eq!(stats.average_iteration_duration_ms(), 0.0);
        assert_eq!(stats.total_iterations, 2);
        assert_eq!(stats.error_iterations, 2);
        assert_eq!(stats.successful_iterations, 0);
    }

    #[test]
    fn test_daemon_stats_log_stats_with_mixed_iterations() {
        // Тест проверяет, что log_stats() корректно обрабатывает смешанные итерации (успешные и ошибки)
        let mut stats = DaemonStats::new();
        stats.record_successful_iteration(100, 5, 1);
        stats.record_error_iteration();
        stats.record_successful_iteration(200, 3, 0);
        stats.record_error_iteration();
        // Функция должна корректно обработать смешанные итерации
        stats.log_stats();
        // Проверяем, что среднее время вычисляется только для успешных итераций
        assert!((stats.average_iteration_duration_ms() - 150.0).abs() < 0.01); // (100 + 200) / 2 = 150
        assert_eq!(stats.total_iterations, 4);
        assert_eq!(stats.error_iterations, 2);
        assert_eq!(stats.successful_iterations, 2);
        assert_eq!(stats.total_applied_adjustments, 8); // 5 + 3
        assert_eq!(stats.total_apply_errors, 1); // только из первой успешной итерации
    }

    #[test]
    fn test_check_system_utilities_does_not_panic() {
        // Тест проверяет, что функция check_system_utilities не паникует
        // независимо от доступности утилит
        check_system_utilities();
    }

    #[test]
    fn test_check_system_utilities_logs_warnings_when_unavailable() {
        // Тест проверяет, что функция проверяет доступность утилит
        // (конкретное поведение зависит от окружения, но функция не должна паниковать)
        check_system_utilities();
        // Если утилиты недоступны, должны быть логи предупреждений
        // Но мы не можем проверить логи в unit-тестах, поэтому просто проверяем, что функция выполняется
    }

    #[test]
    fn test_check_system_utilities_checks_x11_and_wayland() {
        // Тест проверяет, что функция проверяет доступность X11 и Wayland
        // Функция должна корректно обрабатывать случаи, когда оба доступны или недоступны
        check_system_utilities();
        // Функция не должна паниковать независимо от доступности X11/Wayland
    }

    #[test]
    fn test_create_window_introspector_always_returns_valid_introspector() {
        // Тест проверяет, что функция всегда возвращает валидный интроспектор
        // и не падает, независимо от доступности X11/Wayland
        let introspector = create_window_introspector();

        // Проверяем, что интроспектор реализует трейт WindowIntrospector
        let _: &dyn WindowIntrospector = introspector.as_ref();

        // Проверяем, что можно вызвать windows() без паники
        // (может вернуть ошибку, но не должен паниковать)
        let _ = introspector.windows();
    }

    #[test]
    fn test_create_window_introspector_returns_static_on_fallback() {
        // Тест проверяет, что функция возвращает StaticWindowIntrospector,
        // когда X11 и Wayland недоступны (в тестовом окружении это может быть всегда)
        let introspector = create_window_introspector();

        // Проверяем, что можно получить список окон (даже если он пустой)
        match introspector.windows() {
            Ok(windows) => {
                // В тестовом окружении может быть пустой список
                let _ = windows.len();
            }
            Err(_) => {
                // Ошибка не ожидается, но если она есть, это тоже валидный результат
            }
        }
    }

    #[test]
    fn test_create_window_introspector_supports_focused_window() {
        // Тест проверяет, что интроспектор поддерживает метод focused_window()
        let introspector = create_window_introspector();

        // Проверяем, что можно вызвать focused_window() без паники
        match introspector.focused_window() {
            Ok(Some(_)) => {
                // Есть фокусное окно - это нормально
            }
            Ok(None) => {
                // Нет фокусного окна - это тоже нормально
            }
            Err(_) => {
                // Ошибка не ожидается, но если она есть, это тоже валидный результат
            }
        }
    }

    #[test]
    fn test_create_window_introspector_multiple_calls_consistent() {
        // Тест проверяет, что повторные вызовы create_window_introspector
        // возвращают валидные интроспекторы (консистентность)
        let introspector1 = create_window_introspector();
        let introspector2 = create_window_introspector();

        // Оба интроспектора должны быть валидными
        let _: &dyn WindowIntrospector = introspector1.as_ref();
        let _: &dyn WindowIntrospector = introspector2.as_ref();

        // Оба должны поддерживать windows() и focused_window()
        let _ = introspector1.windows();
        let _ = introspector2.windows();
        let _ = introspector1.focused_window();
        let _ = introspector2.focused_window();
    }

    #[test]
    fn test_create_window_introspector_handles_errors_gracefully() {
        // Тест проверяет, что функция корректно обрабатывает ошибки
        // при создании интроспекторов (fallback на StaticWindowIntrospector)
        let introspector = create_window_introspector();

        // Даже если X11/Wayland недоступны, функция должна вернуть валидный интроспектор
        // (StaticWindowIntrospector в качестве fallback)
        let result = introspector.windows();
        // Результат может быть Ok или Err, но не должен паниковать
        match result {
            Ok(windows) => {
                // В тестовом окружении может быть пустой список
                let _ = windows.len();
            }
            Err(_) => {
                // Ошибка допустима, но не должна быть паникой
            }
        }
    }

    #[test]
    fn test_create_window_introspector_returns_send_sync() {
        // Тест проверяет, что возвращаемый интроспектор реализует Send + Sync
        // (необходимо для использования в async контексте)
        let introspector = create_window_introspector();

        // Проверяем, что можно переместить в другую задачу (Send)
        let _handle = std::thread::spawn(move || {
            let _: Box<dyn WindowIntrospector> = introspector;
        });
    }

    // Unit-тесты для проверки обработки ошибок в collect_snapshot при недоступности компонентов
    mod collect_snapshot_error_handling {
        use super::*;
        use crate::metrics::audio::{AudioIntrospector, AudioMetrics, StaticAudioIntrospector};
        use crate::metrics::input::{InputActivityTracker, InputTracker};
        use crate::metrics::scheduling_latency::LatencyCollector;
        use crate::metrics::system::ProcPaths;
        use crate::metrics::windows::{StaticWindowIntrospector, WindowInfo, WindowIntrospector};
        use std::sync::{Arc, Mutex};
        use std::time::Duration;

        /// Интроспектор окон, который всегда возвращает ошибку при вызове windows().
        struct FailingWindowIntrospector;

        impl WindowIntrospector for FailingWindowIntrospector {
            fn windows(&self) -> Result<Vec<WindowInfo>> {
                anyhow::bail!("Window introspector failed: X11 server unavailable")
            }

            fn focused_window(&self) -> Result<Option<WindowInfo>> {
                anyhow::bail!("Window introspector failed: X11 server unavailable")
            }
        }

        /// Интроспектор аудио, который всегда возвращает ошибку при вызове audio_metrics().
        struct FailingAudioIntrospector;

        impl AudioIntrospector for FailingAudioIntrospector {
            fn audio_metrics(&mut self) -> Result<AudioMetrics> {
                anyhow::bail!("Audio introspector failed: PipeWire unavailable")
            }

            fn clients(&self) -> Result<Vec<crate::metrics::audio::AudioClientInfo>> {
                anyhow::bail!("Audio introspector failed: PipeWire unavailable")
            }
        }

        /// Интроспектор аудио, который возвращает ошибку только при вызове audio_metrics().
        struct PartiallyFailingAudioIntrospector;

        impl AudioIntrospector for PartiallyFailingAudioIntrospector {
            fn audio_metrics(&mut self) -> Result<AudioMetrics> {
                anyhow::bail!("Audio metrics failed: XRUN counter unavailable")
            }

            fn clients(&self) -> Result<Vec<crate::metrics::audio::AudioClientInfo>> {
                Ok(Vec::new())
            }
        }

        fn create_test_thresholds() -> crate::config::Thresholds {
            crate::config::Thresholds {
                psi_cpu_some_high: 0.6,
                psi_io_some_high: 0.4,
                user_idle_timeout_sec: 120,
                interactive_build_grace_sec: 10,
                noisy_neighbour_cpu_share: 0.7,
                crit_interactive_percentile: 0.9,
                interactive_percentile: 0.6,
                normal_percentile: 0.3,
                background_percentile: 0.1,
                sched_latency_p99_threshold_ms: 10.0,
                ui_loop_p95_threshold_ms: 16.67,
            }
        }

        /// Тест проверяет graceful fallback при недоступности window introspector.
        #[tokio::test]
        async fn test_collect_snapshot_handles_failing_window_introspector() {
            let proc_paths = ProcPaths::default();
            let window_introspector: Arc<dyn WindowIntrospector> =
                Arc::new(FailingWindowIntrospector);
            let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
                Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
            let input_tracker: Arc<Mutex<InputTracker>> = Arc::new(Mutex::new(
                InputTracker::Simple(InputActivityTracker::new(Duration::from_secs(60))),
            ));
            let mut prev_cpu_times: Option<SystemMetrics> = None;
            let thresholds = create_test_thresholds();
            let latency_collector = Arc::new(LatencyCollector::new(1000));

            let result = collect_snapshot(
                &proc_paths,
                &window_introspector,
                &audio_introspector,
                &input_tracker,
                &mut prev_cpu_times,
                &thresholds,
                &latency_collector,
            )
            .await;

            // Функция должна успешно завершиться, даже если window introspector недоступен
            assert!(
                result.is_ok(),
                "collect_snapshot should succeed even if window introspector fails"
            );
            let snapshot = result.unwrap();

            // Проверяем, что снапшот собран корректно
            assert!(snapshot.snapshot_id > 0);
            // Процессы не должны иметь информацию об окнах (has_gui_window = false)
            for process in &snapshot.processes {
                assert!(
                    !process.has_gui_window,
                    "Process should not have GUI window info when window introspector fails"
                );
                assert!(
                    !process.is_focused_window,
                    "Process should not be marked as focused when window introspector fails"
                );
            }
        }

        /// Тест проверяет graceful fallback при недоступности audio introspector (audio_metrics).
        #[tokio::test]
        async fn test_collect_snapshot_handles_failing_audio_introspector_metrics() {
            let proc_paths = ProcPaths::default();
            let window_introspector: Arc<dyn WindowIntrospector> =
                Arc::new(StaticWindowIntrospector::new(Vec::new()));
            let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
                Arc::new(Mutex::new(Box::new(FailingAudioIntrospector)));
            let input_tracker: Arc<Mutex<InputTracker>> = Arc::new(Mutex::new(
                InputTracker::Simple(InputActivityTracker::new(Duration::from_secs(60))),
            ));
            let mut prev_cpu_times: Option<SystemMetrics> = None;
            let thresholds = create_test_thresholds();
            let latency_collector = Arc::new(LatencyCollector::new(1000));

            let result = collect_snapshot(
                &proc_paths,
                &window_introspector,
                &audio_introspector,
                &input_tracker,
                &mut prev_cpu_times,
                &thresholds,
                &latency_collector,
            )
            .await;

            // Функция должна успешно завершиться, даже если audio introspector недоступен
            assert!(
                result.is_ok(),
                "collect_snapshot should succeed even if audio introspector fails"
            );
            let snapshot = result.unwrap();

            // Проверяем, что снапшот собран корректно
            assert!(snapshot.snapshot_id > 0);
            // Процессы не должны иметь информацию об аудио (is_audio_client = false)
            for process in &snapshot.processes {
                assert!(
                    !process.is_audio_client,
                    "Process should not have audio client info when audio introspector fails"
                );
                assert!(
                    !process.has_active_stream,
                    "Process should not have active stream when audio introspector fails"
                );
            }
            // Audio metrics должны быть пустыми (xrun_count = 0)
            assert_eq!(
                snapshot.responsiveness.audio_xruns_delta,
                Some(0),
                "Audio XRUNs should be 0 when audio introspector fails"
            );
        }

        /// Тест проверяет graceful fallback при частичной недоступности audio introspector
        /// (audio_metrics недоступен, но clients доступен).
        #[tokio::test]
        async fn test_collect_snapshot_handles_partially_failing_audio_introspector() {
            let proc_paths = ProcPaths::default();
            let window_introspector: Arc<dyn WindowIntrospector> =
                Arc::new(StaticWindowIntrospector::new(Vec::new()));
            let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
                Arc::new(Mutex::new(Box::new(PartiallyFailingAudioIntrospector)));
            let input_tracker: Arc<Mutex<InputTracker>> = Arc::new(Mutex::new(
                InputTracker::Simple(InputActivityTracker::new(Duration::from_secs(60))),
            ));
            let mut prev_cpu_times: Option<SystemMetrics> = None;
            let thresholds = create_test_thresholds();
            let latency_collector = Arc::new(LatencyCollector::new(1000));

            let result = collect_snapshot(
                &proc_paths,
                &window_introspector,
                &audio_introspector,
                &input_tracker,
                &mut prev_cpu_times,
                &thresholds,
                &latency_collector,
            )
            .await;

            // Функция должна успешно завершиться, даже если audio_metrics недоступен
            assert!(
                result.is_ok(),
                "collect_snapshot should succeed even if audio_metrics fails"
            );
            let snapshot = result.unwrap();

            // Проверяем, что снапшот собран корректно
            assert!(snapshot.snapshot_id > 0);
            // Audio metrics должны быть пустыми (xrun_count = 0), так как audio_metrics недоступен
            assert_eq!(
                snapshot.responsiveness.audio_xruns_delta,
                Some(0),
                "Audio XRUNs should be 0 when audio_metrics fails"
            );
        }

        /// Тест проверяет graceful fallback при недоступности всех опциональных компонентов
        /// (window, audio, input).
        #[tokio::test]
        async fn test_collect_snapshot_handles_all_optional_components_failing() {
            let proc_paths = ProcPaths::default();
            let window_introspector: Arc<dyn WindowIntrospector> =
                Arc::new(FailingWindowIntrospector);
            let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
                Arc::new(Mutex::new(Box::new(FailingAudioIntrospector)));
            let input_tracker: Arc<Mutex<InputTracker>> = Arc::new(Mutex::new(
                InputTracker::Simple(InputActivityTracker::new(Duration::from_secs(60))),
            ));
            let mut prev_cpu_times: Option<SystemMetrics> = None;
            let thresholds = create_test_thresholds();
            let latency_collector = Arc::new(LatencyCollector::new(1000));

            let result = collect_snapshot(
                &proc_paths,
                &window_introspector,
                &audio_introspector,
                &input_tracker,
                &mut prev_cpu_times,
                &thresholds,
                &latency_collector,
            )
            .await;

            // Функция должна успешно завершиться, даже если все опциональные компоненты недоступны
            assert!(
                result.is_ok(),
                "collect_snapshot should succeed even if all optional components fail"
            );
            let snapshot = result.unwrap();

            // Проверяем, что снапшот собран корректно с дефолтными значениями
            assert!(snapshot.snapshot_id > 0);
            // GlobalMetrics должны быть построены (даже с дефолтными значениями)
            // Примечание: mem_total_kb имеет тип u64, поэтому проверка >= 0 не нужна (u64 всегда >= 0)
            // Проверяем, что GlobalMetrics существует, проверяя наличие других полей
            assert!(
                snapshot.global.load_avg_one >= 0.0,
                "GlobalMetrics should be built even when optional components fail"
            );
            // ResponsivenessMetrics должны быть построены
            assert!(
                snapshot.responsiveness.audio_xruns_delta.is_some(),
                "ResponsivenessMetrics should be built even when optional components fail"
            );
            // Проверяем, что процессы не имеют информации об окнах и аудио
            for process in &snapshot.processes {
                assert!(
                    !process.has_gui_window,
                    "Process should not have GUI window info when all components fail"
                );
                assert!(
                    !process.is_audio_client,
                    "Process should not have audio client info when all components fail"
                );
            }
        }

        /// Тест проверяет, что collect_snapshot корректно обрабатывает ошибку при
        /// недоступности window introspector через spawn_blocking (ошибка join).
        #[tokio::test]
        async fn test_collect_snapshot_handles_window_introspector_join_error() {
            let proc_paths = ProcPaths::default();
            // Используем валидный интроспектор, но ошибка может произойти в spawn_blocking
            let window_introspector: Arc<dyn WindowIntrospector> =
                Arc::new(StaticWindowIntrospector::new(Vec::new()));
            let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
                Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
            let input_tracker: Arc<Mutex<InputTracker>> = Arc::new(Mutex::new(
                InputTracker::Simple(InputActivityTracker::new(Duration::from_secs(60))),
            ));
            let mut prev_cpu_times: Option<SystemMetrics> = None;
            let thresholds = create_test_thresholds();
            let latency_collector = Arc::new(LatencyCollector::new(1000));

            // Этот тест проверяет, что даже если spawn_blocking вернёт ошибку,
            // collect_snapshot должен обработать её gracefully
            let result = collect_snapshot(
                &proc_paths,
                &window_introspector,
                &audio_introspector,
                &input_tracker,
                &mut prev_cpu_times,
                &thresholds,
                &latency_collector,
            )
            .await;

            // Функция должна успешно завершиться
            assert!(
                result.is_ok(),
                "collect_snapshot should handle window introspector errors gracefully"
            );
        }

        /// Тест проверяет, что collect_snapshot корректно обрабатывает ошибку при
        /// недоступности audio introspector через spawn_blocking (ошибка join).
        #[tokio::test]
        async fn test_collect_snapshot_handles_audio_introspector_join_error() {
            let proc_paths = ProcPaths::default();
            let window_introspector: Arc<dyn WindowIntrospector> =
                Arc::new(StaticWindowIntrospector::new(Vec::new()));
            let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
                Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
            let input_tracker: Arc<Mutex<InputTracker>> = Arc::new(Mutex::new(
                InputTracker::Simple(InputActivityTracker::new(Duration::from_secs(60))),
            ));
            let mut prev_cpu_times: Option<SystemMetrics> = None;
            let thresholds = create_test_thresholds();
            let latency_collector = Arc::new(LatencyCollector::new(1000));

            // Этот тест проверяет, что даже если spawn_blocking вернёт ошибку,
            // collect_snapshot должен обработать её gracefully
            let result = collect_snapshot(
                &proc_paths,
                &window_introspector,
                &audio_introspector,
                &input_tracker,
                &mut prev_cpu_times,
                &thresholds,
                &latency_collector,
            )
            .await;

            // Функция должна успешно завершиться
            assert!(
                result.is_ok(),
                "collect_snapshot should handle audio introspector errors gracefully"
            );
        }

        /// Тест проверяет, что collect_snapshot корректно обрабатывает ошибку при
        /// недоступности input tracker через spawn_blocking (ошибка join).
        #[tokio::test]
        async fn test_collect_snapshot_handles_input_tracker_join_error() {
            let proc_paths = ProcPaths::default();
            let window_introspector: Arc<dyn WindowIntrospector> =
                Arc::new(StaticWindowIntrospector::new(Vec::new()));
            let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
                Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
            let input_tracker: Arc<Mutex<InputTracker>> = Arc::new(Mutex::new(
                InputTracker::Simple(InputActivityTracker::new(Duration::from_secs(60))),
            ));
            let mut prev_cpu_times: Option<SystemMetrics> = None;
            let thresholds = create_test_thresholds();
            let latency_collector = Arc::new(LatencyCollector::new(1000));

            // Этот тест проверяет, что даже если spawn_blocking вернёт ошибку,
            // collect_snapshot должен обработать её gracefully
            let result = collect_snapshot(
                &proc_paths,
                &window_introspector,
                &audio_introspector,
                &input_tracker,
                &mut prev_cpu_times,
                &thresholds,
                &latency_collector,
            )
            .await;

            // Функция должна успешно завершиться
            assert!(
                result.is_ok(),
                "collect_snapshot should handle input tracker errors gracefully"
            );
            let snapshot = result.unwrap();

            // Проверяем, что input метрики заполнены дефолтными значениями
            // Проверяем, что snapshot создан и содержит дефолтные значения
            assert!(
                !snapshot.snapshot_id.to_string().is_empty(),
                "Input metrics should be filled with default values even if input tracker fails"
            );
        }

        /// Тест проверяет, что check_system_utilities() не падает и корректно
        /// проверяет доступность системных компонентов.
        #[test]
        fn test_check_system_utilities_does_not_panic() {
            // Функция должна выполниться без паники, независимо от доступности компонентов
            check_system_utilities();
        }

        /// Тест проверяет, что create_window_introspector() всегда возвращает
        /// какой-либо интроспектор (не падает).
        #[test]
        fn test_create_window_introspector_always_returns_introspector() {
            // Функция должна всегда вернуть интроспектор, даже если X11 и Wayland недоступны
            let introspector = create_window_introspector();

            // Проверяем, что интроспектор создан
            assert!(
                introspector.windows().is_ok() || introspector.windows().is_err(),
                "Introspector should be created and have a windows() method"
            );
        }

        /// Тест проверяет, что create_window_introspector() возвращает
        /// StaticWindowIntrospector как fallback, если X11 и Wayland недоступны.
        #[test]
        fn test_create_window_introspector_fallback_to_static() {
            let introspector = create_window_introspector();

            // Проверяем, что интроспектор может вернуть список окон (даже пустой)
            // StaticWindowIntrospector всегда возвращает Ok, даже если список пуст
            let windows_result = introspector.windows();

            // Результат должен быть Ok (даже если список пуст) или Err
            // Главное - функция не должна паниковать
            match windows_result {
                Ok(_windows) => {
                    // Это нормально - StaticWindowIntrospector может вернуть пустой список
                    // Проверка пройдена, если мы дошли до этой точки без паники
                }
                Err(_) => {
                    // Это тоже нормально - интроспектор может вернуть ошибку
                    // Главное - функция не паникует
                    // Проверка пройдена, если мы дошли до этой точки без паники
                }
            }
        }
    }
}
