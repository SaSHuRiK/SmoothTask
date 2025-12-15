//! Модуль мониторинга и оптимизации использования кэша
//!
//! Этот модуль предоставляет расширенные функции для мониторинга и оптимизации
//! использования кэша в системе. Основные возможности:
//! - Мониторинг использования кэша в реальном времени
//! - Анализ эффективности кэширования
//! - Оптимизация параметров кэша на основе нагрузки
//! - Обнаружение и предотвращение проблем с кэшем
//! - Расширенная статистика и метрики кэша

use crate::metrics::cache::MetricsCache;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use tracing::{debug, info, warn};

/// Структура для хранения метрик мониторинга кэша
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheMonitorMetrics {
    /// Временная метка сбора метрик
    pub timestamp: SystemTime,
    /// Общее количество кэшей в системе
    pub total_caches: usize,
    /// Общее использование памяти кэшем в байтах
    pub total_memory_usage: u64,
    /// Общий hit rate по всем кэшам
    pub overall_hit_rate: f64,
    /// Общий miss rate по всем кэшам
    pub overall_miss_rate: f64,
    /// Количество активных кэшей
    pub active_caches: usize,
    /// Количество неактивных кэшей
    pub inactive_caches: usize,
    /// Средний размер кэша
    pub average_cache_size: f64,
    /// Максимальный размер кэша
    pub max_cache_size: u64,
    /// Минимальный размер кэша
    pub min_cache_size: u64,
    /// Метрики по типам кэшей
    pub cache_type_metrics: HashMap<String, CacheTypeMetrics>,
    /// Метрики по приоритетам кэшей
    pub cache_priority_metrics: HashMap<u32, CachePriorityMetrics>,
    /// Тренды использования кэша
    pub usage_trends: CacheUsageTrends,
    /// Рекомендации по оптимизации
    pub optimization_recommendations: Vec<String>,
}

impl Default for CacheMonitorMetrics {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now(),
            total_caches: 0,
            total_memory_usage: 0,
            overall_hit_rate: 0.0,
            overall_miss_rate: 0.0,
            active_caches: 0,
            inactive_caches: 0,
            average_cache_size: 0.0,
            max_cache_size: 0,
            min_cache_size: 0,
            cache_type_metrics: HashMap::new(),
            cache_priority_metrics: HashMap::new(),
            usage_trends: CacheUsageTrends::default(),
            optimization_recommendations: Vec::new(),
        }
    }
}

/// Метрики по типам кэшей
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheTypeMetrics {
    /// Количество кэшей этого типа
    pub cache_count: usize,
    /// Общее использование памяти
    pub total_memory_usage: u64,
    /// Средний hit rate
    pub average_hit_rate: f64,
    /// Средний miss rate
    pub average_miss_rate: f64,
    /// Средний размер кэша
    pub average_size: f64,
    /// Количество активных кэшей
    pub active_caches: usize,
    /// Количество неактивных кэшей
    pub inactive_caches: usize,
}

/// Метрики по приоритетам кэшей
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CachePriorityMetrics {
    /// Количество кэшей с этим приоритетом
    pub cache_count: usize,
    /// Общее использование памяти
    pub total_memory_usage: u64,
    /// Средний hit rate
    pub average_hit_rate: f64,
    /// Средний miss rate
    pub average_miss_rate: f64,
    /// Средний размер кэша
    pub average_size: f64,
}

/// Default implementation for CacheTypeMetrics
impl Default for CacheTypeMetrics {
    fn default() -> Self {
        Self {
            cache_count: 0,
            total_memory_usage: 0,
            average_hit_rate: 0.0,
            average_miss_rate: 0.0,
            average_size: 0.0,
            active_caches: 0,
            inactive_caches: 0,
        }
    }
}

/// Default implementation for CachePriorityMetrics
impl Default for CachePriorityMetrics {
    fn default() -> Self {
        Self {
            cache_count: 0,
            total_memory_usage: 0,
            average_hit_rate: 0.0,
            average_miss_rate: 0.0,
            average_size: 0.0,
        }
    }
}

/// Тренды использования кэша
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheUsageTrends {
    /// Тренд hit rate (увеличение/уменьшение)
    pub hit_rate_trend: f64,
    /// Тренд miss rate (увеличение/уменьшение)
    pub miss_rate_trend: f64,
    /// Тренд использования памяти (увеличение/уменьшение)
    pub memory_usage_trend: f64,
    /// Тренд размера кэша (увеличение/уменьшение)
    pub cache_size_trend: f64,
    /// Тренд активности кэша (увеличение/уменьшение)
    pub activity_trend: f64,
}

impl Default for CacheUsageTrends {
    fn default() -> Self {
        Self {
            hit_rate_trend: 0.0,
            miss_rate_trend: 0.0,
            memory_usage_trend: 0.0,
            cache_size_trend: 0.0,
            activity_trend: 0.0,
        }
    }
}

/// Конфигурация мониторинга кэша
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheMonitorConfig {
    /// Интервал мониторинга в секундах
    pub monitoring_interval_secs: u64,
    /// Включить расширенный мониторинг
    pub enable_extended_monitoring: bool,
    /// Включить анализ эффективности
    pub enable_efficiency_analysis: bool,
    /// Включить оптимизацию параметров
    pub enable_parameter_optimization: bool,
    /// Включить обнаружение проблем
    pub enable_problem_detection: bool,
    /// Минимальный hit rate для предупреждений
    pub min_hit_rate_warning: f64,
    /// Максимальный miss rate для предупреждений
    pub max_miss_rate_warning: f64,
    /// Максимальное использование памяти для предупреждений (в процентах)
    pub max_memory_usage_warning: f64,
    /// Включить автоматическую оптимизацию
    pub enable_auto_optimization: bool,
    /// Агрессивность оптимизации (0.0 - 1.0)
    pub optimization_aggressiveness: f64,
}

impl Default for CacheMonitorConfig {
    fn default() -> Self {
        Self {
            monitoring_interval_secs: 60,
            enable_extended_monitoring: true,
            enable_efficiency_analysis: true,
            enable_parameter_optimization: true,
            enable_problem_detection: true,
            min_hit_rate_warning: 0.7,
            max_miss_rate_warning: 0.3,
            max_memory_usage_warning: 0.8,
            enable_auto_optimization: true,
            optimization_aggressiveness: 0.5,
        }
    }
}

/// Основная структура мониторинга кэша
pub struct CacheMonitor {
    /// Конфигурация мониторинга
    config: CacheMonitorConfig,
    /// История метрик для анализа трендов
    metrics_history: Vec<CacheMonitorMetrics>,
    /// Максимальный размер истории
    max_history_size: usize,
    /// Последние собранные метрики
    last_metrics: Option<CacheMonitorMetrics>,
}

impl CacheMonitor {
    /// Создать новый экземпляр мониторинга кэша
    pub fn new(config: CacheMonitorConfig) -> Self {
        info!(
            "Creating cache monitor with config: interval={}s, extended={}",
            config.monitoring_interval_secs, config.enable_extended_monitoring
        );

        Self {
            config,
            metrics_history: Vec::new(),
            max_history_size: 10,
            last_metrics: None,
        }
    }

    /// Создать новый экземпляр с конфигурацией по умолчанию
    pub fn new_default() -> Self {
        Self::new(CacheMonitorConfig::default())
    }

    /// Создать новый экземпляр с кастомным размером истории
    pub fn with_history_size(config: CacheMonitorConfig, history_size: usize) -> Self {
        Self {
            config,
            metrics_history: Vec::new(),
            max_history_size: history_size,
            last_metrics: None,
        }
    }

    /// Собрать метрики мониторинга кэша
    pub fn collect_cache_metrics(&mut self, caches: &[MetricsCache]) -> Result<CacheMonitorMetrics> {
        let mut metrics = CacheMonitorMetrics::default();
        metrics.timestamp = SystemTime::now();

        // Собираем базовую статистику
        let total_caches = caches.len();
        metrics.total_caches = total_caches;

        // Собираем метрики по каждому кэшу
        let mut total_memory = 0u64;
        let mut total_hits = 0u64;
        let mut total_requests = 0u64;
        let mut active_caches = 0usize;
        let mut cache_sizes = Vec::new();

        // Группировка по типам и приоритетам
        let mut cache_type_metrics: HashMap<String, CacheTypeMetrics> = HashMap::new();
        let mut cache_priority_metrics: HashMap<u32, CachePriorityMetrics> = HashMap::new();

        for cache in caches {
            let perf_metrics = cache.get_performance_metrics();
            let memory_usage = cache.current_memory_usage();
            let cache_size = cache.len();

            total_memory += memory_usage as u64;
            total_hits += perf_metrics.cache_hits.load(std::sync::atomic::Ordering::Relaxed) as u64;
            total_requests += perf_metrics.total_requests.load(std::sync::atomic::Ordering::Relaxed) as u64;
            cache_sizes.push(cache_size as u64);

            // Считаем кэш активным, если он не пустой и есть обращения
            if cache_size > 0 && total_requests > 0 {
                active_caches += 1;
            }

            // Обновляем метрики по типам
            let cache_config = cache.get_config();
            let metric_type = "system_metrics".to_string(); // Упрощение для примера
            
            let type_metrics = cache_type_metrics
                .entry(metric_type)
                .or_insert_with(CacheTypeMetrics::default);
            type_metrics.cache_count += 1;
            type_metrics.total_memory_usage += memory_usage as u64;
            type_metrics.average_size = type_metrics.total_memory_usage as f64 / type_metrics.cache_count as f64;

            // Обновляем метрики по приоритетам
            let priority = 1u32; // Упрощение для примера
            let priority_metrics = cache_priority_metrics
                .entry(priority)
                .or_insert_with(CachePriorityMetrics::default);
            priority_metrics.cache_count += 1;
            priority_metrics.total_memory_usage += memory_usage as u64;
            priority_metrics.average_size = priority_metrics.total_memory_usage as f64 / priority_metrics.cache_count as f64;
        }

        // Рассчитываем общие метрики
        metrics.total_memory_usage = total_memory;
        metrics.overall_hit_rate = if total_requests > 0 {
            total_hits as f64 / total_requests as f64
        } else {
            0.0
        };
        metrics.overall_miss_rate = 1.0 - metrics.overall_hit_rate;
        metrics.active_caches = active_caches;
        metrics.inactive_caches = total_caches - active_caches;

        // Рассчитываем статистику размеров
        if !cache_sizes.is_empty() {
            metrics.average_cache_size = cache_sizes.iter().sum::<u64>() as f64 / cache_sizes.len() as f64;
            metrics.max_cache_size = *cache_sizes.iter().max().unwrap_or(&0);
            metrics.min_cache_size = *cache_sizes.iter().min().unwrap_or(&0);
        }

        // Устанавливаем метрики по типам и приоритетам
        metrics.cache_type_metrics = cache_type_metrics;
        metrics.cache_priority_metrics = cache_priority_metrics;

        // Анализируем тренды, если есть история
        if !self.metrics_history.is_empty() {
            metrics.usage_trends = self.analyze_cache_trends(&metrics);
        }

        // Генерируем рекомендации по оптимизации
        if self.config.enable_efficiency_analysis {
            metrics.optimization_recommendations = self.generate_optimization_recommendations(&metrics);
        }

        // Сохраняем метрики в историю
        self.metrics_history.push(metrics.clone());
        if self.metrics_history.len() > self.max_history_size {
            self.metrics_history.remove(0);
        }

        // Сохраняем последние метрики
        self.last_metrics = Some(metrics.clone());

        info!(
            "Cache monitoring metrics collected: {} caches, {} active, hit_rate={:.2}%, memory={} bytes",
            total_caches, active_caches, metrics.overall_hit_rate * 100.0, total_memory
        );

        Ok(metrics)
    }

    /// Анализировать тренды использования кэша
    fn analyze_cache_trends(&self, current_metrics: &CacheMonitorMetrics) -> CacheUsageTrends {
        if self.metrics_history.is_empty() {
            return CacheUsageTrends::default();
        }

        let previous_metrics = &self.metrics_history[self.metrics_history.len() - 1];
        let mut trends = CacheUsageTrends::default();

        // Рассчитываем тренды
        trends.hit_rate_trend = current_metrics.overall_hit_rate - previous_metrics.overall_hit_rate;
        trends.miss_rate_trend = current_metrics.overall_miss_rate - previous_metrics.overall_miss_rate;
        trends.memory_usage_trend = current_metrics.total_memory_usage as f64 - previous_metrics.total_memory_usage as f64;
        trends.cache_size_trend = current_metrics.average_cache_size - previous_metrics.average_cache_size;
        trends.activity_trend = current_metrics.active_caches as f64 - previous_metrics.active_caches as f64;

        debug!(
            "Cache trends analyzed: hit_rate={:.4}, memory={:.2} bytes, activity={:.2}",
            trends.hit_rate_trend, trends.memory_usage_trend, trends.activity_trend
        );

        trends
    }

    /// Генерировать рекомендации по оптимизации
    fn generate_optimization_recommendations(&self, metrics: &CacheMonitorMetrics) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Проверяем hit rate
        if metrics.overall_hit_rate < self.config.min_hit_rate_warning {
            recommendations.push(format!(
                "Low cache hit rate ({:.2}%) - consider increasing cache size or TTL",
                metrics.overall_hit_rate * 100.0
            ));
        }

        // Проверяем miss rate
        if metrics.overall_miss_rate > self.config.max_miss_rate_warning {
            recommendations.push(format!(
                "High cache miss rate ({:.2}%) - consider optimizing cache strategy",
                metrics.overall_miss_rate * 100.0
            ));
        }

        // Проверяем использование памяти
        let max_memory = metrics.total_memory_usage as f64;
        let memory_usage_percent = if max_memory > 0.0 {
            metrics.total_memory_usage as f64 / max_memory
        } else {
            0.0
        };

        if memory_usage_percent > self.config.max_memory_usage_warning {
            recommendations.push(format!(
                "High memory usage ({:.2}%) - consider reducing cache size or TTL",
                memory_usage_percent * 100.0
            ));
        }

        // Проверяем неактивные кэши
        if metrics.inactive_caches > 0 && metrics.inactive_caches as f64 > metrics.total_caches as f64 * 0.3 {
            recommendations.push(format!(
                "High number of inactive caches ({}/{}) - consider cleaning up unused caches",
                metrics.inactive_caches, metrics.total_caches
            ));
        }

        // Анализируем тренды
        if !metrics.usage_trends.hit_rate_trend.is_nan() && metrics.usage_trends.hit_rate_trend < -0.05 {
            recommendations.push(format!(
                "Decreasing hit rate trend ({:.4}) - monitor cache effectiveness",
                metrics.usage_trends.hit_rate_trend
            ));
        }

        if !metrics.usage_trends.memory_usage_trend.is_nan() && metrics.usage_trends.memory_usage_trend > 1000000.0 {
            recommendations.push(format!(
                "Increasing memory usage trend ({:.2} bytes) - monitor memory consumption",
                metrics.usage_trends.memory_usage_trend
            ));
        }

        debug!(
            "Generated {} optimization recommendations",
            recommendations.len()
        );

        recommendations
    }

    /// Оптимизировать параметры кэша
    pub fn optimize_cache_parameters(&self, metrics: &CacheMonitorMetrics) -> Result<Vec<CacheOptimizationRecommendation>> {
        let mut optimizations = Vec::new();

        // Анализируем каждый тип кэша
        for (cache_type, type_metrics) in &metrics.cache_type_metrics {
            let mut optimization = CacheOptimizationRecommendation {
                cache_type: cache_type.clone(),
                current_size: type_metrics.average_size as u64,
                recommended_size: type_metrics.average_size as u64,
                current_ttl: 5, // Упрощение
                recommended_ttl: 5,
                priority: 1,
                reason: String::new(),
            };

            // Оптимизируем размер на основе hit rate
            if type_metrics.average_hit_rate < self.config.min_hit_rate_warning {
                // Увеличиваем размер для улучшения hit rate
                let increase_factor = 1.0 + (self.config.optimization_aggressiveness * 0.5);
                optimization.recommended_size = (type_metrics.average_size as f64 * increase_factor) as u64;
                optimization.reason.push_str("Low hit rate; ");
            } else if type_metrics.average_hit_rate > 0.9 {
                // Уменьшаем размер, если hit rate очень высокий
                let decrease_factor = 1.0 - (self.config.optimization_aggressiveness * 0.3);
                optimization.recommended_size = (type_metrics.average_size as f64 * decrease_factor) as u64;
                optimization.reason.push_str("High hit rate; ");
            }

            // Оптимизируем TTL на основе активности
            if (type_metrics.active_caches as f64 / type_metrics.cache_count as f64) < 0.5 {
                // Уменьшаем TTL для неактивных кэшей
                optimization.recommended_ttl = (optimization.current_ttl as f64 * 0.8) as u32;
                optimization.reason.push_str("Low activity; ");
            }

            // Убираем последний "; " если есть
            if !optimization.reason.is_empty() {
                optimization.reason.pop();
                optimization.reason.pop();
            }

            if optimization.recommended_size != optimization.current_size ||
               optimization.recommended_ttl != optimization.current_ttl {
                optimizations.push(optimization);
            }
        }

        info!(
            "Generated {} cache optimization recommendations",
            optimizations.len()
        );

        Ok(optimizations)
    }

    /// Обнаружить проблемы с кэшем
    pub fn detect_cache_problems(&self, metrics: &CacheMonitorMetrics) -> Result<Vec<CacheProblem>> {
        let mut problems = Vec::new();

        // Проверяем общие проблемы
        if metrics.overall_hit_rate < self.config.min_hit_rate_warning {
            problems.push(CacheProblem {
                problem_type: CacheProblemType::LowHitRate,
                severity: CacheProblemSeverity::Warning,
                description: format!(
                    "Overall cache hit rate is low: {:.2}% (threshold: {:.2}%)",
                    metrics.overall_hit_rate * 100.0, self.config.min_hit_rate_warning * 100.0
                ),
                affected_caches: "All caches".to_string(),
                recommendation: "Consider increasing cache size or TTL, or review cache strategy".to_string(),
            });
        }

        if metrics.overall_miss_rate > self.config.max_miss_rate_warning {
            problems.push(CacheProblem {
                problem_type: CacheProblemType::HighMissRate,
                severity: CacheProblemSeverity::Warning,
                description: format!(
                    "Overall cache miss rate is high: {:.2}% (threshold: {:.2}%)",
                    metrics.overall_miss_rate * 100.0, self.config.max_miss_rate_warning * 100.0
                ),
                affected_caches: "All caches".to_string(),
                recommendation: "Review cache strategy and data access patterns".to_string(),
            });
        }

        // Проверяем проблемы по типам кэшей
        for (cache_type, type_metrics) in &metrics.cache_type_metrics {
            if type_metrics.average_hit_rate < self.config.min_hit_rate_warning * 0.8 {
                problems.push(CacheProblem {
                    problem_type: CacheProblemType::LowHitRate,
                    severity: CacheProblemSeverity::Critical,
                    description: format!(
                        "Cache type '{}' has very low hit rate: {:.2}%",
                        cache_type, type_metrics.average_hit_rate * 100.0
                    ),
                    affected_caches: cache_type.clone(),
                    recommendation: format!(
                        "Consider increasing cache size or TTL for {} caches, or review their usage pattern",
                        cache_type
                    ),
                });
            }

            if type_metrics.average_miss_rate > self.config.max_miss_rate_warning * 1.2 {
                problems.push(CacheProblem {
                    problem_type: CacheProblemType::HighMissRate,
                    severity: CacheProblemSeverity::Critical,
                    description: format!(
                        "Cache type '{}' has very high miss rate: {:.2}%",
                        cache_type, type_metrics.average_miss_rate * 100.0
                    ),
                    affected_caches: cache_type.clone(),
                    recommendation: format!(
                        "Review cache strategy and data access patterns for {} caches",
                        cache_type
                    ),
                });
            }
        }

        // Проверяем проблемы с памятью
        let memory_usage_percent = if metrics.total_memory_usage > 0 {
            metrics.total_memory_usage as f64 / metrics.max_cache_size as f64
        } else {
            0.0
        };

        if memory_usage_percent > self.config.max_memory_usage_warning * 1.1 {
            problems.push(CacheProblem {
                problem_type: CacheProblemType::HighMemoryUsage,
                severity: CacheProblemSeverity::Critical,
                description: format!(
                    "Cache memory usage is very high: {:.2}% (threshold: {:.2}%)",
                    memory_usage_percent * 100.0, self.config.max_memory_usage_warning * 100.0
                ),
                affected_caches: "All caches".to_string(),
                recommendation: "Consider reducing cache size or TTL, or adding more memory".to_string(),
            });
        }

        if problems.is_empty() {
            debug!("No cache problems detected");
        } else {
            warn!(
                "Detected {} cache problems: {} critical, {} warnings",
                problems.len(),
                problems.iter().filter(|p| p.severity == CacheProblemSeverity::Critical).count(),
                problems.iter().filter(|p| p.severity == CacheProblemSeverity::Warning).count()
            );
        }

        Ok(problems)
    }

    /// Получить последние метрики
    pub fn get_last_metrics(&self) -> Option<CacheMonitorMetrics> {
        self.last_metrics.clone()
    }

    /// Получить историю метрик
    pub fn get_metrics_history(&self) -> Vec<CacheMonitorMetrics> {
        self.metrics_history.clone()
    }

    /// Очистить историю метрик
    pub fn clear_metrics_history(&mut self) {
        self.metrics_history.clear();
        debug!("Cache metrics history cleared");
    }

    /// Экспортировать метрики в JSON
    pub fn export_metrics_to_json(&self, metrics: &CacheMonitorMetrics) -> Result<String> {
        use serde_json::to_string;

        let json_data = serde_json::json!({
            "timestamp": metrics.timestamp,
            "total_caches": metrics.total_caches,
            "total_memory_usage": metrics.total_memory_usage,
            "overall_hit_rate": metrics.overall_hit_rate,
            "overall_miss_rate": metrics.overall_miss_rate,
            "active_caches": metrics.active_caches,
            "inactive_caches": metrics.inactive_caches,
            "average_cache_size": metrics.average_cache_size,
            "max_cache_size": metrics.max_cache_size,
            "min_cache_size": metrics.min_cache_size,
            "cache_type_metrics": metrics.cache_type_metrics,
            "cache_priority_metrics": metrics.cache_priority_metrics,
            "usage_trends": metrics.usage_trends,
            "optimization_recommendations": metrics.optimization_recommendations,
        });

        to_string(&json_data).context("Не удалось сериализовать метрики кэша в JSON")
    }
}

/// Рекомендация по оптимизации кэша
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheOptimizationRecommendation {
    /// Тип кэша
    pub cache_type: String,
    /// Текущий размер кэша
    pub current_size: u64,
    /// Рекомендуемый размер кэша
    pub recommended_size: u64,
    /// Текущий TTL
    pub current_ttl: u32,
    /// Рекомендуемый TTL
    pub recommended_ttl: u32,
    /// Приоритет
    pub priority: u32,
    /// Причина рекомендации
    pub reason: String,
}

/// Проблема с кэшем
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheProblem {
    /// Тип проблемы
    pub problem_type: CacheProblemType,
    /// Серьезность проблемы
    pub severity: CacheProblemSeverity,
    /// Описание проблемы
    pub description: String,
    /// Затрагиваемые кэши
    pub affected_caches: String,
    /// Рекомендация по устранению
    pub recommendation: String,
}

/// Тип проблемы с кэшем
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CacheProblemType {
    LowHitRate,
    HighMissRate,
    HighMemoryUsage,
    CacheThrashing,
    MemoryLeak,
    StaleData,
    InconsistentState,
}

/// Серьезность проблемы с кэшем
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CacheProblemSeverity {
    Info,
    Warning,
    Critical,
}

/// Тесты для модуля мониторинга кэша
#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::cache::{MetricsCache, MetricsCacheConfig};

    #[test]
    fn test_cache_monitor_creation() {
        let config = CacheMonitorConfig::default();
        let monitor = CacheMonitor::new(config);
        assert_eq!(monitor.config.monitoring_interval_secs, 60);
        assert!(monitor.config.enable_extended_monitoring);
    }

    #[test]
    fn test_cache_monitor_default() {
        let monitor = CacheMonitor::new_default();
        assert_eq!(monitor.config.monitoring_interval_secs, 60);
        assert!(monitor.config.enable_problem_detection);
    }

    #[test]
    fn test_cache_monitor_with_history_size() {
        let config = CacheMonitorConfig::default();
        let monitor = CacheMonitor::with_history_size(config, 20);
        assert_eq!(monitor.max_history_size, 20);
    }

    #[test]
    fn test_cache_metrics_collection() {
        let config = CacheMonitorConfig::default();
        let mut monitor = CacheMonitor::new(config);

        // Создаем тестовые кэши
        let cache_config = MetricsCacheConfig::default();
        let cache1 = MetricsCache::new(cache_config.clone());
        let cache2 = MetricsCache::new(cache_config.clone());

        // Добавляем тестовые данные
        let metrics = crate::metrics::system::SystemMetrics::default();
        let mut source_paths = std::collections::HashMap::new();
        source_paths.insert("test".to_string(), std::path::PathBuf::from("/proc/test"));

        cache1.insert(
            "test_key1".to_string(),
            metrics.clone(),
            source_paths.clone(),
            "test_metrics".to_string(),
        );
        cache2.insert(
            "test_key2".to_string(),
            metrics.clone(),
            source_paths.clone(),
            "test_metrics".to_string(),
        );

        // Собираем метрики
        let caches = vec![cache1, cache2];
        let result = monitor.collect_cache_metrics(&caches);
        assert!(result.is_ok());
        let metrics = result.unwrap();

        assert_eq!(metrics.total_caches, 2);
        assert!(metrics.total_memory_usage > 0);
        assert!(metrics.overall_hit_rate >= 0.0);
        assert!(metrics.overall_miss_rate >= 0.0);
    }

    #[test]
    fn test_cache_metrics_empty() {
        let config = CacheMonitorConfig::default();
        let mut monitor = CacheMonitor::new(config);

        // Собираем метрики без кэшей
        let caches: Vec<MetricsCache> = Vec::new();
        let result = monitor.collect_cache_metrics(&caches);
        assert!(result.is_ok());
        let metrics = result.unwrap();

        assert_eq!(metrics.total_caches, 0);
        assert_eq!(metrics.total_memory_usage, 0);
        assert_eq!(metrics.overall_hit_rate, 0.0);
        assert_eq!(metrics.overall_miss_rate, 0.0);
    }

    #[test]
    fn test_cache_optimization_recommendations() {
        let config = CacheMonitorConfig::default();
        let monitor = CacheMonitor::new(config);

        let mut metrics = CacheMonitorMetrics::default();
        metrics.overall_hit_rate = 0.6; // Below threshold
        metrics.overall_miss_rate = 0.4; // Above threshold
        metrics.total_memory_usage = 1000000;
        metrics.max_cache_size = 1000000;
        metrics.inactive_caches = 5;
        metrics.total_caches = 10;

        let recommendations = monitor.generate_optimization_recommendations(&metrics);
        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.contains("Low cache hit rate")));
        assert!(recommendations.iter().any(|r| r.contains("High cache miss rate")));
        assert!(recommendations.iter().any(|r| r.contains("High number of inactive caches")));
    }

    #[test]
    fn test_cache_problem_detection() {
        let config = CacheMonitorConfig::default();
        let monitor = CacheMonitor::new(config);

        let mut metrics = CacheMonitorMetrics::default();
        metrics.overall_hit_rate = 0.6; // Below threshold
        metrics.overall_miss_rate = 0.4; // Above threshold
        metrics.total_memory_usage = 1000000;
        metrics.max_cache_size = 1000000;

        let problems = monitor.detect_cache_problems(&metrics);
        assert!(problems.is_ok());
        let problems = problems.unwrap();
        assert!(!problems.is_empty());
        assert!(problems.iter().any(|p| matches!(p.problem_type, CacheProblemType::LowHitRate)));
        assert!(problems.iter().any(|p| matches!(p.problem_type, CacheProblemType::HighMissRate)));
    }

    #[test]
    fn test_cache_metrics_history() {
        let config = CacheMonitorConfig::default();
        let mut monitor = CacheMonitor::with_history_size(config, 3);

        // Создаем тестовые кэши
        let cache_config = MetricsCacheConfig::default();
        let cache = MetricsCache::new(cache_config);

        // Собираем метрики несколько раз
        for i in 0..5 {
            let caches = vec![cache.clone()];
            let result = monitor.collect_cache_metrics(&caches);
            assert!(result.is_ok());
        }

        // Проверяем, что история не превышает максимальный размер
        assert_eq!(monitor.metrics_history.len(), 3);
    }

    #[test]
    fn test_cache_metrics_export() {
        let config = CacheMonitorConfig::default();
        let mut monitor = CacheMonitor::new(config);

        // Создаем тестовые метрики
        let mut metrics = CacheMonitorMetrics::default();
        metrics.total_caches = 2;
        metrics.total_memory_usage = 1000000;
        metrics.overall_hit_rate = 0.8;

        // Экспортируем в JSON
        let json_result = monitor.export_metrics_to_json(&metrics);
        assert!(json_result.is_ok());
        let json_string = json_result.unwrap();
        assert!(json_string.contains("total_caches"));
        assert!(json_string.contains("1000000"));
        assert!(json_string.contains("0.8"));
    }

    #[test]
    fn test_cache_monitor_trends() {
        let config = CacheMonitorConfig::default();
        let mut monitor = CacheMonitor::new(config);

        // Создаем тестовые кэши
        let cache_config = MetricsCacheConfig::default();
        let cache = MetricsCache::new(cache_config);

        // Собираем начальные метрики
        let caches = vec![cache.clone()];
        let result = monitor.collect_cache_metrics(&caches);
        assert!(result.is_ok());

        // Собираем метрики еще раз для анализа трендов
        let result = monitor.collect_cache_metrics(&caches);
        assert!(result.is_ok());
        let metrics = result.unwrap();

        // Проверяем, что тренды рассчитаны
        assert!(!metrics.usage_trends.hit_rate_trend.is_nan());
        assert!(!metrics.usage_trends.memory_usage_trend.is_nan());
    }
}
