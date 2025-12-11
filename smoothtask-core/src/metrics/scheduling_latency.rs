//! Модуль для измерения scheduling latency через probe-thread (mini-cyclictest).
//!
//! Реализует mini-cyclictest поток, который периодически спит и измеряет задержки пробуждения.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

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
        let mut samples = match self.samples.lock() {
            Ok(guard) => guard,
            Err(e) => {
                warn!(
                    "LatencyCollector mutex is poisoned: {}. Skipping sample.",
                    e
                );
                return;
            }
        };
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
    /// * `None` - если измерений недостаточно (меньше 2) или percentile вне диапазона [0.0, 1.0]
    ///
    /// # Обработка граничных случаев
    ///
    /// * `percentile = 0.0` - возвращает минимальное значение (первый элемент после сортировки)
    /// * `percentile = 1.0` - возвращает максимальное значение (последний элемент после сортировки)
    /// * `percentile < 0.0` или `percentile > 1.0` - возвращает `None` (невалидное значение)
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
    ///
    /// // Граничные случаи
    /// let min = collector.percentile(0.0); // Минимальное значение
    /// let max = collector.percentile(1.0); // Максимальное значение
    /// let invalid = collector.percentile(1.5); // None (невалидное значение)
    /// ```
    pub fn percentile(&self, percentile: f64) -> Option<f64> {
        // Валидация: percentile должен быть в диапазоне [0.0, 1.0]
        if !(0.0..=1.0).contains(&percentile) {
            return None;
        }

        let samples = match self.samples.lock() {
            Ok(guard) => guard,
            Err(e) => {
                warn!(
                    "LatencyCollector mutex is poisoned: {}. Cannot compute percentile.",
                    e
                );
                return None;
            }
        };
        if samples.len() < 2 {
            return None;
        }

        // Копируем измерения в вектор и сортируем
        let mut sorted: Vec<f64> = samples.iter().copied().collect();
        sorted.sort_by(|a, b| {
            a.partial_cmp(b).unwrap_or_else(|| {
                // Если сравнение невозможно (NaN), считаем, что a < b
                if a.is_nan() {
                    std::cmp::Ordering::Less
                } else if b.is_nan() {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            })
        });

        // Вычисляем индекс перцентиля
        // Для percentile = 0.0: index = 0 (минимальное значение)
        // Для percentile = 1.0: index = len - 1 (максимальное значение)
        let index = if percentile == 0.0 {
            0
        } else {
            let computed = (sorted.len() as f64 * percentile).ceil() as usize;
            // Защита от переполнения: если computed == 0, используем 0
            if computed == 0 {
                0
            } else {
                (computed - 1).min(sorted.len() - 1)
            }
        };

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
        match self.samples.lock() {
            Ok(guard) => guard.len(),
            Err(e) => {
                warn!(
                    "LatencyCollector mutex is poisoned: {}. Returning 0 for len().",
                    e
                );
                0
            }
        }
    }

    /// Проверяет, пусто ли окно измерений.
    pub fn is_empty(&self) -> bool {
        match self.samples.lock() {
            Ok(guard) => guard.is_empty(),
            Err(e) => {
                warn!(
                    "LatencyCollector mutex is poisoned: {}. Returning true for is_empty().",
                    e
                );
                true
            }
        }
    }

    /// Очищает все измерения.
    pub fn clear(&self) {
        if let Err(e) = self.samples.lock() {
            warn!(
                "LatencyCollector mutex is poisoned: {}. Cannot clear samples.",
                e
            );
            return;
        }
        if let Ok(mut guard) = self.samples.lock() {
            guard.clear();
        }
    }
}

/// Probe-thread для измерения scheduling latency (mini-cyclictest).
///
/// Создаёт отдельный поток, который периодически спит на заданный интервал
/// и измеряет задержку пробуждения (wakeup_delay = фактическое время - ожидаемое время).
/// Измерения сохраняются в LatencyCollector для последующего вычисления перцентилей.
pub struct LatencyProbe {
    /// Поток probe-thread
    handle: Option<thread::JoinHandle<()>>,
    /// Флаг для остановки probe-thread
    shutdown: Arc<AtomicBool>,
}

impl LatencyProbe {
    /// Создаёт новый LatencyProbe с указанными параметрами.
    ///
    /// # Аргументы
    ///
    /// * `collector` - коллектор для хранения измерений
    /// * `sleep_interval_ms` - интервал сна в миллисекундах (рекомендуется 5-10 мс)
    /// * `max_samples` - максимальное количество измерений в окне (рекомендуется 1000-5000)
    ///
    /// # Пример
    ///
    /// ```
    /// use smoothtask_core::metrics::scheduling_latency::{LatencyCollector, LatencyProbe};
    /// use std::sync::Arc;
    ///
    /// let collector = Arc::new(LatencyCollector::new(1000));
    /// let probe = LatencyProbe::new(Arc::clone(&collector), 5, 1000);
    /// ```
    pub fn new(
        collector: Arc<LatencyCollector>,
        sleep_interval_ms: u64,
        max_samples: usize,
    ) -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = Arc::clone(&shutdown);
        let collector_clone = Arc::clone(&collector);
        let sleep_interval = Duration::from_millis(sleep_interval_ms);

        let handle = thread::Builder::new()
            .name("latency-probe".to_string())
            .spawn(move || {
                Self::probe_thread_loop(collector_clone, shutdown_clone, sleep_interval);
            })
            .unwrap_or_else(|e| {
                error!("Failed to spawn latency probe thread: {}", e);
                panic!("Failed to spawn latency probe thread: {}", e);
            });

        info!(
            "Started latency probe thread (sleep_interval={}ms, max_samples={})",
            sleep_interval_ms, max_samples
        );

        Self {
            handle: Some(handle),
            shutdown,
        }
    }

    /// Основной цикл probe-thread.
    ///
    /// Поток спит на заданный интервал и измеряет задержку пробуждения.
    fn probe_thread_loop(
        collector: Arc<LatencyCollector>,
        shutdown: Arc<AtomicBool>,
        sleep_interval: Duration,
    ) {
        debug!("Latency probe thread started");

        loop {
            // Проверяем shutdown перед каждым измерением
            if shutdown.load(Ordering::Relaxed) {
                debug!("Latency probe thread received shutdown signal");
                break;
            }

            // Запоминаем время перед сном
            let sleep_start = Instant::now();

            // Спим на заданный интервал
            thread::sleep(sleep_interval);

            // Измеряем фактическое время пробуждения
            let wakeup_time = Instant::now();
            let actual_delay = wakeup_time.duration_since(sleep_start);

            // Вычисляем задержку пробуждения (wakeup_delay)
            // Если пробуждение произошло точно в срок, delay = 0
            // Если пробуждение задержалось, delay > 0
            let wakeup_delay = if actual_delay > sleep_interval {
                actual_delay - sleep_interval
            } else {
                // Если пробуждение произошло раньше (маловероятно, но возможно),
                // считаем delay = 0
                Duration::from_secs(0)
            };

            // Преобразуем задержку в миллисекунды
            let latency_ms = wakeup_delay.as_secs_f64() * 1000.0;

            // Добавляем измерение в коллектор
            collector.add_sample(latency_ms);

            // Логируем только значительные задержки (> 1 мс) для отладки
            if latency_ms > 1.0 {
                debug!("Scheduling latency detected: {:.2} ms", latency_ms);
            }
        }

        debug!("Latency probe thread stopped");
    }

    /// Останавливает probe-thread и ждёт его завершения.
    ///
    /// Должна быть вызвана перед уничтожением LatencyProbe для корректного завершения потока.
    pub fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            info!("Stopping latency probe thread");
            self.shutdown.store(true, Ordering::Relaxed);
            if let Err(e) = handle.join() {
                warn!("Failed to join latency probe thread: {:?}", e);
            } else {
                info!("Latency probe thread stopped");
            }
        }
    }
}

impl Drop for LatencyProbe {
    fn drop(&mut self) {
        self.stop();
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

    #[test]
    fn test_latency_collector_percentile_boundary_zero() {
        let collector = LatencyCollector::new(100);
        collector.add_sample(5.0);
        collector.add_sample(10.0);
        collector.add_sample(15.0);

        // percentile = 0.0 должен вернуть минимальное значение
        let p0 = collector.percentile(0.0);
        assert!(p0.is_some());
        assert_eq!(p0.unwrap(), 5.0); // Минимальное значение
    }

    #[test]
    fn test_latency_collector_percentile_boundary_one() {
        let collector = LatencyCollector::new(100);
        collector.add_sample(5.0);
        collector.add_sample(10.0);
        collector.add_sample(15.0);

        // percentile = 1.0 должен вернуть максимальное значение
        let p100 = collector.percentile(1.0);
        assert!(p100.is_some());
        assert_eq!(p100.unwrap(), 15.0); // Максимальное значение
    }

    #[test]
    fn test_latency_collector_percentile_negative() {
        let collector = LatencyCollector::new(100);
        collector.add_sample(5.0);
        collector.add_sample(10.0);
        collector.add_sample(15.0);

        // Отрицательный percentile должен вернуть None
        assert_eq!(collector.percentile(-0.1), None);
        assert_eq!(collector.percentile(-1.0), None);
    }

    #[test]
    fn test_latency_collector_percentile_greater_than_one() {
        let collector = LatencyCollector::new(100);
        collector.add_sample(5.0);
        collector.add_sample(10.0);
        collector.add_sample(15.0);

        // percentile > 1.0 должен вернуть None
        assert_eq!(collector.percentile(1.1), None);
        assert_eq!(collector.percentile(2.0), None);
    }

    #[test]
    fn test_latency_collector_percentile_nan() {
        let collector = LatencyCollector::new(100);
        collector.add_sample(5.0);
        collector.add_sample(10.0);
        collector.add_sample(15.0);

        // NaN percentile должен вернуть None (так как NaN не находится в диапазоне [0.0, 1.0])
        let nan_percentile = f64::NAN;
        assert_eq!(collector.percentile(nan_percentile), None);
    }

    #[test]
    fn test_latency_collector_percentile_infinity() {
        let collector = LatencyCollector::new(100);
        collector.add_sample(5.0);
        collector.add_sample(10.0);
        collector.add_sample(15.0);

        // Infinity percentile должен вернуть None
        assert_eq!(collector.percentile(f64::INFINITY), None);
        assert_eq!(collector.percentile(f64::NEG_INFINITY), None);
    }

    #[test]
    fn test_latency_collector_percentile_edge_cases_with_many_samples() {
        let collector = LatencyCollector::new(100);
        // Добавляем 100 значений от 1.0 до 100.0
        for i in 1..=100 {
            collector.add_sample(i as f64);
        }

        // P0 должен быть минимальным (1.0)
        let p0 = collector.percentile(0.0).unwrap();
        assert_eq!(p0, 1.0);

        // P100 должен быть максимальным (100.0)
        let p100 = collector.percentile(1.0).unwrap();
        assert_eq!(p100, 100.0);

        // P50 должен быть около 50
        let p50 = collector.percentile(0.5).unwrap();
        assert!((p50 - 50.0).abs() < 2.0);
    }

    #[test]
    fn test_latency_probe_starts_and_stops() {
        use std::time::Duration;
        let collector = Arc::new(LatencyCollector::new(1000));
        let mut probe = LatencyProbe::new(Arc::clone(&collector), 5, 1000);

        // Даём probe-thread немного времени для работы
        thread::sleep(Duration::from_millis(50));

        // Проверяем, что измерения собираются
        let initial_len = collector.len();
        assert!(
            initial_len > 0,
            "Probe thread should collect some measurements"
        );

        // Останавливаем probe-thread
        probe.stop();

        // Даём время для завершения потока
        thread::sleep(Duration::from_millis(10));

        // Проверяем, что поток действительно остановился (количество измерений не увеличивается)
        let final_len = collector.len();
        thread::sleep(Duration::from_millis(20));
        let len_after_stop = collector.len();
        assert_eq!(
            len_after_stop, final_len,
            "Probe thread should stop collecting measurements after stop()"
        );
    }

    #[test]
    fn test_latency_probe_collects_measurements() {
        use std::time::Duration;
        let collector = Arc::new(LatencyCollector::new(1000));
        let mut probe = LatencyProbe::new(Arc::clone(&collector), 5, 1000);

        // Даём probe-thread время для сбора нескольких измерений
        thread::sleep(Duration::from_millis(100));

        // Проверяем, что измерения собраны
        let len = collector.len();
        assert!(
            len >= 10,
            "Should collect at least 10 measurements in 100ms (5ms interval)"
        );

        // Проверяем, что можно вычислить перцентили
        let p95 = collector.p95();
        let p99 = collector.p99();
        assert!(p95.is_some(), "Should be able to compute P95");
        assert!(p99.is_some(), "Should be able to compute P99");

        probe.stop();
    }

    #[test]
    fn test_latency_probe_drop_stops_thread() {
        use std::time::Duration;
        let collector = Arc::new(LatencyCollector::new(1000));
        let probe = LatencyProbe::new(Arc::clone(&collector), 5, 1000);

        // Даём probe-thread время для работы
        thread::sleep(Duration::from_millis(50));

        let len_before_drop = collector.len();
        assert!(len_before_drop > 0);

        // Drop должен остановить поток
        drop(probe);

        // Даём время для завершения потока
        thread::sleep(Duration::from_millis(20));

        // Проверяем, что поток остановился
        let len_after_drop = collector.len();
        thread::sleep(Duration::from_millis(20));
        let len_after_wait = collector.len();
        assert_eq!(
            len_after_wait, len_after_drop,
            "Probe thread should stop after drop"
        );
    }
}
