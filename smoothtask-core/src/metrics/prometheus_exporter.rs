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
use crate::metrics::extended_hardware_sensors::{ExtendedHardwareSensors, ExtendedHardwareSensorsMonitor};

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
    /// Enable extended hardware sensors export
    pub enable_extended_hardware_sensors: bool,
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
            enable_extended_hardware_sensors: true,
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
    extended_sensors_monitor: ExtendedHardwareSensorsMonitor,
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
            extended_sensors_monitor: ExtendedHardwareSensorsMonitor::new(
                crate::metrics::extended_hardware_sensors::ExtendedHardwareSensorsConfig::default()
            ),
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
        extended_sensors: &ExtendedHardwareSensors,
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

        // Export extended hardware sensors
        if self.config.enable_extended_hardware_sensors {
            self.export_extended_hardware_sensors(extended_sensors, &mut output)?;
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

    /// Export extended hardware sensors metrics
    fn export_extended_hardware_sensors(&self, sensors: &ExtendedHardwareSensors, output: &mut String) -> Result<()> {
        if self.config.include_help_text {
            output.push_str("# HELP extended_hardware_temperature_c Extended hardware temperature sensors in Celsius\n");
            output.push_str("# TYPE extended_hardware_temperature_c gauge\n");
            output.push_str("# HELP extended_hardware_fan_speed_rpm Extended hardware fan speeds in RPM\n");
            output.push_str("# TYPE extended_hardware_fan_speed_rpm gauge\n");
            output.push_str("# HELP extended_hardware_voltage_v Extended hardware voltages in Volts\n");
            output.push_str("# TYPE extended_hardware_voltage_v gauge\n");
            output.push_str("# HELP extended_hardware_current_a Extended hardware currents in Amperes\n");
            output.push_str("# TYPE extended_hardware_current_a gauge\n");
            output.push_str("# HELP extended_hardware_power_w Extended hardware power in Watts\n");
            output.push_str("# TYPE extended_hardware_power_w gauge\n");
            output.push_str("# HELP extended_hardware_energy_j Extended hardware energy in Joules\n");
            output.push_str("# TYPE extended_hardware_energy_j gauge\n");
            output.push_str("# HELP extended_hardware_humidity_percent Extended hardware humidity in percent\n");
            output.push_str("# TYPE extended_hardware_humidity_percent gauge\n");
            output.push_str("# HELP extended_hardware_pressure_pa Extended hardware pressure in Pascals\n");
            output.push_str("# TYPE extended_hardware_pressure_pa gauge\n");
            output.push_str("# HELP extended_hardware_illumination_lux Extended hardware illumination in Lux\n");
            output.push_str("# TYPE extended_hardware_illumination_lux gauge\n");
            output.push_str("# HELP extended_hardware_custom_sensor Extended hardware custom sensors\n");
            output.push_str("# TYPE extended_hardware_custom_sensor gauge\n");
            output.push_str("# HELP extended_hardware_thunderbolt_speed_gbps Thunderbolt device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_thunderbolt_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_pcie_speed_gbps PCIe device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_pcie_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_usb4_speed_gbps USB4 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_usb4_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_nvme_speed_gbps NVMe device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_nvme_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_thunderbolt5_speed_gbps Thunderbolt 5 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_thunderbolt5_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_pcie6_speed_gbps PCIe 6.0 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_pcie6_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_usb4_v2_speed_gbps USB4 v2 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_usb4_v2_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_nvme_2_0_speed_gbps NVMe 2.0 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_nvme_2_0_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_thunderbolt6_speed_gbps Thunderbolt 6 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_thunderbolt6_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_pcie7_speed_gbps PCIe 7.0 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_pcie7_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_usb4_v3_speed_gbps USB4 v3 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_usb4_v3_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_nvme_3_0_speed_gbps NVMe 3.0 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_nvme_3_0_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_additional_temperature_c Additional extended hardware temperature sensors in Celsius\n");
            output.push_str("# TYPE extended_hardware_additional_temperature_c gauge\n");
            output.push_str("# HELP extended_hardware_additional_frequency_hz Additional extended hardware frequencies in Hertz\n");
            output.push_str("# TYPE extended_hardware_additional_frequency_hz gauge\n");
            output.push_str("# HELP extended_hardware_additional_utilization_percent Additional extended hardware utilizations in percent\n");
            output.push_str("# TYPE extended_hardware_additional_utilization_percent gauge\n");
            output.push_str("# HELP extended_hardware_additional_load_percent Additional extended hardware loads in percent\n");
            output.push_str("# TYPE extended_hardware_additional_load_percent gauge\n");
            output.push_str("# HELP extended_hardware_additional_efficiency_percent Additional extended hardware efficiencies in percent\n");
            output.push_str("# TYPE extended_hardware_additional_efficiency_percent gauge\n");
            output.push_str("# HELP extended_hardware_additional_health_percent Additional extended hardware health in percent\n");
            output.push_str("# TYPE extended_hardware_additional_health_percent gauge\n");
            output.push_str("# HELP extended_hardware_additional_capacity_percent Additional extended hardware capacities in percent\n");
            output.push_str("# TYPE extended_hardware_additional_capacity_percent gauge\n");
            output.push_str("# HELP extended_hardware_additional_throughput_mbps Additional extended hardware throughputs in MB/s\n");
            output.push_str("# TYPE extended_hardware_additional_throughput_mbps gauge\n");
            output.push_str("# HELP extended_hardware_additional_latency_ns Additional extended hardware latencies in nanoseconds\n");
            output.push_str("# TYPE extended_hardware_additional_latency_ns gauge\n");
            output.push_str("# HELP extended_hardware_additional_error_count Additional extended hardware error counts\n");
            output.push_str("# TYPE extended_hardware_additional_error_count gauge\n");
            output.push_str("# HELP extended_hardware_vibration_mps2 Extended hardware vibration sensors in m/s²\n");
            output.push_str("# TYPE extended_hardware_vibration_mps2 gauge\n");
            output.push_str("# HELP extended_hardware_acceleration_mps2 Extended hardware acceleration sensors in m/s²\n");
            output.push_str("# TYPE extended_hardware_acceleration_mps2 gauge\n");
            output.push_str("# HELP extended_hardware_magnetic_field_ut Extended hardware magnetic field sensors in microteslas\n");
            output.push_str("# TYPE extended_hardware_magnetic_field_ut gauge\n");
            output.push_str("# HELP extended_hardware_sound_level_db Extended hardware sound level sensors in decibels\n");
            output.push_str("# TYPE extended_hardware_sound_level_db gauge\n");
            output.push_str("# HELP extended_hardware_light_level_lux Extended hardware light level sensors in lux\n");
            output.push_str("# TYPE extended_hardware_light_level_lux gauge\n");
        }

        // Export temperature sensors
        for (sensor_name, temp) in &sensors.temperatures_c {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_temperature_c{{sensor=\"{}\"}} {}\n", sanitized_name, temp));
        }

        // Export additional fan speeds
        for (sensor_name, speed) in &sensors.additional_fan_speeds_rpm {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_fan_speed_rpm{{sensor=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export additional voltages
        for (sensor_name, voltage) in &sensors.additional_voltages_v {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_voltage_v{{sensor=\"{}\"}} {}\n", sanitized_name, voltage));
        }

        // Export additional currents
        for (sensor_name, current) in &sensors.additional_currents_a {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_current_a{{sensor=\"{}\"}} {}\n", sanitized_name, current));
        }

        // Export additional power
        for (sensor_name, power) in &sensors.additional_power_w {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_power_w{{sensor=\"{}\"}} {}\n", sanitized_name, power));
        }

        // Export additional energy
        for (sensor_name, energy) in &sensors.additional_energy_j {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_energy_j{{sensor=\"{}\"}} {}\n", sanitized_name, energy));
        }

        // Export additional humidity
        for (sensor_name, humidity) in &sensors.additional_humidity_percent {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_humidity_percent{{sensor=\"{}\"}} {}\n", sanitized_name, humidity));
        }

        // Export pressure
        for (sensor_name, pressure) in &sensors.pressure_pa {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_pressure_pa{{sensor=\"{}\"}} {}\n", sanitized_name, pressure));
        }

        // Export illumination
        for (sensor_name, illum) in &sensors.illumination_lux {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_illumination_lux{{sensor=\"{}\"}} {}\n", sanitized_name, illum));
        }

        // Export custom sensors
        for (sensor_name, value, unit) in &sensors.custom_sensors {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            let sanitized_unit = self.sanitize_metric_name(unit);
            output.push_str(&format!("extended_hardware_custom_sensor{{sensor=\"{}\",unit=\"{}\"}} {}\n", sanitized_name, sanitized_unit, value));
        }

        // Export Thunderbolt devices
        for (device_name, speed) in &sensors.thunderbolt_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_thunderbolt_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export PCIe devices
        for (device_name, speed) in &sensors.pcie_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_pcie_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export USB4 devices
        for (device_name, speed) in &sensors.usb4_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_usb4_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export NVMe devices
        for (device_name, speed) in &sensors.nvme_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_nvme_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export Thunderbolt 5 devices
        for (device_name, speed) in &sensors.thunderbolt5_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_thunderbolt5_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export PCIe 6.0 devices
        for (device_name, speed) in &sensors.pcie6_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_pcie6_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export USB4 v2 devices
        for (device_name, speed) in &sensors.usb4_v2_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_usb4_v2_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export NVMe 2.0 devices
        for (device_name, speed) in &sensors.nvme_2_0_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_nvme_2_0_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export Thunderbolt 6 devices
        for (device_name, speed) in &sensors.thunderbolt6_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_thunderbolt6_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export PCIe 7.0 devices
        for (device_name, speed) in &sensors.pcie7_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_pcie7_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export USB4 v3 devices
        for (device_name, speed) in &sensors.usb4_v3_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_usb4_v3_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export NVMe 3.0 devices
        for (device_name, speed) in &sensors.nvme_3_0_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_nvme_3_0_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export additional temperature sensors
        for (sensor_name, temp) in &sensors.additional_temperatures_c {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_temperature_c{{sensor=\"{}\"}} {}\n", sanitized_name, temp));
        }

        // Export additional frequency sensors
        for (sensor_name, freq) in &sensors.additional_frequencies_hz {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_frequency_hz{{sensor=\"{}\"}} {}\n", sanitized_name, freq));
        }

        // Export additional utilization sensors
        for (sensor_name, util) in &sensors.additional_utilizations_percent {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_utilization_percent{{sensor=\"{}\"}} {}\n", sanitized_name, util));
        }

        // Export additional load sensors
        for (sensor_name, load) in &sensors.additional_loads_percent {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_load_percent{{sensor=\"{}\"}} {}\n", sanitized_name, load));
        }

        // Export additional efficiency sensors
        for (sensor_name, eff) in &sensors.additional_efficiencies_percent {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_efficiency_percent{{sensor=\"{}\"}} {}\n", sanitized_name, eff));
        }

        // Export additional health sensors
        for (sensor_name, health) in &sensors.additional_health_percent {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_health_percent{{sensor=\"{}\"}} {}\n", sanitized_name, health));
        }

        // Export additional capacity sensors
        for (sensor_name, cap) in &sensors.additional_capacities_percent {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_capacity_percent{{sensor=\"{}\"}} {}\n", sanitized_name, cap));
        }

        // Export additional throughput sensors
        for (sensor_name, throughput) in &sensors.additional_throughputs_mbps {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_throughput_mbps{{sensor=\"{}\"}} {}\n", sanitized_name, throughput));
        }

        // Export additional latency sensors
        for (sensor_name, latency) in &sensors.additional_latencies_ns {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_latency_ns{{sensor=\"{}\"}} {}\n", sanitized_name, latency));
        }

        // Export additional error sensors
        for (sensor_name, errors) in &sensors.additional_errors_count {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_error_count{{sensor=\"{}\"}} {}\n", sanitized_name, errors));
        }

        // Export vibration sensors
        for (sensor_name, vibration) in &sensors.vibration_mps2 {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_vibration_mps2{{sensor=\"{}\"}} {}\n", sanitized_name, vibration));
        }

        // Export acceleration sensors
        for (sensor_name, acceleration) in &sensors.acceleration_mps2 {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_acceleration_mps2{{sensor=\"{}\"}} {}\n", sanitized_name, acceleration));
        }

        // Export magnetic field sensors
        for (sensor_name, magnetic_field) in &sensors.magnetic_field_ut {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_magnetic_field_ut{{sensor=\"{}\"}} {}\n", sanitized_name, magnetic_field));
        }

        // Export sound level sensors
        for (sensor_name, sound_level) in &sensors.sound_level_db {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_sound_level_db{{sensor=\"{}\"}} {}\n", sanitized_name, sound_level));
        }

        // Export light level sensors
        for (sensor_name, light_level) in &sensors.light_level_lux {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_light_level_lux{{sensor=\"{}\"}} {}\n", sanitized_name, light_level));
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

    /// Export extended hardware sensors metrics in Prometheus format
    pub fn export_extended_hardware_sensors_prometheus(&self, sensors: &ExtendedHardwareSensors) -> Result<String> {
        let mut output = String::new();

        if self.config.include_help_text {
            output.push_str("# HELP extended_hardware_temperature_c Extended hardware temperature sensors in Celsius\n");
            output.push_str("# TYPE extended_hardware_temperature_c gauge\n");
            output.push_str("# HELP extended_hardware_fan_speed_rpm Extended hardware fan speeds in RPM\n");
            output.push_str("# TYPE extended_hardware_fan_speed_rpm gauge\n");
            output.push_str("# HELP extended_hardware_voltage_v Extended hardware voltages in Volts\n");
            output.push_str("# TYPE extended_hardware_voltage_v gauge\n");
            output.push_str("# HELP extended_hardware_current_a Extended hardware currents in Amperes\n");
            output.push_str("# TYPE extended_hardware_current_a gauge\n");
            output.push_str("# HELP extended_hardware_power_w Extended hardware power in Watts\n");
            output.push_str("# TYPE extended_hardware_power_w gauge\n");
            output.push_str("# HELP extended_hardware_energy_j Extended hardware energy in Joules\n");
            output.push_str("# TYPE extended_hardware_energy_j gauge\n");
            output.push_str("# HELP extended_hardware_humidity_percent Extended hardware humidity in percent\n");
            output.push_str("# TYPE extended_hardware_humidity_percent gauge\n");
            output.push_str("# HELP extended_hardware_pressure_pa Extended hardware pressure in Pascals\n");
            output.push_str("# TYPE extended_hardware_pressure_pa gauge\n");
            output.push_str("# HELP extended_hardware_illumination_lux Extended hardware illumination in Lux\n");
            output.push_str("# TYPE extended_hardware_illumination_lux gauge\n");
            output.push_str("# HELP extended_hardware_custom_sensor Extended hardware custom sensors\n");
            output.push_str("# TYPE extended_hardware_custom_sensor gauge\n");
            output.push_str("# HELP extended_hardware_thunderbolt_speed_gbps Thunderbolt device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_thunderbolt_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_pcie_speed_gbps PCIe device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_pcie_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_usb4_speed_gbps USB4 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_usb4_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_nvme_speed_gbps NVMe device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_nvme_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_thunderbolt5_speed_gbps Thunderbolt 5 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_thunderbolt5_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_pcie6_speed_gbps PCIe 6.0 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_pcie6_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_usb4_v2_speed_gbps USB4 v2 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_usb4_v2_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_nvme_2_0_speed_gbps NVMe 2.0 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_nvme_2_0_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_thunderbolt6_speed_gbps Thunderbolt 6 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_thunderbolt6_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_pcie7_speed_gbps PCIe 7.0 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_pcie7_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_usb4_v3_speed_gbps USB4 v3 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_usb4_v3_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_nvme_3_0_speed_gbps NVMe 3.0 device speeds in Gbps\n");
            output.push_str("# TYPE extended_hardware_nvme_3_0_speed_gbps gauge\n");
            output.push_str("# HELP extended_hardware_additional_temperature_c Additional extended hardware temperature sensors in Celsius\n");
            output.push_str("# TYPE extended_hardware_additional_temperature_c gauge\n");
            output.push_str("# HELP extended_hardware_additional_frequency_hz Additional extended hardware frequencies in Hertz\n");
            output.push_str("# TYPE extended_hardware_additional_frequency_hz gauge\n");
            output.push_str("# HELP extended_hardware_additional_utilization_percent Additional extended hardware utilizations in percent\n");
            output.push_str("# TYPE extended_hardware_additional_utilization_percent gauge\n");
            output.push_str("# HELP extended_hardware_additional_load_percent Additional extended hardware loads in percent\n");
            output.push_str("# TYPE extended_hardware_additional_load_percent gauge\n");
            output.push_str("# HELP extended_hardware_additional_efficiency_percent Additional extended hardware efficiencies in percent\n");
            output.push_str("# TYPE extended_hardware_additional_efficiency_percent gauge\n");
            output.push_str("# HELP extended_hardware_additional_health_percent Additional extended hardware health in percent\n");
            output.push_str("# TYPE extended_hardware_additional_health_percent gauge\n");
            output.push_str("# HELP extended_hardware_additional_capacity_percent Additional extended hardware capacities in percent\n");
            output.push_str("# TYPE extended_hardware_additional_capacity_percent gauge\n");
            output.push_str("# HELP extended_hardware_additional_throughput_mbps Additional extended hardware throughputs in MB/s\n");
            output.push_str("# TYPE extended_hardware_additional_throughput_mbps gauge\n");
            output.push_str("# HELP extended_hardware_additional_latency_ns Additional extended hardware latencies in nanoseconds\n");
            output.push_str("# TYPE extended_hardware_additional_latency_ns gauge\n");
            output.push_str("# HELP extended_hardware_additional_error_count Additional extended hardware error counts\n");
            output.push_str("# TYPE extended_hardware_additional_error_count gauge\n");
        }

        // Export temperature sensors
        for (sensor_name, temp) in &sensors.temperatures_c {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_temperature_c{{sensor=\"{}\"}} {}\n", sanitized_name, temp));
        }

        // Export additional fan speeds
        for (sensor_name, speed) in &sensors.additional_fan_speeds_rpm {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_fan_speed_rpm{{sensor=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export additional voltages
        for (sensor_name, voltage) in &sensors.additional_voltages_v {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_voltage_v{{sensor=\"{}\"}} {}\n", sanitized_name, voltage));
        }

        // Export additional currents
        for (sensor_name, current) in &sensors.additional_currents_a {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_current_a{{sensor=\"{}\"}} {}\n", sanitized_name, current));
        }

        // Export additional power
        for (sensor_name, power) in &sensors.additional_power_w {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_power_w{{sensor=\"{}\"}} {}\n", sanitized_name, power));
        }

        // Export additional energy
        for (sensor_name, energy) in &sensors.additional_energy_j {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_energy_j{{sensor=\"{}\"}} {}\n", sanitized_name, energy));
        }

        // Export additional humidity
        for (sensor_name, humidity) in &sensors.additional_humidity_percent {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_humidity_percent{{sensor=\"{}\"}} {}\n", sanitized_name, humidity));
        }

        // Export pressure
        for (sensor_name, pressure) in &sensors.pressure_pa {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_pressure_pa{{sensor=\"{}\"}} {}\n", sanitized_name, pressure));
        }

        // Export illumination
        for (sensor_name, illum) in &sensors.illumination_lux {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_illumination_lux{{sensor=\"{}\"}} {}\n", sanitized_name, illum));
        }

        // Export custom sensors
        for (sensor_name, value, unit) in &sensors.custom_sensors {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            let sanitized_unit = self.sanitize_metric_name(unit);
            output.push_str(&format!("extended_hardware_custom_sensor{{sensor=\"{}\",unit=\"{}\"}} {}\n", sanitized_name, sanitized_unit, value));
        }

        // Export Thunderbolt devices
        for (device_name, speed) in &sensors.thunderbolt_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_thunderbolt_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export PCIe devices
        for (device_name, speed) in &sensors.pcie_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_pcie_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export USB4 devices
        for (device_name, speed) in &sensors.usb4_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_usb4_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export NVMe devices
        for (device_name, speed) in &sensors.nvme_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_nvme_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export Thunderbolt 5 devices
        for (device_name, speed) in &sensors.thunderbolt5_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_thunderbolt5_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export PCIe 6.0 devices
        for (device_name, speed) in &sensors.pcie6_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_pcie6_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export USB4 v2 devices
        for (device_name, speed) in &sensors.usb4_v2_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_usb4_v2_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export NVMe 2.0 devices
        for (device_name, speed) in &sensors.nvme_2_0_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_nvme_2_0_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export Thunderbolt 6 devices
        for (device_name, speed) in &sensors.thunderbolt6_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_thunderbolt6_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export PCIe 7.0 devices
        for (device_name, speed) in &sensors.pcie7_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_pcie7_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export USB4 v3 devices
        for (device_name, speed) in &sensors.usb4_v3_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_usb4_v3_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export NVMe 3.0 devices
        for (device_name, speed) in &sensors.nvme_3_0_devices {
            let sanitized_name = self.sanitize_metric_name(device_name);
            output.push_str(&format!("extended_hardware_nvme_3_0_speed_gbps{{device=\"{}\"}} {}\n", sanitized_name, speed));
        }

        // Export additional temperature sensors
        for (sensor_name, temp) in &sensors.additional_temperatures_c {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_temperature_c{{sensor=\"{}\"}} {}\n", sanitized_name, temp));
        }

        // Export additional frequency sensors
        for (sensor_name, freq) in &sensors.additional_frequencies_hz {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_frequency_hz{{sensor=\"{}\"}} {}\n", sanitized_name, freq));
        }

        // Export additional utilization sensors
        for (sensor_name, util) in &sensors.additional_utilizations_percent {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_utilization_percent{{sensor=\"{}\"}} {}\n", sanitized_name, util));
        }

        // Export additional load sensors
        for (sensor_name, load) in &sensors.additional_loads_percent {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_load_percent{{sensor=\"{}\"}} {}\n", sanitized_name, load));
        }

        // Export additional efficiency sensors
        for (sensor_name, eff) in &sensors.additional_efficiencies_percent {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_efficiency_percent{{sensor=\"{}\"}} {}\n", sanitized_name, eff));
        }

        // Export additional health sensors
        for (sensor_name, health) in &sensors.additional_health_percent {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_health_percent{{sensor=\"{}\"}} {}\n", sanitized_name, health));
        }

        // Export additional capacity sensors
        for (sensor_name, cap) in &sensors.additional_capacities_percent {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_capacity_percent{{sensor=\"{}\"}} {}\n", sanitized_name, cap));
        }

        // Export additional throughput sensors
        for (sensor_name, throughput) in &sensors.additional_throughputs_mbps {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_throughput_mbps{{sensor=\"{}\"}} {}\n", sanitized_name, throughput));
        }

        // Export additional latency sensors
        for (sensor_name, latency) in &sensors.additional_latencies_ns {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_latency_ns{{sensor=\"{}\"}} {}\n", sanitized_name, latency));
        }

        // Export additional error sensors
        for (sensor_name, errors) in &sensors.additional_errors_count {
            let sanitized_name = self.sanitize_metric_name(sensor_name);
            output.push_str(&format!("extended_hardware_additional_error_count{{sensor=\"{}\"}} {}\n", sanitized_name, errors));
        }

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
        extended_sensors: &ExtendedHardwareSensors,
    ) -> Result<String> {
        let prometheus_output = self.export_all_metrics(
            system_metrics, process_metrics, network_metrics, gpu_metrics, ml_metrics, extended_sensors
        )?;
        
        Ok(self.export_with_http_headers(&prometheus_output))
    }
}

impl Default for PrometheusExporter {
    fn default() -> Self {
        Self {
            config: PrometheusExporterConfig::default(),
            custom_metrics: HashMap::new(),
            extended_sensors_monitor: ExtendedHardwareSensorsMonitor::new(
                crate::metrics::extended_hardware_sensors::ExtendedHardwareSensorsConfig::default()
            ),
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

    #[test]
    fn test_extended_hardware_sensors_export() {
        let exporter = PrometheusExporter::new();
        let mut sensors = ExtendedHardwareSensors::default();
        
        // Add some test sensor data
        sensors.temperatures_c.push(("cpu_thermal".to_string(), 45.5));
        sensors.additional_fan_speeds_rpm.push(("chassis_fan".to_string(), 1200.0));
        sensors.additional_voltages_v.push(("vcore".to_string(), 1.25));
        sensors.thunderbolt_devices.push(("thunderbolt_port_1".to_string(), 40.0));
        sensors.pcie_devices.push(("gpu_pcie".to_string(), 16.0));
        sensors.additional_temperatures_c.push(("gpu_temp".to_string(), 65.0));
        sensors.additional_frequencies_hz.push(("cpu_freq".to_string(), 3500000000.0));
        
        // Add new sensor data
        sensors.vibration_mps2.push(("vibration_sensor_1".to_string(), 2.5));
        sensors.acceleration_mps2.push(("accel_sensor_1".to_string(), 9.81));
        sensors.magnetic_field_ut.push(("magnet_sensor_1".to_string(), 50.0));
        sensors.sound_level_db.push(("sound_sensor_1".to_string(), 65.0));
        sensors.light_level_lux.push(("light_sensor_1".to_string(), 1000.0));
        
        // Test the export
        let result = exporter.export_extended_hardware_sensors_prometheus(&sensors);
        assert!(result.is_ok());
        
        let output = result.unwrap();
        
        // Verify the output contains expected metrics
        assert!(output.contains("extended_hardware_temperature_c"));
        assert!(output.contains("sensor=\"cpu_thermal\""));
        assert!(output.contains("45.5"));
        
        assert!(output.contains("extended_hardware_fan_speed_rpm"));
        assert!(output.contains("sensor=\"chassis_fan\""));
        assert!(output.contains("1200"));
        
        assert!(output.contains("extended_hardware_voltage_v"));
        assert!(output.contains("sensor=\"vcore\""));
        assert!(output.contains("1.25"));
        
        assert!(output.contains("extended_hardware_thunderbolt_speed_gbps"));
        assert!(output.contains("device=\"thunderbolt_port_1\""));
        assert!(output.contains("40"));
        
        assert!(output.contains("extended_hardware_pcie_speed_gbps"));
        assert!(output.contains("device=\"gpu_pcie\""));
        assert!(output.contains("16"));
        
        assert!(output.contains("extended_hardware_additional_temperature_c"));
        assert!(output.contains("sensor=\"gpu_temp\""));
        assert!(output.contains("65"));
        
        assert!(output.contains("extended_hardware_additional_frequency_hz"));
        assert!(output.contains("sensor=\"cpu_freq\""));
        assert!(output.contains("3500000000"));
        
        // Verify new sensor metrics
        assert!(output.contains("extended_hardware_vibration_mps2"));
        assert!(output.contains("sensor=\"vibration_sensor_1\""));
        assert!(output.contains("2.5"));
        
        assert!(output.contains("extended_hardware_acceleration_mps2"));
        assert!(output.contains("sensor=\"accel_sensor_1\""));
        assert!(output.contains("9.81"));
        
        assert!(output.contains("extended_hardware_magnetic_field_ut"));
        assert!(output.contains("sensor=\"magnet_sensor_1\""));
        assert!(output.contains("50"));
        
        assert!(output.contains("extended_hardware_sound_level_db"));
        assert!(output.contains("sensor=\"sound_sensor_1\""));
        assert!(output.contains("65"));
        
        assert!(output.contains("extended_hardware_light_level_lux"));
        assert!(output.contains("sensor=\"light_sensor_1\""));
        assert!(output.contains("1000"));
    }

    #[test]
    fn test_extended_hardware_sensors_disabled() {
        let mut config = PrometheusExporterConfig::default();
        config.enable_extended_hardware_sensors = false;
        let exporter = PrometheusExporter::with_config(config);
        let sensors = ExtendedHardwareSensors::default();
        
        // Test that when disabled, no extended sensor metrics are exported
        let result = exporter.export_all_metrics(
            &SystemMetrics::default(),
            &[],
            &[],
            &crate::metrics::gpu::GpuMetricsCollection::default(),
            &HashMap::new(),
            &sensors,
        );
        
        assert!(result.is_ok());
        let output = result.unwrap();
        
        // Should not contain extended hardware sensor metrics
        assert!(!output.contains("extended_hardware_temperature_c"));
        assert!(!output.contains("extended_hardware_fan_speed_rpm"));
    }

    #[test]
    fn test_extended_hardware_sensors_enabled() {
        let mut config = PrometheusExporterConfig::default();
        config.enable_extended_hardware_sensors = true;
        let exporter = PrometheusExporter::with_config(config);
        
        let mut sensors = ExtendedHardwareSensors::default();
        sensors.temperatures_c.push(("test_sensor".to_string(), 30.0));
        
        // Test that when enabled, extended sensor metrics are exported
        let result = exporter.export_all_metrics(
            &SystemMetrics::default(),
            &[],
            &[],
            &crate::metrics::gpu::GpuMetricsCollection::default(),
            &HashMap::new(),
            &sensors,
        );
        
        assert!(result.is_ok());
        let output = result.unwrap();
        
        // Should contain extended hardware sensor metrics
        assert!(output.contains("extended_hardware_temperature_c"));
        assert!(output.contains("sensor=\"test_sensor\""));
        assert!(output.contains("30"));
    }
}
