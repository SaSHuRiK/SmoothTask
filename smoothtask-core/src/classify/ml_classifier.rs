//! ML-классификатор для классификации процессов.
//!
//! Этот модуль предоставляет интерфейс для классификации процессов
//! с использованием ML-моделей. В будущем здесь будет интеграция
//! с ONNX Runtime или другими ML-фреймворками.

use crate::logging::snapshots::ProcessRecord;
use std::collections::HashSet;

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
pub trait MLClassifier: Send + Sync {
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

/// Заглушка ML-классификатора для тестирования.
///
/// Использует простые эвристики для классификации процессов:
/// - Процессы с GUI получают тип "gui" и соответствующие теги
/// - Процессы с высоким CPU получают тип "cpu_intensive"
/// - Процессы с высоким IO получают тип "io_intensive"
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
}
