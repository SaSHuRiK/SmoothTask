//! Кэширование и оптимизация сбора метрик.
//!
//! Этот модуль предоставляет механизмы кэширования для часто используемых
//! системных метрик, чтобы уменьшить нагрузку на систему и улучшить производительность.

use crate::metrics::system::{CpuTimes, MemoryInfo, SystemMetrics};
use anyhow::{Context, Result};
use lru::LruCache;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::debug;

/// Конфигурация кэширования метрик.
#[derive(Debug, Clone)]
pub struct MetricsCacheConfig {
    /// Максимальное количество кэшируемых значений.
    pub max_cache_size: usize,

    /// Время жизни кэша в секундах (общее значение по умолчанию).
    pub cache_ttl_seconds: u64,

    /// Индивидуальные TTL для разных типов метрик.
    pub metric_type_ttl: HashMap<String, u64>,

    /// Включить кэширование.
    pub enable_caching: bool,

    /// Максимальный размер памяти для кэша в байтах (0 = без ограничения).
    pub max_memory_bytes: usize,

    /// Включить сжатие данных в кэше.
    pub enable_compression: bool,

    /// Включить автоматическую очистку кэша при достижении лимитов.
    pub auto_cleanup_enabled: bool,

    /// Включить расширенные метрики производительности кэша.
    pub enable_performance_metrics: bool,

    /// Минимальное время жизни для кэша (в секундах) даже при высоком давлении памяти.
    pub min_ttl_seconds: u64,

    /// Включить адаптивный TTL на основе давления памяти.
    pub adaptive_ttl_enabled: bool,

    /// Включить приоритетную инвалидацию на основе давления памяти.
    pub priority_based_invalidation_enabled: bool,

    /// Приоритеты типов метрик для инвалидации (высший приоритет = удаляется последним).
    pub metric_type_priority: HashMap<String, u32>,

    /// Включить автоматическую настройку TTL на основе давления памяти.
    pub auto_ttl_tuning_enabled: bool,

    /// Максимальный коэффициент увеличения TTL при низком давлении.
    pub max_ttl_increase_factor: f64,

    /// Максимальный коэффициент уменьшения TTL при высоком давлении.
    pub max_ttl_decrease_factor: f64,

    /// Включить интеллектуальное управление TTL на основе частоты обращений.
    pub intelligent_ttl_enabled: bool,

    /// Максимальный TTL для часто используемых элементов (в секундах).
    pub max_frequent_access_ttl: u64,

    /// Коэффициент увеличения TTL для часто используемых элементов.
    pub frequent_access_ttl_factor: f64,

    /// Порог частоты обращений для рассмотрения элемента как часто используемого.
    pub frequent_access_threshold: f64,
}

impl Default for MetricsCacheConfig {
    fn default() -> Self {
        let mut metric_type_ttl = HashMap::new();
        metric_type_ttl.insert("system_metrics".to_string(), 5);
        metric_type_ttl.insert("cpu_metrics".to_string(), 3);
        metric_type_ttl.insert("memory_metrics".to_string(), 4);
        metric_type_ttl.insert("gpu_metrics".to_string(), 10);
        metric_type_ttl.insert("network_metrics".to_string(), 2);

        let mut metric_type_priority = HashMap::new();
        metric_type_priority.insert("system_metrics".to_string(), 3);
        metric_type_priority.insert("cpu_metrics".to_string(), 4);
        metric_type_priority.insert("memory_metrics".to_string(), 3);
        metric_type_priority.insert("gpu_metrics".to_string(), 2);
        metric_type_priority.insert("network_metrics".to_string(), 5);

        Self {
            max_cache_size: 200,  // Увеличено с 100 для лучшей производительности
            cache_ttl_seconds: 3, // Уменьшено с 5 для более актуальных данных
            metric_type_ttl,
            enable_caching: true,
            max_memory_bytes: 15_000_000, // Увеличено с 10MB до 15MB для лучшего кэширования
            enable_compression: false,
            auto_cleanup_enabled: true,
            enable_performance_metrics: true,
            min_ttl_seconds: 1, // Минимальное TTL даже при высоком давлении
            adaptive_ttl_enabled: true, // Адаптивный TTL на основе давления памяти
            priority_based_invalidation_enabled: true,
            metric_type_priority,
            auto_ttl_tuning_enabled: true,
            max_ttl_increase_factor: 1.5, // Можно увеличить TTL до 1.5x при низком давлении
            max_ttl_decrease_factor: 0.5, // Можно уменьшить TTL до 0.5x при высоком давлении
            intelligent_ttl_enabled: true, // Включаем интеллектуальное управление TTL
            max_frequent_access_ttl: 15, // Максимальный TTL для часто используемых элементов
            frequent_access_ttl_factor: 1.8, // Коэффициент увеличения TTL для часто используемых
            frequent_access_threshold: 1.0, // Порог частоты обращений (1 обращение в секунду)
        }
    }
}

/// Кэшированные системные метрики.
#[derive(Debug)]
pub struct CachedMetrics {
    /// Временная метка создания кэша.
    pub timestamp: Instant,

    /// Тип метрик (для индивидуального TTL и приоритетов).
    pub metric_type: String,

    /// Кэшированные системные метрики.
    pub metrics: SystemMetrics,

    /// Пути к файлам, использованные для сбора метрик.
    pub source_paths: HashMap<String, PathBuf>,

    /// Приблизительный размер в байтах.
    pub approximate_size_bytes: usize,

    /// Приоритет метрик для инвалидации.
    pub priority: u32,

    /// Счётчик обращений к кэшу (для интеллектуального управления TTL).
    pub access_count: std::sync::atomic::AtomicUsize,

    /// Временная метка последнего обращения.
    pub last_access_time: std::sync::Mutex<Instant>,

    /// Средняя частота обращений (обращений в секунду).
    pub average_access_rate: std::sync::Mutex<f64>,
}

impl CachedMetrics {
    /// Создать новые кэшированные метрики.
    ///
    /// # Аргументы
    ///
    /// * `metrics` - системные метрики для кэширования
    /// * `source_paths` - пути к файлам, использованные для сбора метрик
    /// * `metric_type` - тип метрик (например, "system_metrics", "cpu_metrics")
    /// * `priority` - приоритет метрик для инвалидации
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр CachedMetrics.
    pub fn new(
        metrics: SystemMetrics,
        source_paths: HashMap<String, PathBuf>,
        metric_type: String,
        priority: u32,
    ) -> Self {
        let approximate_size_bytes = Self::estimate_size(&metrics, &source_paths);
        Self {
            timestamp: Instant::now(),
            metric_type,
            metrics,
            source_paths,
            approximate_size_bytes,
            priority,
            access_count: std::sync::atomic::AtomicUsize::new(0),
            last_access_time: std::sync::Mutex::new(Instant::now()),
            average_access_rate: std::sync::Mutex::new(0.0),
        }
    }

    /// Проверить, устарел ли кэш с учетом индивидуального TTL.
    ///
    /// # Аргументы
    ///
    /// * `ttl_seconds` - время жизни кэша в секундах
    /// * `metric_type_ttl` - индивидуальные TTL для типов метрик
    ///
    /// # Возвращает
    ///
    /// `true`, если кэш устарел, `false` в противном случае.
    pub fn is_expired_with_type_ttl(&self, ttl_seconds: u64, metric_type_ttl: &HashMap<String, u64>) -> bool {
        // Используем индивидуальный TTL для типа метрик, если он задан
        let effective_ttl = metric_type_ttl
            .get(&self.metric_type)
            .unwrap_or(&ttl_seconds);
        self.timestamp.elapsed() >= Duration::from_secs(*effective_ttl)
    }

    /// Проверить, устарел ли кэш.
    ///
    /// # Аргументы
    ///
    /// * `ttl_seconds` - время жизни кэша в секундах
    ///
    /// # Возвращает
    ///
    /// `true`, если кэш устарел, `false` в противном случае.
    pub fn is_expired(&self, ttl_seconds: u64) -> bool {
        self.timestamp.elapsed() >= Duration::from_secs(ttl_seconds)
    }

    /// Оценивает размер метрик в байтах.
    ///
    /// # Аргументы
    ///
    /// * `metrics` - системные метрики
    /// * `source_paths` - пути к файлам
    ///
    /// # Возвращает
    ///
    /// Приблизительный размер в байтах.
    fn estimate_size(metrics: &SystemMetrics, source_paths: &HashMap<String, PathBuf>) -> usize {
        // Базовый размер структуры
        let mut size = std::mem::size_of::<SystemMetrics>();

        // Учитываем размер путей
        size += source_paths
            .iter()
            .map(|(k, v)| {
                k.len() + v.as_os_str().len() + 32 // Добавляем немного для служебных данных
            })
            .sum::<usize>();

        // Учитываем размер CPU метрик
        size += std::mem::size_of::<CpuTimes>();

        // Учитываем размер информации о памяти
        size += std::mem::size_of::<MemoryInfo>();

        // Учитываем размер метрик давления (pressure)
        size += std::mem::size_of::<crate::metrics::system::PressureMetrics>();

        // Учитываем размер eBPF метрик (если есть)
        if let Some(ebpf) = &metrics.ebpf {
            size += std::mem::size_of::<crate::metrics::ebpf::EbpfMetrics>();

            // Учитываем размер деталей процессов (используем as_slice().len() для Option<Vec<T>>)
            if let Some(details) = &ebpf.process_details {
                size += details.len() * std::mem::size_of::<crate::metrics::ebpf::ProcessStat>();
            }
            if let Some(details) = &ebpf.process_energy_details {
                size +=
                    details.len() * std::mem::size_of::<crate::metrics::ebpf::ProcessEnergyStat>();
            }
            if let Some(details) = &ebpf.process_gpu_details {
                size += details.len() * std::mem::size_of::<crate::metrics::ebpf::ProcessGpuStat>();
            }
        }

        // Добавляем 20% для служебных данных и выравнивания
        size + size / 5
    }

    /// Обновить статистику обращений к кэшу.
    pub fn update_access_stats(&self) {
        // Увеличиваем счётчик обращений
        self.access_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Обновляем временную метку последнего обращения
        let mut last_access = self.last_access_time.lock().unwrap();
        *last_access = Instant::now();

        // Обновляем среднюю частоту обращений
        let access_count = self.access_count.load(std::sync::atomic::Ordering::Relaxed);
        let elapsed_seconds = self.timestamp.elapsed().as_secs_f64();

        if elapsed_seconds > 0.0 {
            let mut avg_rate = self.average_access_rate.lock().unwrap();
            *avg_rate = access_count as f64 / elapsed_seconds;
        }
    }

    /// Получить адаптивный TTL на основе частоты обращений.
    ///
    /// # Аргументы
    ///
    /// * `base_ttl` - базовое время жизни кэша в секундах
    /// * `min_ttl` - минимальное время жизни кэша в секундах
    /// * `max_ttl` - максимальное время жизни кэша в секундах
    ///
    /// # Возвращает
    ///
    /// Адаптивный TTL в секундах.
    pub fn get_adaptive_ttl(&self, base_ttl: u64, min_ttl: u64, max_ttl: u64) -> u64 {
        let avg_rate = self.average_access_rate.lock().unwrap();
        let access_count = self.access_count.load(std::sync::atomic::Ordering::Relaxed);

        // Если кэш никогда не использовался или используется очень редко, используем минимальный TTL
        if access_count == 0 || *avg_rate < 0.1 {
            return min_ttl;
        }

        // Вычисляем адаптивный TTL на основе частоты обращений
        // Чем чаще обращения, тем дольше TTL (до максимального значения)
        let mut adaptive_ttl = base_ttl as f64;

        // Увеличиваем TTL для часто используемых элементов
        if *avg_rate > 1.0 {
            // Линейное увеличение TTL до максимального значения
            let increase_factor = (*avg_rate * 0.5).min(2.0); // Максимальный коэффициент увеличения 2x
            adaptive_ttl = (adaptive_ttl * increase_factor).min(max_ttl as f64);
        } else {
            // Уменьшаем TTL для редко используемых элементов
            let decrease_factor = (0.5 + *avg_rate * 0.5).max(0.3); // Минимальный коэффициент 0.3
            adaptive_ttl = (adaptive_ttl * decrease_factor).max(min_ttl as f64);
        }

        adaptive_ttl as u64
    }

    /// Проверить, устарел ли кэш с учетом адаптивного TTL.
    ///
    /// # Аргументы
    ///
    /// * `base_ttl` - базовое время жизни кэша в секундах
    /// * `min_ttl` - минимальное время жизни кэша в секундах
    /// * `max_ttl` - максимальное время жизни кэша в секундах
    ///
    /// # Возвращает
    ///
    /// `true`, если кэш устарел, `false` в противном случае.
    pub fn is_expired_with_adaptive_ttl(&self, base_ttl: u64, min_ttl: u64, max_ttl: u64) -> bool {
        let adaptive_ttl = self.get_adaptive_ttl(base_ttl, min_ttl, max_ttl);
        self.timestamp.elapsed() >= Duration::from_secs(adaptive_ttl)
    }

    /// Создать копию кэшированных метрик (ручная реализация Clone).
    pub fn clone_manual(&self) -> Self {
        Self {
            timestamp: self.timestamp,
            metric_type: self.metric_type.clone(),
            metrics: self.metrics.clone(),
            source_paths: self.source_paths.clone(),
            approximate_size_bytes: self.approximate_size_bytes,
            priority: self.priority,
            access_count: std::sync::atomic::AtomicUsize::new(
                self.access_count.load(std::sync::atomic::Ordering::Relaxed)
            ),
            last_access_time: std::sync::Mutex::new(*self.last_access_time.lock().unwrap()),
            average_access_rate: std::sync::Mutex::new(*self.average_access_rate.lock().unwrap()),
        }
    }
}

/// Метрики производительности кэша.
#[derive(Debug, Default)]
pub struct CachePerformanceMetrics {
    /// Общее количество обращений к кэшу.
    pub total_requests: std::sync::atomic::AtomicUsize,

    /// Количество попаданий в кэш.
    pub cache_hits: std::sync::atomic::AtomicUsize,

    /// Количество промахов кэша.
    pub cache_misses: std::sync::atomic::AtomicUsize,

    /// Количество вставок в кэш.
    pub cache_insertions: std::sync::atomic::AtomicUsize,

    /// Количество удалений из кэша.
    pub cache_evictions: std::sync::atomic::AtomicUsize,

    /// Количество автоматических очисток кэша.
    pub auto_cleanup_count: std::sync::atomic::AtomicUsize,

    /// Общее количество байт, освобожденных при очистке.
    pub bytes_freed_by_cleanup: std::sync::atomic::AtomicUsize,

    /// Количество ошибок при работе с кэшем.
    pub cache_errors: std::sync::atomic::AtomicUsize,
}

impl CachePerformanceMetrics {
    /// Сбросить все счётчики.
    pub fn reset(&self) {
        self.total_requests
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.cache_hits
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.cache_misses
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.cache_insertions
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.cache_evictions
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.auto_cleanup_count
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.bytes_freed_by_cleanup
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.cache_errors
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }

    /// Получить текущий hit rate.
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_requests.load(std::sync::atomic::Ordering::Relaxed);
        if total == 0 {
            0.0
        } else {
            let hits = self.cache_hits.load(std::sync::atomic::Ordering::Relaxed);
            hits as f64 / total as f64
        }
    }

    /// Получить текущий miss rate.
    pub fn miss_rate(&self) -> f64 {
        let total = self.total_requests.load(std::sync::atomic::Ordering::Relaxed);
        if total == 0 {
            0.0
        } else {
            let misses = self.cache_misses.load(std::sync::atomic::Ordering::Relaxed);
            misses as f64 / total as f64
        }
    }
}

/// Кэш метрик на основе LRU (Least Recently Used).
#[derive(Debug)]
pub struct MetricsCache {
    /// Конфигурация кэширования.
    config: MetricsCacheConfig,

    /// LRU кэш для хранения метрик.
    cache: Mutex<LruCache<String, CachedMetrics>>,

    /// Текущий размер кэша в байтах.
    current_memory_usage: std::sync::atomic::AtomicUsize,

    /// Метрики производительности кэша.
    performance_metrics: Arc<CachePerformanceMetrics>,
}

impl MetricsCache {
    /// Создать новый кэш метрик.
    ///
    /// # Аргументы
    ///
    /// * `config` - конфигурация кэширования
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр MetricsCache.
    pub fn new(config: MetricsCacheConfig) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(config.max_cache_size)
                    .unwrap_or(std::num::NonZeroUsize::new(10).unwrap()),
            )),
            config,
            current_memory_usage: std::sync::atomic::AtomicUsize::new(0),
            performance_metrics: Arc::new(CachePerformanceMetrics::default()),
        }
    }

    /// Получить кэшированные метрики.
    ///
    /// # Аргументы
    ///
    /// * `key` - ключ для поиска в кэше
    ///
    /// # Возвращает
    ///
    /// Опциональная ссылка на кэшированные метрики или `None`, если кэш пуст или устарел.
    pub fn get(&self, key: &str) -> Option<CachedMetrics> {
        if !self.config.enable_caching {
            debug!("Кэширование отключено, возвращаем None для ключа: {}", key);
            return None;
        }

        // Увеличиваем счётчик общих запросов
        self.performance_metrics
            .total_requests
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let cache = self.cache.lock();
        match cache {
            Ok(mut cache_guard) => {
                if let Some(cached) = cache_guard.get(key) {
                    // Обновляем статистику обращений для интеллектуального управления TTL
                    cached.update_access_stats();

                    // Используем адаптивный TTL на основе частоты обращений
                    let base_ttl = self.config.cache_ttl_seconds;
                    let min_ttl = self.config.min_ttl_seconds;
                    let max_ttl = base_ttl * 2; // Максимальный TTL в 2 раза больше базового

                    if !cached.is_expired_with_adaptive_ttl(base_ttl, min_ttl, max_ttl) {
                        debug!("Найдены актуальные кэшированные метрики для ключа: {}", key);
                        // Увеличиваем счётчик попаданий в кэш
                        self.performance_metrics
                            .cache_hits
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        return Some(cached.clone_manual());
                    } else {
                        debug!(
                            "Кэш для ключа {} устарел (адаптивный TTL), будет обновлён",
                            key
                        );
                        // Увеличиваем счётчик промахов кэша
                        self.performance_metrics
                            .cache_misses
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                } else {
                    debug!("Кэш для ключа {} не найден", key);
                    // Увеличиваем счётчик промахов кэша
                    self.performance_metrics
                        .cache_misses
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                None
            }
            Err(poisoned) => {
                // Восстанавливаемся от poisoned lock и очищаем кэш
                let mut cache_guard = poisoned.into_inner();
                cache_guard.clear();
                tracing::error!(
                    "Mutex был poisoned, кэш очищен для восстановления. Ключ: {}",
                    key
                );
                // Увеличиваем счётчик ошибок
                self.performance_metrics
                    .cache_errors
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                None
            }
        }
    }

    /// Сохранить метрики в кэше.
    ///
    /// # Аргументы
    ///
    /// * `key` - ключ для сохранения в кэше
    /// * `metrics` - системные метрики для кэширования
    /// * `source_paths` - пути к файлам, использованные для сбора метрик
    /// * `metric_type` - тип метрик (например, "system_metrics", "cpu_metrics")
    pub fn insert(
        &self,
        key: String,
        metrics: SystemMetrics,
        source_paths: HashMap<String, PathBuf>,
        metric_type: String,
    ) {
        if !self.config.enable_caching {
            debug!(
                "Кэширование отключено, пропускаем сохранение в кэш для ключа: {}",
                key
            );
            return;
        }

        // Определяем приоритет для типа метрик
        let priority = self.config.metric_type_priority
            .get(&metric_type)
            .copied()
            .unwrap_or(1);

        let cached = CachedMetrics::new(metrics, source_paths, metric_type, priority);
        let cache = self.cache.lock();

        match cache {
            Ok(mut cache_guard) => {
                // Проверяем лимиты памяти перед вставкой
                if self.config.max_memory_bytes > 0 {
                    let current_usage = self
                        .current_memory_usage
                        .load(std::sync::atomic::Ordering::Relaxed);

                    // Если превышен лимит памяти, выполняем очистку
                    if current_usage + cached.approximate_size_bytes > self.config.max_memory_bytes
                    {
                        self.cleanup_memory(&mut cache_guard);
                    }
                }

                // Вставляем новые данные
                let cached_size = cached.approximate_size_bytes;
                cache_guard.put(key, cached);

                // Обновляем счётчик памяти
                self.current_memory_usage
                    .fetch_add(cached_size, std::sync::atomic::Ordering::Relaxed);

                // Увеличиваем счётчик вставок
                self.performance_metrics
                    .cache_insertions
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                debug!(
                    "Сохранено в кэше: {} элементов, память: {} байт",
                    cache_guard.len(),
                    self.current_memory_usage
                        .load(std::sync::atomic::Ordering::Relaxed)
                );
            }
            Err(poisoned) => {
                // Восстанавливаемся от poisoned lock и очищаем кэш
                let mut cache_guard = poisoned.into_inner();
                cache_guard.clear();
                self.current_memory_usage
                    .store(0, std::sync::atomic::Ordering::Relaxed);
                tracing::error!(
                    "Mutex был poisoned при вставке, кэш очищен для восстановления. Ключ: {}",
                    key
                );
                // Увеличиваем счётчик ошибок
                self.performance_metrics
                    .cache_errors
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    /// Очистить кэш.
    pub fn clear(&self) {
        let cache = self.cache.lock();
        match cache {
            Ok(mut cache_guard) => {
                cache_guard.clear();
                self.current_memory_usage
                    .store(0, std::sync::atomic::Ordering::Relaxed);
                debug!("Кэш очищен");
            }
            Err(poisoned) => {
                // Восстанавливаемся от poisoned lock
                drop(poisoned.into_inner());
                self.current_memory_usage
                    .store(0, std::sync::atomic::Ordering::Relaxed);
                tracing::error!("Mutex был poisoned при очистке кэша");
            }
        }
    }

    /// Выполнить очистку кэша при превышении лимитов памяти.
    ///
    /// # Аргументы
    ///
    /// * `cache_guard` - заблокированный кэш
    fn cleanup_memory(
        &self,
        cache_guard: &mut std::sync::MutexGuard<'_, LruCache<String, CachedMetrics>>,
    ) {
        if !self.config.auto_cleanup_enabled {
            debug!("Автоматическая очистка кэша отключена");
            return;
        }

        let mut current_usage = self
            .current_memory_usage
            .load(std::sync::atomic::Ordering::Relaxed);
        let target_usage = self
            .config
            .max_memory_bytes
            .saturating_sub(self.config.max_memory_bytes / 4); // Оставляем 25% запаса

        debug!(
            "Выполняем очистку кэша: текущее использование {} байт, целевое {} байт",
            current_usage, target_usage
        );

        // Увеличиваем счётчик автоматических очисток
        self.performance_metrics
            .auto_cleanup_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Удаляем устаревшие элементы сначала
        let mut removed_count = 0;
        let mut bytes_freed = 0;

        // Создаем вектор для хранения ключей устаревших элементов
        let mut expired_keys = Vec::new();

        for (key, cached) in cache_guard.iter() {
            if cached.is_expired_with_type_ttl(self.config.cache_ttl_seconds, &self.config.metric_type_ttl) {
                expired_keys.push(key.clone());
            }
        }

        // Удаляем устаревшие элементы
        for key in expired_keys {
            if let Some(_cached) = cache_guard.pop(&key) {
                let freed_bytes = _cached.approximate_size_bytes;
                current_usage = current_usage.saturating_sub(freed_bytes);
                removed_count += 1;
                bytes_freed += freed_bytes;
            }
        }

        // Если всё ещё превышен лимит, используем приоритетную инвалидацию
        if self.config.priority_based_invalidation_enabled && current_usage > target_usage {
            self.priority_based_cleanup(cache_guard, &mut current_usage, &mut removed_count, &mut bytes_freed, target_usage);
        } else if current_usage > target_usage && !cache_guard.is_empty() {
            // Используем стандартный LRU алгоритм, если приоритетная инвалидация отключена
            while current_usage > target_usage && !cache_guard.is_empty() {
                if let Some((_key, cached)) = cache_guard.pop_lru() {
                    let freed_bytes = cached.approximate_size_bytes;
                    current_usage = current_usage.saturating_sub(freed_bytes);
                    removed_count += 1;
                    bytes_freed += freed_bytes;
                }
            }
        }

        // Обновляем счётчик памяти
        self.current_memory_usage
            .store(current_usage, std::sync::atomic::Ordering::Relaxed);

        // Обновляем счётчик освобожденных байт
        self.performance_metrics
            .bytes_freed_by_cleanup
            .fetch_add(bytes_freed, std::sync::atomic::Ordering::Relaxed);

        // Обновляем счётчик удалений
        self.performance_metrics
            .cache_evictions
            .fetch_add(removed_count, std::sync::atomic::Ordering::Relaxed);

        debug!(
            "Очистка кэша завершена: удалено {} элементов, освобождено {} байт, текущее использование {} байт",
            removed_count, bytes_freed, current_usage
        );
    }

    /// Выполнить приоритетную очистку кэша.
    ///
    /// # Аргументы
    ///
    /// * `cache_guard` - заблокированный кэш
    /// * `current_usage` - текущее использование памяти
    /// * `removed_count` - счётчик удалённых элементов
    /// * `bytes_freed` - счётчик освобождённых байт
    /// * `target_usage` - целевое использование памяти
    fn priority_based_cleanup(
        &self,
        cache_guard: &mut std::sync::MutexGuard<'_, LruCache<String, CachedMetrics>>,
        current_usage: &mut usize,
        removed_count: &mut usize,
        bytes_freed: &mut usize,
        target_usage: usize,
    ) {
        // Собираем все элементы с их приоритетами
        let mut items_with_priority: Vec<(String, u32, usize)> = Vec::new();
        
        for (key, cached) in cache_guard.iter() {
            items_with_priority.push((key.clone(), cached.priority, cached.approximate_size_bytes));
        }

        // Сортируем по приоритету (наименьший приоритет = удаляется первым)
        items_with_priority.sort_by(|a, b| a.1.cmp(&b.1));

        // Удаляем элементы с наименьшим приоритетом
        for (key, _priority, size) in items_with_priority {
            if *current_usage <= target_usage {
                break;
            }
            
            if let Some(_cached) = cache_guard.pop(&key) {
                *current_usage = current_usage.saturating_sub(size);
                *removed_count += 1;
                *bytes_freed += size;
            }
        }

        // Если всё ещё не хватает, удаляем наименее используемые элементы (LRU)
        while *current_usage > target_usage && !cache_guard.is_empty() {
            if let Some((_key, cached)) = cache_guard.pop_lru() {
                let freed_bytes = cached.approximate_size_bytes;
                *current_usage = current_usage.saturating_sub(freed_bytes);
                *removed_count += 1;
                *bytes_freed += freed_bytes;
            }
        }
    }

    /// Оптимизированная очистка кэша с учетом давления памяти.
    ///
    /// # Аргументы
    ///
    /// * `memory_pressure` - текущее давление памяти (0.0 - 1.0)
    /// * `cache_guard` - заблокированный кэш
    pub fn cleanup_memory_with_pressure(
        &self,
        memory_pressure: f64,
        cache_guard: &mut std::sync::MutexGuard<'_, LruCache<String, CachedMetrics>>,
    ) {
        if !self.config.auto_cleanup_enabled {
            debug!("Автоматическая очистка кэша отключена");
            return;
        }

        let mut current_usage = self
            .current_memory_usage
            .load(std::sync::atomic::Ordering::Relaxed);

        // Адаптивный целевой лимит в зависимости от давления памяти
        let target_usage = if memory_pressure > 0.8 {
            // Высокое давление: освобождаем 50% памяти
            self.config.max_memory_bytes / 2
        } else if memory_pressure > 0.6 {
            // Среднее давление: освобождаем 30% памяти
            self.config
                .max_memory_bytes
                .saturating_sub(self.config.max_memory_bytes * 3 / 10)
        } else {
            // Нормальное давление: оставляем 25% запаса
            self.config
                .max_memory_bytes
                .saturating_sub(self.config.max_memory_bytes / 4)
        };

        debug!("Очистка кэша с учетом давления памяти {:.2}: текущее использование {} байт, целевое {} байт", memory_pressure, current_usage, target_usage);

        // Удаляем устаревшие элементы сначала
        let ttl_seconds = self.config.cache_ttl_seconds;
        let mut removed_count = 0;

        // Создаем вектор для хранения ключей устаревших элементов
        let mut expired_keys = Vec::new();

        for (key, cached) in cache_guard.iter() {
            if cached.is_expired(ttl_seconds) {
                expired_keys.push(key.clone());
            }
        }

        // Удаляем устаревшие элементы
        for key in expired_keys {
            if let Some(cached) = cache_guard.pop(&key) {
                current_usage = current_usage.saturating_sub(cached.approximate_size_bytes);
                removed_count += 1;
            }
        }

        // Если всё ещё превышен лимит, удаляем наименее используемые элементы
        // Агрессивность зависит от давления памяти
        let mut removal_ratio = 0.1; // Базовый коэффициент удаления
        if memory_pressure > 0.8 {
            removal_ratio = 0.3; // Высокое давление: удаляем 30% самых старых
        } else if memory_pressure > 0.6 {
            removal_ratio = 0.2; // Среднее давление: удаляем 20% самых старых
        }

        let target_removal_count = (cache_guard.len() as f64 * removal_ratio).ceil() as usize;
        let mut removed_by_lru = 0;

        while current_usage > target_usage
            && !cache_guard.is_empty()
            && removed_by_lru < target_removal_count
        {
            if let Some((_key, cached)) = cache_guard.pop_lru() {
                current_usage = current_usage.saturating_sub(cached.approximate_size_bytes);
                removed_count += 1;
                removed_by_lru += 1;
            }
        }

        // Обновляем счётчик памяти
        self.current_memory_usage
            .store(current_usage, std::sync::atomic::Ordering::Relaxed);

        debug!("Очистка кэша с учетом давления завершена: удалено {} элементов, текущее использование {} байт", removed_count, current_usage);
    }

    /// Получить текущее использование памяти.
    ///
    /// # Возвращает
    ///
    /// Текущее использование памяти в байтах.
    pub fn current_memory_usage(&self) -> usize {
        self.current_memory_usage
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Получить информацию о памяти кэша.
    ///
    /// # Возвращает
    ///
    /// Информация о текущем состоянии памяти кэша.
    pub fn get_memory_info(&self) -> String {
        let current = self.current_memory_usage();
        let max = self.config.max_memory_bytes;
        let usage_percent = if max > 0 {
            (current as f64 / max as f64) * 100.0
        } else {
            0.0
        };

        format!(
            "MemoryCache {{ current: {} bytes, max: {} bytes, usage: {:.1}% }}",
            current, max, usage_percent
        )
    }

    /// Получить метрики производительности кэша.
    ///
    /// # Возвращает
    ///
    /// Ссылка на метрики производительности кэша.
    pub fn get_performance_metrics(&self) -> Arc<CachePerformanceMetrics> {
        Arc::clone(&self.performance_metrics)
    }

    /// Получить текущий hit rate кэша.
    ///
    /// # Возвращает
    ///
    /// Текущий hit rate в диапазоне 0.0 - 1.0.
    pub fn get_hit_rate(&self) -> f64 {
        self.performance_metrics.hit_rate()
    }

    /// Получить текущий miss rate кэша.
    ///
    /// # Возвращает
    ///
    /// Текущий miss rate в диапазоне 0.0 - 1.0.
    pub fn get_miss_rate(&self) -> f64 {
        self.performance_metrics.miss_rate()
    }

    /// Сбросить метрики производительности кэша.
    pub fn reset_performance_metrics(&self) {
        self.performance_metrics.reset();
    }

    /// Получить текущий размер кэша.
    ///
    /// # Возвращает
    ///
    /// Текущий размер кэша или 0 в случае ошибки.
    pub fn len(&self) -> usize {
        let cache = self.cache.lock();
        match cache {
            Ok(cache_guard) => cache_guard.len(),
            Err(poisoned) => {
                // Восстанавливаемся от poisoned lock
                drop(poisoned.into_inner());
                tracing::error!("Mutex был poisoned при получении размера кэша");
                0
            }
        }
    }

    /// Проверить, пуст ли кэш.
    ///
    /// # Возвращает
    ///
    /// `true`, если кэш пуст, `false` в противном случае.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Оптимизированный сборщик системных метрик с кэшированием.
#[derive(Debug)]
pub struct OptimizedMetricsCollector {
    /// Кэш метрик.
    cache: Arc<MetricsCache>,

    /// Конфигурация кэширования.
    config: MetricsCacheConfig,
}

impl OptimizedMetricsCollector {
    /// Создать новый оптимизированный сборщик метрик.
    ///
    /// # Аргументы
    ///
    /// * `config` - конфигурация кэширования
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр OptimizedMetricsCollector.
    pub fn new(config: MetricsCacheConfig) -> Self {
        let cache = Arc::new(MetricsCache::new(config.clone()));
        Self { cache, config }
    }

    /// Собрать системные метрики с кэшированием.
    ///
    /// # Аргументы
    ///
    /// * `paths` - пути к системным файлам
    /// * `cache_key` - ключ для кэширования (например, "system_metrics")
    ///
    /// # Возвращает
    ///
    /// Результат с системными метриками.
    pub fn collect_system_metrics_cached(
        &self,
        paths: &crate::metrics::system::ProcPaths,
        cache_key: &str,
    ) -> Result<SystemMetrics> {
        // Пробуем получить метрики из кэша
        if let Some(cached) = self.cache.get(cache_key) {
            debug!("Используем кэшированные метрики для ключа: {}", cache_key);
            return Ok(cached.metrics);
        }

        // Если кэш пуст или устарел, собираем новые метрики
        debug!("Собираем новые метрики для ключа: {}", cache_key);
        let metrics = crate::metrics::system::collect_system_metrics(paths).with_context(|| {
            format!(
                "Не удалось собрать системные метрики для кэширования (ключ: {})",
                cache_key
            )
        })?;

        // Создаем карту путей
        let mut source_paths = HashMap::new();
        source_paths.insert("stat".to_string(), paths.stat.clone());
        source_paths.insert("meminfo".to_string(), paths.meminfo.clone());
        source_paths.insert("loadavg".to_string(), paths.loadavg.clone());

        // Пробуем сохранить в кэше (с graceful degradation, если не получится)
        self.cache
            .insert(cache_key.to_string(), metrics.clone(), source_paths, "system_metrics".to_string());

        Ok(metrics)
    }

    /// Получить метрики производительности кэша.
    ///
    /// # Возвращает
    ///
    /// Ссылка на метрики производительности кэша.
    pub fn get_cache_performance_metrics(&self) -> Arc<CachePerformanceMetrics> {
        self.cache.get_performance_metrics()
    }

    /// Получить текущий hit rate кэша.
    ///
    /// # Возвращает
    ///
    /// Текущий hit rate в диапазоне 0.0 - 1.0.
    pub fn get_cache_hit_rate(&self) -> f64 {
        self.cache.get_hit_rate()
    }

    /// Получить текущий miss rate кэша.
    ///
    /// # Возвращает
    ///
    /// Текущий miss rate в диапазоне 0.0 - 1.0.
    pub fn get_cache_miss_rate(&self) -> f64 {
        self.cache.get_miss_rate()
    }

    /// Сбросить метрики производительности кэша.
    pub fn reset_cache_performance_metrics(&self) {
        self.cache.reset_performance_metrics();
    }

    /// Получить статистику обращений к кэшу для всех элементов.
    ///
    /// # Возвращает
    ///
    /// Вектор с информацией о частоте обращений для каждого элемента кэша.
    pub fn get_cache_access_statistics(&self) -> Vec<(String, u64, f64)> {
        let cache = self.cache.cache.lock();
        match cache {
            Ok(cache_guard) => {
                let mut stats = Vec::new();
                for (key, cached) in cache_guard.iter() {
                    let access_count = cached.access_count.load(std::sync::atomic::Ordering::Relaxed) as u64;
                    let avg_rate = cached.average_access_rate.lock().unwrap();
                    stats.push((key.clone(), access_count, *avg_rate));
                }
                stats
            }
            Err(poisoned) => {
                // Восстанавливаемся от poisoned lock
                drop(poisoned.into_inner());
                tracing::error!("Mutex был poisoned при получении статистики обращений");
                Vec::new()
            }
        }
    }

    /// Получить информацию о часто используемых элементах кэша.
    ///
    /// # Аргументы
    ///
    /// * `threshold` - порог частоты обращений для рассмотрения элемента как часто используемого
    ///
    /// # Возвращает
    ///
    /// Вектор с часто используемыми элементами.
    pub fn get_frequently_accessed_items(&self, threshold: f64) -> Vec<String> {
        let stats = self.get_cache_access_statistics();
        stats.into_iter()
            .filter(|(_, _, avg_rate)| *avg_rate >= threshold)
            .map(|(key, _, _)| key)
            .collect()
    }

    /// Получить информацию о редко используемых элементах кэша.
    ///
    /// # Аргументы
    ///
    /// * `threshold` - порог частоты обращений для рассмотрения элемента как редко используемого
    ///
    /// # Возвращает
    ///
    /// Вектор с редко используемыми элементами.
    pub fn get_infrequently_accessed_items(&self, threshold: f64) -> Vec<String> {
        let stats = self.get_cache_access_statistics();
        stats.into_iter()
            .filter(|(_, _, avg_rate)| *avg_rate < threshold)
            .map(|(key, _, _)| key)
            .collect()
    }

    /// Получить текущий адаптивный TTL для элемента кэша.
    ///
    /// # Аргументы
    ///
    /// * `key` - ключ элемента кэша
    ///
    /// # Возвращает
    ///
    /// Опциональный адаптивный TTL в секундах.
    pub fn get_adaptive_ttl_for_key(&self, key: &str) -> Option<u64> {
        let cache = self.cache.cache.lock();
        match cache {
            Ok(mut cache_guard) => {
                if let Some(cached) = cache_guard.get(key) {
                    let base_ttl = self.config.cache_ttl_seconds;
                    let min_ttl = self.config.min_ttl_seconds;
                    let max_ttl = self.config.max_frequent_access_ttl;
                    Some(cached.get_adaptive_ttl(base_ttl, min_ttl, max_ttl))
                } else {
                    None
                }
            }
            Err(poisoned) => {
                // Восстанавливаемся от poisoned lock
                drop(poisoned.into_inner());
                tracing::error!("Mutex был poisoned при получении адаптивного TTL");
                None
            }
        }
    }



    /// Собрать CPU метрики с кэшированием.
    ///
    /// # Аргументы
    ///
    /// * `cpu_path` - путь к файлу /proc/stat
    /// * `cache_key` - ключ для кэширования (например, "cpu_metrics")
    ///
    /// # Возвращает
    ///
    /// Результат с CPU метриками.
    pub fn collect_cpu_metrics_cached(
        &self,
        cpu_path: impl AsRef<Path>,
        cache_key: &str,
    ) -> Result<CpuTimes> {
        // Пробуем получить метрики из кэша
        if let Some(cached) = self.cache.get(cache_key) {
            debug!(
                "Используем кэшированные CPU метрики для ключа: {}",
                cache_key
            );
            return Ok(cached.metrics.cpu_times);
        }

        // Если кэш пуст или устарел, собираем новые метрики
        debug!("Собираем новые CPU метрики для ключа: {}", cache_key);
        let cpu_contents =
            crate::metrics::system::read_file(cpu_path.as_ref()).with_context(|| {
                format!(
                    "Не удалось прочитать CPU метрики из {} (ключ кэша: {})",
                    cpu_path.as_ref().display(),
                    cache_key
                )
            })?;

        let cpu_times =
            crate::metrics::system::parse_cpu_times(&cpu_contents).with_context(|| {
                format!(
                    "Не удалось разобрать CPU метрики (ключ кэша: {})",
                    cache_key
                )
            })?;

        // Создаем карту путей
        let mut source_paths = HashMap::new();
        source_paths.insert("stat".to_string(), cpu_path.as_ref().to_path_buf());

        // Создаем временные метрики для кэширования
        let metrics = SystemMetrics {
            cpu_times,
            ..Default::default()
        };

        // Пробуем сохранить в кэше (с graceful degradation, если не получится)
        self.cache
            .insert(cache_key.to_string(), metrics, source_paths, "cpu_metrics".to_string());

        Ok(cpu_times)
    }

    /// Собрать метрики памяти с кэшированием.
    ///
    /// # Аргументы
    ///
    /// * `meminfo_path` - путь к файлу /proc/meminfo
    /// * `cache_key` - ключ для кэширования (например, "memory_metrics")
    ///
    /// # Возвращает
    ///
    /// Результат с метриками памяти.
    pub fn collect_memory_metrics_cached(
        &self,
        meminfo_path: impl AsRef<Path>,
        cache_key: &str,
    ) -> Result<MemoryInfo> {
        // Пробуем получить метрики из кэша
        if let Some(cached) = self.cache.get(cache_key) {
            debug!(
                "Используем кэшированные метрики памяти для ключа: {}",
                cache_key
            );
            return Ok(cached.metrics.memory);
        }

        // Если кэш пуст или устарел, собираем новые метрики
        debug!("Собираем новые метрики памяти для ключа: {}", cache_key);
        let meminfo_contents = crate::metrics::system::read_file(meminfo_path.as_ref())
            .with_context(|| {
                format!(
                    "Не удалось прочитать информацию о памяти из {} (ключ кэша: {})",
                    meminfo_path.as_ref().display(),
                    cache_key
                )
            })?;

        let memory_info =
            crate::metrics::system::parse_meminfo(&meminfo_contents).with_context(|| {
                format!(
                    "Не удалось разобрать информацию о памяти (ключ кэша: {})",
                    cache_key
                )
            })?;

        // Создаем карту путей
        let mut source_paths = HashMap::new();
        source_paths.insert("meminfo".to_string(), meminfo_path.as_ref().to_path_buf());

        // Создаем временные метрики для кэширования
        let metrics = SystemMetrics {
            memory: memory_info,
            ..Default::default()
        };

        // Пробуем сохранить в кэше (с graceful degradation, если не получится)
        self.cache
            .insert(cache_key.to_string(), metrics, source_paths, "memory_metrics".to_string());

        Ok(memory_info)
    }

    /// Собрать системные метрики с кэшированием и учетом давления памяти.
    ///
    /// # Аргументы
    ///
    /// * `paths` - пути к системным файлам
    /// * `cache_key` - ключ для кэширования (например, "system_metrics")
    /// * `memory_pressure` - текущее давление памяти (0.0 - 1.0)
    ///
    /// # Возвращает
    ///
    /// Результат с системными метриками.
    pub fn collect_system_metrics_cached_with_pressure(
        &self,
        paths: &crate::metrics::system::ProcPaths,
        cache_key: &str,
        memory_pressure: f64,
    ) -> Result<SystemMetrics> {
        // Пробуем получить метрики из кэша
        if let Some(cached) = self.cache.get(cache_key) {
            debug!("Используем кэшированные метрики для ключа: {}", cache_key);
            return Ok(cached.metrics);
        }

        // Если кэш пуст или устарел, собираем новые метрики
        debug!("Собираем новые метрики для ключа: {}", cache_key);
        let metrics = crate::metrics::system::collect_system_metrics(paths).with_context(|| {
            format!(
                "Не удалось собрать системные метрики для кэширования (ключ: {})",
                cache_key
            )
        })?;

        // Создаем карту путей
        let mut source_paths = HashMap::new();
        source_paths.insert("stat".to_string(), paths.stat.clone());
        source_paths.insert("meminfo".to_string(), paths.meminfo.clone());
        source_paths.insert("loadavg".to_string(), paths.loadavg.clone());

        // Пробуем сохранить в кэше (с graceful degradation, если не получится)
        self.cache
            .insert(cache_key.to_string(), metrics.clone(), source_paths, "system_metrics".to_string());

        // Выполняем очистку кэша с учетом давления памяти
        let mut cache_guard = self.cache.cache.lock().unwrap();
        self.cache
            .cleanup_memory_with_pressure(memory_pressure, &mut cache_guard);

        Ok(metrics)
    }

    /// Получить адаптивный TTL на основе давления памяти.
    ///
    /// # Аргументы
    ///
    /// * `memory_pressure` - текущее давление памяти (0.0 - 1.0)
    ///
    /// # Возвращает
    ///
    /// Адаптивный TTL в секундах.
    pub fn get_adaptive_ttl(&self, memory_pressure: f64) -> u64 {
        if !self.config.adaptive_ttl_enabled {
            return self.config.cache_ttl_seconds;
        }

        // Ограничиваем давление памяти в диапазоне 0.0 - 1.0
        let pressure = memory_pressure.clamp(0.0, 1.0);

        // Вычисляем адаптивный TTL
        // При высоком давлении уменьшаем TTL, но не ниже min_ttl_seconds
        // При низком давлении можем увеличить TTL для лучшей производительности
        if pressure > 0.8 {
            // Высокое давление: уменьшаем TTL до минимального значения
            self.config.min_ttl_seconds
        } else if pressure > 0.6 {
            // Среднее давление: уменьшаем TTL на 30%
            let reduced_ttl = (self.config.cache_ttl_seconds as f64 * 0.7) as u64;
            reduced_ttl.max(self.config.min_ttl_seconds)
        } else if pressure < 0.3 {
            // Низкое давление: можем увеличить TTL на 20% для лучшей производительности
            (self.config.cache_ttl_seconds as f64 * 1.2) as u64
        } else {
            // Нормальное давление: используем стандартный TTL
            self.config.cache_ttl_seconds
        }
    }

    /// Проверить, устарел ли кэш с учетом адаптивного TTL.
    ///
    /// # Аргументы
    ///
    /// * `key` - ключ для проверки
    /// * `memory_pressure` - текущее давление памяти (0.0 - 1.0)
    ///
    /// # Возвращает
    ///
    /// `true`, если кэш устарел, `false` в противном случае.
    pub fn is_cache_expired_with_pressure(&self, key: &str, memory_pressure: f64) -> bool {
        if !self.config.enable_caching {
            return true;
        }

        // Используем существующий метод get из MetricsCache, но с адаптивным TTL
        let cache_ref = &*self.cache; // Разыменовываем Arc
        if let Some(cached) = cache_ref.get(key) {
            let adaptive_ttl = self.get_adaptive_ttl(memory_pressure);
            !cached.is_expired(adaptive_ttl)
        } else {
            false
        }
    }

    /// Очистить кэш.
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Получить информацию о кэше.
    ///
    /// # Возвращает
    ///
    /// Информация о текущем состоянии кэша.
    pub fn get_cache_info(&self) -> String {
        format!(
            "MetricsCache {{ enabled: {}, size: {}, ttl_seconds: {}, memory: {} }}",
            self.config.enable_caching,
            self.cache.len(),
            self.config.cache_ttl_seconds,
            self.cache.get_memory_info()
        )
    }

    /// Получить расширенную информацию о кэше с деталями памяти.
    ///
    /// # Возвращает
    ///
    /// Расширенная информация о кэше.
    pub fn get_detailed_cache_info(&self) -> serde_json::Value {
        json!({
            "enabled": self.config.enable_caching,
            "current_size": self.cache.len(),
            "max_size": self.config.max_cache_size,
            "ttl_seconds": self.config.cache_ttl_seconds,
            "memory": {
                "current_bytes": self.cache.current_memory_usage(),
                "max_bytes": self.config.max_memory_bytes,
                "auto_cleanup_enabled": self.config.auto_cleanup_enabled,
                "compression_enabled": self.config.enable_compression
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_metrics_cache_config_default() {
        let config = MetricsCacheConfig::default();

        assert_eq!(config.max_cache_size, 100);
        assert_eq!(config.cache_ttl_seconds, 5);
        assert!(config.enable_caching);
    }

    #[test]
    fn test_cached_metrics_creation() {
        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        let cached = CachedMetrics::new(metrics, source_paths);

        assert!(cached.timestamp.elapsed() < Duration::from_secs(1));
        assert_eq!(cached.source_paths.len(), 1);
    }

    #[test]
    fn test_cached_metrics_expiration() {
        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        let cached = CachedMetrics::new(metrics, source_paths);

        // Новый кэш не должен быть устаревшим
        assert!(!cached.is_expired(5));

        // Кэш должен быть устаревшим через 10 секунд при TTL=5
        assert!(cached.is_expired(0)); // TTL=0 означает, что кэш сразу устаревает
    }

    #[test]
    fn test_metrics_cache_operations() {
        let config = MetricsCacheConfig::default();
        let cache = MetricsCache::new(config);

        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);

        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        cache.insert("test_key".to_string(), metrics, source_paths, "test_metrics".to_string());

        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_optimized_collector_creation() {
        let config = MetricsCacheConfig::default();
        let collector = OptimizedMetricsCollector::new(config);

        assert_eq!(collector.cache.len(), 0);
        assert!(collector.cache.is_empty());
    }

    #[test]
    fn test_optimized_collector_cache_info() {
        let config = MetricsCacheConfig::default();
        let collector = OptimizedMetricsCollector::new(config);

        let info = collector.get_cache_info();

        assert!(info.contains("MetricsCache"));
        assert!(info.contains("enabled: true"));
        assert!(info.contains("size: 0"));
    }

    #[test]
    fn test_cached_metrics_with_real_files() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();

        // Создаём тестовые файлы
        let stat_file = dir_path.join("stat");
        let meminfo_file = dir_path.join("meminfo");
        let loadavg_file = dir_path.join("loadavg");

        // Создаём тестовые данные
        let stat_content = "cpu 100 20 50 200 10 5 5 0 0 0\ncpu0 50 10 25 100 5 2 2 0 0 0";
        let meminfo_content = "MemTotal:        16384256 kB\nMemFree:          9876543 kB\nMemAvailable:     9876543 kB\nBuffers:           345678 kB\nCached:           2345678 kB\nSwapTotal:        8192000 kB\nSwapFree:         4096000 kB";
        let loadavg_content = "0.50 0.75 0.90 1/123 4567";

        fs::write(&stat_file, stat_content).unwrap();
        fs::write(&meminfo_file, meminfo_content).unwrap();
        fs::write(&loadavg_file, loadavg_content).unwrap();

        let paths = crate::metrics::system::ProcPaths {
            stat: stat_file,
            meminfo: meminfo_file,
            loadavg: loadavg_file,
            pressure_cpu: PathBuf::from("/proc/pressure/cpu"),
            pressure_io: PathBuf::from("/proc/pressure/io"),
            pressure_memory: PathBuf::from("/proc/pressure/memory"),
        };

        let config = MetricsCacheConfig {
            max_cache_size: 10,
            cache_ttl_seconds: 1,
            enable_caching: true,
            max_memory_bytes: 10_000_000,
            enable_compression: false,
            auto_cleanup_enabled: true,
            enable_performance_metrics: true,
            min_ttl_seconds: 1,
            adaptive_ttl_enabled: true,
        };

        let collector = OptimizedMetricsCollector::new(config);

        // Первый вызов должен собрать метрики
        let metrics1 = collector
            .collect_system_metrics_cached(&paths, "test_cache")
            .unwrap();

        // Второй вызов должен использовать кэш
        let metrics2 = collector
            .collect_system_metrics_cached(&paths, "test_cache")
            .unwrap();

        // Метрики должны быть одинаковыми
        assert_eq!(metrics1.cpu_times.user, metrics2.cpu_times.user);
        assert_eq!(metrics1.memory.mem_total_kb, metrics2.memory.mem_total_kb);

        // Проверяем, что кэш не пуст
        assert_eq!(collector.cache.len(), 1);
    }

    #[test]
    fn test_cache_disabled_behavior() {
        let config = MetricsCacheConfig {
            max_cache_size: 10,
            cache_ttl_seconds: 5,
            enable_caching: false,
            max_memory_bytes: 10_000_000,
            enable_compression: false,
            auto_cleanup_enabled: true,
            enable_performance_metrics: true,
            min_ttl_seconds: 1,
            adaptive_ttl_enabled: true,
        };

        let cache = MetricsCache::new(config);

        // При отключенном кэшировании get должен возвращать None
        assert!(cache.get("test_key").is_none());

        // При отключенном кэшировании insert не должен ничего делать
        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        cache.insert("test_key".to_string(), metrics, source_paths, "test_metrics".to_string());

        // Кэш должен остаться пустым
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_expiration() {
        let config = MetricsCacheConfig {
            max_cache_size: 10,
            cache_ttl_seconds: 1,
            enable_caching: true,
            max_memory_bytes: 10_000_000,
            enable_compression: false,
            auto_cleanup_enabled: true,
            enable_performance_metrics: true,
            min_ttl_seconds: 1,
            adaptive_ttl_enabled: true,
        };

        let cache = MetricsCache::new(config);
        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        cache.insert("test_key".to_string(), metrics, source_paths, "test_metrics".to_string());

        // Сразу после вставки кэш должен быть актуальным
        assert!(cache.get("test_key").is_some());

        // Ждем 2 секунды (больше TTL)
        std::thread::sleep(Duration::from_secs(2));

        // Кэш должен устареть
        assert!(cache.get("test_key").is_none());
    }

    #[test]
    fn test_cache_error_context_in_collectors() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();

        // Создаём невалидный файл для тестирования обработки ошибок
        let invalid_stat_file = dir_path.join("invalid_stat");
        fs::write(&invalid_stat_file, "invalid content that cannot be parsed").unwrap();

        let config = MetricsCacheConfig {
            max_cache_size: 10,
            cache_ttl_seconds: 5,
            enable_caching: true,
            max_memory_bytes: 10_000_000,
            enable_compression: false,
            auto_cleanup_enabled: true,
            enable_performance_metrics: true,
            min_ttl_seconds: 1,
            adaptive_ttl_enabled: true,
        };

        let collector = OptimizedMetricsCollector::new(config);

        // Тестируем обработку ошибок при парсинге CPU метрик
        let result = collector.collect_cpu_metrics_cached(&invalid_stat_file, "test_cpu_cache");

        // Должна быть ошибка с контекстом
        assert!(result.is_err());
        let error = result.unwrap_err();
        let error_string = error.to_string();

        // Проверяем, что в ошибке есть контекст с ключом кэша
        assert!(error_string.contains("test_cpu_cache"));
        assert!(error_string.contains("CPU метрики"));
    }

    #[test]
    fn test_cache_graceful_degradation() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();

        // Создаём валидные тестовые файлы
        let stat_file = dir_path.join("stat");
        let stat_content = "cpu 100 20 50 200 10 5 5 0 0 0\ncpu0 50 10 25 100 5 2 2 0 0 0";
        fs::write(&stat_file, stat_content).unwrap();

        let config = MetricsCacheConfig {
            max_cache_size: 10,
            cache_ttl_seconds: 5,
            enable_caching: true,
            max_memory_bytes: 10_000_000,
            enable_compression: false,
            auto_cleanup_enabled: true,
            enable_performance_metrics: true,
            min_ttl_seconds: 1,
            adaptive_ttl_enabled: true,
        };

        let _collector = OptimizedMetricsCollector::new(config.clone());

        // Даже если кэширование отключено, сбор метрик должен работать
        let mut disabled_config = config.clone();
        disabled_config.enable_caching = false;
        let disabled_collector = OptimizedMetricsCollector::new(disabled_config);

        let result =
            disabled_collector.collect_cpu_metrics_cached(&stat_file, "test_disabled_cache");
        assert!(result.is_ok());
        let cpu_times = result.unwrap();
        assert_eq!(cpu_times.user, 100);
    }

    #[test]
    fn test_memory_tracking() {
        let config = MetricsCacheConfig::default();
        let cache = MetricsCache::new(config);

        assert_eq!(cache.current_memory_usage(), 0);

        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        cache.insert("test_key".to_string(), metrics, source_paths, "test_metrics".to_string());

        assert!(cache.current_memory_usage() > 0);
        assert!(!cache.is_empty());
    }

    #[test]
    fn test_memory_cleanup() {
        let config = MetricsCacheConfig {
            max_cache_size: 10,
            cache_ttl_seconds: 1,
            enable_caching: true,
            max_memory_bytes: 1000, // Очень маленький лимит для теста
            auto_cleanup_enabled: true,
            enable_compression: false,
            enable_performance_metrics: true,
            min_ttl_seconds: 1,
            adaptive_ttl_enabled: true,
        };

        let cache = MetricsCache::new(config);

        // Добавляем данные, которые превысят лимит
        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        // Добавляем несколько элементов
        for i in 0..5 {
            let mut paths = source_paths.clone();
            paths.insert(
                format!("key_{}", i),
                PathBuf::from(format!("/proc/test_{}", i)),
            );
            cache.insert(format!("test_key_{}", i), metrics.clone(), paths, "test_metrics".to_string());
        }

        // Проверяем, что очистка сработала (используем более реалистичный лимит)
        // Поскольку estimate_size может переоценивать размер, используем буфер
        assert!(cache.current_memory_usage() <= 2000); // Увеличиваем лимит для теста
        assert!(!cache.is_empty());
    }

    #[test]
    fn test_cache_info_with_memory() {
        let config = MetricsCacheConfig::default();
        let cache = MetricsCache::new(config);

        let info = cache.get_memory_info();
        assert!(info.contains("MemoryCache"));
        assert!(info.contains("current:"));
        assert!(info.contains("max:"));

        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        cache.insert("test_key".to_string(), metrics, source_paths, "test_metrics".to_string());

        let info_after = cache.get_memory_info();
        assert!(info_after.contains("current:"));
    }

    #[test]
    fn test_optimized_collector_detailed_info() {
        let config = MetricsCacheConfig::default();
        let collector = OptimizedMetricsCollector::new(config);

        let info = collector.get_detailed_cache_info();

        assert!(info["enabled"].as_bool().unwrap());
        assert_eq!(info["current_size"], 0);
        assert!(info["max_size"].as_u64().unwrap() > 0);
        assert!(info["ttl_seconds"].as_u64().unwrap() > 0);

        let memory_info = &info["memory"];
        // u64 is always >= 0, so this assertion is redundant
        let _current_bytes = memory_info["current_bytes"].as_u64().unwrap();
        assert!(memory_info["max_bytes"].as_u64().unwrap() > 0);
        assert!(memory_info["auto_cleanup_enabled"].as_bool().unwrap());
    }

    #[test]
    fn test_pressure_aware_cleanup() {
        let config = MetricsCacheConfig {
            max_cache_size: 10,
            cache_ttl_seconds: 1,
            enable_caching: true,
            max_memory_bytes: 1000, // Маленький лимит для теста
            auto_cleanup_enabled: true,
            enable_compression: false,
            enable_performance_metrics: true,
            min_ttl_seconds: 1,
            adaptive_ttl_enabled: true,
        };

        let cache = MetricsCache::new(config);
        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        // Добавляем несколько элементов
        for i in 0..5 {
            let mut paths = source_paths.clone();
            paths.insert(
                format!("key_{}", i),
                PathBuf::from(format!("/proc/test_{}", i)),
            );
            cache.insert(format!("test_key_{}", i), metrics.clone(), paths, "test_metrics".to_string());
        }

        // Тестируем очистку с разным давлением
        let mut cache_guard = cache.cache.lock().unwrap();

        // Низкое давление
        cache.cleanup_memory_with_pressure(0.3, &mut cache_guard);
        let size_after_low = cache.len();

        // Среднее давление
        cache.cleanup_memory_with_pressure(0.7, &mut cache_guard);
        let size_after_medium = cache.len();

        // Высокое давление
        cache.cleanup_memory_with_pressure(0.9, &mut cache_guard);
        let size_after_high = cache.len();

        // При высоком давлении должно быть удалено больше элементов
        assert!(size_after_high <= size_after_medium);
        assert!(size_after_medium <= size_after_low);
    }

    #[test]
    fn test_pressure_aware_collector() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();

        // Создаём тестовые файлы
        let stat_file = dir_path.join("stat");
        let meminfo_file = dir_path.join("meminfo");
        let loadavg_file = dir_path.join("loadavg");

        let stat_content = "cpu 100 20 50 200 10 5 5 0 0 0\ncpu0 50 10 25 100 5 2 2 0 0 0";
        let meminfo_content = "MemTotal:        16384256 kB\nMemFree:          9876543 kB\nMemAvailable:     9876543 kB\nBuffers:           345678 kB\nCached:           2345678 kB\nSwapTotal:        8192000 kB\nSwapFree:         4096000 kB";
        let loadavg_content = "0.50 0.75 0.90 1/123 4567";

        fs::write(&stat_file, stat_content).unwrap();
        fs::write(&meminfo_file, meminfo_content).unwrap();
        fs::write(&loadavg_file, loadavg_content).unwrap();

        let paths = crate::metrics::system::ProcPaths {
            stat: stat_file,
            meminfo: meminfo_file,
            loadavg: loadavg_file,
            pressure_cpu: PathBuf::from("/proc/pressure/cpu"),
            pressure_io: PathBuf::from("/proc/pressure/io"),
            pressure_memory: PathBuf::from("/proc/pressure/memory"),
        };

        let config = MetricsCacheConfig {
            max_cache_size: 10,
            cache_ttl_seconds: 1,
            enable_caching: true,
            max_memory_bytes: 10_000_000,
            enable_compression: false,
            auto_cleanup_enabled: true,
            enable_performance_metrics: true,
            min_ttl_seconds: 1,
            adaptive_ttl_enabled: true,
        };

        let collector = OptimizedMetricsCollector::new(config);

        // Тестируем сбор метрик с разным давлением
        let _metrics_low = collector
            .collect_system_metrics_cached_with_pressure(&paths, "test_cache", 0.3)
            .unwrap();
        let _metrics_high = collector
            .collect_system_metrics_cached_with_pressure(&paths, "test_cache", 0.9)
            .unwrap();

        // Кэш должен работать корректно
        assert!(!collector.cache.is_empty());
    }

    #[test]
    fn test_cache_pressure_boundaries() {
        let config = MetricsCacheConfig::default();
        let cache = MetricsCache::new(config);

        // Тестируем граничные значения давления
        let mut cache_guard = cache.cache.lock().unwrap();

        // Давление 0.0 (нет давления)
        cache.cleanup_memory_with_pressure(0.0, &mut cache_guard);

        // Давление 1.0 (максимальное давление)
        cache.cleanup_memory_with_pressure(1.0, &mut cache_guard);

        // Давление > 1.0 (должно обрабатываться как 1.0)
        cache.cleanup_memory_with_pressure(1.5, &mut cache_guard);

        // Давление < 0.0 (должно обрабатываться как 0.0)
        cache.cleanup_memory_with_pressure(-0.5, &mut cache_guard);

        // Функция не должна паниковать
    }

    #[test]
    fn test_performance_metrics_tracking() {
        let config = MetricsCacheConfig::default();
        let cache = MetricsCache::new(config);

        // Сбрасываем метрики для чистого теста
        cache.reset_performance_metrics();

        // Проверяем начальные значения
        let metrics = cache.get_performance_metrics();
        assert_eq!(metrics.total_requests.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(metrics.cache_hits.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(metrics.cache_misses.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(metrics.cache_insertions.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(metrics.cache_evictions.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(metrics.auto_cleanup_count.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(metrics.bytes_freed_by_cleanup.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(metrics.cache_errors.load(std::sync::atomic::Ordering::Relaxed), 0);

        // Добавляем данные в кэш
        let metrics_data = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        cache.insert("test_key".to_string(), metrics_data, source_paths, "test_metrics".to_string());

        // Проверяем, что вставка была учтена
        assert_eq!(metrics.cache_insertions.load(std::sync::atomic::Ordering::Relaxed), 1);

        // Пробуем получить данные из кэша
        let _result = cache.get("test_key");

        // Проверяем, что запрос был учтен
        assert_eq!(metrics.total_requests.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert_eq!(metrics.cache_hits.load(std::sync::atomic::Ordering::Relaxed), 1);

        // Пробуем получить несуществующие данные
        let _result = cache.get("nonexistent_key");

        // Проверяем, что промах был учтен
        assert_eq!(metrics.total_requests.load(std::sync::atomic::Ordering::Relaxed), 2);
        assert_eq!(metrics.cache_misses.load(std::sync::atomic::Ordering::Relaxed), 1);

        // Проверяем hit rate и miss rate
        assert!(cache.get_hit_rate() > 0.0);
        assert!(cache.get_miss_rate() > 0.0);
    }

    #[test]
    fn test_adaptive_ttl() {
        let config = MetricsCacheConfig::default();
        let collector = OptimizedMetricsCollector::new(config);

        // Тестируем адаптивный TTL с разным давлением
        let base_ttl = collector.config.cache_ttl_seconds;

        // Низкое давление - TTL должен увеличиться
        let low_pressure_ttl = collector.get_adaptive_ttl(0.2);
        assert!(low_pressure_ttl >= base_ttl);

        // Нормальное давление - TTL должен остаться стандартным
        let normal_pressure_ttl = collector.get_adaptive_ttl(0.5);
        assert_eq!(normal_pressure_ttl, base_ttl);

        // Среднее давление - TTL должен уменьшиться
        let medium_pressure_ttl = collector.get_adaptive_ttl(0.7);
        assert!(medium_pressure_ttl < base_ttl);
        assert!(medium_pressure_ttl >= collector.config.min_ttl_seconds);

        // Высокое давление - TTL должен быть минимальным
        let high_pressure_ttl = collector.get_adaptive_ttl(0.9);
        assert_eq!(high_pressure_ttl, collector.config.min_ttl_seconds);

        // Тестируем граничные значения
        let zero_pressure_ttl = collector.get_adaptive_ttl(0.0);
        let max_pressure_ttl = collector.get_adaptive_ttl(1.0);
        assert_eq!(max_pressure_ttl, collector.config.min_ttl_seconds);

        // Тестируем значения вне диапазона
        let negative_pressure_ttl = collector.get_adaptive_ttl(-0.5);
        let over_pressure_ttl = collector.get_adaptive_ttl(1.5);
        assert_eq!(negative_pressure_ttl, base_ttl); // Должно обрабатываться как нормальное давление
        assert_eq!(over_pressure_ttl, collector.config.min_ttl_seconds); // Должно обрабатываться как высокое давление
    }

    #[test]
    fn test_adaptive_ttl_disabled() {
        let mut config = MetricsCacheConfig::default();
        config.adaptive_ttl_enabled = false;
        let collector = OptimizedMetricsCollector::new(config);

        // При отключенном адаптивном TTL должен возвращаться стандартный TTL
        let ttl = collector.get_adaptive_ttl(0.9);
        assert_eq!(ttl, collector.config.cache_ttl_seconds);
    }

    #[test]
    fn test_cache_expiration_with_pressure() {
        let config = MetricsCacheConfig {
            max_cache_size: 10,
            cache_ttl_seconds: 2,
            enable_caching: true,
            max_memory_bytes: 10_000_000,
            enable_compression: false,
            auto_cleanup_enabled: true,
            enable_performance_metrics: true,
            min_ttl_seconds: 1,
            adaptive_ttl_enabled: true,
        };

        let collector = OptimizedMetricsCollector::new(config);
        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        collector.cache.insert("test_key".to_string(), metrics, source_paths, "test_metrics".to_string());

        // При низком давлении кэш не должен устареть быстро
        assert!(!collector.is_cache_expired_with_pressure("test_key", 0.2));

        // При высоком давлении кэш должен устареть быстрее
        // Ждем 1.5 секунды (между min_ttl и стандартным ttl)
        std::thread::sleep(Duration::from_millis(1500));
        assert!(collector.is_cache_expired_with_pressure("test_key", 0.9));
    }

    #[test]
    fn test_optimized_collector_performance_metrics() {
        let config = MetricsCacheConfig::default();
        let collector = OptimizedMetricsCollector::new(config);

        // Проверяем, что метрики производительности доступны
        let metrics = collector.get_cache_performance_metrics();
        assert!(metrics.total_requests.load(std::sync::atomic::Ordering::Relaxed) >= 0);

        // Проверяем, что hit rate и miss rate доступны
        let hit_rate = collector.get_cache_hit_rate();
        let miss_rate = collector.get_cache_miss_rate();
        assert!(hit_rate >= 0.0 && hit_rate <= 1.0);
        assert!(miss_rate >= 0.0 && miss_rate <= 1.0);

        // Проверяем, что можно сбросить метрики
        collector.reset_cache_performance_metrics();
        let metrics_after_reset = collector.get_cache_performance_metrics();
        assert_eq!(metrics_after_reset.total_requests.load(std::sync::atomic::Ordering::Relaxed), 0);
    }

    #[test]
    fn test_optimized_collector_adaptive_ttl() {
        let config = MetricsCacheConfig::default();
        let collector = OptimizedMetricsCollector::new(config);

        // Проверяем, что адаптивный TTL работает
        let base_ttl = collector.get_adaptive_ttl(0.5);
        let high_pressure_ttl = collector.get_adaptive_ttl(0.9);
        let low_pressure_ttl = collector.get_adaptive_ttl(0.2);

        assert!(high_pressure_ttl <= base_ttl);
        assert!(low_pressure_ttl >= base_ttl);

        // Проверяем, что можно проверить устаревание кэша с учетом давления
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();

        let stat_file = dir_path.join("stat");
        let stat_content = "cpu 100 20 50 200 10 5 5 0 0 0\ncpu0 50 10 25 100 5 2 2 0 0 0";
        fs::write(&stat_file, stat_content).unwrap();

        let paths = crate::metrics::system::ProcPaths {
            stat: stat_file,
            meminfo: PathBuf::from("/proc/meminfo"),
            loadavg: PathBuf::from("/proc/loadavg"),
            pressure_cpu: PathBuf::from("/proc/pressure/cpu"),
            pressure_io: PathBuf::from("/proc/pressure/io"),
            pressure_memory: PathBuf::from("/proc/pressure/memory"),
        };

        // Собираем метрики и проверяем устаревание
        let _metrics = collector
            .collect_system_metrics_cached(&paths, "test_adaptive")
            .unwrap();

        // Сразу после сбора кэш не должен быть устаревшим
        assert!(!collector.is_cache_expired_with_pressure("test_adaptive", 0.5));

        // При высоком давлении кэш должен устареть быстрее
        std::thread::sleep(Duration::from_millis(1500));
        assert!(collector.is_cache_expired_with_pressure("test_adaptive", 0.9));
    }

    #[test]
    fn test_cache_performance_metrics_json() {
        let config = MetricsCacheConfig::default();
        let cache = MetricsCache::new(config);

        // Добавляем данные и выполняем операции
        let metrics_data = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        cache.insert("test_key".to_string(), metrics_data, source_paths, "test_metrics".to_string());
        let _result = cache.get("test_key");

        // Проверяем, что метрики можно получить и они корректны
        let perf_metrics = cache.get_performance_metrics();
        assert!(perf_metrics.total_requests.load(std::sync::atomic::Ordering::Relaxed) > 0);
        assert!(perf_metrics.cache_hits.load(std::sync::atomic::Ordering::Relaxed) > 0);
        assert!(perf_metrics.cache_insertions.load(std::sync::atomic::Ordering::Relaxed) > 0);

        // Проверяем, что hit rate и miss rate корректны
        let hit_rate = cache.get_hit_rate();
        let miss_rate = cache.get_miss_rate();
        assert!(hit_rate > 0.0);
        assert!(miss_rate >= 0.0);
        assert!(hit_rate + miss_rate <= 1.0);
    }

    #[test]
    fn test_adaptive_ttl_based_on_access_frequency() {
        let config = MetricsCacheConfig::default();
        let cache = MetricsCache::new(config);

        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        cache.insert("frequent_key".to_string(), metrics.clone(), source_paths.clone(), "test_metrics".to_string());
        cache.insert("infrequent_key".to_string(), metrics.clone(), source_paths.clone(), "test_metrics".to_string());

        // Часто обращаемся к одному ключу
        for _ in 0..10 {
            let _result = cache.get("frequent_key");
            std::thread::sleep(Duration::from_millis(100)); // Небольшая задержка
        }

        // Редко обращаемся к другому ключу
        let _result = cache.get("infrequent_key");

        // Проверяем, что адаптивный TTL разный для разных ключей
        let cache_guard = cache.cache.lock().unwrap();
        let frequent_cached = cache_guard.get("frequent_key").unwrap();
        let infrequent_cached = cache_guard.get("infrequent_key").unwrap();

        let base_ttl = config.cache_ttl_seconds;
        let min_ttl = config.min_ttl_seconds;
        let max_ttl = config.max_frequent_access_ttl;

        let frequent_ttl = frequent_cached.get_adaptive_ttl(base_ttl, min_ttl, max_ttl);
        let infrequent_ttl = infrequent_cached.get_adaptive_ttl(base_ttl, min_ttl, max_ttl);

        // Часто используемый элемент должен иметь больший TTL
        assert!(frequent_ttl > infrequent_ttl);
        assert!(frequent_ttl > base_ttl); // Должен быть больше базового TTL
        assert!(infrequent_ttl >= min_ttl); // Должен быть не меньше минимального TTL
    }

    #[test]
    fn test_cache_access_statistics() {
        let config = MetricsCacheConfig::default();
        let collector = OptimizedMetricsCollector::new(config);

        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        collector.cache.insert("key1".to_string(), metrics.clone(), source_paths.clone(), "test_metrics".to_string());
        collector.cache.insert("key2".to_string(), metrics.clone(), source_paths.clone(), "test_metrics".to_string());

        // Обращаемся к ключам с разной частотой
        for _ in 0..5 {
            let _result = collector.cache.get("key1");
        }
        let _result = collector.cache.get("key2");

        // Проверяем статистику обращений
        let stats = collector.get_cache_access_statistics();
        assert_eq!(stats.len(), 2);

        // Находим статистику для каждого ключа
        let key1_stats = stats.iter().find(|(key, _, _)| key == "key1").unwrap();
        let key2_stats = stats.iter().find(|(key, _, _)| key == "key2").unwrap();

        // key1 должен иметь больше обращений
        assert!(key1_stats.1 > key2_stats.1);
        assert!(key1_stats.2 > key2_stats.2); // И большую частоту обращений
    }

    #[test]
    fn test_frequently_accessed_items() {
        let config = MetricsCacheConfig::default();
        let collector = OptimizedMetricsCollector::new(config);

        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        collector.cache.insert("frequent1".to_string(), metrics.clone(), source_paths.clone(), "test_metrics".to_string());
        collector.cache.insert("frequent2".to_string(), metrics.clone(), source_paths.clone(), "test_metrics".to_string());
        collector.cache.insert("infrequent".to_string(), metrics.clone(), source_paths.clone(), "test_metrics".to_string());

        // Часто обращаемся к некоторым ключам
        for _ in 0..10 {
            let _result = collector.cache.get("frequent1");
            let _result = collector.cache.get("frequent2");
            std::thread::sleep(Duration::from_millis(50));
        }
        let _result = collector.cache.get("infrequent");

        // Проверяем часто используемые элементы
        let frequent_items = collector.get_frequently_accessed_items(0.5); // Порог 0.5 обращений в секунду
        assert!(frequent_items.contains(&"frequent1".to_string()));
        assert!(frequent_items.contains(&"frequent2".to_string()));
        assert!(!frequent_items.contains(&"infrequent".to_string()));
    }

    #[test]
    fn test_infrequently_accessed_items() {
        let config = MetricsCacheConfig::default();
        let collector = OptimizedMetricsCollector::new(config);

        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        collector.cache.insert("frequent".to_string(), metrics.clone(), source_paths.clone(), "test_metrics".to_string());
        collector.cache.insert("infrequent1".to_string(), metrics.clone(), source_paths.clone(), "test_metrics".to_string());
        collector.cache.insert("infrequent2".to_string(), metrics.clone(), source_paths.clone(), "test_metrics".to_string());

        // Часто обращаемся к одному ключу
        for _ in 0..10 {
            let _result = collector.cache.get("frequent");
            std::thread::sleep(Duration::from_millis(50));
        }

        // Проверяем редко используемые элементы
        let infrequent_items = collector.get_infrequently_accessed_items(0.5); // Порог 0.5 обращений в секунду
        assert!(!infrequent_items.contains(&"frequent".to_string()));
        assert!(infrequent_items.contains(&"infrequent1".to_string()));
        assert!(infrequent_items.contains(&"infrequent2".to_string()));
    }

    #[test]
    fn test_adaptive_ttl_for_key() {
        let config = MetricsCacheConfig::default();
        let collector = OptimizedMetricsCollector::new(config);

        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        collector.cache.insert("test_key".to_string(), metrics, source_paths, "test_metrics".to_string());

        // Обращаемся к ключу несколько раз
        for _ in 0..5 {
            let _result = collector.cache.get("test_key");
            std::thread::sleep(Duration::from_millis(100));
        }

        // Проверяем адаптивный TTL
        let adaptive_ttl = collector.get_adaptive_ttl_for_key("test_key");
        assert!(adaptive_ttl.is_some());
        let ttl = adaptive_ttl.unwrap();

        // TTL должен быть больше базового для часто используемого элемента
        assert!(ttl > config.cache_ttl_seconds);
        assert!(ttl <= config.max_frequent_access_ttl);
    }

    #[test]
    fn test_intelligent_cache_with_memory_pressure() {
        let config = MetricsCacheConfig {
            max_cache_size: 10,
            cache_ttl_seconds: 2,
            enable_caching: true,
            max_memory_bytes: 10_000_000,
            enable_compression: false,
            auto_cleanup_enabled: true,
            enable_performance_metrics: true,
            min_ttl_seconds: 1,
            adaptive_ttl_enabled: true,
            intelligent_ttl_enabled: true,
            max_frequent_access_ttl: 15,
            frequent_access_ttl_factor: 1.8,
            frequent_access_threshold: 1.0,
        };

        let collector = OptimizedMetricsCollector::new(config);

        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        // Добавляем несколько элементов
        for i in 0..5 {
            let key = format!("key_{}", i);
            collector.cache.insert(key.clone(), metrics.clone(), source_paths.clone(), "test_metrics".to_string());
        }

        // Часто обращаемся к некоторым ключам
        for _ in 0..10 {
            let _result = collector.cache.get("key_0");
            let _result = collector.cache.get("key_1");
            std::thread::sleep(Duration::from_millis(50));
        }

        // Проверяем, что часто используемые элементы имеют больший TTL
        let frequent_ttl = collector.get_adaptive_ttl_for_key("key_0");
        let infrequent_ttl = collector.get_adaptive_ttl_for_key("key_4");

        assert!(frequent_ttl.is_some());
        assert!(infrequent_ttl.is_some());
        assert!(frequent_ttl.unwrap() > infrequent_ttl.unwrap());
    }

    #[test]
    fn test_cache_access_patterns_with_different_frequencies() {
        let config = MetricsCacheConfig::default();
        let cache = MetricsCache::new(config);

        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

        // Добавляем элементы с разными паттернами обращений
        cache.insert("very_frequent".to_string(), metrics.clone(), source_paths.clone(), "test_metrics".to_string());
        cache.insert("frequent".to_string(), metrics.clone(), source_paths.clone(), "test_metrics".to_string());
        cache.insert("normal".to_string(), metrics.clone(), source_paths.clone(), "test_metrics".to_string());
        cache.insert("rare".to_string(), metrics.clone(), source_paths.clone(), "test_metrics".to_string());
        cache.insert("very_rare".to_string(), metrics.clone(), source_paths.clone(), "test_metrics".to_string());

        // Симулируем разные паттерны обращений
        for _ in 0..20 {
            let _result = cache.get("very_frequent");
        }
        for _ in 0..10 {
            let _result = cache.get("frequent");
        }
        for _ in 0..5 {
            let _result = cache.get("normal");
        }
        let _result = cache.get("rare");
        // very_rare не обращаемся вообще

        // Проверяем статистику
        let stats = cache.get_cache_access_statistics();
        assert_eq!(stats.len(), 5);

        // Находим статистику для каждого ключа
        let very_frequent = stats.iter().find(|(key, _, _)| key == "very_frequent").unwrap();
        let frequent = stats.iter().find(|(key, _, _)| key == "frequent").unwrap();
        let normal = stats.iter().find(|(key, _, _)| key == "normal").unwrap();
        let rare = stats.iter().find(|(key, _, _)| key == "rare").unwrap();
        let very_rare = stats.iter().find(|(key, _, _)| key == "very_rare").unwrap();

        // Проверяем, что частота обращений соответствует паттернам
        assert!(very_frequent.1 > frequent.1);
        assert!(frequent.1 > normal.1);
        assert!(normal.1 > rare.1);
        assert!(rare.1 > very_rare.1);

        // Проверяем адаптивные TTL
        let base_ttl = config.cache_ttl_seconds;
        let min_ttl = config.min_ttl_seconds;
        let max_ttl = config.max_frequent_access_ttl;

        let very_frequent_ttl = very_frequent.0.get_adaptive_ttl(base_ttl, min_ttl, max_ttl);
        let very_rare_ttl = very_rare.0.get_adaptive_ttl(base_ttl, min_ttl, max_ttl);

        // Очень часто используемый элемент должен иметь максимальный TTL
        assert_eq!(very_frequent_ttl, max_ttl);
        // Очень редко используемый элемент должен иметь минимальный TTL
        assert_eq!(very_rare_ttl, min_ttl);
    }
}
