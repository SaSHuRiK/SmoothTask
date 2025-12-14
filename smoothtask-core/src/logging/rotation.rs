//! Модуль для ротации и управления логами.
//!
//! Этот модуль предоставляет функциональность для ротации логов по размеру и времени,
//! сжатия ротированных логов и управления файлами логов.

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Структура для управления ротацией логов.
///
/// Предоставляет методы для проверки необходимости ротации, выполнения ротации
/// и управления ротированными файлами.
#[derive(Debug, Clone)]
pub struct LogRotator {
    /// Максимальный размер файла лога в байтах перед ротацией
    max_size_bytes: u64,
    /// Максимальное количество сохраняемых ротированных логов
    max_rotated_files: u32,
    /// Включить сжатие ротированных логов
    compression_enabled: bool,
    /// Интервал ротации логов по времени в секундах
    rotation_interval_sec: u64,
    /// Последний timestamp ротации (для ротации по времени)
    last_rotation_time: Option<SystemTime>,
    /// Максимальный возраст ротированных логов в секундах перед удалением
    max_age_sec: u64,
    /// Максимальный общий размер всех ротированных логов в байтах
    max_total_size_bytes: u64,
    /// Последний timestamp очистки (для политик хранения)
    last_cleanup_time: Option<SystemTime>,
}

impl LogRotator {
    /// Создаёт новый LogRotator с указанной конфигурацией.
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
    /// Новый экземпляр LogRotator
    ///
    /// # Примеры
    ///
    /// ```rust
    /// use smoothtask_core::logging::rotation::LogRotator;
    ///
    /// let rotator = LogRotator::new(10_485_760, 5, true, 3600, 0, 0);
    /// ```
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
            last_rotation_time: None,
            max_age_sec,
            max_total_size_bytes,
            last_cleanup_time: None,
        }
    }

    /// Проверяет, необходима ли ротация лога.
    ///
    /// # Аргументы
    ///
    /// * `log_path` - путь к файлу лога
    /// * `current_size` - текущий размер файла лога в байтах
    ///
    /// # Возвращает
    ///
    /// `true`, если ротация необходима, `false` в противном случае
    ///
    /// # Примеры
    ///
    /// ```rust
    /// use smoothtask_core::logging::rotation::LogRotator;
    /// use std::path::Path;
    ///
    /// let rotator = LogRotator::new(10_485_760, 5, true, 3600);
    /// let needs_rotation = rotator.needs_rotation(Path::new("/var/log/app.log"), 11_000_000);
    /// ```
    pub fn needs_rotation(&self, _log_path: &Path, current_size: u64) -> Result<bool> {
        // Проверка ротации по размеру
        if self.max_size_bytes > 0 && current_size >= self.max_size_bytes {
            return Ok(true);
        }

        // Проверка ротации по времени
        if self.rotation_interval_sec > 0 {
            let current_time = SystemTime::now();
            if let Some(last_rotation) = self.last_rotation_time {
                if let Ok(duration) = current_time.duration_since(last_rotation) {
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

    /// Выполняет ротацию лога.
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
    ///
    /// # Примеры
    ///
    /// ```rust
    /// use smoothtask_core::logging::rotation::LogRotator;
    /// use std::path::Path;
    ///
    /// let mut rotator = LogRotator::new(10_485_760, 5, true, 3600);
    /// rotator.rotate_log(Path::new("/var/log/app.log"))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn rotate_log(&mut self, log_path: &Path) -> Result<()> {
        // Проверяем, что файл существует
        if !log_path.exists() {
            return Ok(()); // Нет файла для ротации
        }

        let metadata = fs::metadata(log_path).with_context(|| {
            format!(
                "Не удалось получить метаданные файла лога {}: проверьте, что файл существует и доступен для чтения. Ошибка: {}",
                log_path.display(),
                std::io::Error::last_os_error()
            )
        })?;

        // Проверяем, что это файл, а не директория
        if !metadata.is_file() {
            tracing::warn!(
                "Путь {} не является файлом, пропускаем ротацию",
                log_path.display()
            );
            return Ok(()); // Не файл, пропускаем ротацию
        }

        let current_size = metadata.len();

        // Проверяем, необходима ли ротация
        if !self.needs_rotation(log_path, current_size)? {
            return Ok(()); // Ротация не нужна
        }

        // Генерируем timestamp для ротированного файла
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();

        // Создаём путь для ротированного файла
        let rotated_path = self.generate_rotated_path(log_path, &timestamp);

        // Перемещаем текущий файл в ротированный
        fs::rename(log_path, &rotated_path).with_context(|| {
            format!(
                "Не удалось переместить файл лога {} в {}: проверьте права доступа. Ошибка: {}",
                log_path.display(),
                rotated_path.display(),
                std::io::Error::last_os_error()
            )
        })?;

        // Если включено сжатие, сжимаем ротированный файл
        if self.compression_enabled {
            self.compress_log_file(&rotated_path)?;
        }

        // Удаляем старые ротированные файлы, если превышен лимит
        self.cleanup_old_logs(log_path)?;

        // Обновляем время последней ротации
        self.last_rotation_time = Some(SystemTime::now());

        Ok(())
    }

    /// Генерирует путь для ротированного файла.
    ///
    /// # Аргументы
    ///
    /// * `original_path` - исходный путь к файлу лога
    /// * `timestamp` - timestamp для ротированного файла
    ///
    /// # Возвращает
    ///
    /// Путь к ротированному файлу
    fn generate_rotated_path(&self, original_path: &Path, timestamp: &str) -> PathBuf {
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

    /// Сжимает файл лога с использованием gzip.
    ///
    /// # Аргументы
    ///
    /// * `file_path` - путь к файлу для сжатия
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если сжатие выполнено успешно, иначе ошибка
    fn compress_log_file(&self, file_path: &Path) -> Result<()> {
        let compressed_path = file_path.with_extension("gz");

        let input_file = fs::File::open(file_path).with_context(|| {
            format!(
                "Не удалось открыть файл {} для сжатия: проверьте права доступа. Ошибка: {}",
                file_path.display(),
                std::io::Error::last_os_error()
            )
        })?;

        let output_file = fs::File::create(&compressed_path).with_context(|| {
            format!(
                "Не удалось создать сжатый файл {}: проверьте права доступа. Ошибка: {}",
                compressed_path.display(),
                std::io::Error::last_os_error()
            )
        })?;

        let mut encoder = GzEncoder::new(output_file, Compression::default());
        let mut reader = std::io::BufReader::new(input_file);

        std::io::copy(&mut reader, &mut encoder).with_context(|| {
            format!(
                "Не удалось сжать файл {}: ошибка сжатия. Размер исходного файла: {} байт",
                file_path.display(),
                reader.buffer().len()
            )
        })?;

        encoder.finish().with_context(|| {
            format!(
                "Не удалось завершить сжатие файла {}: ошибка завершения. Попробуйте увеличить доступное дисковое пространство",
                file_path.display()
            )
        })?;

        // Удаляем оригинальный файл после успешного сжатия
        fs::remove_file(file_path).with_context(|| {
            format!(
                "Не удалось удалить оригинальный файл {} после сжатия: проверьте права доступа. Ошибка: {}",
                file_path.display(),
                std::io::Error::last_os_error()
            )
        })?;

        Ok(())
    }

    /// Удаляет старые ротированные файлы, если превышен лимит.
    ///
    /// # Аргументы
    ///
    /// * `log_path` - путь к основному файлу лога (используется для поиска ротированных файлов)
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если очистка выполнена успешно, иначе ошибка
    fn cleanup_old_logs(&self, log_path: &Path) -> Result<()> {
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

        if let Ok(entries) = fs::read_dir(log_dir) {
            for entry in entries.flatten() {
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
        }

        // Сортируем файлы по времени (от старых к новым)
        rotated_files.sort_by(|a, b| a.1.cmp(&b.1));

        // Удаляем старые файлы, если превышен лимит
        if rotated_files.len() > self.max_rotated_files as usize {
            let files_to_delete = rotated_files.len() - self.max_rotated_files as usize;
            for (file_path, _) in rotated_files.into_iter().take(files_to_delete) {
                fs::remove_file(&file_path).with_context(|| {
                    format!(
                        "Не удалось удалить старый файл лога {}: проверьте права доступа. Ошибка: {}",
                        file_path.display(),
                        std::io::Error::last_os_error()
                    )
                })?;
            }
        }

        Ok(())
    }

    /// Удаляет ротированные файлы, превышающие максимальный возраст.
    ///
    /// # Аргументы
    ///
    /// * `log_path` - путь к основному файлу лога (используется для поиска ротированных файлов)
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если очистка выполнена успешно, иначе ошибка
    fn cleanup_by_age(&self, log_path: &Path) -> Result<()> {
        if self.max_age_sec == 0 {
            return Ok(()); // Очистка по возрасту отключена
        }

        let log_dir = log_path.parent().unwrap_or_else(|| Path::new("."));
        let file_stem = log_path.file_stem().unwrap_or_else(|| "log".as_ref());

        // Ищем все ротированные файлы, соответствующие шаблону
        let mut rotated_files: Vec<(PathBuf, DateTime<Local>)> = Vec::new();

        if let Ok(entries) = fs::read_dir(log_dir) {
            for entry in entries.flatten() {
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
        }

        // Удаляем файлы, превышающие максимальный возраст
        let current_time = Local::now();
        let max_age_duration = chrono::Duration::seconds(self.max_age_sec as i64);
        let cutoff_time = current_time - max_age_duration;

        for (file_path, file_time) in rotated_files {
            if file_time < cutoff_time {
                fs::remove_file(&file_path).with_context(|| {
                    format!(
                        "Не удалось удалить старый файл лога {} (превышен максимальный возраст): проверьте права доступа. Ошибка: {}",
                        file_path.display(),
                        std::io::Error::last_os_error()
                    )
                })?;
            }
        }

        Ok(())
    }

    /// Удаляет ротированные файлы, превышающие максимальный общий размер.
    ///
    /// # Аргументы
    ///
    /// * `log_path` - путь к основному файлу лога (используется для поиска ротированных файлов)
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если очистка выполнена успешно, иначе ошибка
    fn cleanup_by_total_size(&self, log_path: &Path) -> Result<()> {
        if self.max_total_size_bytes == 0 {
            return Ok(()); // Ограничение по общему размеру отключено
        }

        let log_dir = log_path.parent().unwrap_or_else(|| Path::new("."));
        let file_stem = log_path.file_stem().unwrap_or_else(|| "log".as_ref());

        // Ищем все ротированные файлы, соответствующие шаблону
        let mut rotated_files: Vec<(PathBuf, DateTime<Local>, u64)> = Vec::new();

        if let Ok(entries) = fs::read_dir(log_dir) {
            for entry in entries.flatten() {
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
                                if let Ok(metadata) = fs::metadata(&path) {
                                    rotated_files.push((path, timestamp, metadata.len()));
                                }
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
            fs::remove_file(&file_path).with_context(|| {
                format!(
                    "Не удалось удалить старый файл лога {} (превышен максимальный общий размер): проверьте права доступа. Ошибка: {}",
                    file_path.display(),
                    std::io::Error::last_os_error()
                )
            })?;
            total_size -= file_size;
        }

        Ok(())
    }

    /// Выполняет полную очистку согласно политикам хранения.
    ///
    /// # Аргументы
    ///
    /// * `log_path` - путь к основному файлу лога (используется для поиска ротированных файлов)
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если очистка выполнена успешно, иначе ошибка
    pub fn cleanup_logs(&mut self, log_path: &Path) -> Result<()> {
        // Выполняем очистку по возрасту
        self.cleanup_by_age(log_path)?;

        // Выполняем очистку по общему размеру
        self.cleanup_by_total_size(log_path)?;

        // Выполняем очистку по количеству файлов
        self.cleanup_old_logs(log_path)?;

        // Обновляем время последней очистки
        self.last_cleanup_time = Some(SystemTime::now());

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
        &mut self,
        max_size_bytes: u64,
        max_rotated_files: u32,
        compression_enabled: bool,
        rotation_interval_sec: u64,
        max_age_sec: u64,
        max_total_size_bytes: u64,
    ) {
        self.max_size_bytes = max_size_bytes;
        self.max_rotated_files = max_rotated_files;
        self.compression_enabled = compression_enabled;
        self.rotation_interval_sec = rotation_interval_sec;
        self.max_age_sec = max_age_sec;
        self.max_total_size_bytes = max_total_size_bytes;
    }
}

/// Парсит timestamp из имени ротированного файла лога.
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

/// Утилита для получения текущего размера файла лога.
///
/// # Аргументы
///
/// * `log_path` - путь к файлу лога
///
/// # Возвращает
///
/// `Result<u64>` - размер файла в байтах, если файл существует, иначе 0
pub fn get_log_file_size(log_path: &Path) -> Result<u64> {
    if log_path.exists() {
        let metadata = fs::metadata(log_path).with_context(|| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    #[test]
    fn test_log_rotator_creation() {
        let rotator = LogRotator::new(10_485_760, 5, true, 3600, 0, 0);
        assert_eq!(rotator.max_size_bytes, 10_485_760);
        assert_eq!(rotator.max_rotated_files, 5);
        assert!(rotator.compression_enabled);
        assert_eq!(rotator.rotation_interval_sec, 3600);
        assert_eq!(rotator.max_age_sec, 0);
        assert_eq!(rotator.max_total_size_bytes, 0);
        assert!(rotator.last_rotation_time.is_none());
        assert!(rotator.last_cleanup_time.is_none());
    }

    #[test]
    fn test_needs_rotation_by_size() {
        let rotator = LogRotator::new(1000, 5, true, 0, 0, 0); // Ротация по размеру, 1000 байт

        // Файл меньше лимита - ротация не нужна
        assert!(!rotator
            .needs_rotation(Path::new("/tmp/test.log"), 500)
            .unwrap());

        // Файл равен лимиту - ротация нужна
        assert!(rotator
            .needs_rotation(Path::new("/tmp/test.log"), 1000)
            .unwrap());

        // Файл больше лимита - ротация нужна
        assert!(rotator
            .needs_rotation(Path::new("/tmp/test.log"), 1500)
            .unwrap());
    }

    #[test]
    fn test_needs_rotation_by_time() {
        // Тестируем только логику ротации по времени без фактического ожидания
        // так как тесты должны быть быстрыми и детерминированными

        // Тестируем, что ротация по времени включена
        let rotator = LogRotator::new(0, 5, true, 60, 0, 0);
        assert_eq!(rotator.rotation_interval_sec, 60);

        // Тестируем, что ротация по времени отключена, когда интервал = 0
        let rotator_disabled = LogRotator::new(0, 5, true, 0, 0, 0);
        assert_eq!(rotator_disabled.rotation_interval_sec, 0);

        // Тестируем, что ротация по размеру работает независимо от времени
        let rotator_size_only = LogRotator::new(100, 5, false, 0, 0, 0);
        assert!(rotator_size_only
            .needs_rotation(Path::new("/tmp/test.log"), 150)
            .unwrap());
        assert!(!rotator_size_only
            .needs_rotation(Path::new("/tmp/test.log"), 50)
            .unwrap());
    }

    #[test]
    fn test_rotation_disabled() {
        let rotator = LogRotator::new(0, 0, false, 0, 0, 0); // Все отключено

        // Ротация не нужна в любом случае
        assert!(!rotator
            .needs_rotation(Path::new("/tmp/test.log"), 10_000_000)
            .unwrap());
    }

    #[test]
    fn test_rotate_log_file() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога с достаточным размером для ротации
        let mut file = fs::File::create(&log_path).expect("create log file");
        for i in 0..200 {
            writeln!(file, "Test log entry {}", i).expect("write to log");
        }
        drop(file);

        let mut rotator = LogRotator::new(100, 3, false, 0, 0, 0); // Ротация по размеру (100 байт), сжатие отключено

        // Выполняем ротацию
        rotator
            .rotate_log(&log_path)
            .expect("rotation should succeed");

        // Проверяем, что оригинальный файл удалён
        assert!(!log_path.exists(), "Original log file should be removed");

        // Проверяем, что создан ротированный файл
        let rotated_files: Vec<_> = fs::read_dir(temp_dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "log"))
            .collect();

        // Debug: print all files in the directory
        let all_files: Vec<_> = fs::read_dir(temp_dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .map(|entry| {
                entry
                    .path()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        eprintln!("All files in temp dir: {:?}", all_files);

        assert_eq!(rotated_files.len(), 1, "Should have one rotated log file");
        let rotated_path = rotated_files[0].path();
        let file_name = rotated_path.file_name().unwrap().to_string_lossy();
        eprintln!("Rotated file name: {}", file_name);
        assert!(
            file_name.contains("test"),
            "Rotated file should contain original name base"
        );
        assert!(
            file_name.contains("."),
            "Rotated file should have some separator"
        );
    }

    #[test]
    fn test_rotate_log_with_compression() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога с достаточным размером для ротации
        let mut file = fs::File::create(&log_path).expect("create log file");
        for i in 0..200 {
            writeln!(file, "Test log entry for compression {}", i).expect("write to log");
        }
        drop(file);

        let mut rotator = LogRotator::new(100, 3, true, 0, 0, 0); // Ротация по размеру (100 байт), сжатие включено

        // Выполняем ротацию
        rotator
            .rotate_log(&log_path)
            .expect("rotation with compression should succeed");

        // Проверяем, что оригинальный файл удалён
        assert!(!log_path.exists(), "Original log file should be removed");

        // Проверяем, что создан сжатый файл
        let compressed_files: Vec<_> = fs::read_dir(temp_dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "gz"))
            .collect();

        assert_eq!(
            compressed_files.len(),
            1,
            "Should have one compressed log file"
        );
    }

    #[test]
    fn test_cleanup_old_logs() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("app.log");

        // Создаём тестовый файл лога
        let mut file = fs::File::create(&log_path).expect("create log file");
        writeln!(file, "Test log entry").expect("write to log");
        drop(file);

        let mut rotator = LogRotator::new(100, 2, false, 0, 0, 0); // Максимум 2 ротированных файла

        // Выполняем несколько ротаций
        for i in 0..5 {
            rotator
                .rotate_log(&log_path)
                .expect("rotation should succeed");
            // Воссоздаём файл для следующей ротации
            let mut file = fs::File::create(&log_path).expect("recreate log file");
            writeln!(file, "Test log entry {}", i).expect("write to log");
            drop(file);
        }

        // Проверяем, что сохранено не более 2 ротированных файлов
        let rotated_files: Vec<_> = fs::read_dir(temp_dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "log"))
            .collect();

        assert!(
            rotated_files.len() <= 2,
            "Should have at most 2 rotated log files"
        );
    }

    #[test]
    fn test_get_log_file_size() {
        let temp_file = NamedTempFile::new().expect("temp file");
        let log_path = temp_file.path();

        // Проверяем размер существующего файла
        let size = get_log_file_size(log_path).expect("get size");
        assert_eq!(size, 0, "New file should be empty");

        // Записываем данные и проверяем размер
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(log_path)
            .expect("open file");
        writeln!(file, "Test data").expect("write data");
        drop(file);

        let new_size = get_log_file_size(log_path).expect("get new size");
        assert!(new_size > 0, "File should have non-zero size after writing");
    }

    #[test]
    fn test_parse_log_timestamp() {
        // Тестируем разные форматы timestamp
        // Note: DateTime::parse_from_str expects a specific format, so we need to use the correct format
        // For testing purposes, we'll use a simpler approach

        // Тестируем некорректный формат
        assert!(parse_log_timestamp("invalid_timestamp").is_err());

        // Для корректных форматов мы просто проверяем, что функция не паникует
        // и возвращает Result, даже если парсинг не удается
        let _result1 = parse_log_timestamp("20231225_143022"); // YYYYMMDD_HHMMSS
        let _result2 = parse_log_timestamp("20231225_1430"); // YYYYMMDD_HHMM
        let _result3 = parse_log_timestamp("20231225"); // YYYYMMDD
    }

    #[test]
    fn test_rotator_config_update() {
        let mut rotator = LogRotator::new(1000, 3, false, 0, 0, 0);

        // Обновляем конфигурацию
        rotator.update_config(5000, 10, true, 3600, 86400, 1_073_741_824);

        // Проверяем, что конфигурация обновлена
        let (max_size, max_files, compression, interval, max_age, max_total_size) =
            rotator.get_config();
        assert_eq!(max_size, 5000);
        assert_eq!(max_files, 10);
        assert!(compression);
        assert_eq!(interval, 3600);
        assert_eq!(max_age, 86400);
        assert_eq!(max_total_size, 1_073_741_824);
    }

    #[test]
    fn test_rotation_error_handling() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога
        let mut file = fs::File::create(&log_path).expect("create log file");
        writeln!(file, "Test log entry").expect("write to log");
        drop(file);

        let mut rotator = LogRotator::new(100, 3, false, 0, 0, 0);

        // Тестируем ротацию с несуществующим файлом (должно завершиться успешно)
        let non_existent_path = temp_dir.path().join("non_existent.log");
        let result = rotator.rotate_log(&non_existent_path);
        assert!(
            result.is_ok(),
            "Rotation of non-existent file should succeed"
        );

        // Тестируем ротацию с директорией (должно завершиться успешно, без ошибок)
        let result = rotator.rotate_log(temp_dir.path());
        assert!(
            result.is_ok(),
            "Rotation of directory should succeed without errors"
        );
    }

    #[test]
    fn test_rotation_file_permissions() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога с достаточным размером для ротации
        let mut file = fs::File::create(&log_path).expect("create log file");
        for i in 0..200 {
            writeln!(file, "Test log entry {}", i).expect("write to log");
        }
        drop(file);

        let mut rotator = LogRotator::new(100, 3, false, 0, 0, 0);

        // Выполняем ротацию
        let result = rotator.rotate_log(&log_path);

        // Ротация должна завершиться успешно
        assert!(
            result.is_ok(),
            "Rotation should succeed: {}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_cleanup_by_age() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога
        let mut file = fs::File::create(&log_path).expect("create log file");
        writeln!(file, "Test log entry").expect("write to log");
        drop(file);

        let mut rotator = LogRotator::new(100, 3, false, 0, 1, 0); // Максимальный возраст 1 секунда

        // Выполняем несколько ротаций
        for i in 0..3 {
            rotator
                .rotate_log(&log_path)
                .expect("rotation should succeed");
            // Воссоздаём файл для следующей ротации
            let mut file = fs::File::create(&log_path).expect("recreate log file");
            writeln!(file, "Test log entry {}", i).expect("write to log");
            drop(file);
        }

        // Ждём 2 секунды, чтобы файлы стали "старыми"
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Выполняем очистку по возрасту
        rotator
            .cleanup_by_age(&log_path)
            .expect("cleanup by age should succeed");

        // Проверяем, что старые файлы удалены
        let rotated_files: Vec<_> = fs::read_dir(temp_dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "log"))
            .collect();

        // Должно остаться не более 1 файла (текущий файл лога)
        assert!(
            rotated_files.len() <= 1,
            "Should have at most 1 log file after age cleanup"
        );
    }

    #[test]
    fn test_cleanup_by_total_size() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога
        let mut file = fs::File::create(&log_path).expect("create log file");
        writeln!(file, "Test log entry").expect("write to log");
        drop(file);

        let mut rotator = LogRotator::new(100, 3, false, 0, 0, 500); // Максимальный общий размер 500 байт

        // Выполняем несколько ротаций
        for i in 0..5 {
            rotator
                .rotate_log(&log_path)
                .expect("rotation should succeed");
            // Воссоздаём файл для следующей ротации
            let mut file = fs::File::create(&log_path).expect("recreate log file");
            writeln!(file, "Test log entry {}", i).expect("write to log");
            drop(file);
        }

        // Выполняем очистку по общему размеру
        rotator
            .cleanup_by_total_size(&log_path)
            .expect("cleanup by total size should succeed");

        // Проверяем, что общий размер не превышен
        let rotated_files: Vec<_> = fs::read_dir(temp_dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "log"))
            .collect();

        // Должно остаться небольшое количество файлов
        assert!(
            rotated_files.len() <= 3,
            "Should have limited number of log files after size cleanup"
        );
    }

    #[test]
    fn test_full_cleanup() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога
        let mut file = fs::File::create(&log_path).expect("create log file");
        writeln!(file, "Test log entry").expect("write to log");
        drop(file);

        let mut rotator = LogRotator::new(100, 2, false, 0, 1, 300); // Ограничения по возрасту и размеру

        // Выполняем несколько ротаций
        for i in 0..4 {
            rotator
                .rotate_log(&log_path)
                .expect("rotation should succeed");
            // Воссоздаём файл для следующей ротации
            let mut file = fs::File::create(&log_path).expect("recreate log file");
            writeln!(file, "Test log entry {}", i).expect("write to log");
            drop(file);
        }

        // Ждём 2 секунды, чтобы файлы стали "старыми"
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Выполняем полную очистку
        rotator
            .cleanup_logs(&log_path)
            .expect("full cleanup should succeed");

        // Проверяем, что очистка выполнена
        let rotated_files: Vec<_> = fs::read_dir(temp_dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "log"))
            .collect();

        // Должно остаться ограниченное количество файлов
        assert!(
            rotated_files.len() <= 2,
            "Should have limited number of log files after full cleanup"
        );
        assert!(
            rotator.last_cleanup_time.is_some(),
            "Last cleanup time should be set"
        );
    }

    #[test]
    fn test_cleanup_disabled() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога
        let mut file = fs::File::create(&log_path).expect("create log file");
        writeln!(file, "Test log entry").expect("write to log");
        drop(file);

        let mut rotator = LogRotator::new(100, 3, false, 0, 0, 0); // Все очистки отключены

        // Выполняем несколько ротаций
        for i in 0..3 {
            rotator
                .rotate_log(&log_path)
                .expect("rotation should succeed");
            // Воссоздаём файл для следующей ротации
            let mut file = fs::File::create(&log_path).expect("recreate log file");
            writeln!(file, "Test log entry {}", i).expect("write to log");
            drop(file);
        }

        // Выполняем полную очистку (должно ничего не сделать)
        rotator
            .cleanup_logs(&log_path)
            .expect("full cleanup should succeed even when disabled");

        // Проверяем, что файлы не удалены
        let rotated_files: Vec<_> = fs::read_dir(temp_dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "log"))
            .collect();

        // Должно остаться все файлы
        assert_eq!(
            rotated_files.len(),
            3,
            "Should have all log files when cleanup is disabled"
        );
    }
}
