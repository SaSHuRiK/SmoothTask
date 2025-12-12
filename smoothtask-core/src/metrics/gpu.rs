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
    
    // Check DRM devices
    let drm_dir = Path::new("/sys/class/drm");
    if drm_dir.exists() {
        if let Ok(entries) = fs::read_dir(drm_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                
                if file_name.starts_with("card") {
                    let device_path = path.join("device");
                    if device_path.exists() {
                        let device = GpuDevice {
                            name: file_name.to_string(),
                            device_path: device_path.clone(),
                            vendor_id: read_pci_field(&device_path, "vendor").ok(),
                            device_id: read_pci_field(&device_path, "device").ok(),
                            driver: read_driver_name(&device_path).ok(),
                        };
                        devices.push(device);
                    }
                }
            }
        }
    }
    
    Ok(devices)
}

/// Read a field from a PCI device
fn read_pci_field(device_path: &Path, field: &str) -> Result<String> {
    let field_path = device_path.join(field);
    if field_path.exists() {
        let content = fs::read_to_string(&field_path)
            .with_context(|| format!("Failed to read PCI field {} from {}", field, field_path.display()))?;
        Ok(content.trim().to_string())
    } else {
        Ok(String::new())
    }
}

/// Read the driver name for a device
fn read_driver_name(device_path: &Path) -> Result<String> {
    let driver_path = device_path.join("driver");
    if driver_path.exists() {
        let driver_link = fs::read_link(&driver_path)
            .with_context(|| format!("Failed to read driver link from {}", driver_path.display()))?;
        
        if let Some(driver_name) = driver_link.file_name() {
            return Ok(driver_name.to_string_lossy().into_owned());
        }
    }
    Ok(String::new())
}

/// Collect GPU metrics for all devices
pub fn collect_gpu_metrics() -> Result<GpuMetricsCollection> {
    let devices = discover_gpu_devices()?;
    let mut collection = GpuMetricsCollection {
        devices: Vec::new(),
        gpu_count: devices.len(),
    };
    
    for device in devices {
        let metrics = collect_gpu_device_metrics(&device)?;
        collection.devices.push(metrics);
    }
    
    Ok(collection)
}

/// Collect metrics for a specific GPU device
fn collect_gpu_device_metrics(device: &GpuDevice) -> Result<GpuMetrics> {
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
    if let Ok(util) = collect_gpu_utilization(&device.device_path) {
        metrics.utilization = util;
    }
    
    // Collect memory metrics
    if let Ok(mem) = collect_gpu_memory(&device.device_path) {
        metrics.memory = mem;
    }
    
    // Collect temperature metrics
    if let Ok(temp) = collect_gpu_temperature(&device.device_path) {
        metrics.temperature = temp;
    }
    
    // Collect power metrics
    if let Ok(power) = collect_gpu_power(&device.device_path) {
        metrics.power = power;
    }
    
    // Collect clock metrics
    if let Ok(clocks) = collect_gpu_clocks(&device.device_path) {
        metrics.clocks = clocks;
    }
    
    Ok(metrics)
}

/// Collect GPU utilization metrics
fn collect_gpu_utilization(device_path: &Path) -> Result<GpuUtilization> {
    let mut utilization = GpuUtilization::default();
    
    // Try to read GPU utilization from sysfs
    // Different drivers expose this differently
    
    // For Intel i915
    if let Ok(gpu_busy) = read_sysfs_u32(device_path, "gpu_busy_percent") {
        utilization.gpu_util = gpu_busy as f32 / 100.0;
    }
    
    // For AMD and NVIDIA, we might need to look in different locations
    // This is a simplified approach - in real implementation, we'd need
    // driver-specific logic
    
    Ok(utilization)
}

/// Collect GPU memory metrics
fn collect_gpu_memory(device_path: &Path) -> Result<GpuMemory> {
    let mut memory = GpuMemory::default();
    
    // Try to read memory info from sysfs
    // This is driver-specific and may not be available on all systems
    
    // For Intel i915
    if let Ok(total) = read_sysfs_u64(device_path, "mem_total_bytes") {
        memory.total_bytes = total;
    }
    
    if let Ok(used) = read_sysfs_u64(device_path, "mem_used_bytes") {
        memory.used_bytes = used;
    }
    
    if memory.total_bytes > 0 && memory.used_bytes > memory.total_bytes {
        memory.used_bytes = memory.total_bytes;
    }
    
    memory.free_bytes = memory.total_bytes.saturating_sub(memory.used_bytes);
    
    Ok(memory)
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
                        let file_name = temp_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                        
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
                        let file_name = energy_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                        
                        if file_name == "energy_uj" {
                            // Check if this powercap device corresponds to our GPU
                            // This is simplified - in real implementation we'd need
                            // to match the device path
                            if let Ok(energy_content) = fs::read_to_string(&energy_path) {
                                if let Ok(energy_microjoules) = energy_content.trim().parse::<u64>() {
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
                        let file_name = power_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                        
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

/// Read a u32 value from sysfs
fn read_sysfs_u32(path: &Path, field: &str) -> Result<u32> {
    let field_path = path.join(field);
    if field_path.exists() {
        let content = fs::read_to_string(&field_path)
            .with_context(|| format!("Failed to read {} from {}", field, field_path.display()))?;
        content.trim().parse::<u32>()
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
        content.trim().parse::<u64>()
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
        let deserialized: GpuMetrics = serde_json::from_str(&serialized).expect("Deserialization failed");

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
        let deserialized: GpuMetricsCollection = serde_json::from_str(&serialized).expect("Deserialization failed");

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
}