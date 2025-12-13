# SmoothTask Trainer

Python-инструменты для обучения CatBoostRanker и тюнинга политики SmoothTask.

## Установка

```bash
uv pip install -e .
```

## Использование

### Комплексный Workflow для сбора данных и обучения

Новый модуль `train_pipeline` предоставляет комплексный workflow для сбора данных из снапшотов, валидации датасета и обучения модели.

#### Основные возможности

- **Сбор данных**: Преобразование JSONL файлов снапшотов в SQLite базу данных
- **Валидация**: Проверка качества и достаточности данных
- **Обучение**: Полный pipeline обучения CatBoostRanker
- **Экспорт**: Сохранение моделей в различных форматах (JSON, ONNX)

#### Базовый синтаксис

```bash
# Обучение из файлов снапшотов
uv run smoothtask_trainer.train_pipeline \
    --snapshots snapshots.jsonl \
    --model-json model.json \
    --model-onnx model.onnx

# Обучение из существующей базы данных
uv run smoothtask_trainer.train_pipeline \
    --db snapshots.db \
    --model-json model.json
```

#### Аргументы

- `--snapshots`: Путь к файлу(ам) снапшотов (JSONL или GZ)
- `--db`: Путь к существующей базе данных (альтернатива --snapshots)
- `--model-json`: Путь для сохранения модели в формате JSON (обязательный)
- `--model-onnx`: Опциональный путь для сохранения модели в формате ONNX
- `--use-temp-db`: Использовать временную базу данных
- `--min-snapshots`: Минимальное количество снапшотов для валидации (по умолчанию: 1)
- `--min-processes`: Минимальное количество процессов для валидации (по умолчанию: 10)
- `--min-groups`: Минимальное количество групп для валидации (по умолчанию: 1)

#### Примеры использования

1. **Обучение из файлов снапшотов:**

```bash
uv run smoothtask_trainer.train_pipeline \
    --snapshots /var/lib/smoothtask/snapshots.jsonl \
    --model-json /path/to/model.json \
    --model-onnx /path/to/model.onnx
```

2. **Обучение из нескольких файлов снапшотов:**

```bash
uv run smoothtask_trainer.train_pipeline \
    --snapshots snapshots1.jsonl snapshots2.jsonl.gz \
    --model-json model.json
```

3. **Обучение из существующей базы данных:**

```bash
uv run smoothtask_trainer.train_pipeline \
    --db /var/lib/smoothtask/snapshots.sqlite \
    --model-json model.json
```

4. **Обучение с пользовательскими требованиями к валидации:**

```bash
uv run smoothtask_trainer.train_pipeline \
    --snapshots snapshots.jsonl \
    --model-json model.json \
    --min-snapshots 5 \
    --min-processes 50 \
    --min-groups 10
```

### Программный интерфейс

Модуль `train_pipeline` также предоставляет Python API для интеграции в ваши скрипты:

```python
from smoothtask_trainer import TrainingPipeline, train_from_snapshots, train_from_database

# Вариант 1: Использование класса TrainingPipeline
pipeline = TrainingPipeline(
    snapshot_files=["snapshots1.jsonl", "snapshots2.jsonl"],
    use_temp_db=True,
    min_snapshots=5,
    min_processes=50,
    min_groups=10
)

# Выполнение полного pipeline
model = pipeline.run_complete_pipeline(
    model_path=Path("model.json"),
    onnx_path=Path("model.onnx")
)

# Вариант 2: Удобные функции
train_from_snapshots(
    snapshot_files=["snapshots.jsonl"],
    model_path=Path("model.json"),
    onnx_path=Path("model.onnx")
)

train_from_database(
    db_path=Path("snapshots.db"),
    model_path=Path("model.json")
)
```

### Сбор данных

Модуль `collect_data` предоставляет функции для преобразования JSONL файлов снапшотов в SQLite базу данных:

```python
from smoothtask_trainer import collect_data_from_snapshots, validate_dataset, load_dataset

# Сбор данных из файлов снапшотов
db_path = collect_data_from_snapshots(
    snapshot_files=["snapshots1.jsonl", "snapshots2.jsonl.gz"],
    output_db=Path("output.db")
)

# Валидация датасета
stats = validate_dataset(
    db_path=db_path,
    min_snapshots=5,
    min_processes=50,
    min_groups=10
)

# Загрузка датасета для обучения
df = load_dataset(db_path, validate=True)
```

### Обучение ранкера

Команда `train_ranker` обучает CatBoostRanker на снапшотах процессов и сохраняет модель в различных форматах.

#### Базовый синтаксис

```bash
uv run smoothtask_trainer.train_ranker \
    --db /var/lib/smoothtask/snapshots.sqlite \
    --model-json model.json \
    --model-onnx model.onnx
```

#### Аргументы

- `--db`: Путь к SQLite базе данных со снапшотами процессов (обязательный)
- `--model-json`: Путь для сохранения модели в формате JSON (обязательный)
- `--model-onnx`: Опциональный путь для сохранения модели в формате ONNX

#### Формат входных данных

База данных должна содержать таблицы:
- `snapshots`: основные снапшоты системы
- `processes`: метрики процессов
- `app_groups`: группировка процессов

Каждая запись должна содержать:
- `timestamp`: временная метка снапшота
- `pid`: идентификатор процесса
- `app_group_id`: идентификатор группы приложений
- `teacher_score`: целевая метка для обучения (если доступна)

#### Формат выходных моделей

- **JSON**: Основной формат модели CatBoost, подходит для загрузки в Rust
- **ONNX**: Формат для совместимости с другими системами, требует CatBoost с поддержкой ONNX

#### Примеры использования

1. **Обучение с сохранением только JSON модели:**

```bash
uv run smoothtask_trainer.train_ranker \
    --db /var/lib/smoothtask/snapshots.sqlite \
    --model-json /path/to/model.json
```

2. **Обучение с сохранением JSON и ONNX моделей:**

```bash
uv run smoothtask_trainer.train_ranker \
    --db /var/lib/smoothtask/snapshots.sqlite \
    --model-json /path/to/model.json \
    --model-onnx /path/to/model.onnx
```

3. **Обучение с использованием тестовых данных:**

```bash
# Создать тестовую базу данных
uv run python -c "
import sqlite3
import pandas as pd
from smoothtask_trainer.dataset import create_test_database

create_test_database('test_snapshots.sqlite')
"

# Обучить модель на тестовых данных
uv run smoothtask_trainer.train_ranker \
    --db test_snapshots.sqlite \
    --model-json test_model.json
```

#### Обработка ошибок

Команда проверяет:
- Существование базы данных
- Корректность путей (не директории)
- Достаточность данных для обучения
- Права доступа к файлам

#### Параметры модели

По умолчанию используются:
- Loss function: YetiRank
- Depth: 6
- Learning rate: 0.1
- Iterations: 500
- Random state: 42

#### Экспорт модели в другие форматы

Для экспорта обученной модели в другие форматы используйте команду `export_model`:

```bash
uv run smoothtask_trainer.export_model \
    --model model.json \
    --format onnx \
    --output model.onnx
```

Поддерживаемые форматы: `onnx`, `json`, `cbm`.

##### Расширенный экспорт с метаданными

Новая версия поддерживает экспорт с метаданными и валидацию:

```bash
uv run smoothtask_trainer.export_model \
    --model model.json \
    --format onnx \
    --output model.onnx \
    --metadata '{"version": "1.0.0", "description": "Модель для SmoothTask", "author": "My Team"}' \
    --validate
```

Параметры:
- `--model`: Путь к исходной модели (JSON или CBM)
- `--format`: Формат экспорта (onnx, json, cbm)
- `--output`: Путь для сохранения экспортированной модели
- `--metadata`: Опциональные метаданные в формате JSON (необязательно)
- `--validate`: Выполнять валидацию модели перед экспортом (по умолчанию: true)
- `--no-validate`: Отключить валидацию модели

##### Расширенный экспорт с метаданными

Новая версия поддерживает экспорт с метаданными и валидацию:

```bash
uv run smoothtask_trainer.export_model \
    --model model.json \
    --format onnx \
    --output model.onnx \
    --metadata '{"version": "1.0.0", "description": "Модель для SmoothTask", "author": "My Team"}' \
    --validate
```

Параметры:
- `--model`: Путь к исходной модели (JSON или CBM)
- `--format`: Формат экспорта (onnx, json, cbm)
- `--output`: Путь для сохранения экспортированной модели
- `--metadata`: Опциональные метаданные в формате JSON (необязательно)
- `--validate`: Выполнять валидацию модели перед экспортом (по умолчанию: true)
- `--no-validate`: Отключить валидацию модели

##### Примеры использования

1. **Экспорт с метаданными:**

```bash
uv run smoothtask_trainer.export_model \
    --model /path/to/model.json \
    --format onnx \
    --output /path/to/model.onnx \
    --metadata '{"version": "1.0.0", "dataset_size": 1000, "features": ["cpu", "memory", "io"]}'
```

2. **Экспорт без валидации (для ускорения):**

```bash
uv run smoothtask_trainer.export_model \
    --model model.json \
    --format cbm \
    --output model.cbm \
    --no-validate
```

3. **Экспорт во все форматы:**

```bash
# Экспорт в JSON
uv run smoothtask_trainer.export_model --model model.cbm --format json --output model.json

# Экспорт в ONNX
uv run smoothtask_trainer.export_model --model model.json --format onnx --output model.onnx

# Экспорт в CBM
uv run smoothtask_trainer.export_model --model model.json --format cbm --output model.cbm
```

##### Валидация экспортированных моделей

Для валидации экспортированных моделей используйте функцию `validate_exported_model`:

```python
from smoothtask_trainer.export_model import validate_exported_model

# Валидация модели с метаданными
validation_result = validate_exported_model(
    model_path=Path("model.onnx"),
    expected_format="onnx",
    min_size=1024,
    check_metadata=True
)

print(f"Модель валидна: {validation_result}")
```

##### Формат метаданных

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

##### Обработка ошибок

Функция экспорта предоставляет детальные сообщения об ошибках:

- `FileNotFoundError`: Если исходная модель не найдена
- `ValueError`: Если формат не поддерживается или параметры некорректны
- `PermissionError`: Если нет прав на запись в целевую директорию
- `ValueError`: Если модель не проходит валидацию

Все ошибки содержат практические рекомендации по устранению проблем.

#### Интеграция с SmoothTask

Обученная модель может быть использована в SmoothTask для ранжирования процессов:

```yaml
# В конфигурации SmoothTask
ranker:
  model_path: "/path/to/model.json"
  enabled: true
```

## Расширенные возможности

### Работа с сжатыми файлами

Модуль поддерживает работу с GZIP-сжатыми файлами снапшотов:

```bash
# Обучение из сжатого файла
uv run smoothtask_trainer.train_pipeline \
    --snapshots snapshots.jsonl.gz \
    --model-json model.json
```

### Валидация данных

Модуль предоставляет детальную валидацию данных:

```python
from smoothtask_trainer import validate_dataset

stats = validate_dataset(
    db_path=Path("snapshots.db"),
    min_snapshots=10,
    min_processes=100,
    min_groups=20
)

print(f"Снапшоты: {stats['snapshot_count']}")
print(f"Процессы: {stats['process_count']}")
print(f"Группы: {stats['group_count']}")
print(f"Уникальные процессы: {stats['unique_processes']}")
print(f"Уникальные группы: {stats['unique_groups']}")
```

### Пошаговое выполнение

Вы можете выполнять pipeline пошагово для более точного контроля:

```python
from smoothtask_trainer import TrainingPipeline

pipeline = TrainingPipeline(
    snapshot_files=["snapshots.jsonl"],
    use_temp_db=True
)

# Шаг 1: Сбор данных
db_path = pipeline.collect_data()

# Шаг 2: Валидация данных
stats = pipeline.validate_data()

# Шаг 3: Загрузка данных
df = pipeline.load_data()

# Шаг 4: Подготовка фич
X, y, group_id, cat_features = pipeline.prepare_features(use_categorical=False)

# Шаг 5: Обучение модели
model = pipeline.train_model(Path("model.json"), Path("model.onnx"))

# Шаг 6: Очистка
pipeline.cleanup()
```

## Обработка ошибок

Модуль предоставляет детальную обработку ошибок:

- **FileNotFoundError**: Если файлы снапшотов или база данных не найдены
- **ValueError**: Если данные не проходят валидацию или параметры некорректны
- **CatBoostError**: Если возникают ошибки при обучении модели

Все ошибки содержат детальные сообщения с практическими рекомендациями.

