# SmoothTask — план задач

## Легенда статусов

- [ ] TODO       — задача ещё не делалась
- [~] IN PROGRESS — начата, но не завершена
- [x] DONE       — реализовано и покрыто тестами
- [!] BLOCKED    — есть блокер, нужна дополнительная информация

---

## 1. Ближайшие шаги (Next Up)

- [x] ST-822: Implement eBPF-based system metrics collection with comprehensive error handling
  - Тип: Rust / core / metrics / ebpf
  - Примечания: Full integration of eBPF metrics collection for CPU, memory, and I/O monitoring
  - Приоритет: Высокий
  - Статус: COMPLETED
  - Время выполнения: ~90 минут
  - Оценка времени: ~120 минут
  - Критерии готовности:
    - [x] Research eBPF APIs and existing implementations
    - [x] Implement eBPF-based CPU monitoring with per-core statistics
    - [x] Add eBPF-based memory monitoring with detailed breakdown
    - [x] Implement eBPF-based I/O monitoring with latency tracking
    - [x] Add comprehensive error handling and graceful degradation
    - [x] Integrate with existing metrics collection system
    - [x] Add unit and integration tests
  - Ожидаемые результаты: More accurate and detailed system monitoring with minimal overhead
  - Результаты:
    - Enhanced error classification system with Critical, Recoverable, and Informational categories
    - Improved error recovery strategies based on error category
    - Better integration of eBPF metrics with system metrics (temperature, power, CPU usage)
    - Added health status monitoring functions (get_health_status, is_healthy)
    - Comprehensive test suite for error classification, health monitoring, and recovery
    - Enhanced logging with appropriate severity levels based on error category
    - Graceful degradation with fallback to cached metrics when eBPF is unavailable

- [ ] ST-823: Add X11 window introspection for application focus and window state monitoring
  - Тип: Rust / core / metrics / windows
  - Примечания: Implementation of X11 window monitoring for application focus detection
  - Приоритет: Высокий
  - Оценка времени: ~90 минут
  - Критерии готовности:
    - [ ] Research X11 APIs and EWMH standards
    - [ ] Implement window listing and focus detection
    - [ ] Add application-to-window mapping
    - [ ] Implement workspace/desktop tracking
    - [ ] Add error handling for X11 connection issues
    - [ ] Integrate with process classification system
    - [ ] Add unit and integration tests
  - Ожидаемые результаты: Better application awareness and focus-based priority management

- [ ] ST-826: Enhance policy engine with ML ranker integration and dynamic priority scaling
  - Тип: Rust / core / policy
  - Примечания: Integration of ML-based ranking with dynamic priority adjustment
  - Приоритет: Высокий
  - Оценка времени: ~150 минут
  - Критерии готовности:
    - [ ] Research ML ranker integration patterns
    - [ ] Implement ranker model loading and inference
    - [ ] Add dynamic priority scaling based on system load
    - [ ] Implement hysteresis to prevent priority thrashing
    - [ ] Add comprehensive error handling and fallbacks
    - [ ] Integrate with existing policy engine
    - [ ] Add unit and integration tests
  - Ожидаемые результаты: More intelligent and adaptive priority management

## 2. Бэклог

- [ ] ST-824: Implement PipeWire audio monitoring for XRUN detection and audio stream analysis
  - Тип: Rust / core / metrics / audio
  - Примечания: Audio monitoring for real-time audio application detection and XRUN tracking
  - Приоритет: Средний
  - Оценка времени: ~120 минут
  - Критерии готовности:
    - [ ] Research PipeWire APIs and audio monitoring techniques
    - [ ] Implement XRUN detection for audio glitch monitoring
    - [ ] Add audio stream analysis with process mapping
    - [ ] Implement volume and latency monitoring
    - [ ] Add error handling for audio subsystem issues
    - [ ] Integrate with process classification system
    - [ ] Add unit and integration tests
  - Ожидаемые результаты: Better audio application awareness and real-time audio monitoring

- [ ] ST-825: Add evdev input device monitoring for user activity detection
  - Тип: Rust / core / metrics / input
  - Примечания: Input device monitoring for user presence and activity detection
  - Приоритет: Средний
  - Оценка времени: ~90 минут
  - Критерии готовности:
    - [ ] Research evdev APIs and input monitoring techniques
    - [ ] Implement keyboard and mouse activity tracking
    - [ ] Add touchpad and touchscreen support
    - [ ] Implement user presence detection algorithms
    - [ ] Add error handling for input device issues
    - [ ] Integrate with policy engine for activity-based prioritization
    - [ ] Add unit and integration tests
  - Ожидаемые результаты: Better user activity awareness for intelligent priority management

- [ ] ST-827: Implement comprehensive caching system for metrics to reduce I/O overhead
  - Тип: Rust / core / metrics / optimization
  - Примечания: Advanced caching system to minimize filesystem I/O operations
  - Приоритет: Средний
  - Оценка времени: ~150 минут
  - Критерии готовности:
    - [ ] Research caching strategies and patterns
    - [ ] Implement multi-level caching (in-memory, disk-backed)
    - [ ] Add cache invalidation and refresh mechanisms
    - [ ] Implement cache statistics and monitoring
    - [ ] Add memory-aware cache management
    - [ ] Integrate with existing metrics collection system
    - [ ] Add comprehensive unit and integration tests
  - Ожидаемые результаты: Significant reduction in I/O overhead and improved performance

- [ ] ST-828: Add Wayland window introspection support for modern desktop environments
  - Тип: Rust / core / metrics / windows
  - Примечания: Wayland compositor integration for window monitoring
  - Приоритет: Средний
  - Оценка времени: ~180 минут
  - Критерии готовности:
    - [ ] Research Wayland protocols and compositor APIs
    - [ ] Implement window listing and focus detection for major compositors
    - [ ] Add workspace and virtual desktop tracking
    - [ ] Implement application-to-window mapping
    - [ ] Add error handling for Wayland connection issues
    - [ ] Integrate with existing X11 window monitoring
    - [ ] Add unit and integration tests
  - Ожидаемые результаты: Complete window monitoring support for modern Linux desktops

- [ ] ST-829: Implement advanced error recovery and system health monitoring
  - Тип: Rust / core / health
  - Примечания: Comprehensive system health monitoring and automatic recovery
  - Приоритет: Средний
  - Оценка времени: ~120 минут
  - Критерии готовности:
    - [ ] Research system health monitoring patterns
    - [ ] Implement component health checks and status monitoring
    - [ ] Add automatic recovery mechanisms for critical components
    - [ ] Implement system-wide health scoring
    - [ ] Add comprehensive logging and alerting
    - [ ] Integrate with existing monitoring infrastructure
    - [ ] Add unit and integration tests
  - Ожидаемые результаты: More robust and self-healing system operation

- [ ] ST-830: Add comprehensive integration tests for new features
  - Тип: Testing / integration
  - Примечания: Complete test coverage for all new features and components
  - Приоритет: Высокий
  - Оценка времени: ~180 минут
  - Критерии готовности:
    - [ ] Identify critical integration scenarios
    - [ ] Add integration tests for eBPF metrics
    - [ ] Add integration tests for window monitoring
    - [ ] Add integration tests for audio monitoring
    - [ ] Add integration tests for input monitoring
    - [ ] Add integration tests for policy engine enhancements
    - [ ] Add end-to-end system tests
    - [ ] Ensure all tests pass and provide good coverage
  - Ожидаемые результаты: Comprehensive test coverage and improved system reliability

## 3. Недавно сделано (Recently Done)

- [x] ST-822: Implement eBPF-based system metrics collection with comprehensive error handling
  - Тип: Rust / core / metrics / ebpf
  - Примечания: Full integration of eBPF metrics collection for CPU, memory, and I/O monitoring
  - Результаты: Enhanced error classification, health monitoring, and better integration with system metrics

- [x] ST-821: Улучшить обработку ошибок в GPU модулях с более детальными сообщениями
  - Тип: Rust / core / metrics / error_handling
  - Примечания: Улучшение сообщений об ошибках и механизмов восстановления для GPU мониторинга
  - Результаты: Детальные сообщения об ошибках с возможными причинами и рекомендациями по устранению

- [x] ST-820: Добавить поддержку мониторинга GPU температуры в существующий модуль system.rs
  - Тип: Rust / core / metrics / system
  - Примечания: Расширение существующего мониторинга температуры для включения GPU
  - Результаты: GPU температура собирается из различных источников с приоритезацией

- [x] ST-819: Интегрировать GPU метрики в основной цикл сбора метрик
  - Тип: Rust / core / metrics / integration
  - Примечания: Интеграция новых NVML и AMDGPU метрик в основной модуль сбора метрик
  - Результаты: Полная интеграция GPU мониторинга в основной цикл сбора данных

- [x] ST-818: Добавить поддержку мониторинга температуры CPU/GPU
  - Тип: Rust / core / metrics / system
  - Примечания: Мониторинг температуры для лучшего управления производительностью
  - Результаты: Полноценная функциональность мониторинга температуры с поддержкой различных сенсоров

- [x] ST-817: Улучшить систему логирования с ротацией и сжатием
  - Тип: Rust / core / logging
  - Примечания: Улучшенная система логирования с автоматическим управлением
  - Результаты: Автоматическая ротация и сжатие логов с поддержкой конфигурации

*(Более старые задачи перенесены в архив: см. docs/history/)*

## 4. Блокеры

*(На данный момент нет активных блокеров)*