# SmoothTask — план задач

## Легенда статусов

- [ ] TODO       — задача ещё не делалась
- [~] IN PROGRESS — начата, но не завершена
- [x] DONE       — реализовано и покрыто тестами
- [!] BLOCKED    — есть блокер, нужна дополнительная информация

---

## 1. Ближайшие шаги (Next Up)

- [x] ST-606: Улучшить обработку ошибок в модуле API сервера
  - Тип: Rust / core / api
  - Примечания: Добавление более детальных сообщений об ошибок и улучшение graceful degradation для API endpoint'ов
  - Приоритет: Средний
  - Оценка времени: ~60 минут
  - Статус: COMPLETED
  - Время выполнения: ~60 минут
  - Изменённые файлы:
    - smoothtask-core/src/api/server.rs: Добавлен ApiError enum с различными типами ошибок, реализованы методы to_json_response для детальных JSON ответов, добавлены хелперы graceful_degradation_response и check_component_availability
    - smoothtask-core/Cargo.toml: Добавлена зависимость thiserror для улучшенной обработки ошибок
    - smoothtask-core/tests/api_integration_test.rs: Исправлены ошибки компиляции в тестах (конвертация String в Box<str>)
  - Новые функции:
    - ApiError enum: ValidationError, DataAccessError, ConfigurationError, InternalError, NotFoundError, ServiceUnavailableError
    - graceful_degradation_response: Хелпер для graceful degradation ответов
    - check_component_availability: Хелпер для проверки доступности компонентов
  - Обновлённые функции:
    - process_by_pid_handler: Использует ApiError для валидации и ошибок "не найдено"
    - appgroup_by_id_handler: Использует ApiError для валидации и ошибок "не найдено"
    - config_reload_handler: Использует ApiError для обработки ошибок конфигурации
    - health_detailed_handler: Улучшенная информация о компонентах и общем статусе
    - metrics_handler: Graceful degradation при недоступности метрик
    - processes_handler: Graceful degradation при недоступности процессов
    - appgroups_handler: Graceful degradation при недоступности групп приложений
  - Результаты: Полная реализация улучшенной обработки ошибок с детальными сообщениями, graceful degradation и комплексными тестами

- [ ] ST-607: Оптимизировать производительность сбора метрик процессов
  - Тип: Rust / core / metrics / performance
  - Примечания: Улучшение производительности сбора метрик процессов через кэширование и параллельную обработку
  - Приоритет: Средний
  - Оценка времени: ~90 минут
  - Критерии готовности:
    - Кэширование часто используемых данных о процессах
    - Параллельная обработка метрик процессов
    - Benchmark-тесты для измерения улучшения производительности

- [ ] ST-608: Добавить поддержку мониторинга сетевых соединений через eBPF
  - Тип: Rust / core / metrics / eBPF
  - Примечания: Расширение eBPF функциональности для мониторинга сетевых соединений и трафика
  - Приоритет: Низкий
  - Оценка времени: ~120 минут
  - Критерии готовности:
    - eBPF программа для мониторинга сетевых соединений
    - Интеграция с существующим eBPF модулем
    - Unit и интеграционные тесты

- [ ] ST-609: Улучшить документацию для API и конфигурации
  - Тип: Documentation
  - Примечания: Обновление и расширение документации API и конфигурации с примерами использования
  - Приоритет: Низкий
  - Оценка времени: ~60 минут
  - Критерии готовности:
    - Обновленная документация API с примерами
    - Документация по конфигурации с лучшими практиками
    - Примеры конфигурационных файлов

## 2. Бэклог

- [x] ST-583: Добавить поддержку мониторинга температуры и энергопотребления через eBPF
  - Тип: Rust / core / metrics / eBPF / Hardware
  - Примечания: Расширение eBPF функциональности для мониторинга температуры CPU/GPU и энергопотребления
  - Приоритет: Средний
  - Оценка времени: ~60 минут
  - Зависимости: Текущая реализация eBPF модуля
  - Статус: COMPLETED
  - Время выполнения: ~45 минут
  - Изменённые файлы:
    - smoothtask-core/src/ebpf_programs/gpu_monitor.c: Добавлены поля temperature_celsius и max_temperature_celsius в структуру gpu_stats, улучшено отслеживание энергопотребления
    - smoothtask-core/src/ebpf_programs/gpu_monitor_optimized.c: Добавлены поля temperature_celsius и max_temperature_celsius в структуру gpu_stats_optimized
    - smoothtask-core/src/ebpf_programs/gpu_monitor_high_perf.c: Добавлены поля temperature_celsius и max_temperature_celsius в структуру gpu_stats_high_perf
    - smoothtask-core/src/metrics/ebpf.rs: Добавлены новые функции collect_gpu_compute_units_from_maps, collect_gpu_power_usage_from_maps, collect_gpu_temperature_from_maps; обновлены структуры EbpfMetrics и GpuStat с новыми полями; обновлена функция collect_gpu_metrics_parallel для возврата дополнительных метрик
    - smoothtask-core/tests/ebpf_integration_test.rs: Добавлены тесты test_gpu_temperature_and_power_monitoring и test_gpu_comprehensive_monitoring, обновлен тест test_gpu_monitoring_with_detailed_stats
  - Результаты: Полная реализация мониторинга температуры и энергопотребления GPU через eBPF с поддержкой отслеживания температуры, вычислительных единиц и энергопотребления, добавлены соответствующие тесты

- [x] ST-584: Реализовать расширенную фильтрацию и агрегацию eBPF данных
  - Тип: Rust / core / metrics / eBPF / Processing
  - Примечания: Добавление возможностей фильтрации и агрегации eBPF данных на уровне ядра
  - Приоритет: Средний
  - Оценка времени: ~90 минут
  - Зависимости: ST-581 (GPU мониторинг)
  - Статус: COMPLETED
  - Время выполнения: ~60 минут
  - Изменённые файлы:
    - smoothtask-core/src/metrics/ebpf.rs: Добавлены структуры EbpfFilterConfig, методы apply_filtering, apply_aggregation, apply_filtering_and_aggregation и другие методы для настройки фильтрации и агрегации
  - Новые функции:
    - EbpfFilterConfig: Конфигурация фильтрации и агрегации данных
    - set_filter_config: Установка конфигурации фильтрации
    - apply_filtering: Применение фильтрации к метрикам
    - apply_aggregation: Применение агрегации к метрикам
    - apply_filtering_and_aggregation: Комбинированное применение фильтрации и агрегации
    - set_pid_filtering: Установка фильтрации по идентификаторам процессов
    - set_syscall_type_filtering: Установка фильтрации по типам системных вызовов
    - set_network_protocol_filtering: Установка фильтрации по сетевым протоколам
    - set_port_range_filtering: Установка фильтрации по диапазону портов
    - set_aggregation_parameters: Установка параметров агрегации
    - set_filtering_thresholds: Установка порогов фильтрации
  - Результаты: Полная реализация расширенной фильтрации и агрегации eBPF данных с поддержкой фильтрации по порогам, типам данных, идентификаторам процессов и агрегации детализированных статистик

- [x] ST-585: Оптимизировать использование памяти в eBPF картах
  - Тип: Rust / core / metrics / eBPF / Performance
  - Примечания: Улучшение управления памятью в eBPF картах для уменьшения memory footprint
  - Приоритет: Низкий
  - Оценка времени: ~45 минут
  - Зависимости: Текущая реализация eBPF модуля
  - Статус: COMPLETED
  - Время выполнения: ~45 минут
  - Изменённые файлы:
    - smoothtask-core/src/metrics/ebpf.rs: Добавлены методы optimize_ebpf_memory_usage, optimize_map_memory, analyze_map_usage, clear_map_entries, optimize_program_cache, optimize_detailed_stats, set_max_cached_details, get_max_cached_details
  - Новые функции:
    - optimize_ebpf_memory_usage: Оптимизация использования памяти в eBPF картах
    - optimize_map_memory: Оптимизация памяти для конкретной карты
    - analyze_map_usage: Анализ использования карты
    - clear_map_entries: Очистка всех записей в карте
    - optimize_program_cache: Оптимизация кэша программ
    - optimize_detailed_stats: Оптимизация использования памяти в детализированных статистиках
    - set_max_cached_details: Установка ограничения на количество кэшируемых детализированных статистик
    - get_max_cached_details: Получение текущего ограничения на количество кэшируемых детализированных статистик
  - Результаты: Полная реализация оптимизации памяти в eBPF картах с поддержкой анализа использования карт, очистки неиспользуемых записей, оптимизации кэша программ и ограничения количества детализированных статистик

## 3. Недавно сделано (Recently Done)

- [x] ST-606: Улучшить обработку ошибок в модуле API сервера
  - Тип: Rust / core / api
  - Примечания: Добавление более детальных сообщений об ошибках и улучшение graceful degradation для API endpoint'ов
  - Приоритет: Средний
  - Статус: COMPLETED
  - Время выполнения: ~60 минут
  - Изменённые файлы:
    - smoothtask-core/src/api/server.rs: Добавлен ApiError enum с различными типами ошибок, реализованы методы to_json_response для детальных JSON ответов, добавлены хелперы graceful_degradation_response и check_component_availability
    - smoothtask-core/Cargo.toml: Добавлена зависимость thiserror для улучшенной обработки ошибок
  - Результаты: Полная реализация улучшенной обработки ошибок с детальными сообщениями, graceful degradation и комплексными тестами

- [x] ST-605: Добавить документацию для новых eBPF возможностей
  - Тип: Documentation / API
  - Примечания: Создание подробной документации для новых eBPF функций
  - Приоритет: Низкий
  - Статус: COMPLETED
  - Время выполнения: ~60 минут
  - Изменённые файлы:
    - docs/EBPF_API_DOCUMENTATION.md: Добавлены новые разделы по фильтрации, агрегации, оптимизации памяти и мониторингу температуры
    - README.md: Добавлены примеры использования новых eBPF функций с кодом и конфигурацией
  - Результаты: Полная документация для новых eBPF функций с примерами кода и лучшими практиками

- [x] ST-604: Улучшить обработку ошибок в модуле eBPF
  - Тип: Rust / core / metrics / eBPF
  - Примечания: Добавление более информативных сообщений об ошибках и graceful degradation
  - Приоритет: Средний
  - Статус: COMPLETED
  - Время выполнения: ~60 минут
  - Изменённые файлы:
    - smoothtask-core/src/metrics/ebpf.rs: Улучшенные сообщения об ошибках с контекстом для всех eBPF операций, добавлена graceful degradation при недоступности eBPF функционала
  - Результаты: Улучшенная обработка ошибок с детальными сообщениями и graceful degradation

- [x] ST-603: Добавить комплексные интеграционные тесты для нового eBPF функционала
  - Тип: Rust / core / tests
  - Примечания: Тестирование полного цикла сбора метрик, фильтрации и оптимизации
  - Приоритет: Высокий
  - Статус: COMPLETED
  - Время выполнения: ~90 минут
  - Изменённые файлы:
    - smoothtask-core/tests/ebpf_integration_test.rs: Добавлены комплексные тесты для оптимизации памяти, фильтрации и агрегации, мониторинга температуры
  - Результаты: Полное покрытие тестами нового eBPF функционала

*(Остальные выполненные задачи перенесены в архив: docs/history/PLAN_DONE_archive.md)*

*(Остальные выполненные задачи перенесены в архив: docs/history/PLAN_DONE_archive.md)*

См. архив: docs/history/PLAN_DONE_archive.md

## 4. Блокеры

- [!] ST-099: Вернуть зависимость onnxruntime, когда появится стабильная версия с нужными фичами
  - Нужны: релиз onnxruntime с требуемыми возможностями.