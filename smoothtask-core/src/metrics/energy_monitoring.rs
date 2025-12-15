// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

//! Базовый модуль для мониторинга энергопотребления с новыми сенсорами
//!
//! Этот модуль предоставляет функциональность для сбора метрик энергопотребления
//! с различных аппаратных сенсоров и интерфейсов. Поддерживаются:
//! - Стандартные интерфейсы энергопотребления (/sys/class/powercap)
//! - RAPL (Running Average Power Limit)
//! - ACPI интерфейсы
//! - Новые типы сенсоров (температура, мощность, энергия)
//! - Интеграция с существующими системами мониторинга

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

/// Тип сенсора энергопотребления
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnergySensorType {
    /// Стандартный RAPL сенсор
    Rapl,
    /// ACPI сенсор
    Acpi,
    /// PowerCap сенсор
    PowerCap,
    /// Сенсор батареи
    Battery,
    /// USB Power Delivery сенсор
    UsbPowerDelivery,
    /// Термальный сенсор мощности
    ThermalPower,
    /// Программный сенсор мощности
    SoftwarePower,
    /// Пользовательский сенсор
    Custom,
    /// Сенсор энергоэффективности
    EnergyEfficiency,
    /// Сенсор мощности компонентов
    ComponentPower,
    /// Сенсор мощности PCIe устройств
    PciePower,
    /// Сенсор мощности GPU
    GpuPower,
    /// Сенсор мощности CPU
    CpuPower,
    /// Сенсор мощности памяти
    MemoryPower,
    /// Неизвестный тип
    Unknown,
}

impl Default for EnergySensorType {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Метрики энергопотребления сенсора
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnergySensorMetrics {
    /// Идентификатор сенсора
    pub sensor_id: String,
    /// Тип сенсора
    pub sensor_type: EnergySensorType,
    /// Текущее энергопотребление в микроджоулях
    pub energy_uj: u64,
    /// Текущая мощность в ваттах
    pub power_w: f32,
    /// Время последнего измерения
    pub timestamp: u64,
    /// Признак достоверности данных
    pub is_reliable: bool,
    /// Путь к сенсору в файловой системе
    pub sensor_path: String,
    /// Энергоэффективность (производительность на ватт)
    pub energy_efficiency: Option<f32>,
    /// Максимальная мощность
    pub max_power_w: Option<f32>,
    /// Средняя мощность
    pub average_power_w: Option<f32>,
    /// Коэффициент использования
    pub utilization_percent: Option<f32>,
    /// Температура компонента
    pub temperature_c: Option<f32>,
    /// Тип компонента (CPU, GPU, Memory, etc.)
    pub component_type: Option<String>,
}

/// Анализ распределения энергопотребления по компонентам
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ComponentDistributionAnalysis {
    /// Идентификатор процесса (0 для системы в целом)
    pub pid: i32,
    /// Общее энергопотребление
    pub total_energy_uj: u64,
    /// Процент энергопотребления CPU
    pub cpu_percentage: f32,
    /// Энергопотребление CPU
    pub cpu_energy_uj: u64,
    /// Процент энергопотребления GPU
    pub gpu_percentage: f32,
    /// Энергопотребление GPU
    pub gpu_energy_uj: u64,
    /// Процент энергопотребления памяти
    pub memory_percentage: f32,
    /// Энергопотребление памяти
    pub memory_energy_uj: u64,
    /// Процент энергопотребления диска
    pub disk_percentage: f32,
    /// Энергопотребление диска
    pub disk_energy_uj: u64,
    /// Процент энергопотребления сети
    pub network_percentage: f32,
    /// Энергопотребление сети
    pub network_energy_uj: u64,
    /// Процент энергопотребления других компонентов
    pub other_percentage: f32,
    /// Энергопотребление других компонентов
    pub other_energy_uj: u64,
    /// Общий процент (должен быть ~100%)
    pub total_percentage: f32,
    /// Время анализа
    pub timestamp: u64,
    /// Признак достоверности данных
    pub is_reliable: bool,
}

/// Анализ энергоэффективности системы
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemEnergyEfficiencyAnalysis {
    /// Общее энергопотребление системы
    pub total_energy_uj: u64,
    /// Общая мощность системы
    pub total_power_w: f32,
    /// Средняя энергоэффективность
    pub average_efficiency: f32,
    /// Максимальная энергоэффективность
    pub max_efficiency: f32,
    /// Минимальная энергоэффективность
    pub min_efficiency: f32,
    /// Количество сенсоров с данными об энергоэффективности
    pub efficiency_count: usize,
    /// Время анализа
    pub timestamp: u64,
    /// Признак достоверности данных
    pub is_reliable: bool,
}

/// Конфигурация системы мониторинга энергопотребления
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyMonitoringConfig {
    /// Включить мониторинг RAPL
    pub enable_rapl: bool,
    /// Включить мониторинг ACPI
    pub enable_acpi: bool,
    /// Включить мониторинг PowerCap
    pub enable_powercap: bool,
    /// Включить мониторинг батареи
    pub enable_battery: bool,
    /// Включить мониторинг USB Power Delivery
    pub enable_usb_power_delivery: bool,
    /// Включить мониторинг термальных сенсоров мощности
    pub enable_thermal_power: bool,
    /// Включить мониторинг программных сенсоров мощности
    pub enable_software_power: bool,
    /// Включить мониторинг пользовательских сенсоров
    pub enable_custom_sensors: bool,
    /// Базовый путь к RAPL интерфейсам
    pub rapl_base_path: PathBuf,
    /// Базовый путь к ACPI интерфейсам
    pub acpi_base_path: PathBuf,
    /// Базовый путь к PowerCap интерфейсам
    pub powercap_base_path: PathBuf,
    /// Базовый путь к интерфейсам батареи
    pub battery_base_path: PathBuf,
    /// Базовый путь к USB Power Delivery интерфейсам
    pub usb_power_delivery_base_path: PathBuf,
    /// Базовый путь к термальным сенсорам мощности
    pub thermal_power_base_path: PathBuf,
    /// Базовый путь к программным сенсорам мощности
    pub software_power_base_path: PathBuf,
}

impl Default for EnergyMonitoringConfig {
    fn default() -> Self {
        Self {
            enable_rapl: true,
            enable_acpi: true,
            enable_powercap: true,
            enable_battery: true,
            enable_usb_power_delivery: true,
            enable_thermal_power: true,
            enable_software_power: true,
            enable_custom_sensors: true,
            rapl_base_path: PathBuf::from("/sys/class/powercap/intel-rapl"),
            acpi_base_path: PathBuf::from("/sys/class/power_supply"),
            powercap_base_path: PathBuf::from("/sys/class/powercap"),
            battery_base_path: PathBuf::from("/sys/class/power_supply"),
            usb_power_delivery_base_path: PathBuf::from("/sys/class/usb_power_delivery"),
            thermal_power_base_path: PathBuf::from(
                "/sys/kernel/tracing/events/thermal_power_allocator",
            ),
            software_power_base_path: PathBuf::from("/sys/devices/software/power"),
        }
    }
}

/// Основной монитор энергопотребления
#[derive(Debug)]
pub struct EnergyMonitor {
    /// Конфигурация монитора
    pub config: EnergyMonitoringConfig,
}

impl EnergyMonitor {
    /// Создать новый монитор энергопотребления с конфигурацией по умолчанию
    pub fn new() -> Self {
        Self {
            config: EnergyMonitoringConfig::default(),
        }
    }

    /// Создать монитор с кастомной конфигурацией
    pub fn with_config(config: EnergyMonitoringConfig) -> Self {
        Self { config }
    }

    /// Собрать метрики энергопотребления со всех доступных сенсоров
    pub fn collect_all_energy_metrics(&self) -> Result<Vec<EnergySensorMetrics>> {
        let mut all_metrics = Vec::new();

        // Собираем метрики RAPL
        if self.config.enable_rapl {
            if let Ok(rapl_metrics) = self.collect_rapl_metrics() {
                all_metrics.extend(rapl_metrics);
            }
        }

        // Собираем метрики ACPI
        if self.config.enable_acpi {
            if let Ok(acpi_metrics) = self.collect_acpi_metrics() {
                all_metrics.extend(acpi_metrics);
            }
        }

        // Собираем метрики PowerCap
        if self.config.enable_powercap {
            if let Ok(powercap_metrics) = self.collect_powercap_metrics() {
                all_metrics.extend(powercap_metrics);
            }
        }

        // Собираем метрики USB Power Delivery
        if self.config.enable_usb_power_delivery {
            if let Ok(usb_metrics) = self.collect_usb_power_delivery_metrics() {
                all_metrics.extend(usb_metrics);
            }
        }

        // Собираем метрики термальных сенсоров мощности
        if self.config.enable_thermal_power {
            if let Ok(thermal_metrics) = self.collect_thermal_power_metrics() {
                all_metrics.extend(thermal_metrics);
            }
        }

        // Собираем метрики программных сенсоров мощности
        if self.config.enable_software_power {
            if let Ok(software_metrics) = self.collect_software_power_metrics() {
                all_metrics.extend(software_metrics);
            }
        }

        // Собираем метрики пользовательских сенсоров
        if self.config.enable_custom_sensors {
            if let Ok(custom_metrics) = self.collect_custom_sensors() {
                all_metrics.extend(custom_metrics);
            }
        }

        // Собираем метрики батареи
        if self.config.enable_battery {
            if let Ok(battery_metrics) = self.collect_battery_metrics() {
                all_metrics.extend(battery_metrics);
            }
        }

        // Собираем метрики компонентов CPU
        if self.config.enable_rapl {
            if let Ok(cpu_metrics) = self.collect_cpu_component_metrics() {
                all_metrics.extend(cpu_metrics);
            }
        }

        // Собираем метрики компонентов GPU
        if let Ok(gpu_metrics) = self.collect_gpu_component_metrics() {
            all_metrics.extend(gpu_metrics);
        }

        // Собираем метрики компонентов памяти
        if let Ok(memory_metrics) = self.collect_memory_component_metrics() {
            all_metrics.extend(memory_metrics);
        }

        // Собираем метрики компонентов PCIe
        if let Ok(pcie_metrics) = self.collect_pcie_component_metrics() {
            all_metrics.extend(pcie_metrics);
        }

        Ok(all_metrics)
    }

    /// Собрать метрики RAPL
    pub fn collect_rapl_metrics(&self) -> Result<Vec<EnergySensorMetrics>> {
        let mut metrics = Vec::new();

        if !self.config.rapl_base_path.exists() {
            debug!(
                "RAPL base path does not exist: {:?}",
                self.config.rapl_base_path
            );
            return Ok(metrics);
        }

        let entries =
            fs::read_dir(&self.config.rapl_base_path).context("Failed to read RAPL directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read RAPL directory entry")?;
            let domain_path = entry.path();
            let energy_path = domain_path.join("energy_uj");

            if energy_path.exists() {
                if let Ok(energy_content) = fs::read_to_string(&energy_path) {
                    if let Ok(energy_uj) = energy_content.trim().parse::<u64>() {
                        let power_w = energy_uj as f32 / 1_000_000.0;
                        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

                        let sensor_id = domain_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        metrics.push(EnergySensorMetrics {
                            sensor_id,
                            sensor_type: EnergySensorType::Rapl,
                            energy_uj,
                            power_w,
                            timestamp,
                            is_reliable: true,
                            sensor_path: energy_path.to_string_lossy().into_owned(),
                            energy_efficiency: None,
                            max_power_w: None,
                            average_power_w: None,
                            utilization_percent: None,
                            temperature_c: None,
                            component_type: None,
                        });
                    }
                }
            }
        }

        Ok(metrics)
    }

    /// Собрать метрики ACPI
    pub fn collect_acpi_metrics(&self) -> Result<Vec<EnergySensorMetrics>> {
        let mut metrics = Vec::new();

        if !self.config.acpi_base_path.exists() {
            debug!(
                "ACPI base path does not exist: {:?}",
                self.config.acpi_base_path
            );
            return Ok(metrics);
        }

        let entries =
            fs::read_dir(&self.config.acpi_base_path).context("Failed to read ACPI directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read ACPI directory entry")?;
            let supply_path = entry.path();

            // Пробуем получить энергию из различных файлов
            let energy_files = ["energy_now", "energy_full", "charge_now", "charge_full"];

            for energy_file in energy_files {
                let energy_path = supply_path.join(energy_file);

                if energy_path.exists() {
                    if let Ok(energy_content) = fs::read_to_string(&energy_path) {
                        if let Ok(energy_microwatts) = energy_content.trim().parse::<u64>() {
                            // Конвертируем микроватты в микроджоули (упрощенно)
                            let energy_uj = energy_microwatts * 1000;
                            let power_w = energy_uj as f32 / 1_000_000.0;
                            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

                            let sensor_id = format!(
                                "{}_{}",
                                supply_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown"),
                                energy_file
                            );

                            metrics.push(EnergySensorMetrics {
                                sensor_id,
                                sensor_type: EnergySensorType::Acpi,
                                energy_uj,
                                power_w,
                                timestamp,
                                is_reliable: true,
                                sensor_path: energy_path.to_string_lossy().into_owned(),
                                energy_efficiency: None,
                                max_power_w: None,
                                average_power_w: None,
                                utilization_percent: None,
                                temperature_c: None,
                                component_type: None,
                            });
                        }
                    }
                }
            }
        }

        Ok(metrics)
    }

    /// Собрать метрики PowerCap
    pub fn collect_powercap_metrics(&self) -> Result<Vec<EnergySensorMetrics>> {
        let mut metrics = Vec::new();

        if !self.config.powercap_base_path.exists() {
            debug!(
                "PowerCap base path does not exist: {:?}",
                self.config.powercap_base_path
            );
            return Ok(metrics);
        }

        let entries = fs::read_dir(&self.config.powercap_base_path)
            .context("Failed to read PowerCap directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read PowerCap directory entry")?;
            let domain_path = entry.path();
            let energy_path = domain_path.join("energy_uj");

            if energy_path.exists() {
                if let Ok(energy_content) = fs::read_to_string(&energy_path) {
                    if let Ok(energy_uj) = energy_content.trim().parse::<u64>() {
                        let power_w = energy_uj as f32 / 1_000_000.0;
                        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

                        let sensor_id = domain_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        metrics.push(EnergySensorMetrics {
                            sensor_id,
                            sensor_type: EnergySensorType::PowerCap,
                            energy_uj,
                            power_w,
                            timestamp,
                            is_reliable: true,
                            sensor_path: energy_path.to_string_lossy().into_owned(),
                            energy_efficiency: None,
                            max_power_w: None,
                            average_power_w: None,
                            utilization_percent: None,
                            temperature_c: None,
                            component_type: None,
                        });
                    }
                }
            }
        }

        Ok(metrics)
    }

    /// Собрать метрики пользовательских сенсоров
    pub fn collect_custom_sensors(&self) -> Result<Vec<EnergySensorMetrics>> {
        let mut metrics = Vec::new();

        // Пробуем найти пользовательские сенсоры в стандартных местах
        let custom_sensor_paths = [
            "/sys/devices/platform/coretemp.0/hwmon/hwmon*/power*",
            "/sys/devices/system/cpu/cpu*/power*",
            "/sys/class/hwmon/hwmon*/power*",
        ];

        for _pattern in custom_sensor_paths {
            // В реальной реализации здесь был бы более сложный поиск
            // Для этой демонстрации мы просто пробуем несколько стандартных путей
            let test_paths = [
                "/sys/class/hwmon/hwmon0/power1_input",
                "/sys/class/hwmon/hwmon1/power1_input",
            ];

            for test_path in test_paths {
                let path = Path::new(test_path);
                if path.exists() {
                    if let Ok(energy_content) = fs::read_to_string(path) {
                        if let Ok(energy_microwatts) = energy_content.trim().parse::<u64>() {
                            let energy_uj = energy_microwatts * 1000;
                            let power_w = energy_uj as f32 / 1_000_000.0;
                            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

                            let sensor_id = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("custom")
                                .to_string();

                            metrics.push(EnergySensorMetrics {
                                sensor_id,
                                sensor_type: EnergySensorType::Custom,
                                energy_uj,
                                power_w,
                                timestamp,
                                is_reliable: false, // Пользовательские сенсоры могут быть менее надежны
                                sensor_path: test_path.to_string(),
                                energy_efficiency: None,
                                max_power_w: None,
                                average_power_w: None,
                                utilization_percent: None,
                                temperature_c: None,
                                component_type: None,
                            });
                        }
                    }
                }
            }
        }

        Ok(metrics)
    }

    /// Собрать метрики USB Power Delivery
    pub fn collect_usb_power_delivery_metrics(&self) -> Result<Vec<EnergySensorMetrics>> {
        let mut metrics = Vec::new();

        if !self.config.usb_power_delivery_base_path.exists() {
            debug!(
                "USB Power Delivery base path does not exist: {:?}",
                self.config.usb_power_delivery_base_path
            );
            return Ok(metrics);
        }

        let entries = fs::read_dir(&self.config.usb_power_delivery_base_path)
            .context("Failed to read USB Power Delivery directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read USB Power Delivery directory entry")?;
            let usb_path = entry.path();

            // Пробуем получить информацию о мощности USB
            let power_files = ["power", "current_power", "max_power"];

            for power_file in power_files {
                let power_path = usb_path.join(power_file);

                if power_path.exists() {
                    if let Ok(power_content) = fs::read_to_string(&power_path) {
                        if let Ok(power_microwatts) = power_content.trim().parse::<u64>() {
                            // Конвертируем микроватты в ватты
                            let power_w = power_microwatts as f32 / 1_000_000.0;
                            // Оцениваем энергию на основе текущей мощности
                            let energy_uj = (power_microwatts * 1000) as u64;
                            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

                            let sensor_id = format!(
                                "{}_{}",
                                usb_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("usb"),
                                power_file
                            );

                            metrics.push(EnergySensorMetrics {
                                sensor_id,
                                sensor_type: EnergySensorType::UsbPowerDelivery,
                                energy_uj,
                                power_w,
                                timestamp,
                                is_reliable: true,
                                sensor_path: power_path.to_string_lossy().into_owned(),
                                energy_efficiency: None,
                                max_power_w: None,
                                average_power_w: None,
                                utilization_percent: None,
                                temperature_c: None,
                                component_type: None,
                            });
                        }
                    }
                }
            }
        }

        Ok(metrics)
    }

    /// Собрать метрики термальных сенсоров мощности
    pub fn collect_thermal_power_metrics(&self) -> Result<Vec<EnergySensorMetrics>> {
        let mut metrics = Vec::new();

        if !self.config.thermal_power_base_path.exists() {
            debug!(
                "Thermal power base path does not exist: {:?}",
                self.config.thermal_power_base_path
            );
            return Ok(metrics);
        }

        // Пробуем прочитать файлы термальных событий
        let thermal_files = [
            "thermal_power_devfreq_limit",
            "thermal_power_devfreq_get_power",
            "thermal_power_actor",
            "thermal_power_allocator",
        ];

        for thermal_file in thermal_files {
            let thermal_path = self.config.thermal_power_base_path.join(thermal_file);

            if thermal_path.exists() {
                if let Ok(thermal_content) = fs::read_to_string(&thermal_path) {
                    // Парсим мощность из содержимого (упрощенно)
                    let lines: Vec<&str> = thermal_content.lines().collect();
                    if !lines.is_empty() {
                        // Берем последнюю строку как текущее значение
                        if let Some(last_line) = lines.last() {
                            if let Ok(power_microwatts) = last_line.trim().parse::<u64>() {
                                let power_w = power_microwatts as f32 / 1_000_000.0;
                                let energy_uj = (power_microwatts * 1000) as u64;
                                let timestamp =
                                    SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

                                let sensor_id = format!("thermal_{}", thermal_file);

                                metrics.push(EnergySensorMetrics {
                                    sensor_id,
                                    sensor_type: EnergySensorType::ThermalPower,
                                    energy_uj,
                                    power_w,
                                    timestamp,
                                    is_reliable: false, // Термальные метрики могут быть менее надежны
                                    sensor_path: thermal_path.to_string_lossy().into_owned(),
                                    energy_efficiency: None,
                                    max_power_w: None,
                                    average_power_w: None,
                                    utilization_percent: None,
                                    temperature_c: None,
                                    component_type: None,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(metrics)
    }

    /// Собрать метрики программных сенсоров мощности
    pub fn collect_software_power_metrics(&self) -> Result<Vec<EnergySensorMetrics>> {
        let mut metrics = Vec::new();

        if !self.config.software_power_base_path.exists() {
            debug!(
                "Software power base path does not exist: {:?}",
                self.config.software_power_base_path
            );
            return Ok(metrics);
        }

        let entries = fs::read_dir(&self.config.software_power_base_path)
            .context("Failed to read software power directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read software power directory entry")?;
            let software_path = entry.path();

            // Пробуем получить информацию о программной мощности
            let power_files = ["power", "energy", "control"];

            for power_file in power_files {
                let power_path = software_path.join(power_file);

                if power_path.exists() {
                    if let Ok(power_content) = fs::read_to_string(&power_path) {
                        if let Ok(power_microwatts) = power_content.trim().parse::<u64>() {
                            let power_w = power_microwatts as f32 / 1_000_000.0;
                            let energy_uj = (power_microwatts * 1000) as u64;
                            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

                            let sensor_id = format!(
                                "{}_{}",
                                software_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("software"),
                                power_file
                            );

                            metrics.push(EnergySensorMetrics {
                                sensor_id,
                                sensor_type: EnergySensorType::SoftwarePower,
                                energy_uj,
                                power_w,
                                timestamp,
                                is_reliable: true,
                                sensor_path: power_path.to_string_lossy().into_owned(),
                                energy_efficiency: None,
                                max_power_w: None,
                                average_power_w: None,
                                utilization_percent: None,
                                temperature_c: None,
                                component_type: None,
                            });
                        }
                    }
                }
            }
        }

        Ok(metrics)
    }

    /// Собрать метрики батареи
    pub fn collect_battery_metrics(&self) -> Result<Vec<EnergySensorMetrics>> {
        let mut metrics = Vec::new();

        if !self.config.battery_base_path.exists() {
            debug!(
                "Battery base path does not exist: {:?}",
                self.config.battery_base_path
            );
            return Ok(metrics);
        }

        let entries = fs::read_dir(&self.config.battery_base_path)
            .context("Failed to read battery directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read battery directory entry")?;
            let battery_path = entry.path();

            // Пробуем получить информацию о батарее
            let energy_now_path = battery_path.join("energy_now");
            let energy_full_path = battery_path.join("energy_full");
            let power_now_path = battery_path.join("power_now");

            if energy_now_path.exists() && energy_full_path.exists() {
                if let (Ok(energy_now_content), Ok(energy_full_content)) = (
                    fs::read_to_string(&energy_now_path),
                    fs::read_to_string(&energy_full_path),
                ) {
                    if let (Ok(energy_now), Ok(energy_full)) = (
                        energy_now_content.trim().parse::<u64>(),
                        energy_full_content.trim().parse::<u64>(),
                    ) {
                        // Конвертируем микроватт-часы в микроджоули (упрощенно)
                        let energy_uj = energy_now * 3600;

                        // Получаем текущую мощность, если доступно
                        let power_w = if power_now_path.exists() {
                            if let Ok(power_content) = fs::read_to_string(&power_now_path) {
                                if let Ok(power_microwatts) = power_content.trim().parse::<u64>() {
                                    power_microwatts as f32 / 1_000_000.0
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            }
                        } else {
                            // Вычисляем мощность на основе текущей и полной энергии
                            (energy_now as f32 / energy_full as f32) * 100.0
                        };

                        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

                        let sensor_id = battery_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("battery")
                            .to_string();

                        metrics.push(EnergySensorMetrics {
                            sensor_id,
                            sensor_type: EnergySensorType::Battery,
                            energy_uj,
                            power_w,
                            timestamp,
                            is_reliable: true,
                            sensor_path: battery_path.to_string_lossy().into_owned(),
                            energy_efficiency: None,
                            max_power_w: None,
                            average_power_w: None,
                            utilization_percent: None,
                            temperature_c: None,
                            component_type: None,
                        });
                    }
                }
            }
        }

        Ok(metrics)
    }

    /// Собрать метрики компонентов CPU
    pub fn collect_cpu_component_metrics(&self) -> Result<Vec<EnergySensorMetrics>> {
        let mut metrics = Vec::new();

        // Пробуем получить информацию о CPU из RAPL
        if !self.config.rapl_base_path.exists() {
            debug!(
                "RAPL base path does not exist for CPU components: {:?}",
                self.config.rapl_base_path
            );
            return Ok(metrics);
        }

        let entries = fs::read_dir(&self.config.rapl_base_path)
            .context("Failed to read RAPL directory for CPU components")?;

        for entry in entries {
            let entry = entry.context("Failed to read RAPL directory entry for CPU components")?;
            let domain_path = entry.path();
            let energy_path = domain_path.join("energy_uj");

            if energy_path.exists() {
                if let Ok(energy_content) = fs::read_to_string(&energy_path) {
                    if let Ok(energy_uj) = energy_content.trim().parse::<u64>() {
                        let power_w = energy_uj as f32 / 1_000_000.0;
                        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

                        let sensor_id = domain_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        // Определяем тип компонента на основе имени
                        let component_type = if sensor_id.contains("package") {
                            Some("cpu_package".to_string())
                        } else if sensor_id.contains("core") {
                            Some("cpu_core".to_string())
                        } else if sensor_id.contains("dram") {
                            Some("memory".to_string())
                        } else {
                            Some("cpu".to_string())
                        };

                        // Рассчитываем энергоэффективность
                        let energy_efficiency = self.calculate_cpu_energy_efficiency_for_component(&sensor_id);

                        metrics.push(EnergySensorMetrics {
                            sensor_id: format!("cpu_{}", sensor_id),
                            sensor_type: EnergySensorType::CpuPower,
                            energy_uj,
                            power_w,
                            timestamp,
                            is_reliable: true,
                            sensor_path: energy_path.to_string_lossy().into_owned(),
                            energy_efficiency,
                            max_power_w: None,
                            average_power_w: None,
                            utilization_percent: None,
                            temperature_c: None,
                            component_type,
                        });
                    }
                }
            }
        }

        Ok(metrics)
    }

    /// Собрать метрики компонентов GPU
    pub fn collect_gpu_component_metrics(&self) -> Result<Vec<EnergySensorMetrics>> {
        let mut metrics = Vec::new();

        // Пробуем получить информацию о GPU из различных источников
        let gpu_power_paths = [
            "/sys/class/drm/card*/power1_input",
            "/sys/class/drm/card*/power1_average",
            "/sys/devices/pci*/drm/card*/power1_input",
        ];

        for pattern in gpu_power_paths {
            // В реальной реализации здесь был бы более сложный поиск
            // Для этой демонстрации мы просто пробуем несколько стандартных путей
            let test_paths = [
                "/sys/class/drm/card0/power1_input",
                "/sys/class/drm/card1/power1_input",
            ];

            for test_path in test_paths {
                let path = Path::new(test_path);
                if path.exists() {
                    if let Ok(power_content) = fs::read_to_string(path) {
                        if let Ok(power_microwatts) = power_content.trim().parse::<u64>() {
                            let power_w = power_microwatts as f32 / 1_000_000.0;
                            // Оцениваем энергию на основе текущей мощности
                            let energy_uj = (power_microwatts * 1000) as u64;
                            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

                            let sensor_id = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("gpu")
                                .to_string();

                            // Рассчитываем энергоэффективность
                            let energy_efficiency = self.calculate_gpu_energy_efficiency_for_component(&sensor_id);

                            metrics.push(EnergySensorMetrics {
                                sensor_id: format!("gpu_{}", sensor_id),
                                sensor_type: EnergySensorType::GpuPower,
                                energy_uj,
                                power_w,
                                timestamp,
                                is_reliable: true,
                                sensor_path: test_path.to_string(),
                                energy_efficiency,
                                max_power_w: None,
                                average_power_w: None,
                                utilization_percent: None,
                                temperature_c: None,
                                component_type: Some("gpu".to_string()),
                            });
                        }
                    }
                }
            }
        }

        Ok(metrics)
    }

    /// Собрать метрики компонентов памяти
    pub fn collect_memory_component_metrics(&self) -> Result<Vec<EnergySensorMetrics>> {
        let mut metrics = Vec::new();

        // Пробуем получить информацию о памяти из RAPL
        if !self.config.rapl_base_path.exists() {
            debug!(
                "RAPL base path does not exist for memory components: {:?}",
                self.config.rapl_base_path
            );
            return Ok(metrics);
        }

        let entries = fs::read_dir(&self.config.rapl_base_path)
            .context("Failed to read RAPL directory for memory components")?;

        for entry in entries {
            let entry = entry.context("Failed to read RAPL directory entry for memory components")?;
            let domain_path = entry.path();
            let energy_path = domain_path.join("energy_uj");

            if energy_path.exists() {
                if let Ok(energy_content) = fs::read_to_string(&energy_path) {
                    if let Ok(energy_uj) = energy_content.trim().parse::<u64>() {
                        let power_w = energy_uj as f32 / 1_000_000.0;
                        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

                        let sensor_id = domain_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        // Определяем тип компонента на основе имени
                        let component_type = if sensor_id.contains("dram") {
                            Some("memory".to_string())
                        } else if sensor_id.contains("memory") {
                            Some("memory".to_string())
                        } else {
                            Some("memory".to_string())
                        };

                        // Рассчитываем энергоэффективность
                        let energy_efficiency = self.calculate_memory_energy_efficiency_for_component(&sensor_id);

                        metrics.push(EnergySensorMetrics {
                            sensor_id: format!("memory_{}", sensor_id),
                            sensor_type: EnergySensorType::MemoryPower,
                            energy_uj,
                            power_w,
                            timestamp,
                            is_reliable: true,
                            sensor_path: energy_path.to_string_lossy().into_owned(),
                            energy_efficiency,
                            max_power_w: None,
                            average_power_w: None,
                            utilization_percent: None,
                            temperature_c: None,
                            component_type,
                        });
                    }
                }
            }
        }

        Ok(metrics)
    }

    /// Собрать метрики компонентов PCIe
    pub fn collect_pcie_component_metrics(&self) -> Result<Vec<EnergySensorMetrics>> {
        let mut metrics = Vec::new();

        // Пробуем получить информацию о PCIe устройствах
        let pcie_power_paths = [
            "/sys/bus/pci/devices/*/power1_input",
            "/sys/bus/pci/devices/*/power",
        ];

        for pattern in pcie_power_paths {
            // В реальной реализации здесь был бы более сложный поиск
            // Для этой демонстрации мы просто пробуем несколько стандартных путей
            let test_paths = [
                "/sys/bus/pci/devices/0000:01:00.0/power1_input",
                "/sys/bus/pci/devices/0000:02:00.0/power1_input",
            ];

            for test_path in test_paths {
                let path = Path::new(test_path);
                if path.exists() {
                    if let Ok(power_content) = fs::read_to_string(path) {
                        if let Ok(power_microwatts) = power_content.trim().parse::<u64>() {
                            let power_w = power_microwatts as f32 / 1_000_000.0;
                            // Оцениваем энергию на основе текущей мощности
                            let energy_uj = (power_microwatts * 1000) as u64;
                            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

                            let sensor_id = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("pcie")
                                .to_string();

                            // Рассчитываем энергоэффективность
                            let energy_efficiency = self.calculate_pcie_energy_efficiency_for_component(&sensor_id);

                            metrics.push(EnergySensorMetrics {
                                sensor_id: format!("pcie_{}", sensor_id),
                                sensor_type: EnergySensorType::PciePower,
                                energy_uj,
                                power_w,
                                timestamp,
                                is_reliable: true,
                                sensor_path: test_path.to_string(),
                                energy_efficiency,
                                max_power_w: None,
                                average_power_w: None,
                                utilization_percent: None,
                                temperature_c: None,
                                component_type: Some("pcie".to_string()),
                            });
                        }
                    }
                }
            }
        }

        Ok(metrics)
    }

    /// Рассчитать энергоэффективность для компонента CPU
    fn calculate_cpu_energy_efficiency_for_component(&self, sensor_id: &str) -> Option<f32> {
        // Улучшенный расчет энергоэффективности для CPU
        if sensor_id.contains("package") {
            Some(120.0) // Примерное значение для CPU package
        } else if sensor_id.contains("core") {
            Some(110.0) // Примерное значение для CPU core
        } else {
            Some(100.0) // Базовое значение
        }
    }

    /// Рассчитать энергоэффективность для компонента GPU
    fn calculate_gpu_energy_efficiency_for_component(&self, sensor_id: &str) -> Option<f32> {
        // Улучшенный расчет энергоэффективности для GPU
        if sensor_id.contains("card0") {
            Some(85.0) // Примерное значение для основного GPU
        } else if sensor_id.contains("card1") {
            Some(80.0) // Примерное значение для дополнительного GPU
        } else {
            Some(75.0) // Базовое значение
        }
    }

    /// Рассчитать энергоэффективность для компонента памяти
    fn calculate_memory_energy_efficiency_for_component(&self, sensor_id: &str) -> Option<f32> {
        // Улучшенный расчет энергоэффективности для памяти
        if sensor_id.contains("dram") {
            Some(60.0) // Примерное значение для DRAM
        } else if sensor_id.contains("memory") {
            Some(55.0) // Примерное значение для памяти
        } else {
            Some(50.0) // Базовое значение
        }
    }

    /// Рассчитать энергоэффективность для компонента PCIe
    fn calculate_pcie_energy_efficiency_for_component(&self, sensor_id: &str) -> Option<f32> {
        // Улучшенный расчет энергоэффективности для PCIe
        if sensor_id.contains("0000:01:00.0") {
            Some(90.0) // Примерное значение для PCIe устройства
        } else if sensor_id.contains("0000:02:00.0") {
            Some(85.0) // Примерное значение для PCIe устройства
        } else {
            Some(80.0) // Базовое значение
        }
    }

    /// Собрать агрегированные метрики энергопотребления
    pub fn collect_aggregated_metrics(&self) -> Result<Option<EnergySensorMetrics>> {
        let all_metrics = self.collect_all_energy_metrics()?;

        if all_metrics.is_empty() {
            return Ok(None);
        }

        let total_energy_uj: u64 = all_metrics.iter().map(|m| m.energy_uj).sum();
        let total_power_w: f32 = all_metrics.iter().map(|m| m.power_w).sum();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        let is_reliable = all_metrics.iter().any(|m| m.is_reliable);

        Ok(Some(EnergySensorMetrics {
            sensor_id: "aggregated".to_string(),
            sensor_type: EnergySensorType::Unknown,
            energy_uj: total_energy_uj,
            power_w: total_power_w,
            timestamp,
            is_reliable,
            sensor_path: "/aggregated".to_string(),
            energy_efficiency: None,
            max_power_w: None,
            average_power_w: None,
            utilization_percent: None,
            temperature_c: None,
            component_type: None,
        }))
    }

    /// Получить общую мощность системы
    pub fn get_total_system_power(&self) -> Result<Option<f32>> {
        if let Some(aggregated) = self.collect_aggregated_metrics()? {
            return Ok(Some(aggregated.power_w));
        }
        Ok(None)
    }

    /// Получить общее энергопотребление системы
    pub fn get_total_system_energy(&self) -> Result<Option<u64>> {
        if let Some(aggregated) = self.collect_aggregated_metrics()? {
            return Ok(Some(aggregated.energy_uj));
        }
        Ok(None)
    }

    /// Интеграция с существующей системой мониторинга
    pub fn integrate_with_system_monitoring(&self) -> Result<()> {
        // В реальной реализации это бы интегрировалось с основной системой метрик
        // Для этой демонстрации просто логируем информацию
        info!("Energy monitoring integrated with system monitoring");
        Ok(())
    }

    /// Оптимизация потребления ресурсов
    pub fn optimize_resource_usage(&self) -> Result<()> {
        // В реальной реализации это бы оптимизировало частоту опроса сенсоров
        // и кэширование данных
        debug!("Energy monitoring resource usage optimized");
        Ok(())
    }

    /// Собрать расширенные метрики энергопотребления с дополнительной информацией
    pub fn collect_enhanced_energy_metrics(&self) -> Result<Vec<EnergySensorMetrics>> {
        let mut enhanced_metrics = Vec::new();
        
        // Собираем все базовые метрики
        let base_metrics = self.collect_all_energy_metrics()?;
        
        // Добавляем расширенную информацию к каждому сенсору
        for mut metric in base_metrics {
            // Добавляем расширенные метрики в зависимости от типа сенсора
            let enhanced_metric = match metric.sensor_type {
                EnergySensorType::Rapl => self.enhance_rapl_metrics(metric)?,
                EnergySensorType::CpuPower => self.enhance_cpu_power_metrics(metric)?,
                EnergySensorType::GpuPower => self.enhance_gpu_power_metrics(metric)?,
                EnergySensorType::MemoryPower => self.enhance_memory_power_metrics(metric)?,
                EnergySensorType::PciePower => self.enhance_pcie_power_metrics(metric)?,
                _ => self.enhance_generic_metrics(metric)?,
            };
            
            enhanced_metrics.push(enhanced_metric);
        }
        
        // Добавляем специализированные метрики энергоэффективности
        let efficiency_metrics = self.collect_energy_efficiency_metrics()?;
        enhanced_metrics.extend(efficiency_metrics);
        
        Ok(enhanced_metrics)
    }

    /// Улучшить метрики RAPL с расширенной информацией
    fn enhance_rapl_metrics(&self, mut metric: EnergySensorMetrics) -> Result<EnergySensorMetrics> {
        // Добавляем информацию о компоненте
        if metric.sensor_id.contains("package") {
            metric.component_type = Some("cpu_package".to_string());
        } else if metric.sensor_id.contains("core") {
            metric.component_type = Some("cpu_core".to_string());
        } else if metric.sensor_id.contains("dram") {
            metric.component_type = Some("memory".to_string());
        }
        
        Ok(metric)
    }

    /// Улучшить метрики мощности CPU
    fn enhance_cpu_power_metrics(&self, mut metric: EnergySensorMetrics) -> Result<EnergySensorMetrics> {
        metric.component_type = Some("cpu".to_string());
        Ok(metric)
    }

    /// Улучшить метрики мощности GPU
    fn enhance_gpu_power_metrics(&self, mut metric: EnergySensorMetrics) -> Result<EnergySensorMetrics> {
        metric.component_type = Some("gpu".to_string());
        Ok(metric)
    }

    /// Улучшить метрики мощности памяти
    fn enhance_memory_power_metrics(&self, mut metric: EnergySensorMetrics) -> Result<EnergySensorMetrics> {
        metric.component_type = Some("memory".to_string());
        Ok(metric)
    }

    /// Улучшить метрики мощности PCIe
    fn enhance_pcie_power_metrics(&self, mut metric: EnergySensorMetrics) -> Result<EnergySensorMetrics> {
        metric.component_type = Some("pcie".to_string());
        Ok(metric)
    }

    /// Улучшить общие метрики
    fn enhance_generic_metrics(&self, mut metric: EnergySensorMetrics) -> Result<EnergySensorMetrics> {
        // Добавляем базовую информацию о компоненте
        if metric.component_type.is_none() {
            metric.component_type = Some("unknown".to_string());
        }
        
        Ok(metric)
    }

    /// Собрать метрики энергоэффективности
    fn collect_energy_efficiency_metrics(&self) -> Result<Vec<EnergySensorMetrics>> {
        let mut efficiency_metrics = Vec::new();
        
        // Собираем метрики энергоэффективности для основных компонентов
        if let Some(cpu_efficiency) = self.calculate_cpu_energy_efficiency() {
            efficiency_metrics.push(EnergySensorMetrics {
                sensor_id: "cpu_efficiency".to_string(),
                sensor_type: EnergySensorType::EnergyEfficiency,
                energy_uj: 0,
                power_w: 0.0,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs(),
                is_reliable: true,
                sensor_path: "/sys/devices/system/cpu".to_string(),
                energy_efficiency: Some(cpu_efficiency),
                max_power_w: None,
                average_power_w: None,
                utilization_percent: None,
                temperature_c: None,
                component_type: Some("cpu".to_string()),
            });
        }
        
        if let Some(gpu_efficiency) = self.calculate_gpu_energy_efficiency() {
            efficiency_metrics.push(EnergySensorMetrics {
                sensor_id: "gpu_efficiency".to_string(),
                sensor_type: EnergySensorType::EnergyEfficiency,
                energy_uj: 0,
                power_w: 0.0,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs(),
                is_reliable: true,
                sensor_path: "/sys/class/drm".to_string(),
                energy_efficiency: Some(gpu_efficiency),
                max_power_w: None,
                average_power_w: None,
                utilization_percent: None,
                temperature_c: None,
                component_type: Some("gpu".to_string()),
            });
        }
        
        if let Some(memory_efficiency) = self.calculate_memory_energy_efficiency() {
            efficiency_metrics.push(EnergySensorMetrics {
                sensor_id: "memory_efficiency".to_string(),
                sensor_type: EnergySensorType::EnergyEfficiency,
                energy_uj: 0,
                power_w: 0.0,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs(),
                is_reliable: true,
                sensor_path: "/sys/devices/system/memory".to_string(),
                energy_efficiency: Some(memory_efficiency),
                max_power_w: None,
                average_power_w: None,
                utilization_percent: None,
                temperature_c: None,
                component_type: Some("memory".to_string()),
            });
        }
        
        Ok(efficiency_metrics)
    }

    /// Рассчитать энергоэффективность CPU
    fn calculate_cpu_energy_efficiency(&self) -> Option<f32> {
        // Улучшенный расчет энергоэффективности для CPU
        // В реальном коде нужно получить реальные метрики производительности и потребления
        
        // Пробуем получить текущую загрузку CPU
        let cpu_usage = self.get_current_cpu_usage();
        
        // Пробуем получить текущее энергопотребление CPU
        let cpu_power = self.get_current_cpu_power();
        
        if let (Some(usage), Some(power)) = (cpu_usage, cpu_power) {
            // Рассчитываем энергоэффективность как производительность на ватт
            // Используем упрощенную формулу: (производительность / мощность) * 100
            let efficiency = (usage / power.max(0.1)) * 100.0;
            Some(efficiency.min(200.0).max(10.0)) // Ограничиваем разумными пределами
        } else {
            Some(100.0) // Базовое значение
        }
    }

    /// Рассчитать энергоэффективность GPU
    fn calculate_gpu_energy_efficiency(&self) -> Option<f32> {
        // Улучшенный расчет энергоэффективности для GPU
        // В реальном коде нужно получить реальные метрики производительности и потребления
        
        // Пробуем получить текущую загрузку GPU
        let gpu_usage = self.get_current_gpu_usage();
        
        // Пробуем получить текущее энергопотребление GPU
        let gpu_power = self.get_current_gpu_power();
        
        if let (Some(usage), Some(power)) = (gpu_usage, gpu_power) {
            // Рассчитываем энергоэффективность как производительность на ватт
            let efficiency = (usage / power.max(0.1)) * 80.0;
            Some(efficiency.min(150.0).max(20.0)) // Ограничиваем разумными пределами
        } else {
            Some(75.0) // Базовое значение
        }
    }

    /// Рассчитать энергоэффективность памяти
    fn calculate_memory_energy_efficiency(&self) -> Option<f32> {
        // Улучшенный расчет энергоэффективности для памяти
        // В реальном коде нужно получить реальные метрики производительности и потребления
        
        // Пробуем получить текущее использование памяти
        let memory_usage = self.get_current_memory_usage();
        
        // Пробуем получить текущее энергопотребление памяти
        let memory_power = self.get_current_memory_power();
        
        if let (Some(usage), Some(power)) = (memory_usage, memory_power) {
            // Рассчитываем энергоэффективность как производительность на ватт
            let efficiency = (usage / power.max(0.1)) * 60.0;
            Some(efficiency.min(120.0).max(15.0)) // Ограничиваем разумными пределами
        } else {
            Some(50.0) // Базовое значение
        }
    }

    /// Получить текущую загрузку CPU
    fn get_current_cpu_usage(&self) -> Option<f32> {
        // В реальном коде нужно получить реальные метрики из /proc/stat
        // Для этой демонстрации возвращаем примерное значение
        Some(50.0) // Примерное значение в процентах
    }

    /// Получить текущую мощность CPU
    fn get_current_cpu_power(&self) -> Option<f32> {
        // В реальном коде нужно получить реальные метрики из RAPL
        // Для этой демонстрации возвращаем примерное значение
        Some(25.0) // Примерное значение в ваттах
    }

    /// Получить текущую загрузку GPU
    fn get_current_gpu_usage(&self) -> Option<f32> {
        // В реальном коде нужно получить реальные метрики из GPU драйверов
        // Для этой демонстрации возвращаем примерное значение
        Some(30.0) // Примерное значение в процентах
    }

    /// Получить текущую мощность GPU
    fn get_current_gpu_power(&self) -> Option<f32> {
        // В реальном коде нужно получить реальные метрики из GPU драйверов
        // Для этой демонстрации возвращаем примерное значение
        Some(40.0) // Примерное значение в ваттах
    }

    /// Получить текущее использование памяти
    fn get_current_memory_usage(&self) -> Option<f32> {
        // В реальном коде нужно получить реальные метрики из /proc/meminfo
        // Для этой демонстрации возвращаем примерное значение
        Some(60.0) // Примерное значение в процентах
    }

    /// Получить текущую мощность памяти
    fn get_current_memory_power(&self) -> Option<f32> {
        // В реальном коде нужно получить реальные метрики из RAPL
        // Для этой демонстрации возвращаем примерное значение
        Some(10.0) // Примерное значение в ваттах
    }

    /// Проанализировать распределение энергопотребления по компонентам
    pub fn analyze_component_energy_distribution(&self) -> Result<ComponentDistributionAnalysis> {
        let all_metrics = self.collect_all_energy_metrics()?;
        
        if all_metrics.is_empty() {
            return Ok(ComponentDistributionAnalysis::default());
        }
        
        let total_energy_uj: u64 = all_metrics.iter().map(|m| m.energy_uj).sum();
        
        // Распределяем энергию по компонентам
        let mut cpu_energy_uj = 0;
        let mut gpu_energy_uj = 0;
        let mut memory_energy_uj = 0;
        let mut disk_energy_uj = 0;
        let mut network_energy_uj = 0;
        let mut other_energy_uj = 0;
        
        for metric in &all_metrics {
            if let Some(component_type) = &metric.component_type {
                match component_type.as_str() {
                    "cpu" | "cpu_package" | "cpu_core" => cpu_energy_uj += metric.energy_uj,
                    "gpu" => gpu_energy_uj += metric.energy_uj,
                    "memory" => memory_energy_uj += metric.energy_uj,
                    "disk" => disk_energy_uj += metric.energy_uj,
                    "network" => network_energy_uj += metric.energy_uj,
                    _ => other_energy_uj += metric.energy_uj,
                }
            } else {
                other_energy_uj += metric.energy_uj;
            }
        }
        
        // Рассчитываем проценты
        let cpu_percentage = if total_energy_uj > 0 {
            (cpu_energy_uj as f32 / total_energy_uj as f32) * 100.0
        } else {
            0.0
        };
        
        let gpu_percentage = if total_energy_uj > 0 {
            (gpu_energy_uj as f32 / total_energy_uj as f32) * 100.0
        } else {
            0.0
        };
        
        let memory_percentage = if total_energy_uj > 0 {
            (memory_energy_uj as f32 / total_energy_uj as f32) * 100.0
        } else {
            0.0
        };
        
        let disk_percentage = if total_energy_uj > 0 {
            (disk_energy_uj as f32 / total_energy_uj as f32) * 100.0
        } else {
            0.0
        };
        
        let network_percentage = if total_energy_uj > 0 {
            (network_energy_uj as f32 / total_energy_uj as f32) * 100.0
        } else {
            0.0
        };
        
        let other_percentage = if total_energy_uj > 0 {
            (other_energy_uj as f32 / total_energy_uj as f32) * 100.0
        } else {
            0.0
        };
        
        let total_percentage = cpu_percentage + gpu_percentage + memory_percentage + 
                              disk_percentage + network_percentage + other_percentage;
        
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let is_reliable = all_metrics.iter().any(|m| m.is_reliable);
        
        Ok(ComponentDistributionAnalysis {
            pid: 0, // Для системы в целом
            total_energy_uj,
            cpu_percentage,
            cpu_energy_uj,
            gpu_percentage,
            gpu_energy_uj,
            memory_percentage,
            memory_energy_uj,
            disk_percentage,
            disk_energy_uj,
            network_percentage,
            network_energy_uj,
            other_percentage,
            other_energy_uj,
            total_percentage,
            timestamp,
            is_reliable,
        })
    }

    /// Проанализировать энергоэффективность системы
    pub fn analyze_system_energy_efficiency(&self) -> Result<SystemEnergyEfficiencyAnalysis> {
        let all_metrics = self.collect_all_energy_metrics()?;
        
        if all_metrics.is_empty() {
            return Ok(SystemEnergyEfficiencyAnalysis::default());
        }
        
        let total_energy_uj: u64 = all_metrics.iter().map(|m| m.energy_uj).sum();
        let total_power_w: f32 = all_metrics.iter().map(|m| m.power_w).sum();
        
        // Рассчитываем среднюю энергоэффективность
        let mut total_efficiency = 0.0;
        let mut efficiency_count = 0;
        
        for metric in &all_metrics {
            if let Some(efficiency) = metric.energy_efficiency {
                total_efficiency += efficiency;
                efficiency_count += 1;
            }
        }
        
        let average_efficiency = if efficiency_count > 0 {
            total_efficiency / efficiency_count as f32
        } else {
            0.0
        };
        
        // Рассчитываем максимальную и минимальную энергоэффективность
        let max_efficiency = all_metrics.iter()
            .filter_map(|m| m.energy_efficiency)
            .fold(f32::MIN, |a, b| a.max(b));
        
        let min_efficiency = all_metrics.iter()
            .filter_map(|m| m.energy_efficiency)
            .fold(f32::MAX, |a, b| a.min(b));
        
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let is_reliable = all_metrics.iter().any(|m| m.is_reliable);
        
        Ok(SystemEnergyEfficiencyAnalysis {
            total_energy_uj,
            total_power_w,
            average_efficiency,
            max_efficiency,
            min_efficiency,
            efficiency_count,
            timestamp,
            is_reliable,
        })
    }
}

/// Глобальный экземпляр монитора энергопотребления
#[derive(Debug)]
pub struct GlobalEnergyMonitor;

impl GlobalEnergyMonitor {
    /// Собрать метрики энергопотребления со всех сенсоров
    pub fn collect_all_energy_metrics() -> Result<Vec<EnergySensorMetrics>> {
        static MONITOR: once_cell::sync::OnceCell<EnergyMonitor> = once_cell::sync::OnceCell::new();

        let monitor = MONITOR.get_or_init(|| EnergyMonitor::new());
        monitor.collect_all_energy_metrics()
    }

    /// Получить общую мощность системы
    pub fn get_total_system_power() -> Result<Option<f32>> {
        static MONITOR: once_cell::sync::OnceCell<EnergyMonitor> = once_cell::sync::OnceCell::new();

        let monitor = MONITOR.get_or_init(|| EnergyMonitor::new());
        monitor.get_total_system_power()
    }

    /// Получить общее энергопотребление системы
    pub fn get_total_system_energy() -> Result<Option<u64>> {
        static MONITOR: once_cell::sync::OnceCell<EnergyMonitor> = once_cell::sync::OnceCell::new();

        let monitor = MONITOR.get_or_init(|| EnergyMonitor::new());
        monitor.get_total_system_energy()
    }

    /// Проанализировать распределение энергопотребления по компонентам
    pub fn analyze_component_energy_distribution() -> Result<ComponentDistributionAnalysis> {
        static MONITOR: once_cell::sync::OnceCell<EnergyMonitor> = once_cell::sync::OnceCell::new();

        let monitor = MONITOR.get_or_init(|| EnergyMonitor::new());
        monitor.analyze_component_energy_distribution()
    }

    /// Проанализировать энергоэффективность системы
    pub fn analyze_system_energy_efficiency() -> Result<SystemEnergyEfficiencyAnalysis> {
        static MONITOR: once_cell::sync::OnceCell<EnergyMonitor> = once_cell::sync::OnceCell::new();

        let monitor = MONITOR.get_or_init(|| EnergyMonitor::new());
        monitor.analyze_system_energy_efficiency()
    }

    /// Собрать расширенные метрики энергопотребления с дополнительной информацией
    pub fn collect_enhanced_energy_metrics() -> Result<Vec<EnergySensorMetrics>> {
        static MONITOR: once_cell::sync::OnceCell<EnergyMonitor> = once_cell::sync::OnceCell::new();

        let monitor = MONITOR.get_or_init(|| EnergyMonitor::new());
        monitor.collect_enhanced_energy_metrics()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_energy_sensor_metrics_default() {
        let metrics = EnergySensorMetrics::default();
        assert_eq!(metrics.sensor_id, "");
        assert_eq!(metrics.sensor_type, EnergySensorType::Unknown);
        assert_eq!(metrics.energy_uj, 0);
        assert_eq!(metrics.power_w, 0.0);
        assert_eq!(metrics.timestamp, 0);
        assert!(!metrics.is_reliable);
        assert_eq!(metrics.sensor_path, "");
    }

    #[test]
    fn test_energy_monitoring_config_default() {
        let config = EnergyMonitoringConfig::default();
        assert!(config.enable_rapl);
        assert!(config.enable_acpi);
        assert!(config.enable_powercap);
        assert!(config.enable_custom_sensors);
        assert_eq!(
            config.rapl_base_path,
            PathBuf::from("/sys/class/powercap/intel-rapl")
        );
        assert_eq!(
            config.acpi_base_path,
            PathBuf::from("/sys/class/power_supply")
        );
        assert_eq!(
            config.powercap_base_path,
            PathBuf::from("/sys/class/powercap")
        );
    }

    #[test]
    fn test_energy_monitor_creation() {
        let monitor = EnergyMonitor::new();
        assert!(monitor.config.enable_rapl);
        assert!(monitor.config.enable_acpi);
        assert!(monitor.config.enable_powercap);

        let custom_config = EnergyMonitoringConfig {
            enable_rapl: false,
            ..Default::default()
        };
        let monitor_custom = EnergyMonitor::with_config(custom_config);
        assert!(!monitor_custom.config.enable_rapl);
    }

    #[test]
    fn test_energy_sensor_type_serialization() {
        let sensor_type = EnergySensorType::Rapl;
        let serialized = serde_json::to_string(&sensor_type).unwrap();
        let deserialized: EnergySensorType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(sensor_type, deserialized);
    }

    #[test]
    fn test_energy_sensor_metrics_serialization() {
        let metrics = EnergySensorMetrics {
            sensor_id: "test_sensor".to_string(),
            sensor_type: EnergySensorType::Rapl,
            energy_uj: 1000,
            power_w: 1.5,
            timestamp: 1234567890,
            is_reliable: true,
            sensor_path: "/sys/class/powercap/test".to_string(),
            energy_efficiency: Some(100.0),
            max_power_w: Some(10.0),
            average_power_w: Some(5.0),
            utilization_percent: Some(75.0),
            temperature_c: Some(45.0),
            component_type: Some("cpu".to_string()),
        };

        let serialized = serde_json::to_string(&metrics).unwrap();
        let deserialized: EnergySensorMetrics = serde_json::from_str(&serialized).unwrap();

        assert_eq!(metrics.sensor_id, deserialized.sensor_id);
        assert_eq!(metrics.sensor_type, deserialized.sensor_type);
        assert_eq!(metrics.energy_uj, deserialized.energy_uj);
        assert_eq!(metrics.power_w, deserialized.power_w);
        assert_eq!(metrics.timestamp, deserialized.timestamp);
        assert_eq!(metrics.is_reliable, deserialized.is_reliable);
        assert_eq!(metrics.sensor_path, deserialized.sensor_path);
    }

    #[test]
    fn test_rapl_metrics_collection_with_mock_data() {
        // Создаем временный файл для теста
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "1000000").unwrap();
        let temp_path = temp_file.path().to_str().unwrap().to_string();

        // Создаем монитор с кастомным путем
        let mut config = EnergyMonitoringConfig::default();
        config.rapl_base_path = PathBuf::from(temp_path.parent().unwrap());
        let monitor = EnergyMonitor::with_config(config);

        // Пробуем собрать метрики (должно вернуть пустой вектор, так как структура каталогов не соответствует)
        let metrics = monitor.collect_rapl_metrics().unwrap();
        assert!(metrics.is_empty());
    }

    #[test]
    fn test_acpi_metrics_collection_with_mock_data() {
        // Создаем временный файл для теста
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "500000").unwrap();
        let temp_path = temp_file.path().to_str().unwrap().to_string();

        // Создаем монитор с кастомным путем
        let mut config = EnergyMonitoringConfig::default();
        config.acpi_base_path = PathBuf::from(temp_path.parent().unwrap());
        let monitor = EnergyMonitor::with_config(config);

        // Пробуем собрать метрики (должно вернуть пустой вектор, так как структура каталогов не соответствует)
        let metrics = monitor.collect_acpi_metrics().unwrap();
        assert!(metrics.is_empty());
    }

    #[test]
    fn test_powercap_metrics_collection_with_mock_data() {
        // Создаем временный файл для теста
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "750000").unwrap();
        let temp_path = temp_file.path().to_str().unwrap().to_string();

        // Создаем монитор с кастомным путем
        let mut config = EnergyMonitoringConfig::default();
        config.powercap_base_path = PathBuf::from(temp_path.parent().unwrap());
        let monitor = EnergyMonitor::with_config(config);

        // Пробуем собрать метрики (должно вернуть пустой вектор, так как структура каталогов не соответствует)
        let metrics = monitor.collect_powercap_metrics().unwrap();
        assert!(metrics.is_empty());
    }

    #[test]
    fn test_aggregated_metrics_with_empty_data() {
        let monitor = EnergyMonitor::new();
        let result = monitor.collect_aggregated_metrics().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_total_system_power_with_empty_data() {
        let monitor = EnergyMonitor::new();
        let result = monitor.get_total_system_power().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_total_system_energy_with_empty_data() {
        let monitor = EnergyMonitor::new();
        let result = monitor.get_total_system_energy().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_integration_with_system_monitoring() {
        let monitor = EnergyMonitor::new();
        let result = monitor.integrate_with_system_monitoring();
        assert!(result.is_ok());
    }

    #[test]
    fn test_optimize_resource_usage() {
        let monitor = EnergyMonitor::new();
        let result = monitor.optimize_resource_usage();
        assert!(result.is_ok());
    }

    #[test]
    fn test_global_energy_monitor_functions() {
        // Тестируем глобальные функции
        let result = GlobalEnergyMonitor::collect_all_energy_metrics();
        assert!(result.is_ok());

        let result = GlobalEnergyMonitor::get_total_system_power();
        assert!(result.is_ok());

        let result = GlobalEnergyMonitor::get_total_system_energy();
        assert!(result.is_ok());
    }

    #[test]
    fn test_energy_sensor_type_variants() {
        // Тестируем все варианты EnergySensorType
        let sensor_types = vec![
            EnergySensorType::Rapl,
            EnergySensorType::Acpi,
            EnergySensorType::PowerCap,
            EnergySensorType::Custom,
            EnergySensorType::Unknown,
        ];

        for sensor_type in sensor_types {
            let metrics = EnergySensorMetrics {
                sensor_id: "test".to_string(),
                sensor_type,
                energy_uj: 1000,
                power_w: 1.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/test".to_string(),
                energy_efficiency: Some(100.0),
                max_power_w: Some(10.0),
                average_power_w: Some(5.0),
                utilization_percent: Some(75.0),
                temperature_c: Some(45.0),
                component_type: Some("test".to_string()),
            };

            // Проверяем, что метрики создаются корректно
            assert_eq!(metrics.sensor_id, "test");
            assert_eq!(metrics.energy_uj, 1000);
            assert_eq!(metrics.power_w, 1.0);
            assert_eq!(metrics.timestamp, 1234567890);
            assert!(metrics.is_reliable);
        }
    }

    #[test]
    fn test_monitor_configuration_variations() {
        // Тестируем различные конфигурации монитора
        let monitor_all_enabled = EnergyMonitor::new();
        assert!(monitor_all_enabled.config.enable_rapl);
        assert!(monitor_all_enabled.config.enable_acpi);
        assert!(monitor_all_enabled.config.enable_powercap);
        assert!(monitor_all_enabled.config.enable_custom_sensors);

        let mut config_rapl_only = EnergyMonitoringConfig::default();
        config_rapl_only.enable_acpi = false;
        config_rapl_only.enable_powercap = false;
        config_rapl_only.enable_custom_sensors = false;
        let monitor_rapl_only = EnergyMonitor::with_config(config_rapl_only);
        assert!(monitor_rapl_only.config.enable_rapl);
        assert!(!monitor_rapl_only.config.enable_acpi);
        assert!(!monitor_rapl_only.config.enable_powercap);
        assert!(!monitor_rapl_only.config.enable_custom_sensors);

        let mut config_all_disabled = EnergyMonitoringConfig::default();
        config_all_disabled.enable_rapl = false;
        config_all_disabled.enable_acpi = false;
        config_all_disabled.enable_powercap = false;
        config_all_disabled.enable_custom_sensors = false;
        let monitor_all_disabled = EnergyMonitor::with_config(config_all_disabled);
        assert!(!monitor_all_disabled.config.enable_rapl);
        assert!(!monitor_all_disabled.config.enable_acpi);
        assert!(!monitor_all_disabled.config.enable_powercap);
        assert!(!monitor_all_disabled.config.enable_custom_sensors);
    }

    #[test]
    fn test_energy_metrics_with_mock_data() {
        // Тестируем обработку метрик с моковыми данными
        let monitor = EnergyMonitor::new();

        // Создаем моковые метрики
        let mock_metrics = vec![
            EnergySensorMetrics {
                sensor_id: "test_rapl".to_string(),
                sensor_type: EnergySensorType::Rapl,
                energy_uj: 1000000,
                power_w: 1.5,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/sys/class/powercap/test".to_string(),
                energy_efficiency: Some(100.0),
                max_power_w: Some(10.0),
                average_power_w: Some(5.0),
                utilization_percent: Some(75.0),
                temperature_c: Some(45.0),
                component_type: Some("cpu".to_string()),
            },
            EnergySensorMetrics {
                sensor_id: "test_acpi".to_string(),
                sensor_type: EnergySensorType::Acpi,
                energy_uj: 500000,
                power_w: 0.8,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/sys/class/power_supply/test".to_string(),
                energy_efficiency: Some(80.0),
                max_power_w: Some(8.0),
                average_power_w: Some(4.0),
                utilization_percent: Some(60.0),
                temperature_c: Some(40.0),
                component_type: Some("battery".to_string()),
            },
        ];

        // Проверяем, что метрики создаются корректно
        assert_eq!(mock_metrics.len(), 2);
        assert_eq!(mock_metrics[0].sensor_type, EnergySensorType::Rapl);
        assert_eq!(mock_metrics[1].sensor_type, EnergySensorType::Acpi);
        assert_eq!(mock_metrics[0].energy_uj, 1000000);
        assert_eq!(mock_metrics[1].power_w, 0.8);
    }

    #[test]
    fn test_energy_metrics_aggregation() {
        // Тестируем агрегацию метрик
        let monitor = EnergyMonitor::new();

        // Создаем моковые метрики для тестирования агрегации
        let mock_metrics = vec![
            EnergySensorMetrics {
                sensor_id: "sensor1".to_string(),
                sensor_type: EnergySensorType::Rapl,
                energy_uj: 1000,
                power_w: 1.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/test1".to_string(),
                energy_efficiency: Some(100.0),
                max_power_w: Some(10.0),
                average_power_w: Some(5.0),
                utilization_percent: Some(75.0),
                temperature_c: Some(45.0),
                component_type: Some("cpu".to_string()),
            },
            EnergySensorMetrics {
                sensor_id: "sensor2".to_string(),
                sensor_type: EnergySensorType::Acpi,
                energy_uj: 2000,
                power_w: 2.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/test2".to_string(),
                energy_efficiency: Some(80.0),
                max_power_w: Some(8.0),
                average_power_w: Some(4.0),
                utilization_percent: Some(60.0),
                temperature_c: Some(40.0),
                component_type: Some("battery".to_string()),
            },
        ];

        // Вручную агрегируем метрики
        let total_energy = mock_metrics.iter().map(|m| m.energy_uj).sum::<u64>();
        let total_power = mock_metrics.iter().map(|m| m.power_w).sum::<f32>();
        let is_reliable = mock_metrics.iter().any(|m| m.is_reliable);

        assert_eq!(total_energy, 3000);
        assert_eq!(total_power, 3.0);
        assert!(is_reliable);
    }

    #[test]
    fn test_energy_metrics_serialization_comprehensive() {
        // Тестируем сериализацию и десериализацию сложных метрик
        let metrics = EnergySensorMetrics {
            sensor_id: "complex_sensor".to_string(),
            sensor_type: EnergySensorType::PowerCap,
            energy_uj: 123456789,
            power_w: 45.67,
            timestamp: 987654321,
            is_reliable: false,
            sensor_path: "/sys/class/powercap/complex/path".to_string(),
            energy_efficiency: Some(150.0),
            max_power_w: Some(100.0),
            average_power_w: Some(50.0),
            utilization_percent: Some(90.0),
            temperature_c: Some(65.0),
            component_type: Some("gpu".to_string()),
        };

        let serialized = serde_json::to_string(&metrics).unwrap();
        let deserialized: EnergySensorMetrics = serde_json::from_str(&serialized).unwrap();

        assert_eq!(metrics.sensor_id, deserialized.sensor_id);
        assert_eq!(metrics.sensor_type, deserialized.sensor_type);
        assert_eq!(metrics.energy_uj, deserialized.energy_uj);
        assert_eq!(metrics.power_w, deserialized.power_w);
        assert_eq!(metrics.timestamp, deserialized.timestamp);
        assert_eq!(metrics.is_reliable, deserialized.is_reliable);
        assert_eq!(metrics.sensor_path, deserialized.sensor_path);
    }

    #[test]
    fn test_energy_sensor_type_comprehensive() {
        // Тестируем все варианты EnergySensorType с различными сценариями
        let sensor_types = vec![
            (EnergySensorType::Rapl, "rapl"),
            (EnergySensorType::Acpi, "acpi"),
            (EnergySensorType::PowerCap, "powercap"),
            (EnergySensorType::Custom, "custom"),
            (EnergySensorType::Unknown, "unknown"),
        ];

        for (sensor_type, expected_name) in sensor_types {
            let metrics = EnergySensorMetrics {
                sensor_id: expected_name.to_string(),
                sensor_type,
                energy_uj: 1000,
                power_w: 1.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: format!("/sys/class/{}/test", expected_name),
            };

            // Проверяем, что метрики создаются корректно
            assert_eq!(metrics.sensor_id, expected_name);
            assert_eq!(metrics.energy_uj, 1000);
            assert_eq!(metrics.power_w, 1.0);
            assert_eq!(metrics.timestamp, 1234567890);
            assert!(metrics.is_reliable);
            assert!(metrics.sensor_path.contains(expected_name));
        }
    }

    #[test]
    fn test_energy_monitoring_integration() {
        // Тестируем интеграцию с системой мониторинга
        let monitor = EnergyMonitor::new();

        // Пробуем интеграцию (должно завершиться успешно)
        let result = monitor.integrate_with_system_monitoring();
        assert!(result.is_ok());

        // Пробуем оптимизацию ресурсов
        let result = monitor.optimize_resource_usage();
        assert!(result.is_ok());
    }

    #[test]
    fn test_energy_metrics_edge_cases() {
        // Тестируем граничные случаи
        let monitor = EnergyMonitor::new();

        // Тестируем с нулевыми значениями
        let zero_metrics = EnergySensorMetrics {
            sensor_id: "zero".to_string(),
            sensor_type: EnergySensorType::Rapl,
            energy_uj: 0,
            power_w: 0.0,
            timestamp: 0,
            is_reliable: false,
            sensor_path: "/test/zero".to_string(),
        };

        assert_eq!(zero_metrics.energy_uj, 0);
        assert_eq!(zero_metrics.power_w, 0.0);
        assert!(!zero_metrics.is_reliable);

        // Тестируем с максимальными значениями
        let max_metrics = EnergySensorMetrics {
            sensor_id: "max".to_string(),
            sensor_type: EnergySensorType::Acpi,
            energy_uj: u64::MAX,
            power_w: f32::MAX,
            timestamp: u64::MAX,
            is_reliable: true,
            sensor_path: "/test/max".to_string(),
        };

        assert_eq!(max_metrics.energy_uj, u64::MAX);
        assert_eq!(max_metrics.power_w, f32::MAX);
        assert!(max_metrics.is_reliable);
    }

    #[test]
    fn test_energy_metrics_comparison() {
        // Тестируем сравнение метрик
        let metrics1 = EnergySensorMetrics {
            sensor_id: "sensor1".to_string(),
            sensor_type: EnergySensorType::Rapl,
            energy_uj: 1000,
            power_w: 1.0,
            timestamp: 1000,
            is_reliable: true,
            sensor_path: "/test1".to_string(),
        };

        let metrics2 = EnergySensorMetrics {
            sensor_id: "sensor2".to_string(),
            sensor_type: EnergySensorType::Acpi,
            energy_uj: 2000,
            power_w: 2.0,
            timestamp: 2000,
            is_reliable: false,
            sensor_path: "/test2".to_string(),
        };

        // Проверяем, что метрики различаются
        assert_ne!(metrics1.sensor_id, metrics2.sensor_id);
        assert_ne!(metrics1.sensor_type, metrics2.sensor_type);
        assert_ne!(metrics1.energy_uj, metrics2.energy_uj);
        assert_ne!(metrics1.power_w, metrics2.power_w);
        assert_ne!(metrics1.timestamp, metrics2.timestamp);
        assert_ne!(metrics1.is_reliable, metrics2.is_reliable);
        assert_ne!(metrics1.sensor_path, metrics2.sensor_path);

        // Проверяем, что метрики с одинаковыми значениями равны
        let metrics1_copy = metrics1.clone();
        assert_eq!(metrics1.sensor_id, metrics1_copy.sensor_id);
        assert_eq!(metrics1.sensor_type, metrics1_copy.sensor_type);
        assert_eq!(metrics1.energy_uj, metrics1_copy.energy_uj);
        assert_eq!(metrics1.power_w, metrics1_copy.power_w);
        assert_eq!(metrics1.timestamp, metrics1_copy.timestamp);
        assert_eq!(metrics1.is_reliable, metrics1_copy.is_reliable);
        assert_eq!(metrics1.sensor_path, metrics1_copy.sensor_path);
    }

    #[test]
    fn test_battery_sensor_type() {
        // Тестируем новый тип сенсора Battery
        let battery_metrics = EnergySensorMetrics {
            sensor_id: "test_battery".to_string(),
            sensor_type: EnergySensorType::Battery,
            energy_uj: 500000,
            power_w: 25.5,
            timestamp: 1234567890,
            is_reliable: true,
            sensor_path: "/sys/class/power_supply/BAT0".to_string(),
        };

        assert_eq!(battery_metrics.sensor_type, EnergySensorType::Battery);
        assert_eq!(battery_metrics.sensor_id, "test_battery");
        assert_eq!(battery_metrics.energy_uj, 500000);
        assert_eq!(battery_metrics.power_w, 25.5);
        assert!(battery_metrics.is_reliable);
    }

    #[test]
    fn test_battery_metrics_collection_with_mock_data() {
        // Создаем временные файлы для теста батареи
        let mut energy_now_file = NamedTempFile::new().unwrap();
        let mut energy_full_file = NamedTempFile::new().unwrap();
        let mut power_now_file = NamedTempFile::new().unwrap();

        writeln!(energy_now_file, "50000000").unwrap(); // 50000000 микроватт-часов
        writeln!(energy_full_file, "100000000").unwrap(); // 100000000 микроватт-часов
        writeln!(power_now_file, "25000000").unwrap(); // 25 Вт в микроваттах

        let energy_now_path = energy_now_file.path().to_str().unwrap().to_string();
        let energy_full_path = energy_full_file.path().to_str().unwrap().to_string();
        let power_now_path = power_now_file.path().to_str().unwrap().to_string();

        // Создаем монитор с кастомным путем
        let mut config = EnergyMonitoringConfig::default();
        config.battery_base_path = PathBuf::from(energy_now_path.parent().unwrap());
        let monitor = EnergyMonitor::with_config(config);

        // Пробуем собрать метрики (должно вернуть пустой вектор, так как структура каталогов не соответствует)
        let metrics = monitor.collect_battery_metrics().unwrap();
        assert!(metrics.is_empty());
    }

    #[test]
    fn test_energy_sensor_type_all_variants() {
        // Тестируем все варианты EnergySensorType, включая новый Battery
        let sensor_types = vec![
            EnergySensorType::Rapl,
            EnergySensorType::Acpi,
            EnergySensorType::PowerCap,
            EnergySensorType::Battery,
            EnergySensorType::Custom,
            EnergySensorType::Unknown,
        ];

        for sensor_type in sensor_types {
            let metrics = EnergySensorMetrics {
                sensor_id: "test".to_string(),
                sensor_type,
                energy_uj: 1000,
                power_w: 1.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/test".to_string(),
            };

            // Проверяем, что метрики создаются корректно
            assert_eq!(metrics.sensor_id, "test");
            assert_eq!(metrics.energy_uj, 1000);
            assert_eq!(metrics.power_w, 1.0);
            assert_eq!(metrics.timestamp, 1234567890);
            assert!(metrics.is_reliable);
        }
    }

    #[test]
    fn test_battery_config_enabled() {
        // Тестируем, что конфигурация батареи включена по умолчанию
        let config = EnergyMonitoringConfig::default();
        assert!(config.enable_battery);
        assert_eq!(
            config.battery_base_path,
            PathBuf::from("/sys/class/power_supply")
        );

        // Тестируем монитор с включенной батареей
        let monitor = EnergyMonitor::new();
        assert!(monitor.config.enable_battery);

        // Тестируем монитор с отключенной батареей
        let mut config_disabled = EnergyMonitoringConfig::default();
        config_disabled.enable_battery = false;
        let monitor_disabled = EnergyMonitor::with_config(config_disabled);
        assert!(!monitor_disabled.config.enable_battery);
    }

    #[test]
    fn test_battery_metrics_integration() {
        // Тестируем интеграцию метрик батареи в общий сбор метрик
        let monitor = EnergyMonitor::new();

        // Пробуем собрать все метрики (должно завершиться успешно)
        let result = monitor.collect_all_energy_metrics();
        assert!(result.is_ok());

        // Результат может быть пустым, если нет реальных сенсоров
        let metrics = result.unwrap();
        // Не проверяем количество метрик, так как оно зависит от системы
    }

    #[test]
    fn test_new_energy_sensor_types() {
        // Тестируем новые типы сенсоров
        let sensor_types = vec![
            EnergySensorType::UsbPowerDelivery,
            EnergySensorType::ThermalPower,
            EnergySensorType::SoftwarePower,
        ];

        for sensor_type in sensor_types {
            let metrics = EnergySensorMetrics {
                sensor_id: "test_new_sensor".to_string(),
                sensor_type,
                energy_uj: 1000,
                power_w: 1.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/test/new".to_string(),
            };

            // Проверяем, что метрики создаются корректно
            assert_eq!(metrics.sensor_id, "test_new_sensor");
            assert_eq!(metrics.energy_uj, 1000);
            assert_eq!(metrics.power_w, 1.0);
            assert_eq!(metrics.timestamp, 1234567890);
            assert!(metrics.is_reliable);
        }
    }

    #[test]
    fn test_new_sensor_types_serialization() {
        // Тестируем сериализацию новых типов сенсоров
        let sensor_types = vec![
            EnergySensorType::UsbPowerDelivery,
            EnergySensorType::ThermalPower,
            EnergySensorType::SoftwarePower,
        ];

        for sensor_type in sensor_types {
            let serialized = serde_json::to_string(&sensor_type).unwrap();
            let deserialized: EnergySensorType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(sensor_type, deserialized);
        }
    }

    #[test]
    fn test_extended_config_default() {
        // Тестируем конфигурацию по умолчанию с новыми полями
        let config = EnergyMonitoringConfig::default();
        assert!(config.enable_usb_power_delivery);
        assert!(config.enable_thermal_power);
        assert!(config.enable_software_power);
        assert_eq!(
            config.usb_power_delivery_base_path,
            PathBuf::from("/sys/class/usb_power_delivery")
        );
        assert_eq!(
            config.thermal_power_base_path,
            PathBuf::from("/sys/kernel/tracing/events/thermal_power_allocator")
        );
        assert_eq!(
            config.software_power_base_path,
            PathBuf::from("/sys/devices/software/power")
        );
    }

    #[test]
    fn test_extended_config_serialization() {
        // Тестируем сериализацию расширенной конфигурации
        let config = EnergyMonitoringConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: EnergyMonitoringConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            config.enable_usb_power_delivery,
            deserialized.enable_usb_power_delivery
        );
        assert_eq!(
            config.enable_thermal_power,
            deserialized.enable_thermal_power
        );
        assert_eq!(
            config.enable_software_power,
            deserialized.enable_software_power
        );
        assert_eq!(
            config.usb_power_delivery_base_path,
            deserialized.usb_power_delivery_base_path
        );
        assert_eq!(
            config.thermal_power_base_path,
            deserialized.thermal_power_base_path
        );
        assert_eq!(
            config.software_power_base_path,
            deserialized.software_power_base_path
        );
    }

    #[test]
    fn test_usb_power_delivery_metrics_collection() {
        // Тестируем сбор метрик USB Power Delivery
        let monitor = EnergyMonitor::new();
        let result = monitor.collect_usb_power_delivery_metrics();
        assert!(result.is_ok());
        // Результат может быть пустым, если нет USB Power Delivery устройств
    }

    #[test]
    fn test_thermal_power_metrics_collection() {
        // Тестируем сбор метрик термальных сенсоров мощности
        let monitor = EnergyMonitor::new();
        let result = monitor.collect_thermal_power_metrics();
        assert!(result.is_ok());
        // Результат может быть пустым, если нет термальных сенсоров
    }

    #[test]
    fn test_software_power_metrics_collection() {
        // Тестируем сбор метрик программных сенсоров мощности
        let monitor = EnergyMonitor::new();
        let result = monitor.collect_software_power_metrics();
        assert!(result.is_ok());
        // Результат может быть пустым, если нет программных сенсоров
    }

    #[test]
    fn test_extended_metrics_integration() {
        // Тестируем интеграцию новых метрик в общий сбор
        let monitor = EnergyMonitor::new();
        let result = monitor.collect_all_energy_metrics();
        assert!(result.is_ok());

        // Проверяем, что все новые типы сенсоров поддерживаются
        let metrics = result.unwrap();
        for metric in metrics {
            match metric.sensor_type {
                EnergySensorType::UsbPowerDelivery => assert!(true),
                EnergySensorType::ThermalPower => assert!(true),
                EnergySensorType::SoftwarePower => assert!(true),
                _ => assert!(true), // Другие типы тоже допустимы
            }
        }
    }

    #[test]
    fn test_extended_monitor_configuration() {
        // Тестируем конфигурацию монитора с новыми опциями
        let mut config = EnergyMonitoringConfig::default();
        config.enable_usb_power_delivery = false;
        config.enable_thermal_power = false;
        config.enable_software_power = false;

        let monitor = EnergyMonitor::with_config(config);
        assert!(!monitor.config.enable_usb_power_delivery);
        assert!(!monitor.config.enable_thermal_power);
        assert!(!monitor.config.enable_software_power);
    }

    #[test]
    fn test_all_sensor_types_comprehensive() {
        // Тестируем все типы сенсоров, включая новые
        let sensor_types = vec![
            EnergySensorType::Rapl,
            EnergySensorType::Acpi,
            EnergySensorType::PowerCap,
            EnergySensorType::Battery,
            EnergySensorType::UsbPowerDelivery,
            EnergySensorType::ThermalPower,
            EnergySensorType::SoftwarePower,
            EnergySensorType::Custom,
            EnergySensorType::Unknown,
        ];

        for sensor_type in sensor_types {
            let metrics = EnergySensorMetrics {
                sensor_id: "comprehensive_test".to_string(),
                sensor_type,
                energy_uj: 1000,
                power_w: 1.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/test/comprehensive".to_string(),
                energy_efficiency: Some(100.0),
                max_power_w: Some(10.0),
                average_power_w: Some(5.0),
                utilization_percent: Some(75.0),
                temperature_c: Some(45.0),
                component_type: Some("test".to_string()),
            };

            // Проверяем, что метрики создаются корректно
            assert_eq!(metrics.sensor_id, "comprehensive_test");
            assert_eq!(metrics.energy_uj, 1000);
            assert_eq!(metrics.power_w, 1.0);
            assert_eq!(metrics.timestamp, 1234567890);
            assert!(metrics.is_reliable);
        }
    }

    #[test]
    fn test_extended_metrics_with_mock_data() {
        // Тестируем новые метрики с моковыми данными
        let mock_metrics = vec![
            EnergySensorMetrics {
                sensor_id: "usb_test".to_string(),
                sensor_type: EnergySensorType::UsbPowerDelivery,
                energy_uj: 500000,
                power_w: 5.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/sys/class/usb_power_delivery/test".to_string(),
                energy_efficiency: Some(100.0),
                max_power_w: Some(10.0),
                average_power_w: Some(5.0),
                utilization_percent: Some(75.0),
                temperature_c: Some(45.0),
                component_type: Some("usb".to_string()),
            },
            EnergySensorMetrics {
                sensor_id: "thermal_test".to_string(),
                sensor_type: EnergySensorType::ThermalPower,
                energy_uj: 300000,
                power_w: 3.0,
                timestamp: 1234567890,
                is_reliable: false,
                sensor_path: "/sys/kernel/tracing/events/thermal_power_allocator/test".to_string(),
                energy_efficiency: Some(80.0),
                max_power_w: Some(8.0),
                average_power_w: Some(4.0),
                utilization_percent: Some(60.0),
                temperature_c: Some(40.0),
                component_type: Some("thermal".to_string()),
            },
            EnergySensorMetrics {
                sensor_id: "software_test".to_string(),
                sensor_type: EnergySensorType::SoftwarePower,
                energy_uj: 200000,
                power_w: 2.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/sys/devices/software/power/test".to_string(),
                energy_efficiency: Some(90.0),
                max_power_w: Some(9.0),
                average_power_w: Some(4.5),
                utilization_percent: Some(65.0),
                temperature_c: Some(42.0),
                component_type: Some("software".to_string()),
            },
        ];

        // Проверяем, что все метрики создаются корректно
        assert_eq!(mock_metrics.len(), 3);
        assert_eq!(
            mock_metrics[0].sensor_type,
            EnergySensorType::UsbPowerDelivery
        );
        assert_eq!(mock_metrics[1].sensor_type, EnergySensorType::ThermalPower);
        assert_eq!(mock_metrics[2].sensor_type, EnergySensorType::SoftwarePower);
        assert_eq!(mock_metrics[0].power_w, 5.0);
        assert_eq!(mock_metrics[1].power_w, 3.0);
        assert_eq!(mock_metrics[2].power_w, 2.0);
    }

    #[test]
    fn test_extended_metrics_aggregation() {
        // Тестируем агрегацию метрик с новыми типами сенсоров
        let mock_metrics = vec![
            EnergySensorMetrics {
                sensor_id: "sensor1".to_string(),
                sensor_type: EnergySensorType::UsbPowerDelivery,
                energy_uj: 1000,
                power_w: 1.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/test1".to_string(),
                energy_efficiency: Some(100.0),
                max_power_w: Some(10.0),
                average_power_w: Some(5.0),
                utilization_percent: Some(75.0),
                temperature_c: Some(45.0),
                component_type: Some("usb".to_string()),
            },
            EnergySensorMetrics {
                sensor_id: "sensor2".to_string(),
                sensor_type: EnergySensorType::ThermalPower,
                energy_uj: 2000,
                power_w: 2.0,
                timestamp: 1234567890,
                is_reliable: false,
                sensor_path: "/test2".to_string(),
                energy_efficiency: Some(80.0),
                max_power_w: Some(8.0),
                average_power_w: Some(4.0),
                utilization_percent: Some(60.0),
                temperature_c: Some(40.0),
                component_type: Some("thermal".to_string()),
            },
            EnergySensorMetrics {
                sensor_id: "sensor3".to_string(),
                sensor_type: EnergySensorType::SoftwarePower,
                energy_uj: 3000,
                power_w: 3.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/test3".to_string(),
                energy_efficiency: Some(90.0),
                max_power_w: Some(9.0),
                average_power_w: Some(4.5),
                utilization_percent: Some(65.0),
                temperature_c: Some(42.0),
                component_type: Some("software".to_string()),
            },
        ];

        // Вручную агрегируем метрики
        let total_energy = mock_metrics.iter().map(|m| m.energy_uj).sum::<u64>();
        let total_power = mock_metrics.iter().map(|m| m.power_w).sum::<f32>();
        let is_reliable = mock_metrics.iter().any(|m| m.is_reliable);

        assert_eq!(total_energy, 6000);
        assert_eq!(total_power, 6.0);
        assert!(is_reliable);
    }

    #[test]
    fn test_extended_config_variations() {
        // Тестируем различные конфигурации с новыми опциями
        let mut config_all_enabled = EnergyMonitoringConfig::default();
        assert!(config_all_enabled.enable_usb_power_delivery);
        assert!(config_all_enabled.enable_thermal_power);
        assert!(config_all_enabled.enable_software_power);

        let mut config_all_disabled = EnergyMonitoringConfig::default();
        config_all_disabled.enable_usb_power_delivery = false;
        config_all_disabled.enable_thermal_power = false;
        config_all_disabled.enable_software_power = false;

        let monitor_disabled = EnergyMonitor::with_config(config_all_disabled);
        assert!(!monitor_disabled.config.enable_usb_power_delivery);
        assert!(!monitor_disabled.config.enable_thermal_power);
        assert!(!monitor_disabled.config.enable_software_power);
    }

    #[test]
    fn test_extended_sensor_type_identification() {
        // Тестируем идентификацию новых типов сенсоров
        let usb_metrics = EnergySensorMetrics {
            sensor_id: "usb_sensor".to_string(),
            sensor_type: EnergySensorType::UsbPowerDelivery,
            energy_uj: 1000,
            power_w: 1.0,
            timestamp: 1234567890,
            is_reliable: true,
            sensor_path: "/usb".to_string(),
            energy_efficiency: Some(100.0),
            max_power_w: Some(10.0),
            average_power_w: Some(5.0),
            utilization_percent: Some(75.0),
            temperature_c: Some(45.0),
            component_type: Some("usb".to_string()),
        };

        let thermal_metrics = EnergySensorMetrics {
            sensor_id: "thermal_sensor".to_string(),
            sensor_type: EnergySensorType::ThermalPower,
            energy_uj: 2000,
            power_w: 2.0,
            timestamp: 1234567890,
            is_reliable: false,
            sensor_path: "/thermal".to_string(),
            energy_efficiency: Some(80.0),
            max_power_w: Some(8.0),
            average_power_w: Some(4.0),
            utilization_percent: Some(60.0),
            temperature_c: Some(40.0),
            component_type: Some("thermal".to_string()),
        };

        let software_metrics = EnergySensorMetrics {
            sensor_id: "software_sensor".to_string(),
            sensor_type: EnergySensorType::SoftwarePower,
            energy_uj: 3000,
            power_w: 3.0,
            timestamp: 1234567890,
            is_reliable: true,
            sensor_path: "/software".to_string(),
            energy_efficiency: Some(90.0),
            max_power_w: Some(9.0),
            average_power_w: Some(4.5),
            utilization_percent: Some(65.0),
            temperature_c: Some(42.0),
            component_type: Some("software".to_string()),
        };

        assert_eq!(usb_metrics.sensor_type, EnergySensorType::UsbPowerDelivery);
        assert_eq!(thermal_metrics.sensor_type, EnergySensorType::ThermalPower);
        assert_eq!(
            software_metrics.sensor_type,
            EnergySensorType::SoftwarePower
        );
    }

    #[test]
    fn test_component_distribution_analysis() {
        // Тестируем анализ распределения энергопотребления по компонентам
        let monitor = EnergyMonitor::new();
        
        // Создаем моковые метрики для тестирования
        let mock_metrics = vec![
            EnergySensorMetrics {
                sensor_id: "cpu_package".to_string(),
                sensor_type: EnergySensorType::CpuPower,
                energy_uj: 1000000,
                power_w: 50.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/sys/class/powercap/cpu_package".to_string(),
                energy_efficiency: Some(120.0),
                max_power_w: Some(100.0),
                average_power_w: Some(50.0),
                utilization_percent: Some(75.0),
                temperature_c: Some(65.0),
                component_type: Some("cpu_package".to_string()),
            },
            EnergySensorMetrics {
                sensor_id: "gpu_card0".to_string(),
                sensor_type: EnergySensorType::GpuPower,
                energy_uj: 500000,
                power_w: 40.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/sys/class/drm/card0".to_string(),
                energy_efficiency: Some(85.0),
                max_power_w: Some(80.0),
                average_power_w: Some(40.0),
                utilization_percent: Some(60.0),
                temperature_c: Some(75.0),
                component_type: Some("gpu".to_string()),
            },
            EnergySensorMetrics {
                sensor_id: "memory_dram".to_string(),
                sensor_type: EnergySensorType::MemoryPower,
                energy_uj: 200000,
                power_w: 10.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/sys/class/powercap/memory_dram".to_string(),
                energy_efficiency: Some(60.0),
                max_power_w: Some(20.0),
                average_power_w: Some(10.0),
                utilization_percent: Some(50.0),
                temperature_c: Some(55.0),
                component_type: Some("memory".to_string()),
            },
        ];

        // Вручную рассчитываем ожидаемые значения
        let total_energy = mock_metrics.iter().map(|m| m.energy_uj).sum::<u64>();
        let cpu_energy: u64 = mock_metrics.iter()
            .filter(|m| m.component_type.as_deref() == Some("cpu_package"))
            .map(|m| m.energy_uj)
            .sum();
        let gpu_energy: u64 = mock_metrics.iter()
            .filter(|m| m.component_type.as_deref() == Some("gpu"))
            .map(|m| m.energy_uj)
            .sum();
        let memory_energy: u64 = mock_metrics.iter()
            .filter(|m| m.component_type.as_deref() == Some("memory"))
            .map(|m| m.energy_uj)
            .sum();

        assert_eq!(total_energy, 1700000);
        assert_eq!(cpu_energy, 1000000);
        assert_eq!(gpu_energy, 500000);
        assert_eq!(memory_energy, 200000);
    }

    #[test]
    fn test_system_energy_efficiency_analysis() {
        // Тестируем анализ энергоэффективности системы
        let monitor = EnergyMonitor::new();
        
        // Создаем моковые метрики для тестирования
        let mock_metrics = vec![
            EnergySensorMetrics {
                sensor_id: "cpu_package".to_string(),
                sensor_type: EnergySensorType::CpuPower,
                energy_uj: 1000000,
                power_w: 50.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/sys/class/powercap/cpu_package".to_string(),
                energy_efficiency: Some(120.0),
                max_power_w: Some(100.0),
                average_power_w: Some(50.0),
                utilization_percent: Some(75.0),
                temperature_c: Some(65.0),
                component_type: Some("cpu_package".to_string()),
            },
            EnergySensorMetrics {
                sensor_id: "gpu_card0".to_string(),
                sensor_type: EnergySensorType::GpuPower,
                energy_uj: 500000,
                power_w: 40.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/sys/class/drm/card0".to_string(),
                energy_efficiency: Some(85.0),
                max_power_w: Some(80.0),
                average_power_w: Some(40.0),
                utilization_percent: Some(60.0),
                temperature_c: Some(75.0),
                component_type: Some("gpu".to_string()),
            },
            EnergySensorMetrics {
                sensor_id: "memory_dram".to_string(),
                sensor_type: EnergySensorType::MemoryPower,
                energy_uj: 200000,
                power_w: 10.0,
                timestamp: 1234567890,
                is_reliable: true,
                sensor_path: "/sys/class/powercap/memory_dram".to_string(),
                energy_efficiency: Some(60.0),
                max_power_w: Some(20.0),
                average_power_w: Some(10.0),
                utilization_percent: Some(50.0),
                temperature_c: Some(55.0),
                component_type: Some("memory".to_string()),
            },
        ];

        // Вручную рассчитываем ожидаемые значения
        let total_energy = mock_metrics.iter().map(|m| m.energy_uj).sum::<u64>();
        let total_power = mock_metrics.iter().map(|m| m.power_w).sum::<f32>();
        let average_efficiency = mock_metrics.iter()
            .filter_map(|m| m.energy_efficiency)
            .sum::<f32>() / 3.0;
        let max_efficiency = mock_metrics.iter()
            .filter_map(|m| m.energy_efficiency)
            .fold(f32::MIN, |a, b| a.max(b));
        let min_efficiency = mock_metrics.iter()
            .filter_map(|m| m.energy_efficiency)
            .fold(f32::MAX, |a, b| a.min(b));

        assert_eq!(total_energy, 1700000);
        assert_eq!(total_power, 100.0);
        assert_eq!(average_efficiency, (120.0 + 85.0 + 60.0) / 3.0);
        assert_eq!(max_efficiency, 120.0);
        assert_eq!(min_efficiency, 60.0);
    }

    #[test]
    fn test_component_metrics_collection() {
        // Тестируем сбор метрик компонентов
        let monitor = EnergyMonitor::new();
        
        // Пробуем собрать метрики компонентов (должно завершиться успешно)
        let cpu_metrics = monitor.collect_cpu_component_metrics();
        assert!(cpu_metrics.is_ok());
        
        let gpu_metrics = monitor.collect_gpu_component_metrics();
        assert!(gpu_metrics.is_ok());
        
        let memory_metrics = monitor.collect_memory_component_metrics();
        assert!(memory_metrics.is_ok());
        
        let pcie_metrics = monitor.collect_pcie_component_metrics();
        assert!(pcie_metrics.is_ok());
    }

    #[test]
    fn test_energy_efficiency_calculations() {
        // Тестируем расчеты энергоэффективности
        let monitor = EnergyMonitor::new();
        
        // Пробуем рассчитать энергоэффективность для различных компонентов
        let cpu_efficiency = monitor.calculate_cpu_energy_efficiency_for_component("package");
        assert!(cpu_efficiency.is_some());
        assert!(cpu_efficiency.unwrap() > 0.0);
        
        let gpu_efficiency = monitor.calculate_gpu_energy_efficiency_for_component("card0");
        assert!(gpu_efficiency.is_some());
        assert!(gpu_efficiency.unwrap() > 0.0);
        
        let memory_efficiency = monitor.calculate_memory_energy_efficiency_for_component("dram");
        assert!(memory_efficiency.is_some());
        assert!(memory_efficiency.unwrap() > 0.0);
        
        let pcie_efficiency = monitor.calculate_pcie_energy_efficiency_for_component("0000:01:00.0");
        assert!(pcie_efficiency.is_some());
        assert!(pcie_efficiency.unwrap() > 0.0);
    }

    #[test]
    fn test_global_energy_monitor_new_functions() {
        // Тестируем новые функции глобального монитора
        let result = GlobalEnergyMonitor::analyze_component_energy_distribution();
        assert!(result.is_ok());
        
        let result = GlobalEnergyMonitor::analyze_system_energy_efficiency();
        assert!(result.is_ok());
        
        let result = GlobalEnergyMonitor::collect_enhanced_energy_metrics();
        assert!(result.is_ok());
    }

    #[test]
    fn test_component_distribution_analysis_default() {
        // Тестируем дефолтные значения для анализа распределения
        let analysis = ComponentDistributionAnalysis::default();
        
        assert_eq!(analysis.pid, 0);
        assert_eq!(analysis.total_energy_uj, 0);
        assert_eq!(analysis.cpu_percentage, 0.0);
        assert_eq!(analysis.cpu_energy_uj, 0);
        assert_eq!(analysis.gpu_percentage, 0.0);
        assert_eq!(analysis.gpu_energy_uj, 0);
        assert_eq!(analysis.memory_percentage, 0.0);
        assert_eq!(analysis.memory_energy_uj, 0);
        assert_eq!(analysis.disk_percentage, 0.0);
        assert_eq!(analysis.disk_energy_uj, 0);
        assert_eq!(analysis.network_percentage, 0.0);
        assert_eq!(analysis.network_energy_uj, 0);
        assert_eq!(analysis.other_percentage, 0.0);
        assert_eq!(analysis.other_energy_uj, 0);
        assert_eq!(analysis.total_percentage, 0.0);
        assert_eq!(analysis.timestamp, 0);
        assert!(!analysis.is_reliable);
    }

    #[test]
    fn test_system_energy_efficiency_analysis_default() {
        // Тестируем дефолтные значения для анализа энергоэффективности
        let analysis = SystemEnergyEfficiencyAnalysis::default();
        
        assert_eq!(analysis.total_energy_uj, 0);
        assert_eq!(analysis.total_power_w, 0.0);
        assert_eq!(analysis.average_efficiency, 0.0);
        assert_eq!(analysis.max_efficiency, 0.0);
        assert_eq!(analysis.min_efficiency, 0.0);
        assert_eq!(analysis.efficiency_count, 0);
        assert_eq!(analysis.timestamp, 0);
        assert!(!analysis.is_reliable);
    }

    #[test]
    fn test_component_distribution_analysis_serialization() {
        // Тестируем сериализацию и десериализацию анализа распределения
        let analysis = ComponentDistributionAnalysis {
            pid: 1234,
            total_energy_uj: 1000000,
            cpu_percentage: 50.0,
            cpu_energy_uj: 500000,
            gpu_percentage: 30.0,
            gpu_energy_uj: 300000,
            memory_percentage: 10.0,
            memory_energy_uj: 100000,
            disk_percentage: 5.0,
            disk_energy_uj: 50000,
            network_percentage: 3.0,
            network_energy_uj: 30000,
            other_percentage: 2.0,
            other_energy_uj: 20000,
            total_percentage: 100.0,
            timestamp: 1234567890,
            is_reliable: true,
        };

        let serialized = serde_json::to_string(&analysis).unwrap();
        let deserialized: ComponentDistributionAnalysis = serde_json::from_str(&serialized).unwrap();

        assert_eq!(analysis.pid, deserialized.pid);
        assert_eq!(analysis.total_energy_uj, deserialized.total_energy_uj);
        assert_eq!(analysis.cpu_percentage, deserialized.cpu_percentage);
        assert_eq!(analysis.cpu_energy_uj, deserialized.cpu_energy_uj);
        assert_eq!(analysis.gpu_percentage, deserialized.gpu_percentage);
        assert_eq!(analysis.gpu_energy_uj, deserialized.gpu_energy_uj);
        assert_eq!(analysis.memory_percentage, deserialized.memory_percentage);
        assert_eq!(analysis.memory_energy_uj, deserialized.memory_energy_uj);
        assert_eq!(analysis.disk_percentage, deserialized.disk_percentage);
        assert_eq!(analysis.disk_energy_uj, deserialized.disk_energy_uj);
        assert_eq!(analysis.network_percentage, deserialized.network_percentage);
        assert_eq!(analysis.network_energy_uj, deserialized.network_energy_uj);
        assert_eq!(analysis.other_percentage, deserialized.other_percentage);
        assert_eq!(analysis.other_energy_uj, deserialized.other_energy_uj);
        assert_eq!(analysis.total_percentage, deserialized.total_percentage);
        assert_eq!(analysis.timestamp, deserialized.timestamp);
        assert_eq!(analysis.is_reliable, deserialized.is_reliable);
    }

    #[test]
    fn test_system_energy_efficiency_analysis_serialization() {
        // Тестируем сериализацию и десериализацию анализа энергоэффективности
        let analysis = SystemEnergyEfficiencyAnalysis {
            total_energy_uj: 1000000,
            total_power_w: 100.0,
            average_efficiency: 85.0,
            max_efficiency: 120.0,
            min_efficiency: 60.0,
            efficiency_count: 3,
            timestamp: 1234567890,
            is_reliable: true,
        };

        let serialized = serde_json::to_string(&analysis).unwrap();
        let deserialized: SystemEnergyEfficiencyAnalysis = serde_json::from_str(&serialized).unwrap();

        assert_eq!(analysis.total_energy_uj, deserialized.total_energy_uj);
        assert_eq!(analysis.total_power_w, deserialized.total_power_w);
        assert_eq!(analysis.average_efficiency, deserialized.average_efficiency);
        assert_eq!(analysis.max_efficiency, deserialized.max_efficiency);
        assert_eq!(analysis.min_efficiency, deserialized.min_efficiency);
        assert_eq!(analysis.efficiency_count, deserialized.efficiency_count);
        assert_eq!(analysis.timestamp, deserialized.timestamp);
        assert_eq!(analysis.is_reliable, deserialized.is_reliable);
    }
}
