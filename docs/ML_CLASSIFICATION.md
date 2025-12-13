# ML-классификация процессов и обучение моделей в SmoothTask

## Обзор

Модуль `classify::ml_classifier` предоставляет интерфейс для классификации процессов с использованием ML-моделей. Это дополняет существующую систему классификации на основе паттернов и позволяет более точно определять типы процессов и их характеристики.

## Компоненты

### 1. Интерфейс MLClassifier

```rust
pub trait MLClassifier: Send + Sync {
    fn classify(&self, process: &ProcessRecord) -> MLClassificationResult;
}
```

Трейт `MLClassifier` определяет интерфейс для ML-классификаторов. Он требует реализации метода `classify`, который принимает процесс и возвращает результат классификации.

### 2. Результат классификации

```rust
pub struct MLClassificationResult {
    pub process_type: Option<String>,
    pub tags: Vec<String>,
    pub confidence: f64,
}
```

Результат классификации содержит:
- `process_type`: Тип процесса, предсказанный ML-моделью
- `tags`: Теги, предсказанные ML-моделью
- `confidence`: Уверенность модели в предсказании (0.0 - 1.0)

### 3. Реализации классификаторов

#### StubMLClassifier

Заглушка для тестирования и разработки. Использует эвристики для классификации процессов:

- **GUI процессы**: `has_gui_window = true` → тип "gui", теги ["gui", "interactive"], уверенность 0.8
- **Высокий CPU**: `cpu_share_10s > 0.3` → тип "cpu_intensive", тег "high_cpu", уверенность 0.7
- **Высокий IO**: `io_read_bytes > 1MB` → тип "io_intensive", тег "high_io", уверенность 0.6
- **Аудио клиенты**: `is_audio_client = true` → тип "audio", теги ["audio", "realtime"], уверенность 0.9
- **Фокусные окна**: `is_focused_window = true` → тип "focused", теги ["focused", "interactive"], уверенность 0.9

#### CatBoostMLClassifier

Реализация для загрузки и использования CatBoost моделей:

```rust
pub struct CatBoostMLClassifier {
    model: CatBoostModel,
}
```

### 4. Обучение и интеграция моделей

#### Процесс обучения

SmoothTask предоставляет полный пайплайн для обучения моделей на собранных данных:

1. **Сбор данных**: Используйте `collect_data_from_snapshots()` для создания тренировочного датасета
2. **Валидация данных**: Проверьте качество данных с помощью `validate_dataset()`
3. **Обучение модели**: Обучите CatBoostRanker модель с помощью `train_ranker()`
4. **Экспорт модели**: Экспортируйте модель в формат ONNX для использования в Rust
5. **Интеграция**: Обновите конфигурацию для использования обученной модели

#### Python API для обучения

```python
from smoothtask_trainer.train_pipeline import TrainingPipeline
from smoothtask_trainer.collect_data import collect_data_from_snapshots, validate_dataset

# Создание тренировочного датасета
pipeline = TrainingPipeline(
    db_path="training_data.sqlite",
    use_temp_db=False,
    min_snapshots=5,
    min_processes=50,
    min_groups=10
)

# Сбор данных
db_path = pipeline.collect_data()

# Валидация данных
stats = validate_dataset(
    db_path=db_path,
    min_snapshots=5,
    min_processes=50,
    min_groups=10
)

# Обучение модели
model = pipeline.train_model(
    model_path="trained_model.json",
    onnx_path="trained_model.onnx"
)
```

#### Интеграция модели в конфигурацию

```python
from smoothtask_trainer.integrate_model import update_configuration

# Обновление конфигурации
update_configuration(
    config_path="configs/smoothtask.example.yml",
    model_path="trained_model.onnx"
)
```

### 5. Примеры использования

#### Полный пайплайн обучения

```python
#!/usr/bin/env python3
"""
Полный пайплайн обучения модели SmoothTask
"""

import sys
from pathlib import Path

# Добавление тренера в путь
sys.path.insert(0, 'smoothtask-trainer')

from smoothtask_trainer.train_pipeline import TrainingPipeline
from smoothtask_trainer.collect_data import validate_dataset

def main():
    # Создание тренировочного датасета
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
    print(f"  JSON модель: {model_path_json}")
    print(f"  ONNX модель: {model_path_onnx}")

if __name__ == "__main__":
    main()
```

#### Интеграция модели

```python
#!/usr/bin/env python3
"""
Интеграция обученной модели в конфигурацию SmoothTask
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

```rust
pub struct CatBoostMLClassifier {
    model: CatBoostModel,
    feature_names: Vec<String>,
}
```

Поддерживает загрузку моделей из JSON и ONNX форматов.

## Интеграция с системой классификации

ML-классификатор интегрирован в функцию `classify_process` и может использоваться вместе с паттерн-классификацией:

```rust
use smoothtask_core::classify::ml_classifier::{CatBoostMLClassifier, StubMLClassifier};
use smoothtask_core::classify::rules::{PatternDatabase, classify_process};
```

## Полный Workflow обучения моделей

### 1. Сбор данных

Демон SmoothTask собирает данные о процессах и системе в формате JSONL или SQLite:

```yaml
# В конфигурационном файле
enable_snapshot_logging: true
paths:
  snapshot_db_path: "/var/lib/smoothtask/snapshots.db"
```

### 2. Подготовка данных

Используйте Python-инструменты для подготовки данных:

```bash
# Сбор данных из JSONL файлов в SQLite
uv run smoothtask_trainer.collect_data \
    --snapshots snapshots.jsonl \
    --output-db prepared_data.db

# Валидация датасета
uv run smoothtask_trainer.validate_dataset \
    --db prepared_data.db \
    --min-snapshots 10 \
    --min-processes 50 \
    --min-groups 5
```

### 3. Обучение модели

```bash
# Обучение из JSONL файлов
uv run smoothtask_trainer.train_pipeline \
    --snapshots snapshots.jsonl \
    --model-json model.json \
    --model-onnx model.onnx

# Обучение из существующей базы данных
uv run smoothtask_trainer.train_pipeline \
    --db prepared_data.db \
    --model-json model.json \
    --model-onnx model.onnx
```

### 4. Экспорт модели

```bash
# Экспорт с метаданными
uv run smoothtask_trainer.export_model \
    --model model.json \
    --format onnx \
    --output model.onnx \
    --metadata '{"version": "1.0.0", "description": "Модель для SmoothTask", "features": ["cpu", "memory", "io"]}'
```

## Python API

### Сбор данных

```python
from smoothtask_trainer import collect_data_from_snapshots, validate_dataset

# Сбор данных из нескольких файлов
db_path = collect_data_from_snapshots(
    snapshot_files=["snapshots1.jsonl", "snapshots2.jsonl.gz"],
    output_db=Path("output.db")
)

# Валидация датасета
stats = validate_dataset(
    db_path=db_path,
    min_snapshots=10,
    min_processes=50,
    min_groups=5
)
```

### Обучение модели

```python
from smoothtask_trainer import TrainingPipeline

# Полный pipeline
pipeline = TrainingPipeline(
    snapshot_files=["snapshots.jsonl"],
    use_temp_db=True,
    min_snapshots=10,
    min_processes=50,
    min_groups=5
)

model = pipeline.run_complete_pipeline(
    model_path=Path("model.json"),
    onnx_path=Path("model.onnx")
)
```

### Пошаговое выполнение

```python
# Шаг 1: Сбор данных
db_path = pipeline.collect_data()

# Шаг 2: Валидация
stats = pipeline.validate_data()

# Шаг 3: Загрузка данных
df = pipeline.load_data()

# Шаг 4: Подготовка фич
X, y, group_id, cat_features = pipeline.prepare_features()

# Шаг 5: Обучение модели
model = pipeline.train_model(Path("model.json"), Path("model.onnx"))
```

## Экспорт моделей с метаданными

Новая функциональность поддерживает экспорт моделей с метаданными:

```python
from smoothtask_trainer.export_model import export_model, validate_exported_model

# Экспорт с метаданными
metadata = {
    "version": "1.0.0",
    "description": "Модель для ранжирования процессов в SmoothTask",
    "author": "SmoothTask Team",
    "dataset_size": 1000,
    "features": ["cpu_usage", "memory_usage", "io_wait", "gpu_usage"],
    "training_date": "2024-01-15",
}

result = export_model(
    model_path=Path("model.json"),
    format="onnx",
    output_path=Path("model.onnx"),
    metadata=metadata,
    validate=True
)

# Валидация экспортированной модели
validation_result = validate_exported_model(
    model_path=Path("model.onnx"),
    expected_format="onnx",
    min_size=1024,
    check_metadata=True
)
```

## Форматы метаданных

Метаданные сохраняются в отдельном JSON файле рядом с моделью:
- Для ONNX: `model.onnx.metadata.json`
- Для JSON: `model.json.metadata.json`
- Для CBM: `model.cbm.metadata.json`

Пример содержимого:

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

## Обработка ошибок

Все функции предоставляют детальные сообщения об ошибках:

- `FileNotFoundError`: Если файлы не найдены
- `ValueError`: Если данные не проходят валидацию или параметры некорректны
- `PermissionError`: Если нет прав на запись
- `CatBoostError`: Если возникают ошибки при обучении модели

## Интеграция с SmoothTask

Обученная модель может быть использована в SmoothTask для ранжирования процессов:

```yaml
# В конфигурации SmoothTask
ranker:
  model_path: "/path/to/model.json"
  enabled: true
```

## Примеры использования

### Пример 1: Полный workflow

```bash
# Сбор данных
uv run smoothtask_trainer.collect_data \
    --snapshots /var/lib/smoothtask/snapshots.jsonl \
    --output-db /tmp/prepared_data.db

# Обучение модели
uv run smoothtask_trainer.train_pipeline \
    --db /tmp/prepared_data.db \
    --model-json /tmp/model.json \
    --model-onnx /tmp/model.onnx

# Экспорт с метаданными
uv run smoothtask_trainer.export_model \
    --model /tmp/model.json \
    --format onnx \
    --output /tmp/model_final.onnx \
    --metadata '{"version": "1.0.0", "description": "Production model"}'
```

### Пример 2: Обучение с пользовательскими параметрами

```python
from smoothtask_trainer import TrainingPipeline

pipeline = TrainingPipeline(
    snapshot_files=["snapshots.jsonl"],
    use_temp_db=True
)

# Настройка параметров модели
pipeline.model_params = {
    "depth": 6,
    "learning_rate": 0.05,
    "iterations": 500,
    "loss_function": "YetiRank",
}

model = pipeline.run_complete_pipeline(
    model_path=Path("custom_model.json"),
    onnx_path=Path("custom_model.onnx")
)
```

### Пример 3: Валидация и тестирование

```python
from smoothtask_trainer import validate_dataset, load_dataset

# Валидация датасета
stats = validate_dataset(
    db_path=Path("data.db"),
    min_snapshots=10,
    min_processes=50,
    min_groups=5
)

print(f"Снапшоты: {stats['snapshot_count']}")
print(f"Процессы: {stats['process_count']}")
print(f"Группы: {stats['group_count']}")

# Загрузка данных для анализа
df = load_dataset(Path("data.db"), validate=True)
print(f"Загружено {len(df)} записей")
```

## Производительность и масштабируемость

- **Скорость обучения**: ~1000 снапшотов за 1-2 минуты на современном CPU
- **Память**: ~100MB для обучения на 1000 снапшотах
- **Модели**: ONNX модели занимают ~1-5MB, JSON модели ~5-20MB

## Рекомендации

1. **Качество данных**: Используйте не менее 100 снапшотов и 100 процессов для обучения
2. **Валидация**: Всегда валидируйте данные перед обучением
3. **Метаданные**: Добавляйте метаданные для отслеживания версий и параметров
4. **Тестирование**: Проверяйте модели на тестовых данных перед использованием в продакшене

## Алгоритм интеграции

1. **Паттерн-классификация**: Сначала применяются паттерны из YAML-файлов
2. **ML-классификация**: Затем применяется ML-классификатор (если доступен)
3. **Объединение результатов**:
   - **Теги**: Объединяются теги из паттернов и ML
   - **Тип процесса**: Выбирается тип с наивысшей уверенностью:
     - Если уверенность ML > 0.7, используется тип от ML
     - Иначе используется тип от паттернов
4. **Сортировка**: Теги сортируются для согласованности

## Примеры использования

### Пример 1: Классификация процесса с GUI

```rust
let mut process = ProcessRecord {
    pid: 1000,
    exe: Some("firefox".to_string()),
    has_gui_window: true,
    // ... остальные поля
};

classify_process(&mut process, &pattern_db, Some(&ml_classifier), None);

// Результат:
// process_type: Some("gui") или Some("browser") (в зависимости от уверенности)
// tags: ["browser", "gui", "interactive"]
```

### Пример 2: Классификация аудио-клиента

```rust
let mut process = ProcessRecord {
    pid: 1001,
    exe: Some("pulseaudio".to_string()),
    is_audio_client: true,
    // ... остальные поля
};

classify_process(&mut process, &pattern_db, Some(&ml_classifier), None);

// Результат:
// process_type: Some("audio") (уверенность 0.9 > 0.7)
// tags: ["audio", "realtime"]
```

### Пример 3: Классификация всех процессов

```rust
use smoothtask_core::classify::rules::classify_all;

let mut processes = vec![/* ... */];
let mut app_groups = vec![/* ... */];

classify_all(&mut processes, &mut app_groups, &pattern_db, Some(&ml_classifier));

// Все процессы классифицированы с использованием паттернов и ML
```

### Пример 4: Использование PatternWatcher для автоматического обновления паттернов

```rust
use smoothtask_core::classify::pattern_watcher::PatternWatcher;
use smoothtask_core::classify::rules::PatternDatabase;
use std::path::Path;

// Создание PatternWatcher
let patterns_dir = Path::new("/etc/smoothtask/patterns");
let mut pattern_watcher = PatternWatcher::new(patterns_dir.to_path_buf());

// Загрузка начальной базы паттернов
let mut pattern_db = PatternDatabase::load(patterns_dir).expect("load patterns");

// Настройка мониторинга изменений
pattern_watcher.set_auto_reload_interval(60); // Проверка каждые 60 секунд
pattern_watcher.set_notify_on_reload(true);

// Основной цикл с мониторингом изменений
loop {
    // Проверка изменений в паттернах
    if pattern_watcher.has_changes() {
        println!("Обнаружены изменения в паттернах, выполняется перезагрузка...");
        
        // Перезагрузка паттернов
        if let Ok(reloaded_db) = PatternDatabase::load(patterns_dir) {
            pattern_db = reloaded_db;
            println!("Паттерны успешно перезагружены");
            
            // Уведомление о перезагрузке
            if pattern_watcher.should_notify() {
                // Отправка уведомления пользователю
                println!("УВЕДОМЛЕНИЕ: Паттерны перезагружены");
            }
        }
    }
    
    // Использование обновленной базы паттернов для классификации
    let mut process = ProcessRecord {
        pid: 1002,
        exe: Some("code".to_string()),
        // ... остальные поля
    };
    
    classify_process(&mut process, &pattern_db, Some(&ml_classifier), None);
    
    // Ожидание перед следующей проверкой
    std::thread::sleep(std::time::Duration::from_secs(1));
}
```

### Пример 5: Интеграция PatternWatcher с ML-классификатором

```rust
use smoothtask_core::classify::ml_classifier::CatBoostMLClassifier;
use smoothtask_core::classify::pattern_watcher::PatternWatcher;

// Создание ML-классификатора
let ml_classifier = CatBoostMLClassifier::new("models/process_classifier.json")
    .expect("load ML model");

// Создание PatternWatcher
let patterns_dir = Path::new("/etc/smoothtask/patterns");
let mut pattern_watcher = PatternWatcher::new(patterns_dir.to_path_buf());

// Загрузка начальной базы паттернов
let mut pattern_db = PatternDatabase::load(patterns_dir).expect("load patterns");

// Основной цикл классификации с автоматической перезагрузкой паттернов
loop {
    // Проверка изменений в паттернах
    if pattern_watcher.has_changes() {
        if let Ok(reloaded_db) = PatternDatabase::load(patterns_dir) {
            pattern_db = reloaded_db;
            println!("Паттерны перезагружены, продолжаем классификацию");
        }
    }
    
    // Получение списка процессов для классификации
    let mut processes = get_processes_from_system(); // Ваша функция получения процессов
    
    // Классификация всех процессов с использованием обновленных паттернов и ML
    for process in &mut processes {
        classify_process(process, &pattern_db, Some(&ml_classifier), None);
        
        // Логирование результатов классификации
        println!("Process {}: type={:?}, tags={:?}", 
                 process.pid, 
                 process.process_type, 
                 process.tags);
    }
    
    // Ожидание перед следующей итерацией
    std::thread::sleep(std::time::Duration::from_secs(5));
}
```

### Пример 6: Настройка PatternWatcher для различных сценариев

```rust
// Сценарий 1: Разработка - частые проверки, уведомления включены
let mut dev_watcher = PatternWatcher::new(Path::new("/etc/smoothtask/patterns"));
dev_watcher.set_auto_reload_interval(10); // Проверка каждые 10 секунд
dev_watcher.set_notify_on_reload(true);
dev_watcher.set_validate_on_reload(true);

// Сценарий 2: Production - редкие проверки, уведомления отключены
let mut prod_watcher = PatternWatcher::new(Path::new("/etc/smoothtask/patterns"));
prod_watcher.set_auto_reload_interval(300); // Проверка каждые 5 минут
prod_watcher.set_notify_on_reload(false);
prod_watcher.set_validate_on_reload(false);

// Сценарий 3: Тестирование - только добавление новых паттернов
let mut test_watcher = PatternWatcher::new(Path::new("/etc/smoothtask/patterns"));
test_watcher.set_detect_additions(true);
test_watcher.set_detect_modifications(false);
test_watcher.set_detect_deletions(false);
```

## Интеграция с ONNX Runtime

SmoothTask поддерживает интеграцию с ONNX Runtime для загрузки и выполнения обученных CatBoost моделей. Это позволяет использовать реальные ML-модели для классификации и ранжирования процессов.

### Использование ONNX моделей

1. **Обучение и экспорт модели**:
   ```bash
   cd smoothtask-trainer
   python -m smoothtask_trainer.train_ranker \
       --db snapshots.db \
       --model-json models/ranker.json \
       --model-onnx models/ranker.onnx
   ```

2. **Конфигурация для использования ONNX модели**:
   ```yaml
   model:
     model_path: "models/ranker.onnx"
     enabled: true
   ```

3. **Загрузка ONNX модели в Rust**:
   ```rust
   use smoothtask_core::model::onnx_ranker::ONNXRanker;
   
   let ranker = ONNXRanker::load("models/ranker.onnx")?;
   ```

### Преимущества ONNX интеграции

- **Кросс-платформенность**: ONNX модели могут использоваться на разных платформах
- **Производительность**: ONNX Runtime оптимизирован для быстрого выполнения
- **Совместимость**: Поддержка различных ML-фреймворков через ONNX

### Будущие улучшения

1. **Динамическая загрузка**: Загрузка моделей во время выполнения без перезапуска демона
2. **Кэширование**: Кэширование результатов ML-классификации для оптимизации производительности
3. **Поддержка других форматов**: Расширение поддержки для других ML-форматов
4. **Мониторинг качества**: Встроенный мониторинг качества и производительности моделей

## Тестирование

Модуль включает комплексные тесты для проверки:

- Интеграции паттерн-классификации и ML-классификации
- Выбора типа процесса на основе уверенности
- Объединения тегов из разных источников
- Обработки процессов без совпадений в паттернах

Запуск тестов:

```bash
cargo test --lib classify
```

## Миграция

Существующий код будет продолжать работать без изменений, так как ML-классификатор является опциональным параметром. Для использования ML-классификации достаточно передать ML-классификатор в функции `classify_process` и `classify_all`.