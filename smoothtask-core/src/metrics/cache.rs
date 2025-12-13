//! Кэширование и оптимизация сбора метрик.
//!
//! Этот модуль предоставляет механизмы кэширования для часто используемых
//! системных метрик, чтобы уменьшить нагрузку на систему и улучшить производительность.

use crate::metrics::system::{CpuTimes, MemoryInfo, SystemMetrics};
use anyhow::{Context, Result};
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
}

impl Default for MetricsCacheConfig {
    fn default() -> Self {
        Self {
            max_cache_size: 100,
            cache_ttl_seconds: 5,
            enable_caching: true,
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
        Self {
            timestamp: Instant::now(),
            metrics,
            source_paths,
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
}

/// Кэш метрик на основе LRU (Least Recently Used).
#[derive(Debug)]
pub struct MetricsCache {
    /// Конфигурация кэширования.
    config: MetricsCacheConfig,
    
    /// LRU кэш для хранения метрик.
    cache: Mutex<LruCache<String, CachedMetrics>>,
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
            return None;
        }
        
        let mut cache = self.cache.lock().unwrap();
        if let Some(cached) = cache.get(key) {
            if !cached.is_expired(self.config.cache_ttl_seconds) {
                return Some(cached.clone());
            }
        }
        
        None
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
            return;
        }
        
        let cached = CachedMetrics::new(metrics, source_paths);
        let mut cache = self.cache.lock().unwrap();
        cache.put(key, cached);
        
        debug!("Сохранено в кэше: {} элементов", cache.len());
    }
    
    /// Очистить кэш.
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
        debug!("Кэш очищен");
    }
    
    /// Получить текущий размер кэша.
    ///
    /// # Возвращает
    ///
    /// Текущий размер кэша.
    pub fn len(&self) -> usize {
        let cache = self.cache.lock().unwrap();
        cache.len()
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
        let metrics = crate::metrics::system::collect_system_metrics(paths)?;
        
        // Создаем карту путей
        let mut source_paths = HashMap::new();
        source_paths.insert("stat".to_string(), paths.stat.clone());
        source_paths.insert("meminfo".to_string(), paths.meminfo.clone());
        source_paths.insert("loadavg".to_string(), paths.loadavg.clone());
        
        // Сохраняем в кэше
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
            .with_context(|| format!("Не удалось прочитать CPU метрики из {}", cpu_path.as_ref().display()))?;
        
        let cpu_times = crate::metrics::system::parse_cpu_times(&cpu_contents)
            .with_context(|| "Не удалось разобрать CPU метрики")?;
        
        // Создаем карту путей
        let mut source_paths = HashMap::new();
        source_paths.insert("stat".to_string(), cpu_path.as_ref().to_path_buf());
        
        // Создаем временные метрики для кэширования
        let metrics = SystemMetrics {
            cpu_times: cpu_times,
            ..Default::default()
        };
        
        // Сохраняем в кэше
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
            .with_context(|| format!("Не удалось прочитать информацию о памяти из {}", meminfo_path.as_ref().display()))?;
        
        let memory_info = crate::metrics::system::parse_meminfo(&meminfo_contents)
            .with_context(|| "Не удалось разобрать информацию о памяти")?;
        
        // Создаем карту путей
        let mut source_paths = HashMap::new();
        source_paths.insert("meminfo".to_string(), meminfo_path.as_ref().to_path_buf());
        
        // Создаем временные метрики для кэширования
        let metrics = SystemMetrics {
            memory: memory_info,
            ..Default::default()
        };
        
        // Сохраняем в кэше
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
            "MetricsCache {{ enabled: {}, size: {}, ttl_seconds: {} }}",
            self.config.enable_caching,
            self.cache.len(),
            self.config.cache_ttl_seconds
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::io::Write;
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
}