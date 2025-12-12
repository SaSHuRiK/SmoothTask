# JSON Schema для конфигурации SmoothTask API

Этот документ описывает JSON Schema для конфигурационного файла SmoothTask API. Schema позволяет валидировать конфигурационные файлы и обеспечивать их корректность.

## Основная структура конфигурации

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "SmoothTask Configuration Schema",
  "description": "Schema for SmoothTask daemon configuration file",
  "type": "object",
  "properties": {
    "polling_interval_ms": {
      "type": "integer",
      "minimum": 100,
      "maximum": 10000,
      "default": 1000,
      "description": "Интервал опроса системы в миллисекундах"
    },
    "max_candidates": {
      "type": "integer",
      "minimum": 10,
      "maximum": 1000,
      "default": 150,
      "description": "Максимальное количество кандидатов для обработки"
    },
    "dry_run_default": {
      "type": "boolean",
      "default": false,
      "description": "Режим dry-run по умолчанию (без применения изменений)"
    },
    "policy_mode": {
      "type": "string",
      "enum": ["rules-only", "hybrid", "ml-only"],
      "default": "rules-only",
      "description": "Режим работы Policy Engine"
    },
    "enable_snapshot_logging": {
      "type": "boolean",
      "default": true,
      "description": "Включение логирования снапшотов в SQLite"
    },
    "thresholds": {
      "type": "object",
      "properties": {
        "psi_cpu_some_high": {
          "type": "number",
          "minimum": 0,
          "maximum": 1,
          "default": 0.6,
          "description": "Порог PSI CPU для определения высокого давления"
        },
        "psi_io_some_high": {
          "type": "number",
          "minimum": 0,
          "maximum": 1,
          "default": 0.4,
          "description": "Порог PSI IO для определения высокого давления"
        },
        "user_idle_timeout_sec": {
          "type": "integer",
          "minimum": 30,
          "maximum": 600,
          "default": 120,
          "description": "Таймаут неактивности пользователя в секундах"
        },
        "interactive_build_grace_sec": {
          "type": "integer",
          "minimum": 5,
          "maximum": 60,
          "default": 10,
          "description": "Период отсрочки для интерактивных сборок в секундах"
        },
        "noisy_neighbour_cpu_share": {
          "type": "number",
          "minimum": 0.1,
          "maximum": 1,
          "default": 0.7,
          "description": "Доля CPU для определения 'шумного соседа'"
        },
        "crit_interactive_percentile": {
          "type": "number",
          "minimum": 0,
          "maximum": 1,
          "default": 0.9,
          "description": "Перцентиль для критически интерактивных процессов"
        },
        "interactive_percentile": {
          "type": "number",
          "minimum": 0,
          "maximum": 1,
          "default": 0.6,
          "description": "Перцентиль для интерактивных процессов"
        },
        "normal_percentile": {
          "type": "number",
          "minimum": 0,
          "maximum": 1,
          "default": 0.3,
          "description": "Перцентиль для обычных процессов"
        },
        "background_percentile": {
          "type": "number",
          "minimum": 0,
          "maximum": 1,
          "default": 0.1,
          "description": "Перцентиль для фоновых процессов"
        },
        "sched_latency_p99_threshold_ms": {
          "type": "number",
          "minimum": 5,
          "maximum": 100,
          "default": 20.0,
          "description": "Порог P99 scheduling latency в миллисекундах"
        },
        "ui_loop_p95_threshold_ms": {
          "type": "number",
          "minimum": 5,
          "maximum": 100,
          "default": 16.67,
          "description": "Порог P95 UI loop latency в миллисекундах"
        }
      },
      "required": [
        "psi_cpu_some_high",
        "psi_io_some_high",
        "user_idle_timeout_sec",
        "interactive_build_grace_sec",
        "noisy_neighbour_cpu_share",
        "crit_interactive_percentile",
        "interactive_percentile",
        "normal_percentile",
        "background_percentile",
        "sched_latency_p99_threshold_ms",
        "ui_loop_p95_threshold_ms"
      ]
    },
    "paths": {
      "type": "object",
      "properties": {
        "snapshot_db_path": {
          "type": "string",
          "default": "/var/lib/smoothtask/snapshots.db",
          "description": "Путь к базе данных снапшотов"
        },
        "patterns_dir": {
          "type": "string",
          "default": "/etc/smoothtask/patterns",
          "description": "Директория с паттернами приложений"
        },
        "api_listen_addr": {
          "type": "string",
          "format": "host-port",
          "default": "127.0.0.1:8080",
          "description": "Адрес для прослушивания API сервера"
        }
      },
      "required": [
        "snapshot_db_path",
        "patterns_dir"
      ]
    },
    "notifications": {
      "type": "object",
      "properties": {
        "enabled": {
          "type": "boolean",
          "default": false,
          "description": "Включение системы уведомлений"
        },
        "backend": {
          "type": "string",
          "enum": ["stub", "libnotify"],
          "default": "stub",
          "description": "Тип бэкенда уведомлений"
        },
        "app_name": {
          "type": "string",
          "default": "SmoothTask",
          "description": "Имя приложения для уведомлений"
        },
        "min_level": {
          "type": "string",
          "enum": ["critical", "warning", "info"],
          "default": "warning",
          "description": "Минимальный уровень важности уведомлений"
        }
      },
      "required": [
        "enabled",
        "backend",
        "app_name",
        "min_level"
      ]
    },
    "model": {
      "type": "object",
      "properties": {
        "model_path": {
          "type": "string",
          "default": "models/ranker.onnx",
          "description": "Путь к ONNX модели для ранжирования"
        },
        "enabled": {
          "type": "boolean",
          "default": false,
          "description": "Включение ONNX ранкера"
        }
      },
      "required": [
        "model_path",
        "enabled"
      ]
    }
  },
  "required": [
    "polling_interval_ms",
    "max_candidates",
    "dry_run_default",
    "policy_mode",
    "enable_snapshot_logging",
    "thresholds",
    "paths",
    "notifications",
    "model"
  ]
}
```

## Использование JSON Schema

### Валидация конфигурационного файла

Для валидации конфигурационного файла можно использовать различные инструменты, поддерживающие JSON Schema:

```bash
# Использование ajv-cli для валидации
npx ajv-cli validate -s api-config-schema.json -d smoothtask.yml

# Использование Python с jsonschema
python3 -c "
import jsonschema
import yaml
import json

# Загрузка схемы
with open('api-config-schema.json') as f:
    schema = json.load(f)

# Загрузка конфигурации
with open('smoothtask.yml') as f:
    config = yaml.safe_load(f)

# Валидация
try:
    jsonschema.validate(instance=config, schema=schema)
    print('Configuration is valid!')
except jsonschema.ValidationError as e:
    print(f'Validation error: {e.message}')
    print(f'Path: {list(e.path)}')
"
```

### Генерация JSON Schema

Для генерации JSON Schema из Rust структур можно использовать crate `schemars`:

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
struct Config {
    polling_interval_ms: u64,
    max_candidates: usize,
    dry_run_default: bool,
    policy_mode: String,
    enable_snapshot_logging: bool,
    thresholds: Thresholds,
    paths: Paths,
    notifications: Notifications,
    model: ModelConfig,
}

// Генерация схемы
let schema = schemars::schema_for!(Config);
let json_schema = serde_json::to_string_pretty(&schema).unwrap();
std::fs::write("api-config-schema.json", json_schema).unwrap();
```

## Примеры конфигурационных файлов

### Минимальная конфигурация

```yaml
polling_interval_ms: 1000
max_candidates: 150
dry_run_default: false
policy_mode: "rules-only"
enable_snapshot_logging: true

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7
  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67

paths:
  snapshot_db_path: "/var/lib/smoothtask/snapshots.db"
  patterns_dir: "/etc/smoothtask/patterns"

notifications:
  enabled: false
  backend: "stub"
  app_name: "SmoothTask"
  min_level: "warning"

model:
  model_path: "models/ranker.onnx"
  enabled: false
```

### Полная конфигурация с API

```yaml
polling_interval_ms: 500
max_candidates: 200
dry_run_default: true
policy_mode: "hybrid"
enable_snapshot_logging: true

thresholds:
  psi_cpu_some_high: 0.5
  psi_io_some_high: 0.3
  user_idle_timeout_sec: 60
  interactive_build_grace_sec: 5
  noisy_neighbour_cpu_share: 0.5
  crit_interactive_percentile: 0.8
  interactive_percentile: 0.5
  normal_percentile: 0.2
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 15.0
  ui_loop_p95_threshold_ms: 12.5

paths:
  snapshot_db_path: "/var/lib/smoothtask/snapshots.db"
  patterns_dir: "/etc/smoothtask/patterns"
  api_listen_addr: "127.0.0.1:8080"

notifications:
  enabled: true
  backend: "libnotify"
  app_name: "SmoothTask Production"
  min_level: "warning"

model:
  model_path: "models/ranker.onnx"
  enabled: true
```

## Интеграция с API

JSON Schema может быть интегрировано в API для автоматической валидации конфигурации:

```rust
use axum::{
    extract::State,
    response::Json,
};
use jsonschema::JSONSchema;
use serde_json::json;

async fn validate_config(
    State(schema): State<JSONSchema>,
    Json(config): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    match schema.validate(&config) {
        Ok(_) => Json(json!({
            "status": "success",
            "message": "Configuration is valid"
        })),
        Err(errors) => Json(json!({
            "status": "error",
            "message": "Configuration validation failed",
            "errors": errors
        })),
    }
}
```

## Лучшие практики

1. **Валидация перед применением**: Всегда валидируйте конфигурацию перед её применением
2. **Использование значений по умолчанию**: Указывайте значения по умолчанию для всех опциональных полей
3. **Документирование изменений**: Документируйте изменения в схеме при обновлении конфигурации
4. **Тестирование**: Тестируйте конфигурацию с различными сценариями использования
5. **Обратная совместимость**: Сохраняйте обратную совместимость при изменении схемы

## Ссылки

- [JSON Schema Official Documentation](https://json-schema.org/)
- [JSON Schema Validation](https://json-schema.org/understanding-json-schema/)
- [Schemars - Rust JSON Schema Generator](https://docs.rs/schemars/latest/schemars/)
- [SmoothTask Configuration Guide](SETUP_GUIDE.md)
- [SmoothTask API Documentation](API.md)