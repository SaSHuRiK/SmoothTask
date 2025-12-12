# SmoothTask — план задач

## Легенда статусов

- [ ] TODO       — задача ещё не делалась
- [~] IN PROGRESS — начата, но не завершена
- [x] DONE       — реализовано и покрыто тестами
- [!] BLOCKED    — есть блокер, нужна дополнительная информация

---

## 1. Ближайшие шаги (Next Up)

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

- [x] ST-616: Оптимизировать использование памяти в eBPF картах
  - Тип: Rust / core / metrics / eBPF / Performance
  - Примечания: Улучшение управления памятью в eBPF картах для уменьшения memory footprint
  - Приоритет: Низкий
  - Статус: COMPLETED
  - Время выполнения: ~45 минут
  - Критерии готовности:
    - ✅ Методы оптимизации памяти для eBPF карт
    - ✅ Анализ использования карт
    - ✅ Очистка неиспользуемых записей
    - ✅ Ограничение количества детализированных статистик
  - Изменённые файлы:
    - smoothtask-core/src/metrics/ebpf.rs: Добавлены комплексные тесты для функций оптимизации памяти
  - Новые функции:
    - test_optimize_ebpf_memory_usage: Тест для проверки оптимизации памяти eBPF карт
    - test_max_cached_details_management: Тест для проверки управления ограничением кэшируемых деталей
    - test_optimize_detailed_stats_comprehensive: Тест для проверки оптимизации детализированной статистики
  - Результаты: Полная реализация оптимизации памяти в eBPF картах с поддержкой анализа использования карт, очистки неиспользуемых записей, оптимизации кэша программ и ограничения количества детализированных статистик, добавлены комплексные тесты

- [x] ST-615: Улучшить обработку ошибок в модуле метрик процессов
  - Тип: Rust / core / metrics
  - Примечания: Добавление более детальных сообщений об ошибках и улучшение graceful degradation
  - Приоритет: Средний
  - Статус: COMPLETED
  - Время выполнения: ~45 минут
  - Изменённые файлы:
    - smoothtask-core/src/metrics/process.rs: Улучшены сообщения об ошибках с практическими рекомендациями, добавлены тесты для проверки обработки ошибок
  - Результаты: Полная реализация улучшенной обработки ошибок с детальными сообщениями, graceful degradation и комплексными тестами

- [x] ST-614: Добавить поддержку мониторинга температуры CPU через eBPF
  - Тип: Rust / core / metrics / eBPF / Hardware
  - Примечания: Расширение eBPF функциональности для мониторинга температуры CPU
  - Приоритет: Средний
  - Статус: COMPLETED
  - Время выполнения: ~60 минут
  - Изменённые файлы:
    - smoothtask-core/src/ebpf_programs/cpu_temperature.c: Улучшена eBPF программа с добавлением критической температуры, счетчиков обновлений и ошибок
    - smoothtask-core/src/metrics/ebpf.rs: Обновлена структура CpuTemperatureStat и функции сбора температуры
    - smoothtask-core/src/api/server.rs: Добавлен новый обработчик cpu_temperature_handler и маршрут /api/cpu/temperature
    - smoothtask-core/tests/ebpf_integration_test.rs: Добавлены комплексные тесты для мониторинга температуры CPU
    - smoothtask-core/tests/api_integration_test.rs: Добавлены тесты для нового API endpoint
    - docs/API.md: Добавлена документация для нового endpoint /api/cpu/temperature
  - Результаты: Полная реализация мониторинга температуры CPU через eBPF с поддержкой детализированной статистики по каждому ядру, API endpoint для мониторинга и комплексными тестами

- [x] ST-613: Обновить документацию для новых функций и изменений
  - Тип: Documentation
  - Примечания: Обновление документации для новых функций, API и изменений в коде
  - Приоритет: Низкий
  - Статус: COMPLETED
  - Время выполнения: ~15 минут
  - Результаты: Проверка показала, что документация уже обновлена и актуальна. API.md содержит полную документацию для новых endpoint'ов (/api/cache/stats, /api/cache/clear, /api/cache/config, /api/network/connections, /api/cpu/temperature) с примерами использования и конфигурации. EBPF документация также обновлена с информацией о новых функциях мониторинга сетевых соединений, фильтрации и температуры CPU.

- [x] ST-612: Проверить наличие простых тестов, которые можно добавить
  - Тип: Rust / core / tests
  - Примечания: Проверка всех модулей на наличие функций без тестов или с недостаточным покрытием
  - Приоритет: Низкий
  - Статус: COMPLETED
  - Время выполнения: ~15 минут
  - Результаты: Проверка показала, что основные публичные функции уже имеют тесты. Новые функции (кэш процессов, сетевые соединения) имеют комплексные тесты. Тестовое покрытие находится на хорошем уровне.

- [x] ST-611: Проверить наличие простых улучшений кода (неиспользуемые импорты, упрощения, форматирование)
  - Тип: Rust / core / code quality
  - Примечания: Проверка всех модулей на неиспользуемые импорты, простые улучшения и форматирование
  - Приоритет: Низкий
  - Статус: COMPLETED
  - Время выполнения: ~30 минут
  - Результаты: Все предупреждения clippy устранены, код проходит проверку без ошибок, код отформатирован и оптимизирован

- [x] ST-610: Обновить API для поддержки новой конфигурации кэширования процессов
  - Тип: Rust / core / api
  - Примечания: Добавление API endpoint'ов для управления кэшем процессов и мониторинга его состояния
  - Приоритет: Средний
  - Статус: COMPLETED
  - Время выполнения: ~90 минут
  - Изменённые файлы:
    - smoothtask-core/src/metrics/process.rs: Добавлены ProcessCacheStats, get_process_cache_stats(), обновлена структура ProcessCache
    - smoothtask-core/src/api/server.rs: Добавлены обработчики cache_stats_handler, cache_clear_handler, cache_config_handler, cache_config_update_handler
    - smoothtask-core/tests/api_integration_test.rs: Добавлены тесты test_cache_stats_endpoint, test_cache_clear_endpoint, test_cache_config_endpoint, test_cache_config_update_endpoint
    - docs/API.md: Добавлена документация для новых endpoint'ов /api/cache/stats, /api/cache/clear, /api/cache/config
  - Новые функции:
    - ProcessCache::get_cache_stats(): Получение статистики кэша
    - get_process_cache_stats(): Публичная функция для получения статистики кэша
    - cache_stats_handler(): Обработчик GET /api/cache/stats
    - cache_clear_handler(): Обработчик POST /api/cache/clear
    - cache_config_handler(): Обработчик GET /api/cache/config
    - cache_config_update_handler(): Обработчик POST /api/cache/config
  - Результаты: Полная реализация API для управления кэшем процессов с поддержкой мониторинга, очистки и динамического обновления конфигурации, добавлены комплексные тесты и документация

- [x] ST-608: Добавить поддержку мониторинга сетевых соединений через eBPF
  - Тип: Rust / core / metrics / eBPF
  - Примечания: Расширение eBPF функциональности для мониторинга сетевых соединений и трафика
  - Приоритет: Низкий
  - Статус: COMPLETED
  - Время выполнения: ~90 минут
  - Изменённые файлы:
    - smoothtask-core/src/ebpf_programs/network_connections.c: Добавлены функции trace_connection_close и trace_connection_data
    - smoothtask-core/src/ebpf_programs/network_monitor.c: Добавлены функции trace_tcp_connection и trace_udp_connection
    - smoothtask-core/src/api/server.rs: Добавлены network_connections_handler, format_ip, protocol_to_string, is_connection_active
    - docs/API.md: Добавлена документация для /api/network/connections и конфигурации eBPF
  - Результаты: Полная реализация мониторинга сетевых соединений через eBPF с API и документацией

- [x] ST-609: Улучшить документацию для API и конфигурации
  - Тип: Documentation
  - Примечания: Обновление и расширение документации API и конфигурации с примерами использования
  - Приоритет: Низкий
  - Статус: COMPLETED
  - Время выполнения: ~45 минут
  - Изменённые файлы:
    - docs/API.md: Добавлена документация для /api/network/connections и конфигурации eBPF
  - Результаты: Полная документация для новых eBPF функций с примерами и лучшими практиками

- [x] ST-607: Оптимизировать производительность сбора метрик процессов
  - Тип: Rust / core / metrics / performance
  - Примечания: Улучшение производительности сбора метрик процессов через кэширование и параллельную обработку
  - Приоритет: Средний
  - Статус: COMPLETED
  - Время выполнения: ~120 минут
  - Изменённые файлы:
    - smoothtask-core/src/metrics/process.rs: Добавлены ProcessCacheConfig, ProcessCache, CachedProcessRecord, глобальный кэш PROCESS_CACHE, обновлена функция collect_process_metrics для поддержки кэширования и конфигурируемого параллелизма, добавлены публичные функции clear_process_cache, update_process_cache_config, get_process_cache_config, исправлены ошибки в логике кэширования
    - smoothtask-core/benches/process_metrics_bench.rs: Добавлены комплексные бенчмарки для измерения производительности с различными конфигурациями кэширования и параллелизма
    - smoothtask-core/src/lib.rs: Обновлены импорты и использование collect_process_metrics_legacy для совместимости
    - smoothtask-core/benches/simple_benchmarks.rs: Обновлено использование collect_process_metrics_legacy
  - Новые функции:
    - ProcessCacheConfig: Конфигурация кэширования с параметрами TTL, размера кэша, параллелизма
    - ProcessCache: Структура кэша с методами cleanup_stale_entries, cache_record, get_cached
    - clear_process_cache: Публичная функция для очистки глобального кэша
    - update_process_cache_config: Публичная функция для обновления конфигурации кэша
    - get_process_cache_config: Публичная функция для получения текущей конфигурации
  - Исправления:
    - Исправлена ошибка в cleanup_stale_entries: устранено конфликт заимствования при использовании self.config в замыкании
    - Исправлена логика cache_record: учет новой записи при проверке лимита кэша
    - Добавлены задержки в тестах для гарантии разных временных меток
  - Результаты: Полная реализация оптимизации производительности сбора метрик процессов с поддержкой кэширования, конфигурируемого параллелизма, ограничения размера кэша и комплексными тестами. Все тесты проходят успешно.

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

См. архив: docs/history/PLAN_DONE_archive.md

## 4. Блокеры

- [!] ST-099: Вернуть зависимость onnxruntime, когда появится стабильная версия с нужными фичами
  - Нужны: релиз onnxruntime с требуемыми возможностями.
