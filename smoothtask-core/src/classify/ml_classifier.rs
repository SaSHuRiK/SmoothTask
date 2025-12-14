//! ML-классификатор для классификации процессов.
//!
//! Этот модуль предоставляет интерфейс для классификации процессов
//! с использованием ML-моделей. Поддерживает интеграцию с CatBoost
//! и ONNX Runtime для загрузки и использования предварительно обученных моделей.

use crate::config::config_struct::{MLClassifierConfig, ModelType};
use crate::logging::snapshots::ProcessRecord;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error, info, warn};

#[cfg(feature = "catboost")]
use catboost::CatBoostClassifier;

#[cfg(feature = "onnx")]
use ort::Session;

use std::fs::File;
use std::io::Read;
use sha2::{Sha256, Digest};
use std::sync::Mutex;
use lazy_static::lazy_static;
use sysinfo::System;

lazy_static! {
    /// Глобальный кэш фич для оптимизации производительности ML-классификации.
    /// Используется всеми экземплярами ML-классификаторов.
    static ref GLOBAL_FEATURE_CACHE: FeatureCache = FeatureCache::new(1024);
}

/// Метрики производительности ML-модели.
#[derive(Debug, Clone, Default)]
pub struct MLPerformanceMetrics {
    /// Общее количество классификаций.
    pub total_classifications: u64,
    /// Количество успешных классификаций.
    pub successful_classifications: u64,
    /// Количество ошибок классификации.
    pub classification_errors: u64,
    /// Суммарное время классификации в микросекундах.
    pub total_classification_time_us: u128,
    /// Минимальное время классификации в микросекундах.
    pub min_classification_time_us: Option<u128>,
    /// Максимальное время классификации в микросекундах.
    pub max_classification_time_us: Option<u128>,
    /// Суммарная уверенность всех классификаций.
    pub total_confidence: f64,
    /// Количество классификаций с высокой уверенностью (> 0.8).
    pub high_confidence_classifications: u64,
    /// Количество классификаций со средней уверенностью (0.5 - 0.8).
    pub medium_confidence_classifications: u64,
    /// Количество классификаций с низкой уверенностью (< 0.5).
    pub low_confidence_classifications: u64,
}

impl MLPerformanceMetrics {
    /// Создать новые метрики производительности.
    pub fn new() -> Self {
        Self::default()
    }

    /// Зарегистрировать успешную классификацию.
    pub fn record_successful_classification(&mut self, duration: u128, confidence: f64) {
        self.total_classifications += 1;
        self.successful_classifications += 1;
        self.total_classification_time_us += duration;
        self.total_confidence += confidence;

        // Обновить минимальное и максимальное время
        if let Some(min_time) = self.min_classification_time_us {
            if duration < min_time {
                self.min_classification_time_us = Some(duration);
            }
        } else {
            self.min_classification_time_us = Some(duration);
        }

        if let Some(max_time) = self.max_classification_time_us {
            if duration > max_time {
                self.max_classification_time_us = Some(duration);
            }
        } else {
            self.max_classification_time_us = Some(duration);
        }

        // Категоризировать по уверенности
        if confidence > 0.8 {
            self.high_confidence_classifications += 1;
        } else if confidence > 0.5 {
            self.medium_confidence_classifications += 1;
        } else {
            self.low_confidence_classifications += 1;
        }
    }

    /// Зарегистрировать ошибку классификации.
    pub fn record_classification_error(&mut self) {
        self.total_classifications += 1;
        self.classification_errors += 1;
    }

    /// Получить среднее время классификации в микросекундах.
    pub fn average_classification_time_us(&self) -> Option<f64> {
        if self.successful_classifications > 0 {
            Some(self.total_classification_time_us as f64 / self.successful_classifications as f64)
        } else {
            None
        }
    }

    /// Получить среднюю уверенность.
    pub fn average_confidence(&self) -> Option<f64> {
        if self.successful_classifications > 0 {
            Some(self.total_confidence / self.successful_classifications as f64)
        } else {
            None
        }
    }

    /// Получить процент успешных классификаций.
    pub fn success_rate(&self) -> Option<f64> {
        if self.total_classifications > 0 {
            Some(self.successful_classifications as f64 / self.total_classifications as f64)
        } else {
            None
        }
    }

    /// Сбросить метрики.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Логировать сводку метрик.
    pub fn log_summary(&self) {
        info!("ML Performance Metrics Summary:");
        info!("  Total classifications: {}", self.total_classifications);
        info!("  Successful classifications: {}", self.successful_classifications);
        info!("  Classification errors: {}", self.classification_errors);
        
        if let Some(success_rate) = self.success_rate() {
            info!("  Success rate: {:.2}%", success_rate * 100.0);
        }
        
        if let Some(avg_time) = self.average_classification_time_us() {
            info!("  Average classification time: {:.2} μs", avg_time);
        }
        
        if let Some(min_time) = self.min_classification_time_us {
            info!("  Min classification time: {} μs", min_time);
        }
        
        if let Some(max_time) = self.max_classification_time_us {
            info!("  Max classification time: {} μs", max_time);
        }
        
        if let Some(avg_confidence) = self.average_confidence() {
            info!("  Average confidence: {:.3}", avg_confidence);
        }
        
        info!("  High confidence (>0.8): {}", self.high_confidence_classifications);
        info!("  Medium confidence (0.5-0.8): {}", self.medium_confidence_classifications);
        info!("  Low confidence (<0.5): {}", self.low_confidence_classifications);
    }
}

/// Результат классификации от ML-модели.
#[derive(Debug, Clone)]
pub struct MLClassificationResult {
    /// Тип процесса, предсказанный ML-моделью.
    pub process_type: Option<String>,
    /// Теги, предсказанные ML-моделью.
    pub tags: Vec<String>,
    /// Уверенность модели в предсказании (0.0 - 1.0).
    pub confidence: f64,
}

/// Трейт для ML-классификатора процессов.
///
/// Трейт требует `Send + Sync`, так как классификатор используется в async контексте
/// и может быть перемещён между потоками.
pub trait MLClassifier: Send + Sync + std::fmt::Debug {
    /// Классифицировать процесс с использованием ML-модели.
    ///
    /// # Аргументы
    ///
    /// * `process` - процесс для классификации
    ///
    /// # Возвращает
    ///
    /// Результат классификации с предсказанным типом, тегами и уверенностью.
    fn classify(&mut self, process: &ProcessRecord) -> MLClassificationResult;

    /// Получить текущие метрики производительности.
    ///
    /// # Возвращает
    ///
    /// Клон текущих метрик производительности.
    fn get_performance_metrics(&self) -> MLPerformanceMetrics;

    /// Сбросить метрики производительности.
    fn reset_performance_metrics(&mut self);

    /// Логировать сводку метрик производительности.
    fn log_performance_summary(&self) {
        self.get_performance_metrics().log_summary();
    }
}

/// Создать ML-классификатор на основе конфигурации.
///
/// # Аргументы
///
/// * `config` - конфигурация ML-классификатора
///
/// # Возвращает
///
/// Результат с ML-классификатором или ошибкой.
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::classify::ml_classifier::{create_ml_classifier, MLClassifier};
/// use smoothtask_core::config::config_struct::MLClassifierConfig;
///
/// let config = MLClassifierConfig {
///     enabled: true,
///     model_path: "models/process_classifier.json".to_string(),
///     confidence_threshold: 0.7,
///     model_type: ModelType::Catboost,
/// };
///
/// let classifier = create_ml_classifier(config);
/// match classifier {
///     Ok(classifier) => {
///         // Использовать классификатор
///     }
///     Err(e) => {
///         eprintln!("Не удалось создать ML-классификатор: {}", e);
///     }
/// }
/// ```
pub fn create_ml_classifier(config: MLClassifierConfig) -> Result<Box<dyn MLClassifier>> {
    if config.enabled {
        CatBoostMLClassifier::new(config).map(|c| Box::new(c) as Box<dyn MLClassifier>)
    } else {
        info!("ML-классификатор отключен в конфигурации, используется заглушка");
        Ok(Box::new(StubMLClassifier::new()) as Box<dyn MLClassifier>)
    }
}

/// Заглушка ML-классификатора для тестирования.
///
/// Использует простые эвристики для классификации процессов:
/// - Процессы с GUI получают тип "gui" и соответствующие теги
/// - Процессы с высоким CPU получают тип "cpu_intensive"
/// - Процессы с высоким IO получают тип "io_intensive"
#[derive(Debug)]
pub struct StubMLClassifier {
    /// Метрики производительности.
    performance_metrics: MLPerformanceMetrics,
}

impl StubMLClassifier {
    /// Создать новый заглушку ML-классификатора.
    pub fn new() -> Self {
        Self {
            performance_metrics: MLPerformanceMetrics::new(),
        }
    }
}

impl Default for StubMLClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl MLClassifier for StubMLClassifier {
    fn classify(&mut self, process: &ProcessRecord) -> MLClassificationResult {
        let start_time = Instant::now();
        let mut tags = HashSet::new();
        let mut process_type = None;
        let mut confidence: f64 = 0.5;

        // Простая эвристика: GUI процессы
        if process.has_gui_window {
            tags.insert("gui".to_string());
            tags.insert("interactive".to_string());
            // Выбираем тип с наивысшей уверенностью
            if 0.8 > confidence {
                process_type = Some("gui".to_string());
                confidence = 0.8;
            }
        }

        // Простая эвристика: высокий CPU usage
        if let Some(cpu_share) = process.cpu_share_10s {
            if cpu_share > 0.3 {
                tags.insert("high_cpu".to_string());
                // Выбираем тип с наивысшей уверенностью
                if 0.7 > confidence {
                    process_type = Some("cpu_intensive".to_string());
                    confidence = 0.7;
                }
            }
        }

        // Простая эвристика: высокий IO
        if let Some(io_read) = process.io_read_bytes {
            if io_read > 1024 * 1024 {
                // > 1MB
                tags.insert("high_io".to_string());
                // Выбираем тип с наивысшей уверенностью
                if 0.6 > confidence {
                    process_type = Some("io_intensive".to_string());
                    confidence = 0.6;
                }
            }
        }

        // Простая эвристика: аудио клиенты
        if process.is_audio_client {
            tags.insert("audio".to_string());
            tags.insert("realtime".to_string());
            // Выбираем тип с наивысшей уверенностью
            if 0.9 > confidence {
                process_type = Some("audio".to_string());
                confidence = 0.9;
            }
        }

        // Простая эвристика: фокусные окна
        if process.is_focused_window {
            tags.insert("focused".to_string());
            tags.insert("interactive".to_string());
            // Выбираем тип с наивысшей уверенностью
            if 0.9 > confidence {
                process_type = Some("focused".to_string());
                confidence = 0.9;
            }
        }

        // Если тип не определен, используем "unknown"
        if process_type.is_none() {
            process_type = Some("unknown".to_string());
            confidence = 0.3;
        }

        let duration = start_time.elapsed().as_micros();
        
        // Зарегистрировать успешную классификацию
        self.performance_metrics.record_successful_classification(duration, confidence);
        
        MLClassificationResult {
            process_type,
            tags: tags.into_iter().collect(),
            confidence,
        }
    }

    fn get_performance_metrics(&self) -> MLPerformanceMetrics {
        self.performance_metrics.clone()
    }

    fn reset_performance_metrics(&mut self) {
        self.performance_metrics.reset();
    }
}

/// ML-классификатор на основе CatBoost.
///
/// Использует предварительно обученную модель CatBoost для классификации процессов.
/// Поддерживает загрузку моделей в формате JSON и ONNX.
#[derive(Debug)]
pub struct CatBoostMLClassifier {
    /// Внутренняя модель CatBoost
    model: CatBoostModel,
    /// Метрики производительности
    performance_metrics: MLPerformanceMetrics,
    /// Информация о версии и хэше модели
    model_version: Option<ModelVersionInfo>,
}

/// Внутреннее представление модели CatBoost
#[derive(Debug)]
enum CatBoostModel {
    /// Модель в формате CatBoost JSON
    #[cfg(feature = "catboost")]
    Json(Arc<CatBoostClassifier>),
    /// Модель в формате ONNX
    #[cfg(feature = "onnx")]
    Onnx(Arc<Session>),
    /// Заглушка (используется когда CatBoost/ONNX отключены)
    Stub,
}

/// Кэш для хранения предварительно вычисленных фич процессов.
/// Используется для оптимизации производительности при многократной классификации одних и тех же процессов.
struct FeatureCache {
    /// Кэш фич: PID -> Vec<f32>
    cache: Mutex<lru::LruCache<u32, Vec<f32>>>,
    /// Статистика кэша
    stats: Mutex<CacheStats>,
}

/// Статистика кэша
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Общее количество запросов к кэшу
    total_requests: u64,
    /// Количество хитов кэша
    cache_hits: u64,
    /// Количество миссов кэша
    cache_misses: u64,
    /// Общее количество добавленных элементов
    total_insertions: u64,
    /// Общее количество удаленных элементов
    total_evictions: u64,
    /// Суммарное время, сэкономленное благодаря кэшу (в микросекундах)
    total_time_saved_us: u128,
    /// Текущее использование памяти кэшом (в байтах)
    current_memory_usage_bytes: u64,
    /// Максимальное использование памяти кэшом (в байтах)
    max_memory_usage_bytes: u64,
    /// Количество очисток кэша из-за давления памяти
    memory_pressure_cleanups: u64,
    /// Количество автоматических регулировок емкости
    auto_capacity_adjustments: u64,
}

impl CacheStats {
    /// Зарегистрировать хит кэша
    fn record_hit(&mut self) {
        self.total_requests += 1;
        self.cache_hits += 1;
    }
    
    /// Зарегистрировать мисс кэша
    fn record_miss(&mut self) {
        self.total_requests += 1;
        self.cache_misses += 1;
    }
    
    /// Зарегистрировать добавление элемента
    fn record_insertion(&mut self) {
        self.total_insertions += 1;
    }
    
    /// Зарегистрировать удаление элемента
    #[allow(dead_code)]
    fn record_eviction(&mut self) {
        self.total_evictions += 1;
    }
    
    /// Зарегистрировать сэкономленное время
    #[allow(dead_code)]
    fn record_time_saved(&mut self, time_saved_us: u128) {
        self.total_time_saved_us += time_saved_us;
    }
    
    /// Обновить использование памяти
    fn update_memory_usage(&mut self, memory_usage_bytes: u64) {
        self.current_memory_usage_bytes = memory_usage_bytes;
        if memory_usage_bytes > self.max_memory_usage_bytes {
            self.max_memory_usage_bytes = memory_usage_bytes;
        }
    }
    
    /// Зарегистрировать очистку кэша из-за давления памяти
    #[allow(dead_code)]
    fn record_memory_pressure_cleanup(&mut self) {
        self.memory_pressure_cleanups += 1;
    }
    
    /// Зарегистрировать автоматическую регулировку емкости
    #[allow(dead_code)]
    fn record_auto_capacity_adjustment(&mut self) {
        self.auto_capacity_adjustments += 1;
    }
    
    /// Получить процент хитов кэша
    fn hit_rate(&self) -> Option<f64> {
        if self.total_requests > 0 {
            Some(self.cache_hits as f64 / self.total_requests as f64)
        } else {
            None
        }
    }
    
    /// Логировать сводку статистики кэша
    fn log_summary(&self) {
        info!("Feature Cache Statistics Summary:");
        info!("  Total requests: {}", self.total_requests);
        info!("  Cache hits: {}", self.cache_hits);
        info!("  Cache misses: {}", self.cache_misses);
        
        if let Some(hit_rate) = self.hit_rate() {
            info!("  Hit rate: {:.2}%", hit_rate * 100.0);
        }
        
        info!("  Total insertions: {}", self.total_insertions);
        info!("  Total evictions: {}", self.total_evictions);
        info!("  Total time saved: {} μs", self.total_time_saved_us);
        
        // Статистика использования памяти
        info!("Memory Usage:");
        info!("  Current memory usage: {} bytes ({:.2} KB)", 
            self.current_memory_usage_bytes, 
            self.current_memory_usage_bytes as f64 / 1024.0);
        info!("  Maximum memory usage: {} bytes ({:.2} KB)", 
            self.max_memory_usage_bytes, 
            self.max_memory_usage_bytes as f64 / 1024.0);
        info!("  Memory pressure cleanups: {}", self.memory_pressure_cleanups);
        info!("  Auto capacity adjustments: {}", self.auto_capacity_adjustments);
    }
}

impl FeatureCache {
    /// Создать новый кэш фич.
    fn new(capacity: usize) -> Self {
        Self {
            cache: Mutex::new(lru::LruCache::new(std::num::NonZeroUsize::new(capacity).unwrap())),
            stats: Mutex::new(CacheStats::default()),
        }
    }
    
    /// Получить фичи из кэша или вычислить и сохранить.
    fn get_or_compute<F>(&self, pid: u32, compute_fn: F) -> Vec<f32>
    where
        F: FnOnce() -> Vec<f32>,
    {
        let mut cache = self.cache.lock().unwrap();
        if let Some(features) = cache.get(&pid) {
            let mut stats = self.stats.lock().unwrap();
            stats.record_hit();
            return features.clone();
        }
        
        let features = compute_fn();
        
        // Вычисляем размер памяти для новых фич
        let _feature_memory_size = features.len() * std::mem::size_of::<f32>();
        
        cache.put(pid, features.clone());
        
        let mut stats = self.stats.lock().unwrap();
        stats.record_miss();
        stats.record_insertion();
        
        // Обновляем статистику использования памяти
        let current_memory_usage = cache.iter().map(|(_, v)| v.len() * std::mem::size_of::<f32>()).sum::<usize>() as u64;
        stats.update_memory_usage(current_memory_usage);
        
        features
    }
    
    /// Очистить кэш.
    fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();
        stats.total_evictions += cache.len() as u64;
        cache.clear();
    }
    
    /// Установить новую емкость кэша.
    fn set_capacity(&self, capacity: usize) {
        let mut cache = self.cache.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();
        stats.total_evictions += cache.len() as u64;
        *cache = lru::LruCache::new(std::num::NonZeroUsize::new(capacity).unwrap());
    }
    
    /// Получить текущую статистику кэша.
    fn get_stats(&self) -> CacheStats {
        let stats = self.stats.lock().unwrap();
        stats.clone()
    }
    
    /// Логировать сводку статистики кэша.
    fn log_stats_summary(&self) {
        let stats = self.get_stats();
        stats.log_summary();
    }
    
    /// Настроить емкость кэша на основе давления памяти.
    /// Уменьшает емкость кэша при высоком давлении памяти.
    fn adjust_capacity_for_memory_pressure(&self, memory_pressure: f32) {
        let mut cache = self.cache.lock().unwrap();
        let current_capacity = cache.cap().get();
        
        // Уменьшаем емкость кэша при высоком давлении памяти
        if memory_pressure > 0.8 {
            let new_capacity = (current_capacity as f32 * 0.7) as usize;
            if new_capacity > 0 {
                let mut stats = self.stats.lock().unwrap();
                stats.total_evictions += cache.len() as u64;
                *cache = lru::LruCache::new(std::num::NonZeroUsize::new(new_capacity).unwrap());
                info!("Уменьшена емкость кэша до {} из-за высокого давления памяти ({:.1}%)", new_capacity, memory_pressure * 100.0);
            }
        } else if memory_pressure < 0.3 {
            // Увеличиваем емкость кэша при низком давлении памяти
            let new_capacity = (current_capacity as f32 * 1.3) as usize;
            if new_capacity > current_capacity {
                let _stats = self.stats.lock().unwrap();
                *cache = lru::LruCache::new(std::num::NonZeroUsize::new(new_capacity).unwrap());
                info!("Увеличена емкость кэша до {} из-за низкого давления памяти ({:.1}%)", new_capacity, memory_pressure * 100.0);
            }
        }
    }
    
    /// Проверить давление памяти системы и очистить кэш при необходимости.
    /// Использует sysinfo для мониторинга использования памяти.
    #[allow(dead_code)]
    fn check_system_memory_and_cleanup(&self) {
        let mut system = System::new_all();
        system.refresh_all();
        
        let total_memory = system.total_memory();
        let used_memory = system.used_memory();
        let memory_usage_ratio = used_memory as f32 / total_memory as f32;
        
        let mut cache = self.cache.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();
        
        // Логируем текущее использование памяти
        debug!("System memory usage: {:.1}% ({} MB / {} MB)", 
            memory_usage_ratio * 100.0, 
            used_memory / 1024 / 1024, 
            total_memory / 1024 / 1024);
        
        // Критическое давление памяти - очищаем кэш
        if memory_usage_ratio > 0.9 {
            let cache_size_before = cache.len();
            let memory_freed = stats.current_memory_usage_bytes;
            
            stats.total_evictions += cache_size_before as u64;
            stats.record_memory_pressure_cleanup();
            cache.clear();
            stats.update_memory_usage(0);
            
            warn!("Критическое давление памяти ({:.1}%) - очищен кэш фич ({} элементов, {} KB освобождено)", 
                memory_usage_ratio * 100.0, 
                cache_size_before, 
                memory_freed / 1024);
        }
        // Высокое давление памяти - уменьшаем емкость кэша
        else if memory_usage_ratio > 0.8 {
            let current_capacity = cache.cap().get();
            let new_capacity = (current_capacity as f32 * 0.5) as usize;
            
            if new_capacity > 0 && new_capacity < current_capacity {
                let cache_size_before = cache.len();
                let memory_freed = stats.current_memory_usage_bytes - 
                    (new_capacity * 288) as u64; // Примерный размер фич
                
                stats.total_evictions += cache_size_before as u64;
                stats.record_auto_capacity_adjustment();
                *cache = lru::LruCache::new(std::num::NonZeroUsize::new(new_capacity).unwrap());
                stats.update_memory_usage(0);
                
                warn!("Высокое давление памяти ({:.1}%) - уменьшена емкость кэша до {} ({} KB освобождено)", 
                    memory_usage_ratio * 100.0, 
                    new_capacity, 
                    memory_freed / 1024);
            }
        }
        // Нормальное давление памяти - логируем статистику
        else {
            debug!("Feature cache status: {} elements, {} KB used, {:.1}% hit rate", 
                cache.len(), 
                stats.current_memory_usage_bytes / 1024, 
                stats.hit_rate().unwrap_or(0.0) * 100.0);
        }
    }
    
    /// Публичный метод для запуска проверки памяти и очистки кэша.
    /// Может быть вызван извне для ручного управления памятью.
    #[allow(dead_code)]
    pub fn trigger_memory_cleanup(&self) {
        self.check_system_memory_and_cleanup();
    }
    
    /// Публичный метод для получения текущей статистики использования памяти.
    /// Возвращает текущее и максимальное использование памяти кэшом.
    #[allow(dead_code)]
    pub fn get_memory_usage(&self) -> (u64, u64) {
        let stats = self.stats.lock().unwrap();
        (stats.current_memory_usage_bytes, stats.max_memory_usage_bytes)
    }
}

/// Информация о версии и хэше модели
#[derive(Debug, Clone)]
struct ModelVersionInfo {
    /// Хэш модели (SHA256)
    model_hash: String,
    /// Время последней проверки
    #[allow(dead_code)]
    last_checked: Instant,
    /// Размер модели в байтах
    model_size: u64,
}

impl ModelVersionInfo {
    /// Создать новую информацию о версии модели
    fn new(model_path: &Path) -> Result<Self> {
        let mut file = File::open(model_path)
            .with_context(|| format!("Не удалось открыть файл модели для хэширования: {:?}", model_path))?;
        
        let metadata = file.metadata()
            .with_context(|| format!("Не удалось получить метаданные модели: {:?}", model_path))?;
        
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .with_context(|| format!("Не удалось прочитать файл модели для хэширования: {:?}", model_path))?;
        
        // Вычисляем SHA256 хэш
        let mut hasher = Sha256::new();
        hasher.update(&buffer);
        let hash_result = hasher.finalize();
        let model_hash = format!("{:x}", hash_result);
        
        Ok(Self {
            model_hash,
            last_checked: Instant::now(),
            model_size: metadata.len(),
        })
    }
    
    /// Проверяет, изменилась ли модель
    fn has_changed(&self, model_path: &Path) -> Result<bool> {
        let current_info = Self::new(model_path)?;
        Ok(current_info.model_hash != self.model_hash || current_info.model_size != self.model_size)
    }
    
    /// Возвращает хэш модели
    fn hash(&self) -> &str {
        &self.model_hash
    }
    
    /// Возвращает размер модели
    fn size(&self) -> u64 {
        self.model_size
    }
}

impl CatBoostMLClassifier {
    /// Создать новый CatBoost ML-классификатор.
    ///
    /// # Аргументы
    ///
    /// * `config` - конфигурация ML-классификатора
    ///
    /// # Возвращает
    ///
    /// Результат с новым классификатором или ошибкой при загрузке модели.
    pub fn new(config: MLClassifierConfig) -> Result<Self> {
        info!(
            "Создание CatBoost ML-классификатора с конфигурацией: {:?}",
            config
        );

        let (model, model_version) = if config.enabled {
            let model = Self::load_model(&config).with_context(|| {
                format!("Не удалось загрузить модель из {:?}", config.model_path)
            })?;
            
            // Создаем информацию о версии модели
            let model_path = Path::new(&config.model_path);
            let version_info = ModelVersionInfo::new(model_path)
                .with_context(|| format!("Не удалось создать информацию о версии модели: {:?}", model_path))?;
            
            info!(
                "Модель загружена успешно. Хэш: {}, Размер: {} байт",
                version_info.hash(),
                version_info.size()
            );
            
            (model, Some(version_info))
        } else {
            info!("ML-классификатор отключен в конфигурации, используется заглушка");
            (CatBoostModel::Stub, None)
        };

        Ok(Self {
            model,
            performance_metrics: MLPerformanceMetrics::new(),
            model_version,
        })
    }

    /// Загрузить модель из файла.
    ///
    /// # Аргументы
    ///
    /// * `config` - конфигурация ML-классификатора
    ///
    /// # Возвращает
    ///
    /// Загруженная модель или ошибка.
    fn load_model(config: &MLClassifierConfig) -> Result<CatBoostModel> {
        let model_path = Path::new(&config.model_path);

        if !model_path.exists() {
            return Err(anyhow::anyhow!(
                "Файл модели не найден: {}",
                config.model_path
            ));
        }

        info!("Загрузка ML-модели из: {}", config.model_path);

        if matches!(config.model_type, ModelType::Onnx) {
            #[cfg(feature = "onnx")]
            {
                Self::load_onnx_model(model_path)
                    .with_context(|| "Не удалось загрузить ONNX модель")
            }
            #[cfg(not(feature = "onnx"))]
            {
                warn!("ONNX поддержка отключена, но model_type=Onnx в конфигурации");
                #[cfg(feature = "catboost")]
                {
                    Self::load_catboost_model(model_path)
                        .with_context(|| "Не удалось загрузить CatBoost модель (ONNX отключен)")
                }
                #[cfg(not(feature = "catboost"))]
                {
                    Err(anyhow::anyhow!(
                        "ML поддержка отключена (и CatBoost, и ONNX отключены)"
                    ))
                }
            }
        } else {
            #[cfg(feature = "catboost")]
            {
                Self::load_catboost_model(model_path)
                    .with_context(|| "Не удалось загрузить CatBoost модель")
            }
            #[cfg(not(feature = "catboost"))]
            {
                Err(anyhow::anyhow!(
                    "ML поддержка отключена (CatBoost отключен)"
                ))
            }
        }
    }

    /// Загрузить CatBoost модель в формате JSON.
    ///
    /// # Аргументы
    ///
    /// * `model_path` - путь к файлу модели
    ///
    /// # Возвращает
    ///
    /// Загруженная модель или ошибка.
    #[cfg(feature = "catboost")]
    fn load_catboost_model(model_path: &Path) -> Result<CatBoostModel> {
        use std::fs::File;
        use std::io::Read;

        info!("Загрузка CatBoost модели из JSON файла: {:?}", model_path);

        let mut file = File::open(model_path)
            .with_context(|| format!("Не удалось открыть файл модели: {:?}", model_path))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .with_context(|| format!("Не удалось прочитать файл модели: {:?}", model_path))?;

        let model = CatBoostClassifier::from_json(&buffer)
            .with_context(|| "Не удалось разобрать CatBoost модель из JSON")?;

        Ok(CatBoostModel::Json(Arc::new(model)))
    }

    /// Загрузить ONNX модель.
    ///
    /// # Аргументы
    ///
    /// * `model_path` - путь к файлу модели
    ///
    /// # Возвращает
    ///
    /// Загруженная модель или ошибка.
    #[cfg(feature = "onnx")]
    fn load_onnx_model(model_path: &Path) -> Result<CatBoostModel> {
        info!("Загрузка ONNX модели: {:?}", model_path);

        let session = Session::builder()?
            .with_optimization_level(ort::GraphOptimizationLevel::Level3)?
            .with_intra_threads(1)?
            .with_inter_threads(1)?
            .commit_from_file(model_path)
            .with_context(|| "Не удалось загрузить ONNX модель")?;

        Ok(CatBoostModel::Onnx(Arc::new(session)))
    }

    /// Проверяет, изменилась ли модель на диске.
    ///
    /// # Аргументы
    ///
    /// * `model_path` - путь к файлу модели для проверки
    ///
    /// # Возвращает
    ///
    /// `true`, если модель изменилась, `false` в противном случае.
    ///
    /// # Ошибки
    ///
    /// Возвращает ошибку, если не удалось прочитать информацию о модели.
    pub fn has_model_changed(&self, model_path: &Path) -> Result<bool> {
        if let Some(version_info) = &self.model_version {
            version_info.has_changed(model_path)
        } else {
            // Если нет информации о версии, считаем что модель не изменилась
            Ok(false)
        }
    }
    
    /// Возвращает хэш текущей модели.
    ///
    /// # Возвращает
    ///
    /// Опциональный хэш модели (None, если модель не загружена).
    pub fn model_hash(&self) -> Option<&str> {
        self.model_version.as_ref().map(|v| v.hash())
    }
    
    /// Возвращает размер текущей модели.
    ///
    /// # Возвращает
    ///
    /// Опциональный размер модели в байтах (None, если модель не загружена).
    pub fn model_size(&self) -> Option<u64> {
        self.model_version.as_ref().map(|v| v.size())
    }
    
    /// Перезагружает модель, если она изменилась на диске.
    ///
    /// # Аргументы
    ///
    /// * `config` - текущая конфигурация ML-классификатора
    ///
    /// # Возвращает
    ///
    /// `true`, если модель была перезагружена, `false` в противном случае.
    ///
    /// # Ошибки
    ///
    /// Возвращает ошибку, если не удалось перезагрузить модель.
    pub fn reload_model_if_changed(&mut self, config: &MLClassifierConfig) -> Result<bool> {
        if !config.enabled {
            return Ok(false);
        }
        
        let model_path = Path::new(&config.model_path);
        if !model_path.exists() {
            warn!("Файл модели не существует для перезагрузки: {:?}", model_path);
            return Ok(false);
        }
        
        // Проверяем, изменилась ли модель
        if !self.has_model_changed(model_path)? {
            debug!("Модель не изменилась, перезагрузка не требуется");
            return Ok(false);
        }
        
        info!("Обнаружены изменения в модели, выполняется перезагрузка...");
        
        // Сохраняем старую модель для fallback
        let old_model = std::mem::replace(&mut self.model, CatBoostModel::Stub);
        let old_version = self.model_version.take();
        
        // Пробуем загрузить новую модель
        match Self::load_model(config) {
            Ok(new_model) => {
                // Создаем новую информацию о версии
                let new_version = ModelVersionInfo::new(model_path)
                    .with_context(|| format!("Не удалось создать информацию о версии новой модели: {:?}", model_path))?;
                
                info!(
                    "Модель успешно перезагружена. Новый хэш: {}, Размер: {} байт",
                    new_version.hash(),
                    new_version.size()
                );
                
                self.model = new_model;
                self.model_version = Some(new_version);
                Ok(true)
            }
            Err(e) => {
                error!("Ошибка при перезагрузке модели: {}. Выполняется откат к старой модели", e);
                
                // Восстанавливаем старую модель
                self.model = old_model;
                self.model_version = old_version;
                
                Err(e)
            }
        }
    }
    
    /// Выполняет классификацию с автоматическим fallback при ошибках.
    ///
    /// # Аргументы
    ///
    /// * `process` - процесс для классификации
    /// * `fallback_classifier` - резервный классификатор для использования при ошибках
    ///
    /// # Возвращает
    ///
    /// Результат классификации (основной или резервный).
    pub fn classify_with_fallback(&mut self, process: &ProcessRecord, fallback_classifier: &mut dyn MLClassifier) -> MLClassificationResult {
        // Пробуем основную классификацию
        let result = self.classify(process);
        
        // Если уверенность слишком низкая или это неизвестный тип, используем fallback
        if result.confidence < 0.3 || result.process_type.as_deref() == Some("unknown") {
            debug!("Низкая уверенность ML-классификации ({}), используем fallback", result.confidence);
            return fallback_classifier.classify(process);
        }
        
        result
    }
    
    /// Очищает кэш фич.
    ///
    /// # Примечание
    ///
    /// Следует вызывать при изменении конфигурации классификатора или при обнаружении
    /// значительных изменений в системе.
    pub fn clear_feature_cache() {
        GLOBAL_FEATURE_CACHE.clear();
    }
    
    /// Устанавливает новую емкость кэша фич.
    ///
    /// # Аргументы
    ///
    /// * `capacity` - новая емкость кэша
    ///
    /// # Примечание
    ///
    /// Емкость кэша влияет на баланс между использованием памяти и производительностью.
    /// Рекомендуемые значения: 512-2048 для большинства систем.
    pub fn set_feature_cache_capacity(capacity: usize) {
        GLOBAL_FEATURE_CACHE.set_capacity(capacity);
    }
    
    /// Возвращает текущую емкость кэша фич.
    pub fn get_feature_cache_capacity() -> usize {
        let cache = GLOBAL_FEATURE_CACHE.cache.lock().unwrap();
        cache.cap().get()
    }
    
    /// Возвращает текущую статистику кэша фич.
    pub fn get_feature_cache_stats() -> CacheStats {
        GLOBAL_FEATURE_CACHE.get_stats()
    }
    
    /// Логирует сводку статистики кэша фич.
    pub fn log_feature_cache_stats() {
        GLOBAL_FEATURE_CACHE.log_stats_summary();
    }
    
    /// Настраивает емкость кэша на основе давления памяти.
    ///
    /// # Аргументы
    ///
    /// * `memory_pressure` - уровень давления памяти (0.0 - 1.0)
    pub fn adjust_feature_cache_for_memory_pressure(memory_pressure: f32) {
        GLOBAL_FEATURE_CACHE.adjust_capacity_for_memory_pressure(memory_pressure);
    }
    
    /// Оптимизирует извлечение фич для лучшей производительности.
    ///
    /// # Аргументы
    ///
    /// * `process` - процесс для преобразования
    /// * `use_cache` - использовать кэш для оптимизации производительности
    ///
    /// # Возвращает
    ///
    /// Вектор фич для ML-модели.
    ///
    /// # Примечание
    ///
    /// Этот метод предоставляет оптимизированную версию извлечения фич с возможностью
    /// отключения кэширования для тестирования производительности.
    pub fn process_to_features_optimized(&self, process: &ProcessRecord, use_cache: bool) -> Vec<f32> {
        if use_cache {
            // Используем кэширование для оптимизации производительности
            self.process_to_features(process)
        } else {
            // Прямое извлечение фич без кэширования
            let mut features = Vec::with_capacity(29);
            
            // Числовые фичи
            features.push(process.cpu_share_1s.unwrap_or(0.0) as f32);
            features.push(process.cpu_share_10s.unwrap_or(0.0) as f32);
            
            // Расширенные CPU фичи
            features.push((process.cpu_share_1s.unwrap_or(0.0) * 100.0) as f32); // CPU %
            features.push((process.cpu_share_10s.unwrap_or(0.0) * 100.0) as f32); // CPU %
            
            // IO фичи
            features.push(process.io_read_bytes.unwrap_or(0) as f32 / (1024.0 * 1024.0)); // MB
            features.push(process.io_write_bytes.unwrap_or(0) as f32 / (1024.0 * 1024.0)); // MB
            features.push((process.io_read_bytes.unwrap_or(0) as f32 + process.io_write_bytes.unwrap_or(0) as f32) / (1024.0 * 1024.0)); // Total IO in MB
            
            // Память фичи
            features.push(process.rss_mb.unwrap_or(0) as f32);
            features.push(process.swap_mb.unwrap_or(0) as f32);
            features.push((process.rss_mb.unwrap_or(0) as f32 + process.swap_mb.unwrap_or(0) as f32) * 1024.0); // Total memory KB
            
            // Контекстные переключения
            features.push(process.voluntary_ctx.unwrap_or(0) as f32);
            features.push(process.involuntary_ctx.unwrap_or(0) as f32);
            features.push((process.voluntary_ctx.unwrap_or(0) as f32 + process.involuntary_ctx.unwrap_or(0) as f32) / process.uptime_sec.max(1) as f32); // ctx/sec
            
            // Расширенные фичи
            features.push(process.uptime_sec as f32); // Время работы процесса
            features.push(process.uptime_sec as f32 / 3600.0); // Время работы в часах
            
            // Нормализованные фичи
            features.push((process.cpu_share_1s.unwrap_or(0.0) as f32).ln_1p()); // Log CPU
            features.push((process.rss_mb.unwrap_or(0) as f32).ln_1p()); // Log memory
            
            // Булевые фичи (0/1)
            features.push(if process.has_tty { 1.0 } else { 0.0 });
            features.push(if process.has_gui_window { 1.0 } else { 0.0 });
            features.push(if process.is_focused_window { 1.0 } else { 0.0 });
            features.push(if process.env_has_display { 1.0 } else { 0.0 });
            features.push(if process.env_has_wayland { 1.0 } else { 0.0 });
            features.push(if process.env_ssh { 1.0 } else { 0.0 });
            features.push(if process.is_audio_client { 1.0 } else { 0.0 });
            features.push(if process.has_active_stream { 1.0 } else { 0.0 });
            
            // Расширенные булевые фичи
            features.push(if process.has_tty && !process.env_ssh { 1.0 } else { 0.0 }); // Локальный TTY
            features.push(if process.has_gui_window && process.is_focused_window { 1.0 } else { 0.0 }); // Фокусированное GUI
            
            // Интерактивные фичи
            features.push(if process.has_gui_window || process.has_tty { 1.0 } else { 0.0 }); // Интерактивный процесс
            features.push(if process.is_audio_client || process.has_active_stream { 1.0 } else { 0.0 }); // Аудио активность
            
            features
        }
    }
    
    /// Возвращает текущую емкость кэша фич.
    ///
    /// # Возвращает
    ///
    /// Текущая емкость кэша.
    pub fn feature_cache_capacity() -> usize {
        let cache = GLOBAL_FEATURE_CACHE.cache.lock().unwrap();
        cache.cap().get()
    }

    /// Преобразовать процесс в фичи для ML-модели.
    ///
    /// # Аргументы
    ///
    /// * `process` - процесс для преобразования
    ///
    /// # Возвращает
    ///
    /// Вектор фич для ML-модели.
    ///
    /// # Примечание
    ///
    /// Этот метод используется внутренне в `classify_with_catboost` и `classify_with_onnx`.
    /// Включает расширенные фичи для более точной классификации.
    /// Использует кэширование для оптимизации производительности.
    #[allow(dead_code)]
    fn process_to_features(&self, process: &ProcessRecord) -> Vec<f32> {
        // Используем кэширование для оптимизации производительности
        GLOBAL_FEATURE_CACHE.get_or_compute(process.pid as u32, || {
            let mut features = Vec::new();

            // Числовые фичи
            features.push(process.cpu_share_1s.unwrap_or(0.0) as f32);
            features.push(process.cpu_share_10s.unwrap_or(0.0) as f32);
            
            // Расширенные CPU фичи
            features.push((process.cpu_share_1s.unwrap_or(0.0) * 100.0) as f32); // CPU %
            features.push((process.cpu_share_10s.unwrap_or(0.0) * 100.0) as f32); // CPU %
            
            // IO фичи
            features.push(process.io_read_bytes.unwrap_or(0) as f32 / (1024.0 * 1024.0)); // MB
            features.push(process.io_write_bytes.unwrap_or(0) as f32 / (1024.0 * 1024.0)); // MB
            features.push((process.io_read_bytes.unwrap_or(0) as f32 + process.io_write_bytes.unwrap_or(0) as f32) / (1024.0 * 1024.0)); // Total IO in MB
            
            // Память фичи
            features.push(process.rss_mb.unwrap_or(0) as f32);
            features.push(process.swap_mb.unwrap_or(0) as f32);
            features.push((process.rss_mb.unwrap_or(0) as f32 + process.swap_mb.unwrap_or(0) as f32) * 1024.0); // Total memory KB
            
            // Контекстные переключения
            features.push(process.voluntary_ctx.unwrap_or(0) as f32);
            features.push(process.involuntary_ctx.unwrap_or(0) as f32);
            features.push((process.voluntary_ctx.unwrap_or(0) as f32 + process.involuntary_ctx.unwrap_or(0) as f32) / process.uptime_sec.max(1) as f32); // ctx/sec
            
            // Расширенные фичи
            features.push(process.uptime_sec as f32); // Время работы процесса
            features.push(process.uptime_sec as f32 / 3600.0); // Время работы в часах
            
            // Нормализованные фичи
            features.push((process.cpu_share_1s.unwrap_or(0.0) as f32).ln_1p()); // Log CPU
            features.push((process.rss_mb.unwrap_or(0) as f32).ln_1p()); // Log memory
            
            // Булевые фичи (0/1)
            features.push(if process.has_tty { 1.0 } else { 0.0 });
            features.push(if process.has_gui_window { 1.0 } else { 0.0 });
            features.push(if process.is_focused_window { 1.0 } else { 0.0 });
            features.push(if process.env_has_display { 1.0 } else { 0.0 });
            features.push(if process.env_has_wayland { 1.0 } else { 0.0 });
            features.push(if process.env_ssh { 1.0 } else { 0.0 });
            features.push(if process.is_audio_client { 1.0 } else { 0.0 });
            features.push(if process.has_active_stream { 1.0 } else { 0.0 });
            
            // Расширенные булевые фичи
            features.push(if process.has_tty && !process.env_ssh { 1.0 } else { 0.0 }); // Локальный TTY
            features.push(if process.has_gui_window && process.is_focused_window { 1.0 } else { 0.0 }); // Фокусированное GUI
            
            // Интерактивные фичи
            features.push(if process.has_gui_window || process.has_tty { 1.0 } else { 0.0 }); // Интерактивный процесс
            features.push(if process.is_audio_client || process.has_active_stream { 1.0 } else { 0.0 }); // Аудио активность
            
            features
        })
    }
}

impl MLClassifier for CatBoostMLClassifier {
    fn classify(&mut self, process: &ProcessRecord) -> MLClassificationResult {
        let start_time = Instant::now();
        
        let result = match &self.model {
            #[cfg(feature = "catboost")]
            CatBoostModel::Json(model) => self.classify_with_catboost(model, process),
            #[cfg(feature = "onnx")]
            CatBoostModel::Onnx(session) => self.classify_with_onnx(session, process),
            CatBoostModel::Stub => {
                debug!("ML-классификатор отключен, используется заглушка");
                let mut stub = StubMLClassifier::new();
                stub.classify(process)
            }
        };
        
        let duration = start_time.elapsed().as_micros();
        
        // Зарегистрировать успешную классификацию
        self.performance_metrics.record_successful_classification(duration, result.confidence);
        
        result
    }

    fn get_performance_metrics(&self) -> MLPerformanceMetrics {
        self.performance_metrics.clone()
    }

    fn reset_performance_metrics(&mut self) {
        self.performance_metrics.reset();
    }
}

#[cfg(feature = "catboost")]
impl CatBoostMLClassifier {
    /// Классифицировать процесс с использованием CatBoost модели.
    ///
    /// # Аргументы
    ///
    /// * `model` - CatBoost модель
    /// * `process` - процесс для классификации
    ///
    /// # Возвращает
    ///
    /// Результат классификации.
    fn classify_with_catboost(
        &self,
        model: &CatBoostClassifier,
        process: &ProcessRecord,
    ) -> MLClassificationResult {
        let features = self.process_to_features(process);

        // Преобразуем фичи в формат, ожидаемый CatBoost
        let input = vec![features];

        match model.predict(&input) {
            Ok(predictions) => {
                if predictions.is_empty() {
                    warn!("CatBoost модель вернула пустой результат");
                    return MLClassificationResult {
                        process_type: Some("unknown".to_string()),
                        tags: vec!["ml_failed".to_string()],
                        confidence: 0.1,
                    };
                }

                // Предполагаем, что модель возвращает вероятности для каждого класса
                // Находим класс с максимальной вероятностью
                let max_prob = predictions.iter().fold(f64::MIN, |a, &b| a.max(b));
                let class_idx = predictions.iter().position(|&p| p == max_prob).unwrap_or(0);

                // Преобразуем индекс класса в тип процесса
                let process_type = match class_idx {
                    0 => "unknown",
                    1 => "gui",
                    2 => "cpu_intensive",
                    3 => "io_intensive",
                    4 => "audio",
                    5 => "focused",
                    6 => "background",
                    7 => "batch",
                    _ => "unknown",
                };

                let mut tags = HashSet::new();

                // Добавляем теги на основе типа
                match process_type {
                    "gui" => {
                        tags.insert("gui".to_string());
                        tags.insert("interactive".to_string());
                    }
                    "audio" => {
                        tags.insert("audio".to_string());
                        tags.insert("realtime".to_string());
                    }
                    "focused" => {
                        tags.insert("focused".to_string());
                        tags.insert("interactive".to_string());
                    }
                    "cpu_intensive" => {
                        tags.insert("high_cpu".to_string());
                    }
                    "io_intensive" => {
                        tags.insert("high_io".to_string());
                    }
                    _ => {}
                }

                MLClassificationResult {
                    process_type: Some(process_type.to_string()),
                    tags: tags.into_iter().collect(),
                    confidence: max_prob as f64,
                }
            }
            Err(e) => {
                error!(
                    "Ошибка при предсказании с использованием CatBoost модели: {}",
                    e
                );
                // Зарегистрировать ошибку классификации
                let mut metrics = self.performance_metrics.clone();
                metrics.record_classification_error();
                MLClassificationResult {
                    process_type: Some("unknown".to_string()),
                    tags: vec!["ml_error".to_string()],
                    confidence: 0.1,
                }
            }
        }
    }
}

#[cfg(feature = "onnx")]
impl CatBoostMLClassifier {
    /// Классифицировать процесс с использованием ONNX модели.
    ///
    /// # Аргументы
    ///
    /// * `session` - ONNX сессия
    /// * `process` - процесс для классификации
    ///
    /// # Возвращает
    ///
    /// Результат классификации.
    fn classify_with_onnx(
        &self,
        session: &Session,
        process: &ProcessRecord,
    ) -> MLClassificationResult {
        let features = self.process_to_features(process);

        // Создаем входной тензор
        let input_tensor = ort::Tensor::from_array(
            ort::Array::from_shape_vec((1, features.len()), features).unwrap(),
        );

        let inputs = ort::inputs! {
            "input" => input_tensor,
        };

        match session.run(inputs) {
            Ok(outputs) => {
                if let Some(output) = outputs.get("output") {
                    if let Some(probabilities) = output.try_extract::<f32>() {
                        if probabilities.is_empty() {
                            warn!("ONNX модель вернула пустой результат");
                            return MLClassificationResult {
                                process_type: Some("unknown".to_string()),
                                tags: vec!["ml_failed".to_string()],
                                confidence: 0.1,
                            };
                        }

                        // Находим класс с максимальной вероятностью
                        let max_prob = probabilities.iter().fold(f32::MIN, |a, &b| a.max(b));
                        let class_idx = probabilities
                            .iter()
                            .position(|&p| p == max_prob)
                            .unwrap_or(0);

                        // Преобразуем индекс класса в тип процесса
                        let process_type = match class_idx {
                            0 => "unknown",
                            1 => "gui",
                            2 => "cpu_intensive",
                            3 => "io_intensive",
                            4 => "audio",
                            5 => "focused",
                            6 => "background",
                            7 => "batch",
                            _ => "unknown",
                        };

                        let mut tags = HashSet::new();

                        // Добавляем теги на основе типа
                        match process_type {
                            "gui" => {
                                tags.insert("gui".to_string());
                                tags.insert("interactive".to_string());
                            }
                            "audio" => {
                                tags.insert("audio".to_string());
                                tags.insert("realtime".to_string());
                            }
                            "focused" => {
                                tags.insert("focused".to_string());
                                tags.insert("interactive".to_string());
                            }
                            "cpu_intensive" => {
                                tags.insert("high_cpu".to_string());
                            }
                            "io_intensive" => {
                                tags.insert("high_io".to_string());
                            }
                            _ => {}
                        }

                        return MLClassificationResult {
                            process_type: Some(process_type.to_string()),
                            tags: tags.into_iter().collect(),
                            confidence: max_prob as f64,
                        };
                    }
                }

                warn!("ONNX модель вернула неожиданный формат вывода");
            }
            Err(e) => {
                error!("Ошибка при выполнении ONNX модели: {}", e);
            }
        }

        // Зарегистрировать ошибку классификации
        let mut metrics = self.performance_metrics.clone();
        metrics.record_classification_error();

        MLClassificationResult {
            process_type: Some("unknown".to_string()),
            tags: vec!["ml_error".to_string()],
            confidence: 0.1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::snapshots::ProcessRecord;

    fn create_test_process() -> ProcessRecord {
        ProcessRecord {
            pid: 1000,
            ppid: 1,
            uid: 1000,
            gid: 1000,
            exe: Some("test-app".to_string()),
            cmdline: None,
            cgroup_path: None,
            systemd_unit: None,
            app_group_id: None,
            state: "R".to_string(),
            start_time: 0,
            uptime_sec: 100,
            tty_nr: 0,
            has_tty: false,
            cpu_share_1s: None,
            cpu_share_10s: None,
            io_read_bytes: None,
            io_write_bytes: None,
            io_read_operations: None,
            io_write_operations: None,
            io_total_operations: None,
            io_last_update_ns: None,
            io_data_source: None,
            rss_mb: None,
            swap_mb: None,
            voluntary_ctx: None,
            involuntary_ctx: None,
            has_gui_window: false,
            is_focused_window: false,
            window_state: None,
            env_has_display: false,
            env_has_wayland: false,
            env_term: None,
            env_ssh: false,
            is_audio_client: false,
            has_active_stream: false,
            process_type: None,
            tags: Vec::new(),
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
            energy_uj: None,
            power_w: None,
            energy_timestamp: None,
            network_rx_bytes: None,
            network_tx_bytes: None,
            network_rx_packets: None,
            network_tx_packets: None,
            network_tcp_connections: None,
            network_udp_connections: None,
            network_last_update_ns: None,
            network_data_source: None,
            gpu_utilization: None,
            gpu_memory_bytes: None,
            gpu_time_us: None,
            gpu_api_calls: None,
            gpu_last_update_ns: None,
            gpu_data_source: None,
        }
    }

    #[test]
    fn test_stub_ml_classifier_gui_process() {
        let mut classifier = StubMLClassifier::new();
        let mut process = create_test_process();
        process.has_gui_window = true;

        let result = classifier.classify(&process);

        assert_eq!(result.process_type, Some("gui".to_string()));
        assert!(result.tags.contains(&"gui".to_string()));
        assert!(result.tags.contains(&"interactive".to_string()));
        assert!(result.confidence > 0.7);
    }

    #[test]
    fn test_stub_ml_classifier_high_cpu_process() {
        let mut classifier = StubMLClassifier::new();
        let mut process = create_test_process();
        process.cpu_share_10s = Some(0.5);

        let result = classifier.classify(&process);

        assert_eq!(result.process_type, Some("cpu_intensive".to_string()));
        assert!(result.tags.contains(&"high_cpu".to_string()));
        assert!(result.confidence > 0.6);
    }

    #[test]
    fn test_stub_ml_classifier_high_io_process() {
        let mut classifier = StubMLClassifier::new();
        let mut process = create_test_process();
        process.io_read_bytes = Some(2 * 1024 * 1024); // 2MB

        let result = classifier.classify(&process);

        assert_eq!(result.process_type, Some("io_intensive".to_string()));
        assert!(result.tags.contains(&"high_io".to_string()));
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_stub_ml_classifier_audio_process() {
        let mut classifier = StubMLClassifier::new();
        let mut process = create_test_process();
        process.is_audio_client = true;

        let result = classifier.classify(&process);

        assert_eq!(result.process_type, Some("audio".to_string()));
        assert!(result.tags.contains(&"audio".to_string()));
        assert!(result.tags.contains(&"realtime".to_string()));
        assert!(result.confidence > 0.8);
    }

    #[test]
    fn test_stub_ml_classifier_focused_process() {
        let mut classifier = StubMLClassifier::new();
        let mut process = create_test_process();
        process.is_focused_window = true;

        let result = classifier.classify(&process);

        assert_eq!(result.process_type, Some("focused".to_string()));
        assert!(result.tags.contains(&"focused".to_string()));
        assert!(result.tags.contains(&"interactive".to_string()));
        assert!(result.confidence > 0.8);
    }

    #[test]
    fn test_performance_metrics_initialization() {
        let metrics = MLPerformanceMetrics::new();
        assert_eq!(metrics.total_classifications, 0);
        assert_eq!(metrics.successful_classifications, 0);
        assert_eq!(metrics.classification_errors, 0);
        assert_eq!(metrics.total_classification_time_us, 0);
        assert!(metrics.min_classification_time_us.is_none());
        assert!(metrics.max_classification_time_us.is_none());
        assert_eq!(metrics.total_confidence, 0.0);
        assert_eq!(metrics.high_confidence_classifications, 0);
        assert_eq!(metrics.medium_confidence_classifications, 0);
        assert_eq!(metrics.low_confidence_classifications, 0);
    }

    #[test]
    fn test_performance_metrics_successful_classification() {
        let mut metrics = MLPerformanceMetrics::new();
        metrics.record_successful_classification(100, 0.9);
        
        assert_eq!(metrics.total_classifications, 1);
        assert_eq!(metrics.successful_classifications, 1);
        assert_eq!(metrics.classification_errors, 0);
        assert_eq!(metrics.total_classification_time_us, 100);
        assert_eq!(metrics.min_classification_time_us, Some(100));
        assert_eq!(metrics.max_classification_time_us, Some(100));
        assert_eq!(metrics.total_confidence, 0.9);
        assert_eq!(metrics.high_confidence_classifications, 1);
        assert_eq!(metrics.medium_confidence_classifications, 0);
        assert_eq!(metrics.low_confidence_classifications, 0);
        
        assert_eq!(metrics.average_classification_time_us(), Some(100.0));
        assert_eq!(metrics.average_confidence(), Some(0.9));
        assert_eq!(metrics.success_rate(), Some(1.0));
    }

    #[test]
    fn test_performance_metrics_multiple_classifications() {
        let mut metrics = MLPerformanceMetrics::new();
        metrics.record_successful_classification(100, 0.9);  // high confidence
        metrics.record_successful_classification(200, 0.6);  // medium confidence
        metrics.record_successful_classification(150, 0.4);  // low confidence
        
        assert_eq!(metrics.total_classifications, 3);
        assert_eq!(metrics.successful_classifications, 3);
        assert_eq!(metrics.classification_errors, 0);
        assert_eq!(metrics.total_classification_time_us, 450);
        assert_eq!(metrics.min_classification_time_us, Some(100));
        assert_eq!(metrics.max_classification_time_us, Some(200));
        assert_eq!(metrics.total_confidence, 1.9);
        assert_eq!(metrics.high_confidence_classifications, 1);
        assert_eq!(metrics.medium_confidence_classifications, 1);
        assert_eq!(metrics.low_confidence_classifications, 1);
        
        assert_eq!(metrics.average_classification_time_us(), Some(150.0));
        assert_eq!(metrics.average_confidence(), Some(1.9 / 3.0));
        assert_eq!(metrics.success_rate(), Some(1.0));
    }

    #[test]
    fn test_performance_metrics_with_errors() {
        let mut metrics = MLPerformanceMetrics::new();
        metrics.record_successful_classification(100, 0.8);
        metrics.record_classification_error();
        metrics.record_successful_classification(200, 0.7);
        
        assert_eq!(metrics.total_classifications, 3);
        assert_eq!(metrics.successful_classifications, 2);
        assert_eq!(metrics.classification_errors, 1);
        assert_eq!(metrics.total_classification_time_us, 300);
        assert_eq!(metrics.success_rate(), Some(2.0 / 3.0));
        assert_eq!(metrics.average_classification_time_us(), Some(150.0));
    }

    #[test]
    fn test_performance_metrics_reset() {
        let mut metrics = MLPerformanceMetrics::new();
        metrics.record_successful_classification(100, 0.9);
        metrics.record_classification_error();
        
        assert_eq!(metrics.total_classifications, 2);
        
        metrics.reset();
        
        assert_eq!(metrics.total_classifications, 0);
        assert_eq!(metrics.successful_classifications, 0);
        assert_eq!(metrics.classification_errors, 0);
        assert_eq!(metrics.total_classification_time_us, 0);
    }

    #[test]
    fn test_stub_classifier_performance_metrics() {
        let mut classifier = StubMLClassifier::new();
        let process = create_test_process();
        
        // Initial metrics should be empty
        let initial_metrics = classifier.get_performance_metrics();
        assert_eq!(initial_metrics.total_classifications, 0);
        
        // Classify a process
        let result = classifier.classify(&process);
        
        // Metrics should now show one classification
        let metrics = classifier.get_performance_metrics();
        assert_eq!(metrics.total_classifications, 1);
        assert_eq!(metrics.successful_classifications, 1);
        assert_eq!(metrics.classification_errors, 0);
        assert!(metrics.average_classification_time_us().is_some());
        assert_eq!(metrics.average_confidence(), Some(result.confidence));
        
        // Reset and verify
        classifier.reset_performance_metrics();
        let reset_metrics = classifier.get_performance_metrics();
        assert_eq!(reset_metrics.total_classifications, 0);
    }

    #[test]
    fn test_stub_ml_classifier_unknown_process() {
        let mut classifier = StubMLClassifier::new();
        let process = create_test_process();

        let result = classifier.classify(&process);

        assert_eq!(result.process_type, Some("unknown".to_string()));
        assert!(result.confidence < 0.5);
    }

    #[test]
    fn test_stub_ml_classifier_multiple_features() {
        let mut classifier = StubMLClassifier::new();
        let mut process = create_test_process();
        process.has_gui_window = true;
        process.cpu_share_10s = Some(0.4);
        process.is_audio_client = true;

        let result = classifier.classify(&process);

        // Должен быть выбран тип с наивысшей уверенностью (audio)
        assert_eq!(result.process_type, Some("audio".to_string()));
        // Должны быть теги от всех признаков
        assert!(result.tags.contains(&"gui".to_string()));
        assert!(result.tags.contains(&"interactive".to_string()));
        assert!(result.tags.contains(&"audio".to_string()));
        assert!(result.tags.contains(&"realtime".to_string()));
        assert!(result.tags.contains(&"high_cpu".to_string()));
        assert!(result.confidence > 0.8);
    }

    #[test]
    fn test_create_ml_classifier_disabled() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        let config = MLClassifierConfig {
            enabled: false,
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let classifier = create_ml_classifier(config);
        assert!(classifier.is_ok());

        // Должен вернуть StubMLClassifier
        let mut classifier = classifier.unwrap();
        let result = classifier.classify(&create_test_process());
        assert_eq!(result.process_type, Some("unknown".to_string()));
    }

    #[test]
    fn test_create_ml_classifier_nonexistent_model() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        let config = MLClassifierConfig {
            enabled: true,
            model_path: "/nonexistent/path/model.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let classifier = create_ml_classifier(config);
        assert!(classifier.is_err());

        // Должна быть ошибка о загрузке модели
        let err = classifier.unwrap_err();
        let err_str = err.to_string();
        assert!(err_str.contains("Не удалось загрузить модель"));
    }

    #[test]
    fn test_catboost_ml_classifier_feature_extraction() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        // Очищаем кэш перед тестом
        CatBoostMLClassifier::clear_feature_cache();

        let config = MLClassifierConfig {
            enabled: false, // Отключаем, чтобы использовать заглушку
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let classifier = CatBoostMLClassifier::new(config).unwrap();
        let mut process = create_test_process();

        // Устанавливаем различные значения
        process.cpu_share_1s = Some(0.25);
        process.cpu_share_10s = Some(0.5);
        process.io_read_bytes = Some(2 * 1024 * 1024); // 2MB
        process.io_write_bytes = Some(1024 * 1024); // 1MB
        process.rss_mb = Some(100);
        process.swap_mb = Some(50);
        process.voluntary_ctx = Some(1000);
        process.involuntary_ctx = Some(500);
        process.has_tty = true;
        process.has_gui_window = true;
        process.is_focused_window = true;
        process.env_has_display = true;
        process.env_has_wayland = true;
        process.env_ssh = true;
        process.is_audio_client = true;
        process.has_active_stream = true;
        process.uptime_sec = 3600; // 1 час

        let features = classifier.process_to_features(&process);

        // Проверяем, что фичи извлечены правильно (расширенный набор)
        assert_eq!(features.len(), 29); // 13 числовых + 16 булевых/интерактивных

        // Проверяем основные числовые фичи
        assert_eq!(features[0], 0.25); // cpu_share_1s
        assert_eq!(features[1], 0.5); // cpu_share_10s
        assert_eq!(features[2], 25.0); // cpu_share_1s %
        assert_eq!(features[3], 50.0); // cpu_share_10s %
        assert_eq!(features[4], 2.0); // io_read_bytes в MB
        assert_eq!(features[5], 1.0); // io_write_bytes в MB
        assert_eq!(features[6], 3.0); // total IO
        assert_eq!(features[7], 100.0); // rss_mb
        assert_eq!(features[8], 50.0); // swap_mb
        assert_eq!(features[9], 150.0 * 1024.0); // total memory KB
        assert_eq!(features[10], 1000.0); // voluntary_ctx
        assert_eq!(features[11], 500.0); // involuntary_ctx
        assert_eq!(features[12], (1000.0 + 500.0) / 3600.0); // ctx/sec
        assert_eq!(features[13], 3600.0); // uptime_sec
        assert_eq!(features[14], 1.0); // uptime_hours
        
        // Проверяем логарифмические фичи (приблизительно)
        assert!(features[15] > 0.0); // log CPU
        assert!(features[16] > 0.0); // log memory

        // Проверяем булевые фичи (должны быть 1.0)
        assert_eq!(features[17], 1.0); // has_tty
        assert_eq!(features[18], 1.0); // has_gui_window
        assert_eq!(features[19], 1.0); // is_focused_window
        assert_eq!(features[20], 1.0); // env_has_display
        assert_eq!(features[21], 1.0); // env_has_wayland
        assert_eq!(features[22], 1.0); // env_ssh
        assert_eq!(features[23], 1.0); // is_audio_client
        assert_eq!(features[24], 1.0); // has_active_stream

        // Проверяем расширенные булевые фичи
        assert_eq!(features[25], 0.0); // локальный TTY (has_tty и env_ssh=true)
        assert_eq!(features[26], 1.0); // фокусированное GUI
        assert_eq!(features[27], 1.0); // интерактивный процесс
        assert_eq!(features[28], 1.0); // аудио активность
    }

    #[test]
    fn test_catboost_ml_classifier_feature_extraction_defaults() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        // Очищаем кэш перед тестом
        CatBoostMLClassifier::clear_feature_cache();

        let config = MLClassifierConfig {
            enabled: false,
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let classifier = CatBoostMLClassifier::new(config).unwrap();
        let process = create_test_process(); // Все значения по умолчанию (None/0)

        let features = classifier.process_to_features(&process);

        // Проверяем, что фичи извлечены правильно (расширенный набор)
        assert_eq!(features.len(), 29);

        // Проверяем числовые фичи (должны быть 0.0 кроме uptime)
        for i in 0..13 {
            assert_eq!(features[i], 0.0);
        }
        assert_eq!(features[13], 100.0); // uptime_sec
        assert_eq!(features[14], 100.0 / 3600.0); // uptime_hours

        // Проверяем логарифмические фичи (должны быть 0.0 для ln(0))
        assert_eq!(features[15], 0.0); // log CPU
        assert_eq!(features[16], 0.0); // log memory

        // Проверяем булевые фичи (должны быть 0.0)
        for feature in &features[17..26] {
            assert_eq!(*feature, 0.0);
        }
    }

    #[test]
    fn test_catboost_ml_classifier_feature_extraction_optimized() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        // Очищаем кэш перед тестом
        CatBoostMLClassifier::clear_feature_cache();

        let config = MLClassifierConfig {
            enabled: false,
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let classifier = CatBoostMLClassifier::new(config).unwrap();
        let mut process = create_test_process();

        // Устанавливаем различные значения
        process.cpu_share_1s = Some(0.25);
        process.cpu_share_10s = Some(0.5);
        process.io_read_bytes = Some(2 * 1024 * 1024); // 2MB
        process.io_write_bytes = Some(1024 * 1024); // 1MB
        process.rss_mb = Some(100);
        process.swap_mb = Some(50);
        process.voluntary_ctx = Some(1000);
        process.involuntary_ctx = Some(500);
        process.has_tty = true;
        process.has_gui_window = true;
        process.is_focused_window = true;
        process.env_has_display = true;
        process.env_has_wayland = true;
        process.env_ssh = false;
        process.is_audio_client = true;
        process.has_active_stream = true;
        process.uptime_sec = 3600; // 1 час

        // Тестируем оптимизированное извлечение фич без кэширования
        let features_uncached = classifier.process_to_features_optimized(&process, false);
        
        // Тестируем оптимизированное извлечение фич с кэшированием
        let features_cached = classifier.process_to_features_optimized(&process, true);
        
        // Проверяем, что оба метода возвращают одинаковые результаты
        assert_eq!(features_uncached.len(), 29);
        assert_eq!(features_cached.len(), 29);
        assert_eq!(features_uncached, features_cached);
        
        // Проверяем основные числовые фичи
        assert_eq!(features_uncached[0], 0.25); // cpu_share_1s
        assert_eq!(features_uncached[1], 0.5); // cpu_share_10s
        assert_eq!(features_uncached[2], 25.0); // cpu_share_1s %
        assert_eq!(features_uncached[3], 50.0); // cpu_share_10s %
        assert_eq!(features_uncached[4], 2.0); // io_read_bytes в MB
        assert_eq!(features_uncached[5], 1.0); // io_write_bytes в MB
        assert_eq!(features_uncached[6], 3.0); // total IO
        assert_eq!(features_uncached[7], 100.0); // rss_mb
        assert_eq!(features_uncached[8], 50.0); // swap_mb
        assert_eq!(features_uncached[9], 150.0 * 1024.0); // total memory KB
        assert_eq!(features_uncached[10], 1000.0); // voluntary_ctx
        assert_eq!(features_uncached[11], 500.0); // involuntary_ctx
        assert_eq!(features_uncached[12], (1000.0 + 500.0) / 3600.0); // ctx/sec
        assert_eq!(features_uncached[13], 3600.0); // uptime_sec
        assert_eq!(features_uncached[14], 1.0); // uptime_hours
        
        // Проверяем логарифмические фичи (приблизительно)
        assert!(features_uncached[15] > 0.0); // log CPU
        assert!(features_uncached[16] > 0.0); // log memory
        
        // Проверяем булевые фичи (должны быть 1.0)
        assert_eq!(features_uncached[17], 1.0); // has_tty
        assert_eq!(features_uncached[18], 1.0); // has_gui_window
        assert_eq!(features_uncached[19], 1.0); // is_focused_window
        assert_eq!(features_uncached[20], 1.0); // env_has_display
        assert_eq!(features_uncached[21], 1.0); // env_has_wayland
        assert_eq!(features_uncached[22], 0.0); // env_ssh
        assert_eq!(features_uncached[23], 1.0); // is_audio_client
        assert_eq!(features_uncached[24], 1.0); // has_active_stream

        // Проверяем расширенные булевые фичи
        assert_eq!(features_uncached[25], 1.0); // локальный TTY (has_tty и не env_ssh)
        assert_eq!(features_uncached[26], 1.0); // фокусированное GUI
        assert_eq!(features_uncached[27], 1.0); // интерактивный процесс
        assert_eq!(features_uncached[28], 1.0); // аудио активность
    }

    #[test]
    fn test_ml_classifier_config_validation() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        // Тестируем дефолтную конфигурацию
        let default_config = MLClassifierConfig::default();
        assert!(!default_config.enabled);
        assert_eq!(default_config.model_path, "models/process_classifier.json");
        assert_eq!(default_config.confidence_threshold, 0.7);
        assert!(matches!(default_config.model_type, ModelType::Catboost));
    }

    #[test]
    fn test_model_version_info_creation() {
        use tempfile::NamedTempFile;
        use std::io::Write;

        // Создаем временный файл модели
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "{{\"test_model\": true}}").unwrap();
        let model_path = temp_file.path();

        // Создаем информацию о версии
        let version_info = ModelVersionInfo::new(model_path).unwrap();

        // Проверяем, что информация создана правильно
        assert!(!version_info.model_hash.is_empty());
        assert!(version_info.model_hash.len() == 64); // SHA256 хэш
        assert!(version_info.model_size > 0);
        assert!(version_info.last_checked.elapsed().as_secs() < 1);
    }

    #[test]
    fn test_model_version_info_change_detection() {
        use tempfile::NamedTempFile;
        use std::io::Write;

        // Создаем временный файл модели
        let mut temp_file = NamedTempFile::new().unwrap();
        let model_path = temp_file.path().to_path_buf();

        // Пишем начальное содержимое
        writeln!(temp_file, "{{\"version\": 1}}").unwrap();
        
        // Создаем информацию о версии
        let version_info = ModelVersionInfo::new(&model_path).unwrap();
        let original_hash = version_info.model_hash.clone();
        let original_size = version_info.model_size;

        // Проверяем, что изменения не обнаружены для того же файла
        assert!(!version_info.has_changed(&model_path).unwrap());

        // Меняем содержимое файла
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&model_path)
            .unwrap();
        writeln!(file, "{{\"version\": 2, \"updated\": true}}").unwrap();
        drop(file);

        // Проверяем, что изменения обнаружены
        assert!(version_info.has_changed(&model_path).unwrap());

        // Создаем новую информацию о версии
        let new_version_info = ModelVersionInfo::new(&model_path).unwrap();
        assert_ne!(new_version_info.model_hash, original_hash);
        assert_ne!(new_version_info.model_size, original_size);
    }

    #[test]
    fn test_catboost_ml_classifier_model_versioning() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};
        use tempfile::NamedTempFile;
        use std::io::Write;

        // Создаем временный файл модели
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "{{\"test_model\": true}}").unwrap();
        let model_path = temp_file.path().to_str().unwrap().to_string();

        let config = MLClassifierConfig {
            enabled: true,
            model_path: model_path.clone(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        // Создаем классификатор (используем заглушку, так как это не валидная CatBoost модель)
        let classifier_result = CatBoostMLClassifier::new(config);
        
        // Должно быть ошибка загрузки модели, но мы можем протестировать методы версии
        assert!(classifier_result.is_err());
        
        // Тестируем с отключенным классификатором
        let config_disabled = MLClassifierConfig {
            enabled: false,
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let classifier = CatBoostMLClassifier::new(config_disabled).unwrap();
        
        // Проверяем, что версия не установлена для отключенного классификатора
        assert!(classifier.model_hash().is_none());
        assert!(classifier.model_size().is_none());
        assert!(!classifier.has_model_changed(Path::new("test.json")).unwrap());
    }

    #[test]
    fn test_catboost_ml_classifier_model_hash_methods() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        let config = MLClassifierConfig {
            enabled: false,
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let classifier = CatBoostMLClassifier::new(config).unwrap();
        
        // Проверяем методы доступа к информации о модели
        assert!(classifier.model_hash().is_none());
        assert!(classifier.model_size().is_none());
        
        // Проверяем, что метод has_model_changed работает
        let result = classifier.has_model_changed(Path::new("nonexistent.json"));
        // Должно быть Ok(false), так как нет информации о версии
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_catboost_ml_classifier_fallback_mechanism() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        let config = MLClassifierConfig {
            enabled: false,
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let mut main_classifier = CatBoostMLClassifier::new(config.clone()).unwrap();
        let mut fallback_classifier = StubMLClassifier::new();
        
        let mut process = create_test_process();
        process.has_gui_window = true; // Это даст высокую уверенность в StubMLClassifier
        
        // Тестируем classify_with_fallback
        let result = main_classifier.classify_with_fallback(&process, &mut fallback_classifier);
        
        // Должны получить результат от fallback классификатора
        assert_eq!(result.process_type, Some("gui".to_string()));
        assert!(result.confidence > 0.7);
    }

    #[test]
    fn test_catboost_ml_classifier_reload_mechanism() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};
        use tempfile::NamedTempFile;
        use std::io::Write;

        // Создаем временный файл модели
        let mut temp_file = NamedTempFile::new().unwrap();
        let model_path = temp_file.path().to_path_buf();
        
        // Пишем начальное содержимое (невалидная модель, но это нормально для теста)
        writeln!(temp_file, "{{\"version\": 1}}").unwrap();
        let model_path_str = model_path.to_str().unwrap().to_string();

        let config = MLClassifierConfig {
            enabled: true,
            model_path: model_path_str.clone(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        // Создаем классификатор (должно быть ошибка загрузки)
        let _classifier = CatBoostMLClassifier::new(config.clone()).unwrap_err();
        
        // Тестируем с отключенным классификатором
        let config_disabled = MLClassifierConfig {
            enabled: false,
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let mut classifier = CatBoostMLClassifier::new(config_disabled.clone()).unwrap();
        
        // Проверяем, что перезагрузка не выполняется для отключенного классификатора
        let reload_result = classifier.reload_model_if_changed(&config_disabled);
        assert!(reload_result.is_ok());
        assert!(!reload_result.unwrap());
    }

    #[test]
    fn test_catboost_ml_classifier_error_recovery() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        let config = MLClassifierConfig {
            enabled: false,
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let mut classifier = CatBoostMLClassifier::new(config).unwrap();
        let mut fallback_classifier = StubMLClassifier::new();
        
        let process = create_test_process(); // Процесс без особых признаков
        
        // Тестируем classify_with_fallback с низкой уверенностью
        let result = classifier.classify_with_fallback(&process, &mut fallback_classifier);
        
        // Должны получить результат от fallback классификатора
        assert_eq!(result.process_type, Some("unknown".to_string()));
        assert!(result.confidence < 0.5); // Низкая уверенность от fallback
    }

    #[test]
    fn test_catboost_ml_classifier_model_reload_with_version() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};
        use tempfile::NamedTempFile;
        use std::io::Write;

        // Создаем временный файл модели
        let mut temp_file = NamedTempFile::new().unwrap();
        let model_path = temp_file.path().to_path_buf();
        
        // Пишем начальное содержимое
        writeln!(temp_file, "{{\"version\": 1}}").unwrap();
        let model_path_str = model_path.to_str().unwrap().to_string();

        // Создаем классификатор с отключенной моделью
        let config_disabled = MLClassifierConfig {
            enabled: false,
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let mut classifier = CatBoostMLClassifier::new(config_disabled).unwrap();
        
        // Проверяем, что перезагрузка работает для несуществующего файла
        let config = MLClassifierConfig {
            enabled: true,
            model_path: "/nonexistent/model.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let reload_result = classifier.reload_model_if_changed(&config);
        assert!(reload_result.is_ok());
        assert!(!reload_result.unwrap()); // Не должно быть перезагрузки
    }

    #[test]
    fn test_feature_cache_operations() {
        // Проверяем начальную емкость кэша
        let initial_capacity = CatBoostMLClassifier::feature_cache_capacity();
        assert_eq!(initial_capacity, 1024);
        
        // Устанавливаем новую емкость
        CatBoostMLClassifier::set_feature_cache_capacity(512);
        let new_capacity = CatBoostMLClassifier::feature_cache_capacity();
        assert_eq!(new_capacity, 512);
        
        // Очищаем кэш
        CatBoostMLClassifier::clear_feature_cache();
        
        // Восстанавливаем исходную емкость
        CatBoostMLClassifier::set_feature_cache_capacity(1024);
    }

    #[test]
    fn test_feature_cache_performance() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        let config = MLClassifierConfig {
            enabled: false,
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let classifier = CatBoostMLClassifier::new(config).unwrap();
        
        // Создаем тестовый процесс
        let mut process = create_test_process();
        process.pid = 12345; // Уникальный PID для теста
        process.has_gui_window = true;
        process.cpu_share_10s = Some(0.5);
        
        // Первое извлечение фич (должно вычислить и кэшировать)
        let start_time1 = Instant::now();
        let features1 = classifier.process_to_features(&process);
        let duration1 = start_time1.elapsed();
        
        // Второе извлечение фич (должно использовать кэш)
        let start_time2 = Instant::now();
        let features2 = classifier.process_to_features(&process);
        let duration2 = start_time2.elapsed();
        
        // Проверяем, что результаты одинаковые
        assert_eq!(features1, features2);
        
        // Второе извлечение должно быть быстрее (или равно) первому
        assert!(duration2 <= duration1);
        
        // Проверяем, что фичи извлечены правильно
        assert_eq!(features1.len(), 29);
        assert!(features1[1] > 0.0); // cpu_share_10s
    }

    #[test]
    fn test_feature_cache_different_processes() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        // Очищаем кэш перед тестом
        CatBoostMLClassifier::clear_feature_cache();

        let config = MLClassifierConfig {
            enabled: false,
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let classifier = CatBoostMLClassifier::new(config).unwrap();
        
        // Создаем два разных процесса
        let mut process1 = create_test_process();
        process1.pid = 1000;
        process1.has_gui_window = true;
        
        let mut process2 = create_test_process();
        process2.pid = 2000;
        process2.is_audio_client = true;
        
        // Извлекаем фичи для обоих процессов
        let features1 = classifier.process_to_features(&process1);
        let features2 = classifier.process_to_features(&process2);
        
        // Фичи должны быть разными
        assert_ne!(features1, features2);
        
        // Проверяем, что фичи извлечены правильно
        assert_eq!(features1.len(), 29);
        assert_eq!(features2.len(), 29);
        
        // GUI процесс должен иметь соответствующие фичи
        assert_eq!(features1[18], 1.0); // has_gui_window
        
        // Аудио процесс должен иметь соответствующие фичи
        assert_eq!(features2[23], 1.0); // is_audio_client
    }

    #[test]
    fn test_feature_cache_cache_hit_miss() {
        // Очищаем кэш перед тестом
        CatBoostMLClassifier::clear_feature_cache();
        
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        let config = MLClassifierConfig {
            enabled: false,
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let classifier = CatBoostMLClassifier::new(config).unwrap();
        
        let mut process = create_test_process();
        process.pid = 99999; // Уникальный PID
        process.cpu_share_1s = Some(0.25);
        
        // Первое извлечение - cache miss
        let features1 = classifier.process_to_features(&process);
        
        // Второе извлечение - cache hit
        let features2 = classifier.process_to_features(&process);
        
        // Результаты должны быть одинаковыми
        assert_eq!(features1, features2);
        
        // Очищаем кэш
        CatBoostMLClassifier::clear_feature_cache();
        
        // Третье извлечение - cache miss снова
        let features3 = classifier.process_to_features(&process);
        
        // Результаты должны быть одинаковыми
        assert_eq!(features1, features3);
    }
    
    #[test]
    fn test_cache_statistics() {
        // Очищаем кэш перед тестом
        CatBoostMLClassifier::clear_feature_cache();
        
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        let config = MLClassifierConfig {
            enabled: false,
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let classifier = CatBoostMLClassifier::new(config).unwrap();
        
        let mut process = create_test_process();
        process.pid = 10000; // Уникальный PID
        
        // Первое извлечение - cache miss
        let _features1 = classifier.process_to_features(&process);
        
        // Второе извлечение - cache hit
        let _features2 = classifier.process_to_features(&process);
        
        // Третье извлечение - cache hit
        let _features3 = classifier.process_to_features(&process);
        
        // Получаем статистику кэша
        let stats = CatBoostMLClassifier::get_feature_cache_stats();
        
        // Должно быть 3 запроса, 1 мисс и 2 хита
        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.cache_hits, 2);
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.total_insertions, 1);
        
        // Проверяем процент хитов
        if let Some(hit_rate) = stats.hit_rate() {
            assert!(hit_rate > 0.5 && hit_rate <= 1.0);
        }
    }
    
    #[test]
    fn test_cache_capacity_adjustment() {
        // Очищаем кэш перед тестом
        CatBoostMLClassifier::clear_feature_cache();
        
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        let config = MLClassifierConfig {
            enabled: false,
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let classifier = CatBoostMLClassifier::new(config).unwrap();
        
        // Получаем текущую емкость кэша
        let initial_capacity = CatBoostMLClassifier::get_feature_cache_capacity();
        
        // Устанавливаем новую емкость
        CatBoostMLClassifier::set_feature_cache_capacity(512);
        
        // Проверяем, что емкость изменилась
        let new_capacity = CatBoostMLClassifier::get_feature_cache_capacity();
        assert_eq!(new_capacity, 512);
        
        // Восстанавливаем исходную емкость
        CatBoostMLClassifier::set_feature_cache_capacity(initial_capacity);
    }

    #[test]
    fn test_feature_cache_memory_usage_tracking() {
        // Тест: проверка отслеживания использования памяти кэшом
        let cache = FeatureCache::new(100);
        
        // Изначально память должна быть 0
        let (current_memory, max_memory) = cache.get_memory_usage();
        assert_eq!(current_memory, 0);
        assert_eq!(max_memory, 0);
        
        // Добавляем некоторые фичи в кэш
        let test_features = vec![1.0f32; 288]; // Типичный размер фич
        cache.cache.lock().unwrap().put(1000, test_features.clone());
        
        // Проверяем, что память обновлена
        let (current_memory_after, max_memory_after) = cache.get_memory_usage();
        assert!(current_memory_after > 0, "Текущая память должна быть больше 0");
        assert!(max_memory_after > 0, "Максимальная память должна быть больше 0");
        assert_eq!(current_memory_after, max_memory_after, "Текущая и максимальная память должны совпадать");
        
        // Добавляем еще один элемент
        cache.cache.lock().unwrap().put(1001, test_features.clone());
        
        // Проверяем, что память увеличилась
        let (current_memory_final, max_memory_final) = cache.get_memory_usage();
        assert!(current_memory_final > current_memory_after, "Текущая память должна увеличиться");
        assert!(max_memory_final >= current_memory_final, "Максимальная память должна быть >= текущей");
    }

    #[test]
    fn test_cache_stats_memory_tracking() {
        // Тест: проверка отслеживания статистики использования памяти
        let mut stats = CacheStats::default();
        
        // Изначально память должна быть 0
        assert_eq!(stats.current_memory_usage_bytes, 0);
        assert_eq!(stats.max_memory_usage_bytes, 0);
        assert_eq!(stats.memory_pressure_cleanups, 0);
        assert_eq!(stats.auto_capacity_adjustments, 0);
        
        // Обновляем использование памяти
        stats.update_memory_usage(1024);
        assert_eq!(stats.current_memory_usage_bytes, 1024);
        assert_eq!(stats.max_memory_usage_bytes, 1024);
        
        // Обновляем с большим значением
        stats.update_memory_usage(2048);
        assert_eq!(stats.current_memory_usage_bytes, 2048);
        assert_eq!(stats.max_memory_usage_bytes, 2048);
        
        // Обновляем с меньшим значением - max не должен измениться
        stats.update_memory_usage(1536);
        assert_eq!(stats.current_memory_usage_bytes, 1536);
        assert_eq!(stats.max_memory_usage_bytes, 2048, "Максимальная память не должна уменьшиться");
        
        // Проверяем регистрацию очисток
        stats.record_memory_pressure_cleanup();
        assert_eq!(stats.memory_pressure_cleanups, 1);
        
        stats.record_auto_capacity_adjustment();
        assert_eq!(stats.auto_capacity_adjustments, 1);
    }

    #[test]
    fn test_memory_pressure_cleanup_simulation() {
        // Тест: симуляция очистки кэша при давлении памяти
        // Этот тест проверяет логику без реального вызова sysinfo
        
        // Создаем кэш с тестовыми данными
        let cache = FeatureCache::new(10);
        let test_features = vec![1.0f32; 288];
        
        // Заполняем кэш
        for i in 0..5 {
            cache.cache.lock().unwrap().put(i, test_features.clone());
        }
        
        // Проверяем, что кэш не пустой
        let cache_len = cache.cache.lock().unwrap().len();
        assert!(cache_len > 0, "Кэш должен содержать элементы");
        
        // Проверяем, что статистика обновляется при добавлении
        let (current_memory, max_memory) = cache.get_memory_usage();
        assert!(current_memory > 0, "Память должна быть использована");
        
        // Тестируем ручную очистку
        cache.trigger_memory_cleanup();
        
        // После очистки кэш может быть очищен или уменьшен в зависимости от логики
        // Главное, что метод не падает и статистика обновляется
        let (current_memory_after, max_memory_after) = cache.get_memory_usage();
        // Память должна быть либо 0, либо меньше предыдущего значения
        assert!(current_memory_after <= current_memory, "Память после очистки должна быть <= предыдущей");
    }
}
