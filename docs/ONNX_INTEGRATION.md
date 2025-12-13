# Интеграция ONNX моделей в SmoothTask

## Обзор

Этот документ описывает процесс интеграции ONNX моделей в SmoothTask для ранжирования групп приложений. ONNX (Open Neural Network Exchange) предоставляет стандартный формат для обмена ML-моделями между различными фреймворками.

## Компоненты ONNX интеграции

### 1. Архитектура компонентов

```mermaid
graph TD
    A[SQLite Snapshots] --> B[Python Trainer]
    B --> C[CatBoost Model]
    C --> D[ONNX Export]
    D --> E[ONNX Model File]
    E --> F[Rust ONNXRanker]
    F --> G[Policy Engine]
    G --> H[Priority Adjustments]
```

### 2. Ключевые модули

- **`smoothtask-trainer`**: Python-библиотека для обучения CatBoost моделей и экспорта в ONNX
- **`smoothtask-core::model::onnx_ranker`**: Rust-реализация ONNX ранкера
- **`smoothtask-core::policy::engine`**: Интеграция ONNX ранкера в движок политик

## Процесс обучения и экспорта

### 1. Подготовка данных

Перед обучением необходимо собрать данные о работе системы. SmoothTask предоставляет несколько способов сбора данных:

#### Способ 1: Сбор данных через демон

```bash
# Запустить демон SmoothTask для сбора снапшотов
cargo run --bin smoothtaskd -- --config configs/smoothtask.example.yml

# Просмотреть собранные данные
sqlite3 snapshots.db "SELECT COUNT(*) FROM snapshots;"
```

#### Способ 2: Использование скриптов сбора данных

```bash
# Собрать данные из существующих снапшотов
python3 collect_and_train.py

# Или использовать комплексный скрипт сбора данных
python3 complete_data_collection.py
```

### 2. Обучение модели

SmoothTask предоставляет несколько способов обучения моделей:

#### Способ 1: Использование TrainingPipeline

```python
from smoothtask_trainer.train_pipeline import TrainingPipeline

# Создание пайплайна
pipeline = TrainingPipeline(
    db_path="training_data.sqlite",
    use_temp_db=False,
    min_snapshots=5,
    min_processes=50,
    min_groups=10
)

# Сбор данных
db_path = pipeline.collect_data()

# Обучение модели
model = pipeline.train_model(
    model_path="trained_model.json",
    onnx_path="trained_model.onnx"
)
```

#### Способ 2: Использование командной строки

```bash
# Использовать скрипт обучения
python3 train_model.py

# Или использовать модуль напрямую
cd smoothtask-trainer
python -m smoothtask_trainer.train_ranker \
    --db training_data.sqlite \
    --model-json trained_model.json \
```

### 3. Интеграция модели

После обучения модели необходимо интегрировать её в конфигурацию SmoothTask:

```bash
# Использовать скрипт интеграции
python3 integrate_model.py

# Или вручную обновить конфигурацию
# Обновить configs/smoothtask.example.yml:
# - policy_mode: hybrid
# - model.enabled: true
# - model_path: "/path/to/trained_model.onnx"
```

### 4. Полный пайплайн

Для удобства SmoothTask предоставляет скрипты для полного пайплайна:

```bash
# Сбор данных и обучение
python3 collect_and_train.py

# Интеграция модели
python3 integrate_model.py

# Проверка результатов
ls -la trained_model.*
cat MODEL_README.md
```

### 5. Примеры использования

#### Пример 1: Обучение модели на существующих данных

```python
#!/usr/bin/env python3
"""
Обучение модели на существующих данных
"""

import sys
from pathlib import Path

sys.path.insert(0, 'smoothtask-trainer')

from smoothtask_trainer.train_pipeline import TrainingPipeline
from smoothtask_trainer.collect_data import validate_dataset

def main():
    # Создание тренировочного пайплайна
    pipeline = TrainingPipeline(
        db_path="training_data.sqlite",
        use_temp_db=False,
        min_snapshots=1,
        min_processes=10,
        min_groups=1
    )
    
    # Сбор данных
    db_path = pipeline.collect_data()
    print(f"Данные собраны: {db_path}")
    
    # Валидация данных
    stats = validate_dataset(
        db_path=db_path,
        min_snapshots=5,
        min_processes=50,
        min_groups=10
    )
    
    print(f"Статистика датасета:")
    print(f"  Снапшоты: {stats['snapshot_count']}")
    print(f"  Процессы: {stats['process_count']}")
    print(f"  Группы: {stats['group_count']}")
    
    # Обучение модели
    model_path_json = Path("trained_model.json")
    model_path_onnx = Path("trained_model.onnx")
    
    model = pipeline.train_model(
        model_path=model_path_json,
        onnx_path=model_path_onnx
    )
    
    print(f"Модель обучена!")
    print(f"  JSON модель: {model_path_json} ({model_path_json.stat().st_size} bytes)")
    print(f"  ONNX модель: {model_path_onnx} ({model_path_onnx.stat().st_size} bytes)")

if __name__ == "__main__":
    main()
```

#### Пример 2: Интеграция модели в конфигурацию

```python
#!/usr/bin/env python3
"""
Интеграция обученной модели в конфигурацию
"""

import sys
from pathlib import Path
import yaml

def update_configuration():
    # Проверка наличия файлов модели
    model_json = Path("trained_model.json")
    model_onnx = Path("trained_model.onnx")
    
    if not model_json.exists():
        print(f"Ошибка: JSON модель не найдена: {model_json}")
        return 1
    
    if not model_onnx.exists():
        print(f"Ошибка: ONNX модель не найдена: {model_onnx}")
        return 1
    
    # Чтение конфигурации
    config_path = Path("configs/smoothtask.example.yml")
    
    with open(config_path, 'r') as f:
        config_content = f.read()
    
    # Обновление конфигурации
    updated_config = config_content.replace(
        "policy_mode: rules-only",
        "policy_mode: hybrid"
    )
    
    updated_config = updated_config.replace(
        "model:\n  enabled: false",
        "model:\n  enabled: true"
    )
    
    updated_config = updated_config.replace(
        "model_path: \"models/ranker.onnx\"",
        f"model_path: \"{model_onnx.absolute()}\""
    )
    
    # Запись обновленной конфигурации
    with open(config_path, 'w') as f:
        f.write(updated_config)
    
    print(f"Конфигурация обновлена: {config_path}")
    print(f"  policy_mode: rules-only -> hybrid")
    print(f"  model.enabled: false -> true")
    print(f"  model_path: models/ranker.onnx -> {model_onnx.absolute()}")

if __name__ == "__main__":
    update_configuration()
```

### 6. Лучшие практики

#### Сбор данных

1. **Собирайте данные в разных состояниях системы**: idle, load, interactive
2. **Обеспечьте разнообразие процессов**: background, interactive, latency-critical
3. **Используйте достаточное количество снапшотов**: минимум 10-15 для хорошего качества
4. **Валидируйте данные**: проверяйте качество данных перед обучением

#### Обучение модели

1. **Начинайте с простых параметров**: depth=6, learning_rate=0.1, iterations=500
2. **Используйте YetiRank**: оптимизирован для задач ранжирования
3. **Мониторьте качество**: проверяйте метрики качества на валидационной выборке
4. **Экспериментируйте**: пробуйте разные параметры для улучшения качества

#### Интеграция модели

1. **Проверяйте совместимость**: убедитесь, что модель совместима с текущей версией SmoothTask
2. **Начинайте с hybrid режима**: используйте `policy_mode: hybrid` для постепенного внедрения
3. **Мониторьте производительность**: следите за влиянием модели на систему
4. **Обновляйте регулярно**: переобучайте модель на новых данных

### 7. Устранение неполадок

#### Проблемы с обучением

- **Недостаточно данных**: Увеличьте количество снапшотов и процессов
- **Плохое качество**: Проверьте разнообразие данных и параметры модели
- **Ошибки экспорта**: Убедитесь, что все зависимости установлены

#### Проблемы с интеграцией

- **Модель не загружается**: Проверьте путь к файлу и права доступа
- **Плохая производительность**: Проверьте совместимость модели с текущей версией
- **Ошибки ранжирования**: Проверьте качество обученной модели

```bash
    --model-onnx models/ranker.onnx
```

**Параметры обучения:**
- `loss_function`: YetiRank (оптимизирован для ранжирования)
- `depth`: 6 (глубина деревьев)
- `learning_rate`: 0.1
- `iterations`: 500

### 3. Экспорт модели в ONNX

Модель автоматически экспортируется в ONNX формат при обучении. Также можно экспортировать существующую модель:

```bash
python -m smoothtask_trainer.export_model \
    --model-path models/ranker.json \
    --format onnx \
    --output-path models/ranker.onnx
```

### 4. Экспорт с метаданными

Новая функциональность поддерживает экспорт моделей с метаданными:

```bash
python -m smoothtask_trainer.export_model \
    --model-path models/ranker.json \
    --format onnx \
    --output-path models/ranker.onnx \
    --metadata '{"version": "1.0.0", "description": "Модель для SmoothTask", "features": ["cpu", "memory", "io"]}' \
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

## Конфигурация ONNX ранкера

### 1. Базовая конфигурация

В файле конфигурации `smoothtask.yml`:

```yaml
model:
  # Путь к ONNX модели
  model_path: "models/ranker.onnx"
  
  # Включение ONNX ранкера
  enabled: true

policy:
  # Режим работы: hybrid (использует ONNX + правила)
  mode: "hybrid"
```

### 2. Примеры конфигураций

#### Пример 1: Только ONNX ранкер

```yaml
model:
  model_path: "models/ranker.onnx"
  enabled: true

policy:
  mode: "ml_only"  # Использует только ML-модель
```

#### Пример 2: Гибридный режим (ONNX + правила)

```yaml
model:
  model_path: "models/ranker.onnx"
  enabled: true

policy:
  mode: "hybrid"  # Использует ONNX + правила
  rule_weights:
    ml: 0.7
    rules: 0.3
```

#### Пример 3: Отключение ONNX

```yaml
model:
  enabled: false

policy:
  mode: "rules_only"  # Использует только правила
```

## Интеграция в Rust

### 1. Загрузка ONNX модели

```rust
use smoothtask_core::model::onnx_ranker::ONNXRanker;

// Загрузка модели
let ranker = ONNXRanker::load("models/ranker.onnx")
    .expect("Failed to load ONNX model");
```

### 2. Использование ONNX ранкера

```rust
use smoothtask_core::model::ranker::Ranker;
use smoothtask_core::logging::snapshots::{Snapshot, AppGroupRecord};

// Создание снапшота и групп приложений
let snapshot: Snapshot = /* ... */;
let app_groups: Vec<AppGroupRecord> = /* ... */;

// Ранжирование групп
let results = ranker.rank(&app_groups, &snapshot);

// Обработка результатов
for (app_group_id, result) in &results {
    println!("Group {}: score={:.2}, rank={}", 
             app_group_id, result.score, result.rank);
}
```

### 3. Интеграция в Policy Engine

```rust
use smoothtask_core::policy::engine::PolicyEngine;
use smoothtask_core::config::config_struct::Config;

// Загрузка конфигурации
let config = Config::load("smoothtask.yml")?;

// Создание движка политик с ONNX ранкером
let mut engine = PolicyEngine::new(&config)?;

// Вычисление приоритетов
let priorities = engine.calculate_priorities(&snapshot, &app_groups)?;
```

## Обработка ошибок

### 1. Ошибки загрузки модели

```rust
match ONNXRanker::load("models/ranker.onnx") {
    Ok(ranker) => {
        // Успешная загрузка
    }
    Err(e) => {
        eprintln!("Failed to load ONNX model: {}", e);
        // Fallback на заглушку
        let ranker = StubRanker::new();
    }
}
```

### 2. Ошибки выполнения

```rust
match ranker.rank(&app_groups, &snapshot) {
    Ok(results) => {
        // Успешное ранжирование
    }
    Err(e) => {
        eprintln!("Failed to rank app groups: {}", e);
        // Fallback на правила
        let results = rule_based_ranking(&app_groups);
    }
}
```

## Тестирование ONNX интеграции

### 1. Unit тесты

```bash
# Запуск unit тестов для ONNX ранкера
cargo test --lib model::onnx_ranker
```

### 2. Интеграционные тесты

```bash
# Запуск интеграционных тестов
cargo test --test actuator_integration_test
```

### 3. Тестирование с мок-моделью

```rust
#[test]
fn test_onnx_ranker_with_mock_model() {
    // Создание временной ONNX модели
    let temp_file = tempfile::NamedTempFile::new();
    let model_path = temp_file.path();
    
    // Создание заглушки ONNX модели
    std::fs::write(model_path, "dummy_onnx_content").unwrap();
    
    // Тестирование загрузки
    let result = ONNXRanker::load(model_path);
    assert!(result.is_err()); // Ожидаем ошибку для невалидной модели
}
```

## Troubleshooting

### 1. Ошибки загрузки ONNX модели

**Проблема:** `Failed to load ONNX model: File not found`

**Решение:**
- Убедитесь, что путь к модели указан правильно
- Проверьте, что файл существует: `ls -la models/ranker.onnx`
- Используйте абсолютные пути или правильные относительные пути

**Проблема:** `Invalid ONNX model format`

**Решение:**
- Убедитесь, что модель экспортирована правильно
- Проверьте версию ONNX Runtime: `cargo tree | grep ort`
- Переэкспортируйте модель: `python -m smoothtask_trainer.export_model ...`

### 2. Ошибки выполнения модели

**Проблема:** `Shape mismatch in ONNX model`

**Решение:**
- Проверьте, что количество фич совпадает с ожидаемым
- Обновите модель с правильными данными
- Проверьте логи обучения на наличие предупреждений

**Проблема:** `ONNX Runtime error during inference`

**Решение:**
- Проверьте совместимость версии ONNX Runtime
- Обновите зависимости: `cargo update -p ort`
- Проверьте системные зависимости (CUDA, если используется)

### 3. Производительность

**Проблема:** Медленное выполнение ONNX модели

**Решение:**
- Уменьшите размер модели (уменьшите depth, iterations)
- Используйте квантизацию модели
- Проверьте нагрузку на CPU/GPU

## Лучшие практики

### 1. Управление моделями

- Храните модели в системе контроля версий
- Используйте семантическое версионирование для моделей
- Документируйте изменения в моделях

### 2. Мониторинг

- Логируйте ошибки загрузки и выполнения моделей
- Мониторьте производительность модели
- Отслеживайте метрики качества ранжирования

### 3. Обновление моделей

- Тестируйте новые модели перед развертыванием
- Используйте A/B тестирование для сравнения моделей
- Плавно переходите на новые модели

## Примеры использования

### 1. Полный цикл обучения и развертывания

```bash
# 1. Сбор данных
cargo run --bin smoothtaskd -- --config configs/smoothtask.example.yml

# 2. Обучение модели
cd smoothtask-trainer
python -m smoothtask_trainer.train_ranker \
    --db snapshots.db \
    --model-json models/ranker.json \
    --model-onnx models/ranker.onnx

# 3. Развертывание модели
# Обновите конфигурацию smoothtask.yml
# Перезапустите демон
cargo run --bin smoothtaskd -- --config configs/smoothtask.example.yml
```

### 2. Обновление существующей модели

```bash
# 1. Экспорт существующей модели в ONNX
python -m smoothtask_trainer.export_model \
    --model-path models/ranker.json \
    --format onnx \
    --output-path models/ranker_v2.onnx

# 2. Тестирование новой модели
# Обновите конфигурацию для использования новой модели
# Проверьте логи на наличие ошибок

# 3. Развертывание
# Замените старую модель на новую
mv models/ranker_v2.onnx models/ranker.onnx
```

### 3. Отладка проблем с моделью

```bash
# 1. Проверка модели
python -c "
import onnx
model = onnx.load('models/ranker.onnx')
print('Model inputs:', [input.name for input in model.graph.input])
print('Model outputs:', [output.name for output in model.graph.output])
"

# 2. Проверка совместимости
cargo test --lib model::onnx_ranker::tests

# 3. Проверка интеграции
cargo test --lib policy::engine::tests
```

## Ссылки

- [ONNX Official Documentation](https://onnx.ai/)
- [ONNX Runtime Documentation](https://onnxruntime.ai/)
- [CatBoost ONNX Export](https://catboost.readthedocs.io/en/latest/onnx.html)
- [SmoothTask ML Classification](ML_CLASSIFICATION.md)
- [SmoothTask Architecture](ARCHITECTURE.md)