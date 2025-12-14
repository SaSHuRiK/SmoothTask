//! Hardware Acceleration Monitoring
//!
//! This module provides functionality for monitoring hardware acceleration APIs:
//! - VA-API (Video Acceleration API) for video decoding/encoding
//! - VDPAU (Video Decode and Presentation API for Unix) for video playback
//! - CUDA for GPU computing acceleration

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::warn;

/// Hardware acceleration API type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HardwareAccelerationApi {
    /// Video Acceleration API
    VaApi,
    /// Video Decode and Presentation API for Unix
    Vdpau,
    /// CUDA GPU computing acceleration
    Cuda,
    /// Unknown or unsupported API
    Unknown(String),
}

/// Hardware acceleration device information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardwareAccelerationDevice {
    /// API type
    pub api: HardwareAccelerationApi,
    /// Device name
    pub device_name: String,
    /// Device path
    pub device_path: PathBuf,
    /// Driver version (if available)
    pub driver_version: Option<String>,
    /// Supported profiles (if available)
    pub supported_profiles: Vec<String>,
    /// Supported entrypoints (if available)
    pub supported_entrypoints: Vec<String>,
}

impl Default for HardwareAccelerationDevice {
    fn default() -> Self {
        Self {
            api: HardwareAccelerationApi::Unknown("unknown".to_string()),
            device_name: String::new(),
            device_path: PathBuf::new(),
            driver_version: None,
            supported_profiles: Vec::new(),
            supported_entrypoints: Vec::new(),
        }
    }
}

/// Hardware acceleration usage metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HardwareAccelerationUsage {
    /// Number of active sessions
    pub active_sessions: u32,
    /// Total memory usage in bytes
    pub memory_usage_bytes: u64,
    /// GPU utilization percentage (0.0 to 1.0)
    pub gpu_utilization: f32,
    /// Video decode utilization percentage (0.0 to 1.0)
    pub video_decode_utilization: Option<f32>,
    /// Video encode utilization percentage (0.0 to 1.0)
    pub video_encode_utilization: Option<f32>,
    /// Last activity timestamp
    pub last_activity_timestamp: Option<u64>,
}

/// VA-API specific metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct VaApiMetrics {
    /// VA-API driver version
    pub driver_version: Option<String>,
    /// Supported VA profiles
    pub profiles: Vec<String>,
    /// Supported VA entrypoints
    pub entrypoints: Vec<String>,
    /// Active VA contexts
    pub active_contexts: u32,
    /// VA-API usage metrics
    pub usage: HardwareAccelerationUsage,
}

/// VDPAU specific metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct VdpauMetrics {
    /// VDPAU driver version
    pub driver_version: Option<String>,
    /// Supported VDPAU features
    pub features: Vec<String>,
    /// Active VDPAU sessions
    pub active_sessions: u32,
    /// VDPAU usage metrics
    pub usage: HardwareAccelerationUsage,
}

/// CUDA specific metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CudaMetrics {
    /// CUDA version
    pub cuda_version: Option<String>,
    /// Driver version
    pub driver_version: Option<String>,
    /// Number of CUDA devices
    pub device_count: u32,
    /// CUDA usage metrics
    pub usage: HardwareAccelerationUsage,
    /// CUDA memory metrics
    pub memory_total: Option<u64>,
    pub memory_used: Option<u64>,
    pub memory_free: Option<u64>,
}

/// Comprehensive hardware acceleration metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HardwareAccelerationMetrics {
    /// VA-API metrics (if available)
    pub vaapi: Option<VaApiMetrics>,
    /// VDPAU metrics (if available)
    pub vdpau: Option<VdpauMetrics>,
    /// CUDA metrics (if available)
    pub cuda: Option<CudaMetrics>,
    /// Timestamp of metrics collection
    pub timestamp: u64,
    /// Overall hardware acceleration status
    pub status: HardwareAccelerationStatus,
}

/// Hardware acceleration status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HardwareAccelerationStatus {
    /// All APIs working normally
    Operational,
    /// Some APIs degraded or partially working
    Degraded,
    /// No hardware acceleration available
    Unavailable,
    /// Error occurred during monitoring
    Error(String),
}

impl Default for HardwareAccelerationStatus {
    fn default() -> Self {
        HardwareAccelerationStatus::Unavailable
    }
}

/// Hardware acceleration monitor configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardwareAccelerationMonitorConfig {
    /// Enable VA-API monitoring
    pub enable_vaapi: bool,
    /// Enable VDPAU monitoring
    pub enable_vdpau: bool,
    /// Enable CUDA monitoring
    pub enable_cuda: bool,
    /// Monitoring interval in seconds
    pub monitoring_interval: u64,
    /// Timeout for API calls in milliseconds
    pub api_timeout_ms: u64,
}

impl Default for HardwareAccelerationMonitorConfig {
    fn default() -> Self {
        Self {
            enable_vaapi: true,
            enable_vdpau: true,
            enable_cuda: true,
            monitoring_interval: 60,
            api_timeout_ms: 1000,
        }
    }
}

/// Hardware acceleration monitor
pub struct HardwareAccelerationMonitor {
    config: HardwareAccelerationMonitorConfig,
}

impl HardwareAccelerationMonitor {
    /// Create a new hardware acceleration monitor
    pub fn new(config: HardwareAccelerationMonitorConfig) -> Self {
        Self { config }
    }

    /// Collect hardware acceleration metrics
    pub fn collect_metrics(&self) -> Result<HardwareAccelerationMetrics> {
        let timestamp = chrono::Utc::now().timestamp() as u64;
        let mut metrics = HardwareAccelerationMetrics {
            timestamp,
            ..Default::default()
        };

        let mut status = HardwareAccelerationStatus::Operational;
        let mut has_any_api = false;

        // Collect VA-API metrics if enabled
        if self.config.enable_vaapi {
            match self.collect_vaapi_metrics() {
                Ok(vaapi_metrics) => {
                    metrics.vaapi = Some(vaapi_metrics);
                    has_any_api = true;
                }
                Err(e) => {
                    warn!("Failed to collect VA-API metrics: {}", e);
                    status = HardwareAccelerationStatus::Degraded;
                }
            }
        }

        // Collect VDPAU metrics if enabled
        if self.config.enable_vdpau {
            match self.collect_vdpau_metrics() {
                Ok(vdpau_metrics) => {
                    metrics.vdpau = Some(vdpau_metrics);
                    has_any_api = true;
                }
                Err(e) => {
                    warn!("Failed to collect VDPAU metrics: {}", e);
                    status = HardwareAccelerationStatus::Degraded;
                }
            }
        }

        // Collect CUDA metrics if enabled
        if self.config.enable_cuda {
            match self.collect_cuda_metrics() {
                Ok(cuda_metrics) => {
                    metrics.cuda = Some(cuda_metrics);
                    has_any_api = true;
                }
                Err(e) => {
                    warn!("Failed to collect CUDA metrics: {}", e);
                    status = HardwareAccelerationStatus::Degraded;
                }
            }
        }

        // Set overall status
        if !has_any_api {
            status = HardwareAccelerationStatus::Unavailable;
        }
        metrics.status = status;

        Ok(metrics)
    }

    /// Collect VA-API metrics
    fn collect_vaapi_metrics(&self) -> Result<VaApiMetrics> {
        // Check if VA-API is available by looking for common device files
        let vaapi_devices = vec![
            "/dev/dri/renderD128",
            "/dev/dri/renderD129",
            "/dev/dri/card0",
        ];

        let timestamp = chrono::Utc::now().timestamp() as u64;
        let mut metrics = VaApiMetrics::default();
        let mut found_device = false;

        for device_path in vaapi_devices {
            if Path::new(device_path).exists() {
                found_device = true;
                metrics.driver_version = Some(self.read_driver_version(device_path)?);
                metrics.profiles = self.detect_vaapi_profiles()?;
                metrics.entrypoints = self.detect_vaapi_entrypoints()?;
                metrics.active_contexts = self.count_active_vaapi_contexts()?;
                
                // Set some basic usage metrics (would be enhanced with actual monitoring)
                metrics.usage.active_sessions = metrics.active_contexts;
                metrics.usage.gpu_utilization = 0.0; // Placeholder
                metrics.usage.last_activity_timestamp = Some(timestamp);
                
                break;
            }
        }

        if !found_device {
            return Err(anyhow!("No VA-API devices found"));
        }

        Ok(metrics)
    }

    /// Collect VDPAU metrics
    fn collect_vdpau_metrics(&self) -> Result<VdpauMetrics> {
        // Check if VDPAU is available
        let vdpau_devices = vec![
            "/dev/dri/card0",
            "/dev/dri/card1",
        ];

        let timestamp = chrono::Utc::now().timestamp() as u64;
        let mut metrics = VdpauMetrics::default();
        let mut found_device = false;

        for device_path in vdpau_devices {
            if Path::new(device_path).exists() {
                found_device = true;
                metrics.driver_version = Some(self.read_driver_version(device_path)?);
                metrics.features = self.detect_vdpau_features()?;
                metrics.active_sessions = self.count_active_vdpau_sessions()?;
                
                // Set some basic usage metrics
                metrics.usage.active_sessions = metrics.active_sessions;
                metrics.usage.gpu_utilization = 0.0; // Placeholder
                metrics.usage.last_activity_timestamp = Some(timestamp);
                
                break;
            }
        }

        if !found_device {
            return Err(anyhow!("No VDPAU devices found"));
        }

        Ok(metrics)
    }

    /// Collect CUDA metrics
    fn collect_cuda_metrics(&self) -> Result<CudaMetrics> {
        let timestamp = chrono::Utc::now().timestamp() as u64;
        let mut metrics = CudaMetrics::default();
        
        // Check for CUDA installation
        let cuda_paths = vec![
            "/usr/local/cuda",
            "/usr/lib/cuda",
        ];

        let mut found_cuda = false;
        for cuda_path in cuda_paths {
            if Path::new(cuda_path).exists() {
                found_cuda = true;
                metrics.cuda_version = Some(self.read_cuda_version(cuda_path)?);
                metrics.driver_version = Some(self.read_nvidia_driver_version()?);
                metrics.device_count = self.count_cuda_devices()?;
                
                // Try to get memory info if nvidia-smi is available
                if let Ok(memory_info) = self.get_cuda_memory_info() {
                    metrics.memory_total = Some(memory_info.0);
                    metrics.memory_used = Some(memory_info.1);
                    metrics.memory_free = Some(memory_info.2);
                }
                
                // Set some basic usage metrics
                metrics.usage.active_sessions = 0; // Would need actual monitoring
                metrics.usage.gpu_utilization = 0.0; // Placeholder
                metrics.usage.last_activity_timestamp = Some(timestamp);
                
                break;
            }
        }

        if !found_cuda {
            return Err(anyhow!("CUDA not found"));
        }

        Ok(metrics)
    }

    // Helper methods would be implemented here
    // These are placeholders for the actual implementation

    fn read_driver_version(&self, _device_path: &str) -> Result<String> {
        // In a real implementation, this would read driver version from sysfs or similar
        Ok("unknown".to_string())
    }

    fn detect_vaapi_profiles(&self) -> Result<Vec<String>> {
        // Detect available VA-API profiles
        Ok(vec!["VAProfileMPEG2Simple".to_string(), "VAProfileH264High".to_string()])
    }

    fn detect_vaapi_entrypoints(&self) -> Result<Vec<String>> {
        // Detect available VA-API entrypoints
        Ok(vec!["VLD".to_string(), "IDCT".to_string()])
    }

    fn count_active_vaapi_contexts(&self) -> Result<u32> {
        // Count active VA-API contexts (would need process monitoring)
        Ok(0)
    }

    fn detect_vdpau_features(&self) -> Result<Vec<String>> {
        // Detect available VDPAU features
        Ok(vec!["decode".to_string(), "presentation".to_string()])
    }

    fn count_active_vdpau_sessions(&self) -> Result<u32> {
        // Count active VDPAU sessions
        Ok(0)
    }

    fn read_cuda_version(&self, _cuda_path: &str) -> Result<String> {
        // Read CUDA version
        Ok("11.0".to_string())
    }

    fn read_nvidia_driver_version(&self) -> Result<String> {
        // Read NVIDIA driver version
        Ok("450.80.02".to_string())
    }

    fn count_cuda_devices(&self) -> Result<u32> {
        // Count CUDA devices
        Ok(1)
    }

    fn get_cuda_memory_info(&self) -> Result<(u64, u64, u64)> {
        // Get CUDA memory info using nvidia-smi or similar
        // Returns (total, used, free) in bytes
        Ok((8_589_934_592, 1_073_741_824, 7_516_192_768)) // 8GB total, 1GB used, 7GB free
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_hardware_acceleration_device_default() {
        let device = HardwareAccelerationDevice::default();
        assert_eq!(device.device_name, "");
        assert_eq!(device.supported_profiles.len(), 0);
        assert_eq!(device.supported_entrypoints.len(), 0);
    }

    #[test]
    fn test_hardware_acceleration_usage_default() {
        let usage = HardwareAccelerationUsage::default();
        assert_eq!(usage.active_sessions, 0);
        assert_eq!(usage.memory_usage_bytes, 0);
        assert_eq!(usage.gpu_utilization, 0.0);
    }

    #[test]
    fn test_hardware_acceleration_metrics_default() {
        let metrics = HardwareAccelerationMetrics::default();
        assert!(metrics.vaapi.is_none());
        assert!(metrics.vdpau.is_none());
        assert!(metrics.cuda.is_none());
        assert_eq!(metrics.timestamp, 0);
    }

    #[test]
    fn test_hardware_acceleration_config_default() {
        let config = HardwareAccelerationMonitorConfig::default();
        assert!(config.enable_vaapi);
        assert!(config.enable_vdpau);
        assert!(config.enable_cuda);
        assert_eq!(config.monitoring_interval, 60);
        assert_eq!(config.api_timeout_ms, 1000);
    }

    #[test]
    fn test_hardware_acceleration_monitor_creation() {
        let config = HardwareAccelerationMonitorConfig::default();
        let monitor = HardwareAccelerationMonitor::new(config);
        assert!(monitor.config.enable_vaapi);
    }

    #[test]
    fn test_hardware_acceleration_status_serialization() {
        let status = HardwareAccelerationStatus::Operational;
        let serialized = serde_json::to_string(&status).unwrap();
        assert!(serialized.contains("Operational"));

        let status = HardwareAccelerationStatus::Error("test error".to_string());
        let serialized = serde_json::to_string(&status).unwrap();
        assert!(serialized.contains("test error"));
    }

    #[test]
    fn test_vaapi_metrics_structure() {
        let mut metrics = VaApiMetrics::default();
        metrics.driver_version = Some("1.0.0".to_string());
        metrics.profiles = vec!["H264".to_string(), "HEVC".to_string()];
        metrics.active_contexts = 2;
        
        assert_eq!(metrics.driver_version.unwrap(), "1.0.0");
        assert_eq!(metrics.profiles.len(), 2);
        assert_eq!(metrics.active_contexts, 2);
    }

    #[test]
    fn test_cuda_metrics_structure() {
        let mut metrics = CudaMetrics::default();
        metrics.cuda_version = Some("11.2".to_string());
        metrics.driver_version = Some("460.32.03".to_string());
        metrics.device_count = 2;
        metrics.memory_total = Some(16_000_000_000);
        
        assert_eq!(metrics.cuda_version.unwrap(), "11.2");
        assert_eq!(metrics.driver_version.unwrap(), "460.32.03");
        assert_eq!(metrics.device_count, 2);
        assert_eq!(metrics.memory_total.unwrap(), 16_000_000_000);
    }

    #[test]
    fn test_metrics_collection_with_disabled_apis() {
        let mut config = HardwareAccelerationMonitorConfig::default();
        config.enable_vaapi = false;
        config.enable_vdpau = false;
        config.enable_cuda = false;
        
        let monitor = HardwareAccelerationMonitor::new(config);
        let result = monitor.collect_metrics();
        
        // Should return Unavailable status when all APIs are disabled
        assert!(result.is_ok());
        let metrics = result.unwrap();
        match metrics.status {
            HardwareAccelerationStatus::Unavailable => {},
            _ => panic!("Expected Unavailable status"),
        }
    }

    #[test]
    fn test_timestamp_in_metrics() {
        let config = HardwareAccelerationMonitorConfig::default();
        let monitor = HardwareAccelerationMonitor::new(config);
        let result = monitor.collect_metrics();
        
        if let Ok(metrics) = result {
            // Timestamp should be set and reasonable (within last minute)
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            assert!(metrics.timestamp > 0);
            assert!(metrics.timestamp <= current_time + 60);
        }
    }
}
