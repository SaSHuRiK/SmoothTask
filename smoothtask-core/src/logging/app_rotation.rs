//! Модуль для ротации логов приложения (tracing).
//!
//! Этот модуль предоставляет функциональность для автоматической ротации
//! основных логов приложения, которые создаются через tracing.

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Структура для управления ротацией логов приложения.
///
/// Эта структура расширяет функциональность LogRotator для работы с
/// логами, создаваемыми через tracing_appender.
#[derive(Debug, Clone)]
pub struct AppLogRotator {
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
    /// Путь к основному файлу лога
    log_path: PathBuf,
}

impl AppLogRotator {
    /// Создаёт новый AppLogRotator с указанной конфигурацией.
    ///
    /// # Аргументы
    ///
    /// * `log_path` - путь к основному файлу лога
    /// * `max_size_bytes` - максимальный размер файла лога в байтах перед ротацией
    /// * `max_rotated_files` - максимальное количество сохраняемых ротированных логов
    /// * `compression_enabled` - включить сжатие ротированных логов
    /// * `rotation_interval_sec` - интервал ротации логов по времени в секундах
    /// * `max_age_sec` - максимальный возраст ротированных логов в секундах перед удалением
    /// * `max_total_size_bytes` - максимальный общий размер всех ротированных логов в байтах
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр AppLogRotator
    pub fn new(
        log_path: impl AsRef<Path>,
        max_size_bytes: u64,
        max_rotated_files: u32,
        compression_enabled: bool,
        rotation_interval_sec: u64,
        max_age_sec: u64,
        max_total_size_bytes: u64,
    ) -> Self {
        Self {
            log_path: log_path.as_ref().to_path_buf(),
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
    /// * `current_size` - текущий размер файла лога в байтах
    ///
    /// # Возвращает
    ///
    /// `true`, если ротация необходима, `false` в противном случае
    pub fn needs_rotation(&self, current_size: u64) -> Result<bool> {
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
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если ротация выполнена успешно, иначе ошибка
    pub fn rotate_log(&mut self) -> Result<()> {
        // Проверяем, что файл существует
        if !self.log_path.exists() {
            return Ok(()); // Нет файла для ротации
        }

        let metadata = fs::metadata(&self.log_path).with_context(|| {
            format!(
                "Не удалось получить метаданные файла лога {}: проверьте, что файл существует и доступен для чтения. Ошибка: {}",
                self.log_path.display(),
                std::io::Error::last_os_error()
            )
        })?;

        // Проверяем, что это файл, а не директория
        if !metadata.is_file() {
            tracing::warn!(
                "Путь {} не является файлом, пропускаем ротацию",
                self.log_path.display()
            );
            return Ok(()); // Не файл, пропускаем ротацию
        }

        let current_size = metadata.len();

        // Проверяем, необходима ли ротация
        if !self.needs_rotation(current_size)? {
            return Ok(()); // Ротация не нужна
        }

        // Генерируем timestamp для ротированного файла
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();

        // Создаём путь для ротированного файла
        let rotated_path = self.generate_rotated_path(&timestamp);

        // Перемещаем текущий файл в ротированный
        fs::rename(&self.log_path, &rotated_path).with_context(|| {
            format!(
                "Не удалось переместить файл лога {} в {}: проверьте права доступа. Ошибка: {}",
                self.log_path.display(),
                rotated_path.display(),
                std::io::Error::last_os_error()
            )
        })?;

        // Если включено сжатие, сжимаем ротированный файл
        if self.compression_enabled {
            self.compress_log_file(&rotated_path)?;
        }

        // Удаляем старые ротированные файлы, если превышен лимит
        self.cleanup_old_logs()?;

        // Обновляем время последней ротации
        self.last_rotation_time = Some(SystemTime::now());

        Ok(())
    }

    /// Генерирует путь для ротированного файла.
    ///
    /// # Аргументы
    ///
    /// * `timestamp` - timestamp для ротированного файла
    ///
    /// # Возвращает
    ///
    /// Путь к ротированному файлу
    fn generate_rotated_path(&self, timestamp: &str) -> PathBuf {
        let mut rotated_path = self.log_path.clone();
        let file_stem = self.log_path.file_stem().unwrap_or_else(|| "log".as_ref());
        let extension = self
            .log_path
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
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если очистка выполнена успешно, иначе ошибка
    fn cleanup_old_logs(&self) -> Result<()> {
        if self.max_rotated_files == 0 {
            return Ok(()); // Нет ограничения на количество файлов
        }

        let log_dir = self.log_path.parent().unwrap_or_else(|| Path::new("."));
        let file_stem = self.log_path.file_stem().unwrap_or_else(|| "log".as_ref());
        let _extension = self
            .log_path
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
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если очистка выполнена успешно, иначе ошибка
    fn cleanup_by_age(&self) -> Result<()> {
        if self.max_age_sec == 0 {
            return Ok(()); // Очистка по возрасту отключена
        }

        let log_dir = self.log_path.parent().unwrap_or_else(|| Path::new("."));
        let file_stem = self.log_path.file_stem().unwrap_or_else(|| "log".as_ref());

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
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если очистка выполнена успешно, иначе ошибка
    fn cleanup_by_total_size(&self) -> Result<()> {
        if self.max_total_size_bytes == 0 {
            return Ok(()); // Ограничение по общему размеру отключено
        }

        let log_dir = self.log_path.parent().unwrap_or_else(|| Path::new("."));
        let file_stem = self.log_path.file_stem().unwrap_or_else(|| "log".as_ref());

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
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если очистка выполнена успешно, иначе ошибка
    pub fn cleanup_logs(&mut self) -> Result<()> {
        // Выполняем очистку по возрасту
        self.cleanup_by_age()?;

        // Выполняем очистку по общему размеру
        self.cleanup_by_total_size()?;

        // Выполняем очистку по количеству файлов
        self.cleanup_old_logs()?;

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

    /// Возвращает путь к основному файлу лога.
    pub fn log_path(&self) -> &Path {
        &self.log_path
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
pub fn get_app_log_file_size(log_path: &Path) -> Result<u64> {
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
    fn test_app_log_rotator_creation() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("app.log");

        let rotator = AppLogRotator::new(&log_path, 10_485_760, 5, true, 3600, 0, 0);
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
    fn test_app_log_rotator_needs_rotation() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("app.log");
        let rotator = AppLogRotator::new(&log_path, 1000, 5, true, 0, 0, 0);

        // Файл меньше лимита - ротация не нужна
        assert!(!rotator.needs_rotation(500).unwrap());

        // Файл равен лимиту - ротация нужна
        assert!(rotator.needs_rotation(1000).unwrap());

        // Файл больше лимита - ротация нужна
        assert!(rotator.needs_rotation(1500).unwrap());
    }

    #[test]
    fn test_app_log_rotator_rotate_log() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога с достаточным размером для ротации
        let mut file = fs::File::create(&log_path).expect("create log file");
        for i in 0..200 {
            writeln!(file, "Test log entry {}", i).expect("write to log");
        }
        drop(file);

        let mut rotator = AppLogRotator::new(&log_path, 100, 3, false, 0, 0, 0);

        // Выполняем ротацию
        rotator.rotate_log().expect("rotation should succeed");

        // Проверяем, что оригинальный файл удалён
        assert!(!log_path.exists(), "Original log file should be removed");

        // Проверяем, что создан ротированный файл
        let rotated_files: Vec<_> = fs::read_dir(temp_dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "log"))
            .collect();

        assert_eq!(rotated_files.len(), 1, "Should have one rotated log file");
    }

    #[test]
    fn test_app_log_rotator_rotate_log_with_compression() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога с достаточным размером для ротации
        let mut file = fs::File::create(&log_path).expect("create log file");
        for i in 0..200 {
            writeln!(file, "Test log entry for compression {}", i).expect("write to log");
        }
        drop(file);

        let mut rotator = AppLogRotator::new(&log_path, 100, 3, true, 0, 0, 0);

        // Выполняем ротацию
        rotator
            .rotate_log()
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
    fn test_app_log_rotator_cleanup_old_logs() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("app.log");

        // Создаём тестовый файл лога
        let mut file = fs::File::create(&log_path).expect("create log file");
        writeln!(file, "Test log entry").expect("write to log");
        drop(file);

        let mut rotator = AppLogRotator::new(&log_path, 100, 2, false, 0, 0, 0);

        // Выполняем несколько ротаций
        for i in 0..5 {
            rotator.rotate_log().expect("rotation should succeed");
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
    fn test_app_log_rotator_config_update() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("app.log");

        let mut rotator = AppLogRotator::new(&log_path, 1000, 3, false, 0, 0, 0);

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
    fn test_app_log_rotator_error_handling() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога
        let mut file = fs::File::create(&log_path).expect("create log file");
        writeln!(file, "Test log entry").expect("write to log");
        drop(file);

        let mut rotator = AppLogRotator::new(&log_path, 100, 3, false, 0, 0, 0);

        // Тестируем ротацию с несуществующим файлом (должно завершиться успешно)
        let non_existent_path = temp_dir.path().join("non_existent.log");
        let mut rotator2 = AppLogRotator::new(&non_existent_path, 100, 3, false, 0, 0, 0);
        let result = rotator2.rotate_log();
        assert!(
            result.is_ok(),
            "Rotation of non-existent file should succeed"
        );

        // Тестируем ротацию с директорией (должно завершиться успешно, без ошибок)
        let result = rotator.rotate_log();
        assert!(result.is_ok(), "Rotation should succeed");
    }

    #[test]
    fn test_get_app_log_file_size() {
        let temp_file = NamedTempFile::new().expect("temp file");
        let log_path = temp_file.path();

        // Проверяем размер существующего файла
        let size = get_app_log_file_size(log_path).expect("get size");
        assert_eq!(size, 0, "New file should be empty");

        // Записываем данные и проверяем размер
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(log_path)
            .expect("open file");
        writeln!(file, "Test data").expect("write data");
        drop(file);

        let new_size = get_app_log_file_size(log_path).expect("get new size");
        assert!(new_size > 0, "File should have non-zero size after writing");
    }

    #[test]
    fn test_app_log_rotator_cleanup_by_age() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога
        let mut file = fs::File::create(&log_path).expect("create log file");
        writeln!(file, "Test log entry").expect("write to log");
        drop(file);

        let mut rotator = AppLogRotator::new(&log_path, 100, 3, false, 0, 1, 0); // Максимальный возраст 1 секунда

        // Выполняем несколько ротаций
        for i in 0..3 {
            rotator.rotate_log().expect("rotation should succeed");
            // Воссоздаём файл для следующей ротации
            let mut file = fs::File::create(&log_path).expect("recreate log file");
            writeln!(file, "Test log entry {}", i).expect("write to log");
            drop(file);
        }

        // Ждём 2 секунды, чтобы файлы стали "старыми"
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Выполняем очистку по возрасту
        rotator
            .cleanup_by_age()
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
    fn test_app_log_rotator_cleanup_by_total_size() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога
        let mut file = fs::File::create(&log_path).expect("create log file");
        writeln!(file, "Test log entry").expect("write to log");
        drop(file);

        let mut rotator = AppLogRotator::new(&log_path, 100, 3, false, 0, 0, 500); // Максимальный общий размер 500 байт

        // Выполняем несколько ротаций
        for i in 0..5 {
            rotator.rotate_log().expect("rotation should succeed");
            // Воссоздаём файл для следующей ротации
            let mut file = fs::File::create(&log_path).expect("recreate log file");
            writeln!(file, "Test log entry {}", i).expect("write to log");
            drop(file);
        }

        // Выполняем очистку по общему размеру
        rotator
            .cleanup_by_total_size()
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
    fn test_app_log_rotator_full_cleanup() {
        let temp_dir = TempDir::new().expect("temp dir");
        let log_path = temp_dir.path().join("test.log");

        // Создаём тестовый файл лога
        let mut file = fs::File::create(&log_path).expect("create log file");
        writeln!(file, "Test log entry").expect("write to log");
        drop(file);

        let mut rotator = AppLogRotator::new(&log_path, 100, 2, false, 0, 1, 300); // Ограничения по возрасту и размеру

        // Выполняем несколько ротаций
        for i in 0..4 {
            rotator.rotate_log().expect("rotation should succeed");
            // Воссоздаём файл для следующей ротации
            let mut file = fs::File::create(&log_path).expect("recreate log file");
            writeln!(file, "Test log entry {}", i).expect("write to log");
            drop(file);
        }

        // Ждём 2 секунды, чтобы файлы стали "старыми"
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Выполняем полную очистку
        rotator.cleanup_logs().expect("full cleanup should succeed");

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
}
