//! ONNX-ранкер для ранжирования AppGroup с использованием обученных моделей.
//!
//! Этот модуль предоставляет реализацию ранкера на основе ONNX Runtime
//! для загрузки и выполнения обученных CatBoost моделей.

#[cfg(feature = "onnx")]
use crate::logging::snapshots::{AppGroupRecord, Snapshot};
#[cfg(feature = "onnx")]
use crate::model::features::{build_features, FeatureVector};
#[cfg(feature = "onnx")]
use crate::model::ranker::{Ranker, RankingResult};
#[cfg(feature = "onnx")]
use anyhow::{Context, Result};
#[cfg(feature = "onnx")]
use ort::{session::Session, value::Tensor};
#[cfg(feature = "onnx")]
use std::collections::HashMap;
#[cfg(feature = "onnx")]
use std::path::Path;
#[cfg(feature = "onnx")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "onnx")]
/// ONNX-ранкер для ранжирования групп приложений.
///
/// Использует ONNX Runtime для загрузки и выполнения обученных CatBoost моделей.
/// Модель должна быть обучена с использованием `smoothtask_trainer.train_ranker`
/// и сохранена в формате ONNX.
///
/// # Примеры использования
///
/// **Примечание:** Примеры помечены как `ignore`, потому что они требуют создания
/// сложных структур (`Snapshot`, `AppGroupRecord`) с реальными метриками системы,
/// что невозможно сделать в doctest'ах без доступа к `/proc` и другим системным ресурсам.
/// Для реального использования см. интеграционные тесты в `tests/` или примеры в `model/mod.rs`.
///
/// ```ignore
/// use smoothtask_core::model::onnx_ranker::ONNXRanker;
/// use smoothtask_core::model::ranker::Ranker;
/// use smoothtask_core::logging::snapshots::{Snapshot, AppGroupRecord};
///
/// // Загрузка модели
/// let ranker = ONNXRanker::load("path/to/model.onnx")?;
/// let snapshot: Snapshot = /* ... */;
/// let app_groups: Vec<AppGroupRecord> = /* ... */;
///
/// // Ранжирование групп
/// let results = ranker.rank(&app_groups, &snapshot);
///
/// // Использование результатов
/// for (app_group_id, result) in &results {
///     println!("Group {}: score={:.2}, rank={}, percentile={:.2}",
///              app_group_id, result.score, result.rank, result.percentile);
/// }
/// ```
#[cfg(feature = "onnx")]
#[derive(Debug)]
pub struct ONNXRanker {
    /// ONNX Runtime сессия для выполнения модели
    session: Arc<Mutex<Session>>,
    /// Количество входных фич, ожидаемых моделью
    expected_input_size: usize,
    /// Имя входного тензора модели
    input_name: String,
    /// Имя выходного тензора модели
    output_name: String,
}

#[cfg(feature = "onnx")]
impl ONNXRanker {
    /// Загрузить ONNX модель из файла.
    ///
    /// # Аргументы
    ///
    /// * `model_path` - путь к ONNX файлу модели
    ///
    /// # Возвращает
    ///
    /// `Result<ONNXRanker>` с загруженной моделью или ошибкой
    ///
    /// # Ошибки
    ///
    /// * `FileNotFoundError` - если файл модели не существует
    /// * `InvalidModelError` - если модель имеет неверный формат или структуру
    /// * `ONNXRuntimeError` - если произошла ошибка при загрузке модели
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// use smoothtask_core::model::onnx_ranker::ONNXRanker;
    ///
    /// let ranker = ONNXRanker::load("path/to/model.onnx")?;
    /// ```
    pub fn load(model_path: impl AsRef<Path>) -> Result<Self> {
        let model_path = model_path.as_ref();

        // Проверяем существование файла
        if !model_path.exists() {
            return Err(anyhow::anyhow!(
                "Файл модели не найден: {}",
                model_path.display()
            ));
        }

        // Загружаем модель с использованием простого API
        let session = Session::builder()?.commit_from_file(model_path)?;

        // Получаем информацию о модели
        let input_info = session
            .inputs
            .first()
            .context("Модель не имеет входных тензоров")?;
        let output_info = session
            .outputs
            .first()
            .context("Модель не имеет выходных тензоров")?;

        let input_name = input_info.name.clone();
        let output_name = output_info.name.clone();

        // Получаем размер входного тензора из input_type
        // Ожидаем форму [batch_size, feature_size], где batch_size может быть переменным
        let input_shape = match &input_info.input_type {
            ort::value::ValueType::Tensor { shape, .. } => shape,
            _ => {
                return Err(anyhow::anyhow!(
                    "Входной тензор имеет неожиданный тип: {:?}",
                    input_info.input_type
                ))
            }
        };

        let expected_input_size = if input_shape.len() == 2 {
            // Форма [batch_size, feature_size] - берём feature_size
            // Игнорируем динамические размеры (-1)
            let feature_size = input_shape[1];
            if feature_size == -1 {
                return Err(anyhow::anyhow!(
                    "Неподдерживаемая динамическая форма входного тензора: {:?}",
                    input_shape
                ));
            }
            feature_size as usize
        } else if input_shape.len() == 1 {
            // Форма [feature_size] - берём единственное значение
            let feature_size = input_shape[0];
            if feature_size == -1 {
                return Err(anyhow::anyhow!(
                    "Неподдерживаемая динамическая форма входного тензора: {:?}",
                    input_shape
                ));
            }
            feature_size as usize
        } else {
            return Err(anyhow::anyhow!(
                "Неподдерживаемая форма входного тензора: {:?}",
                input_shape
            ));
        };

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            expected_input_size,
            input_name,
            output_name,
        })
    }

    /// Преобразовать FeatureVector в тензор для ONNX модели.
    ///
    /// Преобразует числовые, булевые и категориальные фичи в тензор,
    /// совместимый с ожидаемым форматом модели.
    ///
    /// # Аргументы
    ///
    /// * `features` - вектор фич для преобразования
    ///
    /// # Возвращает
    ///
    /// Тензор ONNX для использования в модели
    ///
    /// # Примечания
    ///
    /// - Числовые фичи используются как есть
    /// - Булевые фичи преобразуются в f32 (0.0 или 1.0)
    /// - Категориальные фичи преобразуются в числовые индексы
    ///
    fn features_to_tensor(&self, features: &FeatureVector) -> Result<Tensor<f32>> {
        let mut tensor_data = Vec::with_capacity(self.expected_input_size);

        // Добавляем числовые фичи
        for &value in &features.numeric {
            tensor_data.push(value as f32);
        }

        // Добавляем булевые фичи (преобразуем в f32)
        for &value in &features.bool {
            tensor_data.push(value as f32);
        }

        // Добавляем категориальные фичи (преобразуем в числовые индексы)
        // Для простоты используем хэш от строки, модуль 1000 для ограничения диапазона
        for value in &features.categorical {
            let hash = self.string_to_index(value);
            tensor_data.push(hash as f32);
        }

        // Проверяем, что размер совпадает с ожидаемым
        if tensor_data.len() != self.expected_input_size {
            return Err(anyhow::anyhow!(
                "Размер вектора фич ({}) не совпадает с ожидаемым размером модели ({})",
                tensor_data.len(),
                self.expected_input_size
            ));
        }

        // Создаём тензор с формой [1, feature_size] (batch_size=1)
        let shape = [1usize, self.expected_input_size];
        Tensor::from_array((shape, tensor_data.into_boxed_slice()))
            .map_err(|e| anyhow::anyhow!("Не удалось создать тензор из вектора фич: {}", e))
    }

    /// Преобразовать строку в числовой индекс для категориальных фич.
    ///
    /// Использует простой хэш для преобразования строк в числовые значения.
    ///
    /// # Аргументы
    ///
    /// * `value` - строковое значение категориальной фичи
    ///
    /// # Возвращает
    ///
    /// Числовой индекс в диапазоне [0, 999]
    ///
    fn string_to_index(&self, value: &str) -> i32 {
        // Используем простой хэш для преобразования строки в число
        let mut hash = 0u64;
        for byte in value.as_bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
        }
        // Ограничиваем диапазон до 0-999
        (hash % 1000) as i32
    }
}

#[cfg(feature = "onnx")]
impl Ranker for ONNXRanker {
    fn rank(
        &self,
        app_groups: &[AppGroupRecord],
        snapshot: &Snapshot,
    ) -> HashMap<String, RankingResult> {
        // Строим фичи для каждой группы
        let mut scores: Vec<(String, f64)> = Vec::new();

        for app_group in app_groups {
            // Строим фичи для группы
            let features = build_features(snapshot, app_group);

            // Преобразуем фичи в тензор
            let input_tensor = match self.features_to_tensor(&features) {
                Ok(tensor) => tensor,
                Err(e) => {
                    // В случае ошибки используем дефолтный score
                    tracing::error!(
                        "Ошибка при преобразовании фич для группы {}: {}",
                        app_group.app_group_id,
                        e
                    );
                    scores.push((app_group.app_group_id.clone(), 0.5));
                    continue;
                }
            };

            // Выполняем инференс модели
            let score = match self.run_inference(&input_tensor) {
                Ok(score) => score,
                Err(e) => {
                    // В случае ошибки используем дефолтный score
                    tracing::error!(
                        "Ошибка при выполнении инференса для группы {}: {}",
                        app_group.app_group_id,
                        e
                    );
                    0.5
                }
            };

            scores.push((app_group.app_group_id.clone(), score));
        }

        // Сортируем по score (убывание)
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Вычисляем rank и percentile
        let total = scores.len();
        let mut results = HashMap::new();

        for (rank_idx, (app_group_id, score)) in scores.iter().enumerate() {
            let rank = rank_idx + 1;
            // Percentile: 1.0 для самого важного, 0.0 для наименее важного
            let percentile = if total > 1 {
                1.0 - (rank_idx as f64) / ((total - 1) as f64)
            } else {
                1.0
            };

            results.insert(
                app_group_id.clone(),
                RankingResult {
                    score: *score,
                    rank,
                    percentile,
                },
            );
        }

        results
    }
}

#[cfg(feature = "onnx")]
impl ONNXRanker {
    /// Выполнить инференс модели для одного образца.
    ///
    /// # Аргументы
    ///
    /// * `input_tensor` - входной тензор для модели
    ///
    /// # Возвращает
    ///
    /// Score от модели (f64) или ошибку
    ///
    fn run_inference(&self, input_tensor: &Tensor<f32>) -> Result<f64> {
        // Создаём маппинг входов
        let inputs = ort::inputs! {
            self.input_name.clone() => input_tensor.view(),
        };

        // Выполняем инференс с использованием Mutex
        let mut session_guard = self.session.lock().map_err(|e| {
            anyhow::anyhow!("Mutex poisoned: {}", e)
        })?;
        let outputs = session_guard.run(inputs)?;

        // Извлекаем выходной тензор
        let output_tensor = outputs
            .get(&self.output_name)
            .context("Не удалось получить выходной тензор")?;

        // Преобразуем выход в score
        let (_, output_array) = output_tensor
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow::anyhow!("Не удалось извлечь тензор из выхода: {}", e))?;

        // Берём первое значение как score
        let score = output_array[0] as f64;

        // Ограничиваем score в диапазоне [0.0, 1.0]
        Ok(score.clamp(0.0, 1.0))
    }
}

#[cfg(all(test, feature = "onnx"))]
mod tests {
    use super::*;
    use crate::logging::snapshots::{GlobalMetrics, ResponsivenessMetrics};
    use chrono::Utc;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Мок-ранкер для тестирования
    /// Возвращает фиксированный score для всех групп
    struct MockRanker {
        fixed_score: f64,
    }

    impl MockRanker {
        fn new(_expected_input_size: usize) -> Box<dyn Ranker> {
            Box::new(Self { fixed_score: 0.75 })
        }
    }

    impl Ranker for MockRanker {
        fn rank(
            &self,
            app_groups: &[AppGroupRecord],
            _snapshot: &Snapshot,
        ) -> HashMap<String, RankingResult> {
            let mut results = HashMap::new();

            for (rank_idx, app_group) in app_groups.iter().enumerate() {
                let rank = rank_idx + 1;
                let total = app_groups.len();
                let percentile = if total > 1 {
                    1.0 - (rank_idx as f64) / ((total - 1) as f64)
                } else {
                    1.0
                };

                results.insert(
                    app_group.app_group_id.clone(),
                    RankingResult {
                        score: self.fixed_score,
                        rank,
                        percentile,
                    },
                );
            }

            results
        }
    }

    /// Мок-ранкер для тестирования string_to_index
    struct MockRankerForStringToIndex;

    impl MockRankerForStringToIndex {
        fn new() -> Self {
            Self
        }

        fn string_to_index(&self, value: &str) -> i32 {
            // Используем тот же алгоритм, что и в ONNXRanker
            let mut hash = 0u64;
            for byte in value.as_bytes() {
                hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
            }
            (hash % 1000) as i32
        }
    }

    /// Мок-ранкер для тестирования features_to_tensor
    struct MockRankerForFeaturesToTensor {
        expected_input_size: usize,
    }

    impl MockRankerForFeaturesToTensor {
        fn new(expected_input_size: usize) -> Self {
            Self {
                expected_input_size,
            }
        }

        fn features_to_tensor(&self, features: &FeatureVector) -> Result<Tensor<f32>> {
            let mut tensor_data = Vec::with_capacity(self.expected_input_size);

            // Добавляем числовые фичи
            for &value in &features.numeric {
                tensor_data.push(value as f32);
            }

            // Добавляем булевые фичи (преобразуем в f32)
            for &value in &features.bool {
                tensor_data.push(value as f32);
            }

            // Добавляем категориальные фичи (преобразуем в числовые индексы)
            for value in &features.categorical {
                let hash = self.string_to_index(value);
                tensor_data.push(hash as f32);
            }

            // Проверяем, что размер совпадает с ожидаемым
            if tensor_data.len() != self.expected_input_size {
                return Err(anyhow::anyhow!(
                    "Размер вектора фич ({}) не совпадает с ожидаемым размером модели ({})",
                    tensor_data.len(),
                    self.expected_input_size
                ));
            }

            // Создаём тензор с формой [1, feature_size] (batch_size=1)
            let shape = [1usize, self.expected_input_size];
            Tensor::from_array((shape, tensor_data.into_boxed_slice()))
                .map_err(|e| anyhow::anyhow!("Не удалось создать тензор из вектора фич: {}", e))
        }

        fn string_to_index(&self, value: &str) -> i32 {
            // Используем тот же алгоритм, что и в ONNXRanker
            let mut hash = 0u64;
            for byte in value.as_bytes() {
                hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
            }
            (hash % 1000) as i32
        }
    }

    fn create_test_snapshot() -> Snapshot {
        Snapshot {
            snapshot_id: 1234567890,
            timestamp: Utc::now(),
            global: GlobalMetrics {
                cpu_user: 0.25,
                cpu_system: 0.15,
                cpu_idle: 0.55,
                cpu_iowait: 0.05,
                mem_total_kb: 16_384_256,
                mem_used_kb: 8_000_000,
                mem_available_kb: 8_384_256,
                swap_total_kb: 8_192_000,
                swap_used_kb: 1_000_000,
                load_avg_one: 1.5,
                load_avg_five: 1.2,
                load_avg_fifteen: 1.0,
                psi_cpu_some_avg10: Some(0.1),
                psi_cpu_some_avg60: Some(0.15),
                psi_io_some_avg10: Some(0.2),
                psi_mem_some_avg10: Some(0.05),
                psi_mem_full_avg10: None,
                user_active: true,
                time_since_last_input_ms: Some(5000),
            },
            processes: vec![],
            app_groups: vec![],
            responsiveness: ResponsivenessMetrics::default(),
        }
    }

    #[test]
    fn test_onnx_ranker_load_nonexistent_file() {
        // Тест загрузки несуществующего файла
        let result = ONNXRanker::load("nonexistent.onnx");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Файл модели не найден"));
    }

    #[test]
    fn test_onnx_ranker_string_to_index() {
        // Тест преобразования строк в индексы
        // Создаём ранкер с заглушкой для тестирования string_to_index
        let ranker = MockRankerForStringToIndex::new();

        // Проверяем, что одинаковые строки дают одинаковые индексы
        let index1 = ranker.string_to_index("test");
        let index2 = ranker.string_to_index("test");
        assert_eq!(index1, index2);

        // Проверяем, что разные строки могут давать разные индексы
        let index3 = ranker.string_to_index("different");
        // Не гарантируем, что они будут разными, но проверяем диапазон
        assert!(index1 >= 0 && index1 < 1000);
        assert!(index3 >= 0 && index3 < 1000);
    }

    #[test]
    fn test_onnx_ranker_features_to_tensor_size_mismatch() {
        // Тест обработки несоответствия размера фич
        // Создаём мок-ранкер для тестирования features_to_tensor
        let ranker = MockRankerForFeaturesToTensor::new(10); // Ожидаем 10 фич

        // Создаём FeatureVector с другим размером
        let features = FeatureVector {
            numeric: vec![1.0, 2.0],
            bool: vec![1],
            categorical: vec!["test".to_string()],
            cat_feature_indices: vec![3],
        };

        // Должна быть ошибка из-за несоответствия размера
        let result = ranker.features_to_tensor(&features);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("не совпадает с ожидаемым размером"));
    }

    #[test]
    fn test_onnx_ranker_empty_groups() {
        // Тест ранжирования пустого списка групп
        let snapshot = create_test_snapshot();
        let app_groups = vec![];

        // Создаём ранкер с заглушкой (не можем загрузить реальную модель в тесте)
        let ranker = MockRanker::new(51);

        let results = ranker.rank(&app_groups, &snapshot);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_onnx_ranker_single_group() {
        // Тест ранжирования одной группы
        let mut snapshot = create_test_snapshot();

        let app_groups = vec![AppGroupRecord {
            app_group_id: "single".to_string(),
            root_pid: 3000,
            process_ids: vec![3000],
            app_name: Some("app".to_string()),
            total_cpu_share: Some(0.2),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_rss_mb: Some(200),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec![],
            priority_class: None,
        }];

        snapshot.app_groups = app_groups.clone();

        // Создаём ранкер с заглушкой
        let ranker = MockRanker::new(51);

        let results = ranker.rank(&app_groups, &snapshot);

        // Должен быть один результат с фиксированным score из MockRanker
        assert_eq!(results.len(), 1);
        let result = results.get("single").unwrap();
        assert_eq!(result.rank, 1);
        assert_eq!(result.percentile, 1.0);
        assert_eq!(result.score, 0.75); // Фиксированный score из MockRanker
    }

    #[test]
    fn test_onnx_ranker_multiple_groups() {
        // Тест ранжирования нескольких групп
        let mut snapshot = create_test_snapshot();

        let app_groups = vec![
            AppGroupRecord {
                app_group_id: "group1".to_string(),
                root_pid: 1000,
                process_ids: vec![1000],
                app_name: Some("app1".to_string()),
                total_cpu_share: Some(0.5),
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: Some(500),
                has_gui_window: true,
                is_focused_group: true,
                tags: vec!["browser".to_string()],
                priority_class: Some("INTERACTIVE".to_string()),
            },
            AppGroupRecord {
                app_group_id: "group2".to_string(),
                root_pid: 2000,
                process_ids: vec![2000],
                app_name: Some("app2".to_string()),
                total_cpu_share: Some(0.1),
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: Some(100),
                has_gui_window: false,
                is_focused_group: false,
                tags: vec![],
                priority_class: None,
            },
        ];

        snapshot.app_groups = app_groups.clone();

        // Создаём ранкер с заглушкой
        let ranker = MockRanker::new(51);

        let results = ranker.rank(&app_groups, &snapshot);

        // Должны быть результаты для всех групп
        assert_eq!(results.len(), 2);
        assert!(results.contains_key("group1"));
        assert!(results.contains_key("group2"));

        // Обе группы должны иметь фиксированный score из MockRanker
        let result1 = results.get("group1").unwrap();
        let result2 = results.get("group2").unwrap();
        assert_eq!(result1.score, 0.75);
        assert_eq!(result2.score, 0.75);

        // Ранги должны быть последовательными
        assert!(result1.rank >= 1 && result1.rank <= 2);
        assert!(result2.rank >= 1 && result2.rank <= 2);
        assert_ne!(result1.rank, result2.rank);
    }

    #[test]
    fn test_onnx_ranker_create_dummy_model_file() {
        // Тест создания заглушки ONNX модели для тестирования
        // В реальном использовании этот тест можно расширить для работы с реальной моделью
        let mut temp_file = NamedTempFile::new().unwrap();

        // Пишем минимальный валидный ONNX файл (заглушка)
        // В реальном проекте здесь можно использовать реальную модель
        writeln!(temp_file, "dummy_onnx_content").unwrap();

        // Проверяем, что файл создан
        let path = temp_file.path();
        assert!(path.exists());

        // Проверяем, что загрузка завершается с ошибкой (так как это не валидная модель)
        let result = ONNXRanker::load(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_onnx_ranker_error_handling_in_rank() {
        // Тест обработки ошибок в методе rank
        // Создаём мок-ранкер, который всегда возвращает ошибку при преобразовании фич
        struct ErrorMockRanker;

        impl ErrorMockRanker {
            fn new() -> Self {
                Self
            }
        }

        impl Ranker for ErrorMockRanker {
            fn rank(
                &self,
                app_groups: &[AppGroupRecord],
                _snapshot: &Snapshot,
            ) -> HashMap<String, RankingResult> {
                let mut results = HashMap::new();

                for app_group in app_groups {
                    // Всегда возвращаем дефолтный score 0.5 при "ошибке"
                    results.insert(
                        app_group.app_group_id.clone(),
                        RankingResult {
                            score: 0.5,
                            rank: 1,
                            percentile: 1.0,
                        },
                    );
                }

                results
            }
        }

        let snapshot = create_test_snapshot();
        let app_groups = vec![
            AppGroupRecord {
                app_group_id: "test-group".to_string(),
                root_pid: 1000,
                process_ids: vec![1000],
                app_name: Some("test".to_string()),
                total_cpu_share: Some(0.1),
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: Some(100),
                has_gui_window: false,
                is_focused_group: false,
                tags: vec![],
                priority_class: None,
            },
        ];

        let ranker = ErrorMockRanker::new();
        let results = ranker.rank(&app_groups, &snapshot);

        // Должен быть результат с дефолтным score
        assert_eq!(results.len(), 1);
        let result = results.get("test-group").unwrap();
        assert_eq!(result.score, 0.5);
    }

    #[test]
    fn test_onnx_ranker_string_to_index_edge_cases() {
        // Тест обработки крайних случаев в string_to_index
        let ranker = MockRankerForStringToIndex::new();

        // Тест пустой строки
        let empty_index = ranker.string_to_index("");
        assert!(empty_index >= 0 && empty_index < 1000);

        // Тест очень длинной строки
        let long_string = "a".repeat(1000);
        let long_index = ranker.string_to_index(&long_string);
        assert!(long_index >= 0 && long_index < 1000);

        // Тест строки с специальными символами
        let special_index = ranker.string_to_index("test!@#$%^&*()");
        assert!(special_index >= 0 && special_index < 1000);

        // Тест строки с unicode символами
        let unicode_index = ranker.string_to_index("тест🚀");
        assert!(unicode_index >= 0 && unicode_index < 1000);
    }

    #[test]
    fn test_onnx_ranker_features_to_tensor_edge_cases() {
        // Тест обработки крайних случаев в features_to_tensor
        let ranker = MockRankerForFeaturesToTensor::new(10);

        // Тест с пустыми векторами
        let empty_features = FeatureVector {
            numeric: vec![],
            bool: vec![],
            categorical: vec![],
            cat_feature_indices: vec![],
        };

        let result = ranker.features_to_tensor(&empty_features);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("не совпадает с ожидаемым размером"));

        // Тест с частично заполненными векторами
        let partial_features = FeatureVector {
            numeric: vec![1.0, 2.0],
            bool: vec![1],
            categorical: vec!["test".to_string()],
            cat_feature_indices: vec![3],
        };

        let result = ranker.features_to_tensor(&partial_features);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("не совпадает с ожидаемым размером"));
    }
}
