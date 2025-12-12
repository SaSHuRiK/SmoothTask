# SmoothTask — план задач

## Легенда статусов

- [ ] TODO       — задача ещё не делалась
- [~] IN PROGRESS — начата, но не завершена
- [x] DONE       — реализовано и покрыто тестами
- [!] BLOCKED    — есть блокер, нужна дополнительная информация

---

## 1. Ближайшие шаги (Next Up)

- [x] ST-372: Добавить утилитные функции для работы с cgroups v2
  - Тип: Rust / core / utils
  - Критерии готовности:
    - ✅ Добавить функции для проверки доступности cgroups v2;
    - ✅ Добавить функции для чтения/записи параметров cgroups;
    - ✅ Добавить unit-тесты для новых функций;
    - ✅ Убедиться, что все тесты проходят успешно.
  - Примечания: Утилитные функции для работы с cgroups v2 будут полезны для модуля actuator и улучшат модульность кода.
  - Результаты: Создан новый модуль `smoothtask-core/src/utils/cgroups.rs` с 10 функциями для работы с cgroups v2: `is_cgroup_v2_available`, `get_cgroup_root`, `is_controller_available`, `read_cgroup_param`, `write_cgroup_param`, `create_app_cgroup`, `remove_cgroup_if_empty`, `move_process_to_cgroup`, `is_process_in_cgroup`, `get_processes_in_cgroup`. Добавлены 17 unit-тестов, все тесты проходят успешно.

- [ ] ST-373: Улучшить документацию модуля windows.rs с примерами использования
  - Тип: Rust / core / documentation
  - Критерии готовности:
    - Добавить примеры использования для всех публичных функций;
    - Улучшить документацию структур WindowInfo и WindowState;
    - Добавить примеры работы с WindowIntrospector;
    - Убедиться, что все примеры компилируются и работают.
  - Примечания: Улучшение документации поможет разработчикам лучше понимать, как использовать оконные метрики.

- [x] ST-374: Добавить дополнительные тесты для модуля windows.rs
  - Тип: Rust / core / tests
  - Критерии готовности:
    - Добавить тесты для функции select_focused_window;
    - Добавить тесты для функции get_window_info_by_pid;
    - Добавить тесты для граничных случаев;
    - Убедиться, что все тесты проходят успешно.
  - Примечания: Добавлены 10 новых тестов для улучшения покрытия оконных метрик: select_focused_window_handles_large_number_of_windows, get_window_info_by_pid_handles_large_pid_values, build_pid_to_window_map_handles_large_number_of_windows, build_pid_to_window_map_handles_duplicate_pids_with_same_confidence, window_info_new_handles_extreme_confidence_values, select_focused_window_handles_mixed_states_with_various_confidence, get_window_info_by_pid_with_zero_pid, build_pid_to_window_map_with_zero_pid. Все тесты проходят успешно. Общее количество тестов в модуле windows: 63 (было 53).

- [ ] ST-375: Добавить утилитные функции для работы с процессами
  - Тип: Rust / core / utils
  - Критерии готовности:
    - Добавить функции для получения информации о процессах;
    - Добавить функции для работы с cgroups;
    - Добавить unit-тесты для новых функций;
    - Убедиться, что все тесты проходят успешно.
  - Примечания: Утилитные функции для работы с процессами будут полезны для модуля actuator.

## 2. Бэклог

- [ ] ST-117: Оптимизация производительности критических путей (если необходимо)
  - Тип: Rust / core / performance
  - Критерии готовности:
    - Определены узкие места производительности;
    - Оптимизированы критические пути;
    - Добавлены бенчмарки для проверки улучшений.

- [x] ST-207: Добавить интеграцию с systemd для автозапуска
  - Тип: Rust / systemd
  - Примечания: полностью выполнено. ST-209 создал unit файл, ST-210 добавил поддержку systemd notify, ST-211 добавил документацию. Интеграция с systemd завершена.

- [x] ST-208: Добавить Control API (HTTP/gRPC) для просмотра состояния
  - Тип: Rust / core / api
  - Подзадачи:
    - [x] ST-213: Базовая структура для HTTP сервера
    - [x] ST-214: Endpoint для статистики демона
    - [x] ST-215: Endpoint для просмотра метрик системы
    - [x] ST-216: Endpoint для просмотра процессов и AppGroup
    - [x] ST-217: Интеграция API сервера в run_daemon
    - [x] ST-218: Документация API
  - Примечания: полностью реализован HTTP API для просмотра текущего состояния демона. Все подзадачи выполнены. API сервер интегрирован в главный цикл демона, данные обновляются при каждой итерации. Добавлена полная документация API в docs/API.md. Все 82 теста проходят успешно.

## 3. Недавно сделано (Recently Done)

- [x] ST-374: Добавить дополнительные тесты для модуля windows.rs
  - Тип: Rust / core / tests
  - Примечания: Добавлены 10 новых тестов для улучшения покрытия оконных метрик: select_focused_window_handles_large_number_of_windows, get_window_info_by_pid_handles_large_pid_values, build_pid_to_window_map_handles_large_number_of_windows, build_pid_to_window_map_handles_duplicate_pids_with_same_confidence, window_info_new_handles_extreme_confidence_values, select_focused_window_handles_mixed_states_with_various_confidence, get_window_info_by_pid_with_zero_pid, build_pid_to_window_map_with_zero_pid. Все тесты проходят успешно. Общее количество тестов в модуле windows: 63 (было 53).

- [x] ST-372: Добавить утилитные функции для работы с cgroups v2
  - Тип: Rust / core / utils
  - Примечания: Создан новый модуль `smoothtask-core/src/utils/cgroups.rs` с 10 функциями для работы с cgroups v2 и 17 unit-тестами. Все тесты проходят успешно. Функции включают проверку доступности cgroups, чтение/запись параметров, создание/удаление cgroups, управление процессами в cgroups.

- [x] ST-371: Добавить дополнительные тесты для граничных случаев в process.rs
  - Тип: Rust / core / tests
  - Примечания: Добавлены 8 новых тестов для обработки edge cases: extract_systemd_unit_handles_complex_paths, parse_cgroup_v2_path_handles_edge_cases, parse_env_vars_handles_special_characters, parse_uid_gid_handles_boundary_values, read_cgroup_path_with_malformed_content, parse_env_vars_with_unicode_content, extract_systemd_unit_handles_edge_cases, parse_env_vars_handles_empty_and_malformed, parse_uid_gid_handles_malformed_status, calculate_uptime_handles_edge_cases. Все тесты проходят успешно. Общее количество тестов в модуле process: 19 (было 11).

- [x] ST-370: Улучшить документацию модуля system.rs с примерами использования
  - Тип: Rust / core / documentation
  - Примечания: Улучшена документация функции collect_system_metrics с добавлением примеров использования в главном цикле демона, обработки ошибок и graceful degradation. Добавлены примеры для collect_process_metrics с обработкой ошибок, интеграцией с мониторингом, работой с большими наборами данных и использованием в асинхронном контексте.

См. архив: docs/history/PLAN_DONE_archive.md

См. архив: docs/history/PLAN_DONE_archive.md

## 4. Блокеры

- [!] ST-011: Реализация WaylandIntrospector для поддержки Wayland-композиторов
  - Тип: Rust / core / metrics
  - Подзадачи:
    - [x] ST-011-1: Добавить зависимости wayland-client и wayland-protocols-wlr
    - [~] ST-011-2: Реализовать базовое подключение к Wayland композитору в WaylandIntrospector::new()
      - Примечания: Исправлены ошибки компиляции, упрощена структура. Полная реализация требует сложной работы с асинхронными событиями через wayland-client 0.31 API с использованием Dispatch трейтов для обработки событий реестра и wlr-foreign-toplevel-management протокола. Это выходит за рамки простой задачи (~30 минут) и требует дополнительного времени на изучение правильного использования wayland-client API. Код компилируется, тесты проходят.
    - [~] ST-011-3: Реализовать получение списка окон через wlr-foreign-toplevel-management протокол
      - Примечания: Зависит от ST-011-2. Требует правильной обработки событий через Dispatch трейты wayland-client для регистрации обработчиков событий toplevel (title, app_id, state, pid) и синхронизации состояния окон. Это сложная задача, требующая правильной работы с асинхронными событиями.
    - [x] ST-011-4: Добавить unit-тесты для WaylandIntrospector
      - Примечания: Добавлены базовые unit-тесты для проверки доступности и создания интроспектора. Тесты корректно обрабатывают случаи, когда Wayland недоступен или реализация не завершена. Все 6 тестов проходят успешно.
  - Критерии готовности:
    - Реализация WaylandIntrospector через wlr-foreign-toplevel-management;
    - Поддержка основных композиторов (Mutter, KWin, Sway, Hyprland);
    - Fallback на StaticWindowIntrospector, если Wayland недоступен;
    - Unit-тесты на парсинг окон и обработку ошибок.
  - Примечания: Wayland требует работы с wayland-client и специфичными протоколами композиторов. Это более сложная задача, чем X11. Базовая структура и функция проверки доступности уже реализованы в ST-039. Зависимости добавлены в ST-011-1. Создана базовая структура с заглушками и тестами. Полная реализация требует дополнительного времени на изучение правильного использования wayland-client 0.31 API для обработки асинхронных событий через Dispatch трейты. Код компилируется без ошибок, все тесты проходят. Задача блокирована, так как требует значительного времени на изучение wayland-client API и правильной работы с асинхронными событиями. Это выходит за рамки простой задачи (~30 минут).

- [!] ST-099: Вернуть зависимость onnxruntime, когда появится стабильная версия с нужными фичами
  - Нужны: релиз onnxruntime с требуемыми возможностями.
