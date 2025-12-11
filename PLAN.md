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

- [x] ST-315: Добавить smoke-тесты load_snapshots_as_frame для пустых таблиц
  - Тип: Python / trainer / tests
  - Примечания: добавлены два smoke-теста на пустые таблицы и пустые snapshots; проверены возврат пустого DataFrame, отсутствие предупреждений при конвертации булевых столбцов и корректные dtype. Прогнан `uv run pytest smoothtask-trainer/tests/test_dataset.py`.

- [x] ST-314: Проверить astype в tune_policy/dataset на предупреждения
  - Тип: Python / trainer / maintenance
  - Примечания: конверсии bad_responsiveness переведены на nullable boolean -> Int8 без предупреждений; _to_bool теперь приводит напрямую к boolean. Добавлены тесты на отсутствие FutureWarning; прогнан `uv run pytest smoothtask-trainer/tests`.

- [x] ST-313: Архивировать старые DONE-задачи в PLAN.md
  - Тип: Документация / планирование
  - Примечания: в Recently Done оставлены 10 последних задач; ST-304–ST-200 перенесены в `docs/history/PLAN_DONE_archive.md`, ссылка обновлена.

- [x] ST-312: Усилить валидацию типов числовых колонок в build_feature_matrix
  - Тип: Python / trainer / features
  - Примечания: numeric-колонки валидируются перед fillna; нечисловые значения приводят к понятной ValueError с примерами значений. Строковые числа коэрсятся в float без предупреждений. Добавлены тесты на смешанные типы и ошибки; прогнан `uv run pytest smoothtask-trainer/tests/test_features.py`.

- [x] ST-311: Проверить fillna/astype в trainer на предупреждения
  - Тип: Python / trainer / maintenance
  - Примечания: проверены конверсии в build_feature_matrix; добавлен тест, гарантирующий отсутствие FutureWarning при fillna/astype с object/pd.NA и смешанными типами. Поддержан стабильный dtype для числовых и булевых колонок; прогнан `uv run pytest smoothtask-trainer/tests/test_features.py`.

- [x] ST-310: Устранить FutureWarning fillna в features
  - Тип: Python / trainer / maintenance
  - Примечания: build_feature_matrix приводит числовые колонки через `pd.to_numeric` перед `fillna`, категориальные колонки теперь приводятся к `string` до заполнения значений. Добавлен тест, фиксирующий отсутствие FutureWarning при object-колонках и проверяющий типы после заполнения. Прогнан `uv run python -m pytest smoothtask-trainer/tests/test_features.py`.

- [x] ST-309: Прогнать isort/black для trainer и тестов
  - Тип: Python / trainer / code quality
  - Примечания: isort и black применены к `smoothtask_trainer` и `smoothtask-trainer/tests`; форматирование обновлено без изменения логики; `uv run python -m pytest smoothtask-trainer/tests` проходит.

- [x] ST-308: Проверить неиспользуемые импорты в trainer и обновить fmt
  - Тип: Python / trainer / code quality
  - Примечания: импорты в `dataset.py`, `features.py`, `train_ranker.py` проверены, лишних не обнаружено; форматирование обновлено; прогнан `uv run python -m pytest smoothtask-trainer/tests`.

- [x] ST-307: Мини-код-ревизия API ответов на пустые паттерны
  - Тип: Rust / core / api / tests
  - Примечания: добавлен тест `test_patterns_handler_with_empty_database`, фиксирует ответ `/api/patterns` при пустой базе (пустой массив категорий, нулевые счётчики, message=null). Прогнан `cargo test -p smoothtask-core test_patterns_handler_with_empty_database`.

- [x] ST-306: Тесты для build_feature_matrix без части колонок
  - Тип: Python / trainer / tests
  - Примечания: расширен набор тестов в `smoothtask-trainer/tests/test_features.py` на отсутствие необязательных колонок; проверены дефолты для bool/cat/num фич и cat_idx. Прогнан `uv run pytest smoothtask-trainer/tests/test_features.py`.

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
