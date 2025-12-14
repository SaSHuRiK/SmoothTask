//! Модуль для сбора метрик системы и процессов.
//!
//! Этот модуль предоставляет функциональность для сбора различных метрик:
//! - Системные метрики (CPU, память, PSI)
//! - Метрики процессов (CPU, IO, память)
//! - Метрики энергопотребления процессов (RAPL, /proc/power, eBPF)
//! - Метрики окон и фокуса (X11/Wayland)
//! - Метрики аудио (PipeWire/PulseAudio, XRUN)
//! - Метрики ввода пользователя (evdev)
//! - Метрики scheduling latency
//! - Метрики GPU (использование, память, температура, энергопотребление)
//! - Метрики производительности приложений (задержки, FPS, использование ресурсов)
//!
//! # Компоненты
//!
//! - **system**: Глобальные метрики системы из /proc и PSI
//! - **process**: Метрики отдельных процессов
//! - **process_energy**: Мониторинг энергопотребления процессов
//! - **windows**: Интроспекция окон через X11/Wayland
//! - **audio**: Метрики аудио-системы (PipeWire/PulseAudio)
//! - **input**: Отслеживание активности пользователя
//! - **scheduling_latency**: Измерение задержек планировщика
//! - **nvml_wrapper**: Расширенный мониторинг NVIDIA GPU через NVML
//! - **amdgpu_wrapper**: Расширенный мониторинг AMD GPU через AMDGPU
//! - **gpu**: Мониторинг GPU устройств и их метрик
//! - **ebpf**: Высокопроизводительный сбор метрик через eBPF
//! - **filesystem_monitor**: Мониторинг файловой системы в реальном времени
//! - **extended_hardware_sensors**: Расширенный мониторинг аппаратных сенсоров
//! - **app_performance**: Метрики производительности приложений
//! - **ml_performance**: Метрики производительности ML-моделей и экспорт в Prometheus
//! - **hardware_acceleration**: Мониторинг аппаратного ускорения (VA-API, VDPAU, CUDA)
//! - **thunderbolt**: Мониторинг Thunderbolt устройств
//! - **pcie**: Мониторинг PCIe устройств

pub mod amdgpu_wrapper;
pub mod app_performance;
pub mod audio;
pub mod audio_pipewire;
pub mod cache;
pub mod custom;
pub mod ebpf;
pub mod extended_hardware_sensors;
pub mod filesystem_monitor;
pub mod gpu;
pub mod hardware_acceleration;
pub mod input;
pub mod ml_performance;
pub mod network;
pub mod nvml_wrapper;
pub mod process;
pub mod process_energy;
pub mod process_gpu;
pub mod process_network;
pub mod scheduling_latency;
pub mod system;
pub mod windows;
pub mod windows_wayland;
pub mod windows_x11;
