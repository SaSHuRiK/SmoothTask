# SmoothTask — план задач

## Легенда статусов

- [ ] TODO       — задача ещё не делалась
- [~] IN PROGRESS — начата, но не завершена
- [x] DONE       — реализовано и покрыто тестами
- [!] BLOCKED    — есть блокер, нужна дополнительная информация

---

## 1. Ближайшие шаги (Next Up)

- Нет активных ближайших задач — нужно определить новые при следующей сессии.

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

- [x] ST-323: Улучшить сообщение об ошибке для `_json_list`
  - Тип: Python / trainer / dataset
  - Примечания: json.JSONDecodeError оборачивается в ValueError с понятным текстом; добавлен тест на невалидную строку. Прогнан `uv run python -m pytest smoothtask-trainer/tests/test_dataset.py`.

- [x] ST-322: Стабилизировать порядок строк в `load_snapshots_as_frame`
  - Тип: Python / trainer / dataset
  - Примечания: результат сортируется по snapshot_id/pid; добавлен тест на стабильный порядок при несортированных вставках. Прогнан `uv run python -m pytest smoothtask-trainer/tests/test_dataset.py`.

- [x] ST-321: Проверка числовых таргетов teacher/responsiveness
  - Тип: Python / trainer / features
  - Примечания: build_feature_matrix валидирует целевой столбец, выдавая понятный ValueError при нечисловых значениях; числовые строки коэрсятся без предупреждений. Прогнан `uv run python -m pytest smoothtask-trainer/tests/test_features.py`.

- [x] ST-320: Валидация snapshot_id в build_feature_matrix
  - Тип: Python / trainer / features
  - Примечания: snapshot_id приводится к Int64, NaN/None или нечисловые значения вызывают ValueError; group_id сохраняет целочисленный тип после фильтрации таргета. Прогнан `uv run python -m pytest smoothtask-trainer/tests/test_features.py`.

- [x] ST-319: Добавить unit-тесты для `_ensure_column` (дефолты и dtype)
  - Тип: Python / trainer / features / tests
  - Примечания: добавлен тест на сохранение индекса и приведение dtype при существующей колонке и создании новой с дефолтами; проверены boolean и float сценарии; прогнан `uv run python -m pytest smoothtask-trainer/tests/test_features.py`.

- [x] ST-318: Добавить unit-тесты для `_prepare_tags_column` в `features`
  - Тип: Python / trainer / features / tests
  - Примечания: покрыты списки/множества/кортежи/скаляры/NaN/None и пустые коллекции, обеспечена сортировка тегов и возврат "unknown" для отсутствующих значений; прогнан `uv run python -m pytest smoothtask-trainer/tests/test_features.py`.

- [x] ST-317: export_model создаёт директории и валидирует выходной путь
  - Тип: Python / trainer / export
  - Примечания: добавлена автосоздающаяся директория для выхода, явная ошибка при сохранении в каталог; покрыто тестами вложенного пути и запретом на каталог; прогнан `uv run python -m pytest smoothtask-trainer/tests/test_export_model.py`.

- [x] ST-316: Улучшить ошибки load_snapshots_as_frame при отсутствии таблиц
  - Тип: Python / trainer / dataset
  - Примечания: `_load_table` теперь отдаёт ValueError с названием таблицы при отсутствии/ошибке SQLite; добавлена проверка обязательных столбцов и тесты на отсутствие таблицы и PID; прогнан `uv run python -m pytest smoothtask-trainer/tests/test_dataset.py`.

- [x] ST-315: Добавить smoke-тесты load_snapshots_as_frame для пустых таблиц
  - Тип: Python / trainer / tests
  - Примечания: добавлены два smoke-теста на пустые таблицы и пустые snapshots; проверены возврат пустого DataFrame, отсутствие предупреждений при конвертации булевых столбцов и корректные dtype. Прогнан `uv run pytest smoothtask-trainer/tests/test_dataset.py`.
- [x] ST-314: Проверить astype в tune_policy/dataset на предупреждения
  - Тип: Python / trainer / maintenance
  - Примечания: конверсии bad_responsiveness переведены на nullable boolean -> Int8 без предупреждений; _to_bool теперь приводит напрямую к boolean. Добавлены тесты на отсутствие FutureWarning; прогнан `uv run pytest smoothtask-trainer/tests`.

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
