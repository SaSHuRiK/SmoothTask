//! Native Prometheus Exporter Module
//!
//! This module provides comprehensive native support for exporting metrics
//! in Prometheus format. It includes:
//! - System metrics export
//! - Process metrics export
//! - Network metrics export
//! - GPU metrics export
//! - Custom metrics export
//! - ML performance metrics export
//!
//! The exporter follows Prometheus best practices and provides a complete
//! implementation of the Prometheus text-based exposition format.

use anyhow::Result;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::metrics::system::SystemMetrics;
use crate::logging::snapshots::ProcessRecord;
use crate::metrics::network::NetworkInterfaceStats;

/// Simple process metrics structure for Prometheus export
#[derive(Debug, Clone, Default)]
pub struct ProcessMetrics {
    pub pid: i32,
    pub exe: Option<String>,
    pub cpu_usage_percent: f64,
    pub memory_rss_bytes: u64,
    pub memory_swap_bytes: u64,
    pub threads: u32,
    pub open_fds: u32,
}

impl From<&ProcessRecord> for ProcessMetrics {
    fn from(record: &ProcessRecord) -> Self {
        ProcessMetrics {
            pid: record.pid,
            exe: record.exe.clone(),
            cpu_usage_percent: record.cpu_share_1s.unwrap_or(0.0),
            memory_rss_bytes: record.rss_mb.unwrap_or(0) * 1024 * 1024, // Convert MB to bytes
            memory_swap_bytes: record.swap_mb.unwrap_or(0) * 1024 * 1024, // Convert MB to bytes
            threads: 1, // Default value, could be enhanced with actual thread count
            open_fds: 0, // Default value, could be enhanced with actual FD count
        }
    }
}
use crate::metrics::gpu::GpuMetricsCollection;
use crate::metrics::ml_performance::ml_metrics_to_prometheus;
use crate::classify::ml_classifier::MLPerformanceMetrics;

/// Prometheus Exporter Configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PrometheusExporterConfig {
    /// Enable system metrics export
    pub enable_system_metrics: bool,
    /// Enable process metrics export
    pub enable_process_metrics: bool,
    /// Enable network metrics export
    pub enable_network_metrics: bool,
    /// Enable GPU metrics export
    pub enable_gpu_metrics: bool,
    /// Enable ML metrics export
    pub enable_ml_metrics: bool,
    /// Enable custom metrics export
    pub enable_custom_metrics: bool,
    /// Include detailed process metrics
    pub include_detailed_process_metrics: bool,
    /// Maximum number of processes to export
    pub max_processes_to_export: usize,
    /// Include timestamps in metrics
    pub include_timestamps: bool,
    /// Include help text in output
    pub include_help_text: bool,
}

impl Default for PrometheusExporterConfig {
    fn default() -> Self {
        Self {
            enable_system_metrics: true,
            enable_process_metrics: true,
            enable_network_metrics: true,
            enable_gpu_metrics: true,
            enable_ml_metrics: true,
            enable_custom_metrics: true,
            include_detailed_process_metrics: false,
            max_processes_to_export: 100,
            include_timestamps: true,
            include_help_text: true,
        }
    }
}

/// Main Prometheus Exporter Structure
pub struct PrometheusExporter {
    config: PrometheusExporterConfig,
    custom_metrics: HashMap<String, f64>,
}

impl PrometheusExporter {
    /// Create a new PrometheusExporter with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new PrometheusExporter with custom configuration
    pub fn with_config(config: PrometheusExporterConfig) -> Self {
        Self {
            config,
            custom_metrics: HashMap::new(),
        }
    }

    /// Add a custom metric to be exported
    pub fn add_custom_metric(&mut self, name: &str, value: f64) {
        self.custom_metrics.insert(name.to_string(), value);
    }

    /// Remove a custom metric
    pub fn remove_custom_metric(&mut self, name: &str) {
        self.custom_metrics.remove(name);
    }

    /// Clear all custom metrics
    pub fn clear_custom_metrics(&mut self) {
        self.custom_metrics.clear();
    }

    /// Export all metrics in Prometheus format
    pub fn export_all_metrics(
        &self,
        system_metrics: &SystemMetrics,
        process_metrics: &[ProcessRecord],
        network_metrics: &[NetworkInterfaceStats],
        gpu_metrics: &GpuMetricsCollection,
        ml_metrics: &HashMap<String, MLPerformanceMetrics>,
    ) -> Result<String> {
        let mut output = String::new();

        // Add header if help text is enabled
        if self.config.include_help_text {
            output.push_str("# HELP smoothtask_metrics SmoothTask system and process metrics\n");
            output.push_str("# TYPE smoothtask_metrics gauge\n");
        }

        // Export system metrics
        if self.config.enable_system_metrics {
            self.export_system_metrics(system_metrics, &mut output)?;
        }

        // Export process metrics
        if self.config.enable_process_metrics {
            self.export_process_metrics(process_metrics, &mut output)?;
        }

        // Export network metrics
        if self.config.enable_network_metrics {
            self.export_network_metrics(network_metrics, &mut output)?;
        }

        // Export GPU metrics
        if self.config.enable_gpu_metrics {
            self.export_gpu_metrics(gpu_metrics, &mut output)?;
        }

        // Export ML metrics
        if self.config.enable_ml_metrics {
            self.export_ml_metrics(ml_metrics, &mut output)?;
        }

        // Export custom metrics
        if self.config.enable_custom_metrics {
            self.export_custom_metrics(&mut output)?;
        }

        // Add timestamp if enabled
        if self.config.include_timestamps {
            if let Ok(timestamp) = SystemTime::now().duration_since(UNIX_EPOCH) {
                output.push_str(&format!("# smoothtask_export_timestamp {}\n", timestamp.as_secs()));
            }
        }

        Ok(output)
    }

    /// Export system metrics in Prometheus format
    fn export_system_metrics(&self, metrics: &SystemMetrics, output: &mut String) -> Result<()> {
        // CPU metrics
        output.push_str(&format!(
            "system_cpu_usage_percent{{type=\"user\"}} {}\n",
            metrics.cpu_times.user
        ));
        output.push_str(&format!(
            "system_cpu_usage_percent{{type=\"system\"}} {}\n",
            metrics.cpu_times.system
        ));
        output.push_str(&format!(
            "system_cpu_usage_percent{{type=\"idle\"}} {}\n",
            metrics.cpu_times.idle
        ));
        output.push_str(&format!(
            "system_cpu_usage_percent{{type=\"iowait\"}} {}\n",
            metrics.cpu_times.iowait
        ));

        // Memory metrics
        output.push_str(&format!(
            "system_memory_bytes{{type=\"total\"}} {}\n",
            metrics.memory.mem_total_kb * 1024
        ));
        output.push_str(&format!(
            "system_memory_bytes{{type=\"free\"}} {}\n",
            metrics.memory.mem_free_kb * 1024
        ));
        output.push_str(&format!(
            "system_memory_bytes{{type=\"available\"}} {}\n",
            metrics.memory.mem_available_kb * 1024
        ));
        output.push_str(&format!(
            "system_memory_bytes{{type=\"used\"}} {}\n",
            (metrics.memory.mem_total_kb - metrics.memory.mem_free_kb) * 1024
        ));

        // Load average
        output.push_str(&format!(
            "system_load_average{{period=\"1min\"}} {}\n",
            metrics.load_avg.one
        ));
        output.push_str(&format!(
            "system_load_average{{period=\"5min\"}} {}\n",
            metrics.load_avg.five
        ));
        output.push_str(&format!(
            "system_load_average{{period=\"15min\"}} {}\n",
            metrics.load_avg.fifteen
        ));

        // Disk metrics
        for disk_device in &metrics.disk.devices {
            output.push_str(&format!(
                "system_disk_io_bytes{{device=\"{}\",type=\"read\"}} {}\\n",
                disk_device.name, disk_device.read_bytes
            ));
            output.push_str(&format!(
                "system_disk_io_bytes{{device=\"{}\",type=\"write\"}} {}\\n",
                disk_device.name, disk_device.write_bytes
            ));
            output.push_str(&format!(
                "system_disk_io_ops{{device=\"{}\",type=\"read\"}} {}\\n",
                disk_device.name, disk_device.read_ops
            ));
            output.push_str(&format!(
                "system_disk_io_ops{{device=\"{}\",type=\"write\"}} {}\\n",
                disk_device.name, disk_device.write_ops
            ));
            output.push_str(&format!(
                "system_disk_io_time_ms{{device=\"{}\"}} {}\\n",
                disk_device.name, disk_device.io_time
            ));
        }

        Ok(())
    }

    /// Export process metrics in Prometheus format
    fn export_process_metrics(&self, metrics: &[ProcessRecord], output: &mut String) -> Result<()> {
        let limit = std::cmp::min(self.config.max_processes_to_export, metrics.len());
        
        for process in metrics.iter().take(limit) {
            // Basic process metrics
            output.push_str(&format!(
                "process_cpu_usage_percent{{pid=\"{}\",name=\"{}\"}} {}\n",
                process.pid, 
                process.exe.as_deref().unwrap_or("unknown"),
                process.cpu_share_1s.unwrap_or(0.0)
            ));

            output.push_str(&format!(
                "process_memory_bytes{{pid=\"{}\",name=\"{}\",type=\"rss\"}} {}\n",
                process.pid,
                process.exe.as_deref().unwrap_or("unknown"),
                process.rss_mb.unwrap_or(0) * 1024 * 1024
            ));

            output.push_str(&format!(
                "process_memory_bytes{{pid=\"{}\",name=\"{}\",type=\"vms\"}} {}\n",
                process.pid,
                process.exe.as_deref().unwrap_or("unknown"),
                process.swap_mb.unwrap_or(0) * 1024 * 1024
            ));

            // Detailed metrics if enabled
            if self.config.include_detailed_process_metrics {
                output.push_str(&format!(
                    "process_threads{{pid=\"{}\",name=\"{}\"}} {}\n",
                    process.pid,
                    process.exe.as_deref().unwrap_or("unknown"),
                    1 // Default thread count - could be enhanced
                ));

                output.push_str(&format!(
                    "process_fd_count{{pid=\"{}\",name=\"{}\"}} {}\n",
                    process.pid,
                    process.exe.as_deref().unwrap_or("unknown"),
                    0 // Default FD count - could be enhanced
                ));
            }
        }

        Ok(())
    }

    /// Export network metrics in Prometheus format
    fn export_network_metrics(&self, metrics: &[NetworkInterfaceStats], output: &mut String) -> Result<()> {
        for interface in metrics {
            output.push_str(&format!(
                "network_bytes{{interface=\"{}\",direction=\"rx\"}} {}\n",
                interface.name,
                interface.rx_bytes
            ));

            output.push_str(&format!(
                "network_bytes{{interface=\"{}\",direction=\"tx\"}} {}\n",
                interface.name,
                interface.tx_bytes
            ));

            output.push_str(&format!(
                "network_packets{{interface=\"{}\",direction=\"rx\"}} {}\n",
                interface.name,
                interface.rx_packets
            ));

            output.push_str(&format!(
                "network_packets{{interface=\"{}\",direction=\"tx\"}} {}\n",
                interface.name,
                interface.tx_packets
            ));

            output.push_str(&format!(
                "network_errors{{interface=\"{}\",direction=\"rx\"}} {}\n",
                interface.name,
                interface.rx_errors
            ));

            output.push_str(&format!(
                "network_errors{{interface=\"{}\",direction=\"tx\"}} {}\n",
                interface.name,
                interface.tx_errors
            ));
        }

        Ok(())
    }

    /// Export GPU metrics in Prometheus format
    fn export_gpu_metrics(&self, metrics: &GpuMetricsCollection, output: &mut String) -> Result<()> {
        for gpu_metrics in &metrics.devices {
            output.push_str(&format!(
                "gpu_usage_percent{{gpu=\"{}\"}} {}\n",
                gpu_metrics.device.name,
                gpu_metrics.utilization.gpu_util * 100.0
            ));

            output.push_str(&format!(
                "gpu_memory_bytes{{gpu=\"{}\",type=\"total\"}} {}\n",
                gpu_metrics.device.name,
                gpu_metrics.memory.total_bytes
            ));

            output.push_str(&format!(
                "gpu_memory_bytes{{gpu=\"{}\",type=\"used\"}} {}\n",
                gpu_metrics.device.name,
                gpu_metrics.memory.used_bytes
            ));

            output.push_str(&format!(
                "gpu_temperature_celsius{{gpu=\"{}\"}} {}\n",
                gpu_metrics.device.name,
                gpu_metrics.temperature.temperature_c.unwrap_or(0.0)
            ));

            output.push_str(&format!(
                "gpu_power_watts{{gpu=\"{}\"}} {}\n",
                gpu_metrics.device.name,
                gpu_metrics.power.power_w.unwrap_or(0.0)
            ));
        }

        Ok(())
    }

    /// Export ML metrics in Prometheus format
    fn export_ml_metrics(&self, metrics: &HashMap<String, MLPerformanceMetrics>, output: &mut String) -> Result<()> {
        for (model_name, model_metrics) in metrics {
            // Use the existing ML metrics to Prometheus function
            let ml_output = ml_metrics_to_prometheus(model_metrics, model_name);
            output.push_str(&ml_output);
        }

        Ok(())
    }

    /// Export custom metrics in Prometheus format
    fn export_custom_metrics(&self, output: &mut String) -> Result<()> {
        for (name, value) in &self.custom_metrics {
            // Sanitize metric name for Prometheus
            let sanitized_name = self.sanitize_metric_name(name);
            output.push_str(&format!("custom_metric{{name=\"{}\"}} {}\n", sanitized_name, value));
        }

        Ok(())
    }

    /// Sanitize metric name for Prometheus compatibility
    fn sanitize_metric_name(&self, name: &str) -> String {
        name.chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
            .collect::<String>()
    }

    /// Export metrics for a specific process in Prometheus format
    pub fn export_process_metrics_prometheus(&self, metrics: &ProcessRecord) -> Result<String> {
        let mut output = String::new();

        if self.config.include_help_text {
            output.push_str("# HELP process_metrics Process metrics\n");
            output.push_str("# TYPE process_metrics gauge\n");
        }

        // Basic process metrics
        output.push_str(&format!(
            "process_cpu_usage_percent{{pid=\"{}\",name=\"{}\"}} {}\n",
            metrics.pid,
            metrics.exe.as_deref().unwrap_or("unknown"),
            metrics.cpu_share_1s.unwrap_or(0.0)
        ));

        output.push_str(&format!(
            "process_memory_bytes{{pid=\"{}\",name=\"{}\",type=\"rss\"}} {}\n",
            metrics.pid,
            metrics.exe.as_deref().unwrap_or("unknown"),
            metrics.rss_mb.unwrap_or(0) * 1024 * 1024
        ));

        output.push_str(&format!(
            "process_memory_bytes{{pid=\"{}\",name=\"{}\",type=\"vms\"}} {}\n",
            metrics.pid,
            metrics.exe.as_deref().unwrap_or("unknown"),
            metrics.swap_mb.unwrap_or(0) * 1024 * 1024
        ));

        // Detailed metrics if enabled
        if self.config.include_detailed_process_metrics {
            output.push_str(&format!(
                "process_threads{{pid=\"{}\",name=\"{}\"}} {}\n",
                metrics.pid,
                metrics.exe.as_deref().unwrap_or("unknown"),
                1
            ));

            output.push_str(&format!(
                "process_fd_count{{pid=\"{}\",name=\"{}\"}} {}\n",
                metrics.pid,
                metrics.exe.as_deref().unwrap_or("unknown"),
                0
            ));

            output.push_str(&format!(
                "process_cpu_time_seconds{{pid=\"{}\",name=\"{}\",type=\"user\"}} {}\n",
                metrics.pid,
                metrics.exe.as_deref().unwrap_or("unknown"),
                0
            ));

            output.push_str(&format!(
                "process_cpu_time_seconds{{pid=\"{}\",name=\"{}\",type=\"system\"}} {}\n",
                metrics.pid,
                metrics.exe.as_deref().unwrap_or("unknown"),
                0
            ));
        }

        Ok(output)
    }

    /// Export system metrics in Prometheus format (standalone)
    pub fn export_system_metrics_prometheus(&self, metrics: &SystemMetrics) -> Result<String> {
        let mut output = String::new();

        if self.config.include_help_text {
            output.push_str("# HELP system_metrics System metrics\n");
            output.push_str("# TYPE system_metrics gauge\n");
        }

        self.export_system_metrics(metrics, &mut output)?;

        Ok(output)
    }

    /// Export network metrics in Prometheus format (standalone)
    pub fn export_network_metrics_prometheus(&self, metrics: &[NetworkInterfaceStats]) -> Result<String> {
        let mut output = String::new();

        if self.config.include_help_text {
            output.push_str("# HELP network_metrics Network metrics\n");
            output.push_str("# TYPE network_metrics gauge\n");
        }

        self.export_network_metrics(metrics, &mut output)?;

        Ok(output)
    }

    /// Export GPU metrics in Prometheus format (standalone)
    pub fn export_gpu_metrics_prometheus(&self, metrics: &GpuMetricsCollection) -> Result<String> {
        let mut output = String::new();

        if self.config.include_help_text {
            output.push_str("# HELP gpu_metrics GPU metrics\n");
            output.push_str("# TYPE gpu_metrics gauge\n");
        }

        self.export_gpu_metrics(metrics, &mut output)?;

        Ok(output)
    }

    /// Export metrics in Prometheus format with HTTP headers
    pub fn export_with_http_headers(&self, prometheus_output: &str) -> String {
        let mut result = String::new();
        
        // Add HTTP headers
        result.push_str("HTTP/1.1 200 OK\r\n");
        result.push_str("Content-Type: text/plain; version=0.0.4; charset=utf-8\r\n");
        result.push_str(&format!("Content-Length: {}\r\n", prometheus_output.len()));
        result.push_str("\r\n");
        
        // Add the actual Prometheus output
        result.push_str(prometheus_output);
        
        result
    }

    /// Create a simple HTTP response for Prometheus scraping
    pub fn create_prometheus_http_response(
        &self,
        system_metrics: &SystemMetrics,
        process_metrics: &[ProcessRecord],
        network_metrics: &[NetworkInterfaceStats],
        gpu_metrics: &GpuMetricsCollection,
        ml_metrics: &HashMap<String, MLPerformanceMetrics>,
    ) -> Result<String> {
        let prometheus_output = self.export_all_metrics(
            system_metrics, process_metrics, network_metrics, gpu_metrics, ml_metrics
        )?;
        
        Ok(self.export_with_http_headers(&prometheus_output))
    }
}

impl Default for PrometheusExporter {
    fn default() -> Self {
        Self {
            config: PrometheusExporterConfig::default(),
            custom_metrics: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_prometheus_exporter_creation() {
        let exporter = PrometheusExporter::new();
        assert!(matches!(exporter.config.enable_system_metrics, true));
        assert!(matches!(exporter.config.enable_process_metrics, true));
        assert!(matches!(exporter.config.enable_network_metrics, true));
    }

    #[test]
    fn test_custom_metric_management() {
        let mut exporter = PrometheusExporter::new();
        
        // Add custom metrics
        exporter.add_custom_metric("test_metric_1", 42.0);
        exporter.add_custom_metric("test_metric_2", 100.5);
        
        assert_eq!(exporter.custom_metrics.len(), 2);
        assert_eq!(exporter.custom_metrics["test_metric_1"], 42.0);
        
        // Remove a metric
        exporter.remove_custom_metric("test_metric_1");
        assert_eq!(exporter.custom_metrics.len(), 1);
        
        // Clear all metrics
        exporter.clear_custom_metrics();
        assert_eq!(exporter.custom_metrics.len(), 0);
    }

    #[test]
    fn test_metric_name_sanitization() {
        let exporter = PrometheusExporter::new();
        
        let sanitized = exporter.sanitize_metric_name("test-metric.name");
        assert_eq!(sanitized, "test_metric_name");
        
        let sanitized = exporter.sanitize_metric_name("metric with spaces");
        assert_eq!(sanitized, "metric_with_spaces");
    }

    #[test]
    fn test_system_metrics_export() {
        let exporter = PrometheusExporter::new();
        let mut system_metrics = SystemMetrics::default();
        
        // Set some test values
        system_metrics.cpu.user_percent = 30.5;
        system_metrics.cpu.system_percent = 15.2;
        system_metrics.memory.total_bytes = 16 * 1024 * 1024 * 1024; // 16 GB
        system_metrics.memory.used_bytes = 8 * 1024 * 1024 * 1024; // 8 GB
        
        let result = exporter.export_system_metrics_prometheus(&system_metrics);
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert!(output.contains("system_cpu_usage_percent"));
        assert!(output.contains("system_memory_bytes"));
        assert!(output.contains("30.5"));
        assert!(output.contains("15.2"));
    }

    #[test]
    fn test_process_metrics_export() {
        let exporter = PrometheusExporter::new();
        let mut process_metrics = ProcessMetrics::default();
        
        // Set some test values
        process_metrics.pid = 1234;
        process_metrics.exe = Some("test_process".to_string());
        process_metrics.cpu_usage_percent = 25.5;
        process_metrics.memory_rss_bytes = 1024 * 1024; // 1 MB
        process_1 = 4;
        
        let result = exporter.export_process_metrics_prometheus(&process_metrics);
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert!(output.contains("process_cpu_usage_percent"));
        assert!(output.contains("process_memory_bytes"));
        assert!(output.contains("1234"));
        assert!(output.contains("test_process"));
        assert!(output.contains("25.5"));
    }

    #[test]
    fn test_network_metrics_export() {
        let exporter = PrometheusExporter::new();
        let mut network_metrics = NetworkInterfaceStats::default();
        
        // Set some test values
        network_metrics.interface_name = "eth0".to_string();
        network_metrics.rx_bytes = 1024 * 1024; // 1 MB
        network_metrics.tx_bytes = 512 * 1024; // 512 KB
        network_metrics.rx_packets = 1000;
        network_metrics.tx_packets = 500;
        
        let result = exporter.export_network_metrics_prometheus(&[network_metrics]);
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert!(output.contains("network_bytes"));
        assert!(output.contains("network_packets"));
        assert!(output.contains("eth0"));
    }

    #[test]
    fn test_http_headers_export() {
        let exporter = PrometheusExporter::new();
        let test_output = "test_metric 42\n".to_string();
        
        let result = exporter.export_with_http_headers(&test_output);
        assert!(result.contains("HTTP/1.1 200 OK"));
        assert!(result.contains("Content-Type: text/plain"));
        assert!(result.contains("Content-Length:"));
        assert!(result.contains("test_metric 42"));
    }

    #[test]
    fn test_prometheus_format_compliance() {
        let exporter = PrometheusExporter::new();
        let mut system_metrics = SystemMetrics::default();
        
        // Set some test values
        system_metrics.cpu.user_percent = 30.5;
        system_metrics.memory.total_bytes = 16 * 1024 * 1024 * 1024;
        
        let result = exporter.export_system_metrics_prometheus(&system_metrics);
        assert!(result.is_ok());
        
        let output = result.unwrap();
        
        // Check that the output follows Prometheus format
        assert!(output.contains("system_cpu_usage_percent"));
        assert!(output.contains("system_memory_bytes"));
        assert!(output.contains("type=\""));
        
        // Check that values are properly formatted
        assert!(output.contains("30.5"));
        assert!(output.contains(&format!("{}", 16 * 1024 * 1024 * 1024)));
    }

    #[test]
    fn test_empty_metrics_export() {
        let exporter = PrometheusExporter::new();
        let system_metrics = SystemMetrics::default();
        let process_metrics = Vec::<ProcessMetrics>::new();
        let network_metrics = Vec::<NetworkInterfaceStats>::new();
        let gpu_metrics = GpuMetricsCollection::default();
        let ml_metrics = HashMap::<String, MLPerformanceMetrics>::new();
        
        let result = exporter.export_all_metrics(
            &system_metrics, &process_metrics, &network_metrics, &gpu_metrics, &ml_metrics
        );
        
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_config_options() {
        let mut config = PrometheusExporterConfig::default();
        
        // Test disabling various metrics
        config.enable_system_metrics = false;
        config.enable_process_metrics = false;
        config.enable_network_metrics = false;
        config.include_help_text = false;
        
        let exporter = PrometheusExporter::with_config(config);
        
        assert!(!exporter.config.enable_system_metrics);
        assert!(!exporter.config.enable_process_metrics);
        assert!(!exporter.config.enable_network_metrics);
        assert!(!exporter.config.include_help_text);
    }

    #[test]
    fn test_large_metric_values() {
        let exporter = PrometheusExporter::new();
        let mut system_metrics = SystemMetrics::default();
        
        // Test with large values
        system_metrics.memory.total_bytes = u64::MAX;
        system_metrics.memory.used_bytes = u64::MAX / 2;
        
        let result = exporter.export_system_metrics_prometheus(&system_metrics);
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert!(output.contains(&format!("{}", u64::MAX)));
        assert!(output.contains(&format!("{}", u64::MAX / 2)));
    }

    #[test]
    fn test_special_characters_in_labels() {
        let exporter = PrometheusExporter::new();
        let mut process_metrics = ProcessMetrics::default();
        
        // Test with special characters in process name
        process_metrics.pid = 1234;
        process_metrics.exe = Some("test-process.name".to_string());
        process_metrics.cpu_usage_percent = 25.5;
        
        let result = exporter.export_process_metrics_prometheus(&process_metrics);
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert!(output.contains("test-process.name"));
    }
}
