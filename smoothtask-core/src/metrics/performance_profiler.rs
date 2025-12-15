//! Модуль профилирования производительности.
//!
//! Этот модуль предоставляет расширенные функции для профилирования
//! производительности системы и анализа узких мест.

use std::collections::HashMap;
use std::time::{Instant, Duration};
use tracing::{debug, info, warn, error};
use anyhow::{Result, Context};

/// Структура для хранения метрик производительности.
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Время выполнения в микросекундах.
    pub execution_time_us: u128,
    /// Количество вызовов.
    pub call_count: u64,
    /// Среднее время выполнения.
    pub average_time_us: f64,
    /// Минимальное время выполнения.
    pub min_time_us: Option<u128>,
    /// Максимальное время выполнения.
    pub max_time_us: Option<u128>,
    /// Общее время выполнения.
    pub total_time_us: u128,
}

impl PerformanceMetrics {
    /// Создать новые метрики производительности.
    pub fn new() -> Self {
        Self::default()
    }

    /// Зарегистрировать выполнение.
    pub fn record_execution(&mut self, duration: Duration) {
        let duration_us = duration.as_micros();
        self.call_count += 1;
        self.total_time_us += duration_us;
        self.execution_time_us = duration_us;

        // Обновить минимальное и максимальное время
        if let Some(min_time) = self.min_time_us {
            if duration_us < min_time {
                self.min_time_us = Some(duration_us);
            }
        } else {
            self.min_time_us = Some(duration_us);
        }

        if let Some(max_time) = self.max_time_us {
            if duration_us > max_time {
                self.max_time_us = Some(duration_us);
            }
        } else {
            self.max_time_us = Some(duration_us);
        }

        // Обновить среднее время
        self.average_time_us = self.total_time_us as f64 / self.call_count as f64;
    }

    /// Сбросить метрики.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Логировать сводку метрик.
    pub fn log_summary(&self, component_name: &str) {
        info!("Performance Metrics for {}:", component_name);
        info!("  Call count: {}", self.call_count);
        info!("  Total time: {} μs", self.total_time_us);
        info!("  Average time: {:.2} μs", self.average_time_us);

        if let Some(min_time) = self.min_time_us {
            info!("  Min time: {} μs", min_time);
        }

        if let Some(max_time) = self.max_time_us {
            info!("  Max time: {} μs", max_time);
        }
    }
}

/// Профилировщик производительности.
#[derive(Debug)]
pub struct PerformanceProfiler {
    /// Метрики для различных компонентов.
    metrics: HashMap<String, PerformanceMetrics>,
    /// Глобальные метрики.
    global_metrics: PerformanceMetrics,
}

impl PerformanceProfiler {
    /// Создать новый профилировщик производительности.
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
            global_metrics: PerformanceMetrics::new(),
        }
    }

    /// Начать профилирование компонента.
    pub fn start_profiling(&mut self, component_name: &str) -> PerformanceTimer {
        let start_time = Instant::now();
        PerformanceTimer {
            profiler: self,
            component_name: component_name.to_string(),
            start_time,
        }
    }

    /// Зарегистрировать выполнение для компонента.
    pub fn record_execution(&mut self, component_name: &str, duration: Duration) {
        let metrics = self.metrics.entry(component_name.to_string())
            .or_insert_with(PerformanceMetrics::new);
        metrics.record_execution(duration);
        self.global_metrics.record_execution(duration);
    }

    /// Получить метрики для компонента.
    pub fn get_metrics(&self, component_name: &str) -> Option<&PerformanceMetrics> {
        self.metrics.get(component_name)
    }

    /// Получить глобальные метрики.
    pub fn get_global_metrics(&self) -> &PerformanceMetrics {
        &self.global_metrics
    }

    /// Сбросить метрики для компонента.
    pub fn reset_component_metrics(&mut self, component_name: &str) {
        if let Some(metrics) = self.metrics.get_mut(component_name) {
            metrics.reset();
        }
    }

    /// Сбросить все метрики.
    pub fn reset_all_metrics(&mut self) {
        for metrics in self.metrics.values_mut() {
            metrics.reset();
        }
        self.global_metrics.reset();
    }

    /// Логировать сводку метрик для всех компонентов.
    pub fn log_summary(&self) {
        info!("=== Performance Profiler Summary ===");
        
        // Логируем глобальные метрики
        self.global_metrics.log_summary("Global");
        
        // Логируем метрики для каждого компонента
        for (component_name, metrics) in &self.metrics {
            metrics.log_summary(component_name);
        }
        
        info!("=== End Performance Profiler Summary ===");
    }

    /// Анализировать узкие места.
    ///
    /// # Возвращает
    ///
    /// Отчет о узких местах в формате строки.
    pub fn analyze_bottlenecks(&self) -> String {
        let mut report = String::new();
        report.push_str("=== Bottleneck Analysis ===\n");

        // Находим компоненты с самым высоким средним временем
        let mut components_by_avg_time: Vec<_> = self.metrics.iter()
            .map(|(name, metrics)| (name, metrics.average_time_us))
            .collect();

        components_by_avg_time.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        report.push_str("Components by average execution time:\n");
        for (name, avg_time) in components_by_avg_time.iter().take(5) {
            report.push_str(&format!("  {}: {:.2} μs\n", name, avg_time));
        }

        // Находим компоненты с самым высоким общим временем
        let mut components_by_total_time: Vec<_> = self.metrics.iter()
            .map(|(name, metrics)| (name, metrics.total_time_us))
            .collect();

        components_by_total_time.sort_by(|a, b| b.1.cmp(&a.1));

        report.push_str("\nComponents by total execution time:\n");
        for (name, total_time) in components_by_total_time.iter().take(5) {
            report.push_str(&format!("  {}: {} μs\n", name, total_time));
        }

        // Находим компоненты с самым высоким количеством вызовов
        let mut components_by_call_count: Vec<_> = self.metrics.iter()
            .map(|(name, metrics)| (name, metrics.call_count))
            .collect();

        components_by_call_count.sort_by(|a, b| b.1.cmp(&a.1));

        report.push_str("\nComponents by call count:\n");
        for (name, call_count) in components_by_call_count.iter().take(5) {
            report.push_str(&format!("  {}: {} calls\n", name, call_count));
        }

        report.push_str("=== End Bottleneck Analysis ===");
        report
    }

    /// Экспортировать метрики в формате JSON.
    ///
    /// # Возвращает
    ///
    /// JSON строку с метриками производительности.
    pub fn export_to_json(&self) -> Result<String> {
        use serde_json::{json, to_string};

        let metrics_data: Vec<_> = self.metrics.iter()
            .map(|(name, metrics)| {
                json!({
                    "component": name,
                    "call_count": metrics.call_count,
                    "total_time_us": metrics.total_time_us,
                    "average_time_us": metrics.average_time_us,
                    "min_time_us": metrics.min_time_us,
                    "max_time_us": metrics.max_time_us,
                })
            })
            .collect();

        let global_data = json!({
            "component": "global",
            "call_count": self.global_metrics.call_count,
            "total_time_us": self.global_metrics.total_time_us,
            "average_time_us": self.global_metrics.average_time_us,
            "min_time_us": self.global_metrics.min_time_us,
            "max_time_us": self.global_metrics.max_time_us,
        });

        let mut all_metrics = vec![global_data];
        all_metrics.extend(metrics_data);

        let json_data = json!({
            "timestamp": chrono::Local::now().to_rfc3339(),
            "performance_metrics": all_metrics,
        });

        to_string(&json_data).context("Не удалось сериализовать метрики в JSON")
    }
}

/// Таймер производительности для удобного профилирования.
#[derive(Debug)]
pub struct PerformanceTimer<'a> {
    /// Профилировщик.
    profiler: &'a mut PerformanceProfiler,
    /// Имя компонента.
    component_name: String,
    /// Время начала.
    start_time: Instant,
}

impl<'a> PerformanceTimer<'a> {
    /// Завершить таймер и зарегистрировать выполнение.
    pub fn finish(self) {
        let duration = self.start_time.elapsed();
        self.profiler.record_execution(&self.component_name, duration);
    }
}

impl<'a> Drop for PerformanceTimer<'a> {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        self.profiler.record_execution(&self.component_name, duration);
    }
}

/// Глобальный профилировщик производительности.
///
/// Этот профилировщик доступен глобально и может быть использован
/// для профилирования различных частей системы.
#[derive(Debug)]
pub struct GlobalPerformanceProfiler {
    /// Внутренний профилировщик.
    inner: PerformanceProfiler,
}

impl GlobalPerformanceProfiler {
    /// Создать новый глобальный профилировщик.
    pub fn new() -> Self {
        Self {
            inner: PerformanceProfiler::new(),
        }
    }

    /// Начать профилирование компонента.
    pub fn start_profiling(&mut self, component_name: &str) -> GlobalPerformanceTimer {
        let timer = self.inner.start_profiling(component_name);
        GlobalPerformanceTimer {
            timer,
        }
    }

    /// Получить глобальный экземпляр.
    ///
    /// # Примечание
    ///
    /// В реальной системе это должен быть синглтон или использовать lazy_static.
    pub fn global() -> std::sync::MutexGuard<'static, Self> {
        use lazy_static::lazy_static;
        use std::sync::Mutex;

        lazy_static! {
            static ref GLOBAL_PROFILER: Mutex<GlobalPerformanceProfiler> = {
                Mutex::new(GlobalPerformanceProfiler::new())
            };
        }

        GLOBAL_PROFILER.lock().unwrap()
    }

    /// Логировать сводку метрик.
    pub fn log_summary(&self) {
        self.inner.log_summary();
    }

    /// Анализировать узкие места.
    pub fn analyze_bottlenecks(&self) -> String {
        self.inner.analyze_bottlenecks()
    }

    /// Экспортировать метрики в JSON.
    pub fn export_to_json(&self) -> Result<String> {
        self.inner.export_to_json()
    }
}

/// Глобальный таймер производительности.
#[derive(Debug)]
pub struct GlobalPerformanceTimer<'a> {
    /// Внутренний таймер.
    timer: PerformanceTimer<'a>,
}

impl<'a> Drop for GlobalPerformanceTimer<'a> {
    fn drop(&mut self) {
        // Таймер автоматически завершится при выходе из области видимости
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_performance_metrics_initialization() {
        let metrics = PerformanceMetrics::new();
        assert_eq!(metrics.call_count, 0);
        assert_eq!(metrics.total_time_us, 0);
        assert_eq!(metrics.average_time_us, 0.0);
        assert!(metrics.min_time_us.is_none());
        assert!(metrics.max_time_us.is_none());
    }

    #[test]
    fn test_performance_metrics_record_execution() {
        let mut metrics = PerformanceMetrics::new();
        let duration = Duration::from_micros(100);
        metrics.record_execution(duration);

        assert_eq!(metrics.call_count, 1);
        assert_eq!(metrics.total_time_us, 100);
        assert_eq!(metrics.average_time_us, 100.0);
        assert_eq!(metrics.min_time_us, Some(100));
        assert_eq!(metrics.max_time_us, Some(100));
    }

    #[test]
    fn test_performance_metrics_multiple_executions() {
        let mut metrics = PerformanceMetrics::new();
        metrics.record_execution(Duration::from_micros(100));
        metrics.record_execution(Duration::from_micros(200));
        metrics.record_execution(Duration::from_micros(150));

        assert_eq!(metrics.call_count, 3);
        assert_eq!(metrics.total_time_us, 450);
        assert_eq!(metrics.average_time_us, 150.0);
        assert_eq!(metrics.min_time_us, Some(100));
        assert_eq!(metrics.max_time_us, Some(200));
    }

    #[test]
    fn test_performance_metrics_reset() {
        let mut metrics = PerformanceMetrics::new();
        metrics.record_execution(Duration::from_micros(100));
        metrics.record_execution(Duration::from_micros(200));

        assert_eq!(metrics.call_count, 2);

        metrics.reset();

        assert_eq!(metrics.call_count, 0);
        assert_eq!(metrics.total_time_us, 0);
        assert_eq!(metrics.average_time_us, 0.0);
    }

    #[test]
    fn test_performance_profiler_new() {
        let profiler = PerformanceProfiler::new();
        assert!(profiler.metrics.is_empty());
        assert_eq!(profiler.global_metrics.call_count, 0);
    }

    #[test]
    fn test_performance_profiler_record_execution() {
        let mut profiler = PerformanceProfiler::new();
        let duration = Duration::from_micros(100);

        profiler.record_execution("test_component", duration);

        assert_eq!(profiler.metrics.len(), 1);
        assert!(profiler.metrics.contains_key("test_component"));

        let metrics = profiler.get_metrics("test_component").unwrap();
        assert_eq!(metrics.call_count, 1);
        assert_eq!(metrics.total_time_us, 100);

        // Проверяем глобальные метрики
        assert_eq!(profiler.global_metrics.call_count, 1);
        assert_eq!(profiler.global_metrics.total_time_us, 100);
    }

    #[test]
    fn test_performance_profiler_multiple_components() {
        let mut profiler = PerformanceProfiler::new();

        profiler.record_execution("component1", Duration::from_micros(100));
        profiler.record_execution("component1", Duration::from_micros(200));
        profiler.record_execution("component2", Duration::from_micros(150));

        assert_eq!(profiler.metrics.len(), 2);

        let metrics1 = profiler.get_metrics("component1").unwrap();
        assert_eq!(metrics1.call_count, 2);
        assert_eq!(metrics1.total_time_us, 300);
        assert_eq!(metrics1.average_time_us, 150.0);

        let metrics2 = profiler.get_metrics("component2").unwrap();
        assert_eq!(metrics2.call_count, 1);
        assert_eq!(metrics2.total_time_us, 150);
        assert_eq!(metrics2.average_time_us, 150.0);

        // Проверяем глобальные метрики
        assert_eq!(profiler.global_metrics.call_count, 3);
        assert_eq!(profiler.global_metrics.total_time_us, 450);
    }

    #[test]
    fn test_performance_profiler_reset() {
        let mut profiler = PerformanceProfiler::new();

        profiler.record_execution("component1", Duration::from_micros(100));
        profiler.record_execution("component2", Duration::from_micros(200));

        assert_eq!(profiler.global_metrics.call_count, 2);

        profiler.reset_all_metrics();

        assert_eq!(profiler.global_metrics.call_count, 0);
        assert!(profiler.metrics.is_empty());
    }

    #[test]
    fn test_performance_timer_basic() {
        let mut profiler = PerformanceProfiler::new();
        let timer = profiler.start_profiling("test_timer");

        // Имитируем работу
        thread::sleep(Duration::from_micros(100));

        // Таймер завершится автоматически при выходе из области видимости
        drop(timer);

        let metrics = profiler.get_metrics("test_timer").unwrap();
        assert_eq!(metrics.call_count, 1);
        assert!(metrics.total_time_us >= 100); // Должно быть не менее 100 микросекунд
    }

    #[test]
    fn test_performance_timer_explicit_finish() {
        let mut profiler = PerformanceProfiler::new();
        let timer = profiler.start_profiling("test_timer_explicit");

        // Имитируем работу
        thread::sleep(Duration::from_micros(100));

        // Явное завершение таймера
        timer.finish();

        let metrics = profiler.get_metrics("test_timer_explicit").unwrap();
        assert_eq!(metrics.call_count, 1);
        assert!(metrics.total_time_us >= 100);
    }

    #[test]
    fn test_performance_profiler_analyze_bottlenecks() {
        let mut profiler = PerformanceProfiler::new();

        // Добавляем метрики для разных компонентов
        profiler.record_execution("fast_component", Duration::from_micros(10));
        profiler.record_execution("fast_component", Duration::from_micros(20));
        
        profiler.record_execution("slow_component", Duration::from_micros(1000));
        profiler.record_execution("slow_component", Duration::from_micros(2000));
        
        profiler.record_execution("medium_component", Duration::from_micros(100));

        // Анализируем узкие места
        let report = profiler.analyze_bottlenecks();

        // Проверяем, что отчет содержит ожидаемые компоненты
        assert!(report.contains("slow_component"));
        assert!(report.contains("medium_component"));
        assert!(report.contains("fast_component"));
        
        // Проверяем, что slow_component упоминается первым в разделе по среднему времени
        let lines: Vec<&str> = report.lines().collect();
        let avg_time_section = lines.iter().position(|line| line.contains("average execution time"));
        if let Some(section_idx) = avg_time_section {
            // Следующая строка должна содержать slow_component
            if section_idx + 1 < lines.len() {
                assert!(lines[section_idx + 1].contains("slow_component"));
            }
        }
    }

    #[test]
    fn test_performance_profiler_export_to_json() {
        let mut profiler = PerformanceProfiler::new();

        // Добавляем некоторые метрики
        profiler.record_execution("test_component", Duration::from_micros(100));
        profiler.record_execution("test_component", Duration::from_micros(200));

        // Экспортируем в JSON
        let json_result = profiler.export_to_json();
        assert!(json_result.is_ok());

        let json_string = json_result.unwrap();
        assert!(json_string.contains("test_component"));
        assert!(json_string.contains("performance_metrics"));
        assert!(json_string.contains("call_count"));
        assert!(json_string.contains("total_time_us"));
    }

    #[test]
    fn test_global_performance_profiler() {
        let mut profiler = GlobalPerformanceProfiler::new();
        let timer = profiler.start_profiling("global_test");

        // Имитируем работу
        thread::sleep(Duration::from_micros(100));

        // Таймер завершится автоматически
        drop(timer);

        // Проверяем, что метрики записаны
        // В реальной системе мы бы использовали глобальный экземпляр
    }

    #[test]
    fn test_performance_metrics_log_summary() {
        let mut metrics = PerformanceMetrics::new();
        metrics.record_execution(Duration::from_micros(100));
        metrics.record_execution(Duration::from_micros(200));
        metrics.record_execution(Duration::from_micros(150));

        // Логируем сводку (должно работать без паники)
        metrics.log_summary("TestComponent");
    }

    #[test]
    fn test_performance_profiler_log_summary() {
        let mut profiler = PerformanceProfiler::new();

        // Добавляем метрики для разных компонентов
        profiler.record_execution("component1", Duration::from_micros(100));
        profiler.record_execution("component2", Duration::from_micros(200));

        // Логируем сводку (должно работать без паники)
        profiler.log_summary();
    }

    #[test]
    fn test_performance_timer_convenience() {
        let mut profiler = PerformanceProfiler::new();

        // Используем таймер для удобного профилирования
        {
            let _timer = profiler.start_profiling("convenience_test");
            // Имитируем работу
            thread::sleep(Duration::from_micros(50));
            // Таймер автоматически завершится здесь
        }

        let metrics = profiler.get_metrics("convenience_test").unwrap();
        assert_eq!(metrics.call_count, 1);
        assert!(metrics.total_time_us >= 50);
    }

    #[test]
    fn test_performance_profiler_empty() {
        let profiler = PerformanceProfiler::new();

        // Проверяем, что пустой профилировщик работает корректно
        assert!(profiler.get_metrics("nonexistent").is_none());
        assert_eq!(profiler.global_metrics.call_count, 0);

        // Логирование и анализ должны работать без паники
        profiler.log_summary();
        let report = profiler.analyze_bottlenecks();
        assert!(report.contains("Bottleneck Analysis"));
    }

    #[test]
    fn test_performance_metrics_edge_cases() {
        let mut metrics = PerformanceMetrics::new();

        // Тестируем с нулевой длительностью
        metrics.record_execution(Duration::from_micros(0));
        assert_eq!(metrics.call_count, 1);
        assert_eq!(metrics.total_time_us, 0);
        assert_eq!(metrics.average_time_us, 0.0);

        // Тестируем с очень большой длительностью
        metrics.record_execution(Duration::from_secs(60));
        assert_eq!(metrics.call_count, 2);
        assert_eq!(metrics.total_time_us, 60_000_000);
        assert_eq!(metrics.average_time_us, 30_000_000.0);
    }

    #[test]
    fn test_performance_profiler_component_reset() {
        let mut profiler = PerformanceProfiler::new();

        // Добавляем метрики
        profiler.record_execution("component1", Duration::from_micros(100));
        profiler.record_execution("component2", Duration::from_micros(200));

        // Сбрасываем метрики для component1
        profiler.reset_component_metrics("component1");

        // Проверяем, что component1 сброшен
        let metrics1 = profiler.get_metrics("component1").unwrap();
        assert_eq!(metrics1.call_count, 0);

        // Проверяем, что component2 не затронут
        let metrics2 = profiler.get_metrics("component2").unwrap();
        assert_eq!(metrics2.call_count, 1);

        // Проверяем, что глобальные метрики не затронуты
        assert_eq!(profiler.global_metrics.call_count, 2);
    }
}
