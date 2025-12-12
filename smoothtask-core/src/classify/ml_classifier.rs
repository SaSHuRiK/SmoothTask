//! ML-классификатор для классификации процессов.
//!
//! Этот модуль предоставляет интерфейс для классификации процессов
//! с использованием ML-моделей. Поддерживает интеграцию с CatBoost
//! и ONNX Runtime для загрузки и использования предварительно обученных моделей.

use crate::config::config_struct::{MLClassifierConfig, ModelType};
use crate::logging::snapshots::ProcessRecord;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use anyhow::{Context, Result};
use tracing::{debug, error, info, warn};

#[cfg(feature = "catboost")]
use catboost::CatBoostClassifier;

#[cfg(feature = "onnx")]
use ort::Session;

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
    fn classify(&self, process: &ProcessRecord) -> MLClassificationResult;
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
pub struct StubMLClassifier;

impl StubMLClassifier {
    /// Создать новый заглушку ML-классификатора.
    pub fn new() -> Self {
        Self
    }
}

impl Default for StubMLClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl MLClassifier for StubMLClassifier {
    fn classify(&self, process: &ProcessRecord) -> MLClassificationResult {
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
            if io_read > 1024 * 1024 { // > 1MB
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

        MLClassificationResult {
            process_type,
            tags: tags.into_iter().collect(),
            confidence,
        }
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
    /// Конфигурация классификатора
    config: MLClassifierConfig,
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
        info!("Создание CatBoost ML-классификатора с конфигурацией: {:?}", config);

        let model = if config.enabled {
            Self::load_model(&config)
                .with_context(|| format!("Не удалось загрузить модель из {:?}", config.model_path))?
        } else {
            info!("ML-классификатор отключен в конфигурации, используется заглушка");
            CatBoostModel::Stub
        };

        Ok(Self { model, config })
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
                "Файл модели не найден: {}", config.model_path
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
                    Err(anyhow::anyhow!("ML поддержка отключена (и CatBoost, и ONNX отключены)"))
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
                Err(anyhow::anyhow!("ML поддержка отключена (CatBoost отключен)"))
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
    fn classify(&self, process: &ProcessRecord) -> MLClassificationResult {
        match &self.model {
            #[cfg(feature = "catboost")]
            CatBoostModel::Json(model) => {
                self.classify_with_catboost(model, process)
            }
            #[cfg(feature = "onnx")]
            CatBoostModel::Onnx(session) => {
                self.classify_with_onnx(session, process)
            }
            CatBoostModel::Stub => {
                debug!("ML-классификатор отключен, используется заглушка");
                StubMLClassifier::new().classify(process)
            }
        }
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
    fn classify_with_catboost(&self, model: &CatBoostClassifier, process: &ProcessRecord) -> MLClassificationResult {
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
                error!("Ошибка при предсказании с использованием CatBoost модели: {}", e);
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
    fn classify_with_onnx(&self, session: &Session, process: &ProcessRecord) -> MLClassificationResult {
        let features = self.process_to_features(process);

        // Создаем входной тензор
        let input_tensor = ort::Tensor::from_array(
            ort::Array::from_shape_vec((1, features.len()), features)
                .unwrap()
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
                        let class_idx = probabilities.iter().position(|&p| p == max_prob).unwrap_or(0);

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
        }
    }

    #[test]
    fn test_stub_ml_classifier_gui_process() {
        let classifier = StubMLClassifier::new();
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
        let classifier = StubMLClassifier::new();
        let mut process = create_test_process();
        process.cpu_share_10s = Some(0.5);

        let result = classifier.classify(&process);

        assert_eq!(result.process_type, Some("cpu_intensive".to_string()));
        assert!(result.tags.contains(&"high_cpu".to_string()));
        assert!(result.confidence > 0.6);
    }

    #[test]
    fn test_stub_ml_classifier_high_io_process() {
        let classifier = StubMLClassifier::new();
        let mut process = create_test_process();
        process.io_read_bytes = Some(2 * 1024 * 1024); // 2MB

        let result = classifier.classify(&process);

        assert_eq!(result.process_type, Some("io_intensive".to_string()));
        assert!(result.tags.contains(&"high_io".to_string()));
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_stub_ml_classifier_audio_process() {
        let classifier = StubMLClassifier::new();
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
        let classifier = StubMLClassifier::new();
        let mut process = create_test_process();
        process.is_focused_window = true;

        let result = classifier.classify(&process);

        assert_eq!(result.process_type, Some("focused".to_string()));
        assert!(result.tags.contains(&"focused".to_string()));
        assert!(result.tags.contains(&"interactive".to_string()));
        assert!(result.confidence > 0.8);
    }

    #[test]
    fn test_stub_ml_classifier_unknown_process() {
        let classifier = StubMLClassifier::new();
        let process = create_test_process();

        let result = classifier.classify(&process);

        assert_eq!(result.process_type, Some("unknown".to_string()));
        assert!(result.confidence < 0.5);
    }

    #[test]
    fn test_stub_ml_classifier_multiple_features() {
        let classifier = StubMLClassifier::new();
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
        let classifier = classifier.unwrap();
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
        
        // Должна быть ошибка о отсутствующем файле
        let err = classifier.unwrap_err();
        assert!(err.to_string().contains("не найден"));
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
        process.io_write_bytes = Some(1 * 1024 * 1024); // 1MB
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
        assert_eq!(features[1], 0.5);  // cpu_share_10s
        assert_eq!(features[2], 2.0);  // io_read_bytes в MB
        assert_eq!(features[3], 1.0);  // io_write_bytes в MB
        assert_eq!(features[4], 100.0); // rss_mb
        assert_eq!(features[5], 50.0);  // swap_mb
        assert_eq!(features[6], 1000.0); // voluntary_ctx
        assert_eq!(features[7], 500.0);  // involuntary_ctx
        
        // Проверяем булевые фичи (должны быть 1.0)
        for i in 8..16 {
            assert_eq!(features[i], 1.0);
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
        for i in 0..8 {
            assert_eq!(features[i], 0.0);
        }
        
        // Проверяем булевые фичи (должны быть 0.0)
        for i in 8..16 {
            assert_eq!(features[i], 0.0);
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
