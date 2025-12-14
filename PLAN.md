# SmoothTask — план задач

## Легенда статусов

- [ ] TODO       — задача ещё не делалась
- [~] IN PROGRESS — начата, но не завершена
- [x] DONE       — реализовано и покрыто тестами
- [!] BLOCKED    — есть блокер, нужна дополнительная информация

---

## 1. Ближайшие шаги (Next Up)

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

- [x] ST-827: Implement comprehensive caching system for metrics to reduce I/O overhead
  - Тип: Rust / core / metrics / optimization
  - Примечания: Advanced caching system to minimize filesystem I/O operations
  - Приоритет: Средний
  - Оценка времени: ~150 минут
  - Время выполнения: ~120 минут
  - Критерии готовности:
    - [x] Research caching strategies and patterns
    - [x] Implement multi-level caching (in-memory, disk-backed)
    - [x] Add cache invalidation and refresh mechanisms
    - [x] Implement cache statistics and monitoring
    - [x] Add memory-aware cache management
    - [x] Integrate with existing metrics collection system
    - [x] Add comprehensive unit and integration tests
  - Ожидаемые результаты: Significant reduction in I/O overhead and improved performance
  - Результаты:
    - Comprehensive LRU-based caching system with memory management
    - Memory-aware cache cleanup and TTL-based invalidation
    - Integration with system metrics collection via OptimizedMetricsCollector
    - Advanced cache statistics and monitoring capabilities
    - Comprehensive test suite with 15+ test cases covering various scenarios
    - Significant reduction in filesystem I/O operations through intelligent caching

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

- [x] ST-830: Add comprehensive integration tests for new features
  - Тип: Testing / integration
  - Примечания: Complete test coverage for all new features and components
  - Приоритет: Высокий
  - Оценка времени: ~180 минут
  - Время выполнения: ~150 минут
  - Критерии готовности:
    - [x] Identify critical integration scenarios
    - [x] Add integration tests for eBPF metrics
    - [x] Add integration tests for window monitoring
    - [x] Add integration tests for audio monitoring
    - [x] Add integration tests for input monitoring
    - [x] Add integration tests for policy engine enhancements
    - [x] Add end-to-end system tests
    - [x] Ensure all tests pass and provide good coverage
  - Ожидаемые результаты: Comprehensive test coverage and improved system reliability
  - Результаты:
    - Comprehensive eBPF integration tests with mock and real scenarios
    - Window monitoring integration tests with error handling and graceful degradation
    - Audio and input monitoring integration tests with fallback mechanisms
    - Policy engine integration tests covering hysteresis and ML ranker integration
    - End-to-end system tests for daemon operation and critical scenarios
    - High test coverage across all major components and integration points

## 3. Недавно сделано (Recently Done)

- [x] ST-826: Enhance policy engine with ML ranker integration and dynamic priority scaling
  - Тип: Rust / core / policy
  - Примечания: Integration of ML-based ranking with dynamic priority adjustment and hysteresis
  - Результаты: Added hysteresis mechanism to prevent priority thrashing and improved adaptive priority management

- [x] ST-823: Add X11 window introspection for application focus and window state monitoring
  - Тип: Rust / core / metrics / windows
  - Примечания: Implementation of X11 window monitoring for application focus detection
  - Результаты: Comprehensive X11 window introspection with full integration and robust error handling

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

- [x] ST-827: Implement comprehensive caching system for metrics to reduce I/O overhead
  - Тип: Rust / core / metrics / optimization
  - Примечания: Advanced caching system to minimize filesystem I/O operations
  - Результаты: Comprehensive LRU-based caching with memory management and intelligent invalidation

- [x] ST-830: Add comprehensive integration tests for new features
  - Тип: Testing / integration
  - Примечания: Complete test coverage for all new features and components
  - Результаты: Comprehensive test coverage across eBPF, window monitoring, audio/input, and policy engine

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