//! Мониторинг использования GPU процессами.
//!
//! Этот модуль предоставляет функции для сбора метрик использования GPU
//! на уровне отдельных процессов. Использует различные источники данных:
//! - DRM (Direct Rendering Manager) для открытых GPU устройств
//! - NVIDIA NVML (если доступно)
//! - AMD GPU метрики через sysfs
//! - eBPF для мониторинга вызовов GPU API

use crate::logging::snapshots::ProcessRecord;
use anyhow::Result;
use glob;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tracing::{debug, error, info};

/// Информация о GPU, используемом процессом
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessGpuInfo {
    /// Идентификатор процесса
    pub pid: i32,
    /// Идентификатор GPU устройства
    pub gpu_id: String,
    /// Тип GPU (nvidia, amd, intel, etc.)
    pub gpu_type: String,
    /// Использование GPU в процентах (0.0 до 1.0)
    pub utilization: f32,
    /// Использование памяти GPU в байтах
    pub memory_bytes: u64,
    /// Время выполнения на GPU в микросекундах
    pub gpu_time_us: u64,
    /// Количество вызовов GPU API
    pub api_calls: u64,
    /// Температура GPU в градусах Цельсия
    pub temperature_c: Option<f32>,
    /// Потребление энергии GPU в ваттах
    pub power_watts: Option<f32>,
    /// Временная метка последнего обновления
    pub last_update: SystemTime,
    /// Источник данных
    pub data_source: String,
}

impl Default for ProcessGpuInfo {
    fn default() -> Self {
        Self {
            pid: 0,
            gpu_id: String::new(),
            gpu_type: String::new(),
            utilization: 0.0,
            memory_bytes: 0,
            gpu_time_us: 0,
            api_calls: 0,
            temperature_c: None,
            power_watts: None,
            last_update: SystemTime::now(),
            data_source: String::new(),
        }
    }
}

/// Кэш метрик GPU для процессов
#[derive(Debug, Clone)]
pub struct ProcessGpuCache {
    /// Кэш метрик GPU для процессов
    cache: Arc<RwLock<HashMap<i32, ProcessGpuInfo>>>,
    /// Время жизни кэша в секундах
    cache_ttl_seconds: u64,
    /// Максимальное количество кэшируемых процессов
    max_cached_processes: usize,
    /// Включено ли кэширование
    enable_caching: bool,
}

impl ProcessGpuCache {
    /// Создаёт новый кэш метрик GPU для процессов
    pub fn new(cache_ttl_seconds: u64, max_cached_processes: usize, enable_caching: bool) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::with_capacity(max_cached_processes))),
            cache_ttl_seconds,
            max_cached_processes,
            enable_caching,
        }
    }

    /// Создаёт новый кэш метрик GPU для процессов с конфигурацией по умолчанию
    pub fn new_default() -> Self {
        Self::new(60, 1000, true)
    }

    /// Добавляет метрики GPU для процесса в кэш
    pub async fn add_process_gpu_info(&self, process_gpu_info: ProcessGpuInfo) -> Result<()> {
        if !self.enable_caching {
            return Ok(());
        }

        let mut cache = match self.cache.write() {
            Ok(cache) => cache,
            Err(e) => {
                error!("Failed to get write lock: {}", e);
                return Ok(());
            }
        };
        
        // Проверяем, нужно ли очистить старые записи
        if cache.len() >= self.max_cached_processes {
            if let Err(e) = self.cleanup_old_entries().await {
                error!("Failed to cleanup old entries: {}", e);
            }
        }

        cache.insert(process_gpu_info.pid, process_gpu_info);
        Ok(())
    }

    /// Получает метрики GPU для процесса из кэша
    pub async fn get_process_gpu_info(&self, pid: i32) -> Option<ProcessGpuInfo> {
        let cache = self.cache.read().ok()?;
        cache.get(&pid).cloned()
    }

    /// Очищает старые записи из кэша
    pub async fn cleanup_old_entries(&self) -> Result<()> {
        let mut cache = self.cache.write().map_err(|e| {
            anyhow::anyhow!("Failed to get write lock: {}", e)
        })?;
        let now = SystemTime::now();
        
        if self.cache_ttl_seconds == 0 {
            return Ok(()); // Кэш не имеет ограничения по времени
        }

        let cutoff_time = now
            .checked_sub(Duration::from_secs(self.cache_ttl_seconds))
            .unwrap_or(now);

        cache.retain(|_, info| {
            info.last_update >= cutoff_time
        });

        Ok(())
    }

    /// Очищает весь кэш
    pub async fn clear_cache(&self) -> Result<()> {
        let mut cache = self.cache.write().map_err(|e| {
            anyhow::anyhow!("Failed to get write lock: {}", e)
        })?;
        cache.clear();
        Ok(())
    }

    /// Возвращает количество записей в кэше
    pub async fn cache_size(&self) -> usize {
        match self.cache.read() {
            Ok(cache) => cache.len(),
            Err(e) => {
                error!("Failed to get read lock: {}", e);
                0
            }
        }
    }
}

/// Основная структура для мониторинга использования GPU процессами
#[derive(Debug, Clone)]
pub struct ProcessGpuMonitor {
    /// Кэш метрик GPU
    gpu_cache: ProcessGpuCache,
    /// Конфигурация мониторинга
    config: ProcessGpuMonitorConfig,
}

/// Конфигурация мониторинга использования GPU процессами
#[derive(Debug, Clone)]
pub struct ProcessGpuMonitorConfig {
    /// Включить мониторинг через DRM
    pub enable_drm_monitoring: bool,
    /// Включить мониторинг через NVIDIA NVML (если доступно)
    pub enable_nvidia_monitoring: bool,
    /// Включить мониторинг через AMD GPU метрики
    pub enable_amd_monitoring: bool,
    /// Включить мониторинг через eBPF
    pub enable_ebpf_monitoring: bool,
    /// Интервал опроса в секундах
    pub polling_interval_seconds: u64,
    /// Включить кэширование
    pub enable_caching: bool,
    /// Время жизни кэша в секундах
    pub cache_ttl_seconds: u64,
    /// Максимальное количество кэшируемых процессов
    pub max_cached_processes: usize,
}

impl Default for ProcessGpuMonitorConfig {
    fn default() -> Self {
        Self {
            enable_drm_monitoring: true,
            enable_nvidia_monitoring: true,
            enable_amd_monitoring: true,
            enable_ebpf_monitoring: false,
            polling_interval_seconds: 5,
            enable_caching: true,
            cache_ttl_seconds: 60,
            max_cached_processes: 1000,
        }
    }
}

impl ProcessGpuMonitor {
    /// Создаёт новый монитор использования GPU процессами
    pub fn new(config: ProcessGpuMonitorConfig) -> Self {
        let gpu_cache = ProcessGpuCache::new(
            config.cache_ttl_seconds,
            config.max_cached_processes,
            config.enable_caching,
        );

        Self { gpu_cache, config }
    }

    /// Создаёт новый монитор использования GPU процессами с конфигурацией по умолчанию
    pub fn new_default() -> Self {
        Self::new(ProcessGpuMonitorConfig::default())
    }

    /// Собирает метрики использования GPU для всех процессов
    pub async fn collect_process_gpu_metrics(&self) -> Result<HashMap<i32, ProcessGpuInfo>> {
        let mut process_gpu_metrics = HashMap::new();
        
        // Пробуем собрать метрики через DRM
        if self.config.enable_drm_monitoring {
            if let Ok(drm_metrics) = self.collect_drm_gpu_metrics().await {
                process_gpu_metrics.extend(drm_metrics);
            }
        }
        
        // Пробуем собрать метрики через NVIDIA NVML
        if self.config.enable_nvidia_monitoring {
            if let Ok(nvidia_metrics) = self.collect_nvidia_gpu_metrics().await {
                process_gpu_metrics.extend(nvidia_metrics);
            }
        }
        
        // Пробуем собрать метрики через AMD
        if self.config.enable_amd_monitoring {
            if let Ok(amd_metrics) = self.collect_amd_gpu_metrics().await {
                process_gpu_metrics.extend(amd_metrics);
            }
        }
        
        // Пробуем собрать метрики через eBPF
        if self.config.enable_ebpf_monitoring {
            if let Ok(ebpf_metrics) = self.collect_ebpf_gpu_metrics().await {
                process_gpu_metrics.extend(ebpf_metrics);
            }
        }
        
        Ok(process_gpu_metrics)
    }

    /// Собирает метрики использования GPU через DRM
    async fn collect_drm_gpu_metrics(&self) -> Result<HashMap<i32, ProcessGpuInfo>> {
        let metrics = HashMap::new();
        
        // Пробуем найти DRM устройства
        let drm_dir = Path::new("/sys/class/drm");
        if !drm_dir.exists() {
            debug!("DRM directory not found, skipping DRM GPU monitoring");
            return Ok(metrics);
        }
        
        // Пробуем прочитать информацию о процессах, использующих DRM
        // Это упрощённая реализация - в реальной системе нужно использовать
        // более сложные методы для получения информации о процессах
        
        info!("Collecting GPU metrics via DRM (simplified implementation)");
        
        // В реальной реализации здесь будет:
        // 1. Чтение из /proc/<pid>/fd/ для поиска DRM файлов
        // 2. Анализ открытых файлов DRM устройств
        // 3. Получение метрик использования из sysfs
        
        // Пока возвращаем пустые метрики
        Ok(metrics)
    }

    /// Собирает метрики использования GPU через NVIDIA NVML
    async fn collect_nvidia_gpu_metrics(&self) -> Result<HashMap<i32, ProcessGpuInfo>> {
        let metrics = HashMap::new();
        
        // Пробуем найти NVIDIA устройства
        let nvidia_dir = Path::new("/proc/driver/nvidia");
        if !nvidia_dir.exists() {
            debug!("NVIDIA driver directory not found, skipping NVIDIA GPU monitoring");
            return Ok(metrics);
        }
        
        info!("Collecting GPU metrics via NVIDIA NVML (simplified implementation)");
        
        // В реальной реализации здесь будет:
        // 1. Использование NVIDIA NVML библиотеки
        // 2. Получение информации о процессах, использующих GPU
        // 3. Сбор метрик использования GPU
        
        // Пока возвращаем пустые метрики
        Ok(metrics)
    }

    /// Собирает метрики использования GPU через AMD
    async fn collect_amd_gpu_metrics(&self) -> Result<HashMap<i32, ProcessGpuInfo>> {
        let metrics = HashMap::new();
        
        // Пробуем найти AMD GPU устройства
        let amdgpu_dir = Path::new("/sys/class/drm");
        if !amdgpu_dir.exists() {
            debug!("AMD GPU directory not found, skipping AMD GPU monitoring");
            return Ok(metrics);
        }
        
        info!("Collecting GPU metrics via AMD (simplified implementation)");
        
        // В реальной реализации здесь будет:
        // 1. Чтение из sysfs для AMD GPU
        // 2. Анализ использования GPU процессами
        // 3. Сбор метрик температуры и мощности
        
        // Пока возвращаем пустые метрики
        Ok(metrics)
    }

    /// Собирает метрики использования GPU через eBPF
    async fn collect_ebpf_gpu_metrics(&self) -> Result<HashMap<i32, ProcessGpuInfo>> {
        let metrics = HashMap::new();
        
        info!("Collecting GPU metrics via eBPF (simplified implementation)");
        
        // В реальной реализации здесь будет:
        // 1. Использование eBPF программ для мониторинга вызовов GPU API
        // 2. Сбор метрик использования GPU на уровне процессов
        // 3. Анализ времени выполнения на GPU
        
        // Пока возвращаем пустые метрики
        Ok(metrics)
    }

    /// Обновляет метрики GPU для ProcessRecord
    pub async fn update_process_record_with_gpu_metrics(
        &self,
        process_record: &mut ProcessRecord,
    ) -> Result<()> {
        if let Some(gpu_info) = self.get_process_gpu_info(process_record.pid).await? {
            process_record.gpu_utilization = Some(gpu_info.utilization);
            process_record.gpu_memory_bytes = Some(gpu_info.memory_bytes);
            process_record.gpu_time_us = Some(gpu_info.gpu_time_us);
            process_record.gpu_api_calls = Some(gpu_info.api_calls);
            process_record.gpu_last_update_ns = Some(
                gpu_info
                    .last_update
                    .duration_since(SystemTime::UNIX_EPOCH)?
                    .as_nanos() as u64,
            );
            process_record.gpu_data_source = Some(gpu_info.data_source);
        }
        
        Ok(())
    }

    /// Получает метрики GPU для процесса
    pub async fn get_process_gpu_info(&self, pid: i32) -> Result<Option<ProcessGpuInfo>> {
        // Пробуем получить из кэша
        if let Some(cached_info) = self.gpu_cache.get_process_gpu_info(pid).await {
            return Ok(Some(cached_info));
        }
        
        // Если нет в кэше, собираем новые метрики
        let metrics = self.collect_process_gpu_metrics().await?;
        if let Some(info) = metrics.get(&pid) {
            self.gpu_cache.add_process_gpu_info(info.clone()).await?;
            Ok(Some(info.clone()))
        } else {
            Ok(None)
        }
    }

    /// Очищает кэш метрик GPU
    pub async fn clear_gpu_cache(&self) -> Result<()> {
        self.gpu_cache.clear_cache().await
    }

    /// Возвращает количество процессов с метриками GPU в кэше
    pub async fn gpu_cache_size(&self) -> usize {
        self.gpu_cache.cache_size().await
    }
}

lazy_static! {
    /// Глобальный кэш метрик GPU для процессов
    pub static ref GLOBAL_PROCESS_GPU_CACHE: Arc<ProcessGpuCache> = 
        Arc::new(ProcessGpuCache::new_default());
}

lazy_static! {
    /// Глобальный монитор использования GPU процессами
    pub static ref GLOBAL_PROCESS_GPU_MONITOR: Arc<ProcessGpuMonitor> = 
        Arc::new(ProcessGpuMonitor::new_default());
}

/// Вспомогательная функция для создания ProcessGpuMonitor
pub fn create_process_gpu_monitor(config: ProcessGpuMonitorConfig) -> Arc<ProcessGpuMonitor> {
    Arc::new(ProcessGpuMonitor::new(config))
}

/// Вспомогательная функция для создания ProcessGpuMonitor с конфигурацией по умолчанию
pub fn create_default_process_gpu_monitor() -> Arc<ProcessGpuMonitor> {
    Arc::new(ProcessGpuMonitor::new_default())
}

/// Вспомогательная функция для получения метрик GPU для процесса
pub async fn get_process_gpu_metrics(pid: i32) -> Result<Option<ProcessGpuInfo>> {
    GLOBAL_PROCESS_GPU_MONITOR.get_process_gpu_info(pid).await
}

/// Вспомогательная функция для обновления ProcessRecord метриками GPU
pub async fn update_process_record_gpu_metrics(process_record: &mut ProcessRecord) -> Result<()> {
    GLOBAL_PROCESS_GPU_MONITOR
        .update_process_record_with_gpu_metrics(process_record)
        .await
}

/// Вспомогательная функция для очистки кэша метрик GPU
pub async fn clear_process_gpu_cache() -> Result<()> {
    GLOBAL_PROCESS_GPU_MONITOR.clear_gpu_cache().await
}

/// Вспомогательная функция для получения размера кэша метрик GPU
pub async fn get_process_gpu_cache_size() -> usize {
    GLOBAL_PROCESS_GPU_MONITOR.gpu_cache_size().await
}

/// Собирает метрики температуры и энергопотребления GPU
///
/// Эта функция собирает метрики температуры и энергопотребления для всех доступных GPU устройств.
/// Поддерживает различные типы GPU: NVIDIA, AMD, Intel.
pub async fn collect_gpu_temperature_and_power() -> Result<Vec<GpuTemperaturePowerInfo>> {
    let mut gpu_metrics = Vec::new();

    // Пробуем собрать метрики для NVIDIA GPU
    if let Ok(nvidia_metrics) = collect_nvidia_gpu_metrics().await {
        gpu_metrics.extend(nvidia_metrics);
    }

    // Пробуем собрать метрики для AMD GPU
    if let Ok(amd_metrics) = collect_amd_gpu_metrics().await {
        gpu_metrics.extend(amd_metrics);
    }

    // Пробуем собрать метрики для Intel GPU
    if let Ok(intel_metrics) = collect_intel_gpu_metrics().await {
        gpu_metrics.extend(intel_metrics);
    }

    Ok(gpu_metrics)
}

/// Информация о температуре и энергопотреблении GPU
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuTemperaturePowerInfo {
    /// Идентификатор GPU устройства
    pub gpu_id: String,
    /// Тип GPU (nvidia, amd, intel, etc.)
    pub gpu_type: String,
    /// Температура GPU в градусах Цельсия
    pub temperature_c: f32,
    /// Потребление энергии GPU в ваттах
    pub power_watts: f32,
    /// Максимальная температура GPU в градусах Цельсия
    pub max_temperature_c: f32,
    /// Максимальное потребление энергии GPU в ваттах
    pub max_power_watts: f32,
    /// Временная метка сбора метрик
    pub timestamp: SystemTime,
}

impl Default for GpuTemperaturePowerInfo {
    fn default() -> Self {
        Self {
            gpu_id: String::new(),
            gpu_type: String::new(),
            temperature_c: 0.0,
            power_watts: 0.0,
            max_temperature_c: 0.0,
            max_power_watts: 0.0,
            timestamp: SystemTime::now(),
        }
    }
}

/// Собирает метрики температуры и энергопотребления для NVIDIA GPU
async fn collect_nvidia_gpu_metrics() -> Result<Vec<GpuTemperaturePowerInfo>> {
    let mut metrics = Vec::new();

    // Пробуем найти NVIDIA GPU устройства
    // В реальной реализации здесь будет использование NVML (NVIDIA Management Library)
    // Для упрощения возвращаем заглушки
    
    // Проверяем наличие NVIDIA GPU через sysfs
    let nvidia_gpu_paths: glob::Paths = glob::glob("/sys/class/drm/card*/device/vendor")?;
    
    for path_result in nvidia_gpu_paths {
        if let Ok(path) = path_result {
            if let Ok(vendor_content) = fs::read_to_string(&path) {
                if vendor_content.trim() == "0x10de" { // NVIDIA vendor ID
                    let card_id = path.parent().and_then(|p: &std::path::Path| p.parent())
                        .and_then(|p: &std::path::Path| p.file_name())
                        .and_then(|n: &std::ffi::OsStr| n.to_str())
                        .unwrap_or("unknown").to_string();
                    
                    // Собираем метрики температуры и энергопотребления
                    // В реальной реализации здесь будет использование NVML
                    let temp_path = format!("/sys/class/drm/{}/device/hwmon/hwmon*/temp1_input", card_id);
                    let power_path = format!("/sys/class/drm/{}/device/hwmon/hwmon*/power1_input", card_id);
                    
                    let temperature = if let Ok(mut temp_files) = glob::glob(&temp_path) {
                        if let Some(temp_file_result) = temp_files.next() {
                            if let Ok(temp_file) = temp_file_result {
                                if let Ok(temp_content) = fs::read_to_string(&temp_file) {
                                    if let Ok(temp_millis) = temp_content.trim().parse::<u64>() {
                                        temp_millis as f32 / 1000.0
                                    } else {
                                        0.0
                                    }
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };
                    
                    let power = if let Ok(mut power_files) = glob::glob(&power_path) {
                        if let Some(power_file_result) = power_files.next() {
                            if let Ok(power_file) = power_file_result {
                                if let Ok(power_content) = fs::read_to_string(&power_file) {
                                    if let Ok(power_microwatts) = power_content.trim().parse::<u64>() {
                                        power_microwatts as f32 / 1_000_000.0
                                    } else {
                                        0.0
                                    }
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };
                    
                    metrics.push(GpuTemperaturePowerInfo {
                        gpu_id: card_id,
                        gpu_type: "nvidia".to_string(),
                        temperature_c: temperature,
                        power_watts: power,
                        max_temperature_c: 95.0, // Типичное максимальное значение для NVIDIA GPU
                        max_power_watts: 300.0, // Типичное максимальное значение для NVIDIA GPU
                        timestamp: SystemTime::now(),
                    });
                }
            }
        }
    }

    Ok(metrics)
}

/// Собирает метрики температуры и энергопотребления для AMD GPU
async fn collect_amd_gpu_metrics() -> Result<Vec<GpuTemperaturePowerInfo>> {
    let mut metrics = Vec::new();

    // Пробуем найти AMD GPU устройства
    let amd_gpu_paths = glob::glob("/sys/class/drm/card*/device/vendor")?;
    
    for path in amd_gpu_paths {
        if let Ok(path) = path {
            if let Ok(vendor_content) = fs::read_to_string(&path) {
                if vendor_content.trim() == "0x1002" { // AMD vendor ID
                    let card_id = path.parent().and_then(|p| p.parent())
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown").to_string();
                    
                    // Собираем метрики температуры и энергопотребления
                    let temp_path = format!("/sys/class/drm/{}/device/hwmon/hwmon*/temp1_input", card_id);
                    let power_path = format!("/sys/class/drm/{}/device/hwmon/hwmon*/power1_average", card_id);
                    
                    let temperature = if let Ok(mut temp_files) = glob::glob(&temp_path) {
                        if let Some(temp_file) = temp_files.next() {
                            if let Ok(temp_file) = temp_file {
                                if let Ok(temp_content) = fs::read_to_string(&temp_file) {
                                    if let Ok(temp_millis) = temp_content.trim().parse::<u64>() {
                                        temp_millis as f32 / 1000.0
                                    } else {
                                        0.0
                                    }
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };
                    
                    let power = if let Ok(mut power_files) = glob::glob(&power_path) {
                        if let Some(power_file) = power_files.next() {
                            if let Ok(power_file) = power_file {
                                if let Ok(power_content) = fs::read_to_string(&power_file) {
                                    if let Ok(power_microwatts) = power_content.trim().parse::<u64>() {
                                        power_microwatts as f32 / 1_000_000.0
                                    } else {
                                        0.0
                                    }
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };
                    
                    metrics.push(GpuTemperaturePowerInfo {
                        gpu_id: card_id,
                        gpu_type: "amd".to_string(),
                        temperature_c: temperature,
                        power_watts: power,
                        max_temperature_c: 110.0, // Типичное максимальное значение для AMD GPU
                        max_power_watts: 350.0, // Типичное максимальное значение для AMD GPU
                        timestamp: SystemTime::now(),
                    });
                }
            }
        }
    }

    Ok(metrics)
}

/// Собирает метрики температуры и энергопотребления для Intel GPU
async fn collect_intel_gpu_metrics() -> Result<Vec<GpuTemperaturePowerInfo>> {
    let mut metrics = Vec::new();

    // Пробуем найти Intel GPU устройства
    let intel_gpu_paths = glob::glob("/sys/class/drm/card*/device/vendor")?;
    
    for path in intel_gpu_paths {
        if let Ok(path) = path {
            if let Ok(vendor_content) = fs::read_to_string(&path) {
                if vendor_content.trim() == "0x8086" { // Intel vendor ID
                    let card_id = path.parent().and_then(|p| p.parent())
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown").to_string();
                    
                    // Собираем метрики температуры
                    let temp_path = format!("/sys/class/drm/{}/device/hwmon/hwmon*/temp1_input", card_id);
                    
                    let temperature = if let Ok(mut temp_files) = glob::glob(&temp_path) {
                        if let Some(temp_file) = temp_files.next() {
                            if let Ok(temp_file) = temp_file {
                                if let Ok(temp_content) = fs::read_to_string(&temp_file) {
                                    if let Ok(temp_millis) = temp_content.trim().parse::<u64>() {
                                        temp_millis as f32 / 1000.0
                                    } else {
                                        0.0
                                    }
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };
                    
                    // Для Intel GPU энергопотребление может быть недоступно через sysfs
                    // В реальной реализации можно использовать другие источники
                    let power = 0.0;
                    
                    metrics.push(GpuTemperaturePowerInfo {
                        gpu_id: card_id,
                        gpu_type: "intel".to_string(),
                        temperature_c: temperature,
                        power_watts: power,
                        max_temperature_c: 100.0, // Типичное максимальное значение для Intel GPU
                        max_power_watts: 100.0, // Типичное максимальное значение для Intel GPU
                        timestamp: SystemTime::now(),
                    });
                }
            }
        }
    }

    Ok(metrics)
}

/// Обновляет метрики температуры и энергопотребления GPU для процессов
///
/// Эта функция обновляет информацию о температуре и энергопотреблении GPU
/// в существующих метриках процессов.
pub async fn update_process_gpu_temperature_and_power() -> Result<()> {
    let gpu_metrics = collect_gpu_temperature_and_power().await?;
    
    // Обновляем кэш процессов с новой информацией о температуре и энергопотреблении
    let process_gpu_cache = GLOBAL_PROCESS_GPU_CACHE.clone();
    let cache_read = match process_gpu_cache.cache.read() {
        Ok(guard) => guard,
        Err(e) => {
            error!("Failed to get read lock for GPU cache: {}", e);
            return Ok(());
        }
    };
    let mut updates = Vec::new();
    
    for (pid, process_gpu_info) in cache_read.iter() {
        // Находим соответствующие метрики GPU
        for gpu_metric in &gpu_metrics {
            if process_gpu_info.gpu_id == gpu_metric.gpu_id {
                let mut updated_info = process_gpu_info.clone();
                updated_info.temperature_c = Some(gpu_metric.temperature_c);
                updated_info.power_watts = Some(gpu_metric.power_watts);
                updates.push((*pid, updated_info));
                break;
            }
        }
    }
    
    // Применяем обновления
    if !updates.is_empty() {
        let mut cache_write = match process_gpu_cache.cache.write() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to get write lock for GPU cache: {}", e);
                return Ok(());
            }
        };
        for (pid, updated_info) in updates {
            cache_write.insert(pid, updated_info);
        }
    }
    
    Ok(())
}

/// Глобальные функции для работы с температурой и энергопотреблением GPU
pub async fn collect_global_gpu_temperature_and_power() -> Result<Vec<GpuTemperaturePowerInfo>> {
    collect_gpu_temperature_and_power().await
}

pub async fn update_global_process_gpu_temperature_and_power() -> Result<()> {
    update_process_gpu_temperature_and_power().await
}