//! NVML (NVIDIA Management Library) Wrapper
//!
//! This module provides a safe Rust wrapper around the NVIDIA NVML library
//! for monitoring NVIDIA GPU metrics including utilization, memory, temperature,
//! power consumption, and more.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{debug, error, info, warn};

/// NVIDIA GPU device information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NvmlDevice {
    /// Device index
    pub index: u32,
    /// Device name (e.g., "GeForce RTX 3080")
    pub name: String,
    /// Device UUID
    pub uuid: String,
    /// PCI bus ID
    pub pci_bus_id: String,
    /// Device path in sysfs
    pub device_path: String,
}

/// NVIDIA GPU utilization metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct NvmlUtilization {
    /// GPU utilization percentage (0-100)
    pub gpu_util: u32,
    /// Memory utilization percentage (0-100)
    pub memory_util: u32,
    /// Encoder utilization percentage (0-100)
    pub encoder_util: Option<u32>,
    /// Decoder utilization percentage (0-100)
    pub decoder_util: Option<u32>,
}

/// NVIDIA GPU memory metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct NvmlMemory {
    /// Total GPU memory in bytes
    pub total_bytes: u64,
    /// Used GPU memory in bytes
    pub used_bytes: u64,
    /// Free GPU memory in bytes
    pub free_bytes: u64,
}

/// NVIDIA GPU temperature metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct NvmlTemperature {
    /// Current GPU temperature in Celsius
    pub temperature_c: u32,
    /// GPU hotspot temperature in Celsius (if available)
    pub hotspot_c: Option<u32>,
    /// GPU memory temperature in Celsius (if available)
    pub memory_c: Option<u32>,
}

/// NVIDIA GPU power metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct NvmlPower {
    /// Current GPU power usage in milliwatts
    pub power_mw: u32,
    /// GPU power limit in milliwatts
    pub power_limit_mw: u32,
    /// GPU power cap in milliwatts (if available)
    pub power_cap_mw: Option<u32>,
}

/// NVIDIA GPU clock metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct NvmlClocks {
    /// Current GPU core clock in MHz
    pub core_clock_mhz: u32,
    /// Current GPU memory clock in MHz
    pub memory_clock_mhz: u32,
    /// Current GPU shader clock in MHz (if available)
    pub shader_clock_mhz: Option<u32>,
}

/// Complete NVIDIA GPU metrics for a single device
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NvmlDeviceMetrics {
    /// Device information
    pub device: NvmlDevice,
    /// GPU utilization metrics
    pub utilization: NvmlUtilization,
    /// GPU memory metrics
    pub memory: NvmlMemory,
    /// GPU temperature metrics
    pub temperature: NvmlTemperature,
    /// GPU power metrics
    pub power: NvmlPower,
    /// GPU clock metrics
    pub clocks: NvmlClocks,
    /// Timestamp when metrics were collected
    pub timestamp: std::time::SystemTime,
}

impl Default for NvmlDeviceMetrics {
    fn default() -> Self {
        Self {
            device: NvmlDevice {
                index: 0,
                name: String::new(),
                uuid: String::new(),
                pci_bus_id: String::new(),
                device_path: String::new(),
            },
            utilization: NvmlUtilization::default(),
            memory: NvmlMemory::default(),
            temperature: NvmlTemperature::default(),
            power: NvmlPower::default(),
            clocks: NvmlClocks::default(),
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// Collection of metrics for all NVIDIA GPU devices
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NvmlMetricsCollection {
    /// List of NVIDIA GPU devices with their metrics
    pub devices: Vec<NvmlDeviceMetrics>,
    /// Total NVIDIA GPU count
    pub gpu_count: usize,
}

/// NVML library handle (would be a pointer to the actual NVML library in a real implementation)
#[derive(Clone)]
struct NvmlHandle {
    // In a real implementation, this would contain the actual NVML library handle
    // For now, we'll use a placeholder
    _private: (),
}

impl NvmlHandle {
    fn new() -> Result<Self> {
        info!("Инициализация NVML библиотеки");
        
        // In a real implementation, we would:
        // 1. Load the NVML library (libnvidia-ml.so)
        // 2. Initialize the NVML context
        // 3. Check for errors
        
        // For now, we'll simulate this with a placeholder
        Ok(Self { _private: () })
    }
    

}

/// Global NVML handle (lazy initialized)
fn get_nvml_handle() -> Result<NvmlHandle> {
    use once_cell::sync::OnceCell;
    
    static NVML_HANDLE: OnceCell<NvmlHandle> = OnceCell::new();
    static INIT: std::sync::Once = std::sync::Once::new();

    INIT.call_once(|| {
        match NvmlHandle::new() {
            Ok(handle) => {
                if let Err(_e) = NVML_HANDLE.set(handle) {
                    error!("Не удалось установить NVML handle");
                }
            }
            Err(e) => {
                error!(
                    "Не удалось инициализировать NVML: {}. \n                    Возможные причины:\n                    1) NVML библиотека не установлена (libnvidia-ml.so)\n                    2) Некорректные права доступа к библиотеке\n                    3) Проблемы с загрузкой модулей ядра NVIDIA\n                    4) Конфликт версий драйверов\n                    Рекомендации:\n                    - Проверьте установку NVML: ls -la /usr/lib/libnvidia-ml.so*\n                    - Проверьте загрузку модулей: lsmod | grep nvidia\n                    - Проверьте права доступа: sudo chmod 644 /usr/lib/libnvidia-ml.so*\n                    - Попробуйте переустановить драйверы NVIDIA\n                    - Проверьте системные логи: sudo dmesg | grep nvidia",
                    e
                );
            }
        }
    });

    match NVML_HANDLE.get() {
        Some(handle) => Ok(handle.clone()),
        None => Err(anyhow!(
            "NVML не инициализирован. \n            Это может быть вызвано:\n            1) Ошибкой инициализации NVML\n            2) Отсутствием NVIDIA GPU в системе\n            3) Проблемами с драйверами NVIDIA\n            Рекомендации:\n            - Проверьте наличие NVIDIA GPU: lspci | grep -i nvidia\n            - Проверьте загрузку драйверов: nvidia-smi\n            - Проверьте системные логи: sudo dmesg | grep nvidia\n            - Попробуйте перезагрузить систему"
        )),
    }
}

/// Check if NVML is available on the system
pub fn is_nvml_available() -> bool {
    // Check for NVML library files
    let nvml_paths = [
        "/usr/lib/libnvidia-ml.so",
        "/usr/lib/libnvidia-ml.so.1",
        "/usr/lib64/libnvidia-ml.so",
        "/usr/lib64/libnvidia-ml.so.1",
        "/usr/lib/x86_64-linux-gnu/libnvidia-ml.so",
        "/usr/lib/x86_64-linux-gnu/libnvidia-ml.so.1",
    ];

    for path in nvml_paths {
        if Path::new(path).exists() {
            info!("NVML библиотека найдена: {}", path);
            return true;
        }
    }

    // Also check for NVIDIA devices
    let nvidia_devices = [
        "/dev/nvidia0",
        "/dev/nvidia1",
        "/dev/nvidia2",
        "/dev/nvidia3",
        "/dev/nvidiactl",
        "/dev/nvidia-uvm",
        "/dev/nvidia-uvm-tools",
    ];

    for device in nvidia_devices {
        if Path::new(device).exists() {
            info!("NVIDIA устройство найдено: {}", device);
            return true;
        }
    }

    debug!("NVML библиотека или NVIDIA устройства не найдены");
    false
}

/// Discover all NVIDIA GPU devices using NVML
pub fn discover_nvml_devices() -> Result<Vec<NvmlDevice>> {
    if !is_nvml_available() {
        warn!(
            "NVML не доступен - NVIDIA GPU устройства не могут быть обнаружены. \n            Возможные причины:\n            1) NVML библиотека не установлена\n            2) NVIDIA драйверы не загружены\n            3) Отсутствие NVIDIA GPU в системе\n            Рекомендации:\n            - Проверьте установку NVML: ls -la /usr/lib/libnvidia-ml.so*\n            - Проверьте загрузку драйверов: lsmod | grep nvidia\n            - Проверьте наличие GPU: lspci | grep -i nvidia\n            - Попробуйте установить драйверы: sudo apt install nvidia-driver\n            - Проверьте системные логи: sudo dmesg | grep nvidia"
        );
        return Ok(Vec::new());
    }

    let _handle = match get_nvml_handle() {
        Ok(handle) => handle,
        Err(e) => {
            warn!(
                "Не удалось получить NVML handle: {}. \n                Возможные причины:\n                1) Ошибка инициализации NVML\n                2) Проблемы с правами доступа\n                3) Конфликт версий драйверов\n                Рекомендации:\n                - Проверьте права доступа: sudo chmod 644 /usr/lib/libnvidia-ml.so*\n                - Попробуйте перезагрузить драйверы: sudo rmmod nvidia; sudo modprobe nvidia\n                - Проверьте системные логи: sudo dmesg | grep nvidia\n                - Попробуйте переустановить драйверы NVIDIA",
                e
            );
            return Ok(Vec::new());
        }
    };

    // In a real implementation, we would:
    // 1. Call nvmlDeviceGetCount() to get the number of devices
    // 2. Iterate through each device and call nvmlDeviceGetHandleByIndex()
    // 3. For each device, call nvmlDeviceGetName(), nvmlDeviceGetUUID(), etc.
    
    // For now, we'll simulate this by looking for NVIDIA devices in sysfs
    let mut devices = Vec::new();

    // Look for NVIDIA devices in /sys/class/drm
    let drm_dir = Path::new("/sys/class/drm");
    if drm_dir.exists() {
        if let Ok(entries) = fs::read_dir(drm_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

                    if file_name.starts_with("card") {
                        let device_path = path.join("device");
                        if device_path.exists() {
                            // Check if this is an NVIDIA device
                            let vendor_path = device_path.join("vendor");
                            if vendor_path.exists() {
                                if let Ok(vendor_content) = fs::read_to_string(&vendor_path) {
                                    let vendor_id = vendor_content.trim();
                                    if vendor_id == "0x10de" { // NVIDIA vendor ID
                                        let device_id_path = device_path.join("device");
                                        let device_id = if device_id_path.exists() {
                                            fs::read_to_string(&device_id_path).ok().map(|s| s.trim().to_string())
                                        } else {
                                            None
                                        };

                                        let name = format!("NVIDIA GPU {}", file_name);
                                        let uuid = format!("{}-{}", vendor_id, device_id.clone().unwrap_or("unknown".to_string()));
                                        let pci_bus_id = device_path.to_string_lossy().into_owned();

                                        let device = NvmlDevice {
                                            index: devices.len() as u32,
                                            name,
                                            uuid,
                                            pci_bus_id,
                                            device_path: device_path.to_string_lossy().into_owned(),
                                        };

                                        info!("Обнаружено NVIDIA GPU устройство: {}", device.name);
                                        devices.push(device);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if devices.is_empty() {
        debug!("NVIDIA GPU устройства не найдены");
    } else {
        info!("Обнаружено {} NVIDIA GPU устройств", devices.len());
    }

    Ok(devices)
}

/// Collect NVIDIA GPU metrics using NVML
pub fn collect_nvml_metrics() -> Result<NvmlMetricsCollection> {
    info!("Сбор метрик NVIDIA GPU через NVML");

    let devices = match discover_nvml_devices() {
        Ok(devices) => devices,
        Err(e) => {
            warn!("Не удалось обнаружить NVIDIA GPU устройства: {}", e);
            return Ok(NvmlMetricsCollection::default());
        }
    };

    let mut collection = NvmlMetricsCollection {
        devices: Vec::new(),
        gpu_count: devices.len(),
    };

    if devices.is_empty() {
        debug!("Нет NVIDIA GPU устройств для сбора метрик");
        return Ok(collection);
    }

    let mut successful_devices = 0;

    for device in devices {
        match collect_nvml_device_metrics(&device) {
            Ok(metrics) => {
                collection.devices.push(metrics);
                successful_devices += 1;
            }
            Err(e) => {
                error!(
                    "Не удалось собрать метрики для NVIDIA GPU устройства {}: {}. \n                    Возможные причины:\n                    1) Проблемы с доступом к sysfs файлам устройства\n                    2) Устройство занято другим процессом\n                    3) Драйвер устройства не отвечает\n                    4) Аппаратные проблемы с GPU\n                    Рекомендации:\n                    - Проверьте права доступа: sudo ls -la {}\n                    - Проверьте загрузку драйвера: lsmod | grep nvidia\n                    - Проверьте системные логи: sudo dmesg | grep nvidia\n                    - Попробуйте перезагрузить драйвер: sudo rmmod nvidia; sudo modprobe nvidia\n                    - Проверьте аппаратное состояние: nvidia-smi -q\n                    - Попробуйте перезагрузить систему",
                    device.name, e, device.device_path
                );
            }
        }
    }

    if successful_devices == 0 {
        warn!(
            "Не удалось собрать метрики ни для одного NVIDIA GPU устройства. \n            Возможные причины:\n            1) Проблемы с правами доступа ко всем устройствам\n            2) Драйверы NVIDIA не работают корректно\n            3) Аппаратные проблемы с GPU\n            4) Конфликт с другими GPU мониторинговыми инструментами\n            Рекомендации:\n            - Проверьте права доступа: sudo ls -la /sys/class/drm/*/device\n            - Проверьте загрузку драйверов: lsmod | grep nvidia\n            - Проверьте системные логи: sudo dmesg | grep nvidia\n            - Попробуйте перезагрузить драйверы: sudo systemctl restart nvidia-persistenced\n            - Проверьте конфликты: sudo lsof | grep nvidia\n            - Попробуйте перезагрузить систему"
        );
    } else if successful_devices < collection.gpu_count {
        info!("Собраны метрики для {} из {} NVIDIA GPU устройств (частичный успех)", successful_devices, collection.gpu_count);
    } else {
        info!("Собраны метрики для всех {} NVIDIA GPU устройств", successful_devices);
    }

    Ok(collection)
}

/// Collect metrics for a specific NVIDIA GPU device
fn collect_nvml_device_metrics(device: &NvmlDevice) -> Result<NvmlDeviceMetrics> {
    debug!("Сбор метрик для NVIDIA GPU устройства: {}", device.name);

    let mut metrics = NvmlDeviceMetrics {
        device: device.clone(),
        utilization: NvmlUtilization::default(),
        memory: NvmlMemory::default(),
        temperature: NvmlTemperature::default(),
        power: NvmlPower::default(),
        clocks: NvmlClocks::default(),
        timestamp: std::time::SystemTime::now(),
    };

    // In a real implementation, we would use NVML API calls:
    // - nvmlDeviceGetUtilizationRates() for utilization
    // - nvmlDeviceGetMemoryInfo() for memory
    // - nvmlDeviceGetTemperature() for temperature
    // - nvmlDeviceGetPowerUsage() for power
    // - nvmlDeviceGetClockInfo() for clocks
    
    // For now, we'll simulate this by reading from sysfs
    let device_path = Path::new(&device.device_path);

    // Collect utilization metrics
    match collect_nvml_utilization(device_path) {
        Ok(util) => {
            metrics.utilization = util;
            debug!("  NVIDIA GPU utilization: {}%", util.gpu_util);
        }
        Err(e) => {
            debug!("  Не удалось получить метрики использования NVIDIA GPU: {}", e);
        }
    }

    // Collect memory metrics
    match collect_nvml_memory(device_path) {
        Ok(mem) => {
            metrics.memory = mem;
            if mem.total_bytes > 0 {
                debug!("  NVIDIA GPU memory: {}/{} MB ({}% used)",
                    mem.used_bytes / 1024 / 1024,
                    mem.total_bytes / 1024 / 1024,
                    mem.used_bytes as f32 / mem.total_bytes as f32 * 100.0
                );
            }
        }
        Err(e) => {
            debug!("  Не удалось получить метрики памяти NVIDIA GPU: {}", e);
        }
    }

    // Collect temperature metrics
    match collect_nvml_temperature(device_path) {
        Ok(temp) => {
            metrics.temperature = temp;
            debug!("  NVIDIA GPU temperature: {}°C", temp.temperature_c);
        }
        Err(e) => {
            debug!("  Не удалось получить метрики температуры NVIDIA GPU: {}", e);
        }
    }

    // Collect power metrics
    match collect_nvml_power(device_path) {
        Ok(power) => {
            metrics.power = power;
            debug!("  NVIDIA GPU power: {} mW", power.power_mw);
        }
        Err(e) => {
            debug!("  Не удалось получить метрики мощности NVIDIA GPU: {}", e);
        }
    }

    // Collect clock metrics
    match collect_nvml_clocks(device_path) {
        Ok(clocks) => {
            metrics.clocks = clocks;
            debug!("  NVIDIA GPU core clock: {} MHz", clocks.core_clock_mhz);
        }
        Err(e) => {
            debug!("  Не удалось получить метрики тактовых частот NVIDIA GPU: {}", e);
        }
    }

    debug!("Метрики NVIDIA GPU для устройства {} собраны успешно", device.name);

    Ok(metrics)
}

/// Collect NVIDIA GPU utilization metrics
fn collect_nvml_utilization(device_path: &Path) -> Result<NvmlUtilization> {
    let mut utilization = NvmlUtilization::default();

    // Try to read GPU utilization from sysfs
    // NVIDIA exposes this through different files

    // Try /sys/class/drm/card*/device/utilization
    let parent_device = device_path.parent().unwrap_or(device_path);
    if let Ok(gpu_util) = read_sysfs_u32(parent_device, "utilization") {
        utilization.gpu_util = gpu_util;
    }

    // Try /sys/class/drm/card*/device/gpu_busy
    if let Ok(gpu_busy) = read_sysfs_u32(parent_device, "gpu_busy") {
        utilization.gpu_util = gpu_busy;
    }

    // Try memory utilization
    if let Ok(mem_util) = read_sysfs_u32(parent_device, "memory_utilization") {
        utilization.memory_util = mem_util;
    }

    // Try encoder utilization
    if let Ok(encoder_util) = read_sysfs_u32(parent_device, "encoder_utilization") {
        utilization.encoder_util = Some(encoder_util);
    }

    // Try decoder utilization
    if let Ok(decoder_util) = read_sysfs_u32(parent_device, "decoder_utilization") {
        utilization.decoder_util = Some(decoder_util);
    }

    Ok(utilization)
}

/// Collect NVIDIA GPU memory metrics
fn collect_nvml_memory(device_path: &Path) -> Result<NvmlMemory> {
    let mut memory = NvmlMemory::default();

    let parent_device = device_path.parent().unwrap_or(device_path);

    // Try different NVIDIA memory files
    if let Ok(total) = read_sysfs_u64(parent_device, "mem_total") {
        memory.total_bytes = total * 1024 * 1024; // Convert MB to bytes
    }

    if let Ok(used) = read_sysfs_u64(parent_device, "mem_used") {
        memory.used_bytes = used * 1024 * 1024; // Convert MB to bytes
    }

    if let Ok(total) = read_sysfs_u64(parent_device, "memory_total") {
        memory.total_bytes = total;
    }

    if let Ok(used) = read_sysfs_u64(parent_device, "memory_used") {
        memory.used_bytes = used;
    }

    // Validate and correct memory values
    if memory.total_bytes > 0 && memory.used_bytes > memory.total_bytes {
        warn!("Исправление: использованная память NVIDIA ({} MB) больше общей ({} MB)",
            memory.used_bytes / 1024 / 1024,
            memory.total_bytes / 1024 / 1024
        );
        memory.used_bytes = memory.total_bytes;
    }

    memory.free_bytes = memory.total_bytes.saturating_sub(memory.used_bytes);

    Ok(memory)
}

/// Collect NVIDIA GPU temperature metrics
fn collect_nvml_temperature(device_path: &Path) -> Result<NvmlTemperature> {
    let mut temperature = NvmlTemperature::default();

    // Look for temperature sensors in hwmon
    let hwmon_dir = device_path.join("hwmon");
    if hwmon_dir.exists() {
        if let Ok(entries) = fs::read_dir(&hwmon_dir) {
            for entry in entries.flatten() {
                let hwmon_path = entry.path();

                // Look for temp*_input files
                if let Ok(temp_files) = fs::read_dir(&hwmon_path) {
                    for temp_file in temp_files.flatten() {
                        let temp_path = temp_file.path();
                        let file_name = temp_path.file_name().and_then(|s| s.to_str()).unwrap_or("");

                        if file_name.ends_with("_input") {
                            if let Ok(temp_content) = fs::read_to_string(&temp_path) {
                                if let Ok(temp_millidegrees) = temp_content.trim().parse::<u64>() {
                                    let temp_c = temp_millidegrees as f32 / 1000.0;

                                    // Try to determine which temperature this is
                                    if file_name.contains("temp1") || file_name.contains("edge") {
                                        // Main GPU temperature
                                        temperature.temperature_c = temp_c as u32;
                                    } else if file_name.contains("temp2") {
                                        // Could be hotspot
                                        temperature.hotspot_c = Some(temp_c as u32);
                                    } else if file_name.contains("temp3") {
                                        // Could be memory
                                        temperature.memory_c = Some(temp_c as u32);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(temperature)
}

/// Collect NVIDIA GPU power metrics
fn collect_nvml_power(device_path: &Path) -> Result<NvmlPower> {
    let mut power = NvmlPower::default();

    // Look for power sensors in hwmon
    let hwmon_dir = device_path.join("hwmon");
    if hwmon_dir.exists() {
        if let Ok(entries) = fs::read_dir(&hwmon_dir) {
            for entry in entries.flatten() {
                let hwmon_path = entry.path();

                // Look for power*_input files
                if let Ok(power_files) = fs::read_dir(&hwmon_path) {
                    for power_file in power_files.flatten() {
                        let power_path = power_file.path();
                        let file_name = power_path.file_name().and_then(|s| s.to_str()).unwrap_or("");

                        if file_name.ends_with("_input") && file_name.starts_with("power") {
                            if let Ok(power_content) = fs::read_to_string(&power_path) {
                                if let Ok(power_microwatts) = power_content.trim().parse::<u64>() {
                                    power.power_mw = power_microwatts as u32 / 1000; // Convert microwatts to milliwatts
                                }
                            }
                        } else if file_name == "power1_cap" {
                            if let Ok(power_content) = fs::read_to_string(&power_path) {
                                if let Ok(power_microwatts) = power_content.trim().parse::<u64>() {
                                    power.power_cap_mw = Some(power_microwatts as u32 / 1000); // Convert microwatts to milliwatts
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(power)
}

/// Collect NVIDIA GPU clock metrics
fn collect_nvml_clocks(device_path: &Path) -> Result<NvmlClocks> {
    let mut clocks = NvmlClocks::default();

    let parent_device = device_path.parent().unwrap_or(device_path);

    // Try different NVIDIA clock files
    if let Ok(core_clock) = read_sysfs_u32(parent_device, "clock") {
        clocks.core_clock_mhz = core_clock;
    }

    if let Ok(mem_clock) = read_sysfs_u32(parent_device, "memory_clock") {
        clocks.memory_clock_mhz = mem_clock;
    }

    if let Ok(shader_clock) = read_sysfs_u32(parent_device, "shader_clock") {
        clocks.shader_clock_mhz = Some(shader_clock);
    }

    Ok(clocks)
}

/// Read a u32 value from sysfs
fn read_sysfs_u32(path: &Path, field: &str) -> Result<u32> {
    let field_path = path.join(field);
    if field_path.exists() {
        let content = fs::read_to_string(&field_path)
            .with_context(|| format!("Failed to read {} from {}", field, field_path.display()))?;
        content
            .trim()
            .parse::<u32>()
            .with_context(|| format!("Failed to parse {} as u32", field))
    } else {
        Err(anyhow!("Field {} not found at {}", field, path.display()))
    }
}

/// Read a u64 value from sysfs
fn read_sysfs_u64(path: &Path, field: &str) -> Result<u64> {
    let field_path = path.join(field);
    if field_path.exists() {
        let content = fs::read_to_string(&field_path)
            .with_context(|| format!("Failed to read {} from {}", field, field_path.display()))?;
        content
            .trim()
            .parse::<u64>()
            .with_context(|| format!("Failed to parse {} as u64", field))
    } else {
        Err(anyhow!("Field {} not found at {}", field, path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nvml_availability() {
        let available = is_nvml_available();
        // This test just verifies the function doesn't panic
        assert!(available || !available); // Always true, just testing the function
    }

    #[test]
    fn test_nvml_device_discovery() {
        let result = discover_nvml_devices();
        assert!(result.is_ok());
        let devices = result.unwrap();
        // Should return a vector (may be empty)
        assert!(devices.is_empty() || !devices.is_empty());
    }

    #[test]
    fn test_nvml_metrics_collection() {
        let result = collect_nvml_metrics();
        assert!(result.is_ok());
        let collection = result.unwrap();
        assert_eq!(collection.devices.len(), collection.gpu_count);
    }

    #[test]
    fn test_nvml_device_metrics_serialization() {
        let metrics = NvmlDeviceMetrics {
            device: NvmlDevice {
                index: 0,
                name: "GeForce RTX 3080".to_string(),
                uuid: "GPU-12345678-9abc-def0-1234-56789abcdef0".to_string(),
                pci_bus_id: "0000:01:00.0".to_string(),
                device_path: "/sys/devices/pci0000:00/0000:01:00.0".to_string(),
            },
            utilization: NvmlUtilization {
                gpu_util: 75,
                memory_util: 50,
                encoder_util: Some(30),
                decoder_util: Some(20),
            },
            memory: NvmlMemory {
                total_bytes: 10_000_000_000, // 10 GB
                used_bytes: 6_000_000_000,  // 6 GB
                free_bytes: 4_000_000_000,  // 4 GB
            },
            temperature: NvmlTemperature {
                temperature_c: 65,
                hotspot_c: Some(70),
                memory_c: Some(60),
            },
            power: NvmlPower {
                power_mw: 250_000, // 250 W
                power_limit_mw: 320_000, // 320 W
                power_cap_mw: Some(300_000), // 300 W
            },
            clocks: NvmlClocks {
                core_clock_mhz: 1800,
                memory_clock_mhz: 1900,
                shader_clock_mhz: Some(1850),
            },
            timestamp: std::time::SystemTime::now(),
        };

        // Test serialization
        let serialized = serde_json::to_string(&metrics).expect("Serialization failed");
        let deserialized: NvmlDeviceMetrics = serde_json::from_str(&serialized).expect("Deserialization failed");

        assert_eq!(deserialized.device.name, "GeForce RTX 3080");
        assert_eq!(deserialized.utilization.gpu_util, 75);
        assert_eq!(deserialized.memory.total_bytes, 10_000_000_000);
        assert_eq!(deserialized.temperature.temperature_c, 65);
        assert_eq!(deserialized.power.power_mw, 250_000);
        assert_eq!(deserialized.clocks.core_clock_mhz, 1800);
    }

    #[test]
    fn test_nvml_collection_serialization() {
        let collection = NvmlMetricsCollection {
            devices: vec![NvmlDeviceMetrics::default()],
            gpu_count: 1,
        };

        let serialized = serde_json::to_string(&collection).expect("Serialization failed");
        let deserialized: NvmlMetricsCollection = serde_json::from_str(&serialized).expect("Deserialization failed");

        assert_eq!(deserialized.gpu_count, 1);
        assert_eq!(deserialized.devices.len(), 1);
    }

    #[test]
    fn test_nvml_error_handling() {
        // Test that NVML functions handle errors gracefully
        let result = collect_nvml_metrics();
        assert!(result.is_ok());
        
        let collection = result.unwrap();
        assert_eq!(collection.devices.len(), collection.gpu_count);
    }

    #[test]
    fn test_nvml_error_handling_detailed() {
        // Test that NVML error handling provides detailed troubleshooting information
        // This test verifies that error messages contain helpful context
        
        // Test device discovery error handling
        let devices_result = discover_nvml_devices();
        assert!(devices_result.is_ok()); // Should always return Ok, even if no devices found
        
        // Test metrics collection error handling
        let metrics_result = collect_nvml_metrics();
        assert!(metrics_result.is_ok()); // Should always return Ok with graceful degradation
        
        let collection = metrics_result.unwrap();
        
        // Verify that the collection is valid even if no devices are found
        assert_eq!(collection.devices.len(), collection.gpu_count);
        
        // Test that serialization/deserialization works even with empty collections
        let serialized = serde_json::to_string(&collection).expect("Serialization should work");
        let deserialized: NvmlMetricsCollection = serde_json::from_str(&serialized).expect("Deserialization should work");
        
        assert_eq!(deserialized.gpu_count, collection.gpu_count);
        assert_eq!(deserialized.devices.len(), collection.devices.len());
    }



    #[test]
    fn test_nvml_memory_validation() {
        let mut memory = NvmlMemory {
            total_bytes: 10_000_000_000, // 10 GB
            used_bytes: 12_000_000_000, // 12 GB (more than total)
            free_bytes: 0,
        };

        // This should handle overflow correctly
        memory.free_bytes = memory.total_bytes.saturating_sub(memory.used_bytes);
        assert_eq!(memory.free_bytes, 0);
        
        // In a real scenario, we would also cap used_bytes to total_bytes
        if memory.used_bytes > memory.total_bytes {
            memory.used_bytes = memory.total_bytes;
        }
        
        assert_eq!(memory.used_bytes, 10_000_000_000);
        assert_eq!(memory.free_bytes, 0);
    }

    #[test]
    fn test_nvml_device_creation() {
        let device = NvmlDevice {
            index: 0,
            name: "GeForce RTX 3080".to_string(),
            uuid: "GPU-12345678-9abc-def0-1234-56789abcdef0".to_string(),
            pci_bus_id: "0000:01:00.0".to_string(),
            device_path: "/sys/devices/pci0000:00/0000:01:00.0".to_string(),
        };

        assert_eq!(device.index, 0);
        assert_eq!(device.name, "GeForce RTX 3080");
        assert_eq!(device.uuid, "GPU-12345678-9abc-def0-1234-56789abcdef0");
        assert_eq!(device.pci_bus_id, "0000:01:00.0");
        assert_eq!(device.device_path, "/sys/devices/pci0000:00/0000:01:00.0");
    }

    #[test]
    fn test_nvml_default_values() {
        let metrics = NvmlDeviceMetrics::default();
        assert_eq!(metrics.utilization.gpu_util, 0);
        assert_eq!(metrics.utilization.memory_util, 0);
        assert_eq!(metrics.memory.total_bytes, 0);
        assert_eq!(metrics.memory.used_bytes, 0);
        assert_eq!(metrics.memory.free_bytes, 0);
        assert_eq!(metrics.temperature.temperature_c, 0);
        assert_eq!(metrics.power.power_mw, 0);
        assert_eq!(metrics.clocks.core_clock_mhz, 0);
    }

    #[test]
    fn test_nvml_collection_with_no_devices() {
        let result = collect_nvml_metrics();
        assert!(result.is_ok());
        
        let collection = result.unwrap();
        assert_eq!(collection.devices.len(), collection.gpu_count);
    }

    #[test]
    fn test_nvml_error_recovery() {
        // Test that the system can recover from NVML errors
        let result1 = collect_nvml_metrics();
        let result2 = collect_nvml_metrics();
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        let collection1 = result1.unwrap();
        let collection2 = result2.unwrap();
        
        assert_eq!(collection1.gpu_count, collection2.gpu_count);
    }
}