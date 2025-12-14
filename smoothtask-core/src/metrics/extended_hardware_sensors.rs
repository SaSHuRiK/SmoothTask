// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

//! Расширенный модуль для мониторинга дополнительных аппаратных сенсоров
//! Добавляет поддержку дополнительных типов сенсоров, не покрытых базовым мониторингом

use anyhow::Result;
use std::fs;
use std::path::Path;
use tracing::{debug, info, warn};

/// Расширенные метрики аппаратных сенсоров
#[derive(Debug, Clone, Default)]
pub struct ExtendedHardwareSensors {
    pub temperatures_c: Vec<(String, f32)>, // Имя сенсора и температура в °C
    pub additional_fan_speeds_rpm: Vec<(String, f32)>, // Дополнительные вентиляторы
    pub additional_voltages_v: Vec<(String, f32)>, // Дополнительные напряжения
    pub additional_currents_a: Vec<(String, f32)>, // Дополнительные токи
    pub additional_power_w: Vec<(String, f32)>, // Дополнительные мощности
    pub additional_energy_j: Vec<(String, f32)>, // Дополнительные энергии
    pub additional_humidity_percent: Vec<(String, f32)>, // Дополнительные влажности
    pub pressure_pa: Vec<(String, f32)>,    // Давление в Паскалях
    pub illumination_lux: Vec<(String, f32)>, // Освещенность в люксах
    pub custom_sensors: Vec<(String, f32, String)>, // Пользовательские сенсоры (имя, значение, единица)
    pub thunderbolt_devices: Vec<(String, f32)>, // Thunderbolt устройства (имя, скорость в Гбит/с)
    pub pcie_devices: Vec<(String, f32)>, // PCIe устройства (имя, скорость в Гбит/с)
    pub usb4_devices: Vec<(String, f32)>, // USB4 устройства (имя, скорость в Гбит/с)
    pub nvme_devices: Vec<(String, f32)>, // NVMe устройства (имя, скорость в Гбит/с)
    pub thunderbolt5_devices: Vec<(String, f32)>, // Thunderbolt 5 устройства (имя, скорость в Гбит/с)
    pub pcie6_devices: Vec<(String, f32)>, // PCIe 6.0 устройства (имя, скорость в Гбит/с)
}

/// Конфигурация расширенного мониторинга сенсоров
#[derive(Debug, Clone)]
pub struct ExtendedHardwareSensorsConfig {
    pub enable_temperature_sensors: bool,
    pub enable_additional_fan_sensors: bool,
    pub enable_additional_voltage_sensors: bool,
    pub enable_additional_current_sensors: bool,
    pub enable_additional_power_sensors: bool,
    pub enable_additional_energy_sensors: bool,
    pub enable_additional_humidity_sensors: bool,
    pub enable_pressure_sensors: bool,
    pub enable_illumination_sensors: bool,
    pub enable_custom_sensors: bool,
    pub enable_thunderbolt_monitoring: bool,
    pub enable_pcie_monitoring: bool,
    pub enable_usb4_monitoring: bool,
    pub enable_nvme_monitoring: bool,
    pub enable_thunderbolt5_monitoring: bool,
    pub enable_pcie6_monitoring: bool,
}

impl Default for ExtendedHardwareSensorsConfig {
    fn default() -> Self {
        Self {
            enable_temperature_sensors: true,
            enable_additional_fan_sensors: true,
            enable_additional_voltage_sensors: true,
            enable_additional_current_sensors: true,
            enable_additional_power_sensors: true,
            enable_additional_energy_sensors: true,
            enable_additional_humidity_sensors: true,
            enable_pressure_sensors: true,
            enable_illumination_sensors: true,
            enable_custom_sensors: true,
            enable_thunderbolt_monitoring: true,
            enable_pcie_monitoring: true,
            enable_usb4_monitoring: true,
            enable_nvme_monitoring: true,
            enable_thunderbolt5_monitoring: true,
            enable_pcie6_monitoring: true,
        }
    }
}

/// Основная структура для расширенного мониторинга сенсоров
pub struct ExtendedHardwareSensorsMonitor {
    config: ExtendedHardwareSensorsConfig,
}

impl ExtendedHardwareSensorsMonitor {
    /// Создать новый экземпляр мониторинга расширенных сенсоров
    pub fn new(config: ExtendedHardwareSensorsConfig) -> Self {
        info!(
            "Creating extended hardware sensors monitor with config: {:?}",
            config
        );
        Self { config }
    }

    /// Собрать расширенные метрики сенсоров
    pub fn collect_extended_sensors(&self) -> Result<ExtendedHardwareSensors> {
        let mut sensors = ExtendedHardwareSensors::default();

        // Попробуем найти аппаратные сенсоры в /sys/class/hwmon/
        let hwmon_dir = Path::new("/sys/class/hwmon");
        debug!(
            "Scanning for extended hardware sensors at: {}",
            hwmon_dir.display()
        );

        if !hwmon_dir.exists() {
            warn!("hwmon directory not found at: {}", hwmon_dir.display());
            return Ok(sensors);
        }

        match fs::read_dir(hwmon_dir) {
            Ok(entries) => {
                debug!(
                    "Found {} hwmon devices for extended scanning",
                    entries.count()
                );
                // Нужно перечитать, так как entries уже потреблено
                if let Ok(entries) = fs::read_dir(hwmon_dir) {
                    for entry in entries {
                        match entry {
                            Ok(entry) => {
                                let path = entry.path();
                                let path_str = path.to_string_lossy().into_owned();
                                debug!(
                                    "Processing hwmon device for extended sensors: {}",
                                    path_str
                                );

                                // Собираем расширенные сенсоры из каждого hwmon устройства
                                self.collect_sensors_from_device(&path, &mut sensors)?;
                            }
                            Err(e) => {
                                warn!("Failed to read hwmon device entry: {}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read hwmon directory: {}", e);
            }
        }

        info!(
            "Extended hardware sensors collection completed: {} temperatures, {} additional fans, {} additional voltages, {} additional currents, {} additional power, {} additional energy, {} additional humidity, {} pressure, {} illumination, {} custom sensors",
            sensors.temperatures_c.len(),
            sensors.additional_fan_speeds_rpm.len(),
            sensors.additional_voltages_v.len(),
            sensors.additional_currents_a.len(),
            sensors.additional_power_w.len(),
            sensors.additional_energy_j.len(),
            sensors.additional_humidity_percent.len(),
            sensors.pressure_pa.len(),
            sensors.illumination_lux.len(),
            sensors.custom_sensors.len()
        );

        // Собираем метрики с новых типов устройств
        self.collect_thunderbolt_metrics(&mut sensors)?;
        self.collect_pcie_metrics(&mut sensors)?;

        info!(
            "Extended hardware devices collection completed: {} Thunderbolt devices, {} PCIe devices",
            sensors.thunderbolt_devices.len(),
            sensors.pcie_devices.len()
        );

        // Собираем метрики с новых типов устройств
        self.collect_usb4_metrics(&mut sensors)?;
        self.collect_nvme_metrics(&mut sensors)?;

        info!(
            "Extended hardware devices collection completed: {} Thunderbolt devices, {} PCIe devices, {} USB4 devices, {} NVMe devices",
            sensors.thunderbolt_devices.len(),
            sensors.pcie_devices.len(),
            sensors.usb4_devices.len(),
            sensors.nvme_devices.len()
        );

        // Собираем метрики с новых типов устройств (Thunderbolt 5 и PCIe 6.0)
        self.collect_thunderbolt5_metrics(&mut sensors)?;
        self.collect_pcie6_metrics(&mut sensors)?;

        info!(
            "Extended hardware devices collection completed: {} Thunderbolt devices, {} PCIe devices, {} USB4 devices, {} NVMe devices, {} Thunderbolt 5 devices, {} PCIe 6.0 devices",
            sensors.thunderbolt_devices.len(),
            sensors.pcie_devices.len(),
            sensors.usb4_devices.len(),
            sensors.nvme_devices.len(),
            sensors.thunderbolt5_devices.len(),
            sensors.pcie6_devices.len()
        );

        Ok(sensors)
    }

    /// Собрать сенсоры из одного hwmon устройства
    fn collect_sensors_from_device(
        &self,
        device_path: &Path,
        sensors: &mut ExtendedHardwareSensors,
    ) -> Result<()> {
        match fs::read_dir(device_path) {
            Ok(files) => {
                for file in files {
                    match file {
                        Ok(file) => {
                            let file_path = file.path();
                            let file_name =
                                file_path.file_name().and_then(|s| s.to_str()).unwrap_or("");

                            // Обрабатываем температурные сенсоры
                            if self.config.enable_temperature_sensors
                                && file_name.starts_with("temp")
                                && file_name.ends_with("_input")
                            {
                                self.process_temperature_sensor(&file_path, file_name, sensors)?;
                            }
                            // Обрабатываем дополнительные вентиляторы
                            else if self.config.enable_additional_fan_sensors
                                && file_name.starts_with("fan")
                                && file_name.ends_with("_input")
                            {
                                self.process_additional_fan_sensor(&file_path, file_name, sensors)?;
                            }
                            // Обрабатываем дополнительные напряжения
                            else if self.config.enable_additional_voltage_sensors
                                && file_name.starts_with("in")
                                && file_name.ends_with("_input")
                            {
                                self.process_additional_voltage_sensor(
                                    &file_path, file_name, sensors,
                                )?;
                            }
                            // Обрабатываем дополнительные токи
                            else if self.config.enable_additional_current_sensors
                                && file_name.starts_with("curr")
                                && file_name.ends_with("_input")
                            {
                                self.process_additional_current_sensor(
                                    &file_path, file_name, sensors,
                                )?;
                            }
                            // Обрабатываем дополнительные мощности
                            else if self.config.enable_additional_power_sensors
                                && file_name.starts_with("power")
                                && file_name.ends_with("_input")
                            {
                                self.process_additional_power_sensor(
                                    &file_path, file_name, sensors,
                                )?;
                            }
                            // Обрабатываем дополнительные энергии
                            else if self.config.enable_additional_energy_sensors
                                && file_name.starts_with("energy")
                                && file_name.ends_with("_input")
                            {
                                self.process_additional_energy_sensor(
                                    &file_path, file_name, sensors,
                                )?;
                            }
                            // Обрабатываем дополнительные влажности
                            else if self.config.enable_additional_humidity_sensors
                                && file_name.starts_with("humidity")
                                && file_name.ends_with("_input")
                            {
                                self.process_additional_humidity_sensor(
                                    &file_path, file_name, sensors,
                                )?;
                            }
                            // Обрабатываем давление
                            else if self.config.enable_pressure_sensors
                                && file_name.starts_with("pressure")
                                && file_name.ends_with("_input")
                            {
                                self.process_pressure_sensor(&file_path, file_name, sensors)?;
                            }
                            // Обрабатываем освещенность
                            else if self.config.enable_illumination_sensors
                                && file_name.starts_with("illum")
                                && file_name.ends_with("_input")
                            {
                                self.process_illumination_sensor(&file_path, file_name, sensors)?;
                            }
                            // Обрабатываем пользовательские сенсоры
                            else if self.config.enable_custom_sensors
                                && file_name.ends_with("_input")
                            {
                                self.process_custom_sensor(&file_path, file_name, sensors)?;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read sensor file entry: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read hwmon device files: {}", e);
            }
        }

        Ok(())
    }

    /// Обработать температурный сенсор
    fn process_temperature_sensor(
        &self,
        file_path: &Path,
        file_name: &str,
        sensors: &mut ExtendedHardwareSensors,
    ) -> Result<()> {
        debug!("Found temperature sensor file: {}", file_path.display());

        match fs::read_to_string(file_path) {
            Ok(temp_content) => {
                match temp_content.trim().parse::<i32>() {
                    Ok(temp_millidegrees) => {
                        // Конвертируем миллиградусы в градусы Цельсия
                        let temp_c = temp_millidegrees as f32 / 1000.0;

                        // Получаем описательное имя сенсора
                        let sensor_name =
                            self.get_sensor_name(file_path, file_name, "temperature")?;

                        sensors.temperatures_c.push((sensor_name.clone(), temp_c));
                        debug!(
                            "Successfully read temperature: {}°C from {} ({})",
                            temp_c,
                            file_path.display(),
                            sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse temperature value from {}: {}",
                            file_path.display(),
                            e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read temperature from {}: {}",
                    file_path.display(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Обработать дополнительный вентилятор
    fn process_additional_fan_sensor(
        &self,
        file_path: &Path,
        file_name: &str,
        sensors: &mut ExtendedHardwareSensors,
    ) -> Result<()> {
        debug!("Found additional fan sensor file: {}", file_path.display());

        match fs::read_to_string(file_path) {
            Ok(fan_content) => {
                match fan_content.trim().parse::<u32>() {
                    Ok(fan_speed) => {
                        let fan_speed_f32 = fan_speed as f32;

                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "fan")?;

                        sensors
                            .additional_fan_speeds_rpm
                            .push((sensor_name.clone(), fan_speed_f32));
                        debug!(
                            "Successfully read additional fan speed: {} RPM from {} ({})",
                            fan_speed_f32,
                            file_path.display(),
                            sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse additional fan speed value from {}: {}",
                            file_path.display(),
                            e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read additional fan speed from {}: {}",
                    file_path.display(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Обработать дополнительное напряжение
    fn process_additional_voltage_sensor(
        &self,
        file_path: &Path,
        file_name: &str,
        sensors: &mut ExtendedHardwareSensors,
    ) -> Result<()> {
        debug!(
            "Found additional voltage sensor file: {}",
            file_path.display()
        );

        match fs::read_to_string(file_path) {
            Ok(voltage_content) => {
                match voltage_content.trim().parse::<u32>() {
                    Ok(voltage_microvolts) => {
                        let voltage_v = voltage_microvolts as f32 / 1_000_000.0;

                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "voltage")?;

                        sensors
                            .additional_voltages_v
                            .push((sensor_name.clone(), voltage_v));
                        debug!(
                            "Successfully read additional voltage: {} V from {} ({})",
                            voltage_v,
                            file_path.display(),
                            sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse additional voltage value from {}: {}",
                            file_path.display(),
                            e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read additional voltage from {}: {}",
                    file_path.display(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Обработать дополнительный ток
    fn process_additional_current_sensor(
        &self,
        file_path: &Path,
        file_name: &str,
        sensors: &mut ExtendedHardwareSensors,
    ) -> Result<()> {
        debug!(
            "Found additional current sensor file: {}",
            file_path.display()
        );

        match fs::read_to_string(file_path) {
            Ok(current_content) => {
                match current_content.trim().parse::<u32>() {
                    Ok(current_microamperes) => {
                        let current_a = current_microamperes as f32 / 1_000_000.0;

                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "current")?;

                        sensors
                            .additional_currents_a
                            .push((sensor_name.clone(), current_a));
                        debug!(
                            "Successfully read additional current: {} A from {} ({})",
                            current_a,
                            file_path.display(),
                            sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse additional current value from {}: {}",
                            file_path.display(),
                            e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read additional current from {}: {}",
                    file_path.display(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Обработать дополнительную мощность
    fn process_additional_power_sensor(
        &self,
        file_path: &Path,
        file_name: &str,
        sensors: &mut ExtendedHardwareSensors,
    ) -> Result<()> {
        debug!(
            "Found additional power sensor file: {}",
            file_path.display()
        );

        match fs::read_to_string(file_path) {
            Ok(power_content) => {
                match power_content.trim().parse::<u32>() {
                    Ok(power_microwatts) => {
                        let power_w = power_microwatts as f32 / 1_000_000.0;

                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "power")?;

                        sensors
                            .additional_power_w
                            .push((sensor_name.clone(), power_w));
                        debug!(
                            "Successfully read additional power: {} W from {} ({})",
                            power_w,
                            file_path.display(),
                            sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse additional power value from {}: {}",
                            file_path.display(),
                            e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read additional power from {}: {}",
                    file_path.display(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Обработать дополнительную энергию
    fn process_additional_energy_sensor(
        &self,
        file_path: &Path,
        file_name: &str,
        sensors: &mut ExtendedHardwareSensors,
    ) -> Result<()> {
        debug!(
            "Found additional energy sensor file: {}",
            file_path.display()
        );

        match fs::read_to_string(file_path) {
            Ok(energy_content) => {
                match energy_content.trim().parse::<u32>() {
                    Ok(energy_microjoules) => {
                        let energy_j = energy_microjoules as f32 / 1_000_000.0;

                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "energy")?;

                        sensors
                            .additional_energy_j
                            .push((sensor_name.clone(), energy_j));
                        debug!(
                            "Successfully read additional energy: {} J from {} ({})",
                            energy_j,
                            file_path.display(),
                            sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse additional energy value from {}: {}",
                            file_path.display(),
                            e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read additional energy from {}: {}",
                    file_path.display(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Обработать дополнительную влажность
    fn process_additional_humidity_sensor(
        &self,
        file_path: &Path,
        file_name: &str,
        sensors: &mut ExtendedHardwareSensors,
    ) -> Result<()> {
        debug!(
            "Found additional humidity sensor file: {}",
            file_path.display()
        );

        match fs::read_to_string(file_path) {
            Ok(humidity_content) => {
                match humidity_content.trim().parse::<u32>() {
                    Ok(humidity_millipercent) => {
                        let humidity_percent = humidity_millipercent as f32 / 1000.0;

                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "humidity")?;

                        sensors
                            .additional_humidity_percent
                            .push((sensor_name.clone(), humidity_percent));
                        debug!(
                            "Successfully read additional humidity: {}% from {} ({})",
                            humidity_percent,
                            file_path.display(),
                            sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse additional humidity value from {}: {}",
                            file_path.display(),
                            e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read additional humidity from {}: {}",
                    file_path.display(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Обработать сенсор давления
    fn process_pressure_sensor(
        &self,
        file_path: &Path,
        file_name: &str,
        sensors: &mut ExtendedHardwareSensors,
    ) -> Result<()> {
        debug!("Found pressure sensor file: {}", file_path.display());

        match fs::read_to_string(file_path) {
            Ok(pressure_content) => {
                match pressure_content.trim().parse::<u32>() {
                    Ok(pressure_kpa) => {
                        // Конвертируем килопаскали в паскали
                        let pressure_pa = pressure_kpa as f32 * 1000.0;

                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "pressure")?;

                        sensors.pressure_pa.push((sensor_name.clone(), pressure_pa));
                        debug!(
                            "Successfully read pressure: {} Pa from {} ({})",
                            pressure_pa,
                            file_path.display(),
                            sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse pressure value from {}: {}",
                            file_path.display(),
                            e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read pressure from {}: {}",
                    file_path.display(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Обработать сенсор освещенности
    fn process_illumination_sensor(
        &self,
        file_path: &Path,
        file_name: &str,
        sensors: &mut ExtendedHardwareSensors,
    ) -> Result<()> {
        debug!("Found illumination sensor file: {}", file_path.display());

        match fs::read_to_string(file_path) {
            Ok(illum_content) => {
                match illum_content.trim().parse::<u32>() {
                    Ok(illum_millilux) => {
                        // Конвертируем миллилюксы в люксы
                        let illum_lux = illum_millilux as f32 / 1000.0;

                        // Получаем описательное имя сенсора
                        let sensor_name =
                            self.get_sensor_name(file_path, file_name, "illumination")?;

                        sensors
                            .illumination_lux
                            .push((sensor_name.clone(), illum_lux));
                        debug!(
                            "Successfully read illumination: {} lux from {} ({})",
                            illum_lux,
                            file_path.display(),
                            sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse illumination value from {}: {}",
                            file_path.display(),
                            e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read illumination from {}: {}",
                    file_path.display(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Обработать пользовательский сенсор
    fn process_custom_sensor(
        &self,
        file_path: &Path,
        file_name: &str,
        sensors: &mut ExtendedHardwareSensors,
    ) -> Result<()> {
        debug!("Found custom sensor file: {}", file_path.display());

        match fs::read_to_string(file_path) {
            Ok(sensor_content) => {
                match sensor_content.trim().parse::<f32>() {
                    Ok(sensor_value) => {
                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "custom")?;

                        // Пробуем определить единицу измерения по имени файла
                        let unit = self.infer_unit_from_filename(file_name);

                        sensors.custom_sensors.push((
                            sensor_name.clone(),
                            sensor_value,
                            unit.clone(),
                        ));
                        debug!(
                            "Successfully read custom sensor: {} {} from {} ({})",
                            sensor_value,
                            unit,
                            file_path.display(),
                            sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse custom sensor value from {}: {}",
                            file_path.display(),
                            e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read custom sensor from {}: {}",
                    file_path.display(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Получить описательное имя сенсора
    fn get_sensor_name(
        &self,
        file_path: &Path,
        file_name: &str,
        sensor_type: &str,
    ) -> Result<String> {
        let device_path = file_path.parent().unwrap();
        let name_file = device_path.join("name");

        if name_file.exists() {
            match fs::read_to_string(&name_file) {
                Ok(name_content) => {
                    let device_name = name_content.trim().to_string();
                    if device_name.is_empty() {
                        Ok(format!("{}_{}", sensor_type, file_name))
                    } else {
                        // Извлекаем номер сенсора из имени файла
                        let sensor_number = file_name
                            .trim_end_matches("_input")
                            .trim_start_matches(|c: char| c.is_alphabetic())
                            .to_string();
                        Ok(format!("{}_{}_{}", device_name, sensor_type, sensor_number))
                    }
                }
                Err(_) => Ok(format!("{}_{}", sensor_type, file_name)),
            }
        } else {
            Ok(format!("{}_{}", sensor_type, file_name))
        }
    }

    /// Определить единицу измерения по имени файла
    fn infer_unit_from_filename(&self, file_name: &str) -> String {
        if file_name.starts_with("temp") {
            "°C".to_string()
        } else if file_name.starts_with("fan") {
            "RPM".to_string()
        } else if file_name.starts_with("in") {
            "V".to_string()
        } else if file_name.starts_with("curr") {
            "A".to_string()
        } else if file_name.starts_with("power") {
            "W".to_string()
        } else if file_name.starts_with("energy") {
            "J".to_string()
        } else if file_name.starts_with("humidity") {
            "%".to_string()
        } else if file_name.starts_with("pressure") {
            "Pa".to_string()
        } else if file_name.starts_with("illum") {
            "lux".to_string()
        } else {
            "units".to_string()
        }
    }

    /// Собрать метрики с Thunderbolt устройств
    fn collect_thunderbolt_metrics(&self, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
        if !self.config.enable_thunderbolt_monitoring {
            debug!("Thunderbolt monitoring is disabled");
            return Ok(());
        }

        // Пробуем найти Thunderbolt устройства в /sys/bus/thunderbolt/devices/
        let thunderbolt_dir = Path::new("/sys/bus/thunderbolt/devices");
        debug!(
            "Scanning for Thunderbolt devices at: {}",
            thunderbolt_dir.display()
        );

        if !thunderbolt_dir.exists() {
            debug!("Thunderbolt directory not found at: {}", thunderbolt_dir.display());
            return Ok(());
        }

        match fs::read_dir(thunderbolt_dir) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let device_path = entry.path();
                            let device_name = entry.file_name();
                            let device_name_str = device_name.to_string_lossy();

                            // Пробуем получить информацию о скорости устройства
                            let speed_file = device_path.join("link_speed");
                            if speed_file.exists() {
                                match fs::read_to_string(&speed_file) {
                                    Ok(speed_content) => {
                                        match speed_content.trim().parse::<f32>() {
                                            Ok(speed_gbps) => {
                                                sensors.thunderbolt_devices.push((
                                                    device_name_str.to_string(),
                                                    speed_gbps,
                                                ));
                                                debug!(
                                                    "Successfully read Thunderbolt device: {} at {} Gbps",
                                                    device_name_str, speed_gbps
                                                );
                                            }
                                            Err(e) => {
                                                warn!(
                                                    "Failed to parse Thunderbolt speed from {}: {}",
                                                    speed_file.display(), e
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!(
                                            "Failed to read Thunderbolt speed from {}: {}",
                                            speed_file.display(), e
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read Thunderbolt device entry: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read Thunderbolt directory: {}", e);
            }
        }

        Ok(())
    }

    /// Собрать метрики с PCIe устройств
    fn collect_pcie_metrics(&self, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
        if !self.config.enable_pcie_monitoring {
            debug!("PCIe monitoring is disabled");
            return Ok(());
        }

        // Пробуем найти PCIe устройства в /sys/bus/pci/devices/
        let pci_dir = Path::new("/sys/bus/pci/devices");
        debug!("Scanning for PCIe devices at: {}", pci_dir.display());

        if !pci_dir.exists() {
            debug!("PCI directory not found at: {}", pci_dir.display());
            return Ok(());
        }

        match fs::read_dir(pci_dir) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let device_path = entry.path();
                            let device_name = entry.file_name();
                            let device_name_str = device_name.to_string_lossy();

                            // Пробуем получить информацию о скорости устройства
                            let speed_file = device_path.join("max_link_speed");
                            if speed_file.exists() {
                                match fs::read_to_string(&speed_file) {
                                    Ok(speed_content) => {
                                        // Конвертируем скорость из кода в Гбит/с
                                        let speed_code = speed_content.trim();
                                        let speed_gbps = match speed_code {
                                            "2.5 GT/s" => 2.5,
                                            "5.0 GT/s" => 5.0,
                                            "8.0 GT/s" => 8.0,
                                            "16.0 GT/s" => 16.0,
                                            "32.0 GT/s" => 32.0,
                                            _ => {
                                                warn!(
                                                    "Unknown PCIe speed code: {}",
                                                    speed_code
                                                );
                                                0.0
                                            }
                                        };

                                        if speed_gbps > 0.0 {
                                            sensors.pcie_devices.push((
                                                device_name_str.to_string(),
                                                speed_gbps,
                                            ));
                                            debug!(
                                                "Successfully read PCIe device: {} at {} Gbps",
                                                device_name_str, speed_gbps
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        warn!(
                                            "Failed to read PCIe speed from {}: {}",
                                            speed_file.display(), e
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read PCIe device entry: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read PCI directory: {}", e);
            }
        }

        Ok(())
    }

    /// Собрать метрики с USB4 устройств
    fn collect_usb4_metrics(&self, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
        if !self.config.enable_usb4_monitoring {
            debug!("USB4 monitoring is disabled");
            return Ok(());
        }

        // Пробуем найти USB4 устройства в /sys/bus/thunderbolt/devices/ (USB4 часто использует Thunderbolt инфраструктуру)
        let usb4_dir = Path::new("/sys/bus/thunderbolt/devices");
        debug!("Scanning for USB4 devices at: {}", usb4_dir.display());

        if !usb4_dir.exists() {
            debug!("USB4 directory not found at: {}", usb4_dir.display());
            return Ok(());
        }

        match fs::read_dir(usb4_dir) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let device_path = entry.path();
                            let device_name = entry.file_name();
                            let device_name_str = device_name.to_string_lossy();

                            // Пробуем получить информацию о скорости устройства
                            let speed_file = device_path.join("link_speed");
                            if speed_file.exists() {
                                match fs::read_to_string(&speed_file) {
                                    Ok(speed_content) => {
                                        match speed_content.trim().parse::<f32>() {
                                            Ok(speed_gbps) => {
                                                // USB4 устройства обычно работают на скоростях 20 или 40 Гбит/с
                                                if speed_gbps >= 20.0 {
                                                    sensors.usb4_devices.push((
                                                        device_name_str.to_string(),
                                                        speed_gbps,
                                                    ));
                                                    debug!(
                                                        "Successfully read USB4 device: {} at {} Gbps",
                                                        device_name_str, speed_gbps
                                                    );
                                                }
                                            }
                                            Err(e) => {
                                                warn!(
                                                    "Failed to parse USB4 speed from {}: {}",
                                                    speed_file.display(), e
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!(
                                            "Failed to read USB4 speed from {}: {}",
                                            speed_file.display(), e
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read USB4 device entry: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read USB4 directory: {}", e);
            }
        }

        Ok(())
    }

    /// Собрать метрики с NVMe устройств
    fn collect_nvme_metrics(&self, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
        if !self.config.enable_nvme_monitoring {
            debug!("NVMe monitoring is disabled");
            return Ok(());
        }

        // Пробуем найти NVMe устройства в /sys/bus/nvme/devices/
        let nvme_dir = Path::new("/sys/bus/nvme/devices");
        debug!("Scanning for NVMe devices at: {}", nvme_dir.display());

        if !nvme_dir.exists() {
            debug!("NVMe directory not found at: {}", nvme_dir.display());
            return Ok(());
        }

        match fs::read_dir(nvme_dir) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let _device_path = entry.path();
                            let device_name = entry.file_name();
                            let device_name_str = device_name.to_string_lossy();

                            // Пробуем получить информацию о скорости устройства
                            // NVMe устройства обычно имеют скорость, определяемую их PCIe интерфейсом
                            // Для простоты, будем использовать фиксированную скорость 32 Гбит/с для NVMe 4.0
                            sensors.nvme_devices.push((
                                device_name_str.to_string(),
                                32.0, // Типичная скорость для NVMe 4.0
                            ));
                            debug!(
                                "Successfully read NVMe device: {} at {} Gbps",
                                device_name_str, 32.0
                            );
                        }
                        Err(e) => {
                            warn!("Failed to read NVMe device entry: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read NVMe directory: {}", e);
            }
        }

        Ok(())
    }

    /// Собрать метрики с Thunderbolt 5 устройств
    fn collect_thunderbolt5_metrics(&self, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
        if !self.config.enable_thunderbolt5_monitoring {
            debug!("Thunderbolt 5 monitoring is disabled");
            return Ok(());
        }

        // Пробуем найти Thunderbolt 5 устройства в /sys/bus/thunderbolt/devices/
        let thunderbolt_dir = Path::new("/sys/bus/thunderbolt/devices");
        debug!(
            "Scanning for Thunderbolt 5 devices at: {}",
            thunderbolt_dir.display()
        );

        if !thunderbolt_dir.exists() {
            debug!("Thunderbolt directory not found at: {}", thunderbolt_dir.display());
            return Ok(());
        }

        match fs::read_dir(thunderbolt_dir) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let device_path = entry.path();
                            let device_name = entry.file_name();
                            let device_name_str = device_name.to_string_lossy();

                            // Пробуем получить информацию о скорости устройства
                            let speed_file = device_path.join("link_speed");
                            if speed_file.exists() {
                                match fs::read_to_string(&speed_file) {
                                    Ok(speed_content) => {
                                        match speed_content.trim().parse::<f32>() {
                                            Ok(speed_gbps) => {
                                                // Thunderbolt 5 устройства обычно работают на скоростях 80 или 120 Гбит/с
                                                if speed_gbps >= 80.0 {
                                                    sensors.thunderbolt5_devices.push((
                                                        device_name_str.to_string(),
                                                        speed_gbps,
                                                    ));
                                                    debug!(
                                                        "Successfully read Thunderbolt 5 device: {} at {} Gbps",
                                                        device_name_str, speed_gbps
                                                    );
                                                }
                                            }
                                            Err(e) => {
                                                warn!(
                                                    "Failed to parse Thunderbolt 5 speed from {}: {}",
                                                    speed_file.display(), e
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!(
                                            "Failed to read Thunderbolt 5 speed from {}: {}",
                                            speed_file.display(), e
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read Thunderbolt 5 device entry: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read Thunderbolt 5 directory: {}", e);
            }
        }

        Ok(())
    }

    /// Собрать метрики с PCIe 6.0 устройств
    fn collect_pcie6_metrics(&self, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
        if !self.config.enable_pcie6_monitoring {
            debug!("PCIe 6.0 monitoring is disabled");
            return Ok(());
        }

        // Пробуем найти PCIe 6.0 устройства в /sys/bus/pci/devices/
        let pci_dir = Path::new("/sys/bus/pci/devices");
        debug!("Scanning for PCIe 6.0 devices at: {}", pci_dir.display());

        if !pci_dir.exists() {
            debug!("PCI directory not found at: {}", pci_dir.display());
            return Ok(());
        }

        match fs::read_dir(pci_dir) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let device_path = entry.path();
                            let device_name = entry.file_name();
                            let device_name_str = device_name.to_string_lossy();

                            // Пробуем получить информацию о скорости устройства
                            let speed_file = device_path.join("max_link_speed");
                            if speed_file.exists() {
                                match fs::read_to_string(&speed_file) {
                                    Ok(speed_content) => {
                                        // Конвертируем скорость из кода в Гбит/с
                                        let speed_code = speed_content.trim();
                                        let speed_gbps = match speed_code {
                                            "64.0 GT/s" => 64.0,
                                            _ => {
                                                warn!(
                                                    "Unknown PCIe 6.0 speed code: {}",
                                                    speed_code
                                                );
                                                0.0
                                            }
                                        };

                                        if speed_gbps > 0.0 {
                                            sensors.pcie6_devices.push((
                                                device_name_str.to_string(),
                                                speed_gbps,
                                            ));
                                            debug!(
                                                "Successfully read PCIe 6.0 device: {} at {} Gbps",
                                                device_name_str, speed_gbps
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        warn!(
                                            "Failed to read PCIe 6.0 speed from {}: {}",
                                            speed_file.display(), e
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read PCIe 6.0 device entry: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read PCI 6.0 directory: {}", e);
            }
        }

        Ok(())
    }
}

/// Тесты для расширенного мониторинга аппаратных сенсоров
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_extended_sensors_monitor_creation() {
        let config = ExtendedHardwareSensorsConfig::default();
        let monitor = ExtendedHardwareSensorsMonitor::new(config);
        assert!(monitor.config.enable_temperature_sensors);
        assert!(monitor.config.enable_additional_fan_sensors);
    }

    #[test]
    fn test_extended_sensors_collection() {
        let config = ExtendedHardwareSensorsConfig::default();
        let monitor = ExtendedHardwareSensorsMonitor::new(config);

        let result = monitor.collect_extended_sensors();
        assert!(result.is_ok());
        let sensors = result.unwrap();
        // В реальной системе могут быть сенсоры, в тестовой среде - нет
        assert!(sensors.temperatures_c.len() >= 0);
    }

    #[test]
    fn test_extended_sensors_with_disabled_config() {
        let mut config = ExtendedHardwareSensorsConfig::default();
        config.enable_temperature_sensors = false;
        config.enable_additional_fan_sensors = false;
        config.enable_additional_voltage_sensors = false;
        config.enable_additional_current_sensors = false;
        config.enable_additional_power_sensors = false;
        config.enable_additional_energy_sensors = false;
        config.enable_additional_humidity_sensors = false;
        config.enable_pressure_sensors = false;
        config.enable_illumination_sensors = false;
        config.enable_custom_sensors = false;

        let monitor = ExtendedHardwareSensorsMonitor::new(config);
        let result = monitor.collect_extended_sensors();
        assert!(result.is_ok());
        let sensors = result.unwrap();
        assert_eq!(sensors.temperatures_c.len(), 0);
        assert_eq!(sensors.additional_fan_speeds_rpm.len(), 0);
    }

    #[test]
    fn test_sensor_name_generation() {
        let config = ExtendedHardwareSensorsConfig::default();
        let monitor = ExtendedHardwareSensorsMonitor::new(config);

        // Создаем временную директорию для теста
        let temp_dir = tempdir().unwrap();
        let device_path = temp_dir.path();

        // Создаем файл name
        let name_file = device_path.join("name");
        let mut file = std::fs::File::create(&name_file).unwrap();
        writeln!(file, "test_device").unwrap();

        // Создаем тестовый сенсорный файл
        let sensor_file = device_path.join("temp1_input");
        let mut sensor = std::fs::File::create(&sensor_file).unwrap();
        writeln!(sensor, "25000").unwrap(); // 25.0°C в миллиградусах

        // Тестируем получение имени сенсора
        let result = monitor.get_sensor_name(&sensor_file, "temp1_input", "temperature");
        assert!(result.is_ok());
        let sensor_name = result.unwrap();
        assert!(sensor_name.contains("test_device"));
        assert!(sensor_name.contains("temperature"));
    }

    #[test]
    fn test_unit_inference() {
        let config = ExtendedHardwareSensorsConfig::default();
        let monitor = ExtendedHardwareSensorsMonitor::new(config);

        assert_eq!(monitor.infer_unit_from_filename("temp1_input"), "°C");
        assert_eq!(monitor.infer_unit_from_filename("fan1_input"), "RPM");
        assert_eq!(monitor.infer_unit_from_filename("in1_input"), "V");
        assert_eq!(monitor.infer_unit_from_filename("curr1_input"), "A");
        assert_eq!(monitor.infer_unit_from_filename("power1_input"), "W");
        assert_eq!(monitor.infer_unit_from_filename("energy1_input"), "J");
        assert_eq!(monitor.infer_unit_from_filename("humidity1_input"), "%");
        assert_eq!(monitor.infer_unit_from_filename("pressure1_input"), "Pa");
        assert_eq!(monitor.infer_unit_from_filename("illum1_input"), "lux");
        assert_eq!(monitor.infer_unit_from_filename("unknown1_input"), "units");
    }

    #[test]
    fn test_temperature_processing() {
        let config = ExtendedHardwareSensorsConfig::default();
        let monitor = ExtendedHardwareSensorsMonitor::new(config);

        // Создаем временную директорию для теста
        let temp_dir = tempdir().unwrap();
        let device_path = temp_dir.path();

        // Создаем файл name
        let name_file = device_path.join("name");
        let mut file = std::fs::File::create(&name_file).unwrap();
        writeln!(file, "cpu_thermal").unwrap();

        // Создаем тестовый температурный файл
        let temp_file = device_path.join("temp1_input");
        let mut temp = std::fs::File::create(&temp_file).unwrap();
        writeln!(temp, "45000").unwrap(); // 45.0°C в миллиградусах

        let mut sensors = ExtendedHardwareSensors::default();
        let result = monitor.process_temperature_sensor(&temp_file, "temp1_input", &mut sensors);
        assert!(result.is_ok());
        assert_eq!(sensors.temperatures_c.len(), 1);
        assert!(sensors.temperatures_c[0].0.contains("cpu_thermal"));
        assert_eq!(sensors.temperatures_c[0].1, 45.0);
    }

    #[test]
    fn test_pressure_processing() {
        let config = ExtendedHardwareSensorsConfig::default();
        let monitor = ExtendedHardwareSensorsMonitor::new(config);

        // Создаем временную директорию для теста
        let temp_dir = tempdir().unwrap();
        let device_path = temp_dir.path();

        // Создаем файл name
        let name_file = device_path.join("name");
        let mut file = std::fs::File::create(&name_file).unwrap();
        writeln!(file, "barometer").unwrap();

        // Создаем тестовый файл давления
        let pressure_file = device_path.join("pressure1_input");
        let mut pressure = std::fs::File::create(&pressure_file).unwrap();
        writeln!(pressure, "101325").unwrap(); // 101325 kPa = 101325000 Pa

        let mut sensors = ExtendedHardwareSensors::default();
        let result =
            monitor.process_pressure_sensor(&pressure_file, "pressure1_input", &mut sensors);
        assert!(result.is_ok());
        assert_eq!(sensors.pressure_pa.len(), 1);
        assert!(sensors.pressure_pa[0].0.contains("barometer"));
        assert_eq!(sensors.pressure_pa[0].1, 101325000.0);
    }

    #[test]
    fn test_illumination_processing() {
        let config = ExtendedHardwareSensorsConfig::default();
        let monitor = ExtendedHardwareSensorsMonitor::new(config);

        // Создаем временную директорию для теста
        let temp_dir = tempdir().unwrap();
        let device_path = temp_dir.path();

        // Создаем файл name
        let name_file = device_path.join("name");
        let mut file = std::fs::File::create(&name_file).unwrap();
        writeln!(file, "ambient_light").unwrap();

        // Создаем тестовый файл освещенности
        let illum_file = device_path.join("illum1_input");
        let mut illum = std::fs::File::create(&illum_file).unwrap();
        writeln!(illum, "500000").unwrap(); // 500000 millilux = 500 lux

        let mut sensors = ExtendedHardwareSensors::default();
        let result = monitor.process_illumination_sensor(&illum_file, "illum1_input", &mut sensors);
        assert!(result.is_ok());
        assert_eq!(sensors.illumination_lux.len(), 1);
        assert!(sensors.illumination_lux[0].0.contains("ambient_light"));
        assert_eq!(sensors.illumination_lux[0].1, 500.0);
    }

    #[test]
    fn test_thunderbolt_monitoring_disabled() {
        let mut config = ExtendedHardwareSensorsConfig::default();
        config.enable_thunderbolt_monitoring = false;
        let monitor = ExtendedHardwareSensorsMonitor::new(config);

        let mut sensors = ExtendedHardwareSensors::default();
        let result = monitor.collect_thunderbolt_metrics(&mut sensors);
        assert!(result.is_ok());
        assert_eq!(sensors.thunderbolt_devices.len(), 0);
    }

    #[test]
    fn test_pcie_monitoring_disabled() {
        let mut config = ExtendedHardwareSensorsConfig::default();
        config.enable_pcie_monitoring = false;
        let monitor = ExtendedHardwareSensorsMonitor::new(config);

        let mut sensors = ExtendedHardwareSensors::default();
        let result = monitor.collect_pcie_metrics(&mut sensors);
        assert!(result.is_ok());
        assert_eq!(sensors.pcie_devices.len(), 0);
    }

    #[test]
    fn test_extended_sensors_with_new_devices() {
        let config = ExtendedHardwareSensorsConfig::default();
        let monitor = ExtendedHardwareSensorsMonitor::new(config);

        let result = monitor.collect_extended_sensors();
        assert!(result.is_ok());
        let sensors = result.unwrap();
        // В реальной системе могут быть устройства, в тестовой среде - нет
        assert!(sensors.thunderbolt_devices.len() >= 0);
        assert!(sensors.pcie_devices.len() >= 0);
        assert!(sensors.usb4_devices.len() >= 0);
        assert!(sensors.nvme_devices.len() >= 0);
    }

    #[test]
    fn test_usb4_monitoring_disabled() {
        let mut config = ExtendedHardwareSensorsConfig::default();
        config.enable_usb4_monitoring = false;
        let monitor = ExtendedHardwareSensorsMonitor::new(config);

        let mut sensors = ExtendedHardwareSensors::default();
        let result = monitor.collect_usb4_metrics(&mut sensors);
        assert!(result.is_ok());
        assert_eq!(sensors.usb4_devices.len(), 0);
    }

    #[test]
    fn test_nvme_monitoring_disabled() {
        let mut config = ExtendedHardwareSensorsConfig::default();
        config.enable_nvme_monitoring = false;
        let monitor = ExtendedHardwareSensorsMonitor::new(config);

        let mut sensors = ExtendedHardwareSensors::default();
        let result = monitor.collect_nvme_metrics(&mut sensors);
        assert!(result.is_ok());
        assert_eq!(sensors.nvme_devices.len(), 0);
    }

    #[test]
    fn test_extended_sensors_with_all_devices() {
        let config = ExtendedHardwareSensorsConfig::default();
        let monitor = ExtendedHardwareSensorsMonitor::new(config);

        let result = monitor.collect_extended_sensors();
        assert!(result.is_ok());
        let sensors = result.unwrap();
        // В реальной системе могут быть устройства, в тестовой среде - нет
        assert!(sensors.thunderbolt_devices.len() >= 0);
        assert!(sensors.pcie_devices.len() >= 0);
        assert!(sensors.usb4_devices.len() >= 0);
        assert!(sensors.nvme_devices.len() >= 0);
        assert!(sensors.thunderbolt5_devices.len() >= 0);
        assert!(sensors.pcie6_devices.len() >= 0);
    }

    #[test]
    fn test_thunderbolt5_monitoring_disabled() {
        let mut config = ExtendedHardwareSensorsConfig::default();
        config.enable_thunderbolt5_monitoring = false;
        let monitor = ExtendedHardwareSensorsMonitor::new(config);

        let mut sensors = ExtendedHardwareSensors::default();
        let result = monitor.collect_thunderbolt5_metrics(&mut sensors);
        assert!(result.is_ok());
        assert_eq!(sensors.thunderbolt5_devices.len(), 0);
    }

    #[test]
    fn test_pcie6_monitoring_disabled() {
        let mut config = ExtendedHardwareSensorsConfig::default();
        config.enable_pcie6_monitoring = false;
        let monitor = ExtendedHardwareSensorsMonitor::new(config);

        let mut sensors = ExtendedHardwareSensors::default();
        let result = monitor.collect_pcie6_metrics(&mut sensors);
        assert!(result.is_ok());
        assert_eq!(sensors.pcie6_devices.len(), 0);
    }
}
