# SmoothTask Trainer

Python-инструменты для обучения CatBoostRanker и тюнинга политики SmoothTask.

## Установка

```bash
uv pip install -e .
```

## Использование

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

#### Интеграция с SmoothTask

Обученная модель может быть использована в SmoothTask для ранжирования процессов:

```yaml
# В конфигурации SmoothTask
ranker:
  model_path: "/path/to/model.json"
  enabled: true
```

