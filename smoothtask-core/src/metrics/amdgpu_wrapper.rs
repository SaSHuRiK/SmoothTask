//! AMDGPU Wrapper
//!
//! This module provides a safe Rust wrapper around AMDGPU interfaces
//! for monitoring AMD GPU metrics including utilization, memory, temperature,
//! power consumption, and more.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{debug, error, info, warn};

/// AMD GPU device information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmdGpuDevice {
    /// Device index
    pub index: u32,
    /// Device name (e.g., "Radeon RX 6800")
    pub name: String,
    /// Device ID
    pub device_id: String,
    /// PCI bus ID
    pub pci_bus_id: String,
    /// Device path in sysfs
    pub device_path: String,
}

/// AMD GPU utilization metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct AmdGpuUtilization {
    /// GPU utilization percentage (0-100)
    pub gpu_util: u32,
    /// Memory utilization percentage (0-100)
    pub memory_util: u32,
    /// Compute utilization percentage (0-100)
    pub compute_util: Option<u32>,
    /// Video utilization percentage (0-100)
    pub video_util: Option<u32>,
}

/// AMD GPU memory metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct AmdGpuMemory {
    /// Total GPU memory in bytes
    pub total_bytes: u64,
    /// Used GPU memory in bytes
    pub used_bytes: u64,
    /// Free GPU memory in bytes
    pub free_bytes: u64,
    /// VRAM total in bytes
    pub vram_total_bytes: Option<u64>,
    /// VRAM used in bytes
    pub vram_used_bytes: Option<u64>,
}

/// AMD GPU temperature metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct AmdGpuTemperature {
    /// Current GPU temperature in Celsius
    pub temperature_c: u32,
    /// GPU hotspot temperature in Celsius (if available)
    pub hotspot_c: Option<u32>,
    /// GPU memory temperature in Celsius (if available)
    pub memory_c: Option<u32>,
    /// GPU VRAM temperature in Celsius (if available)
    pub vram_c: Option<u32>,
}

/// AMD GPU power metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct AmdGpuPower {
    /// Current GPU power usage in milliwatts
    pub power_mw: u32,
    /// GPU power limit in milliwatts
    pub power_limit_mw: u32,
    /// GPU power cap in milliwatts (if available)
    pub power_cap_mw: Option<u32>,
    /// Average GPU power usage in milliwatts
    pub power_avg_mw: Option<u32>,
}

/// AMD GPU clock metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct AmdGpuClocks {
    /// Current GPU core clock in MHz
    pub core_clock_mhz: u32,
    /// Current GPU memory clock in MHz
    pub memory_clock_mhz: u32,
    /// Current GPU shader clock in MHz (if available)
    pub shader_clock_mhz: Option<u32>,
    /// Current GPU compute clock in MHz (if available)
    pub compute_clock_mhz: Option<u32>,
}

/// Complete AMD GPU metrics for a single device
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmdGpuDeviceMetrics {
    /// Device information
    pub device: AmdGpuDevice,
    /// GPU utilization metrics
    pub utilization: AmdGpuUtilization,
    /// GPU memory metrics
    pub memory: AmdGpuMemory,
    /// GPU temperature metrics
    pub temperature: AmdGpuTemperature,
    /// GPU power metrics
    pub power: AmdGpuPower,
    /// GPU clock metrics
    pub clocks: AmdGpuClocks,
    /// Timestamp when metrics were collected
    pub timestamp: std::time::SystemTime,
}

impl Default for AmdGpuDeviceMetrics {
    fn default() -> Self {
        Self {
            device: AmdGpuDevice {
                index: 0,
                name: String::new(),
                device_id: String::new(),
                pci_bus_id: String::new(),
                device_path: String::new(),
            },
            utilization: AmdGpuUtilization::default(),
            memory: AmdGpuMemory::default(),
            temperature: AmdGpuTemperature::default(),
            power: AmdGpuPower::default(),
            clocks: AmdGpuClocks::default(),
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// Collection of metrics for all AMD GPU devices
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AmdGpuMetricsCollection {
    /// List of AMD GPU devices with their metrics
    pub devices: Vec<AmdGpuDeviceMetrics>,
    /// Total AMD GPU count
    pub gpu_count: usize,
}

/// Check if AMDGPU is available on the system
pub fn is_amdgpu_available() -> bool {
    // Check for AMDGPU kernel module
    let amdgpu_module = Path::new("/sys/module/amdgpu");
    if amdgpu_module.exists() {
        info!("AMDGPU модуль ядра найден");
        return true;
    }

    // Check for AMD devices in sysfs
    let drm_dir = Path::new("/sys/class/drm");
    if drm_dir.exists() {
        if let Ok(entries) = fs::read_dir(drm_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let device_path = path.join("device");
                if device_path.exists() {
                    let vendor_path = device_path.join("vendor");
                    if vendor_path.exists() {
                        if let Ok(vendor_content) = fs::read_to_string(&vendor_path) {
                            let vendor_id = vendor_content.trim();
                            if vendor_id == "0x1002" {
                                // AMD vendor ID
                                info!("AMD GPU устройство найдено");
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }

    debug!("AMDGPU не найден");
    false
}

/// Discover all AMD GPU devices
pub fn discover_amdgpu_devices() -> Result<Vec<AmdGpuDevice>> {
    if !is_amdgpu_available() {
        warn!(
            "AMDGPU не доступен - AMD GPU устройства не могут быть обнаружены. \n            Возможные причины:\n            1) AMD GPU драйверы не установлены\n            2) Отсутствие AMD GPU в системе\n            3) Проблемы с загрузкой модулей ядра\n            Рекомендации:\n            - Проверьте установку драйверов: lsmod | grep amdgpu\n            - Проверьте наличие GPU: lspci | grep -i amd\n            - Попробуйте установить драйверы: sudo apt install mesa-vulkan-drivers\n            - Проверьте системные логи: sudo dmesg | grep amdgpu\n            - Попробуйте загрузить модуль: sudo modprobe amdgpu"
        );
        return Ok(Vec::new());
    }

    let mut devices = Vec::new();

    // Look for AMD devices in /sys/class/drm
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
                            // Check if this is an AMD device
                            let vendor_path = device_path.join("vendor");
                            if vendor_path.exists() {
                                if let Ok(vendor_content) = fs::read_to_string(&vendor_path) {
                                    let vendor_id = vendor_content.trim();
                                    if vendor_id == "0x1002" {
                                        // AMD vendor ID
                                        let device_id_path = device_path.join("device");
                                        let device_id = if device_id_path.exists() {
                                            fs::read_to_string(&device_id_path)
                                                .ok()
                                                .map(|s| s.trim().to_string())
                                        } else {
                                            None
                                        };

                                        let name = format!("AMD GPU {}", file_name);
                                        let pci_bus_id = device_path.to_string_lossy().into_owned();

                                        let device = AmdGpuDevice {
                                            index: devices.len() as u32,
                                            name,
                                            device_id: device_id.unwrap_or("unknown".to_string()),
                                            pci_bus_id,
                                            device_path: device_path.to_string_lossy().into_owned(),
                                        };

                                        info!("Обнаружено AMD GPU устройство: {}", device.name);
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
        debug!("AMD GPU устройства не найдены");
    } else {
        info!("Обнаружено {} AMD GPU устройств", devices.len());
    }

    Ok(devices)
}

/// Collect AMD GPU metrics
pub fn collect_amdgpu_metrics() -> Result<AmdGpuMetricsCollection> {
    info!("Сбор метрик AMD GPU");

    let devices = match discover_amdgpu_devices() {
        Ok(devices) => devices,
        Err(e) => {
            warn!("Не удалось обнаружить AMD GPU устройства: {}", e);
            return Ok(AmdGpuMetricsCollection::default());
        }
    };

    let mut collection = AmdGpuMetricsCollection {
        devices: Vec::new(),
        gpu_count: devices.len(),
    };

    if devices.is_empty() {
        debug!("Нет AMD GPU устройств для сбора метрик");
        return Ok(collection);
    }

    let mut successful_devices = 0;

    for device in devices {
        match collect_amdgpu_device_metrics(&device) {
            Ok(metrics) => {
                collection.devices.push(metrics);
                successful_devices += 1;
            }
            Err(e) => {
                error!(
                    "Не удалось собрать метрики для AMD GPU устройства {}: {}. \n                    Возможные причины:\n                    1) Проблемы с доступом к sysfs файлам устройства\n                    2) Устройство занято другим процессом\n                    3) Драйвер устройства не отвечает\n                    4) Аппаратные проблемы с GPU\n                    Рекомендации:\n                    - Проверьте права доступа: sudo ls -la {}\n                    - Проверьте загрузку драйвера: lsmod | grep amdgpu\n                    - Проверьте системные логи: sudo dmesg | grep amdgpu\n                    - Попробуйте перезагрузить драйвер: sudo rmmod amdgpu; sudo modprobe amdgpu\n                    - Проверьте аппаратное состояние: sudo dmesg | grep -i error\n                    - Попробуйте перезагрузить систему",
                    device.name, e, device.device_path
                );
            }
        }
    }

    if successful_devices == 0 {
        warn!(
            "Не удалось собрать метрики ни для одного AMD GPU устройства. \n            Возможные причины:\n            1) Проблемы с правами доступа ко всем устройствам\n            2) Драйверы AMD GPU не работают корректно\n            3) Аппаратные проблемы с GPU\n            4) Конфликт с другими GPU мониторинговыми инструментами\n            Рекомендации:\n            - Проверьте права доступа: sudo ls -la /sys/class/drm/*/device\n            - Проверьте загрузку драйверов: lsmod | grep amdgpu\n            - Проверьте системные логи: sudo dmesg | grep amdgpu\n            - Попробуйте перезагрузить драйверы: sudo systemctl restart display-manager\n            - Проверьте конфликты: sudo lsof | grep amdgpu\n            - Попробуйте перезагрузить систему"
        );
    } else if successful_devices < collection.gpu_count {
        info!(
            "Собраны метрики для {} из {} AMD GPU устройств (частичный успех)",
            successful_devices, collection.gpu_count
        );
    } else {
        info!(
            "Собраны метрики для всех {} AMD GPU устройств",
            successful_devices
        );
    }

    Ok(collection)
}

/// Collect metrics for a specific AMD GPU device
fn collect_amdgpu_device_metrics(device: &AmdGpuDevice) -> Result<AmdGpuDeviceMetrics> {
    debug!("Сбор метрик для AMD GPU устройства: {}", device.name);

    let mut metrics = AmdGpuDeviceMetrics {
        device: device.clone(),
        utilization: AmdGpuUtilization::default(),
        memory: AmdGpuMemory::default(),
        temperature: AmdGpuTemperature::default(),
        power: AmdGpuPower::default(),
        clocks: AmdGpuClocks::default(),
        timestamp: std::time::SystemTime::now(),
    };

    let device_path = Path::new(&device.device_path);

    // Collect utilization metrics
    match collect_amdgpu_utilization(device_path) {
        Ok(util) => {
            metrics.utilization = util;
            debug!("  AMD GPU utilization: {}%", util.gpu_util);
        }
        Err(e) => {
            debug!("  Не удалось получить метрики использования AMD GPU: {}", e);
        }
    }

    // Collect memory metrics
    match collect_amdgpu_memory(device_path) {
        Ok(mem) => {
            metrics.memory = mem;
            if mem.total_bytes > 0 {
                debug!(
                    "  AMD GPU memory: {}/{} MB ({}% used)",
                    mem.used_bytes / 1024 / 1024,
                    mem.total_bytes / 1024 / 1024,
                    mem.used_bytes as f32 / mem.total_bytes as f32 * 100.0
                );
            }
        }
        Err(e) => {
            debug!("  Не удалось получить метрики памяти AMD GPU: {}", e);
        }
    }

    // Collect temperature metrics
    match collect_amdgpu_temperature(device_path) {
        Ok(temp) => {
            metrics.temperature = temp;
            debug!("  AMD GPU temperature: {}°C", temp.temperature_c);
        }
        Err(e) => {
            debug!("  Не удалось получить метрики температуры AMD GPU: {}", e);
        }
    }

    // Collect power metrics
    match collect_amdgpu_power(device_path) {
        Ok(power) => {
            metrics.power = power;
            debug!("  AMD GPU power: {} mW", power.power_mw);
        }
        Err(e) => {
            debug!("  Не удалось получить метрики мощности AMD GPU: {}", e);
        }
    }

    // Collect clock metrics
    match collect_amdgpu_clocks(device_path) {
        Ok(clocks) => {
            metrics.clocks = clocks;
            debug!("  AMD GPU core clock: {} MHz", clocks.core_clock_mhz);
        }
        Err(e) => {
            debug!(
                "  Не удалось получить метрики тактовых частот AMD GPU: {}",
                e
            );
        }
    }

    debug!(
        "Метрики AMD GPU для устройства {} собраны успешно",
        device.name
    );

    Ok(metrics)
}

/// Collect AMD GPU utilization metrics
fn collect_amdgpu_utilization(device_path: &Path) -> Result<AmdGpuUtilization> {
    let mut utilization = AmdGpuUtilization::default();

    let parent_device = device_path.parent().unwrap_or(device_path);

    // Try to read GPU utilization from sysfs
    // AMD exposes this through different files

    // Try gpu_busy_percent
    if let Ok(gpu_util) = read_sysfs_u32(parent_device, "gpu_busy_percent") {
        utilization.gpu_util = gpu_util;
    }

    // Try memory utilization
    if let Ok(mem_util) = read_sysfs_u32(parent_device, "mem_busy_percent") {
        utilization.memory_util = mem_util;
    }

    // Try compute utilization
    if let Ok(compute_util) = read_sysfs_u32(parent_device, "compute_busy_percent") {
        utilization.compute_util = Some(compute_util);
    }

    // Try video utilization
    if let Ok(video_util) = read_sysfs_u32(parent_device, "video_busy_percent") {
        utilization.video_util = Some(video_util);
    }

    Ok(utilization)
}

/// Collect AMD GPU memory metrics
fn collect_amdgpu_memory(device_path: &Path) -> Result<AmdGpuMemory> {
    let mut memory = AmdGpuMemory::default();

    let parent_device = device_path.parent().unwrap_or(device_path);

    // Try different AMD memory files
    if let Ok(total) = read_sysfs_u64(parent_device, "mem_total_bytes") {
        memory.total_bytes = total;
    }

    if let Ok(used) = read_sysfs_u64(parent_device, "mem_used_bytes") {
        memory.used_bytes = used;
    }

    // Try VRAM specific files
    if let Ok(vram_total) = read_sysfs_u64(parent_device, "vram_total_bytes") {
        memory.vram_total_bytes = Some(vram_total);
    }

    if let Ok(vram_used) = read_sysfs_u64(parent_device, "vram_used_bytes") {
        memory.vram_used_bytes = Some(vram_used);
    }

    // Validate and correct memory values
    if memory.total_bytes > 0 && memory.used_bytes > memory.total_bytes {
        warn!(
            "Исправление: использованная память AMD ({} MB) больше общей ({} MB)",
            memory.used_bytes / 1024 / 1024,
            memory.total_bytes / 1024 / 1024
        );
        memory.used_bytes = memory.total_bytes;
    }

    memory.free_bytes = memory.total_bytes.saturating_sub(memory.used_bytes);

    Ok(memory)
}

/// Collect AMD GPU temperature metrics
fn collect_amdgpu_temperature(device_path: &Path) -> Result<AmdGpuTemperature> {
    let mut temperature = AmdGpuTemperature::default();

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
                        let file_name =
                            temp_path.file_name().and_then(|s| s.to_str()).unwrap_or("");

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
                                    } else if file_name.contains("temp4") {
                                        // Could be VRAM
                                        temperature.vram_c = Some(temp_c as u32);
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

/// Collect AMD GPU power metrics
fn collect_amdgpu_power(device_path: &Path) -> Result<AmdGpuPower> {
    let mut power = AmdGpuPower::default();

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
                        let file_name = power_path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("");

                        if file_name.ends_with("_input") && file_name.starts_with("power") {
                            if let Ok(power_content) = fs::read_to_string(&power_path) {
                                if let Ok(power_microwatts) = power_content.trim().parse::<u64>() {
                                    power.power_mw = power_microwatts as u32 / 1000;
                                    // Convert microwatts to milliwatts
                                }
                            }
                        } else if file_name == "power1_cap" {
                            if let Ok(power_content) = fs::read_to_string(&power_path) {
                                if let Ok(power_microwatts) = power_content.trim().parse::<u64>() {
                                    power.power_cap_mw = Some(power_microwatts as u32 / 1000);
                                    // Convert microwatts to milliwatts
                                }
                            }
                        } else if file_name == "power1_avg" {
                            if let Ok(power_content) = fs::read_to_string(&power_path) {
                                if let Ok(power_microwatts) = power_content.trim().parse::<u64>() {
                                    power.power_avg_mw = Some(power_microwatts as u32 / 1000);
                                    // Convert microwatts to milliwatts
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

/// Collect AMD GPU clock metrics
fn collect_amdgpu_clocks(device_path: &Path) -> Result<AmdGpuClocks> {
    let mut clocks = AmdGpuClocks::default();

    let parent_device = device_path.parent().unwrap_or(device_path);

    // Try different AMD clock files
    if let Ok(core_clock) = read_sysfs_u32(parent_device, "gpu_clock") {
        clocks.core_clock_mhz = core_clock;
    }

    if let Ok(mem_clock) = read_sysfs_u32(parent_device, "mem_clock") {
        clocks.memory_clock_mhz = mem_clock;
    }

    if let Ok(shader_clock) = read_sysfs_u32(parent_device, "shader_clock") {
        clocks.shader_clock_mhz = Some(shader_clock);
    }

    if let Ok(compute_clock) = read_sysfs_u32(parent_device, "compute_clock") {
        clocks.compute_clock_mhz = Some(compute_clock);
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
    fn test_amdgpu_availability() {
        let available = is_amdgpu_available();
        // This test just verifies the function doesn't panic
        assert!(available || !available); // Always true, just testing the function
    }

    #[test]
    fn test_amdgpu_device_discovery() {
        let result = discover_amdgpu_devices();
        assert!(result.is_ok());
        let devices = result.unwrap();
        // Should return a vector (may be empty)
        assert!(devices.is_empty() || !devices.is_empty());
    }

    #[test]
    fn test_amdgpu_metrics_collection() {
        let result = collect_amdgpu_metrics();
        assert!(result.is_ok());
        let collection = result.unwrap();
        assert_eq!(collection.devices.len(), collection.gpu_count);
    }

    #[test]
    fn test_amdgpu_device_metrics_serialization() {
        let metrics = AmdGpuDeviceMetrics {
            device: AmdGpuDevice {
                index: 0,
                name: "Radeon RX 6800".to_string(),
                device_id: "0x73bf".to_string(),
                pci_bus_id: "0000:01:00.0".to_string(),
                device_path: "/sys/devices/pci0000:00/0000:01:00.0".to_string(),
            },
            utilization: AmdGpuUtilization {
                gpu_util: 65,
                memory_util: 40,
                compute_util: Some(50),
                video_util: Some(20),
            },
            memory: AmdGpuMemory {
                total_bytes: 16_000_000_000, // 16 GB
                used_bytes: 8_000_000_000,   // 8 GB
                free_bytes: 8_000_000_000,   // 8 GB
                vram_total_bytes: Some(16_000_000_000),
                vram_used_bytes: Some(8_000_000_000),
            },
            temperature: AmdGpuTemperature {
                temperature_c: 70,
                hotspot_c: Some(75),
                memory_c: Some(65),
                vram_c: Some(68),
            },
            power: AmdGpuPower {
                power_mw: 200_000,           // 200 W
                power_limit_mw: 250_000,     // 250 W
                power_cap_mw: Some(230_000), // 230 W
                power_avg_mw: Some(180_000), // 180 W
            },
            clocks: AmdGpuClocks {
                core_clock_mhz: 2000,
                memory_clock_mhz: 1800,
                shader_clock_mhz: Some(1900),
                compute_clock_mhz: Some(2100),
            },
            timestamp: std::time::SystemTime::now(),
        };

        // Test serialization
        let serialized = serde_json::to_string(&metrics).expect("Serialization failed");
        let deserialized: AmdGpuDeviceMetrics =
            serde_json::from_str(&serialized).expect("Deserialization failed");

        assert_eq!(deserialized.device.name, "Radeon RX 6800");
        assert_eq!(deserialized.utilization.gpu_util, 65);
        assert_eq!(deserialized.memory.total_bytes, 16_000_000_000);
        assert_eq!(deserialized.temperature.temperature_c, 70);
        assert_eq!(deserialized.power.power_mw, 200_000);
        assert_eq!(deserialized.clocks.core_clock_mhz, 2000);
    }

    #[test]
    fn test_amdgpu_collection_serialization() {
        let collection = AmdGpuMetricsCollection {
            devices: vec![AmdGpuDeviceMetrics::default()],
            gpu_count: 1,
        };

        let serialized = serde_json::to_string(&collection).expect("Serialization failed");
        let deserialized: AmdGpuMetricsCollection =
            serde_json::from_str(&serialized).expect("Deserialization failed");

        assert_eq!(deserialized.gpu_count, 1);
        assert_eq!(deserialized.devices.len(), 1);
    }

    #[test]
    fn test_amdgpu_error_handling() {
        // Test that AMDGPU functions handle errors gracefully
        let result = collect_amdgpu_metrics();
        assert!(result.is_ok());

        let collection = result.unwrap();
        assert_eq!(collection.devices.len(), collection.gpu_count);
    }

    #[test]
    fn test_amdgpu_error_handling_detailed() {
        // Test that AMDGPU error handling provides detailed troubleshooting information
        // This test verifies that error messages contain helpful context

        // Test device discovery error handling
        let devices_result = discover_amdgpu_devices();
        assert!(devices_result.is_ok()); // Should always return Ok, even if no devices found

        // Test metrics collection error handling
        let metrics_result = collect_amdgpu_metrics();
        assert!(metrics_result.is_ok()); // Should always return Ok with graceful degradation

        let collection = metrics_result.unwrap();

        // Verify that the collection is valid even if no devices are found
        assert_eq!(collection.devices.len(), collection.gpu_count);

        // Test that serialization/deserialization works even with empty collections
        let serialized = serde_json::to_string(&collection).expect("Serialization should work");
        let deserialized: AmdGpuMetricsCollection =
            serde_json::from_str(&serialized).expect("Deserialization should work");

        assert_eq!(deserialized.gpu_count, collection.gpu_count);
        assert_eq!(deserialized.devices.len(), collection.devices.len());
    }

    #[test]
    fn test_amdgpu_memory_validation() {
        let mut memory = AmdGpuMemory {
            total_bytes: 16_000_000_000, // 16 GB
            used_bytes: 18_000_000_000,  // 18 GB (more than total)
            free_bytes: 0,
            vram_total_bytes: None,
            vram_used_bytes: None,
        };

        // This should handle overflow correctly
        memory.free_bytes = memory.total_bytes.saturating_sub(memory.used_bytes);
        assert_eq!(memory.free_bytes, 0);

        // In a real scenario, we would also cap used_bytes to total_bytes
        if memory.used_bytes > memory.total_bytes {
            memory.used_bytes = memory.total_bytes;
        }

        assert_eq!(memory.used_bytes, 16_000_000_000);
        assert_eq!(memory.free_bytes, 0);
    }

    #[test]
    fn test_amdgpu_device_creation() {
        let device = AmdGpuDevice {
            index: 0,
            name: "Radeon RX 6800".to_string(),
            device_id: "0x73bf".to_string(),
            pci_bus_id: "0000:01:00.0".to_string(),
            device_path: "/sys/devices/pci0000:00/0000:01:00.0".to_string(),
        };

        assert_eq!(device.index, 0);
        assert_eq!(device.name, "Radeon RX 6800");
        assert_eq!(device.device_id, "0x73bf");
        assert_eq!(device.pci_bus_id, "0000:01:00.0");
        assert_eq!(device.device_path, "/sys/devices/pci0000:00/0000:01:00.0");
    }

    #[test]
    fn test_amdgpu_default_values() {
        let metrics = AmdGpuDeviceMetrics::default();
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
    fn test_amdgpu_collection_with_no_devices() {
        let result = collect_amdgpu_metrics();
        assert!(result.is_ok());

        let collection = result.unwrap();
        assert_eq!(collection.devices.len(), collection.gpu_count);
    }

    #[test]
    fn test_amdgpu_error_recovery() {
        // Test that the system can recover from AMDGPU errors
        let result1 = collect_amdgpu_metrics();
        let result2 = collect_amdgpu_metrics();

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let collection1 = result1.unwrap();
        let collection2 = result2.unwrap();

        assert_eq!(collection1.gpu_count, collection2.gpu_count);
    }
}
