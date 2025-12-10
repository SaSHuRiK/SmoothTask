//! Модуль для измерения scheduling latency через probe-thread (mini-cyclictest).
//!
//! Пока реализована только структура для хранения измерений и вычисления перцентилей.
//! Полная реализация probe-thread будет добавлена в задаче ST-045.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Коллектор для хранения измерений scheduling latency и вычисления перцентилей.
///
/// Хранит последние N измерений в скользящем окне и вычисляет перцентили P95 и P99.
/// Потокобезопасен для использования из probe-thread.
#[derive(Debug, Clone)]
pub struct LatencyCollector {
    /// Максимальное количество измерений в окне
    max_samples: usize,
    /// Измерения latency в миллисекундах (скользящее окно)
    samples: Arc<Mutex<VecDeque<f64>>>,
}

impl LatencyCollector {
    /// Создаёт новый LatencyCollector с указанным размером окна.
    ///
    /// # Аргументы
    ///
    /// * `max_samples` - максимальное количество измерений в окне (рекомендуется 1000-5000)
    ///
    /// # Пример
    ///
    /// ```
    /// use smoothtask_core::metrics::scheduling_latency::LatencyCollector;
    ///
    /// let collector = LatencyCollector::new(1000);
    /// ```
    pub fn new(max_samples: usize) -> Self {
        Self {
            max_samples,
            samples: Arc::new(Mutex::new(VecDeque::with_capacity(max_samples))),
        }
    }

    /// Добавляет новое измерение latency в миллисекундах.
    ///
    /// Если окно заполнено, удаляется самое старое измерение.
    ///
    /// # Аргументы
    ///
    /// * `latency_ms` - измеренная latency в миллисекундах
    ///
    /// # Пример
    ///
    /// ```
    /// use smoothtask_core::metrics::scheduling_latency::LatencyCollector;
    ///
    /// let collector = LatencyCollector::new(1000);
    /// collector.add_sample(5.2);
    /// collector.add_sample(3.1);
    /// ```
    pub fn add_sample(&self, latency_ms: f64) {
        let mut samples = self.samples.lock().unwrap();
        if samples.len() >= self.max_samples {
            samples.pop_front();
        }
        samples.push_back(latency_ms);
    }

    /// Вычисляет перцентиль из текущих измерений.
    ///
    /// # Аргументы
    ///
    /// * `percentile` - перцентиль (0.0-1.0), например 0.95 для P95, 0.99 для P99
    ///
    /// # Возвращает
    ///
    /// * `Some(value)` - значение перцентиля в миллисекундах, если есть измерения
    /// * `None` - если измерений недостаточно (меньше 2)
    ///
    /// # Пример
    ///
    /// ```
    /// use smoothtask_core::metrics::scheduling_latency::LatencyCollector;
    ///
    /// let collector = LatencyCollector::new(1000);
    /// collector.add_sample(5.0);
    /// collector.add_sample(10.0);
    /// collector.add_sample(15.0);
    ///
    /// let p95 = collector.percentile(0.95);
    /// assert!(p95.is_some());
    /// ```
    pub fn percentile(&self, percentile: f64) -> Option<f64> {
        let samples = self.samples.lock().unwrap();
        if samples.len() < 2 {
            return None;
        }

        // Копируем измерения в вектор и сортируем
        let mut sorted: Vec<f64> = samples.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Вычисляем индекс перцентиля
        let index = (sorted.len() as f64 * percentile).ceil() as usize - 1;
        let index = index.min(sorted.len() - 1);

        Some(sorted[index])
    }

    /// Вычисляет P95 перцентиль.
    ///
    /// # Возвращает
    ///
    /// * `Some(value)` - P95 в миллисекундах, если есть измерения
    /// * `None` - если измерений недостаточно
    pub fn p95(&self) -> Option<f64> {
        self.percentile(0.95)
    }

    /// Вычисляет P99 перцентиль.
    ///
    /// # Возвращает
    ///
    /// * `Some(value)` - P99 в миллисекундах, если есть измерения
    /// * `None` - если измерений недостаточно
    pub fn p99(&self) -> Option<f64> {
        self.percentile(0.99)
    }

    /// Возвращает количество измерений в окне.
    pub fn len(&self) -> usize {
        self.samples.lock().unwrap().len()
    }

    /// Проверяет, пусто ли окно измерений.
    pub fn is_empty(&self) -> bool {
        self.samples.lock().unwrap().is_empty()
    }

    /// Очищает все измерения.
    pub fn clear(&self) {
        self.samples.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_collector_new() {
        let collector = LatencyCollector::new(100);
        assert_eq!(collector.max_samples, 100);
        assert!(collector.is_empty());
    }

    #[test]
    fn test_latency_collector_add_sample() {
        let collector = LatencyCollector::new(10);
        collector.add_sample(5.0);
        collector.add_sample(10.0);
        assert_eq!(collector.len(), 2);
    }

    #[test]
    fn test_latency_collector_max_samples() {
        let collector = LatencyCollector::new(3);
        collector.add_sample(1.0);
        collector.add_sample(2.0);
        collector.add_sample(3.0);
        assert_eq!(collector.len(), 3);
        collector.add_sample(4.0);
        assert_eq!(collector.len(), 3); // Окно не должно превышать max_samples
    }

    #[test]
    fn test_latency_collector_percentile_empty() {
        let collector = LatencyCollector::new(100);
        assert_eq!(collector.percentile(0.95), None);
    }

    #[test]
    fn test_latency_collector_percentile_single() {
        let collector = LatencyCollector::new(100);
        collector.add_sample(5.0);
        // Нужно минимум 2 измерения для перцентиля
        assert_eq!(collector.percentile(0.95), None);
    }

    #[test]
    fn test_latency_collector_percentile_two_samples() {
        let collector = LatencyCollector::new(100);
        collector.add_sample(5.0);
        collector.add_sample(10.0);
        let p95 = collector.percentile(0.95);
        assert!(p95.is_some());
        // P95 из двух значений должен быть максимальным
        assert_eq!(p95.unwrap(), 10.0);
    }

    #[test]
    fn test_latency_collector_percentile_multiple_samples() {
        let collector = LatencyCollector::new(100);
        // Добавляем 100 значений от 1.0 до 100.0
        for i in 1..=100 {
            collector.add_sample(i as f64);
        }
        let p95 = collector.percentile(0.95);
        assert!(p95.is_some());
        // P95 из 100 значений должен быть около 95
        assert!((p95.unwrap() - 95.0).abs() < 1.0);
    }

    #[test]
    fn test_latency_collector_p95() {
        let collector = LatencyCollector::new(100);
        collector.add_sample(5.0);
        collector.add_sample(10.0);
        collector.add_sample(15.0);
        let p95 = collector.p95();
        assert!(p95.is_some());
        // P95 из трёх значений должен быть максимальным
        assert_eq!(p95.unwrap(), 15.0);
    }

    #[test]
    fn test_latency_collector_p99() {
        let collector = LatencyCollector::new(100);
        collector.add_sample(5.0);
        collector.add_sample(10.0);
        collector.add_sample(15.0);
        let p99 = collector.p99();
        assert!(p99.is_some());
        // P99 из трёх значений должен быть максимальным
        assert_eq!(p99.unwrap(), 15.0);
    }

    #[test]
    fn test_latency_collector_clear() {
        let collector = LatencyCollector::new(100);
        collector.add_sample(5.0);
        collector.add_sample(10.0);
        assert_eq!(collector.len(), 2);
        collector.clear();
        assert!(collector.is_empty());
    }

    #[test]
    fn test_latency_collector_thread_safety() {
        use std::thread;
        let collector = Arc::new(LatencyCollector::new(1000));
        let mut handles = vec![];

        // Создаём несколько потоков, которые добавляют измерения
        for i in 0..10 {
            let collector_clone = Arc::clone(&collector);
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    collector_clone.add_sample((i * 100 + j) as f64);
                }
            });
            handles.push(handle);
        }

        // Ждём завершения всех потоков
        for handle in handles {
            handle.join().unwrap();
        }

        // Проверяем, что все измерения добавлены
        assert_eq!(collector.len(), 1000); // Окно ограничено 1000
    }

    #[test]
    fn test_latency_collector_percentile_accuracy() {
        let collector = LatencyCollector::new(1000);
        // Добавляем значения от 1.0 до 1000.0
        for i in 1..=1000 {
            collector.add_sample(i as f64);
        }

        let p50 = collector.percentile(0.5).unwrap();
        let p95 = collector.percentile(0.95).unwrap();
        let p99 = collector.percentile(0.99).unwrap();

        // P50 должен быть около 500
        assert!((p50 - 500.0).abs() < 10.0);
        // P95 должен быть около 950
        assert!((p95 - 950.0).abs() < 10.0);
        // P99 должен быть около 990
        assert!((p99 - 990.0).abs() < 10.0);
    }
}
