# Руководство по CatBoost v1 в SmoothTask

Этот документ описывает функциональность CatBoost v1 в SmoothTask, включая обучение моделей, интеграцию ONNX Runtime для инференса и использование в режиме `dry-run`.

## Обзор

CatBoost v1 реализация в SmoothTask включает:

1. **Обучение Ranker'а** - обучение CatBoost модели на собранных данных снапшотов
2. **ONNX Runtime интеграция** - загрузка и выполнение ONNX моделей для инференса
3. **Режим dry-run** - тестирование модели без применения приоритетов
4. **Гибридный режим** - использование ML-ранкера вместе с правилами
5. **Экспорт с метаданными** - сохранение моделей с дополнительной информацией

## 1. Обучение CatBoost Ranker'а

### Требования

- Python 3.13+
- Установленные зависимости: `catboost`, `pandas`, `numpy`, `scikit-learn`
- SQLite база данных с собранными снапшотами

### Процесс обучения

1. **Сбор данных**: Демон SmoothTask должен быть запущен с включенным логированием снапшотов:

```yaml
# В конфигурационном файле
enable_snapshot_logging: true
paths:
  snapshot_db_path: "/var/lib/smoothtask/snapshots.db"
```

2. **Подготовка данных**: Данные автоматически собираются в SQLite базу данных с таблицами:
   - `snapshots` - системные метрики и метрики отзывчивости
   - `processes` - информация о процессах
   - `app_groups` - группы приложений

3. **Запуск обучения**:

```bash
cd smoothtask-trainer
.venv/bin/python -m smoothtask_trainer.train_ranker \
  --db-path /var/lib/smoothtask/snapshots.db \
  --model-out models/ranker.cbm \
  --onnx-out models/ranker.onnx
```

### Экспорт с метаданными

Новая функциональность поддерживает экспорт моделей с метаданными:

```bash
# Экспорт с метаданными
.venv/bin/python -m smoothtask_trainer.export_model \
  --model-path models/ranker.json \
  --format onnx \
  --output-path models/ranker.onnx \
  --metadata '{"version": "1.0.0", "description": "Модель для SmoothTask", "author": "My Team"}' \
  --validate
```

Метаданные сохраняются в отдельном JSON файле рядом с моделью:
- Для ONNX: `model.onnx.metadata.json`
- Для JSON: `model.json.metadata.json`
- Для CBM: `model.cbm.metadata.json`

Пример содержимого метаданных:

```json
{
  "version": "1.0.0",
  "description": "Модель для ранжирования процессов в SmoothTask",
  "author": "SmoothTask Team",
  "dataset_size": 1000,
  "features": ["cpu_usage", "memory_usage", "io_wait", "gpu_usage"],
  "training_date": "2024-01-15",
  "model_type": "CatBoostRanker",
  "export_format": "onnx",
  "export_timestamp": 1705324800.123456
}
```

### Параметры обучения

- `--db-path`: Путь к SQLite базе данных со снапшотами
- `--model-out`: Путь для сохранения обученной CatBoost модели (.cbm)
- `--onnx-out`: Путь для сохранения ONNX модели (опционально)
- `--test-size`: Размер тестовой выборки (по умолчанию 0.2)
- `--random-state`: Seed для воспроизводимости (по умолчанию 42)
- `--categorical-features`: Использовать категориальные фичи (по умолчанию False для ONNX)

### Пример обучения

```bash
# Обучение с сохранением в ONNX формат
.venv/bin/python -m smoothtask_trainer.train_ranker \
  --db-path snapshots.db \
  --model-out ranker.cbm \
  --onnx-out ranker.onnx \
  --test-size 0.3 \
  --random-state 42

# Обучение без ONNX (только CatBoost модель)
.venv/bin/python -m smoothtask_trainer.train_ranker \
  --db-path snapshots.db \
  --model-out ranker.cbm \
  --categorical-features true
```

### Валидация данных

Функция обучения выполняет валидацию входных данных:
- Проверка существования базы данных
- Проверка наличия необходимых таблиц
- Проверка наличия достаточного количества данных
- Обработка ошибок и логирование

## 2. ONNX Runtime Интеграция

### Требования

- ONNX Runtime библиотека для Rust (`ort` crate)
- ONNX модель, экспортированная из CatBoost

### Загрузка и выполнение модели

```rust
// Пример использования ONNX ранкера
use smoothtask_core::model::onnx_ranker::OnnxRanker;

let ranker = OnnxRanker::new("models/ranker.onnx")?;
let app_groups = vec![/* ваши AppGroup */];
let scores = ranker.rank_app_groups(&app_groups)?;
```

### Основные функции

1. **Загрузка модели**: `OnnxRanker::new(model_path)`
2. **Выполнение инференса**: `ranker.rank_app_groups(app_groups)`
3. **Преобразование фич**: Автоматическое преобразование фич в тензоры
4. **Обработка результатов**: Преобразование выходных данных в оценки

### Обработка ошибок

- Ошибки загрузки модели
- Ошибки выполнения инференса
- Несовпадение размеров тензоров
- Логирование ошибок

## 3. Режим Dry-Run

### Назначение

Режим dry-run позволяет тестировать ONNX модель без фактического применения приоритетов к процессам. Это полезно для:
- Тестирования новой модели
- Оценки производительности
- Отладки и мониторинга

### Конфигурация

```yaml
# В конфигурационном файле
policy_mode: "hybrid"
dry_run_default: true

# Конфигурация ONNX ранкера
ml_ranker:
  enabled: true
  model_path: "/etc/smoothtask/models/ranker.onnx"
  use_categorical_features: false
```

### Использование

1. **Включение режима dry-run**:

```bash
# Запуск демона в режиме dry-run
sudo /usr/local/bin/smoothtaskd \
  --config /etc/smoothtask/smoothtask.yml \
  --dry-run
```

2. **Проверка работы**:

```bash
# Просмотр логов для проверки работы ранкера
sudo journalctl -u smoothtaskd.service -f

# Проверка API для получения информации о приоритетах
curl http://127.0.0.1:8080/api/appgroups | jq
```

## 4. Гибридный Режим

### Архитектура

Гибридный режим сочетает:
1. **Правила (Rules)**: Базовые семантические правила
2. **ML-ранкер**: CatBoost модель для тонкой настройки приоритетов

### Конфигурация

```yaml
policy_mode: "hybrid"

ml_ranker:
  enabled: true
  model_path: "/etc/smoothtask/models/ranker.onnx"
  use_categorical_features: false
```

### Приоритеты

1. **Guardrails**: Правила имеют высший приоритет
2. **ML-ранкер**: Используется для тонкой настройки внутри классов
3. **Fallback**: Если модель недоступна, используются правила

## 5. Мониторинг и Отладка

### Логирование

```bash
# Просмотр логов демона
sudo journalctl -u smoothtaskd.service -f

# Фильтрация логов по ONNX
sudo journalctl -u smoothtaskd.service | grep ONNX
```

### API Мониторинг

```bash
# Проверка загруженных классов
curl http://127.0.0.1:8080/api/classes | jq

# Проверка групп приложений
curl http://127.0.0.1:8080/api/appgroups | jq

# Проверка конфигурации
curl http://127.0.0.1:8080/api/config | jq '.config.ml_ranker'
```

## 6. Примеры Использования

### Полный цикл обучения и развертывания

```bash
# 1. Сбор данных (демон должен работать с логированием)
sudo systemctl restart smoothtaskd.service
sleep 3600  # Сбор данных в течение часа

# 2. Обучение модели
cd smoothtask-trainer
.venv/bin/python -m smoothtask_trainer.train_ranker \
  --db-path /var/lib/smoothtask/snapshots.db \
  --model-out /etc/smoothtask/models/ranker.cbm \
  --onnx-out /etc/smoothtask/models/ranker.onnx

# 3. Настройка конфигурации
nano /etc/smoothtask/smoothtask.yml
# Изменить policy_mode на "hybrid" и указать путь к модели

# 4. Перезагрузка конфигурации
curl -X POST http://127.0.0.1:8080/api/config/reload

# 5. Мониторинг
curl http://127.0.0.1:8080/api/appgroups | jq '.app_groups[].priority_class'
```

### Тестирование в режиме dry-run

```bash
# 1. Запуск в режиме dry-run
sudo /usr/local/bin/smoothtaskd \
  --config /etc/smoothtask/smoothtask.yml \
  --dry-run

# 2. Проверка логов
sudo journalctl -u smoothtaskd.service -f | grep "dry-run"

# 3. Проверка API
curl http://127.0.0.1:8080/api/appgroups | jq '.app_groups[] | {app_group_id, priority_class}'
```

## 7. Устранение Неполадок

### Ошибки обучения

**Проблема**: Недостаточно данных для обучения

**Решение**:
- Увеличьте время сбора данных
- Проверьте, что логирование снапшотов включено
- Проверьте наличие данных в базе: `sqlite3 snapshots.db "SELECT COUNT(*) FROM snapshots;"`

**Проблема**: Ошибка экспорта в ONNX

**Решение**:
- Убедитесь, что используется совместимая версия CatBoost
- Отключите категориальные фичи для ONNX: `--categorical-features false`
- Проверьте зависимости: `pip install onnx onnxruntime`

### Ошибки инференса

**Проблема**: Модель не загружается

**Решение**:
- Проверьте путь к модели в конфигурации
- Проверьте права доступа: `chmod 644 /etc/smoothtask/models/ranker.onnx`
- Проверьте формат модели: должна быть ONNX

**Проблема**: Ошибки выполнения инференса

**Решение**:
- Проверьте совместимость версии ONNX Runtime
- Проверьте размеры входных тензоров
- Включите режим отладки: `--debug`

## 8. Производительность и Оптимизация

### Оптимизация обучения

- Используйте меньший `test-size` для быстрого тестирования
- Увеличьте `random-state` для воспроизводимости
- Используйте категориальные фичи только при необходимости

### Оптимизация инференса

- Используйте ONNX Runtime с оптимизациями
- Минимизируйте количество фич
- Кэшируйте результаты инференса

## 9. Безопасность

### Защита моделей

- Храните модели в защищенной директории: `/etc/smoothtask/models/`
- Установите правильные права доступа: `chmod 644 /etc/smoothtask/models/*.onnx`
- Используйте проверку целостности моделей

### Логирование

- Включите логирование для мониторинга работы ранкера
- Настройте ротацию логов для предотвращения переполнения диска

## 10. Будущие Улучшения

### Планируемые функции

- Автоматическое обновление моделей
- A/B тестирование разных моделей
- Мониторинг производительности моделей
- Интеграция с системами мониторинга

### Экспериментальные возможности

- Использование других алгоритмов (XGBoost, LightGBM)
- Онлайн-обучение
- Адаптивные пороги

## Ссылки

- [CatBoost документация](https://catboost.readthedocs.io/)
- [ONNX Runtime документация](https://onnxruntime.ai/)
- [SmoothTask API документация](docs/API.md)
- [SmoothTask архитектура](docs/ARCHITECTURE.md)

## Приложение: Формат ONNX Модели

### Входные данные

- `snapshot_id`: Идентификатор снапшота
- `cpu_usage`: Использование CPU
- `memory_usage`: Использование памяти
- `io_usage`: Использование диска
- `is_focused`: Флаг фокусного окна
- `has_audio`: Флаг аудио активности
- `process_type`: Тип процесса (категориальный)

### Выходные данные

- `score`: Оценка приоритета (0.0 - 1.0)
- `priority_class`: Класс приоритета (категориальный)

### Преобразование типов

- Числовые фичи: float32
- Категориальные фичи: int64 (индексы)
- Выходные данные: float32
