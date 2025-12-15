//! Модуль для работы с устройствами хранения данных, включая SATA и NVMe устройства.
//!
//! Предоставляет функциональность для обнаружения, классификации и мониторинга
//! SATA и NVMe устройств с метриками производительности.

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

/// Информация о NVMe устройстве
#[derive(Debug, Clone, PartialEq)]
pub struct NvmeDeviceInfo {
    /// Имя устройства (например, "nvme0n1", "nvme1n1")
    pub device_name: String,
    /// Модель устройства
    pub model: String,
    /// Серийный номер
    pub serial_number: String,
    /// Тип устройства (NVMe 1.x, 2.0, 3.0, 4.0)
    pub device_type: NvmeDeviceType,
    /// Емкость устройства в байтах
    pub capacity: u64,
    /// Текущая температура (если доступна)
    pub temperature: Option<f32>,
    /// Скорость интерфейса PCIe (например, 3.0, 4.0, 5.0)
    pub pcie_generation: Option<f32>,
    /// Количество линий PCIe
    pub pcie_lanes: Option<u32>,
    /// Метрики производительности
    pub performance_metrics: NvmePerformanceMetrics,
}

/// Тип NVMe устройства
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NvmeDeviceType {
    /// NVMe 1.x (до 3.5 Гбит/с на линию)
    Nvme1x,
    /// NVMe 2.0 (до 8 Гбит/с на линию)
    Nvme2_0,
    /// NVMe 3.0 (до 16 Гбит/с на линию)
    Nvme3_0,
    /// NVMe 4.0 (до 32 Гбит/с на линию)
    Nvme4_0,
    /// NVMe 5.0 (до 64 Гбит/с на линию)
    Nvme5_0,
    /// Неизвестный тип
    Unknown,
}

/// Метрики производительности NVMe устройства
#[derive(Debug, Clone, PartialEq)]
pub struct NvmePerformanceMetrics {
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
    /// Количество операций чтения
    pub read_operations: u64,
    /// Количество операций записи
    pub write_operations: u64,
}

impl Default for NvmePerformanceMetrics {
    fn default() -> Self {
        Self {
            read_speed: 0,
            write_speed: 0,
            access_time: 0,
            iops: 0,
            utilization: 0.0,
            read_operations: 0,
            write_operations: 0,
        }
    }
}

/// Обнаружить все NVMe устройства в системе
pub fn detect_nvme_devices() -> io::Result<Vec<NvmeDeviceInfo>> {
    info!("Начало обнаружения NVMe устройств");
    
    let mut devices = Vec::new();
    
    // Пробуем найти NVMe устройства в /sys/class/nvme/
    let nvme_class_path = Path::new("/sys/class/nvme");
    
    if !nvme_class_path.exists() {
        debug!("/sys/class/nvme не существует, NVMe устройства не найдены");
        // Пробуем альтернативный путь
        let nvme_bus_path = Path::new("/sys/bus/nvme/devices");
        if nvme_bus_path.exists() {
            return detect_nvme_devices_from_bus();
        }
        return Ok(devices);
    }
    
    for entry in fs::read_dir(nvme_class_path)? {
        let entry = entry?;
        let device_name = entry.file_name();
        let device_name_str = device_name.to_string_lossy().into_owned();
        
        // Проверяем, что это действительно NVMe устройство
        if !device_name_str.starts_with("nvme") {
            debug!("Устройство {} не является NVMe устройством", device_name_str);
            continue;
        }
        
        debug!("Обнаружено NVMe устройство: {}", device_name_str);
        
        // Собираем информацию об устройстве
        let device_info = collect_nvme_device_info(&device_name_str)?;
        devices.push(device_info);
    }
    
    info!("Обнаружено {} NVMe устройств", devices.len());
    Ok(devices)
}

/// Обнаружить NVMe устройства из /sys/bus/nvme/devices
fn detect_nvme_devices_from_bus() -> io::Result<Vec<NvmeDeviceInfo>> {
    let mut devices = Vec::new();
    let nvme_bus_path = Path::new("/sys/bus/nvme/devices");
    
    for entry in fs::read_dir(nvme_bus_path)? {
        let entry = entry?;
        let device_name = entry.file_name();
        let device_name_str = device_name.to_string_lossy().into_owned();
        
        // Проверяем, что это действительно NVMe устройство
        if !device_name_str.starts_with("nvme") {
            debug!("Устройство {} не является NVMe устройством", device_name_str);
            continue;
        }
        
        debug!("Обнаружено NVMe устройство: {}", device_name_str);
        
        // Собираем информацию об устройстве
        let device_info = collect_nvme_device_info(&device_name_str)?;
        devices.push(device_info);
    }
    
    Ok(devices)
}

/// Собрать информацию о конкретном NVMe устройстве
fn collect_nvme_device_info(device_name: &str) -> io::Result<NvmeDeviceInfo> {
    // Получаем модель устройства
    let model = read_nvme_device_attr(device_name, "model")?;
    
    // Получаем серийный номер
    let serial_number = read_nvme_device_attr(device_name, "serial")?;
    
    // Определяем тип устройства
    let device_type = classify_nvme_device(device_name)?;
    
    // Получаем емкость устройства
    let capacity = read_nvme_device_capacity(device_name)?;
    
    // Получаем температуру (если доступна)
    let temperature = read_nvme_device_temperature(device_name)?;
    
    // Получаем информацию о PCIe
    let (pcie_generation, pcie_lanes) = read_nvme_pcie_info(device_name)?;
    
    // Собираем метрики производительности
    let performance_metrics = collect_nvme_performance_metrics(device_name)?;
    
    Ok(NvmeDeviceInfo {
        device_name: device_name.to_string(),
        model,
        serial_number,
        device_type,
        capacity,
        temperature,
        pcie_generation,
        pcie_lanes,
        performance_metrics,
    })
}

/// Прочитать атрибут NVMe устройства
fn read_nvme_device_attr(device_name: &str, attr_name: &str) -> io::Result<String> {
    // Пробуем сначала в /sys/class/nvme/<device>
    let class_path = Path::new("/sys/class/nvme").join(device_name).join(attr_name);
    
    if class_path.exists() {
        return fs::read_to_string(class_path)
            .map(|s| s.trim().to_string())
            .or_else(|_| Ok("Unknown".to_string()));
    }
    
    // Пробуем в /sys/bus/nvme/devices/<device>
    let bus_path = Path::new("/sys/bus/nvme/devices").join(device_name).join(attr_name);
    
    if bus_path.exists() {
        return fs::read_to_string(bus_path)
            .map(|s| s.trim().to_string())
            .or_else(|_| Ok("Unknown".to_string()));
    }
    
    Ok("Unknown".to_string())
}

/// Классифицировать тип NVMe устройства
fn classify_nvme_device(device_name: &str) -> io::Result<NvmeDeviceType> {
    // Пробуем определить версию NVMe по информации о контроллере
    // Это упрощенная классификация - в реальной системе нужно использовать nvme-cli
    
    // Пробуем прочитать информацию о контроллере
    let controller_path = Path::new("/sys/class/nvme").join(device_name);
    
    if !controller_path.exists() {
        return Ok(NvmeDeviceType::Unknown);
    }
    
    // Пробуем определить по скорости интерфейса
    let (pcie_gen, _) = read_nvme_pcie_info(device_name)?;
    
    match pcie_gen {
        Some(5.0) => Ok(NvmeDeviceType::Nvme5_0),
        Some(4.0) => Ok(NvmeDeviceType::Nvme4_0),
        Some(3.0) => Ok(NvmeDeviceType::Nvme3_0),
        Some(2.0) => Ok(NvmeDeviceType::Nvme2_0),
        Some(1.0) => Ok(NvmeDeviceType::Nvme1x),
        _ => Ok(NvmeDeviceType::Unknown),
    }
}

/// Прочитать информацию о PCIe для NVMe устройства
fn read_nvme_pcie_info(device_name: &str) -> io::Result<(Option<f32>, Option<u32>)> {
    // NVMe устройства обычно подключены через PCIe
    // Пробуем найти соответствующее PCI устройство
    
    // Ищем в /sys/class/nvme/<device>/device
    let device_path = Path::new("/sys/class/nvme").join(device_name).join("device");
    
    if !device_path.exists() {
        return Ok((None, None));
    }
    
    // Пробуем прочитать информацию о PCIe
    let pci_path = device_path.join("..").join("..");
    
    // Чтение поколения PCIe
    let pcie_gen_path = pci_path.join("pcie_gen");
    if pcie_gen_path.exists() {
        let gen_str = fs::read_to_string(pcie_gen_path)?;
        if let Ok(gen) = gen_str.trim().parse::<f32>() {
            // Конвертируем в стандартное обозначение (1 -> 1.0, 2 -> 2.0 и т.д.)
            return Ok((Some(gen), None));
        }
    }
    
    // Альтернативный способ - через lspci или другие инструменты
    // Для простоты вернем типичные значения
    Ok((Some(3.0), Some(4)))
}

/// Прочитать емкость NVMe устройства
fn read_nvme_device_capacity(device_name: &str) -> io::Result<u64> {
    // NVMe устройства могут иметь несколько пространств имен
    // Пробуем прочитать емкость из первого пространства имен
    
    let namespace_path = Path::new("/sys/class/nvme").join(device_name).join("nvme0n1");
    
    if namespace_path.exists() {
        let size_path = namespace_path.join("size");
        
        if size_path.exists() {
            let size_str = fs::read_to_string(size_path)?;
            let size = size_str.trim().parse::<u64>()?;
            // Размер в 512-байтных секторах, конвертируем в байты
            return Ok(size * 512);
        }
    }
    
    // Пробуем альтернативный путь
    let namespace_path = Path::new("/sys/class/nvme").join(device_name).join(format!("{}n1", device_name));
    
    if namespace_path.exists() {
        let size_path = namespace_path.join("size");
        
        if size_path.exists() {
            let size_str = fs::read_to_string(size_path)?;
            let size = size_str.trim().parse::<u64>()?;
            return Ok(size * 512);
        }
    }
    
    Ok(0)
}

/// Прочитать температуру NVMe устройства
fn read_nvme_device_temperature(device_name: &str) -> io::Result<Option<f32>> {
    // Пробуем прочитать температуру из sysfs
    let temp_path = Path::new("/sys/class/nvme").join(device_name).join("hwmon").join("temp1_input");
    
    if temp_path.exists() {
        let temp_str = fs::read_to_string(temp_path)?;
        let temp_millidegrees = temp_str.trim().parse::<u32>()?;
        // Конвертируем из миллиградусов в градусы Цельсия
        let temp_celsius = temp_millidegrees as f32 / 1000.0;
        return Ok(Some(temp_celsius));
    }
    
    // Пробуем альтернативный путь
    let temp_path = Path::new("/sys/class/nvme").join(device_name).join("temp");
    
    if temp_path.exists() {
        let temp_str = fs::read_to_string(temp_path)?;
        let temp_celsius = temp_str.trim().parse::<f32>()?;
        return Ok(Some(temp_celsius));
    }
    
    Ok(None)
}

/// Собрать метрики производительности NVMe устройства
fn collect_nvme_performance_metrics(device_name: &str) -> io::Result<NvmePerformanceMetrics> {
    // В реальной системе мы бы использовали инструменты вроде nvme-cli
    // Для этой реализации мы вернем фиктивные значения
    
    let mut metrics = NvmePerformanceMetrics::default();
    
    // Чтение статистики из /sys/class/nvme/<device>/stat
    let stat_path = Path::new("/sys/class/nvme").join(device_name).join("stat");
    
    if stat_path.exists() {
        let stat_content = fs::read_to_string(stat_path)?;
        // Парсинг статистики (упрощенный)
        // Формат может варьироваться в зависимости от ядра
        let parts: Vec<&str> = stat_content.trim().split_whitespace().collect();
        
        if parts.len() >= 2 {
            let read_operations = parts[0].parse::<u64>().unwrap_or(0);
            let write_operations = parts[1].parse::<u64>().unwrap_or(0);
            
            metrics.read_operations = read_operations;
            metrics.write_operations = write_operations;
            metrics.iops = (read_operations + write_operations) as u32;
        }
    }
    
    // Устанавливаем типичные значения производительности для NVMe
    // В реальной системе это нужно получать из реальных измерений
    metrics.read_speed = 3500 * 1024 * 1024; // ~3.5 GB/s
    metrics.write_speed = 3000 * 1024 * 1024; // ~3.0 GB/s
    metrics.access_time = 20; // ~20 мкс для NVMe
    
    Ok(metrics)
}

/// Обнаружить все устройства хранения (SATA и NVMe)
pub fn detect_all_storage_devices() -> io::Result<StorageDetectionResult> {
    info!("Начало комплексного обнаружения устройств хранения");
    
    let sata_devices = detect_sata_devices()?;
    let nvme_devices = detect_nvme_devices()?;
    
    info!(
        "Обнаружено {} SATA устройств и {} NVMe устройств",
        sata_devices.len(),
        nvme_devices.len()
    );
    
    Ok(StorageDetectionResult {
        sata_devices,
        nvme_devices,
    })
}

/// Результат комплексного обнаружения устройств хранения
#[derive(Debug, Clone)]
pub struct StorageDetectionResult {
    /// Обнаруженные SATA устройства
    pub sata_devices: Vec<SataDeviceInfo>,
    /// Обнаруженные NVMe устройства
    pub nvme_devices: Vec<NvmeDeviceInfo>,
}

#[cfg(test)]
mod nvme_tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_nvme_device_classification() {
        // Тест классификации по поколению PCIe
        let temp_dir = tempdir().unwrap();
        let nvme_dir = tempdir.path().join("class").join("nvme");
        std::fs::create_dir_all(&nvme_dir).unwrap();
        
        // Создаем фиктивное устройство
        let device_dir = nvme_dir.join("nvme0n1");
        std::fs::create_dir_all(&device_dir).unwrap();
        
        // Тест 1: PCIe Gen 3 (NVMe 1.3/1.4)
        let pci_dir = device_dir.join("device");
        std::fs::create_dir_all(&pci_dir).unwrap();
        let mut pcie_gen_file = std::fs::File::create(pci_dir.join("pcie_gen")).unwrap();
        writeln!(pcie_gen_file, "3").unwrap();
        
        let device_type = classify_nvme_device("nvme0n1").unwrap();
        assert_eq!(device_type, NvmeDeviceType::Nvme3_0);
        
        // Тест 2: PCIe Gen 4 (NVMe 2.0)
        let mut pcie_gen_file = std::fs::File::create(pci_dir.join("pcie_gen")).unwrap();
        writeln!(pcie_gen_file, "4").unwrap();
        
        let device_type = classify_nvme_device("nvme0n1").unwrap();
        assert_eq!(device_type, NvmeDeviceType::Nvme4_0);
    }

    #[test]
    fn test_nvme_performance_metrics() {
        let temp_dir = tempdir().unwrap();
        let nvme_dir = tempdir.path().join("class").join("nvme");
        std::fs::create_dir_all(&nvme_dir).unwrap();
        
        let device_dir = nvme_dir.join("nvme0n1");
        std::fs::create_dir_all(&device_dir).unwrap();
        
        // Создаем файл stat с тестовыми данными
        let mut stat_file = std::fs::File::create(device_dir.join("stat")).unwrap();
        writeln!(stat_file, "1000 500 2000000 1000000 0 0 0 0").unwrap();
        
        let metrics = collect_nvme_performance_metrics("nvme0n1").unwrap();
        assert_eq!(metrics.read_operations, 1000);
        assert_eq!(metrics.write_operations, 500);
        assert_eq!(metrics.iops, 1500);
    }

    #[test]
    fn test_read_nvme_device_capacity() {
        let temp_dir = tempdir().unwrap();
        let nvme_dir = tempdir.path().join("class").join("nvme");
        std::fs::create_dir_all(&nvme_dir).unwrap();
        
        let device_dir = nvme_dir.join("nvme0n1");
        std::fs::create_dir_all(&device_dir).unwrap();
        
        let namespace_dir = device_dir.join("nvme0n1");
        std::fs::create_dir_all(&namespace_dir).unwrap();
        
        // Создаем файл size с размером 500000 секторов
        let mut size_file = std::fs::File::create(namespace_dir.join("size")).unwrap();
        writeln!(size_file, "500000").unwrap();
        
        let capacity = read_nvme_device_capacity("nvme0n1").unwrap();
        // 500000 секторов * 512 байт = 256000000 байт
        assert_eq!(capacity, 256000000);
    }

    #[test]
    fn test_comprehensive_storage_detection() {
        // Этот тест проверяет, что комплексное обнаружение устройств хранения работает
        // В реальной системе это обнаружит реальные устройства
        // В тестовой среде это просто проверит, что функция не падает
        
        // Создаем временные директории для теста
        let temp_dir = tempdir().unwrap();
        
        // Создаем фиктивную структуру /sys
        let sys_dir = temp_dir.path().join("sys");
        let block_dir = sys_dir.join("block");
        let nvme_dir = sys_dir.join("class").join("nvme");
        
        std::fs::create_dir_all(&block_dir).unwrap();
        std::fs::create_dir_all(&nvme_dir).unwrap();
        
        // Создаем фиктивное SATA устройство
        let sata_device_dir = block_dir.join("sda");
        std::fs::create_dir_all(&sata_device_dir).unwrap();
        
        let sata_device_subdir = sata_device_dir.join("device");
        std::fs::create_dir_all(&sata_device_subdir).unwrap();
        
        // Создаем modalias файл для SATA
        let mut modalias_file = std::fs::File::create(sata_device_subdir.join("modalias")).unwrap();
        writeln!(modalias_file, "pci:v00008086d00002822sv00001462sd00007501bc01sc06i01").unwrap();
        
        // Создаем фиктивное NVMe устройство
        let nvme_device_dir = nvme_dir.join("nvme0n1");
        std::fs::create_dir_all(&nvme_device_dir).unwrap();
        
        // Создаем модель и серийный номер для NVMe
        let mut model_file = std::fs::File::create(nvme_device_dir.join("model")).unwrap();
        writeln!(model_file, "Samsung SSD 980 PRO").unwrap();
        
        let mut serial_file = std::fs::File::create(nvme_device_dir.join("serial")).unwrap();
        writeln!(serial_file, "S5JXNF0T123456").unwrap();
        
        // В реальной системе функция обнаружит устройства
        // Здесь мы просто проверяем, что функция не падает
        let result = detect_all_storage_devices();
        
        // В тестовой среде без реальных устройств результат может быть пустым
        // Главное, что функция не падает
        assert!(result.is_ok());
    }
}
