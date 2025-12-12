//! Модуль для классификации и группировки процессов.
//!
//! Этот модуль предоставляет функциональность для:
//! - Группировки процессов в AppGroup (группы процессов одного приложения)
//! - Классификации процессов по типам и тегам на основе паттерн-базы
//! - Классификации процессов с использованием ML-моделей
//!
//! # Компоненты
//!
//! - **grouper**: Группировка процессов по cgroup, PID namespace и другим признакам
//! - **rules**: Классификация процессов по паттернам из YAML-конфигов
//! - **ml_classifier**: ML-классификация процессов
//!
//! # Примеры использования
//!
//! ## Классификация с использованием ML
//!
//! ```no_run
//! use smoothtask_core::classify::ml_classifier::{MLClassifier, StubMLClassifier};
//! use smoothtask_core::classify::rules::{PatternDatabase, classify_process};
//! use smoothtask_core::logging::snapshots::ProcessRecord;
//! use std::path::Path;
//!
//! // Загрузка паттернов
//! let pattern_db = PatternDatabase::load(Path::new("configs/patterns")).expect("load patterns");
//!
//! // Создание ML-классификатора (заглушка для тестирования)
//! let ml_classifier = StubMLClassifier::new();
//!
//! // Создание тестового процесса
//! let mut process = ProcessRecord {
//!     pid: 1000,
//!     ppid: 1,
//!     uid: 1000,
//!     gid: 1000,
//!     exe: Some("firefox".to_string()),
//!     has_gui_window: true,
//!     cpu_share_10s: Some(0.5),
//!     // ... остальные поля
//!     process_type: None,
//!     tags: Vec::new(),
//!     // ... остальные поля
//! };
//!
//! // Классификация с использованием паттернов и ML
//! classify_process(&mut process, &pattern_db, Some(&ml_classifier), None);
//!
//! // Результаты классификации
//! println!("Process type: {:?}", process.process_type);
//! println!("Tags: {:?}", process.tags);
//! ```
//!
//! ## Интеграция с системой классификации
//!
//! ```no_run
//! use smoothtask_core::classify::rules::classify_all;
//! use smoothtask_core::classify::ml_classifier::StubMLClassifier;
//! use smoothtask_core::classify::rules::PatternDatabase;
//! use smoothtask_core::logging::snapshots::{ProcessRecord, AppGroupRecord};
//! use std::path::Path;
//!
//! // Загрузка паттернов
//! let pattern_db = PatternDatabase::load(Path::new("configs/patterns")).expect("load patterns");
//!
//! // Создание ML-классификатора
//! let ml_classifier = StubMLClassifier::new();
//!
//! // Создание тестовых данных
//! let mut processes = vec![/* ... */];
//! let mut app_groups = vec![/* ... */];
//!
//! // Классификация всех процессов и групп
//! classify_all(&mut processes, &mut app_groups, &pattern_db, Some(&ml_classifier));
//!
//! // Теперь процессы имеют классификацию на основе паттернов и ML
//! for process in &processes {
//!     println!("PID {}: type={:?}, tags={:?}", 
//!              process.pid, process.process_type, process.tags);
//! }
//! ```

pub mod grouper;
pub mod ml_classifier;
pub mod rules;
