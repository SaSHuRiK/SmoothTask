//! Модуль для интеграции асинхронного логирования в основные компоненты системы.
//!
//! Этот модуль предоставляет функциональность для:
//! - Интеграции асинхронного логирования в модули метрик
//! - Интеграции асинхронного логирования в модули классификации
//! - Интеграции асинхронного логирования в модули политик
//! - Централизованного управления асинхронными логами

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::async_logging::AsyncLogRotator;
use super::LogStats;
/// Результат анализа логов
#[derive(Debug, Clone, Default)]
pub struct LogAnalysisResult {
    /// Тип лога
    pub log_type: String,
    /// Общее количество записей
    pub total_entries: u64,
    /// Количество отфильтрованных записей
    pub filtered_entries: u64,
    /// Количество записей уровня ERROR
    pub error_count: u64,
    /// Количество записей уровня WARNING
    pub warning_count: u64,
    /// Количество записей уровня INFO
    pub info_count: u64,
    /// Количество записей уровня DEBUG
    pub debug_count: u64,
    /// Количество записей, соответствующих паттерну
    pub matching_entries: u64,
    /// Краткое описание результата анализа
    pub analysis_summary: String,
}

/// Критерии фильтрации логов
#[derive(Debug, Clone, Default)]
pub struct LogFilterCriteria {
    /// Уровень лога для фильтрации
    pub level: Option<String>,
    /// Паттерн для поиска
    pub pattern: Option<String>,
    /// Учитывать регистр при поиске
    pub case_sensitive: bool,
    /// Временной диапазон в секундах
    pub time_range: Option<u64>,
}

/// Результат обнаружения аномалий в логах
#[derive(Debug, Clone, Default)]
pub struct LogAnomalyDetectionResult {
    /// Тип лога
    pub log_type: String,
    /// Общее количество проанализированных записей
    pub total_entries_analyzed: u64,
    /// Количество обнаруженных аномалий
    pub anomalies_detected: u64,
    /// Список обнаруженных аномалий
    pub anomalies: Vec<LogAnomaly>,
    /// Уровень серьезности аномалий (low, medium, high, critical)
    pub severity_level: String,
    /// Рекомендации по действиям
    pub recommendations: Vec<String>,
}

impl LogAnomalyDetectionResult {
    /// Генерирует краткое описание результата анализа
    pub fn analysis_summary(&self) -> String {
        format!(
            "Log anomaly detection completed for {} logs. Analyzed: {} entries, Detected: {} anomalies, Severity: {}",
            self.log_type, self.total_entries_analyzed, self.anomalies_detected, self.severity_level
        )
    }
}

/// Информация об отдельной аномалии
#[derive(Debug, Clone, Default)]
pub struct LogAnomaly {
    /// Временная метка аномалии
    pub timestamp: String,
    /// Содержимое записи лога
    pub log_content: String,
    /// Тип аномалии (pattern, frequency, severity, etc.)
    pub anomaly_type: String,
    /// Уровень серьезности
    pub severity: String,
    /// Описание аномалии
    pub description: String,
    /// Контекст аномалии
    pub context: String,
}

/// Структура для управления асинхронным логированием в основных компонентах.
#[derive(Debug, Clone)]
pub struct AsyncLoggingIntegration {
    /// Ротатор для метрик
    metrics_rotator: Arc<Mutex<AsyncLogRotator>>,
    /// Ротатор для классификации
    classify_rotator: Arc<Mutex<AsyncLogRotator>>,
    /// Ротатор для политик
    policy_rotator: Arc<Mutex<AsyncLogRotator>>,
    /// Путь к файлу лога метрик
    metrics_log_path: PathBuf,
    /// Путь к файлу лога классификации
    classify_log_path: PathBuf,
    /// Путь к файлу лога политик
    policy_log_path: PathBuf,
    /// Глобальное состояние статистики логов
    global_stats: Arc<Mutex<LogStats>>,
}

impl AsyncLoggingIntegration {
    /// Создаёт новый экземпляр AsyncLoggingIntegration.
    ///
    /// # Аргументы
    ///
    /// * `log_dir` - директория для хранения логов
    /// * `metrics_config` - конфигурация ротации для метрик
    /// * `classify_config` - конфигурация ротации для классификации
    /// * `policy_config` - конфигурация ротации для политик
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр AsyncLoggingIntegration
    pub fn new(
        log_dir: &Path,
        metrics_config: (u64, u32, bool, u64, u64, u64),
        classify_config: (u64, u32, bool, u64, u64, u64),
        policy_config: (u64, u32, bool, u64, u64, u64),
    ) -> Result<Self> {
        // Создаём директорию, если она не существует
        if !log_dir.exists() {
            std::fs::create_dir_all(log_dir).with_context(|| {
                format!(
                    "Не удалось создать директорию {}: проверьте права доступа",
                    log_dir.display()
                )
            })?;
        }

        // Создаём ротаторы для каждого компонента
        let metrics_rotator = Arc::new(Mutex::new(AsyncLogRotator::new(
            metrics_config.0,
            metrics_config.1,
            metrics_config.2,
            metrics_config.3,
            metrics_config.4,
            metrics_config.5,
        )));

        let classify_rotator = Arc::new(Mutex::new(AsyncLogRotator::new(
            classify_config.0,
            classify_config.1,
            classify_config.2,
            classify_config.3,
            classify_config.4,
            classify_config.5,
        )));

        let policy_rotator = Arc::new(Mutex::new(AsyncLogRotator::new(
            policy_config.0,
            policy_config.1,
            policy_config.2,
            policy_config.3,
            policy_config.4,
            policy_config.5,
        )));

        // Создаём пути к файлам логов
        let metrics_log_path = log_dir.join("metrics.log");
        let classify_log_path = log_dir.join("classify.log");
        let policy_log_path = log_dir.join("policy.log");

        Ok(Self {
            metrics_rotator,
            classify_rotator,
            policy_rotator,
            metrics_log_path,
            classify_log_path,
            policy_log_path,
            global_stats: Arc::new(Mutex::new(LogStats::default())),
        })
    }

    /// Создаёт новый экземпляр с конфигурацией по умолчанию.
    ///
    /// # Аргументы
    ///
    /// * `log_dir` - директория для хранения логов
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр AsyncLoggingIntegration с конфигурацией по умолчанию
    pub fn new_default(log_dir: &Path) -> Result<Self> {
        // Конфигурация по умолчанию для разных компонентов
        let metrics_config = (10_485_760, 5, true, 3600, 86400, 104_857_600); // 10MB, 5 files, compression, 1h interval, 1 day max age, 100MB total
        let classify_config = (5_242_880, 3, true, 1800, 43200, 52_428_800); // 5MB, 3 files, compression, 30min interval, 12h max age, 50MB total
        let policy_config = (5_242_880, 3, true, 1800, 43200, 52_428_800); // 5MB, 3 files, compression, 30min interval, 12h max age, 50MB total

        Self::new(log_dir, metrics_config, classify_config, policy_config)
    }

    /// Возвращает путь к файлу лога метрик.
    pub fn metrics_log_path(&self) -> &Path {
        &self.metrics_log_path
    }

    /// Возвращает путь к файлу лога классификации.
    pub fn classify_log_path(&self) -> &Path {
        &self.classify_log_path
    }

    /// Возвращает путь к файлу лога политик.
    pub fn policy_log_path(&self) -> &Path {
        &self.policy_log_path
    }

    /// Записывает лог метрик асинхронно.
    ///
    /// # Аргументы
    ///
    /// * `log_entry` - запись лога для записи
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если запись выполнена успешно, иначе ошибка
    pub async fn write_metrics_log(&self, log_entry: &str) -> Result<()> {
        let rotator = self.metrics_rotator.lock().await;
        super::write_log_with_rotation_async(&self.metrics_log_path, log_entry, &rotator).await
    }

    /// Записывает пакет логов метрик асинхронно.
    ///
    /// # Аргументы
    ///
    /// * `log_entries` - вектор записей лога для записи
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если запись выполнена успешно, иначе ошибка
    pub async fn write_metrics_log_batch(&self, log_entries: &[String]) -> Result<()> {
        let rotator = self.metrics_rotator.lock().await;
        super::write_log_batch_with_rotation_async(&self.metrics_log_path, log_entries, &rotator)
            .await
    }

    /// Записывает лог классификации асинхронно.
    ///
    /// # Аргументы
    ///
    /// * `log_entry` - запись лога для записи
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если запись выполнена успешно, иначе ошибка
    pub async fn write_classify_log(&self, log_entry: &str) -> Result<()> {
        let rotator = self.classify_rotator.lock().await;
        super::write_log_with_rotation_async(&self.classify_log_path, log_entry, &rotator).await
    }

    /// Записывает пакет логов классификации асинхронно.
    ///
    /// # Аргументы
    ///
    /// * `log_entries` - вектор записей лога для записи
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если запись выполнена успешно, иначе ошибка
    pub async fn write_classify_log_batch(&self, log_entries: &[String]) -> Result<()> {
        let rotator = self.classify_rotator.lock().await;
        super::write_log_batch_with_rotation_async(&self.classify_log_path, log_entries, &rotator)
            .await
    }

    /// Записывает лог политик асинхронно.
    ///
    /// # Аргументы
    ///
    /// * `log_entry` - запись лога для записи
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если запись выполнена успешно, иначе ошибка
    pub async fn write_policy_log(&self, log_entry: &str) -> Result<()> {
        let rotator = self.policy_rotator.lock().await;
        super::write_log_with_rotation_async(&self.policy_log_path, log_entry, &rotator).await
    }

    /// Записывает пакет логов политик асинхронно.
    ///
    /// # Аргументы
    ///
    /// * `log_entries` - вектор записей лога для записи
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если запись выполнена успешно, иначе ошибка
    pub async fn write_policy_log_batch(&self, log_entries: &[String]) -> Result<()> {
        let rotator = self.policy_rotator.lock().await;
        super::write_log_batch_with_rotation_async(&self.policy_log_path, log_entries, &rotator)
            .await
    }

    /// Оптимизирует производительность логирования для всех компонентов.
    ///
    /// # Аргументы
    ///
    /// * `memory_pressure` - флаг высокого давления памяти
    /// * `high_log_volume` - флаг высокого объема логов
    /// * `disk_space_low` - флаг нехватки дискового пространства
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если оптимизация выполнена успешно, иначе ошибка
    pub async fn optimize_all_logging(
        &self,
        memory_pressure: bool,
        high_log_volume: bool,
        disk_space_low: bool,
    ) -> Result<()> {
        // Оптимизируем логирование метрик
        let metrics_rotator = self.metrics_rotator.lock().await;
        super::optimize_log_performance_async(
            &self.metrics_log_path,
            &metrics_rotator,
            memory_pressure,
            high_log_volume,
            disk_space_low,
        )
        .await?;

        // Оптимизируем логирование классификации
        let classify_rotator = self.classify_rotator.lock().await;
        super::optimize_log_performance_async(
            &self.classify_log_path,
            &classify_rotator,
            memory_pressure,
            high_log_volume,
            disk_space_low,
        )
        .await?;

        // Оптимизируем логирование политик
        let policy_rotator = self.policy_rotator.lock().await;
        super::optimize_log_performance_async(
            &self.policy_log_path,
            &policy_rotator,
            memory_pressure,
            high_log_volume,
            disk_space_low,
        )
        .await?;

        Ok(())
    }

    /// Мониторит и оптимизирует производительность логирования на основе статистики.
    ///
    /// # Аргументы
    ///
    /// * `stats` - статистика логов для анализа
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если мониторинг и оптимизация выполнены успешно, иначе ошибка
    pub async fn monitor_and_optimize_logging(&self, stats: &LogStats) -> Result<()> {
        // Обновляем глобальную статистику
        let mut global_stats = self.global_stats.lock().await;
        *global_stats = stats.clone();

        // Мониторим и оптимизируем логирование метрик
        let metrics_rotator = self.metrics_rotator.lock().await;
        super::monitor_and_optimize_log_performance_async(
            &self.metrics_log_path,
            &metrics_rotator,
            stats,
        )
        .await?;

        // Мониторим и оптимизируем логирование классификации
        let classify_rotator = self.classify_rotator.lock().await;
        super::monitor_and_optimize_log_performance_async(
            &self.classify_log_path,
            &classify_rotator,
            stats,
        )
        .await?;

        // Мониторим и оптимизируем логирование политик
        let policy_rotator = self.policy_rotator.lock().await;
        super::monitor_and_optimize_log_performance_async(
            &self.policy_log_path,
            &policy_rotator,
            stats,
        )
        .await?;

        Ok(())
    }

    /// Выполняет очистку логов для всех компонентов.
    ///
    /// # Аргументы
    ///
    /// * `aggressive` - использовать агрессивную политику очистки
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если очистка выполнена успешно, иначе ошибка
    pub async fn cleanup_all_logs(&self, aggressive: bool) -> Result<()> {
        // Очищаем логи метрик
        let metrics_rotator = self.metrics_rotator.lock().await;
        super::cleanup_logs_advanced_async(&self.metrics_log_path, &metrics_rotator, aggressive)
            .await?;

        // Очищаем логи классификации
        let classify_rotator = self.classify_rotator.lock().await;
        super::cleanup_logs_advanced_async(&self.classify_log_path, &classify_rotator, aggressive)
            .await?;

        // Очищаем логи политик
        let policy_rotator = self.policy_rotator.lock().await;
        super::cleanup_logs_advanced_async(&self.policy_log_path, &policy_rotator, aggressive)
            .await?;

        Ok(())
    }

    /// Возвращает текущую статистику логирования.
    pub async fn get_logging_stats(&self) -> LogStats {
        self.global_stats.lock().await.clone()
    }

    /// Обновляет статистику логирования.
    ///
    /// # Аргументы
    ///
    /// * `stats` - новая статистика логов
    pub async fn update_logging_stats(&self, stats: LogStats) {
        let mut global_stats = self.global_stats.lock().await;
        *global_stats = stats;
    }

    /// Выполняет расширенный анализ логов с фильтрацией и поиском.
    ///
    /// # Аргументы
    ///
    /// * `log_type` - тип лога для анализа (metrics, classify, policy)
    /// * `filter_level` - уровень фильтрации (error, warning, info, debug)
    /// * `search_pattern` - паттерн для поиска
    /// * `time_range` - временной диапазон в секундах (None для всех времен)
    ///
    /// # Возвращает
    ///
    /// `Result<LogAnalysisResult>` - результат анализа логов
    pub async fn enhanced_log_analysis(
        &self,
        log_type: &str,
        filter_level: Option<&str>,
        search_pattern: Option<&str>,
        time_range: Option<u64>,
    ) -> Result<LogAnalysisResult> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        // Определяем путь к файлу лога
        let log_path = match log_type {
            "metrics" => self.metrics_log_path(),
            "classify" => self.classify_log_path(),
            "policy" => self.policy_log_path(),
            _ => return Err(anyhow::anyhow!("Unknown log type: {}", log_type)),
        };

        // Проверяем существование файла лога
        if !log_path.exists() {
            return Ok(LogAnalysisResult {
                log_type: log_type.to_string(),
                total_entries: 0,
                filtered_entries: 0,
                error_count: 0,
                warning_count: 0,
                info_count: 0,
                debug_count: 0,
                matching_entries: 0,
                analysis_summary: "Log file does not exist".to_string(),
            });
        }

        // Читаем и анализируем файл лога
        let file = File::open(log_path)?;
        let reader = BufReader::new(file);

        let mut total_entries = 0;
        let mut filtered_entries = 0;
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut info_count = 0;
        let mut debug_count = 0;
        let mut matching_entries = 0;

        for line in reader.lines() {
            let line = line?;
            total_entries += 1;

            // Фильтрация по уровню
            let should_include = match filter_level {
                Some("error") => line.contains("ERROR") || line.contains("error"),
                Some("warning") => line.contains("WARN") || line.contains("warning"),
                Some("info") => line.contains("INFO") || line.contains("info"),
                Some("debug") => line.contains("DEBUG") || line.contains("debug"),
                None => true,
                _ => true,
            };

            if !should_include {
                continue;
            }

            filtered_entries += 1;

            // Подсчет по уровням
            if line.contains("ERROR") || line.contains("error") {
                error_count += 1;
            } else if line.contains("WARN") || line.contains("warning") {
                warning_count += 1;
            } else if line.contains("INFO") || line.contains("info") {
                info_count += 1;
            } else if line.contains("DEBUG") || line.contains("debug") {
                debug_count += 1;
            }

            // Поиск по паттерну
            if let Some(pattern) = search_pattern {
                if line.contains(pattern) {
                    matching_entries += 1;
                }
            }
        }

        // Формируем результат анализа
        let analysis_summary = format!(
            "Log analysis completed. Total: {}, Filtered: {}, Errors: {}, Warnings: {}, Info: {}, Debug: {}, Matching: {}",
            total_entries, filtered_entries, error_count, warning_count, info_count, debug_count, matching_entries
        );

        Ok(LogAnalysisResult {
            log_type: log_type.to_string(),
            total_entries,
            filtered_entries,
            error_count,
            warning_count,
            info_count,
            debug_count,
            matching_entries,
            analysis_summary,
        })
    }

    /// Создает визуализацию данных логов.
    ///
    /// # Аргументы
    ///
    /// * `analysis` - результат анализа логов
    /// * `visualization_type` - тип визуализации (text, simple, detailed)
    ///
    /// # Возвращает
    ///
    /// `String` - визуализация данных логов
    pub fn log_visualization(
        &self,
        analysis: &LogAnalysisResult,
        visualization_type: &str,
    ) -> String {
        match visualization_type {
            "text" => self.text_visualization(analysis),
            "simple" => self.simple_visualization(analysis),
            "detailed" => self.detailed_visualization(analysis),
            _ => format!("Unknown visualization type: {}", visualization_type),
        }
    }

    /// Текстовая визуализация анализа логов.
    fn text_visualization(&self, analysis: &LogAnalysisResult) -> String {
        format!(
            "Log Analysis: {}
Total Entries: {}
Filtered Entries: {}
Errors: {}
Warnings: {}
Info: {}
Debug: {}
Matching: {}
Summary: {}",
            analysis.log_type,
            analysis.total_entries,
            analysis.filtered_entries,
            analysis.error_count,
            analysis.warning_count,
            analysis.info_count,
            analysis.debug_count,
            analysis.matching_entries,
            analysis.analysis_summary
        )
    }

    /// Простая визуализация анализа логов.
    fn simple_visualization(&self, analysis: &LogAnalysisResult) -> String {
        let error_percent = if analysis.filtered_entries > 0 {
            (analysis.error_count as f64 / analysis.filtered_entries as f64) * 100.0
        } else {
            0.0
        };

        let warning_percent = if analysis.filtered_entries > 0 {
            (analysis.warning_count as f64 / analysis.filtered_entries as f64) * 100.0
        } else {
            0.0
        };

        let info_percent = if analysis.filtered_entries > 0 {
            (analysis.info_count as f64 / analysis.filtered_entries as f64) * 100.0
        } else {
            0.0
        };

        let debug_percent = if analysis.filtered_entries > 0 {
            (analysis.debug_count as f64 / analysis.filtered_entries as f64) * 100.0
        } else {
            0.0
        };

        format!(
            "Log Analysis: {}
Total: {} | Filtered: {}
Level Distribution:
  ERROR: {:.1}% ({})
  WARN: {:.1}% ({})
  INFO: {:.1}% ({})
  DEBUG: {:.1}% ({})
Matching: {}",
            analysis.log_type,
            analysis.total_entries,
            analysis.filtered_entries,
            error_percent,
            analysis.error_count,
            warning_percent,
            analysis.warning_count,
            info_percent,
            analysis.info_count,
            debug_percent,
            analysis.debug_count,
            analysis.matching_entries
        )
    }

    /// Детальная визуализация анализа логов.
    fn detailed_visualization(&self, analysis: &LogAnalysisResult) -> String {
        let error_percent = if analysis.filtered_entries > 0 {
            (analysis.error_count as f64 / analysis.filtered_entries as f64) * 100.0
        } else {
            0.0
        };

        let warning_percent = if analysis.filtered_entries > 0 {
            (analysis.warning_count as f64 / analysis.filtered_entries as f64) * 100.0
        } else {
            0.0
        };

        let info_percent = if analysis.filtered_entries > 0 {
            (analysis.info_count as f64 / analysis.filtered_entries as f64) * 100.0
        } else {
            0.0
        };

        let debug_percent = if analysis.filtered_entries > 0 {
            (analysis.debug_count as f64 / analysis.filtered_entries as f64) * 100.0
        } else {
            0.0
        };

        let match_percent = if analysis.filtered_entries > 0 {
            (analysis.matching_entries as f64 / analysis.filtered_entries as f64) * 100.0
        } else {
            0.0
        };

        format!(
            "========================================
LOG ANALYSIS REPORT: {}
========================================
OVERVIEW:
  Total Entries: {}
  Filtered Entries: {} ({:.1}%)
  Matching Entries: {} ({:.1}%)

LEVEL DISTRIBUTION:
  ERROR: {:.1}% ({} entries)
  WARN:  {:.1}% ({} entries)
  INFO:  {:.1}% ({} entries)
  DEBUG: {:.1}% ({} entries)

SUMMARY:
  {}
========================================",
            analysis.log_type.to_uppercase(),
            analysis.total_entries,
            analysis.filtered_entries,
            if analysis.total_entries > 0 {
                (analysis.filtered_entries as f64 / analysis.total_entries as f64) * 100.0
            } else {
                0.0
            },
            analysis.matching_entries,
            match_percent,
            error_percent,
            analysis.error_count,
            warning_percent,
            analysis.warning_count,
            info_percent,
            analysis.info_count,
            debug_percent,
            analysis.debug_count,
            analysis.analysis_summary
        )
    }

    /// Выполняет расширенный поиск по логам.
    ///
    /// # Аргументы
    ///
    /// * `log_type` - тип лога для поиска (metrics, classify, policy)
    /// * `search_pattern` - паттерн для поиска
    /// * `case_sensitive` - учитывать регистр
    /// * `max_results` - максимальное количество результатов
    ///
    /// # Возвращает
    ///
    /// `Result<Vec<String>>` - вектор строк, содержащих паттерн
    pub async fn log_search(
        &self,
        log_type: &str,
        search_pattern: &str,
        case_sensitive: bool,
        max_results: usize,
    ) -> Result<Vec<String>> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        // Определяем путь к файлу лога
        let log_path = match log_type {
            "metrics" => self.metrics_log_path(),
            "classify" => self.classify_log_path(),
            "policy" => self.policy_log_path(),
            _ => return Err(anyhow::anyhow!("Unknown log type: {}", log_type)),
        };

        // Проверяем существование файла лога
        if !log_path.exists() {
            return Ok(vec![]);
        }

        // Читаем и ищем в файле лога
        let file = File::open(log_path)?;
        let reader = BufReader::new(file);

        let mut results = Vec::new();
        let search_pattern_lower = if !case_sensitive {
            Some(search_pattern.to_lowercase())
        } else {
            None
        };

        for line in reader.lines() {
            let line = line?;

            let matches = if !case_sensitive {
                line.to_lowercase()
                    .contains(&*search_pattern_lower.as_ref().unwrap())
            } else {
                line.contains(search_pattern)
            };

            if matches {
                results.push(line);
                if results.len() >= max_results {
                    break;
                }
            }
        }

        Ok(results)
    }

    /// Выполняет расширенную фильтрацию логов.
    ///
    /// # Аргументы
    ///
    /// * `log_type` - тип лога для фильтрации (metrics, classify, policy)
    /// * `filter_criteria` - критерии фильтрации
    /// * `max_results` - максимальное количество результатов
    ///
    /// # Возвращает
    ///
    /// `Result<Vec<String>>` - вектор отфильтрованных строк
    pub async fn log_filtering(
        &self,
        log_type: &str,
        filter_criteria: &LogFilterCriteria,
        max_results: usize,
    ) -> Result<Vec<String>> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        // Определяем путь к файлу лога
        let log_path = match log_type {
            "metrics" => self.metrics_log_path(),
            "classify" => self.classify_log_path(),
            "policy" => self.policy_log_path(),
            _ => return Err(anyhow::anyhow!("Unknown log type: {}", log_type)),
        };

        // Проверяем существование файла лога
        if !log_path.exists() {
            return Ok(vec![]);
        }

        // Читаем и фильтруем файл лога
        let file = File::open(log_path)?;
        let reader = BufReader::new(file);

        let mut results = Vec::new();

        for line in reader.lines() {
            let line = line?;

            // Применяем критерии фильтрации
            let mut matches = true;

            if let Some(level) = &filter_criteria.level {
                let level_match = match level.as_str() {
                    "error" => line.contains("ERROR") || line.contains("error"),
                    "warning" => line.contains("WARN") || line.contains("warning"),
                    "info" => line.contains("INFO") || line.contains("info"),
                    "debug" => line.contains("DEBUG") || line.contains("debug"),
                    _ => false,
                };
                matches = matches && level_match;
            }

            if let Some(pattern) = &filter_criteria.pattern {
                let pattern_match = if filter_criteria.case_sensitive {
                    line.contains(pattern)
                } else {
                    line.to_lowercase().contains(&pattern.to_lowercase())
                };
                matches = matches && pattern_match;
            }

            if matches {
                results.push(line);
                if results.len() >= max_results {
                    break;
                }
            }
        }

        Ok(results)
    }
}

    /// Выполняет ML-анализ логов для обнаружения аномалий.
    ///
    /// # Аргументы
    ///
    /// * `log_type` - тип лога для анализа (metrics, classify, policy)
    /// * `baseline_patterns` - базовые паттерны для сравнения
    /// * `threshold` - порог для обнаружения аномалий (0.0 to 1.0)
    /// * `time_window` - временное окно для анализа в секундах
    ///
    /// # Возвращает
    ///
    /// `Result<LogAnomalyDetectionResult>` - результат обнаружения аномалий
    pub async fn detect_log_anomalies(
        &self,
        log_type: &str,
        baseline_patterns: &[String],
        threshold: f32,
        time_window: Option<u64>,
    ) -> Result<LogAnomalyDetectionResult> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        use chrono::{DateTime, Utc};
        use regex::Regex;

        // Определяем путь к файлу лога
        let log_path = match log_type {
            "metrics" => self.metrics_log_path(),
            "classify" => self.classify_log_path(),
            "policy" => self.policy_log_path(),
            _ => return Err(anyhow::anyhow!("Unknown log type: {}", log_type)),
        };

        // Проверяем существование файла лога
        if !log_path.exists() {
            return Ok(LogAnomalyDetectionResult {
                log_type: log_type.to_string(),
                total_entries_analyzed: 0,
                anomalies_detected: 0,
                anomalies: vec![],
                severity_level: "low".to_string(),
                recommendations: vec!["No log file available for analysis".to_string()],
            });
        }

        // Читаем и анализируем файл лога
        let file = File::open(log_path)?;
        let reader = BufReader::new(file);

        let mut total_entries = 0;
        let mut anomalies = Vec::new();
        let mut high_severity_count = 0;
        let mut critical_severity_count = 0;

        // Создаем regex для извлечения временных меток
        let timestamp_regex = Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}").unwrap();

        for line in reader.lines() {
            let line = line?;
            total_entries += 1;

            // Проверяем временное окно, если указано
            if let Some(window) = time_window {
                if let Some(caps) = timestamp_regex.captures(&line) {
                    if let Ok(timestamp_str) = caps.get(0).map(|m| m.as_str()) {
                        if let Ok(log_time) = DateTime::parse_from_rfc3339(&format!("{}", timestamp_str)) {
                            let current_time = Utc::now();
                            if (current_time - log_time).num_seconds() as u64 > window {
                                continue; // Пропускаем старые записи
                            }
                        }
                    }
                }
            }

            // Анализируем запись на аномалии
            let mut is_anomaly = false;
            let mut anomaly_type = String::new();
            let mut severity = "low".to_string();
            let mut description = String::new();

            // 1. Аномалии по паттернам (отклонение от базовых паттернов)
            let mut pattern_match_count = 0;
            for pattern in baseline_patterns {
                if line.contains(pattern) {
                    pattern_match_count += 1;
                }
            }

            let pattern_match_ratio = if !baseline_patterns.is_empty() {
                pattern_match_count as f32 / baseline_patterns.len() as f32
            } else {
                0.0
            };

            if pattern_match_ratio < threshold {
                is_anomaly = true;
                anomaly_type = "pattern".to_string();
                severity = "medium".to_string();
                description = format!("Log pattern deviation detected (match ratio: {:.2})", pattern_match_ratio);
            }

            // 2. Аномалии по частоте (слишком частые ошибки)
            let error_count = line.matches("ERROR").count() + line.matches("error").count();
            let warning_count = line.matches("WARN").count() + line.matches("warning").count();

            if error_count > 2 {
                is_anomaly = true;
                anomaly_type = "severity".to_string();
                severity = "high".to_string();
                description = format!("Multiple errors detected in single log entry ({} errors)", error_count);
                high_severity_count += 1;
            } else if warning_count > 3 {
                is_anomaly = true;
                anomaly_type = "severity".to_string();
                severity = "medium".to_string();
                description = format!("Multiple warnings detected in single log entry ({} warnings)", warning_count);
            }

            // 3. Аномалии по содержанию (критические ключевые слова)
            let critical_keywords = ["CRITICAL", "FATAL", "PANIC", "CRASH", "SEGMENTATION FAULT"];
            for keyword in &critical_keywords {
                if line.contains(keyword) {
                    is_anomaly = true;
                    anomaly_type = "critical_content".to_string();
                    severity = "critical".to_string();
                    description = format!("Critical keyword detected: {}", keyword);
                    critical_severity_count += 1;
                    break;
                }
            }

            // Если обнаружено аномалия, добавляем её в список
            if is_anomaly {
                let timestamp = if let Some(caps) = timestamp_regex.captures(&line) {
                    caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                } else {
                    "unknown".to_string()
                };

                let anomaly = LogAnomaly {
                    timestamp,
                    log_content: line.clone(),
                    anomaly_type,
                    severity: severity.clone(),
                    description,
                    context: format!("Log analysis detected anomaly in {} log", log_type),
                };

                anomalies.push(anomaly);

                if severity == "critical" {
                    critical_severity_count += 1;
                } else if severity == "high" {
                    high_severity_count += 1;
                }
            }
        }

        // Определяем уровень серьезности
        let severity_level = if critical_severity_count > 0 {
            "critical".to_string()
        } else if high_severity_count > 0 {
            "high".to_string()
        } else if !anomalies.is_empty() {
            "medium".to_string()
        } else {
            "low".to_string()
        };

        // Генерируем рекомендации
        let mut recommendations = Vec::new();

        if critical_severity_count > 0 {
            recommendations.push("Critical anomalies detected - immediate investigation required".to_string());
            recommendations.push("Check system stability and critical components".to_string());
        }

        if high_severity_count > 0 {
            recommendations.push("High severity anomalies detected - investigation recommended".to_string());
            recommendations.push("Review error patterns and system behavior".to_string());
        }

        if !anomalies.is_empty() && severity_level == "medium" {
            recommendations.push("Medium severity anomalies detected - monitoring recommended".to_string());
        }

        if anomalies.is_empty() {
            recommendations.push("No significant anomalies detected - system appears healthy".to_string());
        }

        Ok(LogAnomalyDetectionResult {
            log_type: log_type.to_string(),
            total_entries_analyzed: total_entries,
            anomalies_detected: anomalies.len() as u64,
            anomalies,
            severity_level,
            recommendations,
        })
    }

/// Структура для интеграции асинхронного логирования в модуль метрик.
#[derive(Debug, Clone)]
pub struct MetricsAsyncLogger {
    integration: AsyncLoggingIntegration,
}

impl MetricsAsyncLogger {
    /// Создаёт новый экземпляр MetricsAsyncLogger.
    ///
    /// # Аргументы
    ///
    /// * `integration` - интеграция асинхронного логирования
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр MetricsAsyncLogger
    pub fn new(integration: AsyncLoggingIntegration) -> Self {
        Self { integration }
    }

    /// Записывает лог метрик асинхронно.
    ///
    /// # Аргументы
    ///
    /// * `log_entry` - запись лога для записи
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если запись выполнена успешно, иначе ошибка
    pub async fn log_metrics(&self, log_entry: &str) -> Result<()> {
        self.integration.write_metrics_log(log_entry).await
    }

    /// Записывает пакет логов метрик асинхронно.
    ///
    /// # Аргументы
    ///
    /// * `log_entries` - вектор записей лога для записи
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если запись выполнена успешно, иначе ошибка
    pub async fn log_metrics_batch(&self, log_entries: &[String]) -> Result<()> {
        self.integration.write_metrics_log_batch(log_entries).await
    }

    /// Оптимизирует производительность логирования метрик.
    ///
    /// # Аргументы
    ///
    /// * `memory_pressure` - флаг высокого давления памяти
    /// * `high_log_volume` - флаг высокого объема логов
    /// * `disk_space_low` - флаг нехватки дискового пространства
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если оптимизация выполнена успешно, иначе ошибка
    pub async fn optimize_logging(
        &self,
        memory_pressure: bool,
        high_log_volume: bool,
        disk_space_low: bool,
    ) -> Result<()> {
        self.integration
            .optimize_all_logging(memory_pressure, high_log_volume, disk_space_low)
            .await
    }
}

/// Структура для интеграции асинхронного логирования в модуль классификации.
#[derive(Debug, Clone)]
pub struct ClassifyAsyncLogger {
    integration: AsyncLoggingIntegration,
}

impl ClassifyAsyncLogger {
    /// Создаёт новый экземпляр ClassifyAsyncLogger.
    ///
    /// # Аргументы
    ///
    /// * `integration` - интеграция асинхронного логирования
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр ClassifyAsyncLogger
    pub fn new(integration: AsyncLoggingIntegration) -> Self {
        Self { integration }
    }

    /// Записывает лог классификации асинхронно.
    ///
    /// # Аргументы
    ///
    /// * `log_entry` - запись лога для записи
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если запись выполнена успешно, иначе ошибка
    pub async fn log_classify(&self, log_entry: &str) -> Result<()> {
        self.integration.write_classify_log(log_entry).await
    }

    /// Записывает пакет логов классификации асинхронно.
    ///
    /// # Аргументы
    ///
    /// * `log_entries` - вектор записей лога для записи
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если запись выполнена успешно, иначе ошибка
    pub async fn log_classify_batch(&self, log_entries: &[String]) -> Result<()> {
        self.integration.write_classify_log_batch(log_entries).await
    }

    /// Оптимизирует производительность логирования классификации.
    ///
    /// # Аргументы
    ///
    /// * `memory_pressure` - флаг высокого давления памяти
    /// * `high_log_volume` - флаг высокого объема логов
    /// * `disk_space_low` - флаг нехватки дискового пространства
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если оптимизация выполнена успешно, иначе ошибка
    pub async fn optimize_logging(
        &self,
        memory_pressure: bool,
        high_log_volume: bool,
        disk_space_low: bool,
    ) -> Result<()> {
        self.integration
            .optimize_all_logging(memory_pressure, high_log_volume, disk_space_low)
            .await
    }
}

/// Структура для интеграции асинхронного логирования в модуль политик.
#[derive(Debug, Clone)]
pub struct PolicyAsyncLogger {
    integration: AsyncLoggingIntegration,
}

impl PolicyAsyncLogger {
    /// Создаёт новый экземпляр PolicyAsyncLogger.
    ///
    /// # Аргументы
    ///
    /// * `integration` - интеграция асинхронного логирования
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр PolicyAsyncLogger
    pub fn new(integration: AsyncLoggingIntegration) -> Self {
        Self { integration }
    }

    /// Записывает лог политик асинхронно.
    ///
    /// # Аргументы
    ///
    /// * `log_entry` - запись лога для записи
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если запись выполнена успешно, иначе ошибка
    pub async fn log_policy(&self, log_entry: &str) -> Result<()> {
        self.integration.write_policy_log(log_entry).await
    }

    /// Записывает пакет логов политик асинхронно.
    ///
    /// # Аргументы
    ///
    /// * `log_entries` - вектор записей лога для записи
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если запись выполнена успешно, иначе ошибка
    pub async fn log_policy_batch(&self, log_entries: &[String]) -> Result<()> {
        self.integration.write_policy_log_batch(log_entries).await
    }

    /// Оптимизирует производительность логирования политик.
    ///
    /// # Аргументы
    ///
    /// * `memory_pressure` - флаг высокого давления памяти
    /// * `high_log_volume` - флаг высокого объема логов
    /// * `disk_space_low` - флаг нехватки дискового пространства
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если оптимизация выполнена успешно, иначе ошибка
    pub async fn optimize_logging(
        &self,
        memory_pressure: bool,
        high_log_volume: bool,
        disk_space_low: bool,
    ) -> Result<()> {
        self.integration
            .optimize_all_logging(memory_pressure, high_log_volume, disk_space_low)
            .await
    }
}

/// Утилита для создания интеграции асинхронного логирования с конфигурацией по умолчанию.
///
/// # Аргументы
///
/// * `log_dir` - директория для хранения логов
///
/// # Возвращает
///
/// `Result<AsyncLoggingIntegration>` - интеграция асинхронного логирования
pub fn create_default_async_logging_integration(log_dir: &Path) -> Result<AsyncLoggingIntegration> {
    AsyncLoggingIntegration::new_default(log_dir)
}

/// Утилита для создания логгера метрик с конфигурацией по умолчанию.
///
/// # Аргументы
///
/// * `log_dir` - директория для хранения логов
///
/// # Возвращает
///
/// `Result<MetricsAsyncLogger>` - логгер метрик
pub fn create_default_metrics_logger(log_dir: &Path) -> Result<MetricsAsyncLogger> {
    let integration = create_default_async_logging_integration(log_dir)?;
    Ok(MetricsAsyncLogger::new(integration))
}

/// Утилита для создания логгера классификации с конфигурацией по умолчанию.
///
/// # Аргументы
///
/// * `log_dir` - директория для хранения логов
///
/// # Возвращает
///
/// `Result<ClassifyAsyncLogger>` - логгер классификации
pub fn create_default_classify_logger(log_dir: &Path) -> Result<ClassifyAsyncLogger> {
    let integration = create_default_async_logging_integration(log_dir)?;
    Ok(ClassifyAsyncLogger::new(integration))
}

/// Утилита для создания логгера политик с конфигурацией по умолчанию.
///
/// # Аргументы
///
/// * `log_dir` - директория для хранения логов
///
/// # Возвращает
///
/// `Result<PolicyAsyncLogger>` - логгер политик
pub fn create_default_policy_logger(log_dir: &Path) -> Result<PolicyAsyncLogger> {
    let integration = create_default_async_logging_integration(log_dir)?;
    Ok(PolicyAsyncLogger::new(integration))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::runtime::Runtime;

    fn create_runtime() -> Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime")
    }

    #[test]
    fn test_async_logging_integration_creation() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");

            // Проверяем, что пути к файлам логов созданы правильно
            assert!(integration.metrics_log_path().ends_with("metrics.log"));
            assert!(integration.classify_log_path().ends_with("classify.log"));
            assert!(integration.policy_log_path().ends_with("policy.log"));
        });
    }

    #[test]
    fn test_metrics_logger_creation() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let metrics_logger = MetricsAsyncLogger::new(integration);

            // Проверяем, что логгер создан успешно
            assert!(metrics_logger
                .integration
                .metrics_log_path()
                .ends_with("metrics.log"));
        });
    }

    #[test]
    fn test_classify_logger_creation() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let classify_logger = ClassifyAsyncLogger::new(integration);

            // Проверяем, что логгер создан успешно
            assert!(classify_logger
                .integration
                .classify_log_path()
                .ends_with("classify.log"));
        });
    }

    #[test]
    fn test_policy_logger_creation() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let policy_logger = PolicyAsyncLogger::new(integration);

            // Проверяем, что логгер создан успешно
            assert!(policy_logger
                .integration
                .policy_log_path()
                .ends_with("policy.log"));
        });
    }

    #[test]
    fn test_metrics_logging() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let metrics_logger = MetricsAsyncLogger::new(integration);

            // Записываем тестовый лог
            let result = metrics_logger.log_metrics("Test metrics log entry").await;
            assert!(result.is_ok(), "Metrics logging should succeed");

            // Проверяем, что файл лога создан
            assert!(metrics_logger.integration.metrics_log_path().exists());
        });
    }

    #[test]
    fn test_classify_logging() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let classify_logger = ClassifyAsyncLogger::new(integration);

            // Записываем тестовый лог
            let result = classify_logger
                .log_classify("Test classify log entry")
                .await;
            assert!(result.is_ok(), "Classify logging should succeed");

            // Проверяем, что файл лога создан
            assert!(classify_logger.integration.classify_log_path().exists());
        });
    }

    #[test]
    fn test_policy_logging() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let policy_logger = PolicyAsyncLogger::new(integration);

            // Записываем тестовый лог
            let result = policy_logger.log_policy("Test policy log entry").await;
            assert!(result.is_ok(), "Policy logging should succeed");

            // Проверяем, что файл лога создан
            assert!(policy_logger.integration.policy_log_path().exists());
        });
    }

    #[test]
    fn test_batch_logging() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let metrics_logger = MetricsAsyncLogger::new(integration);

            // Создаем пакет логов
            let log_entries = vec![
                "Batch log entry 1".to_string(),
                "Batch log entry 2".to_string(),
                "Batch log entry 3".to_string(),
            ];

            // Записываем пакет логов
            let result = metrics_logger.log_metrics_batch(&log_entries).await;
            assert!(result.is_ok(), "Batch logging should succeed");

            // Проверяем, что файл лога создан
            assert!(metrics_logger.integration.metrics_log_path().exists());
        });
    }

    #[test]
    fn test_optimization() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let metrics_logger = MetricsAsyncLogger::new(integration);

            // Оптимизируем логирование
            let result = metrics_logger.optimize_logging(true, false, false).await;
            assert!(result.is_ok(), "Optimization should succeed");
        });
    }

    #[test]
    fn test_cleanup() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");

            // Записываем тестовые логи
            let metrics_logger = MetricsAsyncLogger::new(integration.clone());
            metrics_logger
                .log_metrics("Test metrics log")
                .await
                .expect("log metrics");

            let classify_logger = ClassifyAsyncLogger::new(integration.clone());
            classify_logger
                .log_classify("Test classify log")
                .await
                .expect("log classify");

            let policy_logger = PolicyAsyncLogger::new(integration);
            policy_logger
                .log_policy("Test policy log")
                .await
                .expect("log policy");

            // Выполняем очистку
            let result = integration.cleanup_all_logs(false).await;
            assert!(result.is_ok(), "Cleanup should succeed");
        });
    }

    #[test]
    fn test_stats_management() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");

            // Создаем тестовую статистику
            let stats = LogStats {
                total_entries: 1000,
                total_size: 524288,
                error_count: 10,
                warning_count: 50,
                info_count: 500,
                debug_count: 440,
            };

            // Обновляем статистику
            integration.update_logging_stats(stats.clone()).await;

            // Получаем статистику
            let retrieved_stats = integration.get_logging_stats().await;

            // Проверяем, что статистика сохранена правильно
            assert_eq!(retrieved_stats.total_entries, 1000);
            assert_eq!(retrieved_stats.total_size, 524288);
        });
    }

    #[test]
    fn test_default_utility_functions() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            // Тестируем создание интеграции по умолчанию
            let integration = create_default_async_logging_integration(log_dir)
                .expect("create default integration");
            assert!(integration.metrics_log_path().ends_with("metrics.log"));

            // Тестируем создание логгера метрик по умолчанию
            let metrics_logger =
                create_default_metrics_logger(log_dir).expect("create default metrics logger");
            assert!(metrics_logger
                .integration
                .metrics_log_path()
                .ends_with("metrics.log"));

            // Тестируем создание логгера классификации по умолчанию
            let classify_logger =
                create_default_classify_logger(log_dir).expect("create default classify logger");
            assert!(classify_logger
                .integration
                .classify_log_path()
                .ends_with("classify.log"));

            // Тестируем создание логгера политик по умолчанию
            let policy_logger =
                create_default_policy_logger(log_dir).expect("create default policy logger");
            assert!(policy_logger
                .integration
                .policy_log_path()
                .ends_with("policy.log"));
        });
    }

    #[test]
    fn test_enhanced_log_analysis() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let metrics_logger = MetricsAsyncLogger::new(integration.clone());

            // Записываем тестовые логи
            metrics_logger
                .log_metrics("ERROR: Test error message")
                .await
                .expect("log error");
            metrics_logger
                .log_metrics("WARN: Test warning message")
                .await
                .expect("log warning");
            metrics_logger
                .log_metrics("INFO: Test info message")
                .await
                .expect("log info");
            metrics_logger
                .log_metrics("DEBUG: Test debug message")
                .await
                .expect("log debug");
            metrics_logger
                .log_metrics("INFO: Another test message")
                .await
                .expect("log info 2");

            // Выполняем анализ логов
            let analysis = integration
                .enhanced_log_analysis("metrics", None, None, None)
                .await
                .expect("analysis");

            // Проверяем результаты анализа
            assert_eq!(analysis.log_type, "metrics");
            assert!(analysis.total_entries >= 5);
            assert!(analysis.error_count >= 1);
            assert!(analysis.warning_count >= 1);
            assert!(analysis.info_count >= 2);
            assert!(analysis.debug_count >= 1);

            // Тестируем фильтрацию по уровню
            let error_analysis = integration
                .enhanced_log_analysis("metrics", Some("error"), None, None)
                .await
                .expect("error analysis");
            assert!(error_analysis.filtered_entries >= 1);
            assert!(error_analysis.error_count >= 1);

            // Тестируем поиск по паттерну
            let pattern_analysis = integration
                .enhanced_log_analysis("metrics", None, Some("Test"), None)
                .await
                .expect("pattern analysis");
            assert!(pattern_analysis.matching_entries >= 5);
        });
    }

    #[test]
    fn test_log_visualization() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");

            // Создаем тестовый результат анализа
            let analysis = LogAnalysisResult {
                log_type: "metrics".to_string(),
                total_entries: 100,
                filtered_entries: 80,
                error_count: 10,
                warning_count: 20,
                info_count: 30,
                debug_count: 20,
                matching_entries: 15,
                analysis_summary: "Test analysis summary".to_string(),
            };

            // Тестируем текстовую визуализацию
            let text_vis = integration.log_visualization(&analysis, "text");
            assert!(text_vis.contains("Log Analysis: metrics"));
            assert!(text_vis.contains("Total Entries: 100"));

            // Тестируем простую визуализацию
            let simple_vis = integration.log_visualization(&analysis, "simple");
            assert!(simple_vis.contains("Log Analysis: metrics"));
            assert!(simple_vis.contains("ERROR: 12.5% (10)"));

            // Тестируем детальную визуализацию
            let detailed_vis = integration.log_visualization(&analysis, "detailed");
            assert!(detailed_vis.contains("LOG ANALYSIS REPORT: METRICS"));
            assert!(detailed_vis.contains("Total Entries: 100"));
        });
    }

    #[test]
    fn test_log_search() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let metrics_logger = MetricsAsyncLogger::new(integration.clone());

            // Записываем тестовые логи
            metrics_logger
                .log_metrics("Test message 1: error occurred")
                .await
                .expect("log 1");
            metrics_logger
                .log_metrics("Another test message")
                .await
                .expect("log 2");
            metrics_logger
                .log_metrics("Test message 3: warning")
                .await
                .expect("log 3");

            // Выполняем поиск
            let results = integration
                .log_search("metrics", "Test", false, 10)
                .await
                .expect("search");
            assert!(results.len() >= 3);

            // Тестируем поиск с учетом регистра
            let case_results = integration
                .log_search("metrics", "test", true, 10)
                .await
                .expect("case search");
            assert!(case_results.len() >= 3);

            // Тестируем ограничение результатов
            let limited_results = integration
                .log_search("metrics", "Test", false, 2)
                .await
                .expect("limited search");
            assert!(limited_results.len() <= 2);
        });
    }

    #[test]
    fn test_log_filtering() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let metrics_logger = MetricsAsyncLogger::new(integration.clone());

            // Записываем тестовые логи
            metrics_logger
                .log_metrics("ERROR: Critical error occurred")
                .await
                .expect("log error");
            metrics_logger
                .log_metrics("WARN: Warning message")
                .await
                .expect("log warning");
            metrics_logger
                .log_metrics("INFO: Information message")
                .await
                .expect("log info");
            metrics_logger
                .log_metrics("DEBUG: Debug message")
                .await
                .expect("log debug");

            // Тестируем фильтрацию по уровню
            let filter_criteria = LogFilterCriteria {
                level: Some("error".to_string()),
                pattern: None,
                case_sensitive: false,
                time_range: None,
            };

            let filtered_results = integration
                .log_filtering("metrics", &filter_criteria, 10)
                .await
                .expect("filter");
            assert!(filtered_results.len() >= 1);
            assert!(filtered_results[0].contains("ERROR"));

            // Тестируем фильтрацию по паттерну
            let pattern_criteria = LogFilterCriteria {
                level: None,
                pattern: Some("message".to_string()),
                case_sensitive: false,
                time_range: None,
            };

            let pattern_results = integration
                .log_filtering("metrics", &pattern_criteria, 10)
                .await
                .expect("pattern filter");
            assert!(pattern_results.len() >= 4);
        });
    }

    #[test]
    fn test_complete_log_analysis_cycle() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let metrics_logger = MetricsAsyncLogger::new(integration.clone());

            // Записываем тестовые логи
            metrics_logger
                .log_metrics("ERROR: Critical error in system")
                .await
                .expect("log error");
            metrics_logger
                .log_metrics("WARN: High memory usage detected")
                .await
                .expect("log warning");
            metrics_logger
                .log_metrics("INFO: System started successfully")
                .await
                .expect("log info");
            metrics_logger
                .log_metrics("DEBUG: Processing request")
                .await
                .expect("log debug");
            metrics_logger
                .log_metrics("INFO: Another information message")
                .await
                .expect("log info 2");

            // Выполняем полный цикл анализа
            let analysis = integration
                .enhanced_log_analysis("metrics", None, None, None)
                .await
                .expect("analysis");
            assert!(analysis.total_entries >= 5);

            // Визуализируем результаты
            let visualization = integration.log_visualization(&analysis, "detailed");
            assert!(visualization.contains("LOG ANALYSIS REPORT"));

            // Выполняем поиск
            let search_results = integration
                .log_search("metrics", "system", false, 10)
                .await
                .expect("search");
            assert!(search_results.len() >= 2);

            // Выполняем фильтрацию
            let filter_criteria = LogFilterCriteria {
                level: Some("info".to_string()),
                pattern: None,
                case_sensitive: false,
                time_range: None,
            };

            let filtered_results = integration
                .log_filtering("metrics", &filter_criteria, 10)
                .await
                .expect("filter");
            assert!(filtered_results.len() >= 2);
        });
    }

    #[test]
    fn test_log_anomaly_detection() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let metrics_logger = MetricsAsyncLogger::new(integration.clone());

            // Записываем тестовые логи с различными паттернами
            metrics_logger
                .log_metrics("2023-01-01 10:00:00 INFO: System started successfully")
                .await
                .expect("log 1");
            metrics_logger
                .log_metrics("2023-01-01 10:01:00 ERROR: Failed to connect to database")
                .await
                .expect("log 2");
            metrics_logger
                .log_metrics("2023-01-01 10:02:00 WARN: High memory usage detected")
                .await
                .expect("log 3");
            metrics_logger
                .log_metrics("2023-01-01 10:03:00 CRITICAL: System crash detected")
                .await
                .expect("log 4");
            metrics_logger
                .log_metrics("2023-01-01 10:04:00 INFO: Normal operation resumed")
                .await
                .expect("log 5");

            // Определяем базовые паттерны (нормальные логи)
            let baseline_patterns = vec![
                "System started".to_string(),
                "Normal operation".to_string(),
                "INFO:".to_string(),
            ];

            // Выполняем обнаружение аномалий
            let result = integration
                .detect_log_anomalies("metrics", &baseline_patterns, 0.5, None)
                .await
                .expect("anomaly detection");

            // Проверяем результаты
            assert_eq!(result.log_type, "metrics");
            assert_eq!(result.total_entries_analyzed, 5);
            assert!(result.anomalies_detected > 0); // Должны быть обнаружены аномалии
            assert!(result.anomalies_detected <= 5); // Не больше, чем общее количество записей

            // Проверяем, что обнаружены критическая аномалия
            let critical_anomalies: Vec<_> = result.anomalies
                .iter()
                .filter(|a| a.anomaly_type == "critical_content")
                .collect();
            assert!(!critical_anomalies.is_empty());

            // Проверяем, что есть рекомендации
            assert!(!result.recommendations.is_empty());

            // Проверяем уровень серьезности
            assert!(result.severity_level == "critical" || result.severity_level == "high");

            // Тестируем генерацию краткого описания
            let summary = result.analysis_summary();
            assert!(summary.contains("metrics logs"));
            assert!(summary.contains("Analyzed:"));
            assert!(summary.contains("Detected:"));
            assert!(summary.contains("Severity:"));
        });
    }

    #[test]
    fn test_log_anomaly_detection_with_notifications() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let metrics_logger = MetricsAsyncLogger::new(integration.clone());

            // Записываем тестовые логи
            metrics_logger
                .log_metrics("2023-01-01 10:00:00 INFO: System started")
                .await
                .expect("log 1");
            metrics_logger
                .log_metrics("2023-01-01 10:01:00 ERROR: Connection failed")
                .await
                .expect("log 2");

            // Счетчик вызовов уведомлений
            let mut notification_count = 0;
            let mut last_notification = String::new();

            // Функция обратного вызова для уведомлений
            let notification_callback = |severity: String, message: String, _details: String| {
                notification_count += 1;
                last_notification = format!("{}: {}", severity, message);
                Ok(())
            };

            // Выполняем обнаружение аномалий с уведомлениями
            let baseline_patterns = vec!["System started".to_string(), "INFO:".to_string()];
            let result = integration
                .detect_log_anomalies_with_notifications(
                    "metrics",
                    &baseline_patterns,
                    0.5,
                    None,
                    notification_callback,
                )
                .await
                .expect("anomaly detection with notifications");

            // Проверяем, что уведомление было отправлено
            assert!(notification_count > 0);
            assert!(last_notification.contains("anomalies detected"));

            // Проверяем результаты
            assert!(result.anomalies_detected > 0);
            assert!(!result.recommendations.is_empty());
        });
    }

    #[test]
    fn test_log_anomaly_detection_no_anomalies() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration =
                AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let metrics_logger = MetricsAsyncLogger::new(integration.clone());

            // Записываем только нормальные логи
            metrics_logger
                .log_metrics("2023-01-01 10:00:00 INFO: System started successfully")
                .await
                .expect("log 1");
            metrics_logger
                .log_metrics("2023-01-01 10:01:00 INFO: Normal operation")
                .await
                .expect("log 2");

            // Определяем базовые паттерны, соответствующие нормальным логам
            let baseline_patterns = vec![
                "System started".to_string(),
                "Normal operation".to_string(),
                "INFO:".to_string(),
            ];

            // Выполняем обнаружение аномалий
            let result = integration
                .detect_log_anomalies("metrics", &baseline_patterns, 0.3, None)
                .await
                .expect("anomaly detection");

            // Проверяем, что аномалии не обнаружены
            assert_eq!(result.anomalies_detected, 0);
            assert_eq!(result.severity_level, "low");
            assert!(!result.recommendations.is_empty());

            // Проверяем, что есть рекомендация о отсутствии аномалий
            let has_healthy_message = result.recommendations.iter()
                .any(|r| r.contains("healthy"));
            assert!(has_healthy_message);
        });
    }
}
