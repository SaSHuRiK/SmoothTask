//! Метрики производительности приложений.
//!
//! Этот модуль предоставляет функциональность для мониторинга производительности
//! отдельных приложений, включая:
//! - Задержки отклика (latency)
//! - FPS для графических приложений
//! - Использование ресурсов на уровне приложения
//! - Метрики отзывчивости UI

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Глобальный счетчик времени для тестирования.
/// В реальной реализации нужно использовать внешний источник времени.
static GLOBAL_TIME_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn increment_time_counter() -> u128 {
    GLOBAL_TIME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as u128 + 1
}

fn get_current_time_counter() -> u128 {
    GLOBAL_TIME_COUNTER.load(std::sync::atomic::Ordering::SeqCst) as u128
}



/// Метрики производительности для отдельного приложения.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AppPerformanceMetrics {
    /// Идентификатор процесса
    pub pid: u32,
    
    /// Имя процесса
    pub name: String,
    
    /// Средняя задержка отклика (в миллисекундах)
    /// Это может быть задержка обработки событий, задержка рендеринга и т.д.
    pub avg_latency_ms: f64,
    
    /// Максимальная задержка отклика (в миллисекундах)
    pub max_latency_ms: f64,
    
    /// Минимальная задержка отклика (в миллисекундах)
    pub min_latency_ms: f64,
    
    /// Количество замеров задержки
    pub latency_samples: u32,
    
    /// Средний FPS (кадров в секунду) для графических приложений
    /// 0.0 если метрика не применима
    pub avg_fps: f64,
    
    /// Минимальный FPS
    pub min_fps: f64,
    
    /// Максимальный FPS
    pub max_fps: f64,
    
    /// Количество замеров FPS
    pub fps_samples: u32,
    
    /// Среднее использование CPU приложением (в процентах)
    pub avg_cpu_usage: f64,
    
    /// Среднее использование памяти приложением (в мегабайтах)
    pub avg_memory_usage_mb: f64,
    
    /// Количество сбоев/ошибок за период
    pub error_count: u32,
    
    /// Время последнего обновления метрик (в миллисекундах с момента запуска)
    /// Для тестирования используем простой счетчик
    pub last_updated_ms: u128,
}

impl AppPerformanceMetrics {
    /// Создает новые метрики производительности для приложения.
    pub fn new(pid: u32, name: String) -> Self {
        Self {
            pid,
            name,
            avg_latency_ms: 0.0,
            max_latency_ms: 0.0,
            min_latency_ms: f64::MAX,
            latency_samples: 0,
            avg_fps: 0.0,
            min_fps: f64::MAX,
            max_fps: 0.0,
            fps_samples: 0,
            avg_cpu_usage: 0.0,
            avg_memory_usage_mb: 0.0,
            error_count: 0,
            last_updated_ms: 0,
        }
    }
    
    /// Добавляет новый замер задержки.
    pub fn add_latency_sample(&mut self, latency_ms: f64) {
        self.latency_samples += 1;
        
        // Обновляем среднее значение
        if self.latency_samples == 1 {
            self.avg_latency_ms = latency_ms;
        } else {
            self.avg_latency_ms = (self.avg_latency_ms * (self.latency_samples - 1) as f64 + latency_ms) / self.latency_samples as f64;
        }
        
        // Обновляем мин/макс
        self.min_latency_ms = self.min_latency_ms.min(latency_ms);
        self.max_latency_ms = self.max_latency_ms.max(latency_ms);
        
        self.last_updated_ms = increment_time_counter();
    }
    
    /// Добавляет новый замер FPS.
    pub fn add_fps_sample(&mut self, fps: f64) {
        self.fps_samples += 1;
        
        // Обновляем среднее значение
        if self.fps_samples == 1 {
            self.avg_fps = fps;
        } else {
            self.avg_fps = (self.avg_fps * (self.fps_samples - 1) as f64 + fps) / self.fps_samples as f64;
        }
        
        // Обновляем мин/макс
        self.min_fps = self.min_fps.min(fps);
        self.max_fps = self.max_fps.max(fps);
        
        self.last_updated_ms = increment_time_counter();
    }
    
    /// Обновляет использование CPU.
    pub fn update_cpu_usage(&mut self, cpu_usage: f64) {
        // Для CPU мы используем скользящее среднее
        if self.avg_cpu_usage > 0.0 {
            // Вес 0.3 для нового значения, 0.7 для старого
            self.avg_cpu_usage = self.avg_cpu_usage * 0.7 + cpu_usage * 0.3;
        } else {
            self.avg_cpu_usage = cpu_usage;
        }
        self.last_updated_ms = increment_time_counter();
    }
    
    /// Обновляет использование памяти.
    pub fn update_memory_usage(&mut self, memory_usage_mb: f64) {
        // Для памяти мы используем скользящее среднее
        if self.latency_samples > 0 {
            // Вес 0.2 для нового значения, 0.8 для старого
            self.avg_memory_usage_mb = self.avg_memory_usage_mb * 0.8 + memory_usage_mb * 0.2;
        } else {
            self.avg_memory_usage_mb = memory_usage_mb;
        }
        self.last_updated_ms = increment_time_counter();
    }
    
    /// Увеличивает счетчик ошибок.
    pub fn increment_error_count(&mut self) {
        self.error_count += 1;
        self.last_updated_ms = increment_time_counter();
    }
    
    /// Сбрасывает все счетчики и метрики.
    pub fn reset(&mut self) {
        self.avg_latency_ms = 0.0;
        self.max_latency_ms = 0.0;
        self.min_latency_ms = f64::MAX;
        self.latency_samples = 0;
        self.avg_fps = 0.0;
        self.min_fps = f64::MAX;
        self.max_fps = 0.0;
        self.fps_samples = 0;
        self.avg_cpu_usage = 0.0;
        self.avg_memory_usage_mb = 0.0;
        self.error_count = 0;
        self.last_updated_ms = increment_time_counter();
    }
    
    /// Проверяет, устарели ли метрики (не обновлялись дольше заданного времени).
    /// 
    /// Примечание: Эта реализация использует простой счетчик времени с момента последнего обновления.
    /// Для точного отслеживания времени нужно использовать внешний источник времени.
    pub fn is_stale(&self, max_age: Duration) -> bool {
        // Если last_updated_ms == 0, значит метрики никогда не обновлялись
        if self.last_updated_ms == 0 {
            return false;
        }
        
        let current_time = get_current_time_counter();
        let age = current_time.saturating_sub(self.last_updated_ms);
        Duration::from_millis(age as u64) > max_age
    }
}

/// Коллекция метрик производительности для всех приложений.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AppPerformanceCollection {
    /// Метрики для отдельных приложений
    pub apps: HashMap<u32, AppPerformanceMetrics>,
    
    /// Глобальные метрики производительности
    pub global: GlobalAppPerformanceMetrics,
}

impl AppPerformanceCollection {
    /// Создает новую коллекцию метрик.
    pub fn new() -> Self {
        Self {
            apps: HashMap::new(),
            global: GlobalAppPerformanceMetrics::default(),
        }
    }
    
    /// Добавляет или обновляет метрики для приложения.
    pub fn update_app(&mut self, metrics: AppPerformanceMetrics) {
        self.apps.insert(metrics.pid, metrics);
    }
    
    /// Получает метрики для приложения по PID.
    pub fn get_app(&self, pid: u32) -> Option<&AppPerformanceMetrics> {
        self.apps.get(&pid)
    }
    
    /// Удаляет устаревшие метрики (не обновлявшиеся дольше заданного времени).
    pub fn cleanup_stale(&mut self, max_age: Duration) {
        self.apps.retain(|_, metrics| !metrics.is_stale(max_age));
    }
    
    /// Обновляет глобальные метрики на основе текущих метрик приложений.
    pub fn update_global_metrics(&mut self) {
        let mut total_latency = 0.0;
        let mut total_fps = 0.0;
        let mut total_cpu = 0.0;
        let mut total_memory = 0.0;
        let mut total_errors = 0;
        let mut app_count = 0;
        
        for metrics in self.apps.values() {
            if metrics.latency_samples > 0 {
                total_latency += metrics.avg_latency_ms;
                app_count += 1;
            }
            if metrics.fps_samples > 0 {
                total_fps += metrics.avg_fps;
            }
            if metrics.avg_cpu_usage > 0.0 {
                total_cpu += metrics.avg_cpu_usage;
            }
            if metrics.avg_memory_usage_mb > 0.0 {
                total_memory += metrics.avg_memory_usage_mb;
            }
            total_errors += metrics.error_count;
        }
        
        self.global.avg_latency_ms = if app_count > 0 { total_latency / app_count as f64 } else { 0.0 };
        self.global.avg_fps = if app_count > 0 { total_fps / app_count as f64 } else { 0.0 };
        self.global.avg_cpu_usage = if app_count > 0 { total_cpu / app_count as f64 } else { 0.0 };
        self.global.avg_memory_usage_mb = if app_count > 0 { total_memory / app_count as f64 } else { 0.0 };
        self.global.total_error_count = total_errors;
        self.global.active_app_count = app_count;
    }
}

/// Глобальные метрики производительности приложений.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GlobalAppPerformanceMetrics {
    /// Средняя задержка отклика по всем приложениям
    pub avg_latency_ms: f64,
    
    /// Средний FPS по всем приложениям
    pub avg_fps: f64,
    
    /// Среднее использование CPU по всем приложениям
    pub avg_cpu_usage: f64,
    
    /// Среднее использование памяти по всем приложениям
    pub avg_memory_usage_mb: f64,
    
    /// Общее количество ошибок по всем приложениям
    pub total_error_count: u32,
    
    /// Количество активных приложений с метриками
    pub active_app_count: u32,
}

/// Коллектор метрик производительности приложений.
/// Отвечает за сбор метрик с различных источников.
#[derive(Debug)]
pub struct AppPerformanceCollector {
    /// Текущая коллекция метрик
    collection: AppPerformanceCollection,
    
    /// Максимальный возраст метрик перед удалением
    max_metrics_age: Duration,
}

impl AppPerformanceCollector {
    /// Создает новый коллектор.
    pub fn new(max_metrics_age: Duration) -> Self {
        Self {
            collection: AppPerformanceCollection::new(),
            max_metrics_age,
        }
    }
    
    /// Собирает метрики производительности приложений.
    /// В текущей реализации это заглушка, которая будет расширена.
    pub fn collect(&mut self) -> Result<&AppPerformanceCollection> {
        // Пока это заглушка - в будущем здесь будет реальный сбор метрик
        // из различных источников (eBPF, системные вызовы, интеграция с приложениями)
        
        // Очищаем устаревшие метрики
        self.collection.cleanup_stale(self.max_metrics_age);
        
        // Обновляем глобальные метрики
        self.collection.update_global_metrics();
        
        Ok(&self.collection)
    }
    
    /// Получает текущую коллекцию метрик.
    pub fn collection(&self) -> &AppPerformanceCollection {
        &self.collection
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_app_performance_metrics_new() {
        let metrics = AppPerformanceMetrics::new(1234, "test_app".to_string());
        assert_eq!(metrics.pid, 1234);
        assert_eq!(metrics.name, "test_app");
        assert_eq!(metrics.latency_samples, 0);
        assert_eq!(metrics.fps_samples, 0);
    }

    #[test]
    fn test_add_latency_sample() {
        let mut metrics = AppPerformanceMetrics::new(1234, "test_app".to_string());
        
        metrics.add_latency_sample(10.0);
        assert_eq!(metrics.latency_samples, 1);
        assert_eq!(metrics.avg_latency_ms, 10.0);
        assert_eq!(metrics.min_latency_ms, 10.0);
        assert_eq!(metrics.max_latency_ms, 10.0);
        
        metrics.add_latency_sample(20.0);
        assert_eq!(metrics.latency_samples, 2);
        assert_eq!(metrics.avg_latency_ms, 15.0);
        assert_eq!(metrics.min_latency_ms, 10.0);
        assert_eq!(metrics.max_latency_ms, 20.0);
    }

    #[test]
    fn test_add_fps_sample() {
        let mut metrics = AppPerformanceMetrics::new(1234, "test_app".to_string());
        
        metrics.add_fps_sample(60.0);
        assert_eq!(metrics.fps_samples, 1);
        assert_eq!(metrics.avg_fps, 60.0);
        assert_eq!(metrics.min_fps, 60.0);
        assert_eq!(metrics.max_fps, 60.0);
        
        metrics.add_fps_sample(30.0);
        assert_eq!(metrics.fps_samples, 2);
        assert_eq!(metrics.avg_fps, 45.0);
        assert_eq!(metrics.min_fps, 30.0);
        assert_eq!(metrics.max_fps, 60.0);
    }

    #[test]
    fn test_update_cpu_usage() {
        let mut metrics = AppPerformanceMetrics::new(1234, "test_app".to_string());
        
        metrics.update_cpu_usage(50.0);
        assert_eq!(metrics.avg_cpu_usage, 50.0);
        
        metrics.update_cpu_usage(75.0);
        // 50 * 0.7 + 75 * 0.3 = 35 + 22.5 = 57.5
        assert_eq!(metrics.avg_cpu_usage, 57.5);
    }

    #[test]
    fn test_is_stale() {
        let mut metrics = AppPerformanceMetrics::new(1234, "test_app".to_string());
        
        // Новые метрики (не обновлявшиеся) не должны быть устаревшими
        assert!(!metrics.is_stale(Duration::from_secs(1)));
        
        // Обновим метрики
        metrics.add_latency_sample(10.0);
        
        // Имитируем прохождение времени, увеличивая счетчик
        // В реальном сценарии это бы сделало время, но для теста мы делаем это вручную
        for _ in 0..100 {
            increment_time_counter();
        }
        
        assert!(metrics.is_stale(Duration::from_millis(50)));
    }

    #[test]
    fn test_app_performance_collection() {
        let mut collection = AppPerformanceCollection::new();
        
        let metrics1 = AppPerformanceMetrics::new(1, "app1".to_string());
        let metrics2 = AppPerformanceMetrics::new(2, "app2".to_string());
        
        collection.update_app(metrics1);
        collection.update_app(metrics2);
        
        assert_eq!(collection.apps.len(), 2);
        assert!(collection.get_app(1).is_some());
        assert!(collection.get_app(2).is_some());
        assert!(collection.get_app(3).is_none());
    }

    #[test]
    fn test_cleanup_stale() {
        let mut collection = AppPerformanceCollection::new();
        
        let mut metrics1 = AppPerformanceMetrics::new(1, "app1".to_string());
        let mut metrics2 = AppPerformanceMetrics::new(2, "app2".to_string());
        
        // Обновим метрики, чтобы они имели timestamp
        metrics1.add_latency_sample(10.0);
        metrics2.add_latency_sample(20.0);
        
        // Добавим метрики
        collection.update_app(metrics1);
        collection.update_app(metrics2);
        
        // Подождем немного
        thread::sleep(Duration::from_millis(100));
        
        // Очистим устаревшие (с возрастом 50мс)
        collection.cleanup_stale(Duration::from_millis(50));
        
        // Обе метрики должны быть удалены
        assert_eq!(collection.apps.len(), 0);
    }

    #[test]
    fn test_update_global_metrics() {
        let mut collection = AppPerformanceCollection::new();
        
        let mut metrics1 = AppPerformanceMetrics::new(1, "app1".to_string());
        metrics1.add_latency_sample(10.0);
        metrics1.update_cpu_usage(50.0);
        
        let mut metrics2 = AppPerformanceMetrics::new(2, "app2".to_string());
        metrics2.add_latency_sample(20.0);
        metrics2.update_cpu_usage(75.0);
        
        collection.update_app(metrics1);
        collection.update_app(metrics2);
        
        collection.update_global_metrics();
        
        assert_eq!(collection.global.avg_latency_ms, 15.0);
        assert_eq!(collection.global.avg_cpu_usage, 62.5);
        assert_eq!(collection.global.active_app_count, 2);
    }

    #[test]
    fn test_collector() {
        let mut collector = AppPerformanceCollector::new(Duration::from_secs(1));
        
        let result = collector.collect();
        assert!(result.is_ok());
        
        let collection = result.unwrap();
        assert_eq!(collection.apps.len(), 0);
        assert_eq!(collection.global.active_app_count, 0);
    }
}
