//! Модуль для ML-моделей и ранжирования.
//!
//! Этот модуль предоставляет функциональность для построения фич из снапшотов
//! и ранжирования групп приложений на основе их характеристик.
//!
//! # Компоненты
//!
//! - **features**: Построение векторов фич из снапшотов для использования в ML-моделях
//! - **onnx_ranker**: ONNX-ранкер для загрузки и выполнения обученных CatBoost моделей
//! - **ranker**: Интерфейс для ранжирования AppGroup по приоритету (с заглушкой для тестирования)
//!
//! # Примеры использования
//!
//! ## Построение фич
//!
//! **Примечание:** Примеры помечены как `ignore`, потому что они требуют создания
//! сложных структур (`Snapshot`, `AppGroupRecord`) с реальными метриками системы,
//! что невозможно сделать в doctest'ах без доступа к `/proc` и другим системным ресурсам.
//! Для реального использования см. интеграционные тесты в `tests/` или примеры в `lib.rs`.
//!
//! ```ignore
//! use smoothtask_core::model::features::build_features;
//! use smoothtask_core::logging::snapshots::{Snapshot, AppGroupRecord};
//!
//! // Сбор снапшота и группировка процессов (опущено для краткости)
//! let snapshot: Snapshot = /* ... */;
//! let app_group: AppGroupRecord = /* ... */;
//!
//! // Построение фич для AppGroup
//! let features = build_features(&snapshot, &app_group);
//! println!("Total features: {}", features.total_features());
//! ```
//!
//! ## Ранжирование групп
//!
//! **Примечание:** Примеры помечены как `ignore`, потому что они требуют создания
//! сложных структур (`Snapshot`, `AppGroupRecord`) с реальными метриками системы,
//! что невозможно сделать в doctest'ах без доступа к `/proc` и другим системным ресурсам.
//! Для реального использования см. интеграционные тесты в `tests/` или примеры в `lib.rs`.
//!
//! ```ignore
//! use smoothtask_core::model::ranker::{Ranker, StubRanker};
//! use smoothtask_core::logging::snapshots::{Snapshot, AppGroupRecord};
//!
//! let ranker = StubRanker::new();
//! let snapshot: Snapshot = /* ... */;
//! let app_groups: Vec<AppGroupRecord> = /* ... */;
//!
//! // Ранжирование групп
//! let results = ranker.rank(&app_groups, &snapshot);
//!
//! // Использование результатов
//! for (app_group_id, result) in &results {
//!     println!("{}: score={:.2}, rank={}, percentile={:.2}",
//!              app_group_id, result.score, result.rank, result.percentile);
//! }
//! ```
//!
//! ## Интеграция features и ranker
//!
//! Модули `features` и `ranker` работают вместе: `build_features` извлекает метрики
//! из снапшота и группы приложений, а `ranker` использует эти фичи для ранжирования.
//! В текущей реализации `StubRanker` фичи строятся, но используются простые эвристики
//! на основе полей `AppGroupRecord`. В будущем фичи будут использоваться для ML-моделей.
//!
//! **Примечание:** Примеры помечены как `ignore`, потому что они требуют создания
//! сложных структур (`Snapshot`, `AppGroupRecord`) с реальными метриками системы,
//! что невозможно сделать в doctest'ах без доступа к `/proc` и другим системным ресурсам.
//! Для реального использования см. интеграционные тесты в `tests/` или примеры в `lib.rs`.
//!
//! ```ignore
//! use smoothtask_core::model::features::build_features;
//! use smoothtask_core::model::ranker::{Ranker, StubRanker};
//! use smoothtask_core::logging::snapshots::{Snapshot, AppGroupRecord};
//!
//! let snapshot: Snapshot = /* ... */;
//! let app_group: AppGroupRecord = /* ... */;
//!
//! // Построение фич для одной группы
//! let features = build_features(&snapshot, &app_group);
//! println!("Total features: {}", features.total_features());
//!
//! // Ранжирование нескольких групп
//! let ranker = StubRanker::new();
//! let app_groups: Vec<AppGroupRecord> = /* ... */;
//! let results = ranker.rank(&app_groups, &snapshot);
//!
//! // Использование результатов ранжирования
//! for (app_group_id, result) in &results {
//!     if result.percentile > 0.8 {
//!         println!("High priority: {}", app_group_id);
//!     }
//! }
//! ```
//!
//! # Будущие улучшения
//!
//! - Поддержка JSON-формата моделей для офлайн-ранжирования
//! - Кэширование фич для оптимизации производительности
//! - Улучшение обработки категориальных фич в ONNX-ранкере

pub mod features;
pub mod onnx_ranker;
pub mod ranker;
pub mod updater;
pub mod version;
