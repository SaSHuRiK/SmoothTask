//! Модуль для работы с устройствами хранения данных, включая SATA устройства.
//!
//! Предоставляет функциональность для обнаружения, классификации и мониторинга
//! SATA устройств с метриками производительности.

use std::path::Path;
use std::fs;
use std::io;
use tracing::{debug, error, info};

/// Информация о SATA устройстве
#[derive(Debug, Clone, PartialEq)]
pub struct SataDeviceInfo {
    /// Имя устройства (например, "sda", "sdb")
    pub device_name: String,
    /// Модель устройства
    pub model: String,
    /// Серийный номер
    pub serial_number: String,
    /// Тип устройства (HDD, SSD, SSHD)
    pub device_type: SataDeviceType,
    /// Скорость вращения (для HDD, в RPM)
    pub rotation_speed: Option<u32>,
    /// Емкость устройства в байтах
    pub capacity: u64,
    /// Текущая температура (если доступна)
    pub temperature: Option<f32>,
    /// Метрики производительности
    pub performance_metrics: SataPerformanceMetrics,
}

/// Тип SATA устройства
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SataDeviceType {
    /// Жесткий диск (HDD)
    Hdd,
    /// Твердотельный накопитель (SSD)
    Ssd,
    /// Гибридный накопитель (SSHD)
    Sshd,
    /// Неизвестный тип
    Unknown,
}

/// Метрики производительности SATA устройства
#[derive(Debug, Clone, PartialEq)]
pub struct SataPerformanceMetrics {
    /// Скорость чтения (байт/с)
    pub read_speed: u64,
    /// Скорость записи (байт/с)
    pub write_speed: u64,
    /// Время доступа (мкс)
    pub access_time: u32,
    /// Количество операций ввода-вывода в секунду
    pub iops: u32,
    /// Уровень загрузки устройства (0.0 - 1.0)
    pub utilization: f32,
}

impl Default for SataPerformanceMetrics {
    fn default() -> Self {
        Self {
            read_speed: 0,
            write_speed: 0,
            access_time: 0,
            iops: 0,
            utilization: 0.0,
        }
    }
}

/// Обнаружить все SATA устройства в системе
pub fn detect_sata_devices() -> io::Result<Vec<SataDeviceInfo>> {
    info!("Начало обнаружения SATA устройств");
    
    let mut devices = Vec::new();
    let sys_block_path = Path::new("/sys/block");
    
    if !sys_block_path.exists() {
        error!("/sys/block не существует, невозможно обнаружить SATA устройства");
        return Ok(devices);
    }
    
    for entry in fs::read_dir(sys_block_path)? {
        let entry = entry?;
        let device_name = entry.file_name();
        let device_name_str = device_name.to_string_lossy().into_owned();
        
        // Проверяем, является ли устройство SATA (проверяем наличие директории device)
        let device_path = sys_block_path.join(&device_name_str);
        let device_dir = device_path.join("device");
        
        if !device_dir.exists() {
            debug!("Устройство {} не является SATA устройством", device_name_str);
            continue;
        }
        
        // Проверяем, что это SATA устройство (наличие файла modalias с ata)
        let modalias_path = device_dir.join("modalias");
        if !modalias_path.exists() {
            debug!("Устройство {} не имеет modalias, пропускаем", device_name_str);
            continue;
        }
        
        let modalias = fs::read_to_string(&modalias_path)?;
        if !modalias.contains("ata") {
            debug!("Устройство {} не является SATA устройством (modalias: {})", device_name_str, modalias);
            continue;
        }
        
        debug!("Обнаружено SATA устройство: {}", device_name_str);
        
        // Собираем информацию об устройстве
        let device_info = collect_sata_device_info(&device_name_str, &device_dir)?;
        devices.push(device_info);
    }
    
    info!("Обнаружено {} SATA устройств", devices.len());
    Ok(devices)
}

/// Собрать информацию о конкретном SATA устройстве
fn collect_sata_device_info(device_name: &str, device_dir: &Path) -> io::Result<SataDeviceInfo> {
    // Получаем модель устройства
    let model = read_device_attr(device_dir, "model")?;
    
    // Получаем серийный номер
    let serial_number = read_device_attr(device_dir, "serial")?;
    
    // Определяем тип устройства
    let device_type = classify_sata_device(device_dir)?;
    
    // Получаем скорость вращения (если доступна)
    let rotation_speed = read_rotation_speed(device_dir)?;
    
    // Получаем емкость устройства
    let capacity = read_device_capacity(device_name)?;
    
    // Получаем температуру (если доступна)
    let temperature = read_device_temperature(device_dir)?;
    
    // Собираем метрики производительности
    let performance_metrics = collect_sata_performance_metrics(device_name)?;
    
    Ok(SataDeviceInfo {
        device_name: device_name.to_string(),
        model,
        serial_number,
        device_type,
        rotation_speed,
        capacity,
        temperature,
        performance_metrics,
    })
}

/// Прочитать атрибут устройства из файла
fn read_device_attr(device_dir: &Path, attr_name: &str) -> io::Result<String> {
    let attr_path = device_dir.join(attr_name);
    if attr_path.exists() {
        fs::read_to_string(attr_path)
            .map(|s| s.trim().to_string())
            .or_else(|_| Ok("Unknown".to_string()))
    } else {
        Ok("Unknown".to_string())
    }
}

/// Классифицировать тип SATA устройства
fn classify_sata_device(device_dir: &Path) -> io::Result<SataDeviceType> {
    // Проверяем тип устройства
    let device_type = read_device_attr(device_dir, "type")?;
    
    // Проверяем скорость вращения (если 0 или 1, то это SSD)
    let rotation_speed = read_rotation_speed(device_dir)?;
    
    // Если скорость вращения 0 или 1, то это SSD
    if rotation_speed == Some(0) || rotation_speed == Some(1) {
        return Ok(SataDeviceType::Ssd);
    }
    
    // Если скорость вращения больше 1, то это HDD
    if rotation_speed.is_some() && rotation_speed.unwrap() > 1 {
        return Ok(SataDeviceType::Hdd);
    }
    
    // Проверяем по типу устройства
    if device_type.contains("ssd") {
        return Ok(SataDeviceType::Ssd);
    }
    
    if device_type.contains("hdd") || device_type.contains("disk") {
        return Ok(SataDeviceType::Hdd);
    }
    
    // По умолчанию - неизвестный тип
    Ok(SataDeviceType::Unknown)
}

/// Прочитать скорость вращения устройства
fn read_rotation_speed(device_dir: &Path) -> io::Result<Option<u32>> {
    let rotation_speed_path = device_dir.join("queue").join("rotational");
    
    if rotation_speed_path.exists() {
        let rotation_speed_str = fs::read_to_string(rotation_speed_path)?;
        let rotation_speed = rotation_speed_str.trim().parse::<u32>()?;
        
        // 0 или 1 означает невращающееся устройство (SSD)
        // Больше 1 - скорость вращения в RPM
        if rotation_speed == 0 || rotation_speed == 1 {
            Ok(Some(rotation_speed))
        } else {
            // Для HDD скорость вращения обычно 5400, 7200, 10000, 15000 RPM
            Ok(Some(rotation_speed))
        }
    } else {
        Ok(None)
    }
}

/// Прочитать емкость устройства
fn read_device_capacity(device_name: &str) -> io::Result<u64> {
    let size_path = Path::new("/sys/block").join(device_name).join("size");
    
    if size_path.exists() {
        let size_str = fs::read_to_string(size_path)?;
        let size = size_str.trim().parse::<u64>()?;
        // Размер в 512-байтных секторах, конвертируем в байты
        Ok(size * 512)
    } else {
        Ok(0)
    }
}

/// Прочитать температуру устройства
fn read_device_temperature(device_dir: &Path) -> io::Result<Option<f32>> {
    let temp_path = device_dir.join("temp");
    
    if temp_path.exists() {
        let temp_str = fs::read_to_string(temp_path)?;
        let temp = temp_str.trim().parse::<f32>()?;
        // Температура в градусах Цельсия
        Ok(Some(temp))
    } else {
        Ok(None)
    }
}

/// Собрать метрики производительности SATA устройства
fn collect_sata_performance_metrics(device_name: &str) -> io::Result<SataPerformanceMetrics> {
    // В реальной системе мы бы использовали инструменты вроде iostat или smartctl
    // Для этой реализации мы вернем фиктивные значения
    
    let mut metrics = SataPerformanceMetrics::default();
    
    // Чтение статистики из /sys/block/<device>/stat
    let stat_path = Path::new("/sys/block").join(device_name).join("stat");
    
    if stat_path.exists() {
        let stat_content = fs::read_to_string(stat_path)?;
        // Парсинг статистики (упрощенный)
        // Формат: read_ios read_merges read_sectors read_ticks
        //          write_ios write_merges write_sectors write_ticks
        //          in_flight io_ticks time_in_queue
        let parts: Vec<&str> = stat_content.trim().split_whitespace().collect();
        
        if parts.len() >= 8 {
            let read_sectors = parts[2].parse::<u64>().unwrap_or(0);
            let write_sectors = parts[6].parse::<u64>().unwrap_or(0);
            
            // Упрощенный расчет скорости (секторов в секунду)
            // В реальной системе нужно использовать временные метки
            metrics.read_speed = read_sectors * 512; // байт/с
            metrics.write_speed = write_sectors * 512; // байт/с
            
            // Упрощенный расчет IOPS
            let read_ios = parts[0].parse::<u32>().unwrap_or(0);
            let write_ios = parts[4].parse::<u32>().unwrap_or(0);
            metrics.iops = read_ios + write_ios;
        }
    }
    
    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_sata_device_classification() {
        // Создаем временную директорию для теста
        let temp_dir = tempdir().unwrap();
        let device_dir = temp_dir.path().join("sda").join("device");
        std::fs::create_dir_all(&device_dir).unwrap();
        
        // Тест 1: SSD устройство (rotational = 0)
        let mut rotational_file = std::fs::File::create(device_dir.join("queue").join("rotational")).unwrap();
        writeln!(rotational_file, "0").unwrap();
        
        let device_type = classify_sata_device(&device_dir).unwrap();
        assert_eq!(device_type, SataDeviceType::Ssd);
        
        // Тест 2: HDD устройство (rotational = 1)
        let mut rotational_file = std::fs::File::create(device_dir.join("queue").join("rotational")).unwrap();
        writeln!(rotational_file, "1").unwrap();
        
        let device_type = classify_sata_device(&device_dir).unwrap();
        assert_eq!(device_type, SataDeviceType::Hdd);
        
        // Тест 3: HDD устройство с конкретной скоростью вращения
        let mut rotational_file = std::fs::File::create(device_dir.join("queue").join("rotational")).unwrap();
        writeln!(rotational_file, "7200").unwrap();
        
        let device_type = classify_sata_device(&device_dir).unwrap();
        assert_eq!(device_type, SataDeviceType::Hdd);
    }

    #[test]
    fn test_read_rotation_speed() {
        let temp_dir = tempdir().unwrap();
        let device_dir = temp_dir.path().join("sda").join("device");
        std::fs::create_dir_all(&device_dir).unwrap();
        
        // Тест 1: SSD (rotational = 0)
        let mut rotational_file = std::fs::File::create(device_dir.join("queue").join("rotational")).unwrap();
        writeln!(rotational_file, "0").unwrap();
        
        let rotation_speed = read_rotation_speed(&device_dir).unwrap();
        assert_eq!(rotation_speed, Some(0));
        
        // Тест 2: HDD (rotational = 1)
        let mut rotational_file = std::fs::File::create(device_dir.join("queue").join("rotational")).unwrap();
        writeln!(rotational_file, "1").unwrap();
        
        let rotation_speed = read_rotation_speed(&device_dir).unwrap();
        assert_eq!(rotation_speed, Some(1));
        
        // Тест 3: HDD с конкретной скоростью вращения
        let mut rotational_file = std::fs::File::create(device_dir.join("queue").join("rotational")).unwrap();
        writeln!(rotational_file, "7200").unwrap();
        
        let rotation_speed = read_rotation_speed(&device_dir).unwrap();
        assert_eq!(rotation_speed, Some(7200));
    }

    #[test]
    fn test_read_device_capacity() {
        let temp_dir = tempdir().unwrap();
        let block_dir = temp_dir.path().join("block");
        std::fs::create_dir_all(&block_dir).unwrap();
        
        let device_dir = block_dir.join("sda");
        std::fs::create_dir_all(&device_dir).unwrap();
        
        // Создаем файл size с размером 1000000 секторов
        let mut size_file = std::fs::File::create(device_dir.join("size")).unwrap();
        writeln!(size_file, "1000000").unwrap();
        
        let capacity = read_device_capacity("sda").unwrap();
        // 1000000 секторов * 512 байт = 512000000 байт
        assert_eq!(capacity, 512000000);
    }

    #[test]
    fn test_sata_performance_metrics() {
        let temp_dir = tempdir().unwrap();
        let block_dir = temp_dir.path().join("block");
        std::fs::create_dir_all(&block_dir).unwrap();
        
        let device_dir = block_dir.join("sda");
        std::fs::create_dir_all(&device_dir).unwrap();
        
        // Создаем файл stat с тестовыми данными
        let mut stat_file = std::fs::File::create(device_dir.join("stat")).unwrap();
        writeln!(stat_file, "100 0 2000 1000 50 0 1000 500 0 0 0").unwrap();
        
        let metrics = collect_sata_performance_metrics("sda").unwrap();
        assert_eq!(metrics.read_speed, 2000 * 512); // 1024000 байт/с
        assert_eq!(metrics.write_speed, 1000 * 512); // 512000 байт/с
        assert_eq!(metrics.iops, 150); // 100 read + 50 write
    }
}
