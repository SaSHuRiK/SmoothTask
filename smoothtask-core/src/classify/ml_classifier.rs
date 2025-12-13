//! ML-классификатор для классификации процессов.
//!
//! Этот модуль предоставляет интерфейс для классификации процессов
//! с использованием ML-моделей. Поддерживает интеграцию с CatBoost
//! и ONNX Runtime для загрузки и использования предварительно обученных моделей.

use crate::config::config_struct::{MLClassifierConfig, ModelType};
use crate::logging::snapshots::ProcessRecord;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info, warn};

#[cfg(feature = "catboost")]
use catboost::CatBoostClassifier;

#[cfg(feature = "onnx")]
use ort::Session;

/// Метрики производительности ML-модели.
#[derive(Debug, Clone, Default)]
pub struct MLPerformanceMetrics {
    /// Общее количество классификаций.
    pub total_classifications: u64,
    /// Количество успешных классификаций.
    pub successful_classifications: u64,
    /// Количество ошибок классификации.
    pub classification_errors: u64,
    /// Суммарное время классификации в микросекундах.
    pub total_classification_time_us: u128,
    /// Минимальное время классификации в микросекундах.
    pub min_classification_time_us: Option<u128>,
    /// Максимальное время классификации в микросекундах.
    pub max_classification_time_us: Option<u128>,
    /// Суммарная уверенность всех классификаций.
    pub total_confidence: f64,
    /// Количество классификаций с высокой уверенностью (> 0.8).
    pub high_confidence_classifications: u64,
    /// Количество классификаций со средней уверенностью (0.5 - 0.8).
    pub medium_confidence_classifications: u64,
    /// Количество классификаций с низкой уверенностью (< 0.5).
    pub low_confidence_classifications: u64,
}

impl MLPerformanceMetrics {
    /// Создать новые метрики производительности.
    pub fn new() -> Self {
        Self::default()
    }

    /// Зарегистрировать успешную классификацию.
    pub fn record_successful_classification(&mut self, duration: u128, confidence: f64) {
        self.total_classifications += 1;
        self.successful_classifications += 1;
        self.total_classification_time_us += duration;
        self.total_confidence += confidence;

        // Обновить минимальное и максимальное время
        if let Some(min_time) = self.min_classification_time_us {
            if duration < min_time {
                self.min_classification_time_us = Some(duration);
            }
        } else {
            self.min_classification_time_us = Some(duration);
        }

        if let Some(max_time) = self.max_classification_time_us {
            if duration > max_time {
                self.max_classification_time_us = Some(duration);
            }
        } else {
            self.max_classification_time_us = Some(duration);
        }

        // Категоризировать по уверенности
        if confidence > 0.8 {
            self.high_confidence_classifications += 1;
        } else if confidence > 0.5 {
            self.medium_confidence_classifications += 1;
        } else {
            self.low_confidence_classifications += 1;
        }
    }

    /// Зарегистрировать ошибку классификации.
    pub fn record_classification_error(&mut self) {
        self.total_classifications += 1;
        self.classification_errors += 1;
    }

    /// Получить среднее время классификации в микросекундах.
    pub fn average_classification_time_us(&self) -> Option<f64> {
        if self.successful_classifications > 0 {
            Some(self.total_classification_time_us as f64 / self.successful_classifications as f64)
        } else {
            None
        }
    }

    /// Получить среднюю уверенность.
    pub fn average_confidence(&self) -> Option<f64> {
        if self.successful_classifications > 0 {
            Some(self.total_confidence / self.successful_classifications as f64)
        } else {
            None
        }
    }

    /// Получить процент успешных классификаций.
    pub fn success_rate(&self) -> Option<f64> {
        if self.total_classifications > 0 {
            Some(self.successful_classifications as f64 / self.total_classifications as f64)
        } else {
            None
        }
    }

    /// Сбросить метрики.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Логировать сводку метрик.
    pub fn log_summary(&self) {
        info!("ML Performance Metrics Summary:");
        info!("  Total classifications: {}", self.total_classifications);
        info!("  Successful classifications: {}", self.successful_classifications);
        info!("  Classification errors: {}", self.classification_errors);
        
        if let Some(success_rate) = self.success_rate() {
            info!("  Success rate: {:.2}%", success_rate * 100.0);
        }
        
        if let Some(avg_time) = self.average_classification_time_us() {
            info!("  Average classification time: {:.2} μs", avg_time);
        }
        
        if let Some(min_time) = self.min_classification_time_us {
            info!("  Min classification time: {} μs", min_time);
        }
        
        if let Some(max_time) = self.max_classification_time_us {
            info!("  Max classification time: {} μs", max_time);
        }
        
        if let Some(avg_confidence) = self.average_confidence() {
            info!("  Average confidence: {:.3}", avg_confidence);
        }
        
        info!("  High confidence (>0.8): {}", self.high_confidence_classifications);
        info!("  Medium confidence (0.5-0.8): {}", self.medium_confidence_classifications);
        info!("  Low confidence (<0.5): {}", self.low_confidence_classifications);
    }
}

/// Результат классификации от ML-модели.
#[derive(Debug, Clone)]
pub struct MLClassificationResult {
    /// Тип процесса, предсказанный ML-моделью.
    pub process_type: Option<String>,
    /// Теги, предсказанные ML-моделью.
    pub tags: Vec<String>,
    /// Уверенность модели в предсказании (0.0 - 1.0).
    pub confidence: f64,
}

/// Трейт для ML-классификатора процессов.
///
/// Трейт требует `Send + Sync`, так как классификатор используется в async контексте
/// и может быть перемещён между потоками.
pub trait MLClassifier: Send + Sync + std::fmt::Debug {
    /// Классифицировать процесс с использованием ML-модели.
    ///
    /// # Аргументы
    ///
    /// * `process` - процесс для классификации
    ///
    /// # Возвращает
    ///
    /// Результат классификации с предсказанным типом, тегами и уверенностью.
    fn classify(&mut self, process: &ProcessRecord) -> MLClassificationResult;

    /// Получить текущие метрики производительности.
    ///
    /// # Возвращает
    ///
    /// Клон текущих метрик производительности.
    fn get_performance_metrics(&self) -> MLPerformanceMetrics;

    /// Сбросить метрики производительности.
    fn reset_performance_metrics(&mut self);

    /// Логировать сводку метрик производительности.
    fn log_performance_summary(&self) {
        self.get_performance_metrics().log_summary();
    }
}

/// Создать ML-классификатор на основе конфигурации.
///
/// # Аргументы
///
/// * `config` - конфигурация ML-классификатора
///
/// # Возвращает
///
/// Результат с ML-классификатором или ошибкой.
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::classify::ml_classifier::{create_ml_classifier, MLClassifier};
/// use smoothtask_core::config::config_struct::MLClassifierConfig;
///
/// let config = MLClassifierConfig {
///     enabled: true,
///     model_path: "models/process_classifier.json".to_string(),
///     confidence_threshold: 0.7,
///     model_type: ModelType::Catboost,
/// };
///
/// let classifier = create_ml_classifier(config);
/// match classifier {
///     Ok(classifier) => {
///         // Использовать классификатор
///     }
///     Err(e) => {
///         eprintln!("Не удалось создать ML-классификатор: {}", e);
///     }
/// }
/// ```
pub fn create_ml_classifier(config: MLClassifierConfig) -> Result<Box<dyn MLClassifier>> {
    if config.enabled {
        CatBoostMLClassifier::new(config).map(|c| Box::new(c) as Box<dyn MLClassifier>)
    } else {
        info!("ML-классификатор отключен в конфигурации, используется заглушка");
        Ok(Box::new(StubMLClassifier::new()) as Box<dyn MLClassifier>)
    }
}

/// Заглушка ML-классификатора для тестирования.
///
/// Использует простые эвристики для классификации процессов:
/// - Процессы с GUI получают тип "gui" и соответствующие теги
/// - Процессы с высоким CPU получают тип "cpu_intensive"
/// - Процессы с высоким IO получают тип "io_intensive"
#[derive(Debug)]
pub struct StubMLClassifier {
    /// Метрики производительности.
    performance_metrics: MLPerformanceMetrics,
}

impl StubMLClassifier {
    /// Создать новый заглушку ML-классификатора.
    pub fn new() -> Self {
        Self {
            performance_metrics: MLPerformanceMetrics::new(),
        }
    }
}

impl Default for StubMLClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl MLClassifier for StubMLClassifier {
    fn classify(&mut self, process: &ProcessRecord) -> MLClassificationResult {
        let start_time = Instant::now();
        let mut tags = HashSet::new();
        let mut process_type = None;
        let mut confidence: f64 = 0.5;

        // Простая эвристика: GUI процессы
        if process.has_gui_window {
            tags.insert("gui".to_string());
            tags.insert("interactive".to_string());
            // Выбираем тип с наивысшей уверенностью
            if 0.8 > confidence {
                process_type = Some("gui".to_string());
                confidence = 0.8;
            }
        }

        // Простая эвристика: высокий CPU usage
        if let Some(cpu_share) = process.cpu_share_10s {
            if cpu_share > 0.3 {
                tags.insert("high_cpu".to_string());
                // Выбираем тип с наивысшей уверенностью
                if 0.7 > confidence {
                    process_type = Some("cpu_intensive".to_string());
                    confidence = 0.7;
                }
            }
        }

        // Простая эвристика: высокий IO
        if let Some(io_read) = process.io_read_bytes {
            if io_read > 1024 * 1024 {
                // > 1MB
                tags.insert("high_io".to_string());
                // Выбираем тип с наивысшей уверенностью
                if 0.6 > confidence {
                    process_type = Some("io_intensive".to_string());
                    confidence = 0.6;
                }
            }
        }

        // Простая эвристика: аудио клиенты
        if process.is_audio_client {
            tags.insert("audio".to_string());
            tags.insert("realtime".to_string());
            // Выбираем тип с наивысшей уверенностью
            if 0.9 > confidence {
                process_type = Some("audio".to_string());
                confidence = 0.9;
            }
        }

        // Простая эвристика: фокусные окна
        if process.is_focused_window {
            tags.insert("focused".to_string());
            tags.insert("interactive".to_string());
            // Выбираем тип с наивысшей уверенностью
            if 0.9 > confidence {
                process_type = Some("focused".to_string());
                confidence = 0.9;
            }
        }

        // Если тип не определен, используем "unknown"
        if process_type.is_none() {
            process_type = Some("unknown".to_string());
            confidence = 0.3;
        }

        let duration = start_time.elapsed().as_micros();
        
        // Зарегистрировать успешную классификацию
        self.performance_metrics.record_successful_classification(duration, confidence);
        
        MLClassificationResult {
            process_type,
            tags: tags.into_iter().collect(),
            confidence,
        }
    }

    fn get_performance_metrics(&self) -> MLPerformanceMetrics {
        self.performance_metrics.clone()
    }

    fn reset_performance_metrics(&mut self) {
        self.performance_metrics.reset();
    }
}

/// ML-классификатор на основе CatBoost.
///
/// Использует предварительно обученную модель CatBoost для классификации процессов.
/// Поддерживает загрузку моделей в формате JSON и ONNX.
#[derive(Debug)]
pub struct CatBoostMLClassifier {
    /// Внутренняя модель CatBoost
    model: CatBoostModel,
    /// Метрики производительности
    performance_metrics: MLPerformanceMetrics,
}

/// Внутреннее представление модели CatBoost
#[derive(Debug)]
enum CatBoostModel {
    /// Модель в формате CatBoost JSON
    #[cfg(feature = "catboost")]
    Json(Arc<CatBoostClassifier>),
    /// Модель в формате ONNX
    #[cfg(feature = "onnx")]
    Onnx(Arc<Session>),
    /// Заглушка (используется когда CatBoost/ONNX отключены)
    Stub,
}

impl CatBoostMLClassifier {
    /// Создать новый CatBoost ML-классификатор.
    ///
    /// # Аргументы
    ///
    /// * `config` - конфигурация ML-классификатора
    ///
    /// # Возвращает
    ///
    /// Результат с новым классификатором или ошибкой при загрузке модели.
    pub fn new(config: MLClassifierConfig) -> Result<Self> {
        info!(
            "Создание CatBoost ML-классификатора с конфигурацией: {:?}",
            config
        );

        let model = if config.enabled {
            Self::load_model(&config).with_context(|| {
                format!("Не удалось загрузить модель из {:?}", config.model_path)
            })?
        } else {
            info!("ML-классификатор отключен в конфигурации, используется заглушка");
            CatBoostModel::Stub
        };

        Ok(Self {
            model,
            performance_metrics: MLPerformanceMetrics::new(),
        })
    }

    /// Загрузить модель из файла.
    ///
    /// # Аргументы
    ///
    /// * `config` - конфигурация ML-классификатора
    ///
    /// # Возвращает
    ///
    /// Загруженная модель или ошибка.
    fn load_model(config: &MLClassifierConfig) -> Result<CatBoostModel> {
        let model_path = Path::new(&config.model_path);

        if !model_path.exists() {
            return Err(anyhow::anyhow!(
                "Файл модели не найден: {}",
                config.model_path
            ));
        }

        info!("Загрузка ML-модели из: {}", config.model_path);

        if matches!(config.model_type, ModelType::Onnx) {
            #[cfg(feature = "onnx")]
            {
                Self::load_onnx_model(model_path)
                    .with_context(|| "Не удалось загрузить ONNX модель")
            }
            #[cfg(not(feature = "onnx"))]
            {
                warn!("ONNX поддержка отключена, но model_type=Onnx в конфигурации");
                #[cfg(feature = "catboost")]
                {
                    Self::load_catboost_model(model_path)
                        .with_context(|| "Не удалось загрузить CatBoost модель (ONNX отключен)")
                }
                #[cfg(not(feature = "catboost"))]
                {
                    Err(anyhow::anyhow!(
                        "ML поддержка отключена (и CatBoost, и ONNX отключены)"
                    ))
                }
            }
        } else {
            #[cfg(feature = "catboost")]
            {
                Self::load_catboost_model(model_path)
                    .with_context(|| "Не удалось загрузить CatBoost модель")
            }
            #[cfg(not(feature = "catboost"))]
            {
                Err(anyhow::anyhow!(
                    "ML поддержка отключена (CatBoost отключен)"
                ))
            }
        }
    }

    /// Загрузить CatBoost модель в формате JSON.
    ///
    /// # Аргументы
    ///
    /// * `model_path` - путь к файлу модели
    ///
    /// # Возвращает
    ///
    /// Загруженная модель или ошибка.
    #[cfg(feature = "catboost")]
    fn load_catboost_model(model_path: &Path) -> Result<CatBoostModel> {
        use std::fs::File;
        use std::io::Read;

        info!("Загрузка CatBoost модели из JSON файла: {:?}", model_path);

        let mut file = File::open(model_path)
            .with_context(|| format!("Не удалось открыть файл модели: {:?}", model_path))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .with_context(|| format!("Не удалось прочитать файл модели: {:?}", model_path))?;

        let model = CatBoostClassifier::from_json(&buffer)
            .with_context(|| "Не удалось разобрать CatBoost модель из JSON")?;

        Ok(CatBoostModel::Json(Arc::new(model)))
    }

    /// Загрузить ONNX модель.
    ///
    /// # Аргументы
    ///
    /// * `model_path` - путь к файлу модели
    ///
    /// # Возвращает
    ///
    /// Загруженная модель или ошибка.
    #[cfg(feature = "onnx")]
    fn load_onnx_model(model_path: &Path) -> Result<CatBoostModel> {
        info!("Загрузка ONNX модели: {:?}", model_path);

        let session = Session::builder()?
            .with_optimization_level(ort::GraphOptimizationLevel::Level3)?
            .with_intra_threads(1)?
            .with_inter_threads(1)?
            .commit_from_file(model_path)
            .with_context(|| "Не удалось загрузить ONNX модель")?;

        Ok(CatBoostModel::Onnx(Arc::new(session)))
    }

    /// Преобразовать процесс в фичи для ML-модели.
    ///
    /// # Аргументы
    ///
    /// * `process` - процесс для преобразования
    ///
    /// # Возвращает
    ///
    /// Вектор фич для ML-модели.
    ///
    /// # Примечание
    ///
    /// Этот метод используется внутренне в `classify_with_catboost` и `classify_with_onnx`.
    #[allow(dead_code)]
    fn process_to_features(&self, process: &ProcessRecord) -> Vec<f32> {
        let mut features = Vec::new();

        // Числовые фичи
        features.push(process.cpu_share_1s.unwrap_or(0.0) as f32);
        features.push(process.cpu_share_10s.unwrap_or(0.0) as f32);
        features.push(process.io_read_bytes.unwrap_or(0) as f32 / (1024.0 * 1024.0)); // MB
        features.push(process.io_write_bytes.unwrap_or(0) as f32 / (1024.0 * 1024.0)); // MB
        features.push(process.rss_mb.unwrap_or(0) as f32);
        features.push(process.swap_mb.unwrap_or(0) as f32);
        features.push(process.voluntary_ctx.unwrap_or(0) as f32);
        features.push(process.involuntary_ctx.unwrap_or(0) as f32);

        // Булевые фичи (0/1)
        features.push(if process.has_tty { 1.0 } else { 0.0 });
        features.push(if process.has_gui_window { 1.0 } else { 0.0 });
        features.push(if process.is_focused_window { 1.0 } else { 0.0 });
        features.push(if process.env_has_display { 1.0 } else { 0.0 });
        features.push(if process.env_has_wayland { 1.0 } else { 0.0 });
        features.push(if process.env_ssh { 1.0 } else { 0.0 });
        features.push(if process.is_audio_client { 1.0 } else { 0.0 });
        features.push(if process.has_active_stream { 1.0 } else { 0.0 });

        features
    }
}

impl MLClassifier for CatBoostMLClassifier {
    fn classify(&mut self, process: &ProcessRecord) -> MLClassificationResult {
        let start_time = Instant::now();
        
        let result = match &self.model {
            #[cfg(feature = "catboost")]
            CatBoostModel::Json(model) => self.classify_with_catboost(model, process),
            #[cfg(feature = "onnx")]
            CatBoostModel::Onnx(session) => self.classify_with_onnx(session, process),
            CatBoostModel::Stub => {
                debug!("ML-классификатор отключен, используется заглушка");
                let mut stub = StubMLClassifier::new();
                stub.classify(process)
            }
        };
        
        let duration = start_time.elapsed().as_micros();
        
        // Зарегистрировать успешную классификацию
        self.performance_metrics.record_successful_classification(duration, result.confidence);
        
        result
    }

    fn get_performance_metrics(&self) -> MLPerformanceMetrics {
        self.performance_metrics.clone()
    }

    fn reset_performance_metrics(&mut self) {
        self.performance_metrics.reset();
    }
}

#[cfg(feature = "catboost")]
impl CatBoostMLClassifier {
    /// Классифицировать процесс с использованием CatBoost модели.
    ///
    /// # Аргументы
    ///
    /// * `model` - CatBoost модель
    /// * `process` - процесс для классификации
    ///
    /// # Возвращает
    ///
    /// Результат классификации.
    fn classify_with_catboost(
        &self,
        model: &CatBoostClassifier,
        process: &ProcessRecord,
    ) -> MLClassificationResult {
        let features = self.process_to_features(process);

        // Преобразуем фичи в формат, ожидаемый CatBoost
        let input = vec![features];

        match model.predict(&input) {
            Ok(predictions) => {
                if predictions.is_empty() {
                    warn!("CatBoost модель вернула пустой результат");
                    return MLClassificationResult {
                        process_type: Some("unknown".to_string()),
                        tags: vec!["ml_failed".to_string()],
                        confidence: 0.1,
                    };
                }

                // Предполагаем, что модель возвращает вероятности для каждого класса
                // Находим класс с максимальной вероятностью
                let max_prob = predictions.iter().fold(f64::MIN, |a, &b| a.max(b));
                let class_idx = predictions.iter().position(|&p| p == max_prob).unwrap_or(0);

                // Преобразуем индекс класса в тип процесса
                let process_type = match class_idx {
                    0 => "unknown",
                    1 => "gui",
                    2 => "cpu_intensive",
                    3 => "io_intensive",
                    4 => "audio",
                    5 => "focused",
                    6 => "background",
                    7 => "batch",
                    _ => "unknown",
                };

                let mut tags = HashSet::new();

                // Добавляем теги на основе типа
                match process_type {
                    "gui" => {
                        tags.insert("gui".to_string());
                        tags.insert("interactive".to_string());
                    }
                    "audio" => {
                        tags.insert("audio".to_string());
                        tags.insert("realtime".to_string());
                    }
                    "focused" => {
                        tags.insert("focused".to_string());
                        tags.insert("interactive".to_string());
                    }
                    "cpu_intensive" => {
                        tags.insert("high_cpu".to_string());
                    }
                    "io_intensive" => {
                        tags.insert("high_io".to_string());
                    }
                    _ => {}
                }

                MLClassificationResult {
                    process_type: Some(process_type.to_string()),
                    tags: tags.into_iter().collect(),
                    confidence: max_prob as f64,
                }
            }
            Err(e) => {
                error!(
                    "Ошибка при предсказании с использованием CatBoost модели: {}",
                    e
                );
                // Зарегистрировать ошибку классификации
                let mut metrics = self.performance_metrics.clone();
                metrics.record_classification_error();
                MLClassificationResult {
                    process_type: Some("unknown".to_string()),
                    tags: vec!["ml_error".to_string()],
                    confidence: 0.1,
                }
            }
        }
    }
}

#[cfg(feature = "onnx")]
impl CatBoostMLClassifier {
    /// Классифицировать процесс с использованием ONNX модели.
    ///
    /// # Аргументы
    ///
    /// * `session` - ONNX сессия
    /// * `process` - процесс для классификации
    ///
    /// # Возвращает
    ///
    /// Результат классификации.
    fn classify_with_onnx(
        &self,
        session: &Session,
        process: &ProcessRecord,
    ) -> MLClassificationResult {
        let features = self.process_to_features(process);

        // Создаем входной тензор
        let input_tensor = ort::Tensor::from_array(
            ort::Array::from_shape_vec((1, features.len()), features).unwrap(),
        );

        let inputs = ort::inputs! {
            "input" => input_tensor,
        };

        match session.run(inputs) {
            Ok(outputs) => {
                if let Some(output) = outputs.get("output") {
                    if let Some(probabilities) = output.try_extract::<f32>() {
                        if probabilities.is_empty() {
                            warn!("ONNX модель вернула пустой результат");
                            return MLClassificationResult {
                                process_type: Some("unknown".to_string()),
                                tags: vec!["ml_failed".to_string()],
                                confidence: 0.1,
                            };
                        }

                        // Находим класс с максимальной вероятностью
                        let max_prob = probabilities.iter().fold(f32::MIN, |a, &b| a.max(b));
                        let class_idx = probabilities
                            .iter()
                            .position(|&p| p == max_prob)
                            .unwrap_or(0);

                        // Преобразуем индекс класса в тип процесса
                        let process_type = match class_idx {
                            0 => "unknown",
                            1 => "gui",
                            2 => "cpu_intensive",
                            3 => "io_intensive",
                            4 => "audio",
                            5 => "focused",
                            6 => "background",
                            7 => "batch",
                            _ => "unknown",
                        };

                        let mut tags = HashSet::new();

                        // Добавляем теги на основе типа
                        match process_type {
                            "gui" => {
                                tags.insert("gui".to_string());
                                tags.insert("interactive".to_string());
                            }
                            "audio" => {
                                tags.insert("audio".to_string());
                                tags.insert("realtime".to_string());
                            }
                            "focused" => {
                                tags.insert("focused".to_string());
                                tags.insert("interactive".to_string());
                            }
                            "cpu_intensive" => {
                                tags.insert("high_cpu".to_string());
                            }
                            "io_intensive" => {
                                tags.insert("high_io".to_string());
                            }
                            _ => {}
                        }

                        return MLClassificationResult {
                            process_type: Some(process_type.to_string()),
                            tags: tags.into_iter().collect(),
                            confidence: max_prob as f64,
                        };
                    }
                }

                warn!("ONNX модель вернула неожиданный формат вывода");
            }
            Err(e) => {
                error!("Ошибка при выполнении ONNX модели: {}", e);
            }
        }

        // Зарегистрировать ошибку классификации
        let mut metrics = self.performance_metrics.clone();
        metrics.record_classification_error();

        MLClassificationResult {
            process_type: Some("unknown".to_string()),
            tags: vec!["ml_error".to_string()],
            confidence: 0.1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::snapshots::ProcessRecord;

    fn create_test_process() -> ProcessRecord {
        ProcessRecord {
            pid: 1000,
            ppid: 1,
            uid: 1000,
            gid: 1000,
            exe: Some("test-app".to_string()),
            cmdline: None,
            cgroup_path: None,
            systemd_unit: None,
            app_group_id: None,
            state: "R".to_string(),
            start_time: 0,
            uptime_sec: 100,
            tty_nr: 0,
            has_tty: false,
            cpu_share_1s: None,
            cpu_share_10s: None,
            io_read_bytes: None,
            io_write_bytes: None,
            io_read_operations: None,
            io_write_operations: None,
            io_total_operations: None,
            io_last_update_ns: None,
            io_data_source: None,
            rss_mb: None,
            swap_mb: None,
            voluntary_ctx: None,
            involuntary_ctx: None,
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
            tags: Vec::new(),
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
            energy_uj: None,
            power_w: None,
            energy_timestamp: None,
            network_rx_bytes: None,
            network_tx_bytes: None,
            network_rx_packets: None,
            network_tx_packets: None,
            network_tcp_connections: None,
            network_udp_connections: None,
            network_last_update_ns: None,
            network_data_source: None,
            gpu_utilization: None,
            gpu_memory_bytes: None,
            gpu_time_us: None,
            gpu_api_calls: None,
            gpu_last_update_ns: None,
            gpu_data_source: None,
        }
    }

    #[test]
    fn test_stub_ml_classifier_gui_process() {
        let mut classifier = StubMLClassifier::new();
        let mut process = create_test_process();
        process.has_gui_window = true;

        let result = classifier.classify(&process);

        assert_eq!(result.process_type, Some("gui".to_string()));
        assert!(result.tags.contains(&"gui".to_string()));
        assert!(result.tags.contains(&"interactive".to_string()));
        assert!(result.confidence > 0.7);
    }

    #[test]
    fn test_stub_ml_classifier_high_cpu_process() {
        let mut classifier = StubMLClassifier::new();
        let mut process = create_test_process();
        process.cpu_share_10s = Some(0.5);

        let result = classifier.classify(&process);

        assert_eq!(result.process_type, Some("cpu_intensive".to_string()));
        assert!(result.tags.contains(&"high_cpu".to_string()));
        assert!(result.confidence > 0.6);
    }

    #[test]
    fn test_stub_ml_classifier_high_io_process() {
        let mut classifier = StubMLClassifier::new();
        let mut process = create_test_process();
        process.io_read_bytes = Some(2 * 1024 * 1024); // 2MB

        let result = classifier.classify(&process);

        assert_eq!(result.process_type, Some("io_intensive".to_string()));
        assert!(result.tags.contains(&"high_io".to_string()));
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_stub_ml_classifier_audio_process() {
        let mut classifier = StubMLClassifier::new();
        let mut process = create_test_process();
        process.is_audio_client = true;

        let result = classifier.classify(&process);

        assert_eq!(result.process_type, Some("audio".to_string()));
        assert!(result.tags.contains(&"audio".to_string()));
        assert!(result.tags.contains(&"realtime".to_string()));
        assert!(result.confidence > 0.8);
    }

    #[test]
    fn test_stub_ml_classifier_focused_process() {
        let mut classifier = StubMLClassifier::new();
        let mut process = create_test_process();
        process.is_focused_window = true;

        let result = classifier.classify(&process);

        assert_eq!(result.process_type, Some("focused".to_string()));
        assert!(result.tags.contains(&"focused".to_string()));
        assert!(result.tags.contains(&"interactive".to_string()));
        assert!(result.confidence > 0.8);
    }

    #[test]
    fn test_performance_metrics_initialization() {
        let metrics = MLPerformanceMetrics::new();
        assert_eq!(metrics.total_classifications, 0);
        assert_eq!(metrics.successful_classifications, 0);
        assert_eq!(metrics.classification_errors, 0);
        assert_eq!(metrics.total_classification_time_us, 0);
        assert!(metrics.min_classification_time_us.is_none());
        assert!(metrics.max_classification_time_us.is_none());
        assert_eq!(metrics.total_confidence, 0.0);
        assert_eq!(metrics.high_confidence_classifications, 0);
        assert_eq!(metrics.medium_confidence_classifications, 0);
        assert_eq!(metrics.low_confidence_classifications, 0);
    }

    #[test]
    fn test_performance_metrics_successful_classification() {
        let mut metrics = MLPerformanceMetrics::new();
        metrics.record_successful_classification(100, 0.9);
        
        assert_eq!(metrics.total_classifications, 1);
        assert_eq!(metrics.successful_classifications, 1);
        assert_eq!(metrics.classification_errors, 0);
        assert_eq!(metrics.total_classification_time_us, 100);
        assert_eq!(metrics.min_classification_time_us, Some(100));
        assert_eq!(metrics.max_classification_time_us, Some(100));
        assert_eq!(metrics.total_confidence, 0.9);
        assert_eq!(metrics.high_confidence_classifications, 1);
        assert_eq!(metrics.medium_confidence_classifications, 0);
        assert_eq!(metrics.low_confidence_classifications, 0);
        
        assert_eq!(metrics.average_classification_time_us(), Some(100.0));
        assert_eq!(metrics.average_confidence(), Some(0.9));
        assert_eq!(metrics.success_rate(), Some(1.0));
    }

    #[test]
    fn test_performance_metrics_multiple_classifications() {
        let mut metrics = MLPerformanceMetrics::new();
        metrics.record_successful_classification(100, 0.9);  // high confidence
        metrics.record_successful_classification(200, 0.6);  // medium confidence
        metrics.record_successful_classification(150, 0.4);  // low confidence
        
        assert_eq!(metrics.total_classifications, 3);
        assert_eq!(metrics.successful_classifications, 3);
        assert_eq!(metrics.classification_errors, 0);
        assert_eq!(metrics.total_classification_time_us, 450);
        assert_eq!(metrics.min_classification_time_us, Some(100));
        assert_eq!(metrics.max_classification_time_us, Some(200));
        assert_eq!(metrics.total_confidence, 1.9);
        assert_eq!(metrics.high_confidence_classifications, 1);
        assert_eq!(metrics.medium_confidence_classifications, 1);
        assert_eq!(metrics.low_confidence_classifications, 1);
        
        assert_eq!(metrics.average_classification_time_us(), Some(150.0));
        assert_eq!(metrics.average_confidence(), Some(1.9 / 3.0));
        assert_eq!(metrics.success_rate(), Some(1.0));
    }

    #[test]
    fn test_performance_metrics_with_errors() {
        let mut metrics = MLPerformanceMetrics::new();
        metrics.record_successful_classification(100, 0.8);
        metrics.record_classification_error();
        metrics.record_successful_classification(200, 0.7);
        
        assert_eq!(metrics.total_classifications, 3);
        assert_eq!(metrics.successful_classifications, 2);
        assert_eq!(metrics.classification_errors, 1);
        assert_eq!(metrics.total_classification_time_us, 300);
        assert_eq!(metrics.success_rate(), Some(2.0 / 3.0));
        assert_eq!(metrics.average_classification_time_us(), Some(150.0));
    }

    #[test]
    fn test_performance_metrics_reset() {
        let mut metrics = MLPerformanceMetrics::new();
        metrics.record_successful_classification(100, 0.9);
        metrics.record_classification_error();
        
        assert_eq!(metrics.total_classifications, 2);
        
        metrics.reset();
        
        assert_eq!(metrics.total_classifications, 0);
        assert_eq!(metrics.successful_classifications, 0);
        assert_eq!(metrics.classification_errors, 0);
        assert_eq!(metrics.total_classification_time_us, 0);
    }

    #[test]
    fn test_stub_classifier_performance_metrics() {
        let mut classifier = StubMLClassifier::new();
        let process = create_test_process();
        
        // Initial metrics should be empty
        let initial_metrics = classifier.get_performance_metrics();
        assert_eq!(initial_metrics.total_classifications, 0);
        
        // Classify a process
        let result = classifier.classify(&process);
        
        // Metrics should now show one classification
        let metrics = classifier.get_performance_metrics();
        assert_eq!(metrics.total_classifications, 1);
        assert_eq!(metrics.successful_classifications, 1);
        assert_eq!(metrics.classification_errors, 0);
        assert!(metrics.average_classification_time_us().is_some());
        assert_eq!(metrics.average_confidence(), Some(result.confidence));
        
        // Reset and verify
        classifier.reset_performance_metrics();
        let reset_metrics = classifier.get_performance_metrics();
        assert_eq!(reset_metrics.total_classifications, 0);
    }

    #[test]
    fn test_stub_ml_classifier_unknown_process() {
        let mut classifier = StubMLClassifier::new();
        let process = create_test_process();

        let result = classifier.classify(&process);

        assert_eq!(result.process_type, Some("unknown".to_string()));
        assert!(result.confidence < 0.5);
    }

    #[test]
    fn test_stub_ml_classifier_multiple_features() {
        let mut classifier = StubMLClassifier::new();
        let mut process = create_test_process();
        process.has_gui_window = true;
        process.cpu_share_10s = Some(0.4);
        process.is_audio_client = true;

        let result = classifier.classify(&process);

        // Должен быть выбран тип с наивысшей уверенностью (audio)
        assert_eq!(result.process_type, Some("audio".to_string()));
        // Должны быть теги от всех признаков
        assert!(result.tags.contains(&"gui".to_string()));
        assert!(result.tags.contains(&"interactive".to_string()));
        assert!(result.tags.contains(&"audio".to_string()));
        assert!(result.tags.contains(&"realtime".to_string()));
        assert!(result.tags.contains(&"high_cpu".to_string()));
        assert!(result.confidence > 0.8);
    }

    #[test]
    fn test_create_ml_classifier_disabled() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        let config = MLClassifierConfig {
            enabled: false,
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let classifier = create_ml_classifier(config);
        assert!(classifier.is_ok());

        // Должен вернуть StubMLClassifier
        let mut classifier = classifier.unwrap();
        let result = classifier.classify(&create_test_process());
        assert_eq!(result.process_type, Some("unknown".to_string()));
    }

    #[test]
    fn test_create_ml_classifier_nonexistent_model() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        let config = MLClassifierConfig {
            enabled: true,
            model_path: "/nonexistent/path/model.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let classifier = create_ml_classifier(config);
        assert!(classifier.is_err());

        // Должна быть ошибка о загрузке модели
        let err = classifier.unwrap_err();
        let err_str = err.to_string();
        assert!(err_str.contains("Не удалось загрузить модель"));
    }

    #[test]
    fn test_catboost_ml_classifier_feature_extraction() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        let config = MLClassifierConfig {
            enabled: false, // Отключаем, чтобы использовать заглушку
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let classifier = CatBoostMLClassifier::new(config).unwrap();
        let mut process = create_test_process();

        // Устанавливаем различные значения
        process.cpu_share_1s = Some(0.25);
        process.cpu_share_10s = Some(0.5);
        process.io_read_bytes = Some(2 * 1024 * 1024); // 2MB
        process.io_write_bytes = Some(1024 * 1024); // 1MB
        process.rss_mb = Some(100);
        process.swap_mb = Some(50);
        process.voluntary_ctx = Some(1000);
        process.involuntary_ctx = Some(500);
        process.has_tty = true;
        process.has_gui_window = true;
        process.is_focused_window = true;
        process.env_has_display = true;
        process.env_has_wayland = true;
        process.env_ssh = true;
        process.is_audio_client = true;
        process.has_active_stream = true;

        let features = classifier.process_to_features(&process);

        // Проверяем, что фичи извлечены правильно
        assert_eq!(features.len(), 16); // 8 числовых + 8 булевых

        // Проверяем числовые фичи
        assert_eq!(features[0], 0.25); // cpu_share_1s
        assert_eq!(features[1], 0.5); // cpu_share_10s
        assert_eq!(features[2], 2.0); // io_read_bytes в MB
        assert_eq!(features[3], 1.0); // io_write_bytes в MB
        assert_eq!(features[4], 100.0); // rss_mb
        assert_eq!(features[5], 50.0); // swap_mb
        assert_eq!(features[6], 1000.0); // voluntary_ctx
        assert_eq!(features[7], 500.0); // involuntary_ctx

        // Проверяем булевые фичи (должны быть 1.0)
        for feature in &features[8..16] {
            assert_eq!(*feature, 1.0);
        }
    }

    #[test]
    fn test_catboost_ml_classifier_feature_extraction_defaults() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        let config = MLClassifierConfig {
            enabled: false,
            model_path: "test.json".to_string(),
            confidence_threshold: 0.7,
            model_type: ModelType::Catboost,
        };

        let classifier = CatBoostMLClassifier::new(config).unwrap();
        let process = create_test_process(); // Все значения по умолчанию (None/0)

        let features = classifier.process_to_features(&process);

        // Проверяем, что фичи извлечены правильно
        assert_eq!(features.len(), 16);

        // Проверяем числовые фичи (должны быть 0.0)
        for feature in &features[0..8] {
            assert_eq!(*feature, 0.0);
        }

        // Проверяем булевые фичи (должны быть 0.0)
        for feature in &features[8..16] {
            assert_eq!(*feature, 0.0);
        }
    }

    #[test]
    fn test_ml_classifier_config_validation() {
        use crate::config::config_struct::{MLClassifierConfig, ModelType};

        // Тестируем дефолтную конфигурацию
        let default_config = MLClassifierConfig::default();
        assert!(!default_config.enabled);
        assert_eq!(default_config.model_path, "models/process_classifier.json");
        assert_eq!(default_config.confidence_threshold, 0.7);
        assert!(matches!(default_config.model_type, ModelType::Catboost));
    }
}
