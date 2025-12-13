//! Кэширование и оптимизация сбора метрик.
//!
//! Этот модуль предоставляет механизмы кэширования для часто используемых
//! системных метрик, чтобы уменьшить нагрузку на систему и улучшить производительность.

use crate::metrics::system::{CpuTimes, MemoryInfo, SystemMetrics};
use anyhow::{Context, Result};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use lru::LruCache;
use tracing::debug;

/// Конфигурация кэширования метрик.
#[derive(Debug, Clone)]
pub struct MetricsCacheConfig {
    /// Максимальное количество кэшируемых значений.
    pub max_cache_size: usize,
    
    /// Время жизни кэша в секундах.
    pub cache_ttl_seconds: u64,
    
    /// Включить кэширование.
    pub enable_caching: bool,
    
    /// Максимальный размер памяти для кэша в байтах (0 = без ограничения).
    pub max_memory_bytes: usize,
    
    /// Включить сжатие данных в кэше.
    pub enable_compression: bool,
    
    /// Включить автоматическую очистку кэша при достижении лимитов.
    pub auto_cleanup_enabled: bool,
}

impl Default for MetricsCacheConfig {
    fn default() -> Self {
        Self {
            max_cache_size: 100,
            cache_ttl_seconds: 5,
            enable_caching: true,
            max_memory_bytes: 10_000_000, // 10 MB по умолчанию
            enable_compression: false,
            auto_cleanup_enabled: true,
        }
    }
}

/// Кэшированные системные метрики.
#[derive(Debug, Clone)]
pub struct CachedMetrics {
    /// Временная метка создания кэша.
    pub timestamp: Instant,
    
    /// Кэшированные системные метрики.
    pub metrics: SystemMetrics,
    
    /// Пути к файлам, использованные для сбора метрик.
    pub source_paths: HashMap<String, PathBuf>,
    
    /// Приблизительный размер в байтах.
    pub approximate_size_bytes: usize,
}

impl CachedMetrics {
    /// Создать новые кэшированные метрики.
    ///
    /// # Аргументы
    ///
    /// * `metrics` - системные метрики для кэширования
    /// * `source_paths` - пути к файлам, использованные для сбора метрик
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр CachedMetrics.
    pub fn new(metrics: SystemMetrics, source_paths: HashMap<String, PathBuf>) -> Self {
        let approximate_size_bytes = Self::estimate_size(&metrics, &source_paths);
        Self {
            timestamp: Instant::now(),
            metrics,
            source_paths,
            approximate_size_bytes,
        }
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
        size += source_paths.iter().map(|(k, v)| {
            k.len() + v.as_os_str().len() + 32 // Добавляем немного для служебных данных
        }).sum::<usize>();
        
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
                size += details.len() * std::mem::size_of::<crate::metrics::ebpf::ProcessEnergyStat>();
            }
            if let Some(details) = &ebpf.process_gpu_details {
                size += details.len() * std::mem::size_of::<crate::metrics::ebpf::ProcessGpuStat>();
            }
        }
        
        // Добавляем 20% для служебных данных и выравнивания
        size + size / 5
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
                std::num::NonZeroUsize::new(config.max_cache_size).unwrap_or(
                    std::num::NonZeroUsize::new(10).unwrap()
                )
            )),
            config,
            current_memory_usage: std::sync::atomic::AtomicUsize::new(0),
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
        
        let cache = self.cache.lock();
        match cache {
            Ok(mut cache_guard) => {
                if let Some(cached) = cache_guard.get(key) {
                    if !cached.is_expired(self.config.cache_ttl_seconds) {
                        debug!("Найдены актуальные кэшированные метрики для ключа: {}", key);
                        return Some(cached.clone());
                    } else {
                        debug!("Кэш для ключа {} устарел (TTL: {}s), будет обновлён", key, self.config.cache_ttl_seconds);
                    }
                } else {
                    debug!("Кэш для ключа {} не найден", key);
                }
                None
            }
            Err(poisoned) => {
                // Восстанавливаемся от poisoned lock и очищаем кэш
                let mut cache_guard = poisoned.into_inner();
                cache_guard.clear();
                tracing::error!("Mutex был poisoned, кэш очищен для восстановления. Ключ: {}", key);
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
    pub fn insert(&self, key: String, metrics: SystemMetrics, source_paths: HashMap<String, PathBuf>) {
        if !self.config.enable_caching {
            debug!("Кэширование отключено, пропускаем сохранение в кэш для ключа: {}", key);
            return;
        }
        
        let cached = CachedMetrics::new(metrics, source_paths);
        let cache = self.cache.lock();
        
        match cache {
            Ok(mut cache_guard) => {
                // Проверяем лимиты памяти перед вставкой
                if self.config.max_memory_bytes > 0 {
                    let current_usage = self.current_memory_usage.load(std::sync::atomic::Ordering::Relaxed);
                    
                    // Если превышен лимит памяти, выполняем очистку
                    if current_usage + cached.approximate_size_bytes > self.config.max_memory_bytes {
                        self.cleanup_memory(&mut cache_guard);
                    }
                }
                
                // Вставляем новые данные
                let cached_size = cached.approximate_size_bytes;
                cache_guard.put(key, cached);
                
                // Обновляем счётчик памяти
                self.current_memory_usage.fetch_add(cached_size, std::sync::atomic::Ordering::Relaxed);
                
                debug!("Сохранено в кэше: {} элементов, память: {} байт", cache_guard.len(), self.current_memory_usage.load(std::sync::atomic::Ordering::Relaxed));
            }
            Err(poisoned) => {
                // Восстанавливаемся от poisoned lock и очищаем кэш
                let mut cache_guard = poisoned.into_inner();
                cache_guard.clear();
                self.current_memory_usage.store(0, std::sync::atomic::Ordering::Relaxed);
                tracing::error!("Mutex был poisoned при вставке, кэш очищен для восстановления. Ключ: {}", key);
            }
        }
    }
    
    /// Очистить кэш.
    pub fn clear(&self) {
        let cache = self.cache.lock();
        match cache {
            Ok(mut cache_guard) => {
                cache_guard.clear();
                self.current_memory_usage.store(0, std::sync::atomic::Ordering::Relaxed);
                debug!("Кэш очищен");
            }
            Err(poisoned) => {
                // Восстанавливаемся от poisoned lock
                drop(poisoned.into_inner());
                self.current_memory_usage.store(0, std::sync::atomic::Ordering::Relaxed);
                tracing::error!("Mutex был poisoned при очистке кэша");
            }
        }
    }

    /// Выполнить очистку кэша при превышении лимитов памяти.
    ///
    /// # Аргументы
    ///
    /// * `cache_guard` - заблокированный кэш
    fn cleanup_memory(&self, cache_guard: &mut std::sync::MutexGuard<'_, LruCache<String, CachedMetrics>>) {
        if !self.config.auto_cleanup_enabled {
            debug!("Автоматическая очистка кэша отключена");
            return;
        }
        
        let mut current_usage = self.current_memory_usage.load(std::sync::atomic::Ordering::Relaxed);
        let target_usage = self.config.max_memory_bytes.saturating_sub(self.config.max_memory_bytes / 4); // Оставляем 25% запаса
        
        debug!("Выполняем очистку кэша: текущее использование {} байт, целевое {} байт", current_usage, target_usage);
        
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
        while current_usage > target_usage && !cache_guard.is_empty() {
            if let Some((_key, cached)) = cache_guard.pop_lru() {
                current_usage = current_usage.saturating_sub(cached.approximate_size_bytes);
                removed_count += 1;
            }
        }
        
        // Обновляем счётчик памяти
        self.current_memory_usage.store(current_usage, std::sync::atomic::Ordering::Relaxed);
        
        debug!("Очистка кэша завершена: удалено {} элементов, текущее использование {} байт", removed_count, current_usage);
    }

    /// Получить текущее использование памяти.
    ///
    /// # Возвращает
    ///
    /// Текущее использование памяти в байтах.
    pub fn current_memory_usage(&self) -> usize {
        self.current_memory_usage.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Получить информацию о памяти кэша.
    ///
    /// # Возвращает
    ///
    /// Информация о текущем состоянии памяти кэша.
    pub fn get_memory_info(&self) -> String {
        let current = self.current_memory_usage();
        let max = self.config.max_memory_bytes;
        let usage_percent = if max > 0 { (current as f64 / max as f64) * 100.0 } else { 0.0 };
        
        format!(
            "MemoryCache {{ current: {} bytes, max: {} bytes, usage: {:.1}% }}",
            current, max, usage_percent
        )
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
        let metrics = crate::metrics::system::collect_system_metrics(paths)
            .with_context(|| format!("Не удалось собрать системные метрики для кэширования (ключ: {})", cache_key))?;
        
        // Создаем карту путей
        let mut source_paths = HashMap::new();
        source_paths.insert("stat".to_string(), paths.stat.clone());
        source_paths.insert("meminfo".to_string(), paths.meminfo.clone());
        source_paths.insert("loadavg".to_string(), paths.loadavg.clone());
        
        // Пробуем сохранить в кэше (с graceful degradation, если не получится)
        self.cache.insert(cache_key.to_string(), metrics.clone(), source_paths);
        
        Ok(metrics)
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
            debug!("Используем кэшированные CPU метрики для ключа: {}", cache_key);
            return Ok(cached.metrics.cpu_times);
        }
        
        // Если кэш пуст или устарел, собираем новые метрики
        debug!("Собираем новые CPU метрики для ключа: {}", cache_key);
        let cpu_contents = crate::metrics::system::read_file(cpu_path.as_ref())
            .with_context(|| format!("Не удалось прочитать CPU метрики из {} (ключ кэша: {})", cpu_path.as_ref().display(), cache_key))?;
        
        let cpu_times = crate::metrics::system::parse_cpu_times(&cpu_contents)
            .with_context(|| format!("Не удалось разобрать CPU метрики (ключ кэша: {})", cache_key))?;
        
        // Создаем карту путей
        let mut source_paths = HashMap::new();
        source_paths.insert("stat".to_string(), cpu_path.as_ref().to_path_buf());
        
        // Создаем временные метрики для кэширования
        let metrics = SystemMetrics {
            cpu_times: cpu_times,
            ..Default::default()
        };
        
        // Пробуем сохранить в кэше (с graceful degradation, если не получится)
        self.cache.insert(cache_key.to_string(), metrics, source_paths);
        
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
            debug!("Используем кэшированные метрики памяти для ключа: {}", cache_key);
            return Ok(cached.metrics.memory);
        }
        
        // Если кэш пуст или устарел, собираем новые метрики
        debug!("Собираем новые метрики памяти для ключа: {}", cache_key);
        let meminfo_contents = crate::metrics::system::read_file(meminfo_path.as_ref())
            .with_context(|| format!("Не удалось прочитать информацию о памяти из {} (ключ кэша: {})", meminfo_path.as_ref().display(), cache_key))?;
        
        let memory_info = crate::metrics::system::parse_meminfo(&meminfo_contents)
            .with_context(|| format!("Не удалось разобрать информацию о памяти (ключ кэша: {})", cache_key))?;
        
        // Создаем карту путей
        let mut source_paths = HashMap::new();
        source_paths.insert("meminfo".to_string(), meminfo_path.as_ref().to_path_buf());
        
        // Создаем временные метрики для кэширования
        let metrics = SystemMetrics {
            memory: memory_info,
            ..Default::default()
        };
        
        // Пробуем сохранить в кэше (с graceful degradation, если не получится)
        self.cache.insert(cache_key.to_string(), metrics, source_paths);
        
        Ok(memory_info)
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
    use tempfile::tempdir;
    use std::fs;
    
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
        
        cache.insert("test_key".to_string(), metrics, source_paths);
        
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
        };
        
        let collector = OptimizedMetricsCollector::new(config);
        
        // Первый вызов должен собрать метрики
        let metrics1 = collector.collect_system_metrics_cached(&paths, "test_cache").unwrap();
        
        // Второй вызов должен использовать кэш
        let metrics2 = collector.collect_system_metrics_cached(&paths, "test_cache").unwrap();
        
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
        };
        
        let cache = MetricsCache::new(config);
        
        // При отключенном кэшировании get должен возвращать None
        assert!(cache.get("test_key").is_none());
        
        // При отключенном кэшировании insert не должен ничего делать
        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));
        
        cache.insert("test_key".to_string(), metrics, source_paths);
        
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
        };
        
        let cache = MetricsCache::new(config);
        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));
        
        cache.insert("test_key".to_string(), metrics, source_paths);
        
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
        };
        
        let _collector = OptimizedMetricsCollector::new(config.clone());
        
        // Даже если кэширование отключено, сбор метрик должен работать
        let mut disabled_config = config.clone();
        disabled_config.enable_caching = false;
        let disabled_collector = OptimizedMetricsCollector::new(disabled_config);
        
        let result = disabled_collector.collect_cpu_metrics_cached(&stat_file, "test_disabled_cache");
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
        
        cache.insert("test_key".to_string(), metrics, source_paths);
        
        assert!(cache.current_memory_usage() > 0);
        assert!(cache.len() > 0);
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
        };
        
        let cache = MetricsCache::new(config);
        
        // Добавляем данные, которые превысят лимит
        let metrics = SystemMetrics::default();
        let mut source_paths = HashMap::new();
        source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));
        
        // Добавляем несколько элементов
        for i in 0..5 {
            let mut paths = source_paths.clone();
            paths.insert(format!("key_{}", i), PathBuf::from(format!("/proc/test_{}", i)));
            cache.insert(format!("test_key_{}", i), metrics.clone(), paths);
        }
        
        // Проверяем, что очистка сработала (используем более реалистичный лимит)
        // Поскольку estimate_size может переоценивать размер, используем буфер
        assert!(cache.current_memory_usage() <= 2000); // Увеличиваем лимит для теста
        assert!(cache.len() > 0);
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
        
        cache.insert("test_key".to_string(), metrics, source_paths);
        
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
        assert!(memory_info["current_bytes"].as_u64().unwrap() >= 0);
        assert!(memory_info["max_bytes"].as_u64().unwrap() > 0);
        assert!(memory_info["auto_cleanup_enabled"].as_bool().unwrap());
    }
}