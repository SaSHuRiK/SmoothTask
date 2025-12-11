//! Модуль для сбора метрик системы и процессов.
//!
//! Этот модуль предоставляет функциональность для сбора различных метрик:
//! - Системные метрики (CPU, память, PSI)
//! - Метрики процессов (CPU, IO, память)
//! - Метрики окон и фокуса (X11/Wayland)
//! - Метрики аудио (PipeWire/PulseAudio, XRUN)
//! - Метрики ввода пользователя (evdev)
//! - Метрики scheduling latency
//!
//! # Компоненты
//!
//! - **system**: Глобальные метрики системы из /proc и PSI
//! - **process**: Метрики отдельных процессов
//! - **windows**: Интроспекция окон через X11/Wayland
//! - **audio**: Метрики аудио-системы (PipeWire/PulseAudio)
//! - **input**: Отслеживание активности пользователя
//! - **scheduling_latency**: Измерение задержек планировщика

pub mod audio;
pub mod audio_pipewire;
pub mod input;
pub mod process;
pub mod scheduling_latency;
pub mod system;
pub mod windows;
pub mod windows_wayland;
pub mod windows_x11;
