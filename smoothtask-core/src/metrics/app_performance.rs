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

    /// Расширенные метрики производительности
    /// Кадры в секунду (для графических приложений).
    pub fps: Option<f64>,

    /// Задержка отклика приложения (в миллисекундах).
    pub latency_ms: Option<f64>,

    /// Задержка ввода (в миллисекундах).
    pub input_latency_ms: Option<f64>,

    /// Задержка рендеринга (в миллисекундах).
    pub render_latency_ms: Option<f64>,

    /// Использование GPU (в процентах).
    pub gpu_usage_percent: Option<f64>,

    /// Использование памяти GPU (в МБ).
    pub gpu_memory_mb: Option<u64>,

    /// Количество ошибок рендеринга.
    pub render_errors: Option<u64>,

    /// Количество пропущенных кадров.
    pub dropped_frames: Option<u64>,

    /// Среднее время кадра (в миллисекундах).
    pub frame_time_ms: Option<f64>,

    /// Процент использования сети (в процентах).
    pub network_usage_percent: Option<f64>,

    /// Количество активных сетевых соединений.
    pub active_connections: Option<usize>,

    /// Средняя задержка сети (в миллисекундах).
    pub network_latency_ms: Option<f64>,

    /// Количество ошибок сети.
    pub network_errors: Option<u64>,

    /// Использование диска (в процентах).
    pub disk_usage_percent: Option<f64>,

    /// Задержка диска (в миллисекундах).
    pub disk_latency_ms: Option<f64>,

    /// Количество ошибок диска.
    pub disk_errors: Option<u64>,

    /// Общий балл производительности (0-100).
    pub performance_score: Option<u8>,

    /// Индекс стабильности (0-100, где 100 - максимальная стабильность).
    pub stability_index: Option<u8>,

    /// Индекс отзывчивости (0-100, где 100 - максимальная отзывчивость).
    pub responsiveness_index: Option<u8>,
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

    /// Порог FPS для статуса "предупреждение" (кадры в секунду).
    pub fps_warning_threshold: f64,

    /// Порог FPS для статуса "критическое" (кадры в секунду).
    pub fps_critical_threshold: f64,

    /// Порог задержки для статуса "предупреждение" (в миллисекундах).
    pub latency_warning_threshold: f64,

    /// Порог задержки для статуса "критическое" (в миллисекундах).
    pub latency_critical_threshold: f64,

    /// Порог использования GPU для статуса "предупреждение" (в процентах).
    pub gpu_usage_warning_threshold: f64,

    /// Порог использования GPU для статуса "критическое" (в процентах).
    pub gpu_usage_critical_threshold: f64,

    /// Порог использования памяти GPU для статуса "предупреждение" (в МБ).
    pub gpu_memory_warning_threshold: u64,

    /// Порог использования памяти GPU для статуса "критическое" (в МБ).
    pub gpu_memory_critical_threshold: u64,

    /// Порог ошибок рендеринга для статуса "предупреждение" (количество).
    pub render_errors_warning_threshold: u64,

    /// Порог ошибок рендеринга для статуса "критическое" (количество).
    pub render_errors_critical_threshold: u64,

    /// Порог пропущенных кадров для статуса "предупреждение" (количество).
    pub dropped_frames_warning_threshold: u64,

    /// Порог пропущенных кадров для статуса "критическое" (количество).
    pub dropped_frames_critical_threshold: u64,

    /// Порог использования сети для статуса "предупреждение" (в процентах).
    pub network_usage_warning_threshold: f64,

    /// Порог использования сети для статуса "критическое" (в процентах).
    pub network_usage_critical_threshold: f64,

    /// Порог ошибок сети для статуса "предупреждение" (количество).
    pub network_errors_warning_threshold: u64,

    /// Порог ошибок сети для статуса "критическое" (количество).
    pub network_errors_critical_threshold: u64,

    /// Порог использования диска для статуса "предупреждение" (в процентах).
    pub disk_usage_warning_threshold: f64,

    /// Порог использования диска для статуса "критическое" (в процентах).
    pub disk_usage_critical_threshold: f64,

    /// Порог ошибок диска для статуса "предупреждение" (количество).
    pub disk_errors_warning_threshold: u64,

    /// Порог ошибок диска для статуса "критическое" (количество).
    pub disk_errors_critical_threshold: u64,

    /// Порог балла производительности для статуса "предупреждение" (0-100).
    pub performance_score_warning_threshold: u8,

    /// Порог балла производительности для статуса "критическое" (0-100).
    pub performance_score_critical_threshold: u8,

    /// Порог индекса стабильности для статуса "предупреждение" (0-100).
    pub stability_index_warning_threshold: u8,

    /// Порог индекса стабильности для статуса "критическое" (0-100).
    pub stability_index_critical_threshold: u8,

    /// Порог индекса отзывчивости для статуса "предупреждение" (0-100).
    pub responsiveness_index_warning_threshold: u8,

    /// Порог индекса отзывчивости для статуса "критическое" (0-100).
    pub responsiveness_index_critical_threshold: u8,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            cpu_warning_threshold: 70.0,
            cpu_critical_threshold: 90.0,
            memory_warning_threshold: 1000,    // 1 GB
            memory_critical_threshold: 2000,   // 2 GB
            io_warning_threshold: 10_000_000,  // 10 MB/s
            io_critical_threshold: 50_000_000, // 50 MB/s
            fps_warning_threshold: 30.0,       // 30 FPS
            fps_critical_threshold: 15.0,      // 15 FPS
            latency_warning_threshold: 50.0,   // 50 ms
            latency_critical_threshold: 100.0, // 100 ms
            gpu_usage_warning_threshold: 80.0, // 80%
            gpu_usage_critical_threshold: 95.0, // 95%
            gpu_memory_warning_threshold: 2000, // 2 GB
            gpu_memory_critical_threshold: 4000, // 4 GB
            render_errors_warning_threshold: 10, // 10 errors
            render_errors_critical_threshold: 50, // 50 errors
            dropped_frames_warning_threshold: 20, // 20 frames
            dropped_frames_critical_threshold: 100, // 100 frames
            network_usage_warning_threshold: 70.0, // 70%
            network_usage_critical_threshold: 90.0, // 90%
            network_errors_warning_threshold: 5, // 5 errors
            network_errors_critical_threshold: 20, // 20 errors
            disk_usage_warning_threshold: 80.0, // 80%
            disk_usage_critical_threshold: 95.0, // 95%
            disk_errors_warning_threshold: 3, // 3 errors
            disk_errors_critical_threshold: 10, // 10 errors
            performance_score_warning_threshold: 60, // 60/100
            performance_score_critical_threshold: 40, // 40/100
            stability_index_warning_threshold: 70, // 70/100
            stability_index_critical_threshold: 50, // 50/100
            responsiveness_index_warning_threshold: 70, // 70/100
            responsiveness_index_critical_threshold: 50, // 50/100
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
    config: Option<AppPerformanceConfig>,
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
        let _group_name = app_group
            .app_name
            .clone()
            .unwrap_or_else(|| "Unknown".to_string());

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
    config: &AppPerformanceConfig,
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
    let total_memory_mb: u64 = processes.iter().map(|p| p.rss_mb.unwrap_or(0)).sum();

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

    // Вычисляем расширенные метрики
    let (fps, latency_ms, input_latency_ms, render_latency_ms, gpu_usage_percent, gpu_memory_mb,
        render_errors, dropped_frames, frame_time_ms, network_usage_percent, active_connections,
        network_latency_ms, network_errors, disk_usage_percent, disk_latency_ms, disk_errors) =
        calculate_extended_metrics(processes);

    // Вычисляем индексы производительности
    let (performance_score, stability_index, responsiveness_index) = calculate_performance_indices(
        total_cpu_usage,
        total_memory_mb,
        total_io_bytes_per_sec,
        fps,
        latency_ms,
        gpu_usage_percent,
        render_errors,
        dropped_frames,
        network_errors,
        disk_errors,
    );

    // Создаем временный объект метрик для определения статуса
    let mut temp_metrics = AppPerformanceMetrics {
        app_group_id: String::new(),
        app_group_name: String::new(),
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
        response_time_ms: None,
        performance_status: PerformanceStatus::Unknown,
        timestamp: Some(SystemTime::now()),
        tags: Vec::new(),
        fps,
        latency_ms,
        input_latency_ms,
        render_latency_ms,
        gpu_usage_percent,
        gpu_memory_mb,
        render_errors,
        dropped_frames,
        frame_time_ms,
        network_usage_percent,
        active_connections,
        network_latency_ms,
        network_errors,
        disk_usage_percent,
        disk_latency_ms,
        disk_errors,
        performance_score,
        stability_index,
        responsiveness_index,
    };

    // Определяем расширенный статус производительности
    let performance_status = determine_extended_performance_status(&temp_metrics, &config.performance_thresholds);

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
    if fps.is_some_and(|f| f > 0.0) {
        tags.push("has_graphics".to_string());
    }
    if gpu_usage_percent.is_some_and(|g| g > 0.0) {
        tags.push("uses_gpu".to_string());
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
        response_time_ms: None,
        performance_status,
        timestamp: Some(SystemTime::now()),
        tags,
        fps,
        latency_ms,
        input_latency_ms,
        render_latency_ms,
        gpu_usage_percent,
        gpu_memory_mb,
        render_errors,
        dropped_frames,
        frame_time_ms,
        network_usage_percent,
        active_connections,
        network_latency_ms,
        network_errors,
        disk_usage_percent,
        disk_latency_ms,
        disk_errors,
        performance_score,
        stability_index,
        responsiveness_index,
    }
}

/// Вычислить расширенные метрики производительности.
fn calculate_extended_metrics(
    processes: &[&ProcessRecord],
) -> (
    Option<f64>,           // fps
    Option<f64>,           // latency_ms
    Option<f64>,           // input_latency_ms
    Option<f64>,           // render_latency_ms
    Option<f64>,           // gpu_usage_percent
    Option<u64>,           // gpu_memory_mb
    Option<u64>,           // render_errors
    Option<u64>,           // dropped_frames
    Option<f64>,           // frame_time_ms
    Option<f64>,           // network_usage_percent
    Option<usize>,         // active_connections
    Option<f64>,           // network_latency_ms
    Option<u64>,           // network_errors
    Option<f64>,           // disk_usage_percent
    Option<f64>,           // disk_latency_ms
    Option<u64>,           // disk_errors
) {
    // Вычисляем средние значения FPS (если доступны)
    let fps: Option<f64> = {
        let fps_values: Vec<f64> = processes
            .iter()
            .filter_map(|p| p.gpu_api_calls.map(|v| v as f64))
            .collect();
        if !fps_values.is_empty() {
            Some(fps_values.iter().sum::<f64>() / fps_values.len() as f64)
        } else {
            None
        }
    };

    // Вычисляем среднюю задержку (используем время ответа или другие метрики)
    let latency_ms: Option<f64> = {
        let latency_values: Vec<f64> = processes
            .iter()
            .filter_map(|p| p.gpu_time_us.map(|t| t as f64 / 1000.0))
            .collect();
        if !latency_values.is_empty() {
            Some(latency_values.iter().sum::<f64>() / latency_values.len() as f64)
        } else {
            None
        }
    };

    // Вычисляем задержку ввода (используем контекстные переключения как прокси)
    let input_latency_ms: Option<f64> = {
        let total_ctx = processes
            .iter()
            .map(|p| p.voluntary_ctx.unwrap_or(0) + p.involuntary_ctx.unwrap_or(0))
            .sum::<u64>() as f64;
        if total_ctx > 0.0 {
            Some(total_ctx / processes.len().max(1) as f64 * 0.1)
        } else {
            None
        }
    };

    // Вычисляем задержку рендеринга (используем время GPU как прокси)
    let render_latency_ms: Option<f64> = {
        let render_latency_values: Vec<f64> = processes
            .iter()
            .filter_map(|p| p.gpu_time_us.map(|t| t as f64 / 1000.0))
            .collect();
        if !render_latency_values.is_empty() {
            Some(render_latency_values.iter().sum::<f64>() / render_latency_values.len() as f64)
        } else {
            None
        }
    };

    // Вычисляем использование GPU
    let gpu_usage_percent: Option<f64> = {
        let gpu_usage_values: Vec<f64> = processes
            .iter()
            .filter_map(|p| p.gpu_utilization.map(|v| v as f64))
            .collect();
        if !gpu_usage_values.is_empty() {
            Some(gpu_usage_values.iter().sum::<f64>() / gpu_usage_values.len() as f64)
        } else {
            None
        }
    };

    // Вычисляем использование памяти GPU
    let gpu_memory_mb: Option<u64> = {
        let gpu_memory_values: Vec<u64> = processes
            .iter()
            .filter_map(|p| p.gpu_memory_bytes.map(|b| b / 1024 / 1024))
            .collect();
        if !gpu_memory_values.is_empty() {
            Some(gpu_memory_values.iter().sum::<u64>() / gpu_memory_values.len() as u64)
        } else {
            None
        }
    };

    // Вычисляем ошибки рендеринга (используем количество контекстных переключений как прокси)
    let render_errors: Option<u64> = {
        let total_errors = processes
            .iter()
            .map(|p| p.involuntary_ctx.unwrap_or(0) / 10)
            .sum();
        if total_errors > 0 {
            Some(total_errors)
        } else {
            None
        }
    };

    // Вычисляем пропущенные кадры (используем контекстные переключения как прокси)
    let dropped_frames: Option<u64> = {
        let total_dropped = processes
            .iter()
            .map(|p| p.involuntary_ctx.unwrap_or(0) / 20)
            .sum();
        if total_dropped > 0 {
            Some(total_dropped)
        } else {
            None
        }
    };

    // Вычисляем время кадра
    let frame_time_ms: Option<f64> = {
        if let Some(fps) = fps {
            if fps > 0.0 {
                Some(1000.0 / fps)
            } else {
                None
            }
        } else {
            None
        }
    };

    // Вычисляем использование сети
    let network_usage_percent: Option<f64> = {
        let total_network = processes
            .iter()
            .map(|p| {
                let rx = p.network_rx_bytes.unwrap_or(0) as f64;
                let tx = p.network_tx_bytes.unwrap_or(0) as f64;
                rx + tx
            })
            .sum::<f64>();
        if total_network > 0.0 {
            // Нормализуем к процентам (простая эвристика)
            let normalized = (total_network / 1000000.0).min(100.0);
            Some(normalized)
        } else {
            None
        }
    };

    // Вычисляем активные соединения
    let active_connections: Option<usize> = {
        let total_connections = processes
            .iter()
            .map(|p| p.network_tcp_connections.unwrap_or(0) + p.network_udp_connections.unwrap_or(0))
            .sum::<u64>() as usize;
        if total_connections > 0 {
            Some(total_connections)
        } else {
            None
        }
    };

    // Вычисляем задержку сети (используем время ответа как прокси)
    let network_latency_ms: Option<f64> = {
        if let Some(latency) = latency_ms {
            Some(latency * 1.5) // Простая эвристика
        } else {
            None
        }
    };

    // Вычисляем ошибки сети (используем контекстные переключения как прокси)
    let network_errors: Option<u64> = {
        let total_errors = processes
            .iter()
            .map(|p| p.involuntary_ctx.unwrap_or(0) / 50)
            .sum();
        if total_errors > 0 {
            Some(total_errors)
        } else {
            None
        }
    };

    // Вычисляем использование диска
    let disk_usage_percent: Option<f64> = {
        let total_io = processes
            .iter()
            .map(|p| {
                let read = p.io_read_bytes.unwrap_or(0) as f64;
                let write = p.io_write_bytes.unwrap_or(0) as f64;
                read + write
            })
            .sum::<f64>();
        if total_io > 0.0 {
            // Нормализуем к процентам (простая эвристика)
            let normalized = (total_io / 10000000.0).min(100.0);
            Some(normalized)
        } else {
            None
        }
    };

    // Вычисляем задержку диска
    let disk_latency_ms: Option<f64> = {
        if let Some(latency) = latency_ms {
            Some(latency * 2.0) // Простая эвристика
        } else {
            None
        }
    };

    // Вычисляем ошибки диска (используем контекстные переключения как прокси)
    let disk_errors: Option<u64> = {
        let total_errors = processes
            .iter()
            .map(|p| p.involuntary_ctx.unwrap_or(0) / 100)
            .sum();
        if total_errors > 0 {
            Some(total_errors)
        } else {
            None
        }
    };

    (
        fps,
        latency_ms,
        input_latency_ms,
        render_latency_ms,
        gpu_usage_percent,
        gpu_memory_mb,
        render_errors,
        dropped_frames,
        frame_time_ms,
        network_usage_percent,
        active_connections,
        network_latency_ms,
        network_errors,
        disk_usage_percent,
        disk_latency_ms,
        disk_errors,
    )
}

/// Вычислить индексы производительности.
fn calculate_performance_indices(
    total_cpu_usage: f64,
    total_memory_mb: u64,
    total_io_bytes_per_sec: u64,
    fps: Option<f64>,
    latency_ms: Option<f64>,
    gpu_usage_percent: Option<f64>,
    render_errors: Option<u64>,
    dropped_frames: Option<u64>,
    network_errors: Option<u64>,
    disk_errors: Option<u64>,
) -> (Option<u8>, Option<u8>, Option<u8>) {
    // Вычисляем балл производительности (0-100)
    let performance_score = {
        let mut score = 100.0;

        // Штрафуем за высокое использование CPU
        if total_cpu_usage > 80.0 {
            score -= (total_cpu_usage - 80.0) * 0.5;
        }

        // Штрафуем за высокое использование памяти
        if total_memory_mb > 1500 {
            score -= (total_memory_mb as f64 - 1500.0) * 0.02;
        }

        // Штрафуем за высокий ввод-вывод
        if total_io_bytes_per_sec > 25_000_000 {
            score -= (total_io_bytes_per_sec as f64 - 25_000_000.0) * 0.001;
        }

        // Штрафуем за низкий FPS
        if let Some(fps) = fps {
            if fps < 60.0 {
                score -= (60.0 - fps) * 0.2;
            }
        }

        // Штрафуем за высокую задержку
        if let Some(latency) = latency_ms {
            if latency > 30.0 {
                score -= (latency - 30.0) * 0.5;
            }
        }

        // Штрафуем за высокое использование GPU
        if let Some(gpu_usage) = gpu_usage_percent {
            if gpu_usage > 90.0 {
                score -= (gpu_usage - 90.0) * 0.3;
            }
        }

        // Штрафуем за ошибки
        if let Some(errors) = render_errors {
            score -= errors as f64 * 0.1;
        }

        if let Some(dropped) = dropped_frames {
            score -= dropped as f64 * 0.05;
        }

        if let Some(errors) = network_errors {
            score -= errors as f64 * 0.2;
        }

        if let Some(errors) = disk_errors {
            score -= errors as f64 * 0.3;
        }

        Some(score.max(0.0).min(100.0) as u8)
    };

    // Вычисляем индекс стабильности (0-100)
    let stability_index = {
        let mut stability = 100.0;

        // Штрафуем за ошибки
        if let Some(errors) = render_errors {
            stability -= errors as f64 * 0.5;
        }

        if let Some(dropped) = dropped_frames {
            stability -= dropped as f64 * 0.2;
        }

        if let Some(errors) = network_errors {
            stability -= errors as f64 * 1.0;
        }

        if let Some(errors) = disk_errors {
            stability -= errors as f64 * 1.5;
        }

        // Штрафуем за высокую задержку
        if let Some(latency) = latency_ms {
            if latency > 50.0 {
                stability -= (latency - 50.0) * 0.3;
            }
        }

        Some(stability.max(0.0).min(100.0) as u8)
    };

    // Вычисляем индекс отзывчивости (0-100)
    let responsiveness_index = {
        let mut responsiveness = 100.0;

        // Штрафуем за высокую задержку
        if let Some(latency) = latency_ms {
            if latency > 20.0 {
                responsiveness -= (latency - 20.0) * 1.0;
            }
        }

        // Штрафуем за низкий FPS
        if let Some(fps) = fps {
            if fps < 30.0 {
                responsiveness -= (30.0 - fps) * 2.0;
            }
        }

        // Штрафуем за высокое использование CPU
        if total_cpu_usage > 70.0 {
            responsiveness -= (total_cpu_usage - 70.0) * 0.8;
        }

        Some(responsiveness.max(0.0).min(100.0) as u8)
    };

    (performance_score, stability_index, responsiveness_index)
}

/// Определить статус производительности на основе метрик и порогов.
fn determine_performance_status(
    total_cpu_usage: f64,
    total_memory_mb: u64,
    total_io_bytes_per_sec: u64,
    thresholds: &PerformanceThresholds,
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

/// Определить расширенный статус производительности на основе всех метрик.
fn determine_extended_performance_status(
    metrics: &AppPerformanceMetrics,
    thresholds: &PerformanceThresholds,
) -> PerformanceStatus {
    let mut critical_count = 0;
    let mut warning_count = 0;

    // Проверяем базовые метрики
    if metrics.total_cpu_usage > thresholds.cpu_critical_threshold {
        critical_count += 1;
    } else if metrics.total_cpu_usage > thresholds.cpu_warning_threshold {
        warning_count += 1;
    }

    if metrics.total_memory_mb > thresholds.memory_critical_threshold {
        critical_count += 1;
    } else if metrics.total_memory_mb > thresholds.memory_warning_threshold {
        warning_count += 1;
    }

    if metrics.total_io_bytes_per_sec > thresholds.io_critical_threshold {
        critical_count += 1;
    } else if metrics.total_io_bytes_per_sec > thresholds.io_warning_threshold {
        warning_count += 1;
    }

    // Проверяем расширенные метрики
    if let Some(fps) = metrics.fps {
        if fps < thresholds.fps_critical_threshold {
            critical_count += 1;
        } else if fps < thresholds.fps_warning_threshold {
            warning_count += 1;
        }
    }

    if let Some(latency) = metrics.latency_ms {
        if latency > thresholds.latency_critical_threshold {
            critical_count += 1;
        } else if latency > thresholds.latency_warning_threshold {
            warning_count += 1;
        }
    }

    if let Some(gpu_usage) = metrics.gpu_usage_percent {
        if gpu_usage > thresholds.gpu_usage_critical_threshold {
            critical_count += 1;
        } else if gpu_usage > thresholds.gpu_usage_warning_threshold {
            warning_count += 1;
        }
    }

    if let Some(gpu_memory) = metrics.gpu_memory_mb {
        if gpu_memory > thresholds.gpu_memory_critical_threshold {
            critical_count += 1;
        } else if gpu_memory > thresholds.gpu_memory_warning_threshold {
            warning_count += 1;
        }
    }

    if let Some(render_errors) = metrics.render_errors {
        if render_errors > thresholds.render_errors_critical_threshold {
            critical_count += 1;
        } else if render_errors > thresholds.render_errors_warning_threshold {
            warning_count += 1;
        }
    }

    if let Some(dropped_frames) = metrics.dropped_frames {
        if dropped_frames > thresholds.dropped_frames_critical_threshold {
            critical_count += 1;
        } else if dropped_frames > thresholds.dropped_frames_warning_threshold {
            warning_count += 1;
        }
    }

    if let Some(network_usage) = metrics.network_usage_percent {
        if network_usage > thresholds.network_usage_critical_threshold {
            critical_count += 1;
        } else if network_usage > thresholds.network_usage_warning_threshold {
            warning_count += 1;
        }
    }

    if let Some(network_errors) = metrics.network_errors {
        if network_errors > thresholds.network_errors_critical_threshold {
            critical_count += 1;
        } else if network_errors > thresholds.network_errors_warning_threshold {
            warning_count += 1;
        }
    }

    if let Some(disk_usage) = metrics.disk_usage_percent {
        if disk_usage > thresholds.disk_usage_critical_threshold {
            critical_count += 1;
        } else if disk_usage > thresholds.disk_usage_warning_threshold {
            warning_count += 1;
        }
    }

    if let Some(disk_errors) = metrics.disk_errors {
        if disk_errors > thresholds.disk_errors_critical_threshold {
            critical_count += 1;
        } else if disk_errors > thresholds.disk_errors_warning_threshold {
            warning_count += 1;
        }
    }

    if let Some(performance_score) = metrics.performance_score {
        if performance_score < thresholds.performance_score_critical_threshold {
            critical_count += 1;
        } else if performance_score < thresholds.performance_score_warning_threshold {
            warning_count += 1;
        }
    }

    if let Some(stability_index) = metrics.stability_index {
        if stability_index < thresholds.stability_index_critical_threshold {
            critical_count += 1;
        } else if stability_index < thresholds.stability_index_warning_threshold {
            warning_count += 1;
        }
    }

    if let Some(responsiveness_index) = metrics.responsiveness_index {
        if responsiveness_index < thresholds.responsiveness_index_critical_threshold {
            critical_count += 1;
        } else if responsiveness_index < thresholds.responsiveness_index_warning_threshold {
            warning_count += 1;
        }
    }

    // Критический статус, если 3 или более критических метрик
    if critical_count >= 3 {
        return PerformanceStatus::Critical;
    }

    // Предупреждение, если 2 или более предупреждений или 1 критическое
    if warning_count >= 2 || critical_count >= 1 {
        return PerformanceStatus::Warning;
    }

    // Хороший статус
    PerformanceStatus::Good
}

/// Обнаружить аномалии производительности.
///
/// # Аргументы
///
/// * `current_metrics` - Текущие метрики производительности.
/// * `historical_data` - Исторические данные для сравнения.
/// * `config` - Конфигурация для настройки чувствительности.
///
/// # Возвращаемое значение
///
/// Вектор обнаруженных аномалий.
pub fn detect_performance_anomalies(
    current_metrics: &AppPerformanceMetrics,
    historical_data: &[AppPerformanceMetrics],
    config: Option<AppPerformanceConfig>,
) -> Vec<PerformanceAnomaly> {
    let app_config = config.unwrap_or_default();
    let mut anomalies = Vec::new();

    if historical_data.is_empty() {
        return anomalies;
    }

    // Вычисляем средние значения из исторических данных
    let historical_avg_cpu: f64 = historical_data
        .iter()
        .map(|m| m.total_cpu_usage)
        .sum::<f64>() / historical_data.len() as f64;

    let historical_avg_memory: f64 = historical_data
        .iter()
        .map(|m| m.total_memory_mb as f64)
        .sum::<f64>() / historical_data.len() as f64;

    let historical_avg_io: f64 = historical_data
        .iter()
        .map(|m| m.total_io_bytes_per_sec as f64)
        .sum::<f64>() / historical_data.len() as f64;

    let historical_avg_fps: Option<f64> = {
        let fps_values: Vec<f64> = historical_data
            .iter()
            .filter_map(|m| m.fps)
            .collect();
        if !fps_values.is_empty() {
            Some(fps_values.iter().sum::<f64>() / fps_values.len() as f64)
        } else {
            None
        }
    };

    let historical_avg_latency: Option<f64> = {
        let latency_values: Vec<f64> = historical_data
            .iter()
            .filter_map(|m| m.latency_ms)
            .collect();
        if !latency_values.is_empty() {
            Some(latency_values.iter().sum::<f64>() / latency_values.len() as f64)
        } else {
            None
        }
    };

    // Обнаруживаем аномалии CPU
    let cpu_deviation = (current_metrics.total_cpu_usage - historical_avg_cpu).abs();
    if cpu_deviation > app_config.performance_thresholds.cpu_warning_threshold * 0.5 {
        anomalies.push(PerformanceAnomaly {
            metric: "cpu_usage".to_string(),
            current_value: current_metrics.total_cpu_usage,
            historical_avg: Some(historical_avg_cpu),
            deviation: cpu_deviation,
            severity: if cpu_deviation > app_config.performance_thresholds.cpu_critical_threshold * 0.5 {
                AnomalySeverity::High
            } else {
                AnomalySeverity::Medium
            },
            timestamp: current_metrics.timestamp,
        });
    }

    // Обнаруживаем аномалии памяти
    let memory_deviation = (current_metrics.total_memory_mb as f64 - historical_avg_memory).abs();
    if memory_deviation > app_config.performance_thresholds.memory_warning_threshold as f64 * 0.5 {
        anomalies.push(PerformanceAnomaly {
            metric: "memory_usage".to_string(),
            current_value: current_metrics.total_memory_mb as f64,
            historical_avg: Some(historical_avg_memory),
            deviation: memory_deviation,
            severity: if memory_deviation > app_config.performance_thresholds.memory_critical_threshold as f64 * 0.5 {
                AnomalySeverity::High
            } else {
                AnomalySeverity::Medium
            },
            timestamp: current_metrics.timestamp,
        });
    }

    // Обнаруживаем аномалии ввода-вывода
    let io_deviation = (current_metrics.total_io_bytes_per_sec as f64 - historical_avg_io).abs();
    if io_deviation > app_config.performance_thresholds.io_warning_threshold as f64 * 0.5 {
        anomalies.push(PerformanceAnomaly {
            metric: "io_usage".to_string(),
            current_value: current_metrics.total_io_bytes_per_sec as f64,
            historical_avg: Some(historical_avg_io),
            deviation: io_deviation,
            severity: if io_deviation > app_config.performance_thresholds.io_critical_threshold as f64 * 0.5 {
                AnomalySeverity::High
            } else {
                AnomalySeverity::Medium
            },
            timestamp: current_metrics.timestamp,
        });
    }

    // Обнаруживаем аномалии FPS
    if let (Some(current_fps), Some(hist_fps)) = (current_metrics.fps, historical_avg_fps) {
        let fps_deviation = (current_fps - hist_fps).abs();
        if fps_deviation > app_config.performance_thresholds.fps_warning_threshold * 0.5 {
            anomalies.push(PerformanceAnomaly {
                metric: "fps".to_string(),
                current_value: current_fps,
                historical_avg: Some(hist_fps),
                deviation: fps_deviation,
                severity: if fps_deviation > app_config.performance_thresholds.fps_critical_threshold * 0.5 {
                    AnomalySeverity::High
                } else {
                    AnomalySeverity::Medium
                },
                timestamp: current_metrics.timestamp,
            });
        }
    }

    // Обнаруживаем аномалии задержки
    if let (Some(current_latency), Some(hist_latency)) = (current_metrics.latency_ms, historical_avg_latency) {
        let latency_deviation = (current_latency - hist_latency).abs();
        if latency_deviation > app_config.performance_thresholds.latency_warning_threshold * 0.5 {
            anomalies.push(PerformanceAnomaly {
                metric: "latency".to_string(),
                current_value: current_latency,
                historical_avg: Some(hist_latency),
                deviation: latency_deviation,
                severity: if latency_deviation > app_config.performance_thresholds.latency_critical_threshold * 0.5 {
                    AnomalySeverity::High
                } else {
                    AnomalySeverity::Medium
                },
                timestamp: current_metrics.timestamp,
            });
        }
    }

    // Обнаруживаем аномалии GPU
    if let Some(current_gpu) = current_metrics.gpu_usage_percent {
        let historical_avg_gpu: f64 = historical_data
            .iter()
            .filter_map(|m| m.gpu_usage_percent)
            .sum::<f64>() / historical_data.iter().filter(|m| m.gpu_usage_percent.is_some()).count().max(1) as f64;

        let gpu_deviation = (current_gpu - historical_avg_gpu).abs();
        if gpu_deviation > app_config.performance_thresholds.gpu_usage_warning_threshold * 0.5 {
            anomalies.push(PerformanceAnomaly {
                metric: "gpu_usage".to_string(),
                current_value: current_gpu,
                historical_avg: Some(historical_avg_gpu),
                deviation: gpu_deviation,
                severity: if gpu_deviation > app_config.performance_thresholds.gpu_usage_critical_threshold * 0.5 {
                    AnomalySeverity::High
                } else {
                    AnomalySeverity::Medium
                },
                timestamp: current_metrics.timestamp,
            });
        }
    }

    // Обнаруживаем аномалии ошибок
    if let Some(current_errors) = current_metrics.render_errors {
        if current_errors > 0 {
            let historical_avg_errors: f64 = historical_data
                .iter()
                .filter_map(|m| m.render_errors)
                .sum::<u64>() as f64 / historical_data.iter().filter(|m| m.render_errors.is_some()).count().max(1) as f64;

            if current_errors as f64 > historical_avg_errors * 2.0 {
                anomalies.push(PerformanceAnomaly {
                    metric: "render_errors".to_string(),
                    current_value: current_errors as f64,
                    historical_avg: Some(historical_avg_errors),
                    deviation: current_errors as f64 - historical_avg_errors,
                    severity: AnomalySeverity::High,
                    timestamp: current_metrics.timestamp,
                });
            }
        }
    }

    anomalies
}

/// Структура для хранения информации об аномалии производительности.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PerformanceAnomaly {
    /// Метрика, в которой обнаружена аномалия.
    pub metric: String,

    /// Текущее значение метрики.
    pub current_value: f64,

    /// Историческое среднее значение.
    pub historical_avg: Option<f64>,

    /// Отклонение от исторического среднего.
    pub deviation: f64,

    /// Серьезность аномалии.
    pub severity: AnomalySeverity,

    /// Временная метка обнаружения.
    pub timestamp: Option<SystemTime>,
}

/// Серьезность аномалии.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub enum AnomalySeverity {
    /// Низкая серьезность.
    Low,

    /// Средняя серьезность.
    Medium,

    /// Высокая серьезность.
    High,

    /// Критическая серьезность.
    Critical,
}

/// Структура для хранения исторического анализа производительности.
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct PerformanceHistoryAnalysis {
    /// Идентификатор группы приложений.
    pub app_group_id: String,

    /// Среднее использование CPU за период.
    pub avg_cpu_usage: f64,

    /// Максимальное использование CPU за период.
    pub max_cpu_usage: f64,

    /// Среднее использование памяти за период.
    pub avg_memory_mb: f64,

    /// Максимальное использование памяти за период.
    pub max_memory_mb: u64,

    /// Средний ввод-вывод за период.
    pub avg_io_bytes_per_sec: u64,

    /// Максимальный ввод-вывод за период.
    pub max_io_bytes_per_sec: u64,

    /// Средний FPS за период.
    pub avg_fps: Option<f64>,

    /// Максимальный FPS за период.
    pub max_fps: Option<f64>,

    /// Средняя задержка за период.
    pub avg_latency_ms: Option<f64>,

    /// Максимальная задержка за период.
    pub max_latency_ms: Option<f64>,

    /// Среднее использование GPU за период.
    pub avg_gpu_usage_percent: Option<f64>,

    /// Максимальное использование GPU за период.
    pub max_gpu_usage_percent: Option<f64>,

    /// Среднее использование памяти GPU за период.
    pub avg_gpu_memory_mb: Option<u64>,

    /// Максимальное использование памяти GPU за период.
    pub max_gpu_memory_mb: Option<u64>,

    /// Общее количество ошибок рендеринга за период.
    pub total_render_errors: u64,

    /// Общее количество пропущенных кадров за период.
    pub total_dropped_frames: u64,

    /// Средний балл производительности за период.
    pub avg_performance_score: Option<f64>,

    /// Средний индекс стабильности за период.
    pub avg_stability_index: Option<f64>,

    /// Средний индекс отзывчивости за период.
    pub avg_responsiveness_index: Option<f64>,

    /// Количество аномалий, обнаруженных за период.
    pub anomaly_count: usize,

    /// Временная метка начала периода.
    pub period_start: Option<SystemTime>,

    /// Временная метка конца периода.
    pub period_end: Option<SystemTime>,
}

/// Проанализировать исторические данные производительности.
///
/// # Аргументы
///
/// * `history` - История метрик производительности.
/// * `config` - Конфигурация для настройки анализа.
///
/// # Возвращаемое значение
///
/// Анализ исторических данных производительности для каждой группы приложений.
pub fn analyze_performance_history(
    history: &[HashMap<String, AppPerformanceMetrics>],
    config: Option<AppPerformanceConfig>,
) -> Result<HashMap<String, PerformanceHistoryAnalysis>> {
    let app_config = config.unwrap_or_default();
    let mut analysis_results = HashMap::new();

    if history.is_empty() {
        return Ok(analysis_results);
    }

    // Группируем историю по группам приложений
    let mut grouped_history: HashMap<String, Vec<&AppPerformanceMetrics>> = HashMap::new();

    for snapshot in history {
        for (app_group_id, metrics) in snapshot {
            grouped_history
                .entry(app_group_id.clone())
                .or_default()
                .push(metrics);
        }
    }

    // Анализируем каждую группу
    for (app_group_id, metrics_list) in grouped_history {
        if metrics_list.is_empty() {
            continue;
        }

        let analysis = analyze_group_history(&metrics_list, &app_config)?;
        analysis_results.insert(app_group_id, analysis);
    }

    Ok(analysis_results)
}

/// Проанализировать историю производительности для одной группы.
fn analyze_group_history(
    metrics_list: &[&AppPerformanceMetrics],
    config: &AppPerformanceConfig,
) -> Result<PerformanceHistoryAnalysis> {
    if metrics_list.is_empty() {
        return Err(anyhow::anyhow!("Пустой список метрик для анализа"));
    }

    let first_metrics = metrics_list.first().unwrap();
    let last_metrics = metrics_list.last().unwrap();

    // Вычисляем средние и максимальные значения
    let avg_cpu_usage: f64 = metrics_list
        .iter()
        .map(|m| m.total_cpu_usage)
        .sum::<f64>() / metrics_list.len() as f64;

    let max_cpu_usage = metrics_list
        .iter()
        .map(|m| m.total_cpu_usage)
        .fold(0.0, f64::max);

    let avg_memory_mb: f64 = metrics_list
        .iter()
        .map(|m| m.total_memory_mb as f64)
        .sum::<f64>() / metrics_list.len() as f64;

    let max_memory_mb = metrics_list
        .iter()
        .map(|m| m.total_memory_mb)
        .fold(0, u64::max);

    let avg_io_bytes_per_sec: f64 = metrics_list
        .iter()
        .map(|m| m.total_io_bytes_per_sec as f64)
        .sum::<f64>() / metrics_list.len() as f64;

    let max_io_bytes_per_sec = metrics_list
        .iter()
        .map(|m| m.total_io_bytes_per_sec)
        .fold(0, u64::max);

    // Вычисляем средние и максимальные значения для FPS
    let (avg_fps, max_fps) = {
        let fps_values: Vec<f64> = metrics_list
            .iter()
            .filter_map(|m| m.fps)
            .collect();
        if !fps_values.is_empty() {
            let avg = fps_values.iter().sum::<f64>() / fps_values.len() as f64;
            let max = fps_values.iter().fold(0.0f64, |a, &b| a.max(b));
            (Some(avg), Some(max))
        } else {
            (None, None)
        }
    };

    // Вычисляем средние и максимальные значения для задержки
    let (avg_latency_ms, max_latency_ms) = {
        let latency_values: Vec<f64> = metrics_list
            .iter()
            .filter_map(|m| m.latency_ms)
            .collect();
        if !latency_values.is_empty() {
            let avg = latency_values.iter().sum::<f64>() / latency_values.len() as f64;
            let max = latency_values.iter().fold(0.0f64, |a, &b| a.max(b));
            (Some(avg), Some(max))
        } else {
            (None, None)
        }
    };

    // Вычисляем средние и максимальные значения для GPU
    let (avg_gpu_usage_percent, max_gpu_usage_percent) = {
        let gpu_values: Vec<f64> = metrics_list
            .iter()
            .filter_map(|m| m.gpu_usage_percent)
            .collect();
        if !gpu_values.is_empty() {
            let avg = gpu_values.iter().sum::<f64>() / gpu_values.len() as f64;
            let max = gpu_values.iter().fold(0.0f64, |a, &b| a.max(b));
            (Some(avg), Some(max))
        } else {
            (None, None)
        }
    };

    let (avg_gpu_memory_mb, max_gpu_memory_mb) = {
        let gpu_mem_values: Vec<u64> = metrics_list
            .iter()
            .filter_map(|m| m.gpu_memory_mb)
            .collect();
        if !gpu_mem_values.is_empty() {
            let avg = gpu_mem_values.iter().sum::<u64>() as f64 / gpu_mem_values.len() as f64;
            let max = gpu_mem_values.iter().fold(0, |a, &b| a.max(b));
            (Some(avg as u64), Some(max))
        } else {
            (None, None)
        }
    };

    // Вычисляем общее количество ошибок
    let total_render_errors: u64 = metrics_list
        .iter()
        .filter_map(|m| m.render_errors)
        .sum();

    let total_dropped_frames: u64 = metrics_list
        .iter()
        .filter_map(|m| m.dropped_frames)
        .sum();

    // Вычисляем средние индексы производительности
    let (avg_performance_score, avg_stability_index, avg_responsiveness_index) = {
        let score_values: Vec<u8> = metrics_list
            .iter()
            .filter_map(|m| m.performance_score)
            .collect();
        let stability_values: Vec<u8> = metrics_list
            .iter()
            .filter_map(|m| m.stability_index)
            .collect();
        let responsiveness_values: Vec<u8> = metrics_list
            .iter()
            .filter_map(|m| m.responsiveness_index)
            .collect();

        let avg_score = if !score_values.is_empty() {
            Some(score_values.iter().sum::<u8>() as f64 / score_values.len() as f64)
        } else {
            None
        };

        let avg_stability = if !stability_values.is_empty() {
            Some(stability_values.iter().sum::<u8>() as f64 / stability_values.len() as f64)
        } else {
            None
        };

        let avg_responsiveness = if !responsiveness_values.is_empty() {
            Some(responsiveness_values.iter().sum::<u8>() as f64 / responsiveness_values.len() as f64)
        } else {
            None
        };

        (avg_score, avg_stability, avg_responsiveness)
    };

    // Обнаруживаем аномалии
    let mut anomaly_count = 0;
    for (i, metrics) in metrics_list.iter().enumerate() {
        if i > 0 {
            // Используем предыдущие метрики как исторические данные
            let historical_data = &metrics_list[..i];
            let historical_metrics: Vec<AppPerformanceMetrics> = historical_data
                .iter()
                .map(|&m| m.clone())
                .collect();

            let anomalies = detect_performance_anomalies(metrics, &historical_metrics, Some(config.clone()));
            anomaly_count += anomalies.len();
        }
    }

    Ok(PerformanceHistoryAnalysis {
        app_group_id: first_metrics.app_group_id.clone(),
        avg_cpu_usage,
        max_cpu_usage,
        avg_memory_mb,
        max_memory_mb,
        avg_io_bytes_per_sec: avg_io_bytes_per_sec as u64,
        max_io_bytes_per_sec,
        avg_fps,
        max_fps,
        avg_latency_ms,
        max_latency_ms,
        avg_gpu_usage_percent,
        max_gpu_usage_percent,
        avg_gpu_memory_mb,
        max_gpu_memory_mb,
        total_render_errors,
        total_dropped_frames,
        avg_performance_score,
        avg_stability_index,
        avg_responsiveness_index,
        anomaly_count,
        period_start: first_metrics.timestamp,
        period_end: last_metrics.timestamp,
    })
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
    config: Option<AppPerformanceConfig>,
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
    config: Option<AppPerformanceConfig>,
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
    config: Option<AppPerformanceConfig>,
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
    config: Option<AppPerformanceConfig>,
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
    history: &[HashMap<String, AppPerformanceMetrics>],
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
    history: &[HashMap<String, AppPerformanceMetrics>],
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
fn analyze_group_trend(metrics_list: &[&AppPerformanceMetrics]) -> GroupPerformanceTrend {
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
    group_trends: &HashMap<String, GroupPerformanceTrend>,
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
pub fn metrics_to_json(metrics: &HashMap<String, AppPerformanceMetrics>) -> Result<String> {
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
pub fn metrics_to_prometheus(metrics: &HashMap<String, AppPerformanceMetrics>) -> String {
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
            io_read_operations: None,
            io_write_operations: None,
            io_total_operations: None,
            io_last_update_ns: None,
            io_data_source: None,
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
            energy_uj: None,
            power_w: None,
            energy_timestamp: None,
            network_rx_bytes: None,
            network_tx_bytes: None,
            network_rx_packets: None,
            network_tx_packets: None,
            network_tcp_connections: None,
            network_udp_connections: None,
            network_last_update_ns: None,
            network_data_source: None,
            gpu_utilization: None,
            gpu_memory_bytes: None,
            gpu_time_us: None,
            gpu_api_calls: None,
            gpu_last_update_ns: None,
            gpu_data_source: None,
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
            assert_eq!(
                group_trend.memory_trend,
                PerformanceTrendDirection::Increasing
            );
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
            fps: None,
            latency_ms: None,
            input_latency_ms: None,
            render_latency_ms: None,
            gpu_usage_percent: None,
            gpu_memory_mb: None,
            render_errors: None,
            dropped_frames: None,
            frame_time_ms: None,
            network_usage_percent: None,
            active_connections: None,
            network_latency_ms: None,
            network_errors: None,
            disk_usage_percent: None,
            disk_latency_ms: None,
            disk_errors: None,
            performance_score: None,
            stability_index: None,
            responsiveness_index: None,
        };

        metrics.insert("test_group".to_string(), app_metrics);

        let prometheus_output = metrics_to_prometheus(&metrics);

        // Проверяем, что вывод содержит ожидаемые метрики
        assert!(prometheus_output.contains("app_performance_cpu_usage{app_group=\"test_group\"}"));
        assert!(prometheus_output.contains("app_performance_memory_mb{app_group=\"test_group\"}"));
        assert!(prometheus_output
            .contains("app_performance_io_bytes_per_sec{app_group=\"test_group\"}"));
        assert!(prometheus_output.contains("app_performance_status{app_group=\"test_group\"}"));
        assert!(
            prometheus_output.contains("app_performance_process_count{app_group=\"test_group\"}")
        );
    }

    #[test]
    fn test_extended_metrics_calculation() {
        // Создаем тестовые процессы с GPU метриками
        let mut process1 = create_test_process(1);
        process1.cpu_share_1s = Some(10.0);
        process1.rss_mb = Some(100);
        process1.io_read_bytes = Some(1000);
        process1.io_write_bytes = Some(2000);
        process1.voluntary_ctx = Some(10);
        process1.involuntary_ctx = Some(5);
        process1.has_gui_window = true;
        process1.gpu_utilization = Some(25.0);
        process1.gpu_memory_bytes = Some(500 * 1024 * 1024); // 500 MB
        process1.gpu_time_us = Some(1000); // 1 ms
        process1.gpu_api_calls = Some(60.0); // 60 FPS
        process1.network_rx_bytes = Some(10000);
        process1.network_tx_bytes = Some(20000);
        process1.network_tcp_connections = Some(5);
        process1.app_group_id = Some("test_group".to_string());

        let mut process2 = create_test_process(2);
        process2.cpu_share_1s = Some(20.0);
        process2.rss_mb = Some(200);
        process2.io_read_bytes = Some(3000);
        process2.io_write_bytes = Some(4000);
        process2.voluntary_ctx = Some(20);
        process2.involuntary_ctx = Some(10);
        process2.is_audio_client = true;
        process2.gpu_utilization = Some(35.0);
        process2.gpu_memory_bytes = Some(700 * 1024 * 1024); // 700 MB
        process2.gpu_time_us = Some(1500); // 1.5 ms
        process2.gpu_api_calls = Some(50.0); // 50 FPS
        process2.network_rx_bytes = Some(15000);
        process2.network_tx_bytes = Some(25000);
        process2.network_tcp_connections = Some(3);
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
        assert_eq!(metrics.total_io_bytes_per_sec, 10000);
        assert_eq!(metrics.total_context_switches, 45);
        assert_eq!(metrics.processes_with_windows, 1);
        assert_eq!(metrics.processes_with_audio, 1);
        assert_eq!(metrics.processes_with_terminals, 0);

        // Проверяем расширенные метрики
        assert!(metrics.fps.is_some());
        assert_eq!(metrics.fps.unwrap(), 55.0); // (60 + 50) / 2

        assert!(metrics.latency_ms.is_some());
        assert_eq!(metrics.latency_ms.unwrap(), 1.25); // (1.0 + 1.5) / 2

        assert!(metrics.input_latency_ms.is_some());
        assert!(metrics.input_latency_ms.unwrap() > 0.0);

        assert!(metrics.render_latency_ms.is_some());
        assert_eq!(metrics.render_latency_ms.unwrap(), 1.25); // (1.0 + 1.5) / 2

        assert!(metrics.gpu_usage_percent.is_some());
        assert_eq!(metrics.gpu_usage_percent.unwrap(), 30.0); // (25 + 35) / 2

        assert!(metrics.gpu_memory_mb.is_some());
        assert_eq!(metrics.gpu_memory_mb.unwrap(), 600); // (500 + 700) / 2

        assert!(metrics.render_errors.is_some());
        assert!(metrics.render_errors.unwrap() > 0);

        assert!(metrics.dropped_frames.is_some());
        assert!(metrics.dropped_frames.unwrap() > 0);

        assert!(metrics.frame_time_ms.is_some());
        assert!(metrics.frame_time_ms.unwrap() > 0.0);

        assert!(metrics.network_usage_percent.is_some());
        assert!(metrics.network_usage_percent.unwrap() > 0.0);

        assert!(metrics.active_connections.is_some());
        assert_eq!(metrics.active_connections.unwrap(), 8); // 5 + 3

        assert!(metrics.network_latency_ms.is_some());
        assert!(metrics.network_latency_ms.unwrap() > 0.0);

        assert!(metrics.network_errors.is_some());
        assert!(metrics.network_errors.unwrap() > 0);

        assert!(metrics.disk_usage_percent.is_some());
        assert!(metrics.disk_usage_percent.unwrap() > 0.0);

        assert!(metrics.disk_latency_ms.is_some());
        assert!(metrics.disk_latency_ms.unwrap() > 0.0);

        assert!(metrics.disk_errors.is_some());
        assert!(metrics.disk_errors.unwrap() > 0);

        // Проверяем индексы производительности
        assert!(metrics.performance_score.is_some());
        assert!(metrics.performance_score.unwrap() > 0);

        assert!(metrics.stability_index.is_some());
        assert!(metrics.stability_index.unwrap() > 0);

        assert!(metrics.responsiveness_index.is_some());
        assert!(metrics.responsiveness_index.unwrap() > 0);

        // Проверяем теги
        assert!(metrics.tags.contains(&"has_windows".to_string()));
        assert!(metrics.tags.contains(&"has_audio".to_string()));
        assert!(metrics.tags.contains(&"has_graphics".to_string()));
        assert!(metrics.tags.contains(&"uses_gpu".to_string()));
    }

    #[test]
    fn test_extended_performance_status() {
        let thresholds = PerformanceThresholds::default();

        // Тест хорошего статуса с расширенными метриками
        let mut metrics = AppPerformanceMetrics {
            total_cpu_usage: 30.0,
            total_memory_mb: 500,
            total_io_bytes_per_sec: 1_000_000,
            fps: Some(60.0),
            latency_ms: Some(20.0),
            gpu_usage_percent: Some(40.0),
            gpu_memory_mb: Some(1000),
            render_errors: Some(5),
            dropped_frames: Some(10),
            network_usage_percent: Some(30.0),
            network_errors: Some(2),
            disk_usage_percent: Some(40.0),
            disk_errors: Some(1),
            performance_score: Some(90),
            stability_index: Some(95),
            responsiveness_index: Some(90),
            ..Default::default()
        };

        let status = determine_extended_performance_status(&metrics, &thresholds);
        assert_eq!(status, PerformanceStatus::Good);

        // Тест статуса предупреждения (несколько метрик)
        metrics.total_cpu_usage = 75.0;
        metrics.latency_ms = Some(60.0);
        let status = determine_extended_performance_status(&metrics, &thresholds);
        assert_eq!(status, PerformanceStatus::Warning);

        // Тест критического статуса (несколько критических метрик)
        metrics.total_cpu_usage = 95.0;
        metrics.total_memory_mb = 2500;
        metrics.latency_ms = Some(120.0);
        let status = determine_extended_performance_status(&metrics, &thresholds);
        assert_eq!(status, PerformanceStatus::Critical);
    }

    #[test]
    fn test_anomaly_detection() {
        let config = AppPerformanceConfig::default();

        // Создаем исторические данные
        let mut historical_metrics = Vec::new();
        for i in 0..5 {
            let metrics = AppPerformanceMetrics {
                app_group_id: "test_group".to_string(),
                total_cpu_usage: 30.0 + i as f64,
                total_memory_mb: 500 + i as u64 * 100,
                total_io_bytes_per_sec: 1_000_000 + i as u64 * 100_000,
                fps: Some(60.0 - i as f64),
                latency_ms: Some(20.0 + i as f64),
                ..Default::default()
            };
            historical_metrics.push(metrics);
        }

        // Создаем текущие метрики с аномалиями
        let current_metrics = AppPerformanceMetrics {
            app_group_id: "test_group".to_string(),
            total_cpu_usage: 80.0, // Значительное отклонение
            total_memory_mb: 1500, // Значительное отклонение
            total_io_bytes_per_sec: 10_000_000, // Значительное отклонение
            fps: Some(30.0), // Значительное отклонение
            latency_ms: Some(100.0), // Значительное отклонение
            ..Default::default()
        };

        let anomalies = detect_performance_anomalies(&current_metrics, &historical_metrics, Some(config));

        // Должны быть обнаружены несколько аномалий
        assert!(!anomalies.is_empty());
        assert!(anomalies.len() >= 3); // CPU, память, ввод-вывод

        // Проверяем, что аномалии имеют правильные метрики
        let anomaly_metrics: Vec<String> = anomalies.iter().map(|a| a.metric.clone()).collect();
        assert!(anomaly_metrics.contains(&"cpu_usage".to_string()));
        assert!(anomaly_metrics.contains(&"memory_usage".to_string()));
        assert!(anomaly_metrics.contains(&"io_usage".to_string()));
        assert!(anomaly_metrics.contains(&"fps".to_string()));
        assert!(anomaly_metrics.contains(&"latency".to_string()));

        // Проверяем серьезность аномалий
        let high_severity_anomalies = anomalies.iter().filter(|a| matches!(a.severity, AnomalySeverity::High)).count();
        assert!(high_severity_anomalies > 0);
    }

    #[test]
    fn test_performance_history_analysis() {
        let config = AppPerformanceConfig::default();

        // Создаем историю метрик
        let mut history = Vec::new();

        for snapshot_idx in 0..3 {
            let mut metrics_map = HashMap::new();

            let metrics = AppPerformanceMetrics {
                app_group_id: "test_group".to_string(),
                app_group_name: "Test App".to_string(),
                process_count: 2,
                total_cpu_usage: 30.0 + snapshot_idx as f64 * 5.0,
                average_cpu_usage: 15.0 + snapshot_idx as f64 * 2.5,
                peak_cpu_usage: 20.0 + snapshot_idx as f64 * 5.0,
                total_memory_mb: 300 + snapshot_idx as u64 * 50,
                average_memory_mb: 150.0 + snapshot_idx as f64 * 25.0,
                total_io_bytes_per_sec: 10000 + snapshot_idx as u64 * 1000,
                total_context_switches: 45 + snapshot_idx as u64 * 5,
                processes_with_windows: 1,
                processes_with_audio: 1,
                processes_with_terminals: 0,
                response_time_ms: None,
                performance_status: PerformanceStatus::Good,
                timestamp: Some(SystemTime::now()),
                tags: vec!["has_windows".to_string(), "has_audio".to_string()],
                fps: Some(60.0 - snapshot_idx as f64),
                latency_ms: Some(20.0 + snapshot_idx as f64),
                input_latency_ms: Some(5.0 + snapshot_idx as f64),
                render_latency_ms: Some(10.0 + snapshot_idx as f64),
                gpu_usage_percent: Some(30.0 + snapshot_idx as f64 * 5.0),
                gpu_memory_mb: Some(500 + snapshot_idx as u64 * 100),
                render_errors: Some(snapshot_idx as u64),
                dropped_frames: Some(snapshot_idx as u64 * 2),
                frame_time_ms: Some(16.67 + snapshot_idx as f64),
                network_usage_percent: Some(20.0 + snapshot_idx as f64 * 5.0),
                active_connections: Some(5 + snapshot_idx),
                network_latency_ms: Some(30.0 + snapshot_idx as f64),
                network_errors: Some(snapshot_idx as u64),
                disk_usage_percent: Some(40.0 + snapshot_idx as f64 * 5.0),
                disk_latency_ms: Some(20.0 + snapshot_idx as f64),
                disk_errors: Some(snapshot_idx as u64),
                performance_score: Some(90 - snapshot_idx as u8),
                stability_index: Some(95 - snapshot_idx as u8),
                responsiveness_index: Some(85 - snapshot_idx as u8),
            };

            metrics_map.insert("test_group".to_string(), metrics);
            history.push(metrics_map);
        }

        // Анализируем историю
        let analysis_result = analyze_performance_history(&history, Some(config));
        assert!(analysis_result.is_ok());

        let analysis = analysis_result.unwrap();
        assert_eq!(analysis.len(), 1);

        if let Some(group_analysis) = analysis.get("test_group") {
            // Проверяем, что анализ содержит ожидаемые значения
            assert_eq!(group_analysis.app_group_id, "test_group");
            assert!(group_analysis.avg_cpu_usage > 0.0);
            assert!(group_analysis.max_cpu_usage > 0.0);
            assert!(group_analysis.avg_memory_mb > 0);
            assert!(group_analysis.max_memory_mb > 0);
            assert!(group_analysis.avg_io_bytes_per_sec > 0);
            assert!(group_analysis.max_io_bytes_per_sec > 0);

            assert!(group_analysis.avg_fps.is_some());
            assert!(group_analysis.max_fps.is_some());
            assert!(group_analysis.avg_latency_ms.is_some());
            assert!(group_analysis.max_latency_ms.is_some());

            assert!(group_analysis.avg_gpu_usage_percent.is_some());
            assert!(group_analysis.max_gpu_usage_percent.is_some());
            assert!(group_analysis.avg_gpu_memory_mb.is_some());
            assert!(group_analysis.max_gpu_memory_mb.is_some());

            assert!(group_analysis.total_render_errors > 0);
            assert!(group_analysis.total_dropped_frames > 0);

            assert!(group_analysis.avg_performance_score.is_some());
            assert!(group_analysis.avg_stability_index.is_some());
            assert!(group_analysis.avg_responsiveness_index.is_some());

            assert!(group_analysis.anomaly_count > 0);
            assert!(group_analysis.period_start.is_some());
            assert!(group_analysis.period_end.is_some());
        }
    }

    #[test]
    fn test_performance_indices_calculation() {
        // Тест с хорошими метриками
        let (score, stability, responsiveness) = calculate_performance_indices(
            30.0, // CPU
            500,  // Memory
            1_000_000, // IO
            Some(60.0), // FPS
            Some(20.0), // Latency
            Some(40.0), // GPU usage
            Some(5),    // Render errors
            Some(10),   // Dropped frames
            Some(2),    // Network errors
            Some(1),    // Disk errors
        );

        assert!(score.is_some());
        assert!(score.unwrap() >= 80); // Должен быть высокий балл

        assert!(stability.is_some());
        assert!(stability.unwrap() >= 80); // Должна быть высокая стабильность

        assert!(responsiveness.is_some());
        assert!(responsiveness.unwrap() >= 80); // Должна быть высокая отзывчивость

        // Тест с плохими метриками
        let (score, stability, responsiveness) = calculate_performance_indices(
            95.0, // High CPU
            3000, // High memory
            60_000_000, // High IO
            Some(10.0), // Low FPS
            Some(150.0), // High latency
            Some(98.0), // High GPU usage
            Some(50),   // Many render errors
            Some(100),  // Many dropped frames
            Some(20),   // Many network errors
            Some(10),   // Many disk errors
        );

        assert!(score.is_some());
        assert!(score.unwrap() < 50); // Должен быть низкий балл

        assert!(stability.is_some());
        assert!(stability.unwrap() < 50); // Должна быть низкая стабильность

        assert!(responsiveness.is_some());
        assert!(responsiveness.unwrap() < 50); // Должна быть низкая отзывчивость
    }
}
