//! Мониторинг производительности приложений на уровне AppGroup.
//!
//! Этот модуль предоставляет функции для сбора и анализа производительности
//! приложений, сгруппированных по AppGroup. Он фокусируется на метриках,
//! которые важны для пользовательского опыта и производительности системы.

use crate::logging::snapshots::{AppGroupRecord, ProcessRecord};
use crate::metrics::process::{collect_process_metrics, ProcessCacheConfig};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Структура для хранения метрик производительности приложения.
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct AppPerformanceMetrics {
    /// Идентификатор группы приложений.
    pub app_group_id: String,
    
    /// Название группы приложений (для отображения).
    pub app_group_name: String,
    
    /// Количество процессов в группе.
    pub process_count: usize,
    
    /// Общее использование CPU группой (в процентах).
    pub total_cpu_usage: f64,
    
    /// Среднее использование CPU на процесс.
    pub average_cpu_usage: f64,
    
    /// Пиковое использование CPU в группе.
    pub peak_cpu_usage: f64,
    
    /// Общее использование памяти группой (в МБ).
    pub total_memory_mb: u64,
    
    /// Среднее использование памяти на процесс (в МБ).
    pub average_memory_mb: f64,
    
    /// Общий ввод-вывод на диск (байт/сек).
    pub total_io_bytes_per_sec: u64,
    
    /// Количество контекстных переключений (добровольных + принудительных).
    pub total_context_switches: u64,
    
    /// Количество процессов с активными окнами.
    pub processes_with_windows: usize,
    
    /// Количество процессов с активными аудио потоками.
    pub processes_with_audio: usize,
    
    /// Количество процессов с активными терминалами.
    pub processes_with_terminals: usize,
    
    /// Время ответа приложения (если доступно).
    pub response_time_ms: Option<f64>,
    
    /// Статус производительности (хорошо, предупреждение, критическое).
    pub performance_status: PerformanceStatus,
    
    /// Временная метка сбора метрик.
    pub timestamp: Option<SystemTime>,
    
    /// Дополнительные теги и метаданные.
    pub tags: Vec<String>,
}

/// Статус производительности приложения.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq, Default)]
pub enum PerformanceStatus {
    /// Хорошая производительность.
    Good,
    /// Предупреждение о возможных проблемах.
    Warning,
    /// Критическая проблема с производительностью.
    Critical,
    /// Неизвестный статус.
    #[default]
    Unknown,
}

/// Конфигурация для сбора метрик производительности приложений.
#[derive(Debug, Clone)]
pub struct AppPerformanceConfig {
    /// Включить сбор метрик производительности.
    pub enable_app_performance_monitoring: bool,
    
    /// Интервал сбора метрик (в секундах).
    pub collection_interval_seconds: u64,
    
    /// Минимальное количество процессов для мониторинга.
    pub min_processes_for_monitoring: usize,
    
    /// Пороговые значения для определения статуса производительности.
    pub performance_thresholds: PerformanceThresholds,
    
    /// Конфигурация кэша процессов.
    pub process_cache_config: ProcessCacheConfig,
}

impl Default for AppPerformanceConfig {
    fn default() -> Self {
        Self {
            enable_app_performance_monitoring: true,
            collection_interval_seconds: 10,
            min_processes_for_monitoring: 5,
            performance_thresholds: PerformanceThresholds::default(),
            process_cache_config: ProcessCacheConfig::default(),
        }
    }
}

/// Пороговые значения для определения статуса производительности.
#[derive(Debug, Clone)]
pub struct PerformanceThresholds {
    /// Порог CPU для статуса "предупреждение" (в процентах).
    pub cpu_warning_threshold: f64,
    
    /// Порог CPU для статуса "критическое" (в процентах).
    pub cpu_critical_threshold: f64,
    
    /// Порог памяти для статуса "предупреждение" (в МБ).
    pub memory_warning_threshold: u64,
    
    /// Порог памяти для статуса "критическое" (в МБ).
    pub memory_critical_threshold: u64,
    
    /// Порог ввода-вывода для статуса "предупреждение" (в байтах/сек).
    pub io_warning_threshold: u64,
    
    /// Порог ввода-вывода для статуса "критическое" (в байтах/сек).
    pub io_critical_threshold: u64,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            cpu_warning_threshold: 70.0,
            cpu_critical_threshold: 90.0,
            memory_warning_threshold: 1000,  // 1 GB
            memory_critical_threshold: 2000, // 2 GB
            io_warning_threshold: 10_000_000, // 10 MB/s
            io_critical_threshold: 50_000_000, // 50 MB/s
        }
    }
}

/// Собрать метрики производительности для всех групп приложений.
///
/// # Аргументы
///
/// * `app_groups` - Список групп приложений для мониторинга.
/// * `config` - Конфигурация сбора метрик.
///
/// # Возвращаемое значение
///
/// Хэш-карта с метриками производительности для каждой группы приложений.
///
/// # Ошибки
///
/// Возвращает ошибку, если не удалось собрать метрики процессов.
pub fn collect_app_performance_metrics(
    app_groups: &[AppGroupRecord],
    config: Option<AppPerformanceConfig>
) -> Result<HashMap<String, AppPerformanceMetrics>> {
    let app_config = config.unwrap_or_default();
    
    if !app_config.enable_app_performance_monitoring {
        tracing::info!("Мониторинг производительности приложений отключен");
        return Ok(HashMap::new());
    }
    
    tracing::info!(
        "Начало сбора метрик производительности приложений для {} групп",
        app_groups.len()
    );
    
    // Собираем метрики всех процессов
    let start_time = SystemTime::now();
    let cache_config = app_config.process_cache_config.clone();
    let processes = collect_process_metrics(Some(cache_config))?;
    let collection_time = start_time.elapsed().unwrap_or(Duration::from_secs(0));
    
    tracing::debug!(
        "Собрано метрик для {} процессов за {:?}",
        processes.len(),
        collection_time
    );
    
    // Группируем процессы по AppGroup
    let mut app_metrics: HashMap<String, AppPerformanceMetrics> = HashMap::new();
    
    for app_group in app_groups {
        let group_id = app_group.app_group_id.clone();
        let _group_name = app_group.app_name.clone().unwrap_or_else(|| "Unknown".to_string());
        
        // Фильтруем процессы, принадлежащие этой группе
        let group_processes: Vec<_> = processes
            .iter()
            .filter(|p| p.app_group_id.as_ref() == Some(&group_id))
            .collect();
        
        if group_processes.is_empty() {
            continue;
        }
        
        // Вычисляем метрики для группы
        let metrics = calculate_group_metrics(&group_processes, &app_config);
        
        app_metrics.insert(group_id.clone(), metrics);
    }
    
    tracing::info!(
        "Завершен сбор метрик производительности для {} групп приложений",
        app_metrics.len()
    );
    
    Ok(app_metrics)
}

/// Вычислить метрики производительности для группы процессов.
fn calculate_group_metrics(
    processes: &[&ProcessRecord],
    config: &AppPerformanceConfig
) -> AppPerformanceMetrics {
    let process_count = processes.len();
    
    // Вычисляем общие метрики CPU
    let total_cpu_usage: f64 = processes
        .iter()
        .map(|p| p.cpu_share_1s.unwrap_or(0.0))
        .sum();
    
    let average_cpu_usage = if process_count > 0 {
        total_cpu_usage / process_count as f64
    } else {
        0.0
    };
    
    let peak_cpu_usage = processes
        .iter()
        .map(|p| p.cpu_share_1s.unwrap_or(0.0))
        .fold(0.0, f64::max);
    
    // Вычисляем общие метрики памяти
    let total_memory_mb: u64 = processes
        .iter()
        .map(|p| p.rss_mb.unwrap_or(0))
        .sum();
    
    let average_memory_mb = if process_count > 0 {
        total_memory_mb as f64 / process_count as f64
    } else {
        0.0
    };
    
    // Вычисляем общие метрики ввода-вывода
    let total_io_bytes_per_sec: u64 = processes
        .iter()
        .map(|p| {
            let read_bytes = p.io_read_bytes.unwrap_or(0);
            let write_bytes = p.io_write_bytes.unwrap_or(0);
            read_bytes + write_bytes
        })
        .sum();
    
    // Вычисляем общие контекстные переключения
    let total_context_switches: u64 = processes
        .iter()
        .map(|p| {
            let voluntary = p.voluntary_ctx.unwrap_or(0);
            let involuntary = p.involuntary_ctx.unwrap_or(0);
            voluntary + involuntary
        })
        .sum();
    
    // Считаем процессы с окнами
    let processes_with_windows = processes
        .iter()
        .filter(|p| p.has_gui_window || p.is_focused_window)
        .count();
    
    // Считаем процессы с аудио
    let processes_with_audio = processes
        .iter()
        .filter(|p| p.is_audio_client || p.has_active_stream)
        .count();
    
    // Считаем процессы с терминалами
    let processes_with_terminals = processes
        .iter()
        .filter(|p| p.has_tty || p.env_term.is_some())
        .count();
    
    // Определяем статус производительности
    let performance_status = determine_performance_status(
        total_cpu_usage,
        total_memory_mb,
        total_io_bytes_per_sec,
        &config.performance_thresholds
    );
    
    // Собираем теги
    let mut tags = Vec::new();
    if processes_with_windows > 0 {
        tags.push("has_windows".to_string());
    }
    if processes_with_audio > 0 {
        tags.push("has_audio".to_string());
    }
    if processes_with_terminals > 0 {
        tags.push("has_terminals".to_string());
    }
    
    // Определяем имя группы (используем имя первого процесса или "Unknown")
    let app_group_name = processes
        .first()
        .and_then(|p| p.app_group_id.as_ref())
        .map(|id| format!("AppGroup_{}", id))
        .unwrap_or_else(|| "Unknown_AppGroup".to_string());
    
    // Определяем ID группы
    let app_group_id = processes
        .first()
        .and_then(|p| p.app_group_id.as_ref())
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    
    AppPerformanceMetrics {
        app_group_id,
        app_group_name,
        process_count,
        total_cpu_usage,
        average_cpu_usage,
        peak_cpu_usage,
        total_memory_mb,
        average_memory_mb,
        total_io_bytes_per_sec,
        total_context_switches,
        processes_with_windows,
        processes_with_audio,
        processes_with_terminals,
        response_time_ms: None, // Будет реализовано позже
        performance_status,
        timestamp: Some(SystemTime::now()),
        tags,
    }
}

/// Определить статус производительности на основе метрик и порогов.
fn determine_performance_status(
    total_cpu_usage: f64,
    total_memory_mb: u64,
    total_io_bytes_per_sec: u64,
    thresholds: &PerformanceThresholds
) -> PerformanceStatus {
    let cpu_critical = total_cpu_usage > thresholds.cpu_critical_threshold;
    let cpu_warning = total_cpu_usage > thresholds.cpu_warning_threshold;
    
    let memory_critical = total_memory_mb > thresholds.memory_critical_threshold;
    let memory_warning = total_memory_mb > thresholds.memory_warning_threshold;
    
    let io_critical = total_io_bytes_per_sec > thresholds.io_critical_threshold;
    let io_warning = total_io_bytes_per_sec > thresholds.io_warning_threshold;
    
    // Критический статус, если хотя бы один критический порог превышен
    if cpu_critical || memory_critical || io_critical {
        return PerformanceStatus::Critical;
    }
    
    // Предупреждение, если хотя бы один порог предупреждения превышен
    if cpu_warning || memory_warning || io_warning {
        return PerformanceStatus::Warning;
    }
    
    // Хороший статус, если все метрики в норме
    PerformanceStatus::Good
}

/// Собрать метрики производительности для конкретной группы приложений.
///
/// # Аргументы
///
/// * `app_group` - Группа приложений для мониторинга.
/// * `config` - Конфигурация сбора метрик.
///
/// # Возвращаемое значение
///
/// Метрики производительности для указанной группы приложений.
pub fn collect_app_group_performance(
    app_group: &AppGroupRecord,
    config: Option<AppPerformanceConfig>
) -> Result<Option<AppPerformanceMetrics>> {
    let app_config = config.unwrap_or_default();
    
    if !app_config.enable_app_performance_monitoring {
        return Ok(None);
    }
    
    // Собираем метрики всех процессов
    let cache_config = app_config.process_cache_config.clone();
    let processes = collect_process_metrics(Some(cache_config))?;
    
    // Фильтруем процессы, принадлежащие этой группе
    let group_processes: Vec<_> = processes
        .iter()
        .filter(|p| p.app_group_id.as_ref() == Some(&app_group.app_group_id))
        .collect();
    
    if group_processes.is_empty() {
        return Ok(None);
    }
    
    // Вычисляем метрики для группы
    let metrics = calculate_group_metrics(&group_processes, &app_config);
    
    Ok(Some(metrics))
}

/// Собрать метрики производительности для всех процессов в системе.
///
/// # Аргументы
///
/// * `config` - Конфигурация сбора метрик.
///
/// # Возвращаемое значение
///
/// Метрики производительности для всех процессов, сгруппированные по AppGroup.
pub fn collect_all_app_performance(
    config: Option<AppPerformanceConfig>
) -> Result<HashMap<String, AppPerformanceMetrics>> {
    let app_config = config.unwrap_or_default();
    
    if !app_config.enable_app_performance_monitoring {
        tracing::info!("Мониторинг производительности приложений отключен");
        return Ok(HashMap::new());
    }
    
    // Собираем метрики всех процессов
    let cache_config = app_config.process_cache_config.clone();
    let processes = collect_process_metrics(Some(cache_config))?;
    
    // Группируем процессы по AppGroup
    let mut app_groups_map: HashMap<String, Vec<&ProcessRecord>> = HashMap::new();
    
    for process in &processes {
        if let Some(app_group_id) = &process.app_group_id {
            app_groups_map
                .entry(app_group_id.clone())
                .or_default()
                .push(process);
        }
    }
    
    // Вычисляем метрики для каждой группы
    let mut app_metrics: HashMap<String, AppPerformanceMetrics> = HashMap::new();
    
    for (app_group_id, group_processes) in app_groups_map {
        if group_processes.len() >= app_config.min_processes_for_monitoring {
            let metrics = calculate_group_metrics(&group_processes, &app_config);
            app_metrics.insert(app_group_id, metrics);
        }
    }
    
    tracing::info!(
        "Собрано метрик производительности для {} групп приложений",
        app_metrics.len()
    );
    
    Ok(app_metrics)
}

/// Собрать метрики производительности для конкретного процесса.
///
/// # Аргументы
///
/// * `process` - Процесс для анализа.
/// * `config` - Конфигурация сбора метрик.
///
/// # Возвращаемое значение
///
/// Метрики производительности для указанного процесса.
pub fn collect_process_performance(
    process: &ProcessRecord,
    config: Option<AppPerformanceConfig>
) -> Result<AppPerformanceMetrics> {
    let app_config = config.unwrap_or_default();
    
    if !app_config.enable_app_performance_monitoring {
        return Err(anyhow::anyhow!("Мониторинг производительности отключен"));
    }
    
    // Создаем временную группу с одним процессом
    let group_processes = vec![process];
    let metrics = calculate_group_metrics(&group_processes, &app_config);
    
    Ok(metrics)
}

/// Собрать исторические метрики производительности.
///
/// # Аргументы
///
/// * `duration` - Длительность сбора истории (в секундах).
/// * `interval` - Интервал между сборами (в секундах).
/// * `config` - Конфигурация сбора метрик.
///
/// # Возвращаемое значение
///
/// Вектор метрик производительности, собранных за указанный период.
pub fn collect_performance_history(
    duration: Duration,
    interval: Duration,
    config: Option<AppPerformanceConfig>
) -> Result<Vec<HashMap<String, AppPerformanceMetrics>>> {
    let app_config = config.unwrap_or_default();
    
    if !app_config.enable_app_performance_monitoring {
        return Err(anyhow::anyhow!("Мониторинг производительности отключен"));
    }
    
    let start_time = SystemTime::now();
    let mut history = Vec::new();
    
    while start_time.elapsed().unwrap_or(Duration::from_secs(0)) < duration {
        let metrics = collect_all_app_performance(Some(app_config.clone()))?;
        history.push(metrics);
        
        if interval > Duration::from_secs(0) {
            std::thread::sleep(interval);
        }
    }
    
    Ok(history)
}

/// Проанализировать тренды производительности.
///
/// # Аргументы
///
/// * `history` - История метрик производительности.
///
/// # Возвращаемое значение
///
/// Анализ трендов производительности.
pub fn analyze_performance_trends(
    history: &[HashMap<String, AppPerformanceMetrics>]
) -> Result<PerformanceTrends> {
    if history.is_empty() {
        return Err(anyhow::anyhow!("История метрик пуста"));
    }
    
    let mut trends = PerformanceTrends::default();
    
    // Анализируем тренды для каждой группы
    for (app_group_id, metrics_list) in group_history_by_app_group(history) {
        let group_trend = analyze_group_trend(&metrics_list);
        trends.group_trends.insert(app_group_id, group_trend);
    }
    
    // Вычисляем общие тренды
    trends.overall_trend = calculate_overall_trend(&trends.group_trends);
    trends.analysis_timestamp = Some(SystemTime::now());
    
    Ok(trends)
}

/// Группировать историю метрик по группам приложений.
fn group_history_by_app_group(
    history: &[HashMap<String, AppPerformanceMetrics>]
) -> HashMap<String, Vec<&AppPerformanceMetrics>> {
    let mut grouped_history: HashMap<String, Vec<&AppPerformanceMetrics>> = HashMap::new();
    
    for snapshot in history {
        for (app_group_id, metrics) in snapshot {
            grouped_history
                .entry(app_group_id.clone())
                .or_default()
                .push(metrics);
        }
    }
    
    grouped_history
}

/// Проанализировать тренд для одной группы приложений.
fn analyze_group_trend(
    metrics_list: &[&AppPerformanceMetrics]
) -> GroupPerformanceTrend {
    if metrics_list.is_empty() {
        return GroupPerformanceTrend::default();
    }
    
    let first = metrics_list.first().unwrap();
    let last = metrics_list.last().unwrap();
    
    let cpu_change = last.total_cpu_usage - first.total_cpu_usage;
    let memory_change = last.total_memory_mb as f64 - first.total_memory_mb as f64;
    let io_change = last.total_io_bytes_per_sec as f64 - first.total_io_bytes_per_sec as f64;
    
    let cpu_trend = if cpu_change > 0.0 {
        PerformanceTrendDirection::Increasing
    } else if cpu_change < 0.0 {
        PerformanceTrendDirection::Decreasing
    } else {
        PerformanceTrendDirection::Stable
    };
    
    let memory_trend = if memory_change > 0.0 {
        PerformanceTrendDirection::Increasing
    } else if memory_change < 0.0 {
        PerformanceTrendDirection::Decreasing
    } else {
        PerformanceTrendDirection::Stable
    };
    
    let io_trend = if io_change > 0.0 {
        PerformanceTrendDirection::Increasing
    } else if io_change < 0.0 {
        PerformanceTrendDirection::Decreasing
    } else {
        PerformanceTrendDirection::Stable
    };
    
    GroupPerformanceTrend {
        cpu_trend,
        memory_trend,
        io_trend,
        average_cpu_usage: last.average_cpu_usage,
        average_memory_mb: last.average_memory_mb,
        average_io_bytes_per_sec: last.total_io_bytes_per_sec as f64,
    }
}

/// Вычислить общий тренд производительности.
fn calculate_overall_trend(
    group_trends: &HashMap<String, GroupPerformanceTrend>
) -> OverallPerformanceTrend {
    if group_trends.is_empty() {
        return OverallPerformanceTrend::default();
    }
    
    let mut cpu_increasing = 0;
    let mut cpu_decreasing = 0;
    let mut memory_increasing = 0;
    let mut memory_decreasing = 0;
    let mut io_increasing = 0;
    let mut io_decreasing = 0;
    
    for trend in group_trends.values() {
        match trend.cpu_trend {
            PerformanceTrendDirection::Increasing => cpu_increasing += 1,
            PerformanceTrendDirection::Decreasing => cpu_decreasing += 1,
            PerformanceTrendDirection::Stable => {}
        }
        
        match trend.memory_trend {
            PerformanceTrendDirection::Increasing => memory_increasing += 1,
            PerformanceTrendDirection::Decreasing => memory_decreasing += 1,
            PerformanceTrendDirection::Stable => {}
        }
        
        match trend.io_trend {
            PerformanceTrendDirection::Increasing => io_increasing += 1,
            PerformanceTrendDirection::Decreasing => io_decreasing += 1,
            PerformanceTrendDirection::Stable => {}
        }
    }
    
    let total_groups = group_trends.len() as f64;
    
    let cpu_trend = if cpu_increasing > cpu_decreasing {
        PerformanceTrendDirection::Increasing
    } else if cpu_decreasing > cpu_increasing {
        PerformanceTrendDirection::Decreasing
    } else {
        PerformanceTrendDirection::Stable
    };
    
    let memory_trend = if memory_increasing > memory_decreasing {
        PerformanceTrendDirection::Increasing
    } else if memory_decreasing > memory_increasing {
        PerformanceTrendDirection::Decreasing
    } else {
        PerformanceTrendDirection::Stable
    };
    
    let io_trend = if io_increasing > io_decreasing {
        PerformanceTrendDirection::Increasing
    } else if io_decreasing > io_increasing {
        PerformanceTrendDirection::Decreasing
    } else {
        PerformanceTrendDirection::Stable
    };
    
    OverallPerformanceTrend {
        cpu_trend,
        memory_trend,
        io_trend,
        groups_improving: cpu_decreasing + memory_decreasing + io_decreasing,
        groups_degrading: cpu_increasing + memory_increasing + io_increasing,
        total_groups,
    }
}

/// Структура для хранения трендов производительности.
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct PerformanceTrends {
    /// Общий тренд производительности.
    pub overall_trend: OverallPerformanceTrend,
    
    /// Тренды для отдельных групп приложений.
    pub group_trends: HashMap<String, GroupPerformanceTrend>,
    
    /// Временная метка анализа.
    pub analysis_timestamp: Option<SystemTime>,
}

/// Общий тренд производительности.
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct OverallPerformanceTrend {
    /// Тренд использования CPU.
    pub cpu_trend: PerformanceTrendDirection,
    
    /// Тренд использования памяти.
    pub memory_trend: PerformanceTrendDirection,
    
    /// Тренд ввода-вывода.
    pub io_trend: PerformanceTrendDirection,
    
    /// Количество групп, показывающих улучшение.
    pub groups_improving: usize,
    
    /// Количество групп, показывающих ухудшение.
    pub groups_degrading: usize,
    
    /// Общее количество групп.
    pub total_groups: f64,
}

/// Тренд производительности для группы приложений.
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct GroupPerformanceTrend {
    /// Тренд использования CPU.
    pub cpu_trend: PerformanceTrendDirection,
    
    /// Тренд использования памяти.
    pub memory_trend: PerformanceTrendDirection,
    
    /// Тренд ввода-вывода.
    pub io_trend: PerformanceTrendDirection,
    
    /// Среднее использование CPU.
    pub average_cpu_usage: f64,
    
    /// Среднее использование памяти (в МБ).
    pub average_memory_mb: f64,
    
    /// Средний ввод-вывод (в байтах/сек).
    pub average_io_bytes_per_sec: f64,
}

/// Направление тренда производительности.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq, Default)]
pub enum PerformanceTrendDirection {
    /// Производительность улучшается (метрики уменьшаются).
    Decreasing,
    
    /// Производительность ухудшается (метрики увеличиваются).
    Increasing,
    
    /// Производительность стабильна.
    #[default]
    Stable,
}

/// Преобразовать метрики производительности в JSON для API.
///
/// # Аргументы
///
/// * `metrics` - Метрики производительности для преобразования.
///
/// # Возвращаемое значение
///
/// JSON представление метрик производительности.
pub fn metrics_to_json(
    metrics: &HashMap<String, AppPerformanceMetrics>
) -> Result<String> {
    serde_json::to_string(metrics)
        .context("Не удалось сериализовать метрики производительности в JSON")
}

/// Преобразовать метрики производительности в формат для Prometheus.
///
/// # Аргументы
///
/// * `metrics` - Метрики производительности для преобразования.
///
/// # Возвращаемое значение
///
/// Формат Prometheus для метрик производительности.
pub fn metrics_to_prometheus(
    metrics: &HashMap<String, AppPerformanceMetrics>
) -> String {
    let mut output = String::new();
    
    for (app_group_id, metrics) in metrics {
        // Метрики CPU
        output.push_str(&format!(
            "app_performance_cpu_usage{{app_group=\"{}\"}} {}\n",
            app_group_id, metrics.total_cpu_usage
        ));
        
        output.push_str(&format!(
            "app_performance_avg_cpu_usage{{app_group=\"{}\"}} {}\n",
            app_group_id, metrics.average_cpu_usage
        ));
        
        // Метрики памяти
        output.push_str(&format!(
            "app_performance_memory_mb{{app_group=\"{}\"}} {}\n",
            app_group_id, metrics.total_memory_mb
        ));
        
        output.push_str(&format!(
            "app_performance_avg_memory_mb{{app_group=\"{}\"}} {}\n",
            app_group_id, metrics.average_memory_mb
        ));
        
        // Метрики ввода-вывода
        output.push_str(&format!(
            "app_performance_io_bytes_per_sec{{app_group=\"{}\"}} {}\n",
            app_group_id, metrics.total_io_bytes_per_sec
        ));
        
        // Метрики статуса
        let status_value = match metrics.performance_status {
            PerformanceStatus::Good => 0,
            PerformanceStatus::Warning => 1,
            PerformanceStatus::Critical => 2,
            PerformanceStatus::Unknown => 3,
        };
        
        output.push_str(&format!(
            "app_performance_status{{app_group=\"{}\"}} {}\n",
            app_group_id, status_value
        ));
        
        // Метрики процессов
        output.push_str(&format!(
            "app_performance_process_count{{app_group=\"{}\"}} {}\n",
            app_group_id, metrics.process_count
        ));
    }
    
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::snapshots::ProcessRecord;

    #[test]
    fn test_performance_status_determination() {
        let thresholds = PerformanceThresholds::default();
        
        // Тест хорошего статуса
        let status = determine_performance_status(30.0, 500, 1_000_000, &thresholds);
        assert_eq!(status, PerformanceStatus::Good);
        
        // Тест статуса предупреждения (CPU)
        let status = determine_performance_status(75.0, 500, 1_000_000, &thresholds);
        assert_eq!(status, PerformanceStatus::Warning);
        
        // Тест статуса предупреждения (память)
        let status = determine_performance_status(30.0, 1500, 1_000_000, &thresholds);
        assert_eq!(status, PerformanceStatus::Warning);
        
        // Тест статуса предупреждения (ввод-вывод)
        let status = determine_performance_status(30.0, 500, 15_000_000, &thresholds);
        assert_eq!(status, PerformanceStatus::Warning);
        
        // Тест критического статуса (CPU)
        let status = determine_performance_status(95.0, 500, 1_000_000, &thresholds);
        assert_eq!(status, PerformanceStatus::Critical);
        
        // Тест критического статуса (память)
        let status = determine_performance_status(30.0, 2500, 1_000_000, &thresholds);
        assert_eq!(status, PerformanceStatus::Critical);
        
        // Тест критического статуса (ввод-вывод)
        let status = determine_performance_status(30.0, 500, 60_000_000, &thresholds);
        assert_eq!(status, PerformanceStatus::Critical);
    }

    /// Helper function to create a default ProcessRecord for testing
    fn create_test_process(pid: i32) -> ProcessRecord {
        ProcessRecord {
            pid,
            ppid: 1,
            uid: 1000,
            gid: 1000,
            exe: Some("test".to_string()),
            cmdline: Some("test".to_string()),
            cgroup_path: None,
            systemd_unit: None,
            app_group_id: None,
            state: "R".to_string(),
            start_time: 0,
            uptime_sec: 0,
            tty_nr: 0,
            has_tty: false,
            cpu_share_1s: None,
            cpu_share_10s: None,
            io_read_bytes: None,
            io_write_bytes: None,
            rss_mb: None,
            swap_mb: None,
            voluntary_ctx: None,
            involuntary_ctx: None,
            has_gui_window: false,
            is_focused_window: false,
            window_state: None,
            env_has_display: false,
            env_has_wayland: false,
            env_term: None,
            env_ssh: false,
            is_audio_client: false,
            has_active_stream: false,
            process_type: None,
            tags: vec![],
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
        }
    }

    #[test]
    fn test_group_metrics_calculation() {
        // Создаем тестовые процессы
        let mut process1 = create_test_process(1);
        process1.cpu_share_1s = Some(10.0);
        process1.rss_mb = Some(100);
        process1.io_read_bytes = Some(1000);
        process1.io_write_bytes = Some(2000);
        process1.voluntary_ctx = Some(10);
        process1.involuntary_ctx = Some(5);
        process1.has_gui_window = true;
        process1.app_group_id = Some("test_group".to_string());
        
        let mut process2 = create_test_process(2);
        process2.cpu_share_1s = Some(20.0);
        process2.rss_mb = Some(200);
        process2.io_read_bytes = Some(3000);
        process2.io_write_bytes = Some(4000);
        process2.voluntary_ctx = Some(20);
        process2.involuntary_ctx = Some(10);
        process2.is_audio_client = true;
        process2.app_group_id = Some("test_group".to_string());
        
        let processes = vec![&process1, &process2];
        let config = AppPerformanceConfig::default();
        
        let metrics = calculate_group_metrics(&processes, &config);
        
        // Проверяем основные метрики
        assert_eq!(metrics.process_count, 2);
        assert_eq!(metrics.total_cpu_usage, 30.0);
        assert_eq!(metrics.average_cpu_usage, 15.0);
        assert_eq!(metrics.peak_cpu_usage, 20.0);
        assert_eq!(metrics.total_memory_mb, 300);
        assert_eq!(metrics.average_memory_mb, 150.0);
        assert_eq!(metrics.total_io_bytes_per_sec, 10000); // 1000+2000+3000+4000
        assert_eq!(metrics.total_context_switches, 45); // 10+5+20+10
        assert_eq!(metrics.processes_with_windows, 1);
        assert_eq!(metrics.processes_with_audio, 1);
        assert_eq!(metrics.processes_with_terminals, 0);
    }

    #[test]
    fn test_performance_metrics_serialization() {
        let metrics = AppPerformanceMetrics {
            app_group_id: "test_group".to_string(),
            app_group_name: "Test Application".to_string(),
            process_count: 2,
            total_cpu_usage: 30.0,
            average_cpu_usage: 15.0,
            peak_cpu_usage: 20.0,
            total_memory_mb: 300,
            average_memory_mb: 150.0,
            total_io_bytes_per_sec: 10000,
            total_context_switches: 45,
            processes_with_windows: 1,
            processes_with_audio: 1,
            processes_with_terminals: 0,
            response_time_ms: Some(100.0),
            performance_status: PerformanceStatus::Good,
            timestamp: Some(SystemTime::now()),
            tags: vec!["has_windows".to_string(), "has_audio".to_string()],
        };
        
        // Тестируем сериализацию в JSON
        let json_result = serde_json::to_string(&metrics);
        assert!(json_result.is_ok());
        
        // Тестируем преобразование в Prometheus
        let mut metrics_map = HashMap::new();
        metrics_map.insert("test_group".to_string(), metrics);
        
        let prometheus_output = metrics_to_prometheus(&metrics_map);
        assert!(prometheus_output.contains("app_performance_cpu_usage"));
        assert!(prometheus_output.contains("app_performance_memory_mb"));
        assert!(prometheus_output.contains("app_performance_io_bytes_per_sec"));
    }

    #[test]
    fn test_config_defaults() {
        let config = AppPerformanceConfig::default();
        
        assert!(config.enable_app_performance_monitoring);
        assert_eq!(config.collection_interval_seconds, 10);
        assert_eq!(config.min_processes_for_monitoring, 5);
        assert_eq!(config.performance_thresholds.cpu_warning_threshold, 70.0);
        assert_eq!(config.performance_thresholds.cpu_critical_threshold, 90.0);
    }

    #[test]
    fn test_disabled_monitoring() {
        let config = AppPerformanceConfig {
            enable_app_performance_monitoring: false,
            ..Default::default()
        };
        
        let app_groups = vec![];
        let result = collect_app_performance_metrics(&app_groups, Some(config));
        
        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert!(metrics.is_empty());
    }

    #[test]
    fn test_empty_app_groups() {
        let config = AppPerformanceConfig::default();
        let app_groups = vec![];
        
        let result = collect_app_performance_metrics(&app_groups, Some(config));
        
        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert!(metrics.is_empty());
    }

    #[test]
    fn test_trend_analysis() {
        // Создаем историю метрик
        let mut history = Vec::new();
        
        // Первая точка данных
        let mut metrics1 = HashMap::new();
        let app_metrics1 = AppPerformanceMetrics {
            app_group_id: "test_group".to_string(),
            app_group_name: String::new(),
            process_count: 0,
            total_cpu_usage: 30.0,
            average_cpu_usage: 0.0,
            peak_cpu_usage: 0.0,
            total_memory_mb: 500,
            average_memory_mb: 0.0,
            total_io_bytes_per_sec: 5_000_000,
            total_context_switches: 0,
            ..Default::default()
        };
        metrics1.insert("test_group".to_string(), app_metrics1);
        history.push(metrics1);
        
        // Вторая точка данных (увеличение метрик)
        let mut metrics2 = HashMap::new();
        let app_metrics2 = AppPerformanceMetrics {
            app_group_id: "test_group".to_string(),
            app_group_name: String::new(),
            process_count: 0,
            total_cpu_usage: 40.0,
            average_cpu_usage: 0.0,
            peak_cpu_usage: 0.0,
            total_memory_mb: 600,
            average_memory_mb: 0.0,
            total_io_bytes_per_sec: 6_000_000,
            total_context_switches: 0,
            ..Default::default()
        };
        metrics2.insert("test_group".to_string(), app_metrics2);
        history.push(metrics2);
        
        // Анализируем тренды
        let trends_result = analyze_performance_trends(&history);
        assert!(trends_result.is_ok());
        
        let trends = trends_result.unwrap();
        assert_eq!(trends.group_trends.len(), 1);
        
        if let Some(group_trend) = trends.group_trends.get("test_group") {
            assert_eq!(group_trend.cpu_trend, PerformanceTrendDirection::Increasing);
            assert_eq!(group_trend.memory_trend, PerformanceTrendDirection::Increasing);
            assert_eq!(group_trend.io_trend, PerformanceTrendDirection::Increasing);
        }
    }

    #[test]
    fn test_prometheus_format() {
        let mut metrics = HashMap::new();
        
        let app_metrics = AppPerformanceMetrics {
            app_group_id: "test_group".to_string(),
            app_group_name: "Test App".to_string(),
            process_count: 2,
            total_cpu_usage: 30.0,
            average_cpu_usage: 15.0,
            peak_cpu_usage: 20.0,
            total_memory_mb: 300,
            average_memory_mb: 150.0,
            total_io_bytes_per_sec: 10000,
            total_context_switches: 45,
            processes_with_windows: 1,
            processes_with_audio: 1,
            processes_with_terminals: 0,
            response_time_ms: None,
            performance_status: PerformanceStatus::Good,
            timestamp: Some(SystemTime::now()),
            tags: vec![],
        };
        
        metrics.insert("test_group".to_string(), app_metrics);
        
        let prometheus_output = metrics_to_prometheus(&metrics);
        
        // Проверяем, что вывод содержит ожидаемые метрики
        assert!(prometheus_output.contains("app_performance_cpu_usage{app_group=\"test_group\"}"));
        assert!(prometheus_output.contains("app_performance_memory_mb{app_group=\"test_group\"}"));
        assert!(prometheus_output.contains("app_performance_io_bytes_per_sec{app_group=\"test_group\"}"));
        assert!(prometheus_output.contains("app_performance_status{app_group=\"test_group\"}"));
        assert!(prometheus_output.contains("app_performance_process_count{app_group=\"test_group\"}"));
    }
}