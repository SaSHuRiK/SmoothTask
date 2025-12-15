//! Модуль конфигурации SmoothTask.
//!
//! Предоставляет функциональность для загрузки, валидации и управления конфигурацией демона.
//! Включает основную конфигурацию и механизмы мониторинга изменений.

pub mod auto_reload;
pub mod config_struct;
pub mod watcher;

pub use auto_reload::ConfigAutoReload;
