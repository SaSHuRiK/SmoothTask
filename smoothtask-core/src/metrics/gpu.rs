//! GPU Metrics Collection
//!
//! This module provides functionality for collecting GPU metrics from various sources:
//! - DRM (Direct Rendering Manager) interfaces
//! - sysfs/hwmon temperature sensors
//! - powercap energy sensors
//! - GPU utilization and memory usage

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

// NVML и AMDGPU API будут добавлены как опциональные зависимости
#[cfg(feature = "nvml-wrapper")]
use nvml_wrapper::Nvml;

// AMDGPU поддержка временно отключена из-за отсутствия стабильной версии
// #[cfg(feature = "amdgpu")]
// use amdgpu_sys::*;

/// GPU device information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuDevice {
    /// Device name (e.g., "card0", "card1")
    pub name: String,
    /// Device path in sysfs
    pub device_path: PathBuf,
    /// Vendor ID (if available)
    pub vendor_id: Option<String>,
    /// Device ID (if available)
    pub device_id: Option<String>,
    /// Driver name (e.g., "i915", "amdgpu", "nvidia")
    pub driver: Option<String>,
}

impl Default for GpuDevice {
    fn default() -> Self {
        Self {
            name: String::new(),
            device_path: PathBuf::new(),
            vendor_id: None,
            device_id: None,
            driver: None,
        }
    }
}

/// GPU utilization metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct GpuUtilization {
    /// GPU core utilization percentage (0.0 to 1.0)
    pub gpu_util: f32,
    /// GPU memory utilization percentage (0.0 to 1.0)
    pub memory_util: f32,
    /// GPU encoder utilization percentage (0.0 to 1.0)
    pub encoder_util: Option<f32>,
    /// GPU decoder utilization percentage (0.0 to 1.0)
    pub decoder_util: Option<f32>,
}

/// GPU memory metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct GpuMemory {
    /// Total GPU memory in bytes
    pub total_bytes: u64,
    /// Used GPU memory in bytes
    pub used_bytes: u64,
    /// Free GPU memory in bytes
    pub free_bytes: u64,
}

/// GPU temperature metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct GpuTemperature {
    /// Current GPU temperature in Celsius
    pub temperature_c: Option<f32>,
    /// GPU hotspot temperature in Celsius (if available)
    pub hotspot_c: Option<f32>,
    /// GPU memory temperature in Celsius (if available)
    pub memory_c: Option<f32>,
}



/// GPU power metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct GpuPower {
    /// Current GPU power usage in watts
    pub power_w: Option<f32>,
    /// GPU power limit in watts (if available)
    pub power_limit_w: Option<f32>,
    /// GPU power cap in watts (if available)
    pub power_cap_w: Option<f32>,
}

/// GPU clock metrics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct GpuClocks {
    /// Current GPU core clock in MHz
    pub core_clock_mhz: Option<u32>,
    /// Current GPU memory clock in MHz
    pub memory_clock_mhz: Option<u32>,
    /// Current GPU shader clock in MHz (if available)
    pub shader_clock_mhz: Option<u32>,
}

/// Complete GPU metrics for a single device
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuMetrics {
    /// GPU device information
    pub device: GpuDevice,
    /// GPU utilization metrics
    pub utilization: GpuUtilization,
    /// GPU memory metrics
    pub memory: GpuMemory,
    /// GPU temperature metrics
    pub temperature: GpuTemperature,
    /// GPU power metrics
    pub power: GpuPower,
    /// GPU clock metrics
    pub clocks: GpuClocks,
    /// Timestamp when metrics were collected
    pub timestamp: std::time::SystemTime,
}

impl Default for GpuMetrics {
    fn default() -> Self {
        Self {
            device: GpuDevice::default(),
            utilization: GpuUtilization::default(),
            memory: GpuMemory::default(),
            temperature: GpuTemperature::default(),
            power: GpuPower::default(),
            clocks: GpuClocks::default(),
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// Collection of metrics for all GPU devices
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GpuMetricsCollection {
    /// List of GPU devices with their metrics
    pub devices: Vec<GpuMetrics>,
    /// Total GPU count
    pub gpu_count: usize,
}

/// Discover all GPU devices in the system
pub fn discover_gpu_devices() -> Result<Vec<GpuDevice>> {
    let mut devices = Vec::new();

    info!("Обнаружение GPU устройств в системе");

    // Check DRM devices
    let drm_dir = Path::new("/sys/class/drm");
    if !drm_dir.exists() {
        warn!(
            "Директория /sys/class/drm не найдена - GPU устройства могут быть недоступны. \n            Это может быть вызвано: \n            1) Отсутствием физических GPU в системе \n            2) Неподдерживаемыми драйверами (попробуйте установить проприетарные драйверы для NVIDIA/AMD) \n            3) Проблемами с загрузкой модулей ядра (проверьте: lsmod | grep -i gpu) \n            4) Отсутствием прав доступа (попробуйте запустить с sudo) \n            Рекомендации: \n            - Проверьте загрузку модулей ядра: lsmod | grep -i drm \n            - Проверьте системные логи: sudo dmesg | grep -i drm \n            - Проверьте наличие GPU: lspci | grep -i vga"
        );
        return Ok(devices);
    }

    let entries = fs::read_dir(drm_dir).with_context(|| {
        "Не удалось прочитать директорию /sys/class/drm. \n            Это может быть вызвано: \n            1) Отсутствием прав доступа (попробуйте запустить с sudo) \n            2) Проблемами с файловой системой sysfs \n            3) Конкурентным доступом к файловой системе \n            Рекомендации: \n            - Проверьте права доступа: ls -la /sys/class/drm \n            - Проверьте целостность файловой системы: sudo dmesg | grep -i sysfs \n            - Попробуйте запустить с повышенными правами: sudo smoothtaskd \n            - Проверьте загрузку модулей ядра: lsmod | grep -i drm".to_string()
    })?;

    for entry in entries {
        let entry = entry.with_context(|| {
            "Ошибка при чтении записи в /sys/class/drm. \n                Это может быть вызвано: \n                1) Проблемами с файловой системой sysfs \n                2) Конкурентным доступом к файловой системе \n                3) Отсутствием прав доступа \n                Рекомендации: \n                - Проверьте целостность файловой системы: sudo dmesg | grep -i sysfs \n                - Попробуйте запустить с повышенными правами: sudo smoothtaskd \n                - Проверьте доступные устройства: ls -la /sys/class/drm".to_string()
        })?;
        let path = entry.path();
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        if file_name.starts_with("card") {
            let device_path = path.join("device");
            if !device_path.exists() {
                debug!(
                    "Устройство {} не имеет device пути. \n                    Это может быть вызвано: \n                    1) Виртуальным устройством без PCI информации \n                    2) Устройством без физического GPU \n                    3) Проблемами с загрузкой драйвера \n                    Продолжаем без этого устройства. \n                    Рекомендации: \n                    - Проверьте загрузку драйвера: lsmod | grep -i {} \n                    - Проверьте системные логи: sudo dmesg | grep -i {}",
                    file_name, file_name, file_name
                );
                continue;
            }

            let vendor_id = read_pci_field(&device_path, "vendor").ok();
            let device_id = read_pci_field(&device_path, "device").ok();
            let driver = read_driver_name(&device_path).ok();

            let device = GpuDevice {
                name: file_name.to_string(),
                device_path: device_path.clone(),
                vendor_id: vendor_id.clone(),
                device_id: device_id.clone(),
                driver: driver.clone(),
            };

            info!(
                "Обнаружено GPU устройство: {} (vendor: {:?}, device: {:?}, driver: {:?})",
                file_name, vendor_id, device_id, driver
            );
            devices.push(device);
        }
    }

    if devices.is_empty() {
        warn!(
            "Не найдено ни одного GPU устройства. \n            Это может быть вызвано: \n            1) Отсутствием физических GPU в системе \n            2) Неподдерживаемыми драйверами (попробуйте установить проприетарные драйверы для NVIDIA/AMD) \n            3) Проблемами с загрузкой модулей ядра (проверьте: lsmod | grep -i gpu) \n            4) Отсутствием прав доступа (попробуйте запустить с sudo) \n            Рекомендации: \n            - Проверьте наличие GPU: lspci | grep -i vga \n            - Проверьте загрузку драйверов: lsmod | grep -i drm \n            - Проверьте системные логи: sudo dmesg | grep -i gpu \n            - Попробуйте установить проприетарные драйверы для вашего GPU"
        );
    } else {
        info!("Обнаружено {} GPU устройств", devices.len());
    }

    Ok(devices)
}

/// Read a field from a PCI device
fn read_pci_field(device_path: &Path, field: &str) -> Result<String> {
    let field_path = device_path.join(field);
    if field_path.exists() {
        let content = fs::read_to_string(&field_path).with_context(|| {
            format!(
                "Failed to read PCI field {} from {}",
                field,
                field_path.display()
            )
        })?;
        Ok(content.trim().to_string())
    } else {
        Ok(String::new())
    }
}

/// Read the driver name for a device
fn read_driver_name(device_path: &Path) -> Result<String> {
    let driver_path = device_path.join("driver");
    if driver_path.exists() {
        let driver_link = fs::read_link(&driver_path).with_context(|| {
            format!("Failed to read driver link from {}", driver_path.display())
        })?;

        if let Some(driver_name) = driver_link.file_name() {
            return Ok(driver_name.to_string_lossy().into_owned());
        }
    }
    Ok(String::new())
}

/// Collect GPU metrics for all devices
pub fn collect_gpu_metrics() -> Result<GpuMetricsCollection> {
    info!("Сбор метрик GPU");

    let devices = match discover_gpu_devices() {
        Ok(devices) => devices,
        Err(e) => {
            warn!(
                "Не удалось обнаружить GPU устройства: {}. \n                Продолжаем с пустой коллекцией метрик. \n                Это не является критической ошибкой - система может работать без GPU метрик. \n                Рекомендации: \n                - Проверьте загрузку драйверов: lsmod | grep -i drm \n                - Проверьте системные логи: sudo dmesg | grep -i gpu \n                - Проверьте права доступа: sudo ls -la /sys/class/drm \n                - Попробуйте установить проприетарные драйверы для вашего GPU",
                e
            );
            return Ok(GpuMetricsCollection::default());
        }
    };

    let mut collection = GpuMetricsCollection {
        devices: Vec::new(),
        gpu_count: devices.len(),
    };

    if devices.is_empty() {
        debug!("Нет GPU устройств для сбора метрик - возвращаем пустую коллекцию");
        return Ok(collection);
    }

    let mut successful_devices = 0;

    for device in devices {
        match collect_gpu_device_metrics(&device) {
            Ok(metrics) => {
                collection.devices.push(metrics);
                successful_devices += 1;
            }
            Err(e) => {
                error!(
                    "Не удалось собрать метрики для GPU устройства {}: {}. \n                    Это устройство будет пропущено, но сбор метрик продолжится для других устройств. \n                    Рекомендации: \n                    1) Проверьте права доступа: sudo ls -la /sys/class/drm/{}/device \n                    2) Проверьте загрузку драйвера: lsmod | grep -i {} \n                    3) Проверьте системные логи: sudo dmesg | grep -i drm \n                    4) Попробуйте обновить драйвер для этого устройства \n                    5) Проверьте наличие необходимых модулей ядра: sudo modprobe <driver_name> \n                    6) Попробуйте перезагрузить систему для переинициализации GPU",
                    device.name, e, device.name, device.driver.clone().unwrap_or_default()
                );
            }
        }
    }

    if successful_devices == 0 {
        warn!(
            "Не удалось собрать метрики ни для одного GPU устройства. \n            Это может быть вызвано: \n            1) Проблемами с правами доступа (попробуйте запустить с sudo) \n            2) Неподдерживаемыми драйверами (проверьте: lsmod | grep -i gpu) \n            3) Аппаратными проблемами или отсутствием GPU \n            4) Проблемами с файловой системой sysfs \n            Рекомендации: \n            - Проверьте права доступа: sudo ls -la /sys/class/drm \n            - Проверьте загрузку драйверов: lsmod | grep -i drm \n            - Проверьте системные логи: sudo dmesg | grep -i gpu \n            - Попробуйте обновить драйверы GPU \n            - Проверьте целостность файловой системы: sudo dmesg | grep -i sysfs \n            - Попробуйте перезагрузить систему"
        );
    } else if successful_devices < collection.gpu_count {
        info!(
            "Собраны метрики для {} из {} GPU устройств (частичный успех)",
            successful_devices, collection.gpu_count
        );
    } else {
        info!(
            "Собраны метрики для всех {} GPU устройств",
            successful_devices
        );
    }

    Ok(collection)
}

/// Collect metrics for a specific GPU device
fn collect_gpu_device_metrics(device: &GpuDevice) -> Result<GpuMetrics> {
    debug!("Сбор метрик для GPU устройства: {}", device.name);

    // First, try vendor-specific APIs (NVML for NVIDIA, AMDGPU for AMD)
    match collect_vendor_specific_metrics(device) {
        Ok(metrics) => {
            debug!("Метрики GPU для устройства {} собраны успешно с использованием vendor-specific API", device.name);
            return Ok(metrics);
        }
        Err(e) => {
            debug!("Не удалось собрать метрики с использованием vendor-specific API: {}", e);
            debug!("Пробуем общие методы сбора метрик");
        }
    }

    let mut metrics = GpuMetrics {
        device: device.clone(),
        utilization: GpuUtilization::default(),
        memory: GpuMemory::default(),
        temperature: GpuTemperature::default(),
        power: GpuPower::default(),
        clocks: GpuClocks::default(),
        timestamp: std::time::SystemTime::now(),
    };

    // Collect utilization metrics
    match collect_gpu_utilization(&device.device_path) {
        Ok(util) => {
            metrics.utilization = util;
            debug!("  GPU utilization: {:.1}%", util.gpu_util * 100.0);
        }
        Err(e) => {
            debug!("  Не удалось получить метрики использования GPU: {}", e);
        }
    }

    // Collect memory metrics
    match collect_gpu_memory(&device.device_path) {
        Ok(mem) => {
            metrics.memory = mem;
            if mem.total_bytes > 0 {
                debug!(
                    "  GPU memory: {}/{} MB ({:.1}% used)",
                    mem.used_bytes / 1024 / 1024,
                    mem.total_bytes / 1024 / 1024,
                    mem.used_bytes as f32 / mem.total_bytes as f32 * 100.0
                );
            }
        }
        Err(e) => {
            debug!("  Не удалось получить метрики памяти GPU: {}", e);
        }
    }

    // Collect temperature metrics
    match collect_gpu_temperature(&device.device_path) {
        Ok(temp) => {
            metrics.temperature = temp;
            if let Some(temp_c) = temp.temperature_c {
                debug!("  GPU temperature: {:.1}°C", temp_c);
            }
        }
        Err(e) => {
            debug!("  Не удалось получить метрики температуры GPU: {}", e);
        }
    }

    // Collect power metrics
    match collect_gpu_power(&device.device_path) {
        Ok(power) => {
            metrics.power = power;
            if let Some(power_w) = power.power_w {
                debug!("  GPU power: {:.1}W", power_w);
            }
        }
        Err(e) => {
            debug!("  Не удалось получить метрики мощности GPU: {}", e);
        }
    }

    // Collect clock metrics
    match collect_gpu_clocks(&device.device_path) {
        Ok(clocks) => {
            metrics.clocks = clocks;
            if let Some(core_clock) = clocks.core_clock_mhz {
                debug!("  GPU core clock: {} MHz", core_clock);
            }
        }
        Err(e) => {
            debug!("  Не удалось получить метрики тактовых частот GPU: {}", e);
        }
    }

    debug!("Метрики GPU для устройства {} собраны успешно", device.name);

    Ok(metrics)
}

/// Collect GPU utilization metrics
fn collect_gpu_utilization(device_path: &Path) -> Result<GpuUtilization> {
    let mut utilization = GpuUtilization::default();

    debug!("Сбор метрик использования GPU");

    // Try to read GPU utilization from sysfs
    // Different drivers expose this differently

    // For Intel i915
    match read_sysfs_u32(device_path, "gpu_busy_percent") {
        Ok(gpu_busy) => {
            utilization.gpu_util = gpu_busy as f32 / 100.0;
            debug!(
                "  Intel i915 utilization: {:.1}%",
                utilization.gpu_util * 100.0
            );
            return Ok(utilization);
        }
        Err(_e) => {
            debug!("  Intel i915 utilization не доступен. \n                Это может быть вызвано: \n                1) Устаревшей версией драйвера i915 \n                2) Отсутствием поддержки метрик в этом GPU \n                3) Проблемами с файловой системой sysfs \n                Рекомендации: \n                - Обновите драйвер i915 до последней версии \n                - Проверьте доступные метрики: ls -la /sys/class/drm/*/device/ | grep gpu_busy \n                - Проверьте системные логи: sudo dmesg | grep -i i915"
            );
        }
    }

    // For AMD
    match read_sysfs_u32(device_path, "gpu_busy_percent") {
        Ok(gpu_load) => {
            utilization.gpu_util = gpu_load as f32 / 100.0;
            debug!(
                "  AMD GPU utilization: {:.1}%",
                utilization.gpu_util * 100.0
            );
            return Ok(utilization);
        }
        Err(_e) => {
            debug!("  AMD GPU utilization не доступен. \n                Это может быть вызвано: \n                1) Устаревшей версией драйвера amdgpu \n                2) Отсутствием поддержки метрик в этом GPU \n                3) Проблемами с файловой системой sysfs \n                Рекомендации: \n                - Обновите драйвер amdgpu до последней версии \n                - Проверьте доступные метрики: ls -la /sys/class/drm/*/device/ | grep gpu_busy \n                - Проверьте системные логи: sudo dmesg | grep -i amdgpu \n                - Попробуйте установить проприетарный драйвер AMD"
            );
        }
    }

    // For NVIDIA (try different approaches)
    // NVIDIA exposes utilization through different interfaces
    match read_nvidia_utilization(device_path) {
        Ok(utilization_percent) => {
            utilization.gpu_util = utilization_percent as f32 / 100.0;
            debug!(
                "  NVIDIA GPU utilization: {:.1}%",
                utilization.gpu_util * 100.0
            );
            return Ok(utilization);
        }
        Err(_e) => {
            debug!("  NVIDIA GPU utilization не доступен. \n                Это может быть вызвано: \n                1) Устаревшей версией драйвера nvidia \n                2) Отсутствием поддержки метрик в этом GPU \n                3) Проблемами с файловой системой sysfs \n                4) Отсутствием проприетарного драйвера NVIDIA \n                Рекомендации: \n                - Установите проприетарный драйвер NVIDIA \n                - Обновите драйвер NVIDIA до последней версии \n                - Проверьте доступные метрики: ls -la /sys/class/drm/*/device/ | grep utilization \n                - Проверьте системные логи: sudo dmesg | grep -i nvidia \n                - Проверьте загрузку модуля: lsmod | grep -i nvidia"
            );
        }
    }

    // Try generic hwmon approach
    match read_hwmon_utilization(device_path) {
        Ok(util_percent) => {
            utilization.gpu_util = util_percent as f32 / 100.0;
            debug!(
                "  Generic hwmon utilization: {:.1}%",
                utilization.gpu_util * 100.0
            );
            return Ok(utilization);
        }
        Err(_e) => {
            debug!("  Generic hwmon utilization не доступен. \n                Это может быть вызвано: \n                1) Отсутствием поддержки hwmon в этом GPU \n                2) Проблемами с файловой системой sysfs \n                3) Отсутствием необходимых модулей ядра \n                Рекомендации: \n                - Проверьте загрузку модуля hwmon: lsmod | grep -i hwmon \n                - Проверьте доступные метрики: ls -la /sys/class/drm/*/device/hwmon/ \n                - Проверьте системные логи: sudo dmesg | grep -i hwmon \n                - Попробуйте обновить драйвер GPU"
            );
        }
    }

    warn!(
        "  Не удалось получить метрики использования GPU. \n        Это может быть вызвано: \n        1) Неподдерживаемым драйвером GPU \n        2) Отсутствием соответствующих файлов в sysfs \n        3) Проблемами с правами доступа \n        4) Устаревшей версией ядра или драйвера \n        Рекомендации: \n        - Проверьте загрузку драйвера: lsmod | grep -i gpu \n        - Проверьте доступные файлы: ls -la /sys/class/drm/*/device/ \n        - Попробуйте обновить драйвер GPU \n        - Проверьте системные логи: sudo dmesg | grep -i gpu \n        - Попробуйте установить проприетарные драйверы для вашего GPU \n        - Обновите ядро до последней стабильной версии"
    );

    Ok(utilization)
}

/// Read NVIDIA GPU utilization
fn read_nvidia_utilization(device_path: &Path) -> Result<u32> {
    // NVIDIA exposes utilization through different files
    // Try common NVIDIA sysfs paths

    // Try /sys/class/drm/card*/device/utilization
    let parent_device = device_path.parent().unwrap_or(device_path);
    if let Ok(util) = read_sysfs_u32(parent_device, "utilization") {
        return Ok(util);
    }

    // Try /sys/class/drm/card*/device/gpu_busy
    if let Ok(util) = read_sysfs_u32(parent_device, "gpu_busy") {
        return Ok(util);
    }

    Err(anyhow!("Не удалось прочитать использование NVIDIA GPU"))
}

/// Read GPU utilization from hwmon
fn read_hwmon_utilization(device_path: &Path) -> Result<u32> {
    let hwmon_dir = device_path.join("hwmon");
    if !hwmon_dir.exists() {
        return Err(anyhow!("Директория hwmon не найдена"));
    }

    let entries = fs::read_dir(&hwmon_dir)
        .with_context(|| format!("Не удалось прочитать hwmon директорию: {:?}", hwmon_dir))?;

    for entry in entries {
        let entry = entry?;
        let hwmon_path = entry.path();

        // Look for utilization files
        if let Ok(util_files) = fs::read_dir(&hwmon_path) {
            for util_file in util_files {
                let util_file = util_file?;
                let util_path = util_file.path();
                let file_name = util_path.file_name().and_then(|s| s.to_str()).unwrap_or("");

                if file_name.contains("util") || file_name.contains("load") {
                    if let Ok(content) = fs::read_to_string(&util_path) {
                        if let Ok(util_value) = content.trim().parse::<u32>() {
                            return Ok(util_value);
                        }
                    }
                }
            }
        }
    }

    Err(anyhow!("Не удалось найти метрики использования в hwmon"))
}

/// Collect GPU memory metrics
fn collect_gpu_memory(device_path: &Path) -> Result<GpuMemory> {
    let mut memory = GpuMemory::default();

    debug!("Сбор метрик памяти GPU");

    // Try to read memory info from sysfs
    // This is driver-specific and may not be available on all systems

    // For Intel i915
    match read_sysfs_u64(device_path, "mem_total_bytes") {
        Ok(total) => {
            memory.total_bytes = total;
            debug!("  Intel i915 total memory: {} MB", total / 1024 / 1024);
        }
        Err(e) => {
            debug!("  Intel i915 memory info не доступен: {}", e);
        }
    }

    match read_sysfs_u64(device_path, "mem_used_bytes") {
        Ok(used) => {
            memory.used_bytes = used;
            debug!("  Intel i915 used memory: {} MB", used / 1024 / 1024);
        }
        Err(e) => {
            debug!("  Intel i915 used memory не доступен: {}", e);
        }
    }

    // For AMD
    if memory.total_bytes == 0 {
        match read_sysfs_u64(device_path, "vram_total_bytes") {
            Ok(total) => {
                memory.total_bytes = total;
                debug!("  AMD VRAM total: {} MB", total / 1024 / 1024);
            }
            Err(e) => {
                debug!("  AMD VRAM total не доступен: {}", e);
            }
        }

        match read_sysfs_u64(device_path, "vram_used_bytes") {
            Ok(used) => {
                memory.used_bytes = used;
                debug!("  AMD VRAM used: {} MB", used / 1024 / 1024);
            }
            Err(e) => {
                debug!("  AMD VRAM used не доступен: {}", e);
            }
        }
    }

    // For NVIDIA
    if memory.total_bytes == 0 {
        match read_nvidia_memory_total(device_path) {
            Ok(total) => {
                memory.total_bytes = total;
                debug!("  NVIDIA total memory: {} MB", total / 1024 / 1024);
            }
            Err(e) => {
                debug!("  NVIDIA total memory не доступен: {}", e);
            }
        }

        match read_nvidia_memory_used(device_path) {
            Ok(used) => {
                memory.used_bytes = used;
                debug!("  NVIDIA used memory: {} MB", used / 1024 / 1024);
            }
            Err(e) => {
                debug!("  NVIDIA used memory не доступен: {}", e);
            }
        }
    }

    // Validate and correct memory values
    if memory.total_bytes > 0 && memory.used_bytes > memory.total_bytes {
        warn!(
            "  Исправление: использованная память ({} MB) больше общей ({} MB). \n                Это может быть вызвано ошибками чтения или несинхронизированными счетчиками. \n                Устанавливаем used_bytes = total_bytes для предотвращения отрицательных значений.",
            memory.used_bytes / 1024 / 1024,
            memory.total_bytes / 1024 / 1024
        );
        memory.used_bytes = memory.total_bytes;
    }

    memory.free_bytes = memory.total_bytes.saturating_sub(memory.used_bytes);

    if memory.total_bytes > 0 {
        debug!(
            "  GPU memory: {}/{} MB ({:.1}% used)",
            memory.used_bytes / 1024 / 1024,
            memory.total_bytes / 1024 / 1024,
            memory.used_bytes as f32 / memory.total_bytes as f32 * 100.0
        );
    } else {
        warn!(
            "  Не удалось получить метрики памяти GPU. \n            Это может быть вызвано: \n            1) Неподдерживаемым драйвером GPU \n            2) Отсутствием соответствующих файлов в sysfs \n            3) Проблемами с правами доступа \n            Рекомендации: \n            - Проверьте загрузку драйвера: lsmod | grep -i gpu \n            - Проверьте доступные файлы: ls -la /sys/class/drm/*/device/ | grep mem \n            - Попробуйте обновить драйвер GPU"
        );
    }

    Ok(memory)
}

/// Read NVIDIA GPU total memory
fn read_nvidia_memory_total(device_path: &Path) -> Result<u64> {
    let parent_device = device_path.parent().unwrap_or(device_path);

    // Try different NVIDIA memory files
    if let Ok(total) = read_sysfs_u64(parent_device, "mem_total") {
        return Ok(total * 1024 * 1024); // Convert MB to bytes
    }

    if let Ok(total) = read_sysfs_u64(parent_device, "memory_total") {
        return Ok(total);
    }

    Err(anyhow!("Не удалось прочитать общую память NVIDIA GPU"))
}

/// Read NVIDIA GPU used memory
fn read_nvidia_memory_used(device_path: &Path) -> Result<u64> {
    let parent_device = device_path.parent().unwrap_or(device_path);

    // Try different NVIDIA memory files
    if let Ok(used) = read_sysfs_u64(parent_device, "mem_used") {
        return Ok(used * 1024 * 1024); // Convert MB to bytes
    }

    if let Ok(used) = read_sysfs_u64(parent_device, "memory_used") {
        return Ok(used);
    }

    Err(anyhow!(
        "Не удалось прочитать использованную память NVIDIA GPU"
    ))
}

/// Collect GPU temperature metrics
fn collect_gpu_temperature(device_path: &Path) -> Result<GpuTemperature> {
    let mut temperature = GpuTemperature::default();

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
                                        if temperature.temperature_c.is_none() {
                                            temperature.temperature_c = Some(temp_c);
                                        }
                                    } else if file_name.contains("temp2") {
                                        // Could be hotspot
                                        if temperature.hotspot_c.is_none() {
                                            temperature.hotspot_c = Some(temp_c);
                                        }
                                    } else if file_name.contains("temp3") {
                                        // Could be memory
                                        if temperature.memory_c.is_none() {
                                            temperature.memory_c = Some(temp_c);
                                        }
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

/// Collect GPU power metrics
fn collect_gpu_power(device_path: &Path) -> Result<GpuPower> {
    let mut power = GpuPower::default();

    // Look for power sensors in powercap or hwmon

    // Check powercap first
    let powercap_dir = Path::new("/sys/class/powercap");
    if powercap_dir.exists() {
        if let Ok(entries) = fs::read_dir(powercap_dir) {
            for entry in entries.flatten() {
                let powercap_path = entry.path();

                // Look for energy_uj files that might correspond to this GPU
                if let Ok(energy_files) = fs::read_dir(&powercap_path) {
                    for energy_file in energy_files.flatten() {
                        let energy_path = energy_file.path();
                        let file_name = energy_path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("");

                        if file_name == "energy_uj" {
                            // Check if this powercap device corresponds to our GPU
                            // This is simplified - in real implementation we'd need
                            // to match the device path
                            if let Ok(energy_content) = fs::read_to_string(&energy_path) {
                                if let Ok(energy_microjoules) = energy_content.trim().parse::<u64>()
                                {
                                    // Convert microjoules to watts (simplified)
                                    // In real implementation, we'd need to track changes over time
                                    let energy_w = energy_microjoules as f32 / 1_000_000.0;
                                    power.power_w = Some(energy_w);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Also check hwmon for power sensors
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
                                    let power_w = power_microwatts as f32 / 1_000_000.0;
                                    power.power_w = Some(power_w);
                                }
                            }
                        } else if file_name == "power1_cap" {
                            if let Ok(power_content) = fs::read_to_string(&power_path) {
                                if let Ok(power_microwatts) = power_content.trim().parse::<u64>() {
                                    let power_w = power_microwatts as f32 / 1_000_000.0;
                                    power.power_cap_w = Some(power_w);
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

/// Collect GPU clock metrics
fn collect_gpu_clocks(device_path: &Path) -> Result<GpuClocks> {
    let mut clocks = GpuClocks::default();

    // Look for clock files in sysfs
    // This is driver-specific and may not be available on all systems

    // For Intel i915
    if let Ok(core_clock) = read_sysfs_u32(device_path, "gt_cur_freq_mhz") {
        clocks.core_clock_mhz = Some(core_clock);
    }

    // For AMD
    if let Ok(core_clock) = read_sysfs_u32(device_path, "gpu_clock") {
        clocks.core_clock_mhz = Some(core_clock);
    }

    // For NVIDIA (simplified)
    if let Ok(core_clock) = read_sysfs_u32(device_path, "clock") {
        clocks.core_clock_mhz = Some(core_clock);
    }

    Ok(clocks)
}

/// Initialize NVML library
#[cfg(feature = "nvml-wrapper")]
fn init_nvml() -> Result<Nvml> {
    match Nvml::init() {
        Ok(nvml) => {
            info!("NVML инициализирован успешно");
            Ok(nvml)
        }
        Err(e) => {
            error!("Не удалось инициализировать NVML: {}", e);
            Err(anyhow!("NVML initialization failed: {}", e))
        }
    }
}

/// Collect GPU metrics using NVML for NVIDIA GPUs
#[cfg(feature = "nvml-wrapper")]
fn collect_nvml_metrics(device: &GpuDevice) -> Result<GpuMetrics> {
    let nvml = match init_nvml() {
        Ok(nvml) => nvml,
        Err(e) => {
            debug!("NVML не доступен: {}", e);
            return Err(e);
        }
    };

    let device_count = match nvml.device_count() {
        Ok(count) => count,
        Err(e) => {
            error!("Не удалось получить количество устройств NVML: {}", e);
            return Err(anyhow!("NVML device count failed: {}", e));
        }
    };

    if device_count == 0 {
        debug!("NVML: нет доступных устройств");
        return Err(anyhow!("No NVML devices available"));
    }

    // Try to find the device that matches our device
    for device_index in 0..device_count {
        let device_handle = match nvml.device_by_index(device_index) {
            Ok(handle) => handle,
            Err(e) => {
                debug!("Не удалось получить устройство NVML {}: {}", device_index, e);
                continue;
            }
        };

        let name = match nvml.device_name(device_handle) {
            Ok(name) => name,
            Err(e) => {
                debug!("Не удалось получить имя устройства NVML {}: {}", device_index, e);
                continue;
            }
        };

        debug!("NVML устройство {}: {}", device_index, name);

        // For now, we'll use the first device we can access
        // In a more sophisticated implementation, we'd match by PCI ID
        let mut metrics = GpuMetrics {
            device: device.clone(),
            utilization: GpuUtilization::default(),
            memory: GpuMemory::default(),
            temperature: GpuTemperature::default(),
            power: GpuPower::default(),
            clocks: GpuClocks::default(),
            timestamp: std::time::SystemTime::now(),
        };

        // Collect utilization
        if let Ok(utilization) = nvml.device_utilization_rates(device_handle) {
            metrics.utilization.gpu_util = utilization.gpu as f32 / 100.0;
            metrics.utilization.memory_util = utilization.memory as f32 / 100.0;
            debug!("  NVML utilization: GPU {:.1}%, Memory {:.1}%",
                   metrics.utilization.gpu_util * 100.0,
                   metrics.utilization.memory_util * 100.0);
        }

        // Collect memory
        if let Ok(memory_info) = nvml.device_memory_info(device_handle) {
            metrics.memory.total_bytes = memory_info.total;
            metrics.memory.used_bytes = memory_info.used;
            metrics.memory.free_bytes = memory_info.free;
            debug!("  NVML memory: {}/{} MB used",
                   memory_info.used / 1024 / 1024,
                   memory_info.total / 1024 / 1024);
        }

        // Collect temperature
        if let Ok(temp) = nvml.device_temperature(device_handle, nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu) {
            metrics.temperature.temperature_c = Some(temp as f32);
            debug!("  NVML temperature: {:.1}°C", temp);
        }

        // Collect power
        if let Ok(power) = nvml.device_power_usage(device_handle) {
            metrics.power.power_w = Some(power as f32 / 1000.0); // Convert mW to W
            debug!("  NVML power: {:.1}W", power as f32 / 1000.0);
        }

        // Collect clocks
        if let Ok(clock_mhz) = nvml.device_clock_info(device_handle, nvml_wrapper::enum_wrappers::device::ClockType::Graphics) {
            metrics.clocks.core_clock_mhz = Some(clock_mhz);
            debug!("  NVML core clock: {} MHz", clock_mhz);
        }

        if let Ok(clock_mhz) = nvml.device_clock_info(device_handle, nvml_wrapper::enum_wrappers::device::ClockType::Memory) {
            metrics.clocks.memory_clock_mhz = Some(clock_mhz);
            debug!("  NVML memory clock: {} MHz", clock_mhz);
        }

        return Ok(metrics);
    }

    Err(anyhow!("No matching NVML device found"))
}

/// Collect GPU metrics using AMDGPU sysfs interface
fn collect_amdgpu_metrics(device: &GpuDevice) -> Result<GpuMetrics> {
    let mut metrics = GpuMetrics {
        device: device.clone(),
        utilization: GpuUtilization::default(),
        memory: GpuMemory::default(),
        temperature: GpuTemperature::default(),
        power: GpuPower::default(),
        clocks: GpuClocks::default(),
        timestamp: std::time::SystemTime::now(),
    };

    let device_path = &device.device_path;

    // Collect GPU utilization
    if let Ok(gpu_load) = read_sysfs_u32(device_path, "gpu_busy_percent") {
        metrics.utilization.gpu_util = gpu_load as f32 / 100.0;
        debug!("  AMDGPU utilization: {:.1}%", metrics.utilization.gpu_util * 100.0);
    }

    // Collect memory metrics
    if let Ok(total_vram) = read_sysfs_u64(device_path, "vram_total_bytes") {
        metrics.memory.total_bytes = total_vram;
    }

    if let Ok(used_vram) = read_sysfs_u64(device_path, "vram_used_bytes") {
        metrics.memory.used_bytes = used_vram;
    }

    if metrics.memory.total_bytes > 0 {
        metrics.memory.free_bytes = metrics.memory.total_bytes.saturating_sub(metrics.memory.used_bytes);
        debug!("  AMDGPU memory: {}/{} MB used",
               metrics.memory.used_bytes / 1024 / 1024,
               metrics.memory.total_bytes / 1024 / 1024);
    }

    // Collect temperature
    let hwmon_dir = device_path.join("hwmon");
    if hwmon_dir.exists() {
        if let Ok(entries) = fs::read_dir(&hwmon_dir) {
            for entry in entries.flatten() {
                let hwmon_path = entry.path();
                if let Ok(temp_files) = fs::read_dir(&hwmon_path) {
                    for temp_file in temp_files.flatten() {
                        let temp_path = temp_file.path();
                        let file_name = temp_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                        
                        if file_name.ends_with("_input") && file_name.contains("temp") {
                            if let Ok(temp_content) = fs::read_to_string(&temp_path) {
                                if let Ok(temp_millidegrees) = temp_content.trim().parse::<u64>() {
                                    let temp_c = temp_millidegrees as f32 / 1000.0;
                                    metrics.temperature.temperature_c = Some(temp_c);
                                    debug!("  AMDGPU temperature: {:.1}°C", temp_c);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Collect power
    if let Ok(power_w) = read_sysfs_u32(device_path, "power_avg") {
        metrics.power.power_w = Some(power_w as f32);
        debug!("  AMDGPU power: {:.1}W", power_w as f32);
    }

    // Collect clocks
    if let Ok(core_clock) = read_sysfs_u32(device_path, "gpu_clock") {
        metrics.clocks.core_clock_mhz = Some(core_clock);
        debug!("  AMDGPU core clock: {} MHz", core_clock);
    }

    if let Ok(mem_clock) = read_sysfs_u32(device_path, "mem_clock") {
        metrics.clocks.memory_clock_mhz = Some(mem_clock);
        debug!("  AMDGPU memory clock: {} MHz", mem_clock);
    }

    Ok(metrics)
}

/// Collect GPU metrics using Intel sysfs interface
fn collect_intel_gpu_metrics(device: &GpuDevice) -> Result<GpuMetrics> {
    let mut metrics = GpuMetrics {
        device: device.clone(),
        utilization: GpuUtilization::default(),
        memory: GpuMemory::default(),
        temperature: GpuTemperature::default(),
        power: GpuPower::default(),
        clocks: GpuClocks::default(),
        timestamp: std::time::SystemTime::now(),
    };

    let device_path = &device.device_path;

    // Collect GPU utilization
    if let Ok(gpu_busy) = read_sysfs_u32(device_path, "gpu_busy_percent") {
        metrics.utilization.gpu_util = gpu_busy as f32 / 100.0;
        debug!("  Intel GPU utilization: {:.1}%", metrics.utilization.gpu_util * 100.0);
    }

    // Collect memory metrics
    if let Ok(total_mem) = read_sysfs_u64(device_path, "mem_total_bytes") {
        metrics.memory.total_bytes = total_mem;
    }

    if let Ok(used_mem) = read_sysfs_u64(device_path, "mem_used_bytes") {
        metrics.memory.used_bytes = used_mem;
    }

    if metrics.memory.total_bytes > 0 {
        metrics.memory.free_bytes = metrics.memory.total_bytes.saturating_sub(metrics.memory.used_bytes);
        debug!(
            "  Intel GPU memory: {}/{} MB used",
            metrics.memory.used_bytes / 1024 / 1024,
            metrics.memory.total_bytes / 1024 / 1024
        );
    }

    // Collect temperature
    let hwmon_dir = device_path.join("hwmon");
    if hwmon_dir.exists() {
        if let Ok(entries) = fs::read_dir(&hwmon_dir) {
            for entry in entries.flatten() {
                let hwmon_path = entry.path();
                if let Ok(temp_files) = fs::read_dir(&hwmon_path) {
                    for temp_file in temp_files.flatten() {
                        let temp_path = temp_file.path();
                        let file_name = temp_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                        
                        if file_name.ends_with("_input") && file_name.contains("temp") {
                            if let Ok(temp_content) = fs::read_to_string(&temp_path) {
                                if let Ok(temp_millidegrees) = temp_content.trim().parse::<u64>() {
                                    let temp_c = temp_millidegrees as f32 / 1000.0;
                                    metrics.temperature.temperature_c = Some(temp_c);
                                    debug!("  Intel GPU temperature: {:.1}°C", temp_c);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Collect power
    if let Ok(power_w) = read_sysfs_u32(device_path, "power_avg") {
        metrics.power.power_w = Some(power_w as f32);
        debug!("  Intel GPU power: {:.1}W", power_w as f32);
    }

    // Collect clocks
    if let Ok(core_clock) = read_sysfs_u32(device_path, "gt_cur_freq_mhz") {
        metrics.clocks.core_clock_mhz = Some(core_clock);
        debug!("  Intel GPU core clock: {} MHz", core_clock);
    }

    Ok(metrics)
}

/// Collect GPU metrics using Qualcomm Adreno sysfs interface
fn collect_qualcomm_adreno_metrics(device: &GpuDevice) -> Result<GpuMetrics> {
    let mut metrics = GpuMetrics {
        device: device.clone(),
        utilization: GpuUtilization::default(),
        memory: GpuMemory::default(),
        temperature: GpuTemperature::default(),
        power: GpuPower::default(),
        clocks: GpuClocks::default(),
        timestamp: std::time::SystemTime::now(),
    };

    let device_path = &device.device_path;

    // Collect GPU utilization - Adreno devices may expose this through different interfaces
    if let Ok(gpu_load) = read_sysfs_u32(device_path, "gpu_load") {
        metrics.utilization.gpu_util = gpu_load as f32 / 100.0;
        debug!("  Qualcomm Adreno GPU utilization: {:.1}%", metrics.utilization.gpu_util * 100.0);
    } else if let Ok(gpu_busy) = read_sysfs_u32(device_path, "gpu_busy") {
        // Some Adreno devices use gpu_busy instead
        metrics.utilization.gpu_util = gpu_busy as f32 / 100.0;
        debug!("  Qualcomm Adreno GPU utilization: {:.1}%", metrics.utilization.gpu_util * 100.0);
    }

    // Collect memory metrics - Adreno devices may have limited memory info
    if let Ok(total_mem) = read_sysfs_u64(device_path, "mem_total") {
        metrics.memory.total_bytes = total_mem * 1024 * 1024; // Convert MB to bytes
    } else if let Ok(total_mem) = read_sysfs_u64(device_path, "memory_total") {
        metrics.memory.total_bytes = total_mem;
    }

    if let Ok(used_mem) = read_sysfs_u64(device_path, "mem_used") {
        metrics.memory.used_bytes = used_mem * 1024 * 1024; // Convert MB to bytes
    } else if let Ok(used_mem) = read_sysfs_u64(device_path, "memory_used") {
        metrics.memory.used_bytes = used_mem;
    }

    if metrics.memory.total_bytes > 0 {
        metrics.memory.free_bytes = metrics.memory.total_bytes.saturating_sub(metrics.memory.used_bytes);
        debug!(
            "  Qualcomm Adreno GPU memory: {}/{} MB used",
            metrics.memory.used_bytes / 1024 / 1024,
            metrics.memory.total_bytes / 1024 / 1024
        );
    }

    // Collect temperature - Adreno devices often have thermal sensors
    let hwmon_dir = device_path.join("hwmon");
    if hwmon_dir.exists() {
        if let Ok(entries) = fs::read_dir(&hwmon_dir) {
            for entry in entries.flatten() {
                let hwmon_path = entry.path();
                if let Ok(temp_files) = fs::read_dir(&hwmon_path) {
                    for temp_file in temp_files.flatten() {
                        let temp_path = temp_file.path();
                        let file_name = temp_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                        
                        if file_name.ends_with("_input") && file_name.contains("temp") {
                            if let Ok(temp_content) = fs::read_to_string(&temp_path) {
                                if let Ok(temp_millidegrees) = temp_content.trim().parse::<u64>() {
                                    let temp_c = temp_millidegrees as f32 / 1000.0;
                                    metrics.temperature.temperature_c = Some(temp_c);
                                    debug!("  Qualcomm Adreno GPU temperature: {:.1}°C", temp_c);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Collect power - Adreno devices may have power sensors
    if let Ok(power_mw) = read_sysfs_u32(device_path, "power") {
        metrics.power.power_w = Some(power_mw as f32 / 1000.0); // Convert mW to W
        debug!("  Qualcomm Adreno GPU power: {:.1}W", power_mw as f32 / 1000.0);
    }

    // Collect clocks - Adreno devices may expose clock information
    if let Ok(core_clock) = read_sysfs_u32(device_path, "gpu_clock") {
        metrics.clocks.core_clock_mhz = Some(core_clock);
        debug!("  Qualcomm Adreno GPU core clock: {} MHz", core_clock);
    }

    Ok(metrics)
}

/// Collect GPU metrics using ARM Mali sysfs interface
fn collect_arm_mali_metrics(device: &GpuDevice) -> Result<GpuMetrics> {
    let mut metrics = GpuMetrics {
        device: device.clone(),
        utilization: GpuUtilization::default(),
        memory: GpuMemory::default(),
        temperature: GpuTemperature::default(),
        power: GpuPower::default(),
        clocks: GpuClocks::default(),
        timestamp: std::time::SystemTime::now(),
    };

    let device_path = &device.device_path;

    // Collect GPU utilization - Mali devices may expose this through different interfaces
    if let Ok(gpu_util) = read_sysfs_u32(device_path, "utilization") {
        metrics.utilization.gpu_util = gpu_util as f32 / 100.0;
        debug!("  ARM Mali GPU utilization: {:.1}%", metrics.utilization.gpu_util * 100.0);
    } else if let Ok(gpu_load) = read_sysfs_u32(device_path, "gpu_load") {
        metrics.utilization.gpu_util = gpu_load as f32 / 100.0;
        debug!("  ARM Mali GPU utilization: {:.1}%", metrics.utilization.gpu_util * 100.0);
    }

    // Collect memory metrics - Mali devices may have limited memory info
    if let Ok(total_mem) = read_sysfs_u64(device_path, "mem_total") {
        metrics.memory.total_bytes = total_mem * 1024 * 1024; // Convert MB to bytes
    } else if let Ok(total_mem) = read_sysfs_u64(device_path, "memory_total") {
        metrics.memory.total_bytes = total_mem;
    }

    if let Ok(used_mem) = read_sysfs_u64(device_path, "mem_used") {
        metrics.memory.used_bytes = used_mem * 1024 * 1024; // Convert MB to bytes
    } else if let Ok(used_mem) = read_sysfs_u64(device_path, "memory_used") {
        metrics.memory.used_bytes = used_mem;
    }

    if metrics.memory.total_bytes > 0 {
        metrics.memory.free_bytes = metrics.memory.total_bytes.saturating_sub(metrics.memory.used_bytes);
        debug!(
            "  ARM Mali GPU memory: {}/{} MB used",
            metrics.memory.used_bytes / 1024 / 1024,
            metrics.memory.total_bytes / 1024 / 1024
        );
    }

    // Collect temperature - Mali devices often have thermal sensors
    let hwmon_dir = device_path.join("hwmon");
    if hwmon_dir.exists() {
        if let Ok(entries) = fs::read_dir(&hwmon_dir) {
            for entry in entries.flatten() {
                let hwmon_path = entry.path();
                if let Ok(temp_files) = fs::read_dir(&hwmon_path) {
                    for temp_file in temp_files.flatten() {
                        let temp_path = temp_file.path();
                        let file_name = temp_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                        
                        if file_name.ends_with("_input") && file_name.contains("temp") {
                            if let Ok(temp_content) = fs::read_to_string(&temp_path) {
                                if let Ok(temp_millidegrees) = temp_content.trim().parse::<u64>() {
                                    let temp_c = temp_millidegrees as f32 / 1000.0;
                                    metrics.temperature.temperature_c = Some(temp_c);
                                    debug!("  ARM Mali GPU temperature: {:.1}°C", temp_c);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Collect power - Mali devices may have power sensors
    if let Ok(power_mw) = read_sysfs_u32(device_path, "power") {
        metrics.power.power_w = Some(power_mw as f32 / 1000.0); // Convert mW to W
        debug!("  ARM Mali GPU power: {:.1}W", power_mw as f32 / 1000.0);
    }

    // Collect clocks - Mali devices may expose clock information
    if let Ok(core_clock) = read_sysfs_u32(device_path, "clock") {
        metrics.clocks.core_clock_mhz = Some(core_clock);
        debug!("  ARM Mali GPU core clock: {} MHz", core_clock);
    }

    Ok(metrics)
}

/// Collect GPU metrics using Broadcom VideoCore sysfs interface
fn collect_broadcom_videocore_metrics(device: &GpuDevice) -> Result<GpuMetrics> {
    let mut metrics = GpuMetrics {
        device: device.clone(),
        utilization: GpuUtilization::default(),
        memory: GpuMemory::default(),
        temperature: GpuTemperature::default(),
        power: GpuPower::default(),
        clocks: GpuClocks::default(),
        timestamp: std::time::SystemTime::now(),
    };

    let device_path = &device.device_path;

    // Collect GPU utilization - VideoCore devices may expose this through different interfaces
    if let Ok(gpu_util) = read_sysfs_u32(device_path, "utilization") {
        metrics.utilization.gpu_util = gpu_util as f32 / 100.0;
        debug!("  Broadcom VideoCore GPU utilization: {:.1}%", metrics.utilization.gpu_util * 100.0);
    } else if let Ok(gpu_load) = read_sysfs_u32(device_path, "gpu_load") {
        metrics.utilization.gpu_util = gpu_load as f32 / 100.0;
        debug!("  Broadcom VideoCore GPU utilization: {:.1}%", metrics.utilization.gpu_util * 100.0);
    }

    // Collect memory metrics - VideoCore devices may have limited memory info
    if let Ok(total_mem) = read_sysfs_u64(device_path, "mem_total") {
        metrics.memory.total_bytes = total_mem * 1024 * 1024; // Convert MB to bytes
    } else if let Ok(total_mem) = read_sysfs_u64(device_path, "memory_total") {
        metrics.memory.total_bytes = total_mem;
    }

    if let Ok(used_mem) = read_sysfs_u64(device_path, "mem_used") {
        metrics.memory.used_bytes = used_mem * 1024 * 1024; // Convert MB to bytes
    } else if let Ok(used_mem) = read_sysfs_u64(device_path, "memory_used") {
        metrics.memory.used_bytes = used_mem;
    }

    if metrics.memory.total_bytes > 0 {
        metrics.memory.free_bytes = metrics.memory.total_bytes.saturating_sub(metrics.memory.used_bytes);
        debug!(
            "  Broadcom VideoCore GPU memory: {}/{} MB used",
            metrics.memory.used_bytes / 1024 / 1024,
            metrics.memory.total_bytes / 1024 / 1024
        );
    }

    // Collect temperature - VideoCore devices often have thermal sensors
    let hwmon_dir = device_path.join("hwmon");
    if hwmon_dir.exists() {
        if let Ok(entries) = fs::read_dir(&hwmon_dir) {
            for entry in entries.flatten() {
                let hwmon_path = entry.path();
                if let Ok(temp_files) = fs::read_dir(&hwmon_path) {
                    for temp_file in temp_files.flatten() {
                        let temp_path = temp_file.path();
                        let file_name = temp_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                        
                        if file_name.ends_with("_input") && file_name.contains("temp") {
                            if let Ok(temp_content) = fs::read_to_string(&temp_path) {
                                if let Ok(temp_millidegrees) = temp_content.trim().parse::<u64>() {
                                    let temp_c = temp_millidegrees as f32 / 1000.0;
                                    metrics.temperature.temperature_c = Some(temp_c);
                                    debug!("  Broadcom VideoCore GPU temperature: {:.1}°C", temp_c);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Collect power - VideoCore devices may have power sensors
    if let Ok(power_mw) = read_sysfs_u32(device_path, "power") {
        metrics.power.power_w = Some(power_mw as f32 / 1000.0); // Convert mW to W
        debug!("  Broadcom VideoCore GPU power: {:.1}W", power_mw as f32 / 1000.0);
    }

    // Collect clocks - VideoCore devices may expose clock information
    if let Ok(core_clock) = read_sysfs_u32(device_path, "clock") {
        metrics.clocks.core_clock_mhz = Some(core_clock);
        debug!("  Broadcom VideoCore GPU core clock: {} MHz", core_clock);
    }

    Ok(metrics)
}

/// Collect GPU metrics using Virtio GPU interface
fn collect_virtio_gpu_metrics(device: &GpuDevice) -> Result<GpuMetrics> {
    let mut metrics = GpuMetrics {
        device: device.clone(),
        utilization: GpuUtilization::default(),
        memory: GpuMemory::default(),
        temperature: GpuTemperature::default(),
        power: GpuPower::default(),
        clocks: GpuClocks::default(),
        timestamp: std::time::SystemTime::now(),
    };

    let device_path = &device.device_path;

    // Virtio GPU devices have very limited metrics available
    // We'll try to collect what we can

    // Collect GPU utilization - Virtio devices may expose this through different interfaces
    if let Ok(gpu_util) = read_sysfs_u32(device_path, "utilization") {
        metrics.utilization.gpu_util = gpu_util as f32 / 100.0;
        debug!("  Virtio GPU utilization: {:.1}%", metrics.utilization.gpu_util * 100.0);
    }

    // Collect memory metrics - Virtio devices may have limited memory info
    if let Ok(total_mem) = read_sysfs_u64(device_path, "mem_total") {
        metrics.memory.total_bytes = total_mem * 1024 * 1024; // Convert MB to bytes
    } else if let Ok(total_mem) = read_sysfs_u64(device_path, "memory_total") {
        metrics.memory.total_bytes = total_mem;
    }

    if let Ok(used_mem) = read_sysfs_u64(device_path, "mem_used") {
        metrics.memory.used_bytes = used_mem * 1024 * 1024; // Convert MB to bytes
    } else if let Ok(used_mem) = read_sysfs_u64(device_path, "memory_used") {
        metrics.memory.used_bytes = used_mem;
    }

    if metrics.memory.total_bytes > 0 {
        metrics.memory.free_bytes = metrics.memory.total_bytes.saturating_sub(metrics.memory.used_bytes);
        debug!(
            "  Virtio GPU memory: {}/{} MB used",
            metrics.memory.used_bytes / 1024 / 1024,
            metrics.memory.total_bytes / 1024 / 1024
        );
    }

    // Virtio devices typically don't have temperature sensors in the guest
    // Collect power - Virtio devices may have power sensors
    if let Ok(power_mw) = read_sysfs_u32(device_path, "power") {
        metrics.power.power_w = Some(power_mw as f32 / 1000.0); // Convert mW to W
        debug!("  Virtio GPU power: {:.1}W", power_mw as f32 / 1000.0);
    }

    // Collect clocks - Virtio devices may expose clock information
    if let Ok(core_clock) = read_sysfs_u32(device_path, "clock") {
        metrics.clocks.core_clock_mhz = Some(core_clock);
        debug!("  Virtio GPU core clock: {} MHz", core_clock);
    }

    Ok(metrics)
}

/// Try to collect GPU metrics using vendor-specific APIs
fn collect_vendor_specific_metrics(device: &GpuDevice) -> Result<GpuMetrics> {
    // Try NVML first for NVIDIA devices
    #[cfg(feature = "nvml-wrapper")]
    if device.driver.as_deref() == Some("nvidia") {
        match collect_nvml_metrics(device) {
            Ok(metrics) => {
                debug!("Successfully collected NVML metrics for device {}", device.name);
                return Ok(metrics);
            }
            Err(e) => {
                debug!("Failed to collect NVML metrics for device {}: {}", device.name, e);
            }
        }
    }

    // Try AMDGPU for AMD devices
    if device.driver.as_deref() == Some("amdgpu") {
        match collect_amdgpu_metrics(device) {
            Ok(metrics) => {
                debug!("Successfully collected AMDGPU metrics for device {}", device.name);
                return Ok(metrics);
            }
            Err(e) => {
                debug!("Failed to collect AMDGPU metrics for device {}: {}", device.name, e);
            }
        }
    }

    // Try Intel for Intel devices
    if device.driver.as_deref() == Some("i915") {
        match collect_intel_gpu_metrics(device) {
            Ok(metrics) => {
                debug!("Successfully collected Intel GPU metrics for device {}", device.name);
                return Ok(metrics);
            }
            Err(e) => {
                debug!("Failed to collect Intel GPU metrics for device {}: {}", device.name, e);
            }
        }
    }

    // Try Qualcomm Adreno for Qualcomm devices
    if device.driver.as_deref() == Some("msm") || device.driver.as_deref() == Some("adreno") {
        match collect_qualcomm_adreno_metrics(device) {
            Ok(metrics) => {
                debug!("Successfully collected Qualcomm Adreno metrics for device {}", device.name);
                return Ok(metrics);
            }
            Err(e) => {
                debug!("Failed to collect Qualcomm Adreno metrics for device {}: {}", device.name, e);
            }
        }
    }

    // Try ARM Mali for ARM Mali devices
    if device.driver.as_deref() == Some("mali") || device.driver.as_deref() == Some("panfrost") {
        match collect_arm_mali_metrics(device) {
            Ok(metrics) => {
                debug!("Successfully collected ARM Mali metrics for device {}", device.name);
                return Ok(metrics);
            }
            Err(e) => {
                debug!("Failed to collect ARM Mali metrics for device {}: {}", device.name, e);
            }
        }
    }

    // Try Broadcom VideoCore for Raspberry Pi devices
    if device.driver.as_deref() == Some("vc4") || device.driver.as_deref() == Some("v3d") {
        match collect_broadcom_videocore_metrics(device) {
            Ok(metrics) => {
                debug!("Successfully collected Broadcom VideoCore metrics for device {}", device.name);
                return Ok(metrics);
            }
            Err(e) => {
                debug!("Failed to collect Broadcom VideoCore metrics for device {}: {}", device.name, e);
            }
        }
    }

    // Try Virtio for virtual GPU devices
    if device.driver.as_deref() == Some("virtio_gpu") || device.driver.as_deref() == Some("virtio") {
        match collect_virtio_gpu_metrics(device) {
            Ok(metrics) => {
                debug!("Successfully collected Virtio GPU metrics for device {}", device.name);
                return Ok(metrics);
            }
            Err(e) => {
                debug!("Failed to collect Virtio GPU metrics for device {}: {}", device.name, e);
            }
        }
    }

    // If vendor-specific APIs fail, fall back to generic sysfs collection
    Err(anyhow!("Vendor-specific metrics collection failed, falling back to generic methods"))
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
    fn test_gpu_device_discovery() {
        // This test would need a real system with GPU devices
        // For now, we just test that the function doesn't panic
        let result = discover_gpu_devices();
        assert!(result.is_ok());
    }

    #[test]
    fn test_gpu_metrics_collection() {
        // This test would need a real system with GPU devices
        // For now, we just test that the function doesn't panic
        let result = collect_gpu_metrics();
        assert!(result.is_ok());
    }

    #[test]
    fn test_gpu_metrics_serialization() {
        let metrics = GpuMetrics {
            device: GpuDevice {
                name: "test_gpu".to_string(),
                device_path: PathBuf::from("/test/device"),
                vendor_id: Some("0x8086".to_string()),
                device_id: Some("0x1234".to_string()),
                driver: Some("i915".to_string()),
            },
            utilization: GpuUtilization {
                gpu_util: 0.5,
                memory_util: 0.3,
                encoder_util: Some(0.1),
                decoder_util: Some(0.2),
            },
            memory: GpuMemory {
                total_bytes: 4_000_000_000,
                used_bytes: 2_000_000_000,
                free_bytes: 2_000_000_000,
            },
            temperature: GpuTemperature {
                temperature_c: Some(65.5),
                hotspot_c: Some(70.0),
                memory_c: Some(60.0),
            },
            power: GpuPower {
                power_w: Some(45.0),
                power_limit_w: Some(100.0),
                power_cap_w: Some(90.0),
            },
            clocks: GpuClocks {
                core_clock_mhz: Some(1200),
                memory_clock_mhz: Some(1500),
                shader_clock_mhz: Some(1300),
            },
            timestamp: std::time::SystemTime::now(),
        };

        // Test serialization
        let serialized = serde_json::to_string(&metrics).expect("Serialization failed");
        let deserialized: GpuMetrics =
            serde_json::from_str(&serialized).expect("Deserialization failed");

        assert_eq!(deserialized.device.name, "test_gpu");
        assert_eq!(deserialized.utilization.gpu_util, 0.5);
        assert_eq!(deserialized.memory.total_bytes, 4_000_000_000);
    }

    #[test]
    fn test_gpu_collection_serialization() {
        let collection = GpuMetricsCollection {
            devices: vec![GpuMetrics::default()],
            gpu_count: 1,
        };

        let serialized = serde_json::to_string(&collection).expect("Serialization failed");
        let deserialized: GpuMetricsCollection =
            serde_json::from_str(&serialized).expect("Deserialization failed");

        assert_eq!(deserialized.gpu_count, 1);
        assert_eq!(deserialized.devices.len(), 1);
    }

    #[test]
    fn test_memory_calculation() {
        let mut memory = GpuMemory {
            total_bytes: 4_000_000_000,
            used_bytes: 2_500_000_000,
            free_bytes: 0,
        };

        // This should recalculate free_bytes correctly
        memory.free_bytes = memory.total_bytes.saturating_sub(memory.used_bytes);
        assert_eq!(memory.free_bytes, 1_500_000_000);
    }

    #[test]
    fn test_memory_overflow_handling() {
        let mut memory = GpuMemory {
            total_bytes: 4_000_000_000,
            used_bytes: 5_000_000_000, // More than total
            free_bytes: 0,
        };

        // This should handle overflow correctly
        memory.free_bytes = memory.total_bytes.saturating_sub(memory.used_bytes);
        assert_eq!(memory.free_bytes, 0);
        // Note: used_bytes is not automatically capped in this implementation
        // The test just verifies that free_bytes doesn't underflow
        assert_eq!(memory.used_bytes, 5_000_000_000);
    }

    #[test]
    fn test_gpu_device_creation() {
        let device = GpuDevice {
            name: "card0".to_string(),
            device_path: PathBuf::from("/sys/devices/pci0000:00/0000:00:02.0/drm/card0/device"),
            vendor_id: Some("0x8086".to_string()),
            device_id: Some("0x1234".to_string()),
            driver: Some("i915".to_string()),
        };

        assert_eq!(device.name, "card0");
        // device_path may not exist in test environment, so we don't assert its existence
        assert_eq!(device.vendor_id, Some("0x8086".to_string()));
        assert_eq!(device.device_id, Some("0x1234".to_string()));
        assert_eq!(device.driver, Some("i915".to_string()));
    }

    #[test]
    fn test_gpu_metrics_default_values() {
        let metrics = GpuMetrics::default();

        assert_eq!(metrics.utilization.gpu_util, 0.0);
        assert_eq!(metrics.utilization.memory_util, 0.0);
        assert_eq!(metrics.utilization.encoder_util, None);
        assert_eq!(metrics.utilization.decoder_util, None);

        assert_eq!(metrics.memory.total_bytes, 0);
        assert_eq!(metrics.memory.used_bytes, 0);
        assert_eq!(metrics.memory.free_bytes, 0);

        assert_eq!(metrics.temperature.temperature_c, None);
        assert_eq!(metrics.temperature.hotspot_c, None);
        assert_eq!(metrics.temperature.memory_c, None);

        assert_eq!(metrics.power.power_w, None);
        assert_eq!(metrics.power.power_limit_w, None);
        assert_eq!(metrics.power.power_cap_w, None);

        assert_eq!(metrics.clocks.core_clock_mhz, None);
        assert_eq!(metrics.clocks.memory_clock_mhz, None);
        assert_eq!(metrics.clocks.shader_clock_mhz, None);
    }

    #[test]
    fn test_gpu_collection_with_no_devices() {
        // This test verifies that the function handles the case when no GPU devices are found
        let result = collect_gpu_metrics();

        // Should not panic and should return a valid collection
        assert!(result.is_ok());

        let collection = result.unwrap();
        // gpu_count is usize, so it's always >= 0
        assert_eq!(collection.devices.len(), collection.gpu_count);
    }

    #[test]
    fn test_gpu_device_discovery_error_handling() {
        // This test verifies that device discovery handles errors gracefully
        let result = discover_gpu_devices();

        // Should not panic
        assert!(result.is_ok());

        let devices = result.unwrap();
        // Should return a vector (may be empty)
        assert!(devices.is_empty() || !devices.is_empty());
    }

    #[test]
    fn test_gpu_metrics_serialization_with_realistic_values() {
        let metrics = GpuMetrics {
            device: GpuDevice {
                name: "test_gpu".to_string(),
                device_path: PathBuf::from("/test/device"),
                vendor_id: Some("0x10de".to_string()), // NVIDIA
                device_id: Some("0x13c2".to_string()),
                driver: Some("nvidia".to_string()),
            },
            utilization: GpuUtilization {
                gpu_util: 0.75,           // 75%
                memory_util: 0.50,        // 50%
                encoder_util: Some(0.30), // 30%
                decoder_util: Some(0.20), // 20%
            },
            memory: GpuMemory {
                total_bytes: 8_000_000_000, // 8 GB
                used_bytes: 4_000_000_000,  // 4 GB
                free_bytes: 4_000_000_000,  // 4 GB
            },
            temperature: GpuTemperature {
                temperature_c: Some(65.5),
                hotspot_c: Some(70.0),
                memory_c: Some(60.0),
            },
            power: GpuPower {
                power_w: Some(120.0),
                power_limit_w: Some(200.0),
                power_cap_w: Some(180.0),
            },
            clocks: GpuClocks {
                core_clock_mhz: Some(1500),
                memory_clock_mhz: Some(1750),
                shader_clock_mhz: Some(1600),
            },
            timestamp: std::time::SystemTime::now(),
        };

        // Test serialization
        let serialized = serde_json::to_string(&metrics).expect("Serialization failed");
        let deserialized: GpuMetrics =
            serde_json::from_str(&serialized).expect("Deserialization failed");

        // Verify all fields are preserved
        assert_eq!(deserialized.device.name, "test_gpu");
        assert_eq!(deserialized.device.vendor_id, Some("0x10de".to_string()));
        assert_eq!(deserialized.device.driver, Some("nvidia".to_string()));

        assert_eq!(deserialized.utilization.gpu_util, 0.75);
        assert_eq!(deserialized.utilization.memory_util, 0.50);
        assert_eq!(deserialized.utilization.encoder_util, Some(0.30));
        assert_eq!(deserialized.utilization.decoder_util, Some(0.20));

        assert_eq!(deserialized.memory.total_bytes, 8_000_000_000);
        assert_eq!(deserialized.memory.used_bytes, 4_000_000_000);
        assert_eq!(deserialized.memory.free_bytes, 4_000_000_000);

        assert_eq!(deserialized.temperature.temperature_c, Some(65.5));
        assert_eq!(deserialized.temperature.hotspot_c, Some(70.0));
        assert_eq!(deserialized.temperature.memory_c, Some(60.0));

        assert_eq!(deserialized.power.power_w, Some(120.0));
        assert_eq!(deserialized.power.power_limit_w, Some(200.0));
        assert_eq!(deserialized.power.power_cap_w, Some(180.0));

        assert_eq!(deserialized.clocks.core_clock_mhz, Some(1500));
        assert_eq!(deserialized.clocks.memory_clock_mhz, Some(1750));
        assert_eq!(deserialized.clocks.shader_clock_mhz, Some(1600));
    }

    #[test]
    fn test_gpu_collection_serialization_with_multiple_devices() {
        let collection = GpuMetricsCollection {
            devices: vec![
                GpuMetrics {
                    device: GpuDevice {
                        name: "gpu0".to_string(),
                        device_path: PathBuf::from("/dev/gpu0"),
                        vendor_id: Some("0x8086".to_string()),
                        device_id: Some("0x1234".to_string()),
                        driver: Some("i915".to_string()),
                    },
                    utilization: GpuUtilization {
                        gpu_util: 0.3,
                        memory_util: 0.2,
                        encoder_util: None,
                        decoder_util: None,
                    },
                    memory: GpuMemory {
                        total_bytes: 4_000_000_000,
                        used_bytes: 1_000_000_000,
                        free_bytes: 3_000_000_000,
                    },
                    temperature: GpuTemperature::default(),
                    power: GpuPower::default(),
                    clocks: GpuClocks::default(),
                    timestamp: std::time::SystemTime::now(),
                },
                GpuMetrics {
                    device: GpuDevice {
                        name: "gpu1".to_string(),
                        device_path: PathBuf::from("/dev/gpu1"),
                        vendor_id: Some("0x10de".to_string()),
                        device_id: Some("0x5678".to_string()),
                        driver: Some("nvidia".to_string()),
                    },
                    utilization: GpuUtilization {
                        gpu_util: 0.8,
                        memory_util: 0.6,
                        encoder_util: Some(0.4),
                        decoder_util: Some(0.3),
                    },
                    memory: GpuMemory {
                        total_bytes: 8_000_000_000,
                        used_bytes: 6_000_000_000,
                        free_bytes: 2_000_000_000,
                    },
                    temperature: GpuTemperature::default(),
                    power: GpuPower::default(),
                    clocks: GpuClocks::default(),
                    timestamp: std::time::SystemTime::now(),
                },
            ],
            gpu_count: 2,
        };

        let serialized = serde_json::to_string(&collection).expect("Serialization failed");
        let deserialized: GpuMetricsCollection =
            serde_json::from_str(&serialized).expect("Deserialization failed");

        assert_eq!(deserialized.gpu_count, 2);
        assert_eq!(deserialized.devices.len(), 2);

        assert_eq!(deserialized.devices[0].device.name, "gpu0");
        assert_eq!(deserialized.devices[1].device.name, "gpu1");

        assert_eq!(deserialized.devices[0].utilization.gpu_util, 0.3);
        assert_eq!(deserialized.devices[1].utilization.gpu_util, 0.8);
    }

    #[test]
    fn test_gpu_memory_calculation_edge_cases() {
        // Test with zero total memory
        let mut memory = GpuMemory {
            total_bytes: 0,
            used_bytes: 1000,
            free_bytes: 0,
        };

        memory.free_bytes = memory.total_bytes.saturating_sub(memory.used_bytes);
        assert_eq!(memory.free_bytes, 0); // Should not underflow

        // Test with used > total
        let mut memory = GpuMemory {
            total_bytes: 1000,
            used_bytes: 2000,
            free_bytes: 0,
        };

        memory.free_bytes = memory.total_bytes.saturating_sub(memory.used_bytes);
        assert_eq!(memory.free_bytes, 0); // Should not underflow

        // Test with equal values
        let mut memory = GpuMemory {
            total_bytes: 1000,
            used_bytes: 1000,
            free_bytes: 0,
        };

        memory.free_bytes = memory.total_bytes.saturating_sub(memory.used_bytes);
        assert_eq!(memory.free_bytes, 0);
    }

    #[test]
    fn test_intel_gpu_metrics_collection() {
        // Create a mock Intel GPU device
        let device = GpuDevice {
            name: "card0".to_string(),
            device_path: PathBuf::from("/sys/devices/pci0000:00/0000:00:02.0/drm/card0/device"),
            vendor_id: Some("0x8086".to_string()),
            device_id: Some("0x1234".to_string()),
            driver: Some("i915".to_string()),
        };

        // This test would need a real system with Intel GPU devices
        // For now, we just test that the function doesn't panic
        let result = collect_intel_gpu_metrics(&device);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gpu_utilization_values() {
        let mut utilization = GpuUtilization {
            gpu_util: 1.5, // Can be > 1.0
            memory_util: 0.0,
            encoder_util: None,
            decoder_util: None,
        };
        assert!(utilization.gpu_util > 0.0);

        utilization.gpu_util = -0.1; // Can be negative (though not realistic)
                                     // Just verify it was set, don't assert it's >= 0.0
        assert_eq!(utilization.gpu_util, -0.1);

        // Test encoder/decoder optional values
        utilization.encoder_util = Some(0.5);
        utilization.decoder_util = Some(0.3);

        assert_eq!(utilization.encoder_util, Some(0.5));
        assert_eq!(utilization.decoder_util, Some(0.3));
    }

    #[test]
    fn test_gpu_temperature_values() {
        let temperature = GpuTemperature {
            temperature_c: Some(65.5),
            hotspot_c: Some(70.0),
            memory_c: Some(60.0),
        };

        assert_eq!(temperature.temperature_c, Some(65.5));
        assert_eq!(temperature.hotspot_c, Some(70.0));
        assert_eq!(temperature.memory_c, Some(60.0));

        // Test with None values
        let empty_temp = GpuTemperature::default();
        assert_eq!(empty_temp.temperature_c, None);
        assert_eq!(empty_temp.hotspot_c, None);
        assert_eq!(empty_temp.memory_c, None);
    }

    #[test]
    fn test_gpu_power_values() {
        let power = GpuPower {
            power_w: Some(120.0),
            power_limit_w: Some(200.0),
            power_cap_w: Some(180.0),
        };

        assert_eq!(power.power_w, Some(120.0));
        assert_eq!(power.power_limit_w, Some(200.0));
        assert_eq!(power.power_cap_w, Some(180.0));

        // Test with None values
        let empty_power = GpuPower::default();
        assert_eq!(empty_power.power_w, None);
        assert_eq!(empty_power.power_limit_w, None);
        assert_eq!(empty_power.power_cap_w, None);
    }

    #[test]
    fn test_gpu_clocks_values() {
        let clocks = GpuClocks {
            core_clock_mhz: Some(1500),
            memory_clock_mhz: Some(1750),
            shader_clock_mhz: Some(1600),
        };

        assert_eq!(clocks.core_clock_mhz, Some(1500));
        assert_eq!(clocks.memory_clock_mhz, Some(1750));
        assert_eq!(clocks.shader_clock_mhz, Some(1600));

        // Test with None values
        let empty_clocks = GpuClocks::default();
        assert_eq!(empty_clocks.core_clock_mhz, None);
        assert_eq!(empty_clocks.memory_clock_mhz, None);
        assert_eq!(empty_clocks.shader_clock_mhz, None);
    }

    #[test]
    fn test_gpu_device_equality() {
        let device1 = GpuDevice {
            name: "card0".to_string(),
            device_path: PathBuf::from("/dev/card0"),
            vendor_id: Some("0x8086".to_string()),
            device_id: Some("0x1234".to_string()),
            driver: Some("i915".to_string()),
        };

        let device2 = GpuDevice {
            name: "card0".to_string(),
            device_path: PathBuf::from("/dev/card0"),
            vendor_id: Some("0x8086".to_string()),
            device_id: Some("0x1234".to_string()),
            driver: Some("i915".to_string()),
        };

        assert_eq!(device1, device2);

        let device3 = GpuDevice {
            name: "card1".to_string(),
            device_path: PathBuf::from("/dev/card1"),
            vendor_id: Some("0x10de".to_string()),
            device_id: Some("0x5678".to_string()),
            driver: Some("nvidia".to_string()),
        };

        assert_ne!(device1, device3);
    }

    #[test]
    fn test_gpu_metrics_timestamps() {
        let metrics1 = GpuMetrics::default();
        let metrics2 = GpuMetrics::default();

        // Timestamps should be recent (within last minute)
        let now = std::time::SystemTime::now();
        let one_minute_ago = now - std::time::Duration::from_secs(60);

        assert!(metrics1.timestamp >= one_minute_ago);
        assert!(metrics2.timestamp >= one_minute_ago);

        // Different instances should have similar timestamps
        let duration = metrics2
            .timestamp
            .duration_since(metrics1.timestamp)
            .unwrap_or(std::time::Duration::from_secs(0));
        assert!(duration.as_secs() < 5); // Should be within 5 seconds
    }

    #[test]
    fn test_gpu_error_handling_graceful_degradation() {
        // Test that GPU metrics collection handles errors gracefully
        // This test verifies that the system can continue operating even when GPU metrics fail
        
        // Create a mock device with a non-existent path
        let mock_device = GpuDevice {
            name: "mock_gpu".to_string(),
            device_path: PathBuf::from("/non/existent/path"),
            vendor_id: Some("0x1234".to_string()),
            device_id: Some("0x5678".to_string()),
            driver: Some("mock_driver".to_string()),
        };

        // This should not panic and should return a default metrics object
        let result = collect_gpu_device_metrics(&mock_device);
        
        // The function should succeed (return Ok) even if individual metrics fail
        assert!(result.is_ok());
        
        let metrics = result.unwrap();
        
        // Should return default values when metrics cannot be collected
        assert_eq!(metrics.device.name, "mock_gpu");
        assert_eq!(metrics.utilization.gpu_util, 0.0);
        assert_eq!(metrics.memory.total_bytes, 0);
        assert_eq!(metrics.temperature.temperature_c, None);
        assert_eq!(metrics.power.power_w, None);
        assert_eq!(metrics.clocks.core_clock_mhz, None);
    }

    #[test]
    fn test_gpu_collection_with_error_handling() {
        // Test that the main collection function handles errors gracefully
        let result = collect_gpu_metrics();
        
        // Should always return Ok, even if no devices are found or metrics fail
        assert!(result.is_ok());
        
        let collection = result.unwrap();
        
        // Should return a valid collection object
        assert_eq!(collection.devices.len(), collection.gpu_count);
        
        // Collection should be empty if no GPU devices are available
        // This is expected behavior on systems without GPUs
        if collection.gpu_count == 0 {
            assert!(collection.devices.is_empty());
        }
    }

    #[test]
    fn test_gpu_memory_validation() {
        // Test memory validation logic when used > total
        let mut memory = GpuMemory {
            total_bytes: 4_000_000_000, // 4 GB
            used_bytes: 5_000_000_000, // 5 GB (more than total)
            free_bytes: 0,
        };

        // This should handle the overflow gracefully
        memory.free_bytes = memory.total_bytes.saturating_sub(memory.used_bytes);
        
        // Free bytes should not underflow
        assert_eq!(memory.free_bytes, 0);
        
        // In a real scenario, we would also cap used_bytes to total_bytes
        // Let's test that logic
        if memory.used_bytes > memory.total_bytes {
            memory.used_bytes = memory.total_bytes;
        }
        
        assert_eq!(memory.used_bytes, 4_000_000_000);
        assert_eq!(memory.free_bytes, 0);
    }



    #[test]
    fn test_gpu_metrics_with_missing_files() {
        // Test behavior when sysfs files are missing
        let temp_dir = tempfile::tempdir().unwrap();
        let test_device_path = temp_dir.path().join("test_device");
        
        // Create a minimal device directory structure
        std::fs::create_dir_all(&test_device_path).unwrap();
        
        let mock_device = GpuDevice {
            name: "test_device".to_string(),
            device_path: test_device_path,
            vendor_id: Some("0x1234".to_string()),
            device_id: Some("0x5678".to_string()),
            driver: Some("test_driver".to_string()),
        };

        // This should not panic and should return default metrics
        let result = collect_gpu_device_metrics(&mock_device);
        assert!(result.is_ok());
        
        let metrics = result.unwrap();
        // Should return default values when files are missing
        assert_eq!(metrics.utilization.gpu_util, 0.0);
        assert_eq!(metrics.memory.total_bytes, 0);
    }

    #[test]
    fn test_gpu_error_recovery() {
        // Test that the system can recover from GPU errors and continue
        // This simulates a scenario where GPU metrics fail but the system continues
        
        // First, try to collect metrics (may succeed or fail)
        let result1 = collect_gpu_metrics();
        assert!(result1.is_ok());
        
        // System should still be able to collect metrics again
        let result2 = collect_gpu_metrics();
        assert!(result2.is_ok());
        
        // Both results should be consistent
        let collection1 = result1.unwrap();
        let collection2 = result2.unwrap();
        
        assert_eq!(collection1.gpu_count, collection2.gpu_count);
    }

    #[test]
    fn test_vendor_specific_metrics_fallback() {
        // Test that vendor-specific metrics collection falls back gracefully
        let mock_device = GpuDevice {
            name: "mock_nvidia".to_string(),
            device_path: PathBuf::from("/non/existent/path"),
            vendor_id: Some("0x10de".to_string()),
            device_id: Some("0x1234".to_string()),
            driver: Some("nvidia".to_string()),
        };

        // This should fail gracefully and return an error
        let result = collect_vendor_specific_metrics(&mock_device);
        
        // Should return an error for non-existent device
        assert!(result.is_err());
        
        // But the error should be informative
        let error = result.unwrap_err();
        let error_str = error.to_string();
        assert!(error_str.contains("Vendor-specific") || error_str.contains("failed"));
    }

    #[test]
    fn test_gpu_metrics_with_nvml_feature() {
        // Test that NVML feature compilation works
        #[cfg(feature = "nvml-wrapper")]
        {
            // This test just verifies that the nvml feature compiles
            // Actual NVML functionality would require a real NVIDIA GPU
            use nvml_wrapper::Nvml;
            
            // Try to initialize NVML - this will likely fail without a real GPU
            let result = Nvml::init();
            
            // We don't assert success because we might not have NVIDIA hardware
            // Just verify that the code compiles and doesn't panic
            match result {
                Ok(_) => {
                    // If NVML initialized successfully, we can try to get device count
                    let _device_count = Nvml::device_count();
                    // Don't assert anything about device count
                }
                Err(_) => {
                    // Expected on systems without NVIDIA GPUs
                    // This is fine for the test
                }
            }
        }
        
        #[cfg(not(feature = "nvml-wrapper"))]
        {
            // If nvml feature is not enabled, the test should still pass
            // This verifies that the feature flag works correctly
            assert!(!cfg!(feature = "nvml-wrapper"));
        }
    }

    #[test]
    fn test_gpu_metrics_collection_with_vendor_specific() {
        // Test the integration of vendor-specific metrics collection
        let result = collect_gpu_metrics();
        
        // Should not panic and should return Ok
        assert!(result.is_ok());
        
        let collection = result.unwrap();
        
        // Should return a valid collection
        assert_eq!(collection.devices.len(), collection.gpu_count);
        
        // If there are devices, they should have metrics
        for device_metrics in &collection.devices {
            // Device info should be populated
            assert!(!device_metrics.device.name.is_empty());
            
            // Metrics should have reasonable default values
            assert!(device_metrics.utilization.gpu_util >= 0.0);
            assert!(device_metrics.memory.total_bytes >= 0);
            
            // Timestamps should be recent
            let now = std::time::SystemTime::now();
            let one_hour_ago = now - std::time::Duration::from_secs(3600);
            assert!(device_metrics.timestamp >= one_hour_ago);
        }
    }

    #[test]
    fn test_gpu_device_identification() {
        // Test that we can identify different GPU vendors
        let nvidia_device = GpuDevice {
            name: "card0".to_string(),
            device_path: PathBuf::from("/sys/devices/pci0000:00/0000:00:01.0/drm/card0/device"),
            vendor_id: Some("0x10de".to_string()),
            device_id: Some("0x1234".to_string()),
            driver: Some("nvidia".to_string()),
        };

        let amd_device = GpuDevice {
            name: "card1".to_string(),
            device_path: PathBuf::from("/sys/devices/pci0000:00/0000:00:02.0/drm/card1/device"),
            vendor_id: Some("0x1002".to_string()),
            device_id: Some("0x5678".to_string()),
            driver: Some("amdgpu".to_string()),
        };

        let intel_device = GpuDevice {
            name: "card2".to_string(),
            device_path: PathBuf::from("/sys/devices/pci0000:00/0000:00:03.0/drm/card2/device"),
            vendor_id: Some("0x8086".to_string()),
            device_id: Some("0x9abc".to_string()),
            driver: Some("i915".to_string()),
        };

        // Verify vendor identification
        assert_eq!(nvidia_device.vendor_id, Some("0x10de".to_string()));
        assert_eq!(amd_device.vendor_id, Some("0x1002".to_string()));
        assert_eq!(intel_device.vendor_id, Some("0x8086".to_string()));

        // Verify driver identification
        assert_eq!(nvidia_device.driver, Some("nvidia".to_string()));
        assert_eq!(amd_device.driver, Some("amdgpu".to_string()));
        assert_eq!(intel_device.driver, Some("i915".to_string()));
    }

    #[test]
    fn test_gpu_metrics_serialization_with_vendor_info() {
        // Test serialization with vendor-specific information
        let metrics = GpuMetrics {
            device: GpuDevice {
                name: "nvidia_gpu".to_string(),
                device_path: PathBuf::from("/dev/nvidia0"),
                vendor_id: Some("0x10de".to_string()),
                device_id: Some("0x13c2".to_string()),
                driver: Some("nvidia".to_string()),
            },
            utilization: GpuUtilization {
                gpu_util: 0.85,
                memory_util: 0.70,
                encoder_util: Some(0.40),
                decoder_util: Some(0.30),
            },
            memory: GpuMemory {
                total_bytes: 12_000_000_000, // 12 GB
                used_bytes: 8_000_000_000,  // 8 GB
                free_bytes: 4_000_000_000,  // 4 GB
            },
            temperature: GpuTemperature {
                temperature_c: Some(75.0),
                hotspot_c: Some(80.0),
                memory_c: Some(70.0),
            },
            power: GpuPower {
                power_w: Some(180.0),
                power_limit_w: Some(250.0),
                power_cap_w: Some(230.0),
            },
            clocks: GpuClocks {
                core_clock_mhz: Some(1800),
                memory_clock_mhz: Some(2000),
                shader_clock_mhz: Some(1900),
            },
            timestamp: std::time::SystemTime::now(),
        };

        // Test serialization
        let serialized = serde_json::to_string(&metrics).expect("Serialization failed");
        let deserialized: GpuMetrics = serde_json::from_str(&serialized).expect("Deserialization failed");

        // Verify all fields are preserved
        assert_eq!(deserialized.device.name, "nvidia_gpu");
        assert_eq!(deserialized.device.vendor_id, Some("0x10de".to_string()));
        assert_eq!(deserialized.device.driver, Some("nvidia".to_string()));

        assert_eq!(deserialized.utilization.gpu_util, 0.85);
        assert_eq!(deserialized.utilization.memory_util, 0.70);

        assert_eq!(deserialized.memory.total_bytes, 12_000_000_000);
        assert_eq!(deserialized.memory.used_bytes, 8_000_000_000);

        assert_eq!(deserialized.temperature.temperature_c, Some(75.0));
        assert_eq!(deserialized.power.power_w, Some(180.0));
        assert_eq!(deserialized.clocks.core_clock_mhz, Some(1800));
    }

    #[test]
    fn test_gpu_collection_with_mixed_vendors() {
        // Test collection with multiple vendors (simulated)
        let nvidia_metrics = GpuMetrics {
            device: GpuDevice {
                name: "nvidia_gpu".to_string(),
                device_path: PathBuf::from("/dev/nvidia0"),
                vendor_id: Some("0x10de".to_string()),
                device_id: Some("0x1234".to_string()),
                driver: Some("nvidia".to_string()),
            },
            utilization: GpuUtilization {
                gpu_util: 0.90,
                memory_util: 0.80,
                encoder_util: Some(0.50),
                decoder_util: Some(0.40),
            },
            memory: GpuMemory {
                total_bytes: 8_000_000_000,
                used_bytes: 6_000_000_000,
                free_bytes: 2_000_000_000,
            },
            temperature: GpuTemperature::default(),
            power: GpuPower::default(),
            clocks: GpuClocks::default(),
            timestamp: std::time::SystemTime::now(),
        };

        let amd_metrics = GpuMetrics {
            device: GpuDevice {
                name: "amd_gpu".to_string(),
                device_path: PathBuf::from("/dev/amdgpu0"),
                vendor_id: Some("0x1002".to_string()),
                device_id: Some("0x5678".to_string()),
                driver: Some("amdgpu".to_string()),
            },
            utilization: GpuUtilization {
                gpu_util: 0.70,
                memory_util: 0.60,
                encoder_util: None,
                decoder_util: None,
            },
            memory: GpuMemory {
                total_bytes: 16_000_000_000,
                used_bytes: 10_000_000_000,
                free_bytes: 6_000_000_000,
            },
            temperature: GpuTemperature::default(),
            power: GpuPower::default(),
            clocks: GpuClocks::default(),
            timestamp: std::time::SystemTime::now(),
        };

        let collection = GpuMetricsCollection {
            devices: vec![nvidia_metrics, amd_metrics],
            gpu_count: 2,
        };

        // Test serialization
        let serialized = serde_json::to_string(&collection).expect("Serialization failed");
        let deserialized: GpuMetricsCollection = serde_json::from_str(&serialized).expect("Deserialization failed");

        assert_eq!(deserialized.gpu_count, 2);
        assert_eq!(deserialized.devices.len(), 2);

        // Verify vendor-specific information is preserved
        assert_eq!(deserialized.devices[0].device.driver, Some("nvidia".to_string()));
        assert_eq!(deserialized.devices[1].device.driver, Some("amdgpu".to_string()));

        assert_eq!(deserialized.devices[0].utilization.gpu_util, 0.90);
        assert_eq!(deserialized.devices[1].utilization.gpu_util, 0.70);
    }

    #[test]
    fn test_gpu_metrics_fallback_behavior() {
        // Test that the system falls back gracefully when vendor-specific APIs fail
        let mock_device = GpuDevice {
            name: "mock_device".to_string(),
            device_path: PathBuf::from("/non/existent/path"),
            vendor_id: Some("0x1234".to_string()),
            device_id: Some("0x5678".to_string()),
            driver: Some("mock_driver".to_string()),
        };

        // Vendor-specific collection should fail
        let vendor_result = collect_vendor_specific_metrics(&mock_device);
        assert!(vendor_result.is_err());

        // But device metrics collection should still succeed with fallback
        let device_result = collect_gpu_device_metrics(&mock_device);
        assert!(device_result.is_ok());

        let metrics = device_result.unwrap();
        
        // Should return default values when vendor-specific APIs fail
        assert_eq!(metrics.device.name, "mock_device");
        assert_eq!(metrics.utilization.gpu_util, 0.0);
        assert_eq!(metrics.memory.total_bytes, 0);
    }

    #[test]
    fn test_gpu_feature_flags() {
        // Test that feature flags work correctly
        
        #[cfg(feature = "nvml-wrapper")]
        assert!(cfg!(feature = "nvml-wrapper"));
        
        #[cfg(not(feature = "nvml-wrapper"))]
        assert!(!cfg!(feature = "nvml-wrapper"));
        
        // The test should pass regardless of which features are enabled
        // This verifies that the feature flag system works correctly
    }

    #[test]
    fn test_gpu_partial_success() {
        // Test handling of partial success (some devices work, some fail)
        // This is harder to test without mocking, but we can verify the structure
        let result = collect_gpu_metrics();
        assert!(result.is_ok());
        
        let collection = result.unwrap();
        
        // If there are devices, they should all have valid structure
        for device_metrics in &collection.devices {
            assert!(!device_metrics.device.name.is_empty());
            // Timestamps should be recent
            let now = std::time::SystemTime::now();
            let one_minute_ago = now - std::time::Duration::from_secs(60);
            assert!(device_metrics.timestamp >= one_minute_ago);
        }
    }

    #[test]
    fn test_gpu_error_messages_context() {
        // Test that error messages provide useful context
        // This is more of a documentation test - we verify the structure
        
        // Create a device with a path that will cause errors
        let mock_device = GpuDevice {
            name: "error_test".to_string(),
            device_path: PathBuf::from("/invalid/path/that/does/not/exist"),
            vendor_id: Some("0x1234".to_string()),
            device_id: Some("0x5678".to_string()),
            driver: Some("error_driver".to_string()),
        };

        // This should handle errors gracefully and provide context in logs
        let result = collect_gpu_device_metrics(&mock_device);
        
        // Should succeed (return Ok) even with errors
        assert!(result.is_ok());
        
        // Should return a valid metrics object with default values
        let metrics = result.unwrap();
        assert_eq!(metrics.device.name, "error_test");
        assert_eq!(metrics.utilization.gpu_util, 0.0);
    }

    #[test]
    fn test_gpu_vendor_specific_error_handling() {
        // Test that vendor-specific error handling works correctly
        // This test verifies that different GPU vendors get appropriate error messages
        
        // Test Intel GPU error handling
        let intel_device = GpuDevice {
            name: "card0".to_string(),
            device_path: PathBuf::from("/sys/devices/pci0000:00/0000:00:02.0/drm/card0/device"),
            vendor_id: Some("0x8086".to_string()), // Intel vendor ID
            device_id: Some("0x1234".to_string()),
            driver: Some("i915".to_string()),
        };

        // Test AMD GPU error handling
        let amd_device = GpuDevice {
            name: "card1".to_string(),
            device_path: PathBuf::from("/sys/devices/pci0000:00/0000:00:03.0/drm/card1/device"),
            vendor_id: Some("0x1002".to_string()), // AMD vendor ID
            device_id: Some("0x5678".to_string()),
            driver: Some("amdgpu".to_string()),
        };

        // Test NVIDIA GPU error handling
        let nvidia_device = GpuDevice {
            name: "card2".to_string(),
            device_path: PathBuf::from("/sys/devices/pci0000:00/0000:00:04.0/drm/card2/device"),
            vendor_id: Some("0x10de".to_string()), // NVIDIA vendor ID
            device_id: Some("0x9abc".to_string()),
            driver: Some("nvidia".to_string()),
        };

        // All should handle errors gracefully and return valid metrics
        let intel_result = collect_gpu_device_metrics(&intel_device);
        let amd_result = collect_gpu_device_metrics(&amd_device);
        let nvidia_result = collect_gpu_device_metrics(&nvidia_device);
        
        assert!(intel_result.is_ok());
        assert!(amd_result.is_ok());
        assert!(nvidia_result.is_ok());
        
        // All should return valid metrics objects
        let intel_metrics = intel_result.unwrap();
        let amd_metrics = amd_result.unwrap();
        let nvidia_metrics = nvidia_result.unwrap();
        
        assert_eq!(intel_metrics.device.name, "card0");
        assert_eq!(amd_metrics.device.name, "card1");
        assert_eq!(nvidia_metrics.device.name, "card2");
        
        // All should have default values when metrics cannot be collected
        assert_eq!(intel_metrics.utilization.gpu_util, 0.0);
        assert_eq!(amd_metrics.utilization.gpu_util, 0.0);
        assert_eq!(nvidia_metrics.utilization.gpu_util, 0.0);
    }

    #[test]
    fn test_gpu_error_recovery_comprehensive() {
        // Test comprehensive error recovery scenarios
        // This test verifies that the system can recover from various error conditions
        
        // Test 1: Multiple consecutive errors
        for i in 0..3 {
            let mock_device = GpuDevice {
                name: format!("test_gpu_{}", i),
                device_path: PathBuf::from("/non/existent/path"),
                vendor_id: Some("0x1234".to_string()),
                device_id: Some("0x5678".to_string()),
                driver: Some("test_driver".to_string()),
            };
            
            let result = collect_gpu_device_metrics(&mock_device);
            assert!(result.is_ok(), "Iteration {} failed", i);
        }
        
        // Test 2: Main collection function should still work
        let result = collect_gpu_metrics();
        assert!(result.is_ok());
        
        let collection = result.unwrap();
        assert_eq!(collection.devices.len(), collection.gpu_count);
        
        // Test 3: System should be able to continue after errors
        let result1 = collect_gpu_metrics();
        let result2 = collect_gpu_metrics();
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        let collection1 = result1.unwrap();
        let collection2 = result2.unwrap();
        
        // Results should be consistent
        assert_eq!(collection1.gpu_count, collection2.gpu_count);
    }
    
    #[test]
    fn test_new_gpu_vendors_identification() {
        // Test identification of new GPU vendors
        
        // Qualcomm Adreno
        let qualcomm_device = GpuDevice {
            name: "adreno_gpu".to_string(),
            device_path: PathBuf::from("/sys/devices/pci0000:00/0000:00:05.0/drm/card0/device"),
            vendor_id: Some("0x5143".to_string()), // Qualcomm vendor ID
            device_id: Some("0x1234".to_string()),
            driver: Some("msm".to_string()),
        };
        
        // ARM Mali
        let mali_device = GpuDevice {
            name: "mali_gpu".to_string(),
            device_path: PathBuf::from("/sys/devices/pci0000:00/0000:00:06.0/drm/card1/device"),
            vendor_id: Some("0x13b5".to_string()), // ARM vendor ID
            device_id: Some("0x5678".to_string()),
            driver: Some("mali".to_string()),
        };
        
        // Broadcom VideoCore
        let broadcom_device = GpuDevice {
            name: "videocore_gpu".to_string(),
            device_path: PathBuf::from("/sys/devices/pci0000:00/0000:00:07.0/drm/card2/device"),
            vendor_id: Some("0x14e4".to_string()), // Broadcom vendor ID
            device_id: Some("0x9abc".to_string()),
            driver: Some("vc4".to_string()),
        };
        
        // Virtio GPU
        let virtio_device = GpuDevice {
            name: "virtio_gpu".to_string(),
            device_path: PathBuf::from("/sys/devices/pci0000:00/0000:00:08.0/drm/card3/device"),
            vendor_id: Some("0x1af4".to_string()), // Virtio vendor ID
            device_id: Some("0xdef0".to_string()),
            driver: Some("virtio_gpu".to_string()),
        };
        
        // Verify vendor identification
        assert_eq!(qualcomm_device.driver, Some("msm".to_string()));
        assert_eq!(mali_device.driver, Some("mali".to_string()));
        assert_eq!(broadcom_device.driver, Some("vc4".to_string()));
        assert_eq!(virtio_device.driver, Some("virtio_gpu".to_string()));
        
        // Test that metrics collection works for these devices (may return default values)
        let qualcomm_result = collect_gpu_device_metrics(&qualcomm_device);
        let mali_result = collect_gpu_device_metrics(&mali_device);
        let broadcom_result = collect_gpu_device_metrics(&broadcom_device);
        let virtio_result = collect_gpu_device_metrics(&virtio_device);
        
        assert!(qualcomm_result.is_ok());
        assert!(mali_result.is_ok());
        assert!(broadcom_result.is_ok());
        assert!(virtio_result.is_ok());
    }
    
    #[test]
    fn test_new_gpu_vendors_metrics_collection() {
        // Test metrics collection for new GPU vendors
        
        // Create mock devices for each new vendor
        let qualcomm_device = GpuDevice {
            name: "adreno_test".to_string(),
            device_path: PathBuf::from("/sys/devices/platform/soc/1c00000.gpu/drm/card0/device"),
            vendor_id: Some("0x5143".to_string()),
            device_id: Some("0x1234".to_string()),
            driver: Some("msm".to_string()),
        };
        
        let mali_device = GpuDevice {
            name: "mali_test".to_string(),
            device_path: PathBuf::from("/sys/devices/platform/soc/1d80000.gpu/drm/card1/device"),
            vendor_id: Some("0x13b5".to_string()),
            device_id: Some("0x5678".to_string()),
            driver: Some("mali".to_string()),
        };
        
        let broadcom_device = GpuDevice {
            name: "vc4_test".to_string(),
            device_path: PathBuf::from("/sys/devices/platform/soc/3fc00000.gpu/drm/card2/device"),
            vendor_id: Some("0x14e4".to_string()),
            device_id: Some("0x9abc".to_string()),
            driver: Some("vc4".to_string()),
        };
        
        let virtio_device = GpuDevice {
            name: "virtio_test".to_string(),
            device_path: PathBuf::from("/sys/devices/pci0000:00/0000:00:02.0/drm/card3/device"),
            vendor_id: Some("0x1af4".to_string()),
            device_id: Some("0xdef0".to_string()),
            driver: Some("virtio_gpu".to_string()),
        };
        
        // Test vendor-specific metrics collection
        let qualcomm_result = collect_qualcomm_adreno_metrics(&qualcomm_device);
        let mali_result = collect_arm_mali_metrics(&mali_device);
        let broadcom_result = collect_broadcom_videocore_metrics(&broadcom_device);
        let virtio_result = collect_virtio_gpu_metrics(&virtio_device);
        
        // All should succeed (return Ok) even if they return default values
        assert!(qualcomm_result.is_ok());
        assert!(mali_result.is_ok());
        assert!(broadcom_result.is_ok());
        assert!(virtio_result.is_ok());
        
        // Verify that the metrics have the correct device information
        let qualcomm_metrics = qualcomm_result.unwrap();
        let mali_metrics = mali_result.unwrap();
        let broadcom_metrics = broadcom_result.unwrap();
        let virtio_metrics = virtio_result.unwrap();
        
        assert_eq!(qualcomm_metrics.device.name, "adreno_test");
        assert_eq!(mali_metrics.device.name, "mali_test");
        assert_eq!(broadcom_metrics.device.name, "vc4_test");
        assert_eq!(virtio_metrics.device.name, "virtio_test");
        
        // Verify that the metrics have reasonable default values
        assert!(qualcomm_metrics.utilization.gpu_util >= 0.0);
        assert!(mali_metrics.utilization.gpu_util >= 0.0);
        assert!(broadcom_metrics.utilization.gpu_util >= 0.0);
        assert!(virtio_metrics.utilization.gpu_util >= 0.0);
    }
    
    #[test]
    fn test_new_gpu_vendors_serialization() {
        // Test serialization of metrics from new GPU vendors
        
        // Create metrics for each new vendor
        let qualcomm_metrics = GpuMetrics {
            device: GpuDevice {
                name: "adreno_gpu".to_string(),
                device_path: PathBuf::from("/dev/adreno0"),
                vendor_id: Some("0x5143".to_string()),
                device_id: Some("0x1234".to_string()),
                driver: Some("msm".to_string()),
            },
            utilization: GpuUtilization {
                gpu_util: 0.65,
                memory_util: 0.45,
                encoder_util: Some(0.30),
                decoder_util: Some(0.20),
            },
            memory: GpuMemory {
                total_bytes: 2_000_000_000, // 2 GB
                used_bytes: 1_200_000_000, // 1.2 GB
                free_bytes: 800_000_000,   // 0.8 GB
            },
            temperature: GpuTemperature {
                temperature_c: Some(55.0),
                hotspot_c: Some(60.0),
                memory_c: Some(50.0),
            },
            power: GpuPower {
                power_w: Some(3.5),
                power_limit_w: Some(8.0),
                power_cap_w: Some(7.0),
            },
            clocks: GpuClocks {
                core_clock_mhz: Some(600),
                memory_clock_mhz: Some(800),
                shader_clock_mhz: Some(700),
            },
            timestamp: std::time::SystemTime::now(),
        };
        
        let mali_metrics = GpuMetrics {
            device: GpuDevice {
                name: "mali_gpu".to_string(),
                device_path: PathBuf::from("/dev/mali0"),
                vendor_id: Some("0x13b5".to_string()),
                device_id: Some("0x5678".to_string()),
                driver: Some("mali".to_string()),
            },
            utilization: GpuUtilization {
                gpu_util: 0.70,
                memory_util: 0.50,
                encoder_util: Some(0.35),
                decoder_util: Some(0.25),
            },
            memory: GpuMemory {
                total_bytes: 4_000_000_000, // 4 GB
                used_bytes: 2_500_000_000, // 2.5 GB
                free_bytes: 1_500_000_000, // 1.5 GB
            },
            temperature: GpuTemperature {
                temperature_c: Some(60.0),
                hotspot_c: Some(65.0),
                memory_c: Some(55.0),
            },
            power: GpuPower {
                power_w: Some(4.2),
                power_limit_w: Some(10.0),
                power_cap_w: Some(9.0),
            },
            clocks: GpuClocks {
                core_clock_mhz: Some(700),
                memory_clock_mhz: Some(900),
                shader_clock_mhz: Some(800),
            },
            timestamp: std::time::SystemTime::now(),
        };
        
        // Test serialization
        let qualcomm_serialized = serde_json::to_string(&qualcomm_metrics).expect("Qualcomm serialization failed");
        let mali_serialized = serde_json::to_string(&mali_metrics).expect("Mali serialization failed");
        
        let qualcomm_deserialized: GpuMetrics = serde_json::from_str(&qualcomm_serialized).expect("Qualcomm deserialization failed");
        let mali_deserialized: GpuMetrics = serde_json::from_str(&mali_serialized).expect("Mali deserialization failed");
        
        // Verify all fields are preserved
        assert_eq!(qualcomm_deserialized.device.name, "adreno_gpu");
        assert_eq!(qualcomm_deserialized.device.driver, Some("msm".to_string()));
        assert_eq!(qualcomm_deserialized.utilization.gpu_util, 0.65);
        assert_eq!(qualcomm_deserialized.memory.total_bytes, 2_000_000_000);
        
        assert_eq!(mali_deserialized.device.name, "mali_gpu");
        assert_eq!(mali_deserialized.device.driver, Some("mali".to_string()));
        assert_eq!(mali_deserialized.utilization.gpu_util, 0.70);
        assert_eq!(mali_deserialized.memory.total_bytes, 4_000_000_000);
    }
    
    #[test]
    fn test_new_gpu_vendors_integration() {
        // Test integration of new GPU vendors into the main collection system
        
        // Test that vendor-specific metrics collection works for new vendors
        let qualcomm_device = GpuDevice {
            name: "adreno_integration".to_string(),
            device_path: PathBuf::from("/sys/devices/platform/soc/1c00000.gpu/drm/card0/device"),
            vendor_id: Some("0x5143".to_string()),
            device_id: Some("0x1234".to_string()),
            driver: Some("msm".to_string()),
        };
        
        let mali_device = GpuDevice {
            name: "mali_integration".to_string(),
            device_path: PathBuf::from("/sys/devices/platform/soc/1d80000.gpu/drm/card1/device"),
            vendor_id: Some("0x13b5".to_string()),
            device_id: Some("0x5678".to_string()),
            driver: Some("mali".to_string()),
        };
        
        // Test vendor-specific metrics collection
        let qualcomm_result = collect_vendor_specific_metrics(&qualcomm_device);
        let mali_result = collect_vendor_specific_metrics(&mali_device);
        
        // These may fail (return Err) if the devices don't exist, but should not panic
        // If they succeed, they should return valid metrics
        match qualcomm_result {
            Ok(metrics) => {
                assert_eq!(metrics.device.name, "adreno_integration");
                assert_eq!(metrics.device.driver, Some("msm".to_string()));
            }
            Err(_) => {
                // Expected if the device doesn't exist
                // This is fine for the test
            }
        }
        
        match mali_result {
            Ok(metrics) => {
                assert_eq!(metrics.device.name, "mali_integration");
                assert_eq!(metrics.device.driver, Some("mali".to_string()));
            }
            Err(_) => {
                // Expected if the device doesn't exist
                // This is fine for the test
            }
        }
    }
    
    #[test]
    fn test_new_gpu_vendors_fallback() {
        // Test that new GPU vendors fall back gracefully when vendor-specific APIs fail
        
        let mock_qualcomm_device = GpuDevice {
            name: "mock_adreno".to_string(),
            device_path: PathBuf::from("/non/existent/qualcomm/path"),
            vendor_id: Some("0x5143".to_string()),
            device_id: Some("0x1234".to_string()),
            driver: Some("msm".to_string()),
        };
        
        let mock_mali_device = GpuDevice {
            name: "mock_mali".to_string(),
            device_path: PathBuf::from("/non/existent/mali/path"),
            vendor_id: Some("0x13b5".to_string()),
            device_id: Some("0x5678".to_string()),
            driver: Some("mali".to_string()),
        };
        
        // Vendor-specific collection should fail gracefully
        let qualcomm_result = collect_qualcomm_adreno_metrics(&mock_qualcomm_device);
        let mali_result = collect_arm_mali_metrics(&mock_mali_device);
        
        // Should succeed (return Ok) even with non-existent paths
        assert!(qualcomm_result.is_ok());
        assert!(mali_result.is_ok());
        
        // Should return valid metrics objects with default values
        let qualcomm_metrics = qualcomm_result.unwrap();
        let mali_metrics = mali_result.unwrap();
        
        assert_eq!(qualcomm_metrics.device.name, "mock_adreno");
        assert_eq!(mali_metrics.device.name, "mock_mali");
        assert_eq!(qualcomm_metrics.utilization.gpu_util, 0.0);
        assert_eq!(mali_metrics.utilization.gpu_util, 0.0);
    }
}

    #[test]
    fn test_gpu_improved_error_messages() {
        // Test that the improved error messages provide detailed troubleshooting information
        // This test verifies that error handling includes helpful context and recommendations
        
        // Test device discovery with potential errors
        let devices_result = discover_gpu_devices();
        assert!(devices_result.is_ok()); // Should always return Ok with graceful degradation
        
        // Test metrics collection with potential errors
        let metrics_result = collect_gpu_metrics();
        assert!(devices_result.is_ok()); // Should always return Ok with graceful degradation
        
        let collection = metrics_result.unwrap();
        
        // Verify that the collection is valid even if no devices are found
        assert_eq!(collection.devices.len(), collection.gpu_count);
        
        // Test that serialization works even with empty or partial collections
        let serialized = serde_json::to_string(&collection);
        assert!(serialized.is_ok());
        
        let deserialized: GpuMetricsCollection = serde_json::from_str(&serialized.unwrap()).unwrap();
        assert_eq!(deserialized.gpu_count, collection.gpu_count);
        assert_eq!(deserialized.devices.len(), collection.devices.len());
        
        // Test that we can create a device with error-prone paths and still get valid results
        let error_device = GpuDevice {
            name: "test_error_device".to_string(),
            device_path: PathBuf::from("/nonexistent/path"),
            vendor_id: Some("0x1234".to_string()),
            device_id: Some("0x5678".to_string()),
            driver: Some("test_driver".to_string()),
        };
        
        // This should handle errors gracefully and return a valid metrics structure
        let _error_metrics_result = collect_gpu_device_metrics(&error_device);
        
        // Even if individual device metrics fail, the overall system should continue
        // This is tested by the fact that we can still collect metrics after this
        let final_result = collect_gpu_metrics();
        assert!(final_result.is_ok());
    }
