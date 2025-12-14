//! Асинхронный модуль для логирования и управления логами.
//!
//! Этот модуль предоставляет асинхронные версии функциональности логирования
//! для улучшения производительности и уменьшения блокировок в основных потоках.

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs;

use tokio::sync::Mutex;

/// Асинхронная структура для управления ротацией логов.
///
/// Предоставляет асинхронные методы для проверки необходимости ротации,
/// выполнения ротации и управления ротированными файлами.
#[derive(Debug)]
pub struct AsyncLogRotator {
    /// Максимальный размер файла лога в байтах перед ротацией
    max_size_bytes: u64,
    /// Максимальное количество сохраняемых ротированных логов
    max_rotated_files: u32,
    /// Включить сжатие ротированных логов
    compression_enabled: bool,
    /// Интервал ротации логов по времени в секундах
    rotation_interval_sec: u64,
    /// Последний timestamp ротации (для ротации по времени)
    last_rotation_time: Mutex<Option<SystemTime>>,
    /// Максимальный возраст ротированных логов в секундах перед удалением
    max_age_sec: u64,
    /// Максимальный общий размер всех ротированных логов в байтах
    max_total_size_bytes: u64,
    /// Последний timestamp очистки (для политик хранения)
    last_cleanup_time: Mutex<Option<SystemTime>>,
}

impl AsyncLogRotator {
    /// Создаёт новый AsyncLogRotator с указанной конфигурацией.
    ///
    /// # Аргументы
    ///
    /// * `max_size_bytes` - максимальный размер файла лога в байтах перед ротацией
    /// * `max_rotated_files` - максимальное количество сохраняемых ротированных логов
    /// * `compression_enabled` - включить сжатие ротированных логов
    /// * `rotation_interval_sec` - интервал ротации логов по времени в секундах
    /// * `max_age_sec` - максимальный возраст ротированных логов в секундах перед удалением
    /// * `max_total_size_bytes` - максимальный общий размер всех ротированных логов в байтах
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр AsyncLogRotator
    pub fn new(
        max_size_bytes: u64,
        max_rotated_files: u32,
        compression_enabled: bool,
        rotation_interval_sec: u64,
        max_age_sec: u64,
        max_total_size_bytes: u64,
    ) -> Self {
        Self {
            max_size_bytes,
            max_rotated_files,
            compression_enabled,
            rotation_interval_sec,
            last_rotation_time: Mutex::new(None),
            max_age_sec,
            max_total_size_bytes,
            last_cleanup_time: Mutex::new(None),
        }
    }

    /// Проверяет, необходима ли ротация лога (асинхронная версия).
    ///
    /// # Аргументы
    ///
    /// * `log_path` - путь к файлу лога
    /// * `current_size` - текущий размер файла лога в байтах
    ///
    /// # Возвращает
    ///
    /// `true`, если ротация необходима, `false` в противном случае
    pub async fn needs_rotation(&self, _log_path: &Path, current_size: u64) -> Result<bool> {
        // Проверка ротации по размеру
        if self.max_size_bytes > 0 && current_size >= self.max_size_bytes {
            return Ok(true);
        }

        // Проверка ротации по времени
        if self.rotation_interval_sec > 0 {
            let current_time = SystemTime::now();
            let last_rotation = self.last_rotation_time.lock().await;
            
            if let Some(last_time) = *last_rotation {
                if let Ok(duration) = current_time.duration_since(last_time) {
                    if duration.as_secs() >= self.rotation_interval_sec {
                        return Ok(true);
                    }
                }
            } else {
                // Если это первый запуск и ротация по времени включена,
                // сразу выполняем ротацию, чтобы установить базовый timestamp
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Выполняет ротацию лога асинхронно.
    ///
    /// Создаёт новый файл лога, перемещает текущий файл в ротированный,
    /// при необходимости сжимает его и удаляет старые ротированные файлы.
    ///
    /// # Аргументы
    ///
    /// * `log_path` - путь к текущему файлу лога
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если ротация выполнена успешно, иначе ошибка
    pub async fn rotate_log(&self, log_path: &Path) -> Result<()> {
        // Проверяем, что файл существует
        if !fs::try_exists(log_path).await? {
            return Ok(()); // Нет файла для ротации
        }

        let metadata = fs::metadata(log_path).await.with_context(|| {
            format!(
                "Не удалось получить метаданные файла лога {}: проверьте, что файл существует и доступен для чтения",
                log_path.display()
            )
        })?;

        // Проверяем, что это файл, а не директория
        if !metadata.is_file() {
            tracing::warn!("Путь {} не является файлом, пропускаем ротацию", log_path.display());
            return Ok(()); // Не файл, пропускаем ротацию
        }

        let current_size = metadata.len();

        // Проверяем, необходима ли ротация
        if !self.needs_rotation(log_path, current_size).await? {
            return Ok(()); // Ротация не нужна
        }

        // Генерируем timestamp для ротированного файла
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();

        // Создаём путь для ротированного файла
        let rotated_path = self.generate_rotated_path(log_path, &timestamp).await;

        // Перемещаем текущий файл в ротированный
        fs::rename(log_path, &rotated_path).await.with_context(|| {
            format!(
                "Не удалось переместить файл лога {} в {}: проверьте права доступа",
                log_path.display(),
                rotated_path.display()
            )
        })?;

        // Если включено сжатие, сжимаем ротированный файл асинхронно
        if self.compression_enabled {
            self.compress_log_file(&rotated_path).await?;
        }

        // Удаляем старые ротированные файлы, если превышен лимит
        self.cleanup_old_logs(log_path).await?;

        // Обновляем время последней ротации
        let mut last_rotation = self.last_rotation_time.lock().await;
        *last_rotation = Some(SystemTime::now());

        Ok(())
    }

    /// Генерирует путь для ротированного файла (асинхронная версия).
    ///
    /// # Аргументы
    ///
    /// * `original_path` - исходный путь к файлу лога
    /// * `timestamp` - timestamp для ротированного файла
    ///
    /// # Возвращает
    ///
    /// Путь к ротированному файлу
    async fn generate_rotated_path(&self, original_path: &Path, timestamp: &str) -> PathBuf {
        let mut rotated_path = original_path.to_path_buf();
        let file_stem = original_path.file_stem().unwrap_or_else(|| "log".as_ref());
        let extension = original_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("log");

        // Формируем имя ротированного файла: original_name.timestamp.extension
        rotated_path.set_file_name(format!(
            "{}.{}.{}",
            file_stem.to_string_lossy(),
            timestamp,
            extension
        ));
        rotated_path
    }

    /// Сжимает файл лога с использованием gzip (асинхронная версия).
    ///
    /// # Аргументы
    ///
    /// * `file_path` - путь к файлу для сжатия
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если сжатие выполнено успешно, иначе ошибка
    async fn compress_log_file(&self, file_path: &Path) -> Result<()> {
        let compressed_path = file_path.with_extension("gz");
        let file_path_buf = file_path.to_path_buf();
        let compressed_path_buf = compressed_path.to_path_buf();

        // Для сжатия используем синхронный код в асинхронном контексте
        // с помощью spawn_blocking для предотвращения блокировки async runtime
        tokio::task::spawn_blocking(move || {
            let input_file = std::fs::File::open(&file_path_buf).with_context(|| {
                format!(
                    "Не удалось открыть файл {} для сжатия: проверьте права доступа",
                    file_path_buf.display()
                )
            })?;

            let output_file = std::fs::File::create(&compressed_path_buf).with_context(|| {
                format!(
                    "Не удалось создать сжатый файл {}: проверьте права доступа",
                    compressed_path_buf.display()
                )
            })?;

            let mut encoder = GzEncoder::new(output_file, Compression::default());
            let mut reader = std::io::BufReader::new(input_file);

            std::io::copy(&mut reader, &mut encoder).with_context(|| {
                format!(
                    "Не удалось сжать файл {}: ошибка сжатия",
                    file_path_buf.display()
                )
            })?;

            encoder.finish().with_context(|| {
                format!(
                    "Не удалось завершить сжатие файла {}: ошибка завершения",
                    file_path_buf.display()
                )
            })?;

            // Удаляем оригинальный файл после успешного сжатия
            std::fs::remove_file(&file_path_buf).with_context(|| {
                format!(
                    "Не удалось удалить оригинальный файл {} после сжатия: проверьте права доступа",
                    file_path_buf.display()
                )
            })?;

            Ok(())
        }).await?
    }

    /// Удаляет старые ротированные файлы, если превышен лимит (асинхронная версия).
    ///
    /// # Аргументы
    ///
    /// * `log_path` - путь к основному файлу лога (используется для поиска ротированных файлов)
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если очистка выполнена успешно, иначе ошибка
    async fn cleanup_old_logs(&self, log_path: &Path) -> Result<()> {
        if self.max_rotated_files == 0 {
            return Ok(()); // Нет ограничения на количество файлов
        }

        let log_dir = log_path.parent().unwrap_or_else(|| Path::new("."));
        let file_stem = log_path.file_stem().unwrap_or_else(|| "log".as_ref());
        let _extension = log_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("log");

        // Ищем все ротированные файлы, соответствующие шаблону
        let mut rotated_files: Vec<(PathBuf, DateTime<Local>)> = Vec::new();

        let mut entries = fs::read_dir(log_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                // Проверяем, соответствует ли файл шаблону: file_stem.timestamp.extension[.gz]
                if file_name.starts_with(&format!("{}.", file_stem.to_string_lossy())) {
                    // Извлекаем timestamp из имени файла
                    let parts: Vec<&str> = file_name.split('.').collect();
                    if parts.len() >= 3 {
                        let timestamp_part = parts[parts.len() - 2]; // Предпоследняя часть - timestamp
                        if let Ok(timestamp) = parse_log_timestamp(timestamp_part) {
                            rotated_files.push((path, timestamp));
                        }
                    }
                }
            }
        }

        // Сортируем файлы по времени (от старых к новым)
        rotated_files.sort_by(|a, b| a.1.cmp(&b.1));

        // Удаляем старые файлы, если превышен лимит
        if rotated_files.len() > self.max_rotated_files as usize {
            let files_to_delete = rotated_files.len() - self.max_rotated_files as usize;
            for (file_path, _) in rotated_files.into_iter().take(files_to_delete) {
                fs::remove_file(&file_path).await.with_context(|| {
                    format!(
                        "Не удалось удалить старый файл лога {}: проверьте права доступа",
                        file_path.display()
                    )
                })?;
            }
        }

        Ok(())
    }

    /// Удаляет ротированные файлы, превышающие максимальный возраст (асинхронная версия).
    ///
    /// # Аргументы
    ///
    /// * `log_path` - путь к основному файлу лога (используется для поиска ротированных файлов)
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если очистка выполнена успешно, иначе ошибка
    async fn cleanup_by_age(&self, log_path: &Path) -> Result<()> {
        if self.max_age_sec == 0 {
            return Ok(()); // Очистка по возрасту отключена
        }

        let log_dir = log_path.parent().unwrap_or_else(|| Path::new("."));
        let file_stem = log_path.file_stem().unwrap_or_else(|| "log".as_ref());

        // Ищем все ротированные файлы, соответствующие шаблону
        let mut rotated_files: Vec<(PathBuf, DateTime<Local>)> = Vec::new();

        let mut entries = fs::read_dir(log_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                // Проверяем, соответствует ли файл шаблону: file_stem.timestamp.extension[.gz]
                if file_name.starts_with(&format!("{}.", file_stem.to_string_lossy())) {
                    // Извлекаем timestamp из имени файла
                    let parts: Vec<&str> = file_name.split('.').collect();
                    if parts.len() >= 3 {
                        let timestamp_part = parts[parts.len() - 2]; // Предпоследняя часть - timestamp
                        if let Ok(timestamp) = parse_log_timestamp(timestamp_part) {
                            rotated_files.push((path, timestamp));
                        }
                    }
                }
            }
        }

        // Удаляем файлы, превышающие максимальный возраст
        let current_time = Local::now();
        let max_age_duration = chrono::Duration::seconds(self.max_age_sec as i64);
        let cutoff_time = current_time - max_age_duration;

        for (file_path, file_time) in rotated_files {
            if file_time < cutoff_time {
                fs::remove_file(&file_path).await.with_context(|| {
                    format!(
                        "Не удалось удалить старый файл лога {} (превышен максимальный возраст): проверьте права доступа",
                        file_path.display()
                    )
                })?;
            }
        }

        Ok(())
    }

    /// Удаляет ротированные файлы, превышающие максимальный общий размер (асинхронная версия).
    ///
    /// # Аргументы
    ///
    /// * `log_path` - путь к основному файлу лога (используется для поиска ротированных файлов)
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если очистка выполнена успешно, иначе ошибка
    async fn cleanup_by_total_size(&self, log_path: &Path) -> Result<()> {
        if self.max_total_size_bytes == 0 {
            return Ok(()); // Ограничение по общему размеру отключено
        }

        let log_dir = log_path.parent().unwrap_or_else(|| Path::new("."));
        let file_stem = log_path.file_stem().unwrap_or_else(|| "log".as_ref());

        // Ищем все ротированные файлы, соответствующие шаблону
        let mut rotated_files: Vec<(PathBuf, DateTime<Local>, u64)> = Vec::new();

        let mut entries = fs::read_dir(log_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                // Проверяем, соответствует ли файл шаблону: file_stem.timestamp.extension[.gz]
                if file_name.starts_with(&format!("{}.", file_stem.to_string_lossy())) {
                    // Извлекаем timestamp из имени файла
                    let parts: Vec<&str> = file_name.split('.').collect();
                    if parts.len() >= 3 {
                        let timestamp_part = parts[parts.len() - 2]; // Предпоследняя часть - timestamp
                        if let Ok(timestamp) = parse_log_timestamp(timestamp_part) {
                            // Получаем размер файла
                            if let Ok(metadata) = fs::metadata(&path).await {
                                rotated_files.push((path, timestamp, metadata.len()));
                            }
                        }
                    }
                }
            }
        }

        // Сортируем файлы по времени (от старых к новым)
        rotated_files.sort_by(|a, b| a.1.cmp(&b.1));

        // Удаляем старые файлы, если превышен лимит по общему размеру
        let mut total_size: u64 = rotated_files.iter().map(|(_, _, size)| size).sum();
        
        while total_size > self.max_total_size_bytes && !rotated_files.is_empty() {
            let (file_path, _, file_size) = rotated_files.remove(0); // Удаляем самый старый файл
            fs::remove_file(&file_path).await.with_context(|| {
                format!(
                    "Не удалось удалить старый файл лога {} (превышен максимальный общий размер): проверьте права доступа",
                    file_path.display()
                )
            })?;
            total_size -= file_size;
        }

        Ok(())
    }

    /// Выполняет полную очистку согласно политикам хранения (асинхронная версия).
    ///
    /// # Аргументы
    ///
    /// * `log_path` - путь к основному файлу лога (используется для поиска ротированных файлов)
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если очистка выполнена успешно, иначе ошибка
    pub async fn cleanup_logs(&self, log_path: &Path) -> Result<()> {
        // Выполняем очистку по возрасту
        self.cleanup_by_age(log_path).await?;
        
        // Выполняем очистку по общему размеру
        self.cleanup_by_total_size(log_path).await?;
        
        // Выполняем очистку по количеству файлов
        self.cleanup_old_logs(log_path).await?;
        
        // Обновляем время последней очистки
        let mut last_cleanup = self.last_cleanup_time.lock().await;
        *last_cleanup = Some(SystemTime::now());
        
        Ok(())
    }

    /// Возвращает текущую конфигурацию ротации.
    ///
    /// # Возвращает
    ///
    /// Клон текущей конфигурации
    pub fn get_config(&self) -> (u64, u32, bool, u64, u64, u64) {
        (
            self.max_size_bytes,
            self.max_rotated_files,
            self.compression_enabled,
            self.rotation_interval_sec,
            self.max_age_sec,
            self.max_total_size_bytes,
        )
    }

    /// Обновляет конфигурацию ротации.
    ///
    /// # Аргументы
    ///
    /// * `max_size_bytes` - новый максимальный размер файла лога
    /// * `max_rotated_files` - новое максимальное количество ротированных файлов
    /// * `compression_enabled` - новый флаг сжатия
    /// * `rotation_interval_sec` - новый интервал ротации по времени
    /// * `max_age_sec` - новый максимальный возраст ротированных логов
    /// * `max_total_size_bytes` - новый максимальный общий размер ротированных логов
    pub fn update_config(
        &self,
        max_size_bytes: u64,
        max_rotated_files: u32,
        compression_enabled: bool,
        rotation_interval_sec: u64,
        max_age_sec: u64,
        max_total_size_bytes: u64,
    ) -> Result<()> {
        // Note: This is a simplified version since we're using Mutex for thread safety
        // In a more complex implementation, we might want to use Arc<Mutex<Self>>
        // But for now, we'll keep it simple and document the limitation
        
        // Actually, since we're using Mutex for internal state, we can't easily
        // update the config without breaking the API. Let's implement this properly.
        
        // For async compatibility, we'll need to use Arc<Mutex<Self>> for full mutability
        // But that would be a breaking change. Let's document this limitation.
        
        // For now, we'll return an error to indicate this operation isn't supported
        // in the async version for simplicity
        Err(anyhow::anyhow!(
            "update_config не поддерживается в AsyncLogRotator. Используйте синхронную версию или создайте новый экземпляр."
        ))
    }
}

/// Асинхронная утилита для получения текущего размера файла лога.
///
/// # Аргументы
///
/// * `log_path` - путь к файлу лога
///
/// # Возвращает
///
/// `Result<u64>` - размер файла в байтах, если файл существует, иначе 0
pub async fn get_log_file_size_async(log_path: &Path) -> Result<u64> {
    if fs::try_exists(log_path).await? {
        let metadata = fs::metadata(log_path).await.with_context(|| {
            format!(
                "Не удалось получить метаданные файла лога {}: проверьте, что файл существует и доступен для чтения",
                log_path.display()
            )
        })?;
        Ok(metadata.len())
    } else {
        Ok(0)
    }
}

/// Парсит timestamp из имени ротированного файла лога (общая функция).
///
/// # Аргументы
///
/// * `timestamp_str` - строка с timestamp в формате YYYYMMDD_HHMMSS
///
/// # Возвращает
///
/// `Result<DateTime<Local>>` - DateTime, если парсинг успешен, иначе ошибка
fn parse_log_timestamp(timestamp_str: &str) -> Result<DateTime<Local>> {
    // Пробуем разные форматы timestamp
    let formats = [
        "%Y%m%d_%H%M%S", // YYYYMMDD_HHMMSS
        "%Y%m%d_%H%M",   // YYYYMMDD_HHMM
        "%Y%m%d",        // YYYYMMDD
    ];

    for format in formats {
        if let Ok(datetime) = DateTime::parse_from_str(timestamp_str, format) {
            return Ok(datetime.with_timezone(&Local));
        }
    }

    Err(anyhow::anyhow!(
        "Не удалось разобрать timestamp '{}' в имени файла лога: ожидается формат YYYYMMDD_HHMMSS, YYYYMMDD_HHMM или YYYYMMDD",
        timestamp_str
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};
    use tokio::runtime::Runtime;

    fn create_runtime() -> Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime")
    }

    #[test]
    fn test_async_log_rotator_creation() {
        let runtime = create_runtime();
        
        runtime.block_on(async {
            let rotator = AsyncLogRotator::new(10_485_760, 5, true, 3600, 0, 0);
            assert_eq!(rotator.max_size_bytes, 10_485_760);
            assert_eq!(rotator.max_rotated_files, 5);
            assert!(rotator.compression_enabled);
            assert_eq!(rotator.rotation_interval_sec, 3600);
            assert_eq!(rotator.max_age_sec, 0);
            assert_eq!(rotator.max_total_size_bytes, 0);
        });
    }

    #[test]
    fn test_async_needs_rotation_by_size() {
        let runtime = create_runtime();
        
        runtime.block_on(async {
            let rotator = AsyncLogRotator::new(1000, 5, true, 0, 0, 0); // Ротация по размеру, 1000 байт

            // Файл меньше лимита - ротация не нужна
            assert!(!rotator
                .needs_rotation(Path::new("/tmp/test.log"), 500)
                .await
                .unwrap());

            // Файл равен лимиту - ротация нужна
            assert!(rotator
                .needs_rotation(Path::new("/tmp/test.log"), 1000)
                .await
                .unwrap());

            // Файл больше лимита - ротация нужна
            assert!(rotator
                .needs_rotation(Path::new("/tmp/test.log"), 1500)
                .await
                .unwrap());
        });
    }

    #[test]
    fn test_async_rotate_log_file() {
        let runtime = create_runtime();
        
        runtime.block_on(async {
            let temp_dir = TempDir::new().expect("temp dir");
            let log_path = temp_dir.path().join("test.log");

            // Создаём тестовый файл лога с достаточным размером для ротации
            let mut file = std::fs::File::create(&log_path).expect("create log file");
            for i in 0..200 {
                writeln!(file, "Test log entry {}", i).expect("write to log");
            }
            drop(file);

            let rotator = AsyncLogRotator::new(100, 3, false, 0, 0, 0); // Ротация по размеру (100 байт), сжатие отключено

            // Выполняем ротацию
            rotator
                .rotate_log(&log_path)
                .await
                .expect("rotation should succeed");

            // Проверяем, что оригинальный файл удалён
            assert!(!log_path.exists(), "Original log file should be removed");

            // Проверяем, что создан ротированный файл
            let rotated_files: Vec<_> = std::fs::read_dir(temp_dir.path())
                .expect("read dir")
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "log"))
                .collect();

            assert_eq!(rotated_files.len(), 1, "Should have one rotated log file");
        });
    }

    #[test]
    fn test_async_get_log_file_size() {
        let runtime = create_runtime();
        
        runtime.block_on(async {
            let temp_file = NamedTempFile::new().expect("temp file");
            let log_path = temp_file.path();

            // Проверяем размер существующего файла
            let size = get_log_file_size_async(log_path).await.expect("get size");
            assert_eq!(size, 0, "New file should be empty");

            // Записываем данные и проверяем размер
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(log_path)
                .expect("open file");
            writeln!(file, "Test data").expect("write data");
            drop(file);

            let new_size = get_log_file_size_async(log_path).await.expect("get new size");
            assert!(new_size > 0, "File should have non-zero size after writing");
        });
    }

    #[test]
    fn test_async_rotator_config() {
        let runtime = create_runtime();
        
        runtime.block_on(async {
            let rotator = AsyncLogRotator::new(1000, 3, false, 0, 0, 0);

            // Проверяем, что конфигурация доступна
            let (max_size, max_files, compression, interval, max_age, max_total_size) = rotator.get_config();
            assert_eq!(max_size, 1000);
            assert_eq!(max_files, 3);
            assert!(!compression);
            assert_eq!(interval, 0);
            assert_eq!(max_age, 0);
            assert_eq!(max_total_size, 0);
        });
    }

    #[test]
    fn test_async_rotation_disabled() {
        let runtime = create_runtime();
        
        runtime.block_on(async {
            let rotator = AsyncLogRotator::new(0, 0, false, 0, 0, 0); // Все отключено

            // Ротация не нужна в любом случае
            assert!(!rotator
                .needs_rotation(Path::new("/tmp/test.log"), 10_000_000)
                .await
                .unwrap());
        });
    }

    #[test]
    fn test_async_error_handling() {
        let runtime = create_runtime();
        
        runtime.block_on(async {
            let temp_dir = TempDir::new().expect("temp dir");
            let log_path = temp_dir.path().join("test.log");

            // Создаём тестовый файл лога
            let mut file = std::fs::File::create(&log_path).expect("create log file");
            writeln!(file, "Test log entry").expect("write to log");
            drop(file);

            let rotator = AsyncLogRotator::new(100, 3, false, 0, 0, 0);

            // Тестируем ротацию с несуществующим файлом (должно завершиться успешно)
            let non_existent_path = temp_dir.path().join("non_existent.log");
            let result = rotator.rotate_log(&non_existent_path).await;
            assert!(result.is_ok(), "Rotation of non-existent file should succeed");
        });
    }

    #[test]
    fn test_async_update_config_not_supported() {
        let runtime = create_runtime();
        
        runtime.block_on(async {
            let temp_dir = TempDir::new().expect("temp dir");
            let log_path = temp_dir.path().join("test.log");
            
            let rotator = AsyncLogRotator::new(1000, 3, false, 0, 0, 0);

            // Проверяем, что update_config возвращает ошибку
            let result = rotator.update_config(5000, 10, true, 3600, 86400, 1_073_741_824);
            assert!(result.is_err(), "update_config should return an error in async version");
        });
    }

    #[test]
    fn test_async_cleanup_error_handling() {
        let runtime = create_runtime();
        
        runtime.block_on(async {
            let temp_dir = TempDir::new().expect("temp dir");
            let log_path = temp_dir.path().join("test.log");

            // Создаём тестовый файл лога
            let mut file = std::fs::File::create(&log_path).expect("create log file");
            writeln!(file, "Test log entry").expect("write to log");
            drop(file);

            let rotator = AsyncLogRotator::new(100, 3, false, 0, 0, 0);

            // Тестируем очистку с несуществующим файлом (должно завершиться успешно)
            let non_existent_path = temp_dir.path().join("non_existent.log");
            let result = rotator.cleanup_logs(&non_existent_path).await;
            assert!(result.is_ok(), "Cleanup of non-existent file should succeed");
        });
    }
}
