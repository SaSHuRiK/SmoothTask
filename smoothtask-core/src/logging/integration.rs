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
        super::write_log_batch_with_rotation_async(&self.metrics_log_path, log_entries, &rotator).await
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
        super::write_log_batch_with_rotation_async(&self.classify_log_path, log_entries, &rotator).await
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
        super::write_log_batch_with_rotation_async(&self.policy_log_path, log_entries, &rotator).await
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
    pub async fn optimize_all_logging(&self, memory_pressure: bool, high_log_volume: bool, disk_space_low: bool) -> Result<()> {
        // Оптимизируем логирование метрик
        let metrics_rotator = self.metrics_rotator.lock().await;
        super::optimize_log_performance_async(&self.metrics_log_path, &metrics_rotator, memory_pressure, high_log_volume, disk_space_low).await?;

        // Оптимизируем логирование классификации
        let classify_rotator = self.classify_rotator.lock().await;
        super::optimize_log_performance_async(&self.classify_log_path, &classify_rotator, memory_pressure, high_log_volume, disk_space_low).await?;

        // Оптимизируем логирование политик
        let policy_rotator = self.policy_rotator.lock().await;
        super::optimize_log_performance_async(&self.policy_log_path, &policy_rotator, memory_pressure, high_log_volume, disk_space_low).await?;

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
        super::monitor_and_optimize_log_performance_async(&self.metrics_log_path, &metrics_rotator, stats).await?;

        // Мониторим и оптимизируем логирование классификации
        let classify_rotator = self.classify_rotator.lock().await;
        super::monitor_and_optimize_log_performance_async(&self.classify_log_path, &classify_rotator, stats).await?;

        // Мониторим и оптимизируем логирование политик
        let policy_rotator = self.policy_rotator.lock().await;
        super::monitor_and_optimize_log_performance_async(&self.policy_log_path, &policy_rotator, stats).await?;

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
        super::cleanup_logs_advanced_async(&self.metrics_log_path, &metrics_rotator, aggressive).await?;

        // Очищаем логи классификации
        let classify_rotator = self.classify_rotator.lock().await;
        super::cleanup_logs_advanced_async(&self.classify_log_path, &classify_rotator, aggressive).await?;

        // Очищаем логи политик
        let policy_rotator = self.policy_rotator.lock().await;
        super::cleanup_logs_advanced_async(&self.policy_log_path, &policy_rotator, aggressive).await?;

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
    pub async fn optimize_logging(&self, memory_pressure: bool, high_log_volume: bool, disk_space_low: bool) -> Result<()> {
        self.integration.optimize_all_logging(memory_pressure, high_log_volume, disk_space_low).await
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
    pub async fn optimize_logging(&self, memory_pressure: bool, high_log_volume: bool, disk_space_low: bool) -> Result<()> {
        self.integration.optimize_all_logging(memory_pressure, high_log_volume, disk_space_low).await
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
    pub async fn optimize_logging(&self, memory_pressure: bool, high_log_volume: bool, disk_space_low: bool) -> Result<()> {
        self.integration.optimize_all_logging(memory_pressure, high_log_volume, disk_space_low).await
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
            let integration = AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            
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
            let integration = AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let metrics_logger = MetricsAsyncLogger::new(integration);
            
            // Проверяем, что логгер создан успешно
            assert!(metrics_logger.integration.metrics_log_path().ends_with("metrics.log"));
        });
    }

    #[test]
    fn test_classify_logger_creation() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration = AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let classify_logger = ClassifyAsyncLogger::new(integration);
            
            // Проверяем, что логгер создан успешно
            assert!(classify_logger.integration.classify_log_path().ends_with("classify.log"));
        });
    }

    #[test]
    fn test_policy_logger_creation() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration = AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let policy_logger = PolicyAsyncLogger::new(integration);
            
            // Проверяем, что логгер создан успешно
            assert!(policy_logger.integration.policy_log_path().ends_with("policy.log"));
        });
    }

    #[test]
    fn test_metrics_logging() {
        let runtime = create_runtime();
        let temp_dir = TempDir::new().expect("temp dir");
        let log_dir = temp_dir.path();

        runtime.block_on(async {
            let integration = AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
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
            let integration = AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            let classify_logger = ClassifyAsyncLogger::new(integration);
            
            // Записываем тестовый лог
            let result = classify_logger.log_classify("Test classify log entry").await;
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
            let integration = AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
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
            let integration = AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
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
            let integration = AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
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
            let integration = AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            
            // Записываем тестовые логи
            let metrics_logger = MetricsAsyncLogger::new(integration.clone());
            metrics_logger.log_metrics("Test metrics log").await.expect("log metrics");
            
            let classify_logger = ClassifyAsyncLogger::new(integration.clone());
            classify_logger.log_classify("Test classify log").await.expect("log classify");
            
            let policy_logger = PolicyAsyncLogger::new(integration);
            policy_logger.log_policy("Test policy log").await.expect("log policy");
            
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
            let integration = AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
            
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
            let integration = create_default_async_logging_integration(log_dir).expect("create default integration");
            assert!(integration.metrics_log_path().ends_with("metrics.log"));
            
            // Тестируем создание логгера метрик по умолчанию
            let metrics_logger = create_default_metrics_logger(log_dir).expect("create default metrics logger");
            assert!(metrics_logger.integration.metrics_log_path().ends_with("metrics.log"));
            
            // Тестируем создание логгера классификации по умолчанию
            let classify_logger = create_default_classify_logger(log_dir).expect("create default classify logger");
            assert!(classify_logger.integration.classify_log_path().ends_with("classify.log"));
            
            // Тестируем создание логгера политик по умолчанию
            let policy_logger = create_default_policy_logger(log_dir).expect("create default policy logger");
            assert!(policy_logger.integration.policy_log_path().ends_with("policy.log"));
        });
    }
}