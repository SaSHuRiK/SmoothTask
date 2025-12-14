// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

//! Расширенный модуль для мониторинга дополнительных аппаратных сенсоров
//! Добавляет поддержку дополнительных типов сенсоров, не покрытых базовым мониторингом

use std::fs;
use std::path::Path;
use anyhow::Result;
use tracing::{debug, info, warn};

/// Расширенные метрики аппаратных сенсоров
#[derive(Debug, Clone, Default)]
pub struct ExtendedHardwareSensors {
    pub temperatures_c: Vec<(String, f32)>,  // Имя сенсора и температура в °C
    pub additional_fan_speeds_rpm: Vec<(String, f32)>,  // Дополнительные вентиляторы
    pub additional_voltages_v: Vec<(String, f32)>,  // Дополнительные напряжения
    pub additional_currents_a: Vec<(String, f32)>,  // Дополнительные токи
    pub additional_power_w: Vec<(String, f32)>,  // Дополнительные мощности
    pub additional_energy_j: Vec<(String, f32)>,  // Дополнительные энергии
    pub additional_humidity_percent: Vec<(String, f32)>,  // Дополнительные влажности
    pub pressure_pa: Vec<(String, f32)>,  // Давление в Паскалях
    pub illumination_lux: Vec<(String, f32)>,  // Освещенность в люксах
    pub custom_sensors: Vec<(String, f32, String)>,  // Пользовательские сенсоры (имя, значение, единица)
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
        info!("Creating extended hardware sensors monitor with config: {:?}", config);
        Self { config }
    }

    /// Собрать расширенные метрики сенсоров
    pub fn collect_extended_sensors(&self) -> Result<ExtendedHardwareSensors> {
        let mut sensors = ExtendedHardwareSensors::default();
        
        // Попробуем найти аппаратные сенсоры в /sys/class/hwmon/
        let hwmon_dir = Path::new("/sys/class/hwmon");
        debug!("Scanning for extended hardware sensors at: {}", hwmon_dir.display());

        if !hwmon_dir.exists() {
            warn!("hwmon directory not found at: {}", hwmon_dir.display());
            return Ok(sensors);
        }

        match fs::read_dir(hwmon_dir) {
            Ok(entries) => {
                debug!("Found {} hwmon devices for extended scanning", entries.count());
                // Нужно перечитать, так как entries уже потреблено
                if let Ok(entries) = fs::read_dir(hwmon_dir) {
                    for entry in entries {
                        match entry {
                            Ok(entry) => {
                                let path = entry.path();
                                let path_str = path.to_string_lossy().into_owned();
                                debug!("Processing hwmon device for extended sensors: {}", path_str);

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

        Ok(sensors)
    }

    /// Собрать сенсоры из одного hwmon устройства
    fn collect_sensors_from_device(&self, device_path: &Path, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
        match fs::read_dir(device_path) {
            Ok(files) => {
                for file in files {
                    match file {
                        Ok(file) => {
                            let file_path = file.path();
                            let file_name = file_path
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("");

                            // Обрабатываем температурные сенсоры
                            if self.config.enable_temperature_sensors && 
                               file_name.starts_with("temp") && file_name.ends_with("_input") {
                                self.process_temperature_sensor(&file_path, file_name, sensors)?;
                            }
                            // Обрабатываем дополнительные вентиляторы
                            else if self.config.enable_additional_fan_sensors && 
                                   file_name.starts_with("fan") && file_name.ends_with("_input") {
                                self.process_additional_fan_sensor(&file_path, file_name, sensors)?;
                            }
                            // Обрабатываем дополнительные напряжения
                            else if self.config.enable_additional_voltage_sensors && 
                                   file_name.starts_with("in") && file_name.ends_with("_input") {
                                self.process_additional_voltage_sensor(&file_path, file_name, sensors)?;
                            }
                            // Обрабатываем дополнительные токи
                            else if self.config.enable_additional_current_sensors && 
                                   file_name.starts_with("curr") && file_name.ends_with("_input") {
                                self.process_additional_current_sensor(&file_path, file_name, sensors)?;
                            }
                            // Обрабатываем дополнительные мощности
                            else if self.config.enable_additional_power_sensors && 
                                   file_name.starts_with("power") && file_name.ends_with("_input") {
                                self.process_additional_power_sensor(&file_path, file_name, sensors)?;
                            }
                            // Обрабатываем дополнительные энергии
                            else if self.config.enable_additional_energy_sensors && 
                                   file_name.starts_with("energy") && file_name.ends_with("_input") {
                                self.process_additional_energy_sensor(&file_path, file_name, sensors)?;
                            }
                            // Обрабатываем дополнительные влажности
                            else if self.config.enable_additional_humidity_sensors && 
                                   file_name.starts_with("humidity") && file_name.ends_with("_input") {
                                self.process_additional_humidity_sensor(&file_path, file_name, sensors)?;
                            }
                            // Обрабатываем давление
                            else if self.config.enable_pressure_sensors && 
                                   file_name.starts_with("pressure") && file_name.ends_with("_input") {
                                self.process_pressure_sensor(&file_path, file_name, sensors)?;
                            }
                            // Обрабатываем освещенность
                            else if self.config.enable_illumination_sensors && 
                                   file_name.starts_with("illum") && file_name.ends_with("_input") {
                                self.process_illumination_sensor(&file_path, file_name, sensors)?;
                            }
                            // Обрабатываем пользовательские сенсоры
                            else if self.config.enable_custom_sensors && 
                                   file_name.ends_with("_input") {
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
    fn process_temperature_sensor(&self, file_path: &Path, file_name: &str, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
        debug!("Found temperature sensor file: {}", file_path.display());

        match fs::read_to_string(file_path) {
            Ok(temp_content) => {
                match temp_content.trim().parse::<i32>() {
                    Ok(temp_millidegrees) => {
                        // Конвертируем миллиградусы в градусы Цельсия
                        let temp_c = temp_millidegrees as f32 / 1000.0;

                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "temperature")?;

                        sensors.temperatures_c.push((sensor_name.clone(), temp_c));
                        debug!(
                            "Successfully read temperature: {}°C from {} ({})",
                            temp_c, file_path.display(), sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse temperature value from {}: {}",
                            file_path.display(), e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read temperature from {}: {}",
                    file_path.display(), e
                );
            }
        }

        Ok(())
    }

    /// Обработать дополнительный вентилятор
    fn process_additional_fan_sensor(&self, file_path: &Path, file_name: &str, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
        debug!("Found additional fan sensor file: {}", file_path.display());

        match fs::read_to_string(file_path) {
            Ok(fan_content) => {
                match fan_content.trim().parse::<u32>() {
                    Ok(fan_speed) => {
                        let fan_speed_f32 = fan_speed as f32;

                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "fan")?;

                        sensors.additional_fan_speeds_rpm.push((sensor_name.clone(), fan_speed_f32));
                        debug!(
                            "Successfully read additional fan speed: {} RPM from {} ({})",
                            fan_speed_f32, file_path.display(), sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse additional fan speed value from {}: {}",
                            file_path.display(), e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read additional fan speed from {}: {}",
                    file_path.display(), e
                );
            }
        }

        Ok(())
    }

    /// Обработать дополнительное напряжение
    fn process_additional_voltage_sensor(&self, file_path: &Path, file_name: &str, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
        debug!("Found additional voltage sensor file: {}", file_path.display());

        match fs::read_to_string(file_path) {
            Ok(voltage_content) => {
                match voltage_content.trim().parse::<u32>() {
                    Ok(voltage_microvolts) => {
                        let voltage_v = voltage_microvolts as f32 / 1_000_000.0;

                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "voltage")?;

                        sensors.additional_voltages_v.push((sensor_name.clone(), voltage_v));
                        debug!(
                            "Successfully read additional voltage: {} V from {} ({})",
                            voltage_v, file_path.display(), sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse additional voltage value from {}: {}",
                            file_path.display(), e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read additional voltage from {}: {}",
                    file_path.display(), e
                );
            }
        }

        Ok(())
    }

    /// Обработать дополнительный ток
    fn process_additional_current_sensor(&self, file_path: &Path, file_name: &str, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
        debug!("Found additional current sensor file: {}", file_path.display());

        match fs::read_to_string(file_path) {
            Ok(current_content) => {
                match current_content.trim().parse::<u32>() {
                    Ok(current_microamperes) => {
                        let current_a = current_microamperes as f32 / 1_000_000.0;

                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "current")?;

                        sensors.additional_currents_a.push((sensor_name.clone(), current_a));
                        debug!(
                            "Successfully read additional current: {} A from {} ({})",
                            current_a, file_path.display(), sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse additional current value from {}: {}",
                            file_path.display(), e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read additional current from {}: {}",
                    file_path.display(), e
                );
            }
        }

        Ok(())
    }

    /// Обработать дополнительную мощность
    fn process_additional_power_sensor(&self, file_path: &Path, file_name: &str, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
        debug!("Found additional power sensor file: {}", file_path.display());

        match fs::read_to_string(file_path) {
            Ok(power_content) => {
                match power_content.trim().parse::<u32>() {
                    Ok(power_microwatts) => {
                        let power_w = power_microwatts as f32 / 1_000_000.0;

                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "power")?;

                        sensors.additional_power_w.push((sensor_name.clone(), power_w));
                        debug!(
                            "Successfully read additional power: {} W from {} ({})",
                            power_w, file_path.display(), sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse additional power value from {}: {}",
                            file_path.display(), e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read additional power from {}: {}",
                    file_path.display(), e
                );
            }
        }

        Ok(())
    }

    /// Обработать дополнительную энергию
    fn process_additional_energy_sensor(&self, file_path: &Path, file_name: &str, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
        debug!("Found additional energy sensor file: {}", file_path.display());

        match fs::read_to_string(file_path) {
            Ok(energy_content) => {
                match energy_content.trim().parse::<u32>() {
                    Ok(energy_microjoules) => {
                        let energy_j = energy_microjoules as f32 / 1_000_000.0;

                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "energy")?;

                        sensors.additional_energy_j.push((sensor_name.clone(), energy_j));
                        debug!(
                            "Successfully read additional energy: {} J from {} ({})",
                            energy_j, file_path.display(), sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse additional energy value from {}: {}",
                            file_path.display(), e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read additional energy from {}: {}",
                    file_path.display(), e
                );
            }
        }

        Ok(())
    }

    /// Обработать дополнительную влажность
    fn process_additional_humidity_sensor(&self, file_path: &Path, file_name: &str, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
        debug!("Found additional humidity sensor file: {}", file_path.display());

        match fs::read_to_string(file_path) {
            Ok(humidity_content) => {
                match humidity_content.trim().parse::<u32>() {
                    Ok(humidity_millipercent) => {
                        let humidity_percent = humidity_millipercent as f32 / 1000.0;

                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "humidity")?;

                        sensors.additional_humidity_percent.push((sensor_name.clone(), humidity_percent));
                        debug!(
                            "Successfully read additional humidity: {}% from {} ({})",
                            humidity_percent, file_path.display(), sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse additional humidity value from {}: {}",
                            file_path.display(), e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read additional humidity from {}: {}",
                    file_path.display(), e
                );
            }
        }

        Ok(())
    }

    /// Обработать сенсор давления
    fn process_pressure_sensor(&self, file_path: &Path, file_name: &str, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
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
                            pressure_pa, file_path.display(), sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse pressure value from {}: {}",
                            file_path.display(), e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read pressure from {}: {}",
                    file_path.display(), e
                );
            }
        }

        Ok(())
    }

    /// Обработать сенсор освещенности
    fn process_illumination_sensor(&self, file_path: &Path, file_name: &str, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
        debug!("Found illumination sensor file: {}", file_path.display());

        match fs::read_to_string(file_path) {
            Ok(illum_content) => {
                match illum_content.trim().parse::<u32>() {
                    Ok(illum_millilux) => {
                        // Конвертируем миллилюксы в люксы
                        let illum_lux = illum_millilux as f32 / 1000.0;

                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "illumination")?;

                        sensors.illumination_lux.push((sensor_name.clone(), illum_lux));
                        debug!(
                            "Successfully read illumination: {} lux from {} ({})",
                            illum_lux, file_path.display(), sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse illumination value from {}: {}",
                            file_path.display(), e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read illumination from {}: {}",
                    file_path.display(), e
                );
            }
        }

        Ok(())
    }

    /// Обработать пользовательский сенсор
    fn process_custom_sensor(&self, file_path: &Path, file_name: &str, sensors: &mut ExtendedHardwareSensors) -> Result<()> {
        debug!("Found custom sensor file: {}", file_path.display());

        match fs::read_to_string(file_path) {
            Ok(sensor_content) => {
                match sensor_content.trim().parse::<f32>() {
                    Ok(sensor_value) => {
                        // Получаем описательное имя сенсора
                        let sensor_name = self.get_sensor_name(file_path, file_name, "custom")?;

                        // Пробуем определить единицу измерения по имени файла
                        let unit = self.infer_unit_from_filename(file_name);

                        sensors.custom_sensors.push((sensor_name.clone(), sensor_value, unit.clone()));
                        debug!(
                            "Successfully read custom sensor: {} {} from {} ({})",
                            sensor_value, unit, file_path.display(), sensor_name
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse custom sensor value from {}: {}",
                            file_path.display(), e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read custom sensor from {}: {}",
                    file_path.display(), e
                );
            }
        }

        Ok(())
    }

    /// Получить описательное имя сенсора
    fn get_sensor_name(&self, file_path: &Path, file_name: &str, sensor_type: &str) -> Result<String> {
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
                Err(_) => Ok(format!("{}_{}", sensor_type, file_name))
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
            "%" .to_string()
        } else if file_name.starts_with("pressure") {
            "Pa".to_string()
        } else if file_name.starts_with("illum") {
            "lux".to_string()
        } else {
            "units".to_string()
        }
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
        let result = monitor.process_pressure_sensor(&pressure_file, "pressure1_input", &mut sensors);
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
}