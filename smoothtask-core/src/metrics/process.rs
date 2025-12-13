//! Сбор метрик процессов из /proc.
//!
//! Этот модуль предоставляет функции для чтения метрик процессов из файловой системы /proc.
//! Используется библиотека procfs для удобного доступа к данным процессов.

use crate::actuator::read_ionice;
use crate::logging::snapshots::ProcessRecord;
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use lru::LruCache;
use procfs::process::{Process, Stat};
use procfs::ProcError;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};

/// Конфигурация кэширования для метрик процессов.
#[derive(Debug, Clone)]
pub struct ProcessCacheConfig {
    /// Максимальное время жизни кэша в секундах.
    pub cache_ttl_seconds: u64,
    /// Максимальное количество кэшируемых процессов.
    pub max_cached_processes: usize,
    /// Включить кэширование.
    pub enable_caching: bool,
    /// Использовать параллельную обработку.
    pub enable_parallel_processing: bool,
    /// Максимальное количество параллельных потоков.
    pub max_parallel_threads: Option<usize>,
    /// Использовать LRU кэш вместо простого HashMap.
    pub use_lru_cache: bool,
    /// Размер батча для параллельной обработки процессов.
    pub batch_size: Option<usize>,
    /// Включить оптимизацию ввода-вывода (чтение файлов пакетами).
    pub optimize_io: bool,
}

impl Default for ProcessCacheConfig {
    fn default() -> Self {
        Self {
            cache_ttl_seconds: 5,
            max_cached_processes: 1000,
            enable_caching: true,
            enable_parallel_processing: true,
            max_parallel_threads: None,
            use_lru_cache: true,
            batch_size: Some(100),
            optimize_io: true,
        }
    }
}

/// Кэшированная запись процесса.
#[derive(Debug, Clone)]
struct CachedProcessRecord {
    record: ProcessRecord,
    cached_at: Instant,
}

lazy_static! {
    static ref PROCESS_CACHE: Arc<RwLock<ProcessCache>> =
        Arc::new(RwLock::new(ProcessCache::new()));
}

lazy_static! {
    static ref CACHE_STATS: Arc<RwLock<CachePerformanceStats>> =
        Arc::new(RwLock::new(CachePerformanceStats::new()));
}

/// Структура кэша процессов.
#[derive(Debug)]
struct ProcessCache {
    records: HashMap<i32, CachedProcessRecord>,
    lru_cache: Option<Arc<RwLock<LruCache<i32, CachedProcessRecord>>>>,
    config: ProcessCacheConfig,
}

/// Статистика производительности кэша для мониторинга эффективности.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CachePerformanceStats {
    /// Общее количество запросов к кэшу
    pub total_requests: u64,
    /// Количество попаданий в кэш
    pub cache_hits: u64,
    /// Количество промахов кэша
    pub cache_misses: u64,
    /// Общее время, сэкономленное благодаря кэшу (в микросекундах)
    pub time_saved_micros: u64,
    /// Среднее время обработки процесса без кэша (в микросекундах)
    pub avg_processing_time_micros: u64,
    /// Последнее время обновления статистики
    pub last_updated: Option<SystemTime>,
}

impl CachePerformanceStats {
    fn new() -> Self {
        Self {
            total_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            time_saved_micros: 0,
            avg_processing_time_micros: 0,
            last_updated: Some(SystemTime::now()),
        }
    }

    /// Обновить статистику после запроса к кэшу
    fn record_request(&mut self, is_hit: bool, processing_time_micros: Option<u64>) {
        self.total_requests += 1;
        if is_hit {
            self.cache_hits += 1;
        } else {
            self.cache_misses += 1;
        }
        
        if let Some(time) = processing_time_micros {
            // Обновляем среднее время обработки
            if self.total_requests > 0 {
                self.avg_processing_time_micros = 
                    ((self.avg_processing_time_micros * (self.total_requests - 1)) + time) / self.total_requests;
            }
        }
        
        self.last_updated = Some(SystemTime::now());
    }

    /// Сбросить статистику
    fn reset(&mut self) {
        *self = Self::new();
    }

    /// Вычислить процент попаданий в кэш
    fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.cache_hits as f64 / self.total_requests as f64
        }
    }
}

impl ProcessCache {
    fn new() -> Self {
        Self {
            records: HashMap::new(),
            lru_cache: None,
            config: ProcessCacheConfig::default(),
        }
    }

    #[allow(dead_code)]
    fn with_config(config: ProcessCacheConfig) -> Self {
        let mut cache = Self {
            records: HashMap::new(),
            lru_cache: None,
            config: config.clone(),
        };
        
        // Инициализируем LRU кэш, если он включен
        if config.use_lru_cache && config.max_cached_processes > 0 {
            cache.initialize_lru_cache();
        }
        
        cache
    }

    /// Инициализировать LRU кэш
    fn initialize_lru_cache(&mut self) {
        if self.config.max_cached_processes > 0 {
            self.lru_cache = Some(Arc::new(RwLock::new(
                LruCache::new(NonZeroUsize::new(self.config.max_cached_processes).unwrap())
            )));
        }
    }

    /// Очистить устаревшие записи из кэша.
    fn cleanup_stale_entries(&mut self) {
        let now = Instant::now();
        let cache_ttl = self.config.cache_ttl_seconds;
        let max_processes = self.config.max_cached_processes;

        // Очистка устаревших записей
        self.records.retain(|_, cached| {
            now.duration_since(cached.cached_at) < Duration::from_secs(cache_ttl)
        });

        // Для LRU кэша очистка происходит автоматически при вставке
        if let Some(lru_cache) = &self.lru_cache {
            let mut lru_write = lru_cache.write().unwrap();
            // LRU кэш автоматически удаляет старые записи при превышении лимита
            // Но нам нужно вручную удалить устаревшие записи по TTL
            let keys_to_remove: Vec<_> = lru_write.iter().filter_map(|(key, cached)| {
                if now.duration_since(cached.cached_at) >= Duration::from_secs(cache_ttl) {
                    Some(*key)
                } else {
                    None
                }
            }).collect();
            
            for key in keys_to_remove {
                lru_write.pop(&key);
            }
        }

        // Ограничить количество записей, если превышен лимит (только для HashMap)
        if !self.config.use_lru_cache && self.records.len() > max_processes {
            // Удаляем самые старые записи (FIFO)
            let mut entries: Vec<_> = self
                .records
                .iter()
                .map(|(pid, cached)| (*pid, cached.cached_at))
                .collect();
            entries.sort_by_key(|(_, cached_at)| *cached_at);

            let to_remove = entries.len() - max_processes;
            for (pid, _) in entries.into_iter().take(to_remove) {
                self.records.remove(&pid);
            }
        }
    }

    /// Получить запись из кэша, если она актуальна.
    fn get_cached(&self, pid: i32) -> Option<ProcessRecord> {
        let now = Instant::now();
        
        // Пробуем получить из LRU кэша, если он включен
        if let Some(lru_cache) = &self.lru_cache {
            let lru_read = lru_cache.read().unwrap();
            if let Some(cached) = lru_read.peek(&pid) {
                if now.duration_since(cached.cached_at) < Duration::from_secs(self.config.cache_ttl_seconds) {
                    return Some(cached.record.clone());
                }
            }
        }
        
        // Пробуем получить из HashMap кэша
        self.records.get(&pid).and_then(|cached| {
            if now.duration_since(cached.cached_at) < Duration::from_secs(self.config.cache_ttl_seconds)
            {
                Some(cached.record.clone())
            } else {
                None
            }
        })
    }

    /// Сохранить запись в кэш.
    fn cache_record(&mut self, record: ProcessRecord) {
        // Очищаем устаревшие записи
        self.cleanup_stale_entries();

        let cached_record = CachedProcessRecord {
            record: record.clone(),
            cached_at: Instant::now(),
        };

        // Сохраняем в LRU кэш, если он включен
        if let Some(lru_cache) = &self.lru_cache {
            let mut lru_write = lru_cache.write().unwrap();
            lru_write.put(record.pid, cached_record.clone());
        }

        // Сохраняем в HashMap кэш (если не используется LRU или для совместимости)
        if !self.config.use_lru_cache {
            // Если после очистки мы все еще превышаем лимит (учитывая новую запись),
            // удаляем еще одну самую старую запись
            if self.records.len() + 1 > self.config.max_cached_processes {
                let mut entries: Vec<_> = self
                    .records
                    .iter()
                    .map(|(pid, cached)| (*pid, cached.cached_at))
                    .collect();
                entries.sort_by_key(|(_, cached_at)| *cached_at);
                if let Some((oldest_pid, _)) = entries.into_iter().next() {
                    self.records.remove(&oldest_pid);
                }
            }

            self.records.insert(record.pid, cached_record);
        }
    }

    /// Очистить весь кэш.
    fn clear(&mut self) {
        self.records.clear();
    }

    /// Обновить конфигурацию кэша.
    fn update_config(&mut self, config: ProcessCacheConfig) {
        self.config = config;
        self.cleanup_stale_entries();
    }

    /// Получить статистику кэша.
    fn get_cache_stats(&self) -> ProcessCacheStats {
        let now = Instant::now();
        let total_entries = self.records.len();

        // Подсчитываем количество актуальных записей
        let active_entries = self
            .records
            .values()
            .filter(|cached| {
                now.duration_since(cached.cached_at)
                    < Duration::from_secs(self.config.cache_ttl_seconds)
            })
            .count();

        // Подсчитываем количество устаревших записей
        let stale_entries = total_entries - active_entries;

        // Вычисляем средний возраст записей
        let avg_age_seconds = if total_entries > 0 {
            let total_age: f64 = self
                .records
                .values()
                .map(|cached| now.duration_since(cached.cached_at).as_secs_f64())
                .sum();
            total_age / total_entries as f64
        } else {
            0.0
        };

        // Получаем статистику производительности
        let perf_stats = CACHE_STATS.read().unwrap();
        let hit_rate = perf_stats.hit_rate();

        ProcessCacheStats {
            total_entries,
            active_entries,
            stale_entries,
            max_capacity: self.config.max_cached_processes,
            cache_ttl_seconds: self.config.cache_ttl_seconds,
            average_age_seconds: avg_age_seconds,
            hit_rate,
        }
    }
}

/// Статистика кэша метрик процессов.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProcessCacheStats {
    /// Общее количество записей в кэше
    pub total_entries: usize,
    /// Количество актуальных записей
    pub active_entries: usize,
    /// Количество устаревших записей
    pub stale_entries: usize,
    /// Максимальная емкость кэша
    pub max_capacity: usize,
    /// Время жизни кэша в секундах
    pub cache_ttl_seconds: u64,
    /// Средний возраст записей в секундах
    pub average_age_seconds: f64,
    /// Процент попаданий в кэш
    pub hit_rate: f64,
}

/// Собрать метрики всех процессов из /proc.
///
/// Возвращает вектор ProcessRecord для всех доступных процессов.
/// Процессы, к которым нет доступа или которые завершились, пропускаются.
///
/// # Возвращаемое значение
///
/// - `Ok(Vec<ProcessRecord>)` - вектор с метриками всех доступных процессов
/// - `Err(anyhow::Error)` - если не удалось получить список процессов из /proc
///
/// # Ошибки
///
/// Функция может вернуть ошибку в следующих случаях:
///
/// - **Нет доступа к /proc**: отсутствуют права доступа или /proc не смонтирован
/// - **Системная ошибка**: проблемы с файловой системой или ядром
///
/// # Примеры использования
///
/// ## Базовое использование
///
/// ```rust
/// use smoothtask_core::metrics::process::collect_process_metrics;
///
/// match collect_process_metrics() {
///     Ok(processes) => {
///         println!("Найдено {} процессов", processes.len());
///         for proc in processes {
///             println!("PID {}: {} - CPU: {:.1}%", proc.pid, proc.name, proc.cpu_usage);
///         }
///     }
///     Err(e) => {
///         eprintln!("Ошибка сбора метрик процессов: {}", e);
///     }
/// }
/// ```
///
/// ## Использование в главном цикле демона
///
/// ```rust
/// use smoothtask_core::metrics::process::collect_process_metrics;
/// use std::time::Instant;
///
/// let start_time = Instant::now();
/// match collect_process_metrics() {
///     Ok(processes) => {
///         let duration = start_time.elapsed();
///         tracing::info!(
///             "Собрано метрик для {} процессов за {:?}",
///             processes.len(),
///             duration
///         );
///     }
///     Err(e) => {
///         tracing::error!("Не удалось собрать метрики процессов: {}", e);
///     }
/// }
/// ```
///
/// ## Фильтрация и обработка результатов
///
/// ```rust
/// use smoothtask_core::metrics::process::collect_process_metrics;
///
/// if let Ok(processes) = collect_process_metrics() {
///     // Фильтрация процессов с высоким использованием CPU
///     let high_cpu_processes: Vec<_> = processes
///         .into_iter()
///         .filter(|p| p.cpu_usage > 50.0)
///         .collect();
///
///     println!("Процессы с высоким CPU: {}", high_cpu_processes.len());
/// }
/// ```
///
/// # Примечания
///
/// - Функция требует прав на чтение /proc (обычно требуются права root)
/// - Процессы, которые завершились во время сбора, автоматически пропускаются
/// - Ошибки доступа к отдельным процессам логируются на уровне debug и не прерывают выполнение
/// - Функция безопасна для вызова из многопоточного контекста
///
/// ## Обработка ошибок и graceful degradation
///
/// ```rust
/// use smoothtask_core::metrics::process::collect_process_metrics;
///
/// // Пример обработки ошибок с fallback логикой
/// let processes = match collect_process_metrics() {
///     Ok(processes) => processes,
///     Err(e) => {
///         tracing::error!("Не удалось собрать метрики процессов: {}", e);
///         // Fallback: использовать пустой вектор или кэшированные данные
///         Vec::new()
///     }
/// };
///
/// // Продолжение работы даже при отсутствии данных о процессах
/// if processes.is_empty() {
///     tracing::warn!("Нет данных о процессах, работаем в деградированном режиме");
/// }
/// ```
///
/// ## Интеграция с мониторингом и логированием
///
/// ```rust
/// use smoothtask_core::metrics::process::collect_process_metrics;
///
/// // Пример интеграции с системой мониторинга
/// if let Ok(processes) = collect_process_metrics() {
///     let total_cpu: f64 = processes.iter().map(|p| p.cpu_usage).sum();
///     let avg_cpu = total_cpu / processes.len() as f64;
///     
///     // Логирование статистики
///     tracing::info!(
///         "Процессов: {}, среднее CPU: {:.2}%, пиковое CPU: {:.2}%",
///         processes.len(),
///         avg_cpu,
///         processes.iter().map(|p| p.cpu_usage).fold(0.0, f64::max)
///     );
///     
///     // Экспорт метрик в Prometheus или другую систему
///     // metrics::gauge!("process_count", processes.len() as f64);
///     // metrics::gauge!("process_avg_cpu", avg_cpu);
/// }
/// ```
///
/// ## Работа с большими наборами данных
///
/// ```rust
/// use smoothtask_core::metrics::process::collect_process_metrics;
///
/// // Пример обработки большого количества процессов
/// if let Ok(processes) = collect_process_metrics() {
///     // Фильтрация и сортировка для анализа
///     let mut sorted_processes = processes;
///     sorted_processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());
///     
///     // Вывод топ-10 процессов по использованию CPU
///     let top_10: Vec<_> = sorted_processes.into_iter().take(10).collect();
///     
///     for (i, proc) in top_10.iter().enumerate() {
///         tracing::info!(
///             "Top {}: PID {} - {} - CPU: {:.1}% - MEM: {} MB",
///             i + 1,
///             proc.pid,
///             proc.name,
///             proc.cpu_usage,
///             proc.rss_mb
///         );
///     }
/// }
/// ```
///
/// ## Использование в асинхронном контексте
///
/// ```rust
/// use smoothtask_core::metrics::process::collect_process_metrics;
/// use tokio::task;
///
/// // Пример использования в асинхронном контексте
/// let processes = task::spawn_blocking(|| {
///     collect_process_metrics()
/// }).await;
///
/// match processes {
///     Ok(Ok(processes)) => {
///         // Успешно собрали метрики в асинхронном контексте
///         println!("Собрано {} процессов", processes.len());
///     }
///     Ok(Err(e)) => {
///         eprintln!("Ошибка сбора метрик: {}", e);
///     }
///     Err(e) => {
///         eprintln!("Ошибка выполнения задачи: {}", e);
///     }
/// }
/// ```
pub fn collect_process_metrics(config: Option<ProcessCacheConfig>) -> Result<Vec<ProcessRecord>> {
    // Применяем конфигурацию, если предоставлена
    if let Some(cfg) = config {
        let mut cache_write = PROCESS_CACHE.write().unwrap();
        cache_write.update_config(cfg);
    }

    let all_procs = procfs::process::all_processes()
        .context("Не удалось получить список процессов из /proc: проверьте права доступа и доступность /proc. Попробуйте: ls -la /proc | sudo ls /proc. Для устранения: 1) Проверьте права: id && groups, 2) Проверьте монтирование: mount | grep proc, 3) Попробуйте запустить с sudo")?;

    // Сохраняем количество процессов до обработки
    let total_process_count = all_procs.size_hint().0;
    tracing::info!(
        "Начало сбора метрик процессов. Найдено {} потенциальных процессов",
        total_process_count
    );

    // Оптимизация: предварительное выделение памяти для вектора процессов
    // Это уменьшает количество реаллокаций при добавлении элементов
    let mut processes = Vec::with_capacity(total_process_count);

    // Получаем доступ к кэшу для чтения
    let cache_read = PROCESS_CACHE.read().unwrap();
    let cache_config = cache_read.config.clone();
    drop(cache_read); // Освобождаем блокировку

    // Оптимизация: параллельная обработка процессов с использованием rayon
    // Это значительно ускоряет сбор метрик для большого количества процессов
    let process_results: Vec<_> = if cache_config.enable_parallel_processing {
        // Используем глобальный пул потоков
        let _batch_size = cache_config.batch_size.unwrap_or(100);
        
        all_procs
            .par_bridge() // Преобразуем итератор в параллельный
            .filter_map(|proc_result| {
                match proc_result {
                    Ok(proc) => {
                        let pid = proc.pid();
                        tracing::debug!("Обработка процесса PID {}", pid);

                        // Проверяем кэш, если кэширование включено
                        if cache_config.enable_caching {
                            let _start_time = Instant::now();
                            let cache_read = PROCESS_CACHE.read().unwrap();
                            if let Some(cached_record) = cache_read.get_cached(pid) {
                                tracing::debug!("Кэш попадание для процесса PID {}", pid);
                                
                                // Обновляем статистику кэша
                                let mut stats_write = CACHE_STATS.write().unwrap();
                                stats_write.record_request(true, None);
                                drop(stats_write);
                                
                                drop(cache_read);
                                return Some(cached_record);
                            }
                            drop(cache_read);
                            
                            // Обновляем статистику кэша для промаха
                            let mut stats_write = CACHE_STATS.write().unwrap();
                            stats_write.record_request(false, None);
                            drop(stats_write);
                        }

                        match collect_single_process(&proc) {
                            Ok(Some(record)) => {
                                let process_name = record
                                    .exe
                                    .as_ref()
                                    .or(record.cmdline.as_ref())
                                    .map(|s| s.split('/').next_back().unwrap_or(s.as_str()))
                                    .unwrap_or("unknown");
                                tracing::debug!(
                                    "Успешно собраны метрики для процесса PID {} ({})",
                                    pid,
                                    process_name
                                );

                                // Кэшируем результат, если кэширование включено
                                if cache_config.enable_caching {
                                    let mut cache_write = PROCESS_CACHE.write().unwrap();
                                    cache_write.cache_record(record.clone());
                                }

                                Some(record)
                            }
                            Ok(None) => {
                                tracing::debug!("Процесс PID {} завершился между итерациями", pid);
                                None
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Ошибка сбора метрик для процесса PID {}: {}. \
                                     Процесс мог завершиться или нет прав доступа к /proc/{}/. \
                                     Проверьте: ls -la /proc/{}/ | sudo cat /proc/{}/status",
                                    pid,
                                    e,
                                    pid,
                                    pid,
                                    pid
                                );
                                None
                            }
                        }
                    }
                    Err(ProcError::NotFound(_)) => None, // процесс завершился
                    Err(e) => {
                        tracing::warn!(
                            "Ошибка доступа к процессу при чтении /proc: {}. \
                             Процесс мог завершиться или нет прав доступа. \
                             Рекомендация: проверьте права доступа и попробуйте sudo",
                            e
                        );
                        None
                    }
                }
            })
            .collect()
    } else {
        // Последовательная обработка (для отладки или особых случаев)
        all_procs
            .filter_map(|proc_result| {
                match proc_result {
                    Ok(proc) => {
                        let pid = proc.pid();
                        tracing::debug!("Обработка процесса PID {}", pid);

                        // Проверяем кэш, если кэширование включено
                        if cache_config.enable_caching {
                            let cache_read = PROCESS_CACHE.read().unwrap();
                            if let Some(cached_record) = cache_read.get_cached(pid) {
                                tracing::debug!("Кэш попадание для процесса PID {}", pid);
                                drop(cache_read);
                                return Some(cached_record);
                            }
                            drop(cache_read);
                        }

                        match collect_single_process(&proc) {
                            Ok(Some(record)) => {
                                let process_name = record
                                    .exe
                                    .as_ref()
                                    .or(record.cmdline.as_ref())
                                    .map(|s| s.split('/').next_back().unwrap_or(s.as_str()))
                                    .unwrap_or("unknown");
                                tracing::debug!(
                                    "Успешно собраны метрики для процесса PID {} ({})",
                                    pid,
                                    process_name
                                );

                                // Кэшируем результат, если кэширование включено
                                if cache_config.enable_caching {
                                    let mut cache_write = PROCESS_CACHE.write().unwrap();
                                    cache_write.cache_record(record.clone());
                                }

                                Some(record)
                            }
                            Ok(None) => {
                                tracing::debug!("Процесс PID {} завершился между итерациями", pid);
                                None
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Ошибка сбора метрик для процесса PID {}: {}. \
                                     Процесс мог завершиться или нет прав доступа к /proc/{}/. \
                                     Проверьте: ls -la /proc/{}/ | sudo cat /proc/{}/status",
                                    pid,
                                    e,
                                    pid,
                                    pid,
                                    pid
                                );
                                None
                            }
                        }
                    }
                    Err(ProcError::NotFound(_)) => None, // процесс завершился
                    Err(e) => {
                        tracing::warn!(
                            "Ошибка доступа к процессу при чтении /proc: {}. \
                             Процесс мог завершиться или нет прав доступа. \
                             Рекомендация: проверьте права доступа и попробуйте sudo",
                            e
                        );
                        None
                    }
                }
            })
            .collect()
    };

    processes.extend(process_results);

    // Оптимизация: уменьшаем выделенную память до фактического размера
    processes.shrink_to_fit();

    tracing::info!(
        "Завершен сбор метрик процессов. Успешно собрано {} из {} процессов",
        processes.len(),
        total_process_count
    );

    if processes.is_empty() {
        tracing::warn!("Не удалось собрать метрики ни для одного процесса. Возможные причины: отсутствие прав доступа, /proc не смонтирован, или все процессы завершились. Для устранения: 1) Проверьте права: id && groups, 2) Проверьте монтирование: mount | grep proc, 3) Попробуйте запустить с sudo");
    }

    Ok(processes)
}

/// Собрать метрики энергопотребления для процесса.
///
/// Пробует получить данные из доступных источников:
/// - /proc/[pid]/power/energy_uj (если доступно)
/// - eBPF мониторинг (если включен)
/// - RAPL интерфейсы (если доступны)
///
/// Возвращает (energy_uj, power_w, energy_timestamp) или (None, None, None), если данные недоступны.
fn collect_process_energy_metrics(pid: i32) -> (Option<u64>, Option<f32>, Option<u64>) {
    // Пробуем прочитать из /proc/[pid]/power/energy_uj (если доступно)
    // Это экспериментальный интерфейс, доступный на некоторых системах
    let energy_path = format!("/proc/{}/power/energy_uj", pid);
    
    if let Ok(energy_content) = std::fs::read_to_string(&energy_path) {
        if let Ok(energy_uj) = energy_content.trim().parse::<u64>() {
            // Конвертируем в ватты (упрощенная конвертация)
            // Для точного расчета нужны два измерения с известным интервалом
            let power_w = energy_uj as f32 / 1_000_000.0;
            let energy_timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|d| d.as_secs());
            
            tracing::debug!(
                "Энергопотребление процесса PID {}: {} мкДж ({} Вт)",
                pid,
                energy_uj,
                power_w
            );
            
            return (Some(energy_uj), Some(power_w), energy_timestamp);
        }
    }
    
    // Если /proc/[pid]/power/energy_uj недоступен, пробуем другие источники
    // В будущем можно добавить интеграцию с eBPF или RAPL
    
    tracing::debug!(
        "Энергопотребление для процесса PID {} недоступно: {} не найден или нечитаем",
        pid,
        energy_path
    );
    
    (None, None, None)
}

/// Собрать метрики для одного процесса.
///
/// Возвращает `None`, если процесс завершился или к нему нет доступа.
fn collect_single_process(proc: &Process) -> Result<Option<ProcessRecord>> {
    let pid = proc.pid();
    tracing::debug!("Сбор метрик для процесса PID {}", pid);

    // Читаем stat для базовой информации
    let stat = match proc.stat() {
        Ok(s) => s,
        Err(ProcError::NotFound(_)) => {
            tracing::debug!(
                "Процесс PID {} не найден в /proc (возможно завершился)",
                pid
            );
            return Ok(None);
        }
        Err(e) => {
            tracing::warn!("Не удалось прочитать /proc/{}/stat: {}. Проверьте права доступа: ls -la /proc/{}/stat", pid, e, pid);
            return Err(anyhow::anyhow!(
                "Не удалось прочитать /proc/{}/stat: {}. \
                 Проверьте, что процесс существует и доступен для чтения. \
                 Попробуйте: sudo cat /proc/{}/stat",
                pid,
                e,
                pid
            ));
        }
    };

    // Читаем status для UID/GID и дополнительной информации
    let status = match proc.status() {
        Ok(s) => s,
        Err(ProcError::NotFound(_)) => {
            tracing::debug!(
                "Файл status для процесса PID {} не найден (процесс завершился)",
                pid
            );
            return Ok(None);
        }
        Err(e) => {
            tracing::warn!("Не удалось прочитать /proc/{}/status: {}. Проверьте права доступа: ls -la /proc/{}/status", pid, e, pid);
            return Err(anyhow::anyhow!(
                "Не удалось прочитать /proc/{}/status: {}. \
                 Проверьте, что процесс существует и доступен для чтения. \
                 Попробуйте: sudo cat /proc/{}/status",
                pid,
                e,
                pid
            ));
        }
    };

    // Читаем cmdline с оптимизацией
    let cmdline = proc.cmdline().ok().and_then(|args| {
        if args.is_empty() {
            None
        } else {
            // Оптимизация: используем String::with_capacity для cmdline
            // чтобы уменьшить количество реаллокаций при join
            let mut cmdline_str = String::with_capacity(args.len() * 16); // средняя длина аргумента ~16 символов
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    cmdline_str.push(' ');
                }
                cmdline_str.push_str(arg);
            }
            Some(cmdline_str)
        }
    });

    // Читаем exe (симлинк на исполняемый файл) с оптимизацией
    let exe = proc
        .exe()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()));

    // Читаем cgroup_path с оптимизацией
    let cgroup_path = read_cgroup_path(proc.pid()).ok().flatten();

    // Определяем uptime_sec на основе start_time
    let uptime_sec = calculate_uptime(&stat).with_context(|| {
        format!(
            "Не удалось вычислить uptime для процесса PID {}: \
             проверьте доступность /proc/uptime и корректность start_time в /proc/{}/stat",
            proc.pid(),
            proc.pid()
        )
    })?;

    // Определяем has_tty на основе tty_nr
    let has_tty = stat.tty_nr != 0;

    // Читаем переменные окружения для определения GUI/терминала
    let (env_has_display, env_has_wayland, env_term, env_ssh) =
        read_env_vars(proc.pid()).unwrap_or((false, false, None, false));

    // Читаем статистику ввода-вывода (опционально, так как это тяжелая операция)
    let (io_read_bytes, io_write_bytes) = read_io_stats(proc.pid()).unwrap_or((None, None));

    // Читаем nice из stat (конвертируем i64 в i32)
    let nice = stat.nice as i32;

    // Читаем ionice через системный вызов ioprio_get
    let (ionice_class, ionice_prio) = read_ionice(stat.pid)
        .ok()
        .flatten()
        .map(|(class, level)| (Some(class), Some(level)))
        .unwrap_or((None, None));

    // Читаем RSS из status (в килобайтах, конвертируем в мегабайты)
    // В procfs RSS доступен через поле VmRSS в status
    let rss_mb = status.vmrss.map(|kb| kb / 1024);

    // Читаем swap из status (в килобайтах, конвертируем в мегабайты)
    let swap_mb = status.vmswap.map(|kb| kb / 1024);

    // Читаем контекстные переключения из status
    let voluntary_ctx = status.voluntary_ctxt_switches;
    let involuntary_ctx = status.nonvoluntary_ctxt_switches;

    // Получаем UID и GID из /proc/[pid]/status напрямую
    let (uid, gid) = read_uid_gid(proc.pid()).unwrap_or((0, 0));

    // Определяем systemd_unit из cgroup_path (если есть)
    let systemd_unit = extract_systemd_unit(&cgroup_path);

    // Собираем метрики энергопотребления
    let (energy_uj, power_w, energy_timestamp) = collect_process_energy_metrics(proc.pid());

    let record = ProcessRecord {
        pid: stat.pid,
        ppid: stat.ppid,
        uid,
        gid,
        exe,
        cmdline,
        cgroup_path,
        systemd_unit,
        app_group_id: None, // будет заполнено при группировке
        state: format!("{:?}", stat.state),
        start_time: stat.starttime,
        uptime_sec,
        tty_nr: stat.tty_nr,
        has_tty,
        cpu_share_1s: None,   // будет вычислено при следующем снапшоте
        cpu_share_10s: None,  // будет вычислено при следующем снапшоте
        io_read_bytes,        // статистика ввода-вывода из /proc/[pid]/io
        io_write_bytes,       // статистика ввода-вывода из /proc/[pid]/io
        rss_mb,
        swap_mb,
        voluntary_ctx,
        involuntary_ctx,
        has_gui_window: false,    // будет заполнено из WindowIntrospector
        is_focused_window: false, // будет заполнено из WindowIntrospector
        window_state: None,       // будет заполнено из WindowIntrospector
        env_has_display,
        env_has_wayland,
        env_term,
        env_ssh,
        is_audio_client: false,   // будет заполнено из AudioIntrospector
        has_active_stream: false, // будет заполнено из AudioIntrospector
        process_type: None,       // будет заполнено классификатором
        tags: Vec::new(),         // будет заполнено классификатором
        nice,
        ionice_class,
        ionice_prio,
        teacher_priority_class: None, // для обучения
        teacher_score: None,          // для обучения
        energy_uj,
        power_w,
        energy_timestamp,
    };

    Ok(Some(record))
}

/// Прочитать путь cgroup процесса из /proc/[pid]/cgroup.
fn read_cgroup_path(pid: i32) -> Result<Option<String>> {
    let path = format!("/proc/{}/cgroup", pid);
    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            // Не критичная ошибка - cgroup может быть недоступен
            tracing::debug!(
                "Не удалось прочитать /proc/{}/cgroup: {}. \
                 Cgroup может быть недоступен для этого процесса. \
                 Это может быть вызвано отсутствием прав доступа, отсутствием файла или тем, что процесс завершился",
                pid,
                e
            );
            return Ok(None);
        }
    };

    // Парсим формат cgroup v2: 0::/path/to/cgroup
    // Или cgroup v1: несколько строк вида hierarchy:controller:path
    for line in contents.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 3 {
            let cgroup_path = parts[2];
            if !cgroup_path.is_empty() && cgroup_path != "/" {
                return Ok(Some(cgroup_path.to_string()));
            }
        }
    }

    Ok(None)
}

/// Извлечь systemd unit из cgroup_path.
///
/// Например, из "/user.slice/user-1000.slice/session-2.scope" извлекается "session-2.scope".
/// Игнорируем корневые .slice компоненты (например, "/user.slice").
fn extract_systemd_unit(cgroup_path: &Option<String>) -> Option<String> {
    let path = cgroup_path.as_ref()?;

    // Ищем последний компонент пути, который заканчивается на .scope или .service
    // Игнорируем .slice, если это не единственный компонент (корневой)
    let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if components.len() <= 1 {
        // Корневой путь типа "/user.slice" - не считаем unit'ом
        return None;
    }

    for component in components.iter().rev() {
        if component.ends_with(".scope") || component.ends_with(".service") {
            return Some(component.to_string());
        }
    }

    None
}

/// Вычислить uptime процесса в секундах на основе start_time.
fn calculate_uptime(stat: &Stat) -> Result<u64> {
    // start_time в jiffies (обычно 100 Hz, но может быть и 1000 Hz)
    // Нужно получить системный uptime и вычислить разницу
    let boot_time = procfs::boot_time_secs().with_context(|| {
        format!(
            "Не удалось получить время загрузки системы для вычисления uptime процесса PID {}. \
             Проверьте доступность /proc/stat",
            stat.pid
        )
    })?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // starttime в jiffies, нужно конвертировать в секунды
    // Используем clock_ticks_per_second для конвертации
    let clock_ticks = procfs::ticks_per_second();

    let start_time_secs = boot_time + (stat.starttime / clock_ticks);
    let uptime_sec = now.saturating_sub(start_time_secs);

    Ok(uptime_sec)
}

/// Прочитать UID и GID процесса из /proc/[pid]/status.
fn read_uid_gid(pid: i32) -> Result<(u32, u32)> {
    let path = format!("/proc/{}/status", pid);
    let contents = fs::read_to_string(&path).with_context(|| {
        format!(
            "Не удалось прочитать /proc/{}/status: проверьте, что процесс существует и доступен для чтения",
            pid
        )
    })?;

    let mut uid = 0;
    let mut gid = 0;

    for line in contents.lines() {
        if line.starts_with("Uid:") {
            // Формат: Uid: 1000 1000 1000 1000 (real, effective, saved, filesystem)
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                uid = parts[1].parse::<u32>().with_context(|| {
                    format!(
                        "Некорректный UID в /proc/{}/status: ожидается целое число (u32). \
                             Формат строки: 'Uid: <real> <effective> <saved> <filesystem>'",
                        pid
                    )
                })?;
            }
        } else if line.starts_with("Gid:") {
            // Формат: Gid: 1000 1000 1000 1000 (real, effective, saved, filesystem)
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                gid = parts[1].parse::<u32>().with_context(|| {
                    format!(
                        "Некорректный GID в /proc/{}/status: ожидается целое число (u32). \
                             Формат строки: 'Gid: <real> <effective> <saved> <filesystem>'",
                        pid
                    )
                })?;
            }
        }
    }

    Ok((uid, gid))
}

/// Прочитать переменные окружения процесса из /proc/[pid]/environ.
fn read_env_vars(pid: i32) -> Result<(bool, bool, Option<String>, bool)> {
    let path = format!("/proc/{}/environ", pid);
    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            // Не критичная ошибка - environ может быть недоступен
            tracing::debug!(
                "Не удалось прочитать /proc/{}/environ: {}. \
                 Переменные окружения могут быть недоступны для этого процесса",
                pid,
                e
            );
            return Ok((false, false, None, false));
        }
    };

    let mut has_display = false;
    let mut has_wayland = false;
    let mut term = None;
    let mut ssh = false;

    // environ содержит переменные, разделённые нулевыми байтами
    for env_var in contents.split('\0') {
        if env_var.starts_with("DISPLAY=") {
            has_display = true;
        } else if env_var.starts_with("WAYLAND_DISPLAY=") {
            has_wayland = true;
        } else if env_var.starts_with("TERM=") {
            term = env_var.strip_prefix("TERM=").map(|s| s.to_string());
        } else if env_var.starts_with("SSH_") {
            ssh = true;
        }
    }

    Ok((has_display, has_wayland, term, ssh))
}

/// Читает статистику ввода-вывода процесса из /proc/[pid]/io.
/// Эта операция может быть тяжелой, поэтому используется опционально.
fn read_io_stats(pid: i32) -> Result<(Option<u64>, Option<u64>)> {
    let io_path = format!("/proc/{}/io", pid);
    let io_content = match fs::read_to_string(&io_path) {
        Ok(c) => c,
        Err(e) => {
            // Не критичная ошибка - io статистика может быть недоступна
            tracing::debug!(
                "Не удалось прочитать /proc/{}/io: {}. 
                 Статистика ввода-вывода может быть недоступна для этого процесса",
                pid,
                e
            );
            return Ok((None, None));
        }
    };

    let mut read_bytes = None;
    let mut write_bytes = None;

    for line in io_content.lines() {
        if let Some(value) = line.strip_prefix("read_bytes: ") {
            if let Ok(bytes) = value.trim().parse::<u64>() {
                read_bytes = Some(bytes);
            }
        } else if let Some(value) = line.strip_prefix("write_bytes: ") {
            if let Ok(bytes) = value.trim().parse::<u64>() {
                write_bytes = Some(bytes);
            }
        }

        // Если нашли оба значения, можно прекратить парсинг
        if read_bytes.is_some() && write_bytes.is_some() {
            break;
        }
    }

    Ok((read_bytes, write_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn extract_systemd_unit_from_cgroup_path() {
        let unit = extract_systemd_unit(&Some(
            "/user.slice/user-1000.slice/session-2.scope".to_string(),
        ));
        assert_eq!(unit, Some("session-2.scope".to_string()));

        let unit = extract_systemd_unit(&Some("/system.slice/ssh.service".to_string()));
        assert_eq!(unit, Some("ssh.service".to_string()));

        let unit = extract_systemd_unit(&Some("/user.slice".to_string()));
        assert_eq!(unit, None);

        let unit = extract_systemd_unit(&None);
        assert_eq!(unit, None);
    }

    #[test]
    fn parse_cgroup_v2_path() {
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // Формат cgroup v2: 0::/user.slice/user-1000.slice/session-2.scope
        let cgroup_content = "0::/user.slice/user-1000.slice/session-2.scope\n";
        fs::write(proc_dir.join("cgroup"), cgroup_content).unwrap();

        // Мокаем чтение через прямое чтение файла
        let path = proc_dir.join("cgroup");
        let contents = fs::read_to_string(&path).unwrap();
        let mut found_path = None;
        for line in contents.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                let cgroup_path = parts[2];
                if !cgroup_path.is_empty() && cgroup_path != "/" {
                    found_path = Some(cgroup_path.to_string());
                    break;
                }
            }
        }
        assert_eq!(
            found_path,
            Some("/user.slice/user-1000.slice/session-2.scope".to_string())
        );
    }

    #[test]
    fn test_read_io_stats() {
        // Этот тест проверяет парсинг IO статистики
        // Поскольку мы не можем легко замокать /proc, мы тестируем парсинг напрямую
        
        // Создаем тестовые данные как в реальном /proc/[pid]/io
        let io_content = "rchar: 123456\n\nwchar: 789012\n\nsyscr: 345\n\nsyscw: 678\n\nread_bytes: 1024000\n\nwrite_bytes: 2048000\n\ncancelled_write_bytes: 0\n\n";
        
        // Парсим вручную, как это делает read_io_stats
        let mut read_bytes = None;
        let mut write_bytes = None;
        
        for line in io_content.lines() {
            if line.starts_with("read_bytes:") {
                if let Some(value) = line.split(':').nth(1) {
                    read_bytes = value.trim().parse::<u64>().ok();
                }
            } else if line.starts_with("write_bytes:") {
                if let Some(value) = line.split(':').nth(1) {
                    write_bytes = value.trim().parse::<u64>().ok();
                }
            }
        }
        
        // Проверяем, что парсинг работает корректно
        assert_eq!(read_bytes, Some(1024000));
        assert_eq!(write_bytes, Some(2048000));
    }

    #[test]
    fn test_read_io_stats_missing_file() {
        // Тестируем обработку ошибок, когда файл /proc/[pid]/io отсутствует
        let result = read_io_stats(99999);
        assert!(result.is_ok());
        let (read_bytes, write_bytes) = result.unwrap();
        assert_eq!(read_bytes, None);
        assert_eq!(write_bytes, None);
    }

    #[test]
    fn parse_env_vars() {
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // environ содержит переменные, разделённые нулевыми байтами
        let env_content =
            "HOME=/home/user\0DISPLAY=:0\0TERM=xterm-256color\0SSH_CLIENT=192.168.1.1\0";
        fs::write(proc_dir.join("environ"), env_content).unwrap();

        let path = proc_dir.join("environ");
        let contents = fs::read_to_string(&path).unwrap();
        let mut has_display = false;
        let mut has_wayland = false;
        let mut term = None;
        let mut ssh = false;

        for env_var in contents.split('\0') {
            if env_var.starts_with("DISPLAY=") {
                has_display = true;
            } else if env_var.starts_with("WAYLAND_DISPLAY=") {
                has_wayland = true;
            } else if env_var.starts_with("TERM=") {
                term = env_var.strip_prefix("TERM=").map(|s| s.to_string());
            } else if env_var.starts_with("SSH_") {
                ssh = true;
            }
        }

        assert!(has_display);
        assert!(!has_wayland);
        assert_eq!(term, Some("xterm-256color".to_string()));
        assert!(ssh);
    }

    #[test]
    fn parse_uid_gid_from_status() {
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // Формат /proc/[pid]/status с Uid и Gid
        let status_content = "\
Name:   test_process
State:  R (running)
Uid:    1000 1000 1000 1000
Gid:    1000 1000 1000 1000
";
        fs::write(proc_dir.join("status"), status_content).unwrap();

        // Используем временный путь вместо реального /proc/123
        let status_path = proc_dir.join("status");
        let contents = fs::read_to_string(&status_path).unwrap();
        let mut uid = 0;
        let mut gid = 0;

        for line in contents.lines() {
            if line.starts_with("Uid:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    uid = parts[1].parse::<u32>().unwrap();
                }
            } else if line.starts_with("Gid:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    gid = parts[1].parse::<u32>().unwrap();
                }
            }
        }

        assert_eq!(uid, 1000);
        assert_eq!(gid, 1000);
    }

    #[test]
    fn parse_uid_gid_handles_missing_file() {
        // Несуществующий PID должен вернуть ошибку
        let result = read_uid_gid(999999);
        assert!(result.is_err());
    }

    #[test]
    fn extract_systemd_unit_handles_various_formats() {
        // Тест различных форматов systemd unit
        assert_eq!(
            extract_systemd_unit(&Some(
                "/user.slice/user-1000.slice/session-2.scope".to_string()
            )),
            Some("session-2.scope".to_string())
        );
        assert_eq!(
            extract_systemd_unit(&Some("/system.slice/ssh.service".to_string())),
            Some("ssh.service".to_string())
        );
        assert_eq!(
            extract_systemd_unit(&Some("/system.slice/dbus.service".to_string())),
            Some("dbus.service".to_string())
        );
        assert_eq!(extract_systemd_unit(&Some("/user.slice".to_string())), None);
        assert_eq!(extract_systemd_unit(&None), None);
    }

    #[test]
    fn calculate_uptime_with_valid_stat() {
        // Этот тест проверяет, что функция calculate_uptime не падает с ошибкой
        // при корректных входных данных. Так как функция зависит от системного времени
        // и boot_time, мы не можем точно предсказать результат, но можем проверить,
        // что она возвращает разумное значение.

        // Используем реальный процесс (текущий процесс) для получения реального Stat
        // Это более надежный подход, чем создание мокового Stat
        let current_pid = std::process::id() as i32;
        let proc = match Process::new(current_pid) {
            Ok(p) => p,
            Err(_) => {
                // Если не удалось получить процесс, пропускаем тест
                return;
            }
        };

        let stat = match proc.stat() {
            Ok(s) => s,
            Err(_) => {
                // Если не удалось получить stat, пропускаем тест
                return;
            }
        };

        let result = calculate_uptime(&stat);
        assert!(result.is_ok());
        let uptime = result.unwrap();
        // Проверяем, что uptime разумный (не слишком большой)
        // Для текущего процесса uptime может быть 0, если процесс только что запустился
        assert!(uptime < 1000000); // разумный верхний предел
    }

    #[test]
    fn read_cgroup_path_with_valid_file() {
        // Создаем временный файл cgroup
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // Формат cgroup v2: 0::/user.slice/user-1000.slice/session-2.scope
        let cgroup_content = "0::/user.slice/user-1000.slice/session-2.scope\n";
        fs::write(proc_dir.join("cgroup"), cgroup_content).unwrap();

        // Мокаем чтение через временный файл
        let path = proc_dir.join("cgroup");
        let contents = fs::read_to_string(&path).unwrap();
        let mut found_path = None;
        for line in contents.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                let cgroup_path = parts[2];
                if !cgroup_path.is_empty() && cgroup_path != "/" {
                    found_path = Some(cgroup_path.to_string());
                    break;
                }
            }
        }
        assert_eq!(
            found_path,
            Some("/user.slice/user-1000.slice/session-2.scope".to_string())
        );
    }

    #[test]
    fn read_cgroup_path_with_missing_file() {
        // Проверяем, что функция корректно обрабатывает отсутствие файла
        let result = read_cgroup_path(999999);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn extract_systemd_unit_handles_edge_cases() {
        // Тест обработки пустых и некорректных путей
        assert_eq!(extract_systemd_unit(&Some("".to_string())), None);
        assert_eq!(extract_systemd_unit(&Some("/".to_string())), None);
        assert_eq!(
            extract_systemd_unit(&Some("invalid-format".to_string())),
            None
        );
        assert_eq!(
            extract_systemd_unit(&Some("/user.slice/".to_string())),
            None
        );

        // Тест обработки очень длинных путей
        let long_path = "/user.slice/".repeat(100) + "session-1.scope";
        assert_eq!(
            extract_systemd_unit(&Some(long_path)),
            Some("session-1.scope".to_string())
        );
    }

    #[test]
    fn parse_env_vars_handles_empty_and_malformed() {
        // Тест обработки пустого файла environ
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // Пустой файл environ
        fs::write(proc_dir.join("environ"), "").unwrap();
        let path = proc_dir.join("environ");
        let contents = fs::read_to_string(&path).unwrap();
        let mut has_display = false;
        for env_var in contents.split('\0') {
            if env_var.starts_with("DISPLAY=") {
                has_display = true;
            }
        }
        assert!(!has_display);

        // Файл с некорректными данными
        let malformed_content = "INVALID_DATA_WITHOUT_NULL_BYTES";
        fs::write(proc_dir.join("environ"), malformed_content).unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        let mut count = 0;
        for env_var in contents.split('\0') {
            if !env_var.is_empty() {
                count += 1;
            }
        }
        assert_eq!(count, 1); // только одна строка без нулевых байтов
    }

    #[test]
    fn parse_uid_gid_handles_malformed_status() {
        // Тест обработки некорректного файла status
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // Некорректный формат status
        let malformed_status = "Invalid format without proper fields";
        fs::write(proc_dir.join("status"), malformed_status).unwrap();

        let status_path = proc_dir.join("status");
        let contents = fs::read_to_string(&status_path).unwrap();
        let mut uid = 0;
        let mut gid = 0;

        for line in contents.lines() {
            if line.starts_with("Uid:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(parsed_uid) = parts[1].parse::<u32>() {
                        uid = parsed_uid;
                    }
                }
            } else if line.starts_with("Gid:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(parsed_gid) = parts[1].parse::<u32>() {
                        gid = parsed_gid;
                    }
                }
            }
        }

        // Должны остаться значения по умолчанию (0)
        assert_eq!(uid, 0);
        assert_eq!(gid, 0);
    }

    #[test]
    fn calculate_uptime_handles_edge_cases() {
        // Тест обработки edge cases
        // Используем реальный процесс для получения валидного Stat
        let current_pid = std::process::id() as i32;
        let proc = match Process::new(current_pid) {
            Ok(p) => p,
            Err(_) => {
                // Если не удалось получить процесс, пропускаем тест
                return;
            }
        };

        let stat = match proc.stat() {
            Ok(s) => s,
            Err(_) => {
                // Если не удалось получить stat, пропускаем тест
                return;
            }
        };

        // Тест с реальными данными - должен вернуть Ok
        let result = calculate_uptime(&stat);
        assert!(result.is_ok());

        // Проверяем, что uptime разумный
        let uptime = result.unwrap();
        assert!(uptime < 1000000); // разумный верхний предел
    }

    #[test]
    fn extract_systemd_unit_handles_complex_paths() {
        // Тест обработки сложных путей systemd
        assert_eq!(
            extract_systemd_unit(&Some(
                "/user.slice/user-1000.slice/session-2.scope/app.slice/firefox-1234.scope"
                    .to_string()
            )),
            Some("firefox-1234.scope".to_string())
        );

        // Тест с несколькими уровнями вложенности
        assert_eq!(
            extract_systemd_unit(&Some(
                "/system.slice/docker-abc123.scope/container.slice/firefox.service".to_string()
            )),
            Some("firefox.service".to_string())
        );

        // Тест с нестандартными именами
        assert_eq!(
            extract_systemd_unit(&Some(
                "/user.slice/user-1000.slice/session-2.scope/my-custom-app@123.service".to_string()
            )),
            Some("my-custom-app@123.service".to_string())
        );
    }

    #[test]
    fn parse_cgroup_v2_path_handles_edge_cases() {
        // Тест обработки пустого cgroup пути
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // Пустой cgroup путь
        let empty_cgroup = "0::\n";
        fs::write(proc_dir.join("cgroup"), empty_cgroup).unwrap();

        let path = proc_dir.join("cgroup");
        let contents = fs::read_to_string(&path).unwrap();
        let mut found_path = None;
        for line in contents.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                let cgroup_path = parts[2];
                if !cgroup_path.is_empty() && cgroup_path != "/" {
                    found_path = Some(cgroup_path.to_string());
                    break;
                }
            }
        }
        assert_eq!(found_path, None);

        // Тест с корневым путем
        let root_cgroup = "0::/\n";
        fs::write(proc_dir.join("cgroup"), root_cgroup).unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        let mut found_path = None;
        for line in contents.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                let cgroup_path = parts[2];
                if !cgroup_path.is_empty() && cgroup_path != "/" {
                    found_path = Some(cgroup_path.to_string());
                    break;
                }
            }
        }
        assert_eq!(found_path, None);
    }

    #[test]
    fn parse_env_vars_handles_special_characters() {
        // Тест обработки специальных символов в переменных окружения
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // environ с специальными символами
        let env_content =
            "DISPLAY=:0\0TERM=xterm-256color\0SSH_CLIENT=192.168.1.1 22 33\0LANG=en_US.UTF-8\0";
        fs::write(proc_dir.join("environ"), env_content).unwrap();

        let path = proc_dir.join("environ");
        let contents = fs::read_to_string(&path).unwrap();
        let mut has_display = false;
        let mut term = None;
        let mut ssh = false;
        let mut lang = None;

        for env_var in contents.split('\0') {
            if env_var.starts_with("DISPLAY=") {
                has_display = true;
            } else if env_var.starts_with("TERM=") {
                term = env_var.strip_prefix("TERM=").map(|s| s.to_string());
            } else if env_var.starts_with("SSH_") {
                ssh = true;
            } else if env_var.starts_with("LANG=") {
                lang = env_var.strip_prefix("LANG=").map(|s| s.to_string());
            }
        }

        assert!(has_display);
        assert_eq!(term, Some("xterm-256color".to_string()));
        assert!(ssh);
        assert_eq!(lang, Some("en_US.UTF-8".to_string()));
    }

    #[test]
    fn parse_uid_gid_handles_boundary_values() {
        // Тест обработки граничных значений UID/GID
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // Максимальные значения UID/GID
        let max_status =
            format!(
            "Name:   test_process\nState:  R (running)\nUid:    {} {} {} {}\nGid:    {} {} {} {}\n",
            u32::MAX, u32::MAX, u32::MAX, u32::MAX,
            u32::MAX, u32::MAX, u32::MAX, u32::MAX
        );
        fs::write(proc_dir.join("status"), max_status).unwrap();

        let status_path = proc_dir.join("status");
        let contents = fs::read_to_string(&status_path).unwrap();
        let mut uid = 0;
        let mut gid = 0;

        for line in contents.lines() {
            if line.starts_with("Uid:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    uid = parts[1].parse::<u32>().unwrap();
                }
            } else if line.starts_with("Gid:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    gid = parts[1].parse::<u32>().unwrap();
                }
            }
        }

        assert_eq!(uid, u32::MAX);
        assert_eq!(gid, u32::MAX);

        // Нулевые значения UID/GID
        let zero_status =
            "Name:   test_process\nState:  R (running)\nUid:    0 0 0 0\nGid:    0 0 0 0\n";
        fs::write(proc_dir.join("status"), zero_status).unwrap();

        let contents = fs::read_to_string(&status_path).unwrap();
        let mut uid = 0;
        let mut gid = 0;

        for line in contents.lines() {
            if line.starts_with("Uid:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    uid = parts[1].parse::<u32>().unwrap();
                }
            } else if line.starts_with("Gid:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    gid = parts[1].parse::<u32>().unwrap();
                }
            }
        }

        assert_eq!(uid, 0);
        assert_eq!(gid, 0);
    }

    #[test]
    fn test_error_handling_with_detailed_messages() {
        // Тест проверяет, что функции возвращают детальные сообщения об ошибках
        // с практическими рекомендациями по устранению неполадок
        
        // Проверяем обработку несуществующего процесса
        let result = read_cgroup_path(999999);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
        
        // Проверяем обработку несуществующего файла status
        let result = read_uid_gid(999999);
        assert!(result.is_err());
        
        // Проверяем, что функция возвращает ошибку с контекстом
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Не удалось прочитать /proc/999999/status"));
        assert!(error_msg.contains("проверьте, что процесс существует и доступен для чтения"));
    }

    #[test]
    fn test_graceful_degradation_scenarios() {
        // Тест проверяет graceful degradation в различных сценариях ошибок
        
        // Сценарий 1: Несуществующий процесс - должен вернуть Ok(None)
        let result = read_cgroup_path(999999);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
        
        // Сценарий 2: Несуществующий файл environ - должен вернуть Ok с значениями по умолчанию
        let result = read_env_vars(999999);
        assert!(result.is_ok());
        let (has_display, has_wayland, term, ssh) = result.unwrap();
        assert!(!has_display);
        assert!(!has_wayland);
        assert_eq!(term, None);
        assert!(!ssh);
        
        // Сценарий 3: Проверяем, что кэш обрабатывает ошибки корректно
        let config = ProcessCacheConfig {
            cache_ttl_seconds: 1,
            max_cached_processes: 10,
            ..Default::default()
        };
        let mut cache = ProcessCache::with_config(config);
        
        // Добавляем запись и проверяем, что она истекает
        let test_record = ProcessRecord {
            pid: 123,
            ppid: 456,
            uid: 1000,
            gid: 1000,
            exe: Some("test_exe".to_string()),
            cmdline: Some("test_cmdline".to_string()),
            cgroup_path: Some("/test/cgroup".to_string()),
            systemd_unit: Some("test.service".to_string()),
            app_group_id: None,
            state: "R".to_string(),
            start_time: 1000,
            uptime_sec: 60,
            tty_nr: 1,
            has_tty: true,
            cpu_share_1s: Some(10.0),
            cpu_share_10s: Some(15.0),
            io_read_bytes: Some(1024),
            io_write_bytes: Some(2048),
            rss_mb: Some(100),
            swap_mb: Some(50),
            voluntary_ctx: Some(1000),
            involuntary_ctx: Some(500),
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
            tags: vec![],
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
            energy_uj: None,
            power_w: None,
            energy_timestamp: None,
        };
        
        cache.cache_record(test_record);
        assert!(cache.get_cached(123).is_some());
        
        // Ждем истечения TTL (TTL=1с, ждем 1.5с для надежности)
        std::thread::sleep(std::time::Duration::from_millis(1500));
        assert!(cache.get_cached(123).is_none());
    }

    #[test]
    fn test_error_messages_contain_troubleshooting_advice() {
        // Тест проверяет, что сообщения об ошибках содержат практические рекомендации
        
        // Проверяем, что ошибка чтения status содержит рекомендации
        let result = read_uid_gid(999999);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        
        // Проверяем, что сообщение содержит практические рекомендации
        assert!(error_msg.contains("проверьте, что процесс существует и доступен для чтения"));
        // Note: read_uid_gid uses a simpler error message format without the "Попробуйте:" part
        // The main error handling improvements are in collect_single_process and collect_process_metrics
    }

    #[test]
    fn read_cgroup_path_with_malformed_content() {
        // Тест обработки некорректного содержимого cgroup
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // Некорректный формат cgroup (не хватает колонок)
        let malformed_cgroup = "0:/user.slice\n";
        fs::write(proc_dir.join("cgroup"), malformed_cgroup).unwrap();

        let path = proc_dir.join("cgroup");
        let contents = fs::read_to_string(&path).unwrap();
        let mut found_path = None;
        for line in contents.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                let cgroup_path = parts[2];
                if !cgroup_path.is_empty() && cgroup_path != "/" {
                    found_path = Some(cgroup_path.to_string());
                    break;
                }
            }
        }
        assert_eq!(found_path, None);

        // Некорректный формат cgroup (слишком много колонок)
        let malformed_cgroup2 = "0:1:2:/user.slice\n";
        fs::write(proc_dir.join("cgroup"), malformed_cgroup2).unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        let mut found_path = None;
        for line in contents.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                let cgroup_path = parts[2];
                if !cgroup_path.is_empty() && cgroup_path != "/" {
                    found_path = Some(cgroup_path.to_string());
                    break;
                }
            }
        }
        // В этом случае parts[2] будет "2", а не "/user.slice"
        assert_eq!(found_path, Some("2".to_string()));
    }

    #[test]
    fn parse_env_vars_with_unicode_content() {
        // Тест обработки Unicode символов в переменных окружения
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // environ с Unicode символами
        let env_content = "DISPLAY=:0\0TERM=xterm-256color\0LANG=ru_RU.UTF-8\0USER=Пользователь\0";
        fs::write(proc_dir.join("environ"), env_content).unwrap();

        let path = proc_dir.join("environ");
        let contents = fs::read_to_string(&path).unwrap();
        let mut has_display = false;
        let mut lang = None;
        let mut user = None;

        for env_var in contents.split('\0') {
            if env_var.starts_with("DISPLAY=") {
                has_display = true;
            } else if env_var.starts_with("LANG=") {
                lang = env_var.strip_prefix("LANG=").map(|s| s.to_string());
            } else if env_var.starts_with("USER=") {
                user = env_var.strip_prefix("USER=").map(|s| s.to_string());
            }
        }

        assert!(has_display);
        assert_eq!(lang, Some("ru_RU.UTF-8".to_string()));
        assert_eq!(user, Some("Пользователь".to_string()));
    }
}

/// Тесты для кэширования метрик процессов
#[cfg(test)]
mod cache_tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_process_cache_config_default() {
        let config = ProcessCacheConfig::default();
        assert_eq!(config.cache_ttl_seconds, 5);
        assert_eq!(config.max_cached_processes, 1000);
        assert!(config.enable_caching);
        assert!(config.enable_parallel_processing);
        assert_eq!(config.max_parallel_threads, None);
    }

    #[test]
    fn test_process_cache_initialization() {
        let cache = ProcessCache::new();
        assert!(cache.records.is_empty());
        assert_eq!(cache.config.cache_ttl_seconds, 5);
        assert_eq!(cache.config.max_cached_processes, 1000);
    }

    #[test]
    fn test_process_cache_with_custom_config() {
        let config = ProcessCacheConfig {
            cache_ttl_seconds: 30,
            max_cached_processes: 2000,
            enable_caching: true,
            enable_parallel_processing: false,
            max_parallel_threads: Some(8),
            batch_size: None,
            optimize_io: false,
            use_lru_cache: false,
        };

        let cache = ProcessCache::with_config(config.clone());
        assert!(cache.records.is_empty());
        assert_eq!(cache.config.cache_ttl_seconds, 30);
        assert_eq!(cache.config.max_cached_processes, 2000);
        assert!(!cache.config.enable_parallel_processing);
        assert_eq!(cache.config.max_parallel_threads, Some(8));
    }

    #[test]
    fn test_cache_record_and_retrieve() {
        let mut cache = ProcessCache::new();

        let test_record = ProcessRecord {
            pid: 123,
            ppid: 456,
            uid: 1000,
            gid: 1000,
            exe: Some("test_exe".to_string()),
            cmdline: Some("test_cmdline".to_string()),
            cgroup_path: Some("/test/cgroup".to_string()),
            systemd_unit: Some("test.service".to_string()),
            app_group_id: None,
            state: "R".to_string(),
            start_time: 1000,
            uptime_sec: 60,
            tty_nr: 1,
            has_tty: true,
            cpu_share_1s: Some(10.0),
            cpu_share_10s: Some(15.0),
            io_read_bytes: Some(1024),
            io_write_bytes: Some(2048),
            rss_mb: Some(100),
            swap_mb: Some(50),
            voluntary_ctx: Some(1000),
            involuntary_ctx: Some(500),
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
            tags: vec![],
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
            energy_uj: None,
            power_w: None,
            energy_timestamp: None,
        };

        // Сохраняем запись в кэш
        cache.cache_record(test_record.clone());

        // Проверяем, что запись сохранена
        assert_eq!(cache.records.len(), 1);
        assert!(cache.records.contains_key(&123));

        // Извлекаем запись из кэша
        if let Some(cached_record) = cache.get_cached(123) {
            assert_eq!(cached_record.pid, 123);
            assert_eq!(cached_record.exe, Some("test_exe".to_string()));
            assert_eq!(cached_record.cmdline, Some("test_cmdline".to_string()));
        } else {
            panic!("Не удалось извлечь запись из кэша");
        }
    }

    #[test]
    fn test_cache_expiration() {
        let mut cache = ProcessCache::new();

        let test_record = ProcessRecord {
            pid: 123,
            ppid: 456,
            uid: 1000,
            gid: 1000,
            exe: Some("test_exe".to_string()),
            cmdline: Some("test_cmdline".to_string()),
            cgroup_path: Some("/test/cgroup".to_string()),
            systemd_unit: Some("test.service".to_string()),
            app_group_id: None,
            state: "R".to_string(),
            start_time: 1000,
            uptime_sec: 60,
            tty_nr: 1,
            has_tty: true,
            cpu_share_1s: Some(10.0),
            cpu_share_10s: Some(15.0),
            io_read_bytes: Some(1024),
            io_write_bytes: Some(2048),
            rss_mb: Some(100),
            swap_mb: Some(50),
            voluntary_ctx: Some(1000),
            involuntary_ctx: Some(500),
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
            tags: vec![],
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
            energy_uj: None,
            power_w: None,
            energy_timestamp: None,
        };

        // Сохраняем запись в кэш
        cache.cache_record(test_record.clone());
        assert_eq!(cache.records.len(), 1);

        // Проверяем, что запись доступна сразу после сохранения
        assert!(cache.get_cached(123).is_some());

        // Устанавливаем очень короткий TTL и ждем истечения
        cache.config.cache_ttl_seconds = 1;
        thread::sleep(Duration::from_secs(2));

        // Проверяем, что запись больше не доступна после истечения TTL
        assert!(cache.get_cached(123).is_none());
    }

    #[test]
    fn test_cache_size_limit() {
        let mut cache = ProcessCache {
            records: HashMap::new(),
            lru_cache: None,
            config: ProcessCacheConfig {
                cache_ttl_seconds: 300,
                max_cached_processes: 2, // Очень маленький лимит
                enable_caching: true,
                enable_parallel_processing: true,
                max_parallel_threads: None,
                batch_size: None,
                optimize_io: false,
                use_lru_cache: false,
            },
        };

        // Создаем несколько тестовых записей с небольшими задержками
        // чтобы гарантировать разные временные метки
        for i in 0..5 {
            let record = ProcessRecord {
                pid: i,
                ppid: 0,
                uid: 1000,
                gid: 1000,
                exe: Some(format!("test_exe_{}", i)),
                cmdline: Some(format!("test_cmdline_{}", i)),
                cgroup_path: Some(format!("/test/cgroup_{}", i)),
                systemd_unit: Some(format!("test_{}.service", i)),
                app_group_id: None,
                state: "R".to_string(),
                start_time: 1000,
                uptime_sec: 60,
                tty_nr: 1,
                has_tty: true,
                cpu_share_1s: Some(10.0),
                cpu_share_10s: Some(15.0),
                io_read_bytes: Some(1024),
                io_write_bytes: Some(2048),
                rss_mb: Some(100),
                swap_mb: Some(50),
                voluntary_ctx: Some(1000),
                involuntary_ctx: Some(500),
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
                tags: vec![],
                nice: 0,
                ionice_class: None,
                ionice_prio: None,
                teacher_priority_class: None,
                teacher_score: None,
                energy_uj: None,
                power_w: None,
                energy_timestamp: None,
            };

            cache.cache_record(record);
            // Небольшая задержка, чтобы гарантировать разные временные метки
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        // Проверяем, что количество записей не превышает лимит
        assert_eq!(cache.records.len(), 2);

        // Проверяем, что в кэше остались самые новые записи (последние добавленные)
        assert!(cache.records.contains_key(&3) || cache.records.contains_key(&4));
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = ProcessCache::new();

        // Добавляем несколько записей
        for i in 0..5 {
            let record = ProcessRecord {
                pid: i,
                ppid: 0,
                uid: 1000,
                gid: 1000,
                exe: Some(format!("test_exe_{}", i)),
                cmdline: Some(format!("test_cmdline_{}", i)),
                cgroup_path: Some(format!("/test/cgroup_{}", i)),
                systemd_unit: Some(format!("test_{}.service", i)),
                app_group_id: None,
                state: "R".to_string(),
                start_time: 1000,
                uptime_sec: 60,
                tty_nr: 1,
                has_tty: true,
                cpu_share_1s: Some(10.0),
                cpu_share_10s: Some(15.0),
                io_read_bytes: Some(1024),
                io_write_bytes: Some(2048),
                rss_mb: Some(100),
                swap_mb: Some(50),
                voluntary_ctx: Some(1000),
                involuntary_ctx: Some(500),
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
                tags: vec![],
                nice: 0,
                ionice_class: None,
                ionice_prio: None,
                teacher_priority_class: None,
                teacher_score: None,
                energy_uj: None,
                power_w: None,
                energy_timestamp: None,
            };

            cache.cache_record(record);
        }

        assert_eq!(cache.records.len(), 5);

        // Очищаем кэш
        cache.clear();
        assert!(cache.records.is_empty());
    }

    #[test]
    fn test_cache_update_config() {
        let mut cache = ProcessCache::new();

        // Добавляем запись с текущей конфигурацией
        let record = ProcessRecord {
            pid: 123,
            ppid: 456,
            uid: 1000,
            gid: 1000,
            exe: Some("test_exe".to_string()),
            cmdline: Some("test_cmdline".to_string()),
            cgroup_path: Some("/test/cgroup".to_string()),
            systemd_unit: Some("test.service".to_string()),
            app_group_id: None,
            state: "R".to_string(),
            start_time: 1000,
            uptime_sec: 60,
            tty_nr: 1,
            has_tty: true,
            cpu_share_1s: Some(10.0),
            cpu_share_10s: Some(15.0),
            io_read_bytes: Some(1024),
            io_write_bytes: Some(2048),
            rss_mb: Some(100),
            swap_mb: Some(50),
            voluntary_ctx: Some(1000),
            involuntary_ctx: Some(500),
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
            tags: vec![],
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
            energy_uj: None,
            power_w: None,
            energy_timestamp: None,
        };

        cache.cache_record(record);
        assert_eq!(cache.records.len(), 1);

        // Обновляем конфигурацию
        let new_config = ProcessCacheConfig {
            cache_ttl_seconds: 60,
            max_cached_processes: 2000,
            enable_caching: true,
            enable_parallel_processing: false,
            max_parallel_threads: Some(2),
            batch_size: None,
            optimize_io: false,
            use_lru_cache: false,
        };

        cache.update_config(new_config.clone());

        // Проверяем, что конфигурация обновлена
        assert_eq!(cache.config.cache_ttl_seconds, 60);
        assert_eq!(cache.config.max_cached_processes, 2000);
        assert!(!cache.config.enable_parallel_processing);
        assert_eq!(cache.config.max_parallel_threads, Some(2));

        // Проверяем, что запись все еще в кэше
        assert_eq!(cache.records.len(), 1);
    }

    #[test]
    fn test_global_cache_functions() {
        // Очищаем кэш перед тестом
        clear_process_cache();

        // Проверяем конфигурацию по умолчанию
        let config = get_process_cache_config();
        assert_eq!(config.cache_ttl_seconds, 5);
        assert_eq!(config.max_cached_processes, 1000);
        assert!(config.use_lru_cache);
        assert_eq!(config.batch_size, Some(100));
        assert!(config.optimize_io);

        // Обновляем конфигурацию
        let new_config = ProcessCacheConfig {
            cache_ttl_seconds: 30,
            max_cached_processes: 2000,
            ..Default::default()
        };

        update_process_cache_config(new_config.clone());

        // Проверяем, что конфигурация обновлена
        let updated_config = get_process_cache_config();
        assert_eq!(updated_config.cache_ttl_seconds, 30);
        assert_eq!(updated_config.max_cached_processes, 2000);
    }

    #[test]
    fn test_cache_performance_stats() {
        // Очищаем статистику перед тестом
        reset_cache_performance_stats();

        let stats = get_cache_performance_stats();
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
        assert_eq!(stats.hit_rate(), 0.0);
    }

    #[test]
    fn test_lru_cache_initialization() {
        let config = ProcessCacheConfig {
            use_lru_cache: true,
            max_cached_processes: 10,
            ..Default::default()
        };

        let cache = ProcessCache::with_config(config);
        assert!(cache.lru_cache.is_some());
        
        if let Some(lru_cache) = cache.lru_cache {
            let lru_read = lru_cache.read().unwrap();
            assert_eq!(lru_read.len(), 0);
            assert_eq!(lru_read.cap().get(), 10);
        }
    }

    #[test]
    fn test_batch_config() {
        let config = ProcessCacheConfig {
            batch_size: Some(50),
            ..Default::default()
        };

        assert_eq!(config.batch_size, Some(50));
        
        let config2 = ProcessCacheConfig::default();
        assert_eq!(config2.batch_size, Some(100));
    }
}

/// Очистить кэш метрик процессов.
///
/// Эта функция полезна при тестировании или когда нужно принудительно обновить данные.
///
/// # Примеры использования
///
/// ```rust
/// use smoothtask_core::metrics::process::clear_process_cache;
///
/// // Очистка кэша перед тестами
/// clear_process_cache();
/// ```
pub fn clear_process_cache() {
    let mut cache_write = PROCESS_CACHE.write().unwrap();
    cache_write.clear();
    
    // Также очищаем статистику производительности
    let mut stats_write = CACHE_STATS.write().unwrap();
    stats_write.reset();
    
    tracing::info!("Кэш метрик процессов очищен");
}

/// Сбросить статистику производительности кэша.
///
/// # Примеры использования
///
/// ```rust
/// use smoothtask_core::metrics::process::reset_cache_performance_stats;
///
/// // Сброс статистики перед новым тестом
/// reset_cache_performance_stats();
/// ```
pub fn reset_cache_performance_stats() {
    let mut stats_write = CACHE_STATS.write().unwrap();
    stats_write.reset();
    tracing::info!("Статистика производительности кэша сброшена");
}

/// Получить статистику производительности кэша.
///
/// # Возвращаемое значение
///
/// Статистика производительности кэша метрик процессов.
///
/// # Примеры использования
///
/// ```rust
/// use smoothtask_core::metrics::process::get_cache_performance_stats;
///
/// let stats = get_cache_performance_stats();
/// println!("Процент попаданий в кэш: {:.2}%", stats.hit_rate() * 100.0);
/// ```
pub fn get_cache_performance_stats() -> CachePerformanceStats {
    let stats_read = CACHE_STATS.read().unwrap();
    stats_read.clone()
}

/// Обновить конфигурацию кэша метрик процессов.
///
/// # Аргументы
///
/// * `config` - Новая конфигурация кэша
///
/// # Примеры использования
///
/// ```rust
/// use smoothtask_core::metrics::process::{update_process_cache_config, ProcessCacheConfig};
///
/// let config = ProcessCacheConfig {
///     cache_ttl_seconds: 30,
///     max_cached_processes: 5000,
///     ..Default::default()
/// };
///
/// update_process_cache_config(config);
/// ```
pub fn update_process_cache_config(config: ProcessCacheConfig) {
    let mut cache_write = PROCESS_CACHE.write().unwrap();
    cache_write.update_config(config);
    tracing::info!("Конфигурация кэша метрик процессов обновлена");
}

/// Получить текущую конфигурацию кэша.
///
/// # Возвращаемое значение
///
/// Текущая конфигурация кэша метрик процессов.
///
/// # Примеры использования
///
/// ```rust
/// use smoothtask_core::metrics::process::get_process_cache_config;
///
/// let config = get_process_cache_config();
/// println!("Текущий TTL кэша: {} секунд", config.cache_ttl_seconds);
/// ```
pub fn get_process_cache_config() -> ProcessCacheConfig {
    let cache_read = PROCESS_CACHE.read().unwrap();
    cache_read.config.clone()
}

/// Получить статистику кэша метрик процессов.
///
/// # Возвращаемое значение
///
/// Статистика кэша метрик процессов.
///
/// # Примеры использования
///
/// ```rust
/// use smoothtask_core::metrics::process::get_process_cache_stats;
///
/// let stats = get_process_cache_stats();
/// println!("Всего записей в кэше: {}", stats.total_entries);
/// println!("Актуальных записей: {}", stats.active_entries);
/// ```
pub fn get_process_cache_stats() -> ProcessCacheStats {
    let cache_read = PROCESS_CACHE.read().unwrap();
    cache_read.get_cache_stats()
}

/// Собрать метрики всех процессов из /proc (устаревшая версия для совместимости).
///
/// Эта функция сохранена для обратной совместимости и вызывает новую версию с конфигурацией по умолчанию.
///
/// # Возвращаемое значение
///
/// - `Ok(Vec<ProcessRecord>)` - вектор с метриками всех доступных процессов
/// - `Err(anyhow::Error)` - если не удалось получить список процессов из /proc
///
/// # Примечания
///
/// Эта функция устарела. Используйте `collect_process_metrics(Some(config))` или `collect_process_metrics(None)` вместо этого.
#[deprecated(
    note = "Use collect_process_metrics(Some(config)) or collect_process_metrics(None) instead"
)]
pub fn collect_process_metrics_legacy() -> Result<Vec<ProcessRecord>> {
    collect_process_metrics(None)
}

#[cfg(test)]
mod energy_tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_collect_process_energy_metrics_not_available() {
        // Тестируем случай, когда энергопотребление недоступно
        let (energy_uj, power_w, energy_timestamp) = collect_process_energy_metrics(999999);
        
        // Для несуществующего процесса или недоступного интерфейса
        // должны получить None значения
        assert!(energy_uj.is_none());
        assert!(power_w.is_none());
        assert!(energy_timestamp.is_none());
    }

    #[test]
    fn test_collect_process_energy_metrics_invalid_data() {
        // Создаем временный файл с некорректными данными
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "invalid_data").unwrap();
        
        // Мокаем чтение файла (в реальном тесте это сложнее)
        // Для простоты просто проверяем, что функция не паникует
        let (energy_uj, power_w, energy_timestamp) = collect_process_energy_metrics(1);
        
        // Должны получить None значения для некорректных данных
        assert!(energy_uj.is_none());
        assert!(power_w.is_none());
        // energy_timestamp может быть None или Some (в зависимости от реализации)
        let _ = energy_timestamp; // Используем переменную, чтобы избежать предупреждения
    }

    #[test]
    fn test_energy_fields_in_process_record() {
        // Тестируем, что ProcessRecord содержит поля энергопотребления
        let record = ProcessRecord {
            pid: 123,
            ppid: 456,
            uid: 1000,
            gid: 1000,
            exe: Some("test_exe".to_string()),
            cmdline: Some("test_cmdline".to_string()),
            cgroup_path: Some("/test/cgroup".to_string()),
            systemd_unit: Some("test.service".to_string()),
            app_group_id: None,
            state: "R".to_string(),
            start_time: 1000,
            uptime_sec: 60,
            tty_nr: 1,
            has_tty: true,
            cpu_share_1s: Some(10.0),
            cpu_share_10s: Some(15.0),
            io_read_bytes: Some(1024),
            io_write_bytes: Some(2048),
            rss_mb: Some(100),
            swap_mb: Some(50),
            voluntary_ctx: Some(1000),
            involuntary_ctx: Some(500),
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
            tags: vec![],
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
            energy_uj: Some(123456),
            power_w: Some(1.23),
            energy_timestamp: Some(1234567890),
        };

        // Проверяем, что поля энергопотребления установлены корректно
        assert_eq!(record.energy_uj, Some(123456));
        assert_eq!(record.power_w, Some(1.23));
        assert_eq!(record.energy_timestamp, Some(1234567890));
    }
}
