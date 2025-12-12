# ML-классификация процессов

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

### 3. Заглушка для тестирования

`StubMLClassifier` предоставляет простую реализацию для тестирования и разработки. Он использует эвристики для классификации процессов:

- **GUI процессы**: `has_gui_window = true` → тип "gui", теги ["gui", "interactive"], уверенность 0.8
- **Высокий CPU**: `cpu_share_10s > 0.3` → тип "cpu_intensive", тег "high_cpu", уверенность 0.7
- **Высокий IO**: `io_read_bytes > 1MB` → тип "io_intensive", тег "high_io", уверенность 0.6
- **Аудио клиенты**: `is_audio_client = true` → тип "audio", теги ["audio", "realtime"], уверенность 0.9
- **Фокусные окна**: `is_focused_window = true` → тип "focused", теги ["focused", "interactive"], уверенность 0.9

## Интеграция с системой классификации

ML-классификатор интегрирован в функцию `classify_process` и может использоваться вместе с паттерн-классификацией:

```rust
use smoothtask_core::classify::ml_classifier::StubMLClassifier;
use smoothtask_core::classify::rules::{PatternDatabase, classify_process};
use smoothtask_core::logging::snapshots::ProcessRecord;

// Загрузка паттернов
let pattern_db = PatternDatabase::load("configs/patterns").expect("load patterns");

// Создание ML-классификатора
let ml_classifier = StubMLClassifier::new();

// Создание процесса
let mut process = ProcessRecord {
    pid: 1000,
    exe: Some("firefox".to_string()),
    has_gui_window: true,
    cpu_share_10s: Some(0.5),
    // ... остальные поля
    process_type: None,
    tags: Vec::new(),
};

// Классификация с использованием паттернов и ML
classify_process(&mut process, &pattern_db, Some(&ml_classifier), None);

// Результаты
println!("Type: {:?}", process.process_type);  // Может быть "browser" или "gui"
println!("Tags: {:?}", process.tags);          // Теги из паттернов и ML
```

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

## Будущие улучшения

1. **Интеграция с ONNX Runtime**: Загрузка и использование реальных ML-моделей
2. **Обучение моделей**: Интеграция с `smoothtask-trainer` для обучения моделей классификации
3. **Динамическая загрузка**: Загрузка моделей во время выполнения без перезапуска демона
4. **Кэширование**: Кэширование результатов ML-классификации для оптимизации производительности

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
