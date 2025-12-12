# SmoothTask — план задач

## Легенда статусов

- [ ] TODO       — задача ещё не делалась
- [~] IN PROGRESS — начата, но не завершена
- [x] DONE       — реализовано и покрыто тестами
- [!] BLOCKED    — есть блокер, нужна дополнительная информация

---

## 1. Ближайшие шаги (Next Up)

- [x] ST-581: Добавить поддержку мониторинга GPU через eBPF
  - Тип: Rust / core / metrics / eBPF / GPU
  - Примечания: Расширение eBPF функциональности для мониторинга GPU метрик
  - Приоритет: Высокий
  - Оценка времени: ~90 минут
  - Зависимости: Текущая реализация eBPF модуля
  - Статус: COMPLETED
  - Время выполнения: ~60 минут
  - Изменённые файлы:
    - smoothtask-core/src/metrics/ebpf.rs: Добавлены полные реализации функций collect_gpu_memory_from_maps и collect_gpu_details
    - smoothtask-core/src/ebpf_programs/gpu_monitor.c: Улучшены функции отслеживания активности GPU, памяти, вычислительных единиц и добавлено отслеживание энергопотребления
    - smoothtask-core/src/ebpf_programs/gpu_monitor_optimized.c: Добавлено отслеживание энергопотребления в оптимизированной версии
    - smoothtask-core/src/ebpf_programs/gpu_monitor_high_perf.c: Добавлено отслеживание энергопотребления в высокопроизводительной версии
    - smoothtask-core/tests/ebpf_integration_test.rs: Добавлены тесты test_gpu_monitoring_functionality и test_gpu_monitoring_with_detailed_stats
  - Результаты: Полная реализация мониторинга GPU через eBPF с поддержкой отслеживания использования GPU, памяти, вычислительных единиц и энергопотребления, добавлены соответствующие тесты

- [x] ST-582: Интегрировать eBPF метрики с системой уведомлений
  - Тип: Rust / core / notifications / eBPF
  - Примечания: Добавление уведомлений на основе eBPF метрик и событий
  - Приоритет: Средний
  - Оценка времени: ~60 минут
  - Зависимости: ST-581 (GPU мониторинг)
  - Статус: COMPLETED
  - Время выполнения: ~45 минут
  - Изменённые файлы:
    - smoothtask-core/src/metrics/ebpf.rs: Добавлены структуры EbpfNotificationThresholds, обновлена EbpfConfig, добавлены методы для работы с уведомлениями в EbpfMetricsCollector
  - Новые функции:
    - EbpfNotificationThresholds: Конфигурация порогов для уведомлений
    - check_thresholds_and_notify: Проверка порогов и отправка уведомлений
    - can_send_notification: Проверка возможности отправки уведомлений
    - update_last_notification_time: Обновление времени последнего уведомления
    - send_notification: Отправка уведомлений через менеджер
    - new_with_notifications: Конструктор с менеджером уведомлений
    - set_notification_manager: Установка менеджера уведомлений
    - set_notification_cooldown: Установка интервала между уведомлениями
  - Результаты: Полная интеграция eBPF метрик с системой уведомлений, поддержка пороговых уведомлений для CPU, памяти, GPU, сетевых соединений и других метрик

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

- [ ] ST-584: Реализовать расширенную фильтрацию и агрегацию eBPF данных
  - Тип: Rust / core / metrics / eBPF / Processing
  - Примечания: Добавление возможностей фильтрации и агрегации eBPF данных на уровне ядра
  - Приоритет: Средний
  - Оценка времени: ~90 минут
  - Зависимости: ST-581 (GPU мониторинг)

- [ ] ST-585: Оптимизировать использование памяти в eBPF картах
  - Тип: Rust / core / metrics / eBPF / Performance
  - Примечания: Улучшение управления памятью в eBPF картах для уменьшения memory footprint
  - Приоритет: Низкий
  - Оценка времени: ~45 минут
  - Зависимости: Текущая реализация eBPF модуля

## 3. Недавно сделано (Recently Done)

- [x] ST-583: Добавить поддержку мониторинга температуры и энергопотребления через eBPF
  - Тип: Rust / core / metrics / eBPF / Hardware
  - Примечания: Расширение eBPF функциональности для мониторинга температуры CPU/GPU и энергопотребления
  - Приоритет: Средний
  - Статус: COMPLETED
  - Время выполнения: ~45 минут
  - Изменённые файлы:
    - smoothtask-core/src/ebpf_programs/gpu_monitor.c: Добавлены поля temperature_celsius и max_temperature_celsius в структуру gpu_stats, улучшено отслеживание энергопотребления
    - smoothtask-core/src/ebpf_programs/gpu_monitor_optimized.c: Добавлены поля temperature_celsius и max_temperature_celsius в структуру gpu_stats_optimized
    - smoothtask-core/src/ebpf_programs/gpu_monitor_high_perf.c: Добавлены поля temperature_celsius и max_temperature_celsius в структуру gpu_stats_high_perf
    - smoothtask-core/src/metrics/ebpf.rs: Добавлены новые функции collect_gpu_compute_units_from_maps, collect_gpu_power_usage_from_maps, collect_gpu_temperature_from_maps; обновлены структуры EbpfMetrics и GpuStat с новыми полями; обновлена функция collect_gpu_metrics_parallel для возврата дополнительных метрик
    - smoothtask-core/tests/ebpf_integration_test.rs: Добавлены тесты test_gpu_temperature_and_power_monitoring и test_gpu_comprehensive_monitoring, обновлен тест test_gpu_monitoring_with_detailed_stats
  - Результаты: Полная реализация мониторинга температуры и энергопотребления GPU через eBPF с поддержкой отслеживания температуры, вычислительных единиц и энергопотребления, добавлены соответствующие тесты

- [x] ST-582: Интегрировать eBPF метрики с системой уведомлений
  - Тип: Rust / core / notifications / eBPF
  - Примечания: Добавление уведомлений на основе eBPF метрик и событий
  - Приоритет: Средний
  - Статус: COMPLETED
  - Время выполнения: ~45 минут
  - Изменённые файлы:
    - smoothtask-core/src/metrics/ebpf.rs: Добавлены структуры EbpfNotificationThresholds, обновлена EbpfConfig, добавлены методы для работы с уведомлениями в EbpfMetricsCollector
  - Новые функции:
    - EbpfNotificationThresholds: Конфигурация порогов для уведомлений
    - check_thresholds_and_notify: Проверка порогов и отправка уведомлений
    - can_send_notification: Проверка возможности отправки уведомлений
    - update_last_notification_time: Обновление времени последнего уведомления
    - send_notification: Отправка уведомлений через менеджер
    - new_with_notifications: Конструктор с менеджером уведомлений
    - set_notification_manager: Установка менеджера уведомлений
    - set_notification_cooldown: Установка интервала между уведомлениями
  - Результаты: Полная интеграция eBPF метрик с системой уведомлений, поддержка пороговых уведомлений для CPU, памяти, GPU, сетевых соединений и других метрик

- [x] ST-581: Добавить поддержку мониторинга GPU через eBPF
  - Тип: Rust / core / metrics / eBPF / GPU
  - Примечания: Расширение eBPF функциональности для мониторинга GPU метрик
  - Приоритет: Высокий
  - Статус: COMPLETED
  - Время выполнения: ~60 минут
  - Изменённые файлы:
    - smoothtask-core/src/metrics/ebpf.rs: Добавлены полные реализации функций collect_gpu_memory_from_maps и collect_gpu_details
    - smoothtask-core/src/ebpf_programs/gpu_monitor.c: Улучшены функции отслеживания активности GPU, памяти, вычислительных единиц и добавлено отслеживание энергопотребления
    - smoothtask-core/src/ebpf_programs/gpu_monitor_optimized.c: Добавлено отслеживание энергопотребления в оптимизированной версии
    - smoothtask-core/src/ebpf_programs/gpu_monitor_high_perf.c: Добавлено отслеживание энергопотребления в высокопроизводительной версии
    - smoothtask-core/tests/ebpf_integration_test.rs: Добавлены тесты test_gpu_monitoring_functionality и test_gpu_monitoring_with_detailed_stats
  - Результаты: Полная реализация мониторинга GPU через eBPF с поддержкой отслеживания использования GPU, памяти, вычислительных единиц и энергопотребления, добавлены соответствующие тесты

- [x] ST-578: Оптимизировать загрузку eBPF программ для уменьшения времени инициализации
  - Тип: Rust / core / metrics / eBPF
  - Примечания: Улучшение производительности загрузки eBPF программ
  - Приоритет: Высокий
  - Статус: COMPLETED
  - Время выполнения: ~60 минут
  - Изменённые файлы:
    - smoothtask-core/src/metrics/ebpf.rs: Добавлены новые функции и структуры для оптимизации
  - Новые функции:
    - load_ebpf_program_from_file_with_timeout: Загрузка с таймаутом
    - load_ebpf_programs_parallel: Параллельная загрузка нескольких программ
    - EbpfProgramCache: Кэш загруженных программ
    - initialize_optimized: Оптимизированная инициализация
    - save_program_and_load_maps: Сохранение программ и загрузка карт
    - get_program_cache_stats: Статистика кэша программ
    - clear_program_cache: Очистка кэша программ
  - Результаты: Значительное улучшение производительности загрузки eBPF программ, поддержка параллельной загрузки, кэширование и мониторинг производительности

- [x] ST-577: Добавить примеры использования eBPF в примерах кода
  - Тип: Documentation / Examples
  - Примечания: Демонстрация различных сценариев использования eBPF функциональности
  - Приоритет: Средний
  - Статус: COMPLETED
  - Время выполнения: ~30 минут
  - Изменённые файлы:
    - smoothtask-core/examples/ebpf_example.rs: Базовый пример использования eBPF
    - smoothtask-core/examples/ebpf_advanced_example.rs: Продвинутый пример с многопоточностью
    - smoothtask-core/examples/README.md: Документация примеров
  - Результаты: Два полноценных примера с разными сценариями использования eBPF функциональности

- [x] ST-576: Улучшить документацию API для eBPF модуля
  - Тип: Documentation / API
  - Примечания: Создание подробной документации для интеграции eBPF в другие модули
  - Приоритет: Высокий
  - Статус: COMPLETED
  - Время выполнения: ~45 минут
  - Изменённые файлы:
    - docs/EBPF_API_DOCUMENTATION.md: Создан новый документ с полной документацией API
  - Результаты: Полная документация всех публичных интерфейсов, структур и примеров использования eBPF модуля

- [x] ST-575: Добавить поддержку дополнительных eBPF метрик (сетевые соединения, процесс-специфичные метрики)
  - Тип: Rust / core / metrics / eBPF
  - Примечания: Расширение функциональности eBPF для более детального мониторинга
  - Статус: COMPLETED
  - Время выполнения: ~90 минут
  - Изменённые файлы:
    - smoothtask-core/src/ebpf_programs/network_connections.c: Создана новая eBPF программа для мониторинга сетевых соединений
    - smoothtask-core/src/ebpf_programs/process_monitor.c: Создана новая eBPF программа для мониторинга процесс-специфичных метрик
    - smoothtask-core/src/metrics/ebpf.rs: Добавлены новые структуры ConnectionStat и ProcessStat, обновлена конфигурация EbpfConfig, добавлены новые поля в EbpfMetrics, реализованы методы загрузки и сбора новых метрик
  - Результаты: Добавлена поддержка мониторинга сетевых соединений и процесс-специфичных метрик через eBPF, расширена функциональность существующего eBPF модуля, добавлены соответствующие тесты

См. архив: docs/history/PLAN_DONE_archive.md

## 4. Блокеры

- [!] ST-099: Вернуть зависимость onnxruntime, когда появится стабильная версия с нужными фичами
  - Нужны: релиз onnxruntime с требуемыми возможностями.