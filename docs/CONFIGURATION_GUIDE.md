# Руководство по конфигурации SmoothTask

Это руководство поможет вам настроить SmoothTask для различных сценариев использования.

## 📖 Основная структура конфигурации

```yaml
# Основные настройки
polling_interval_ms: 1000
enable_snapshot_logging: true

# Пути к файлам
paths:
  snapshot_db_path: "/var/lib/smoothtask/snapshots.db"
  patterns_dir: "/etc/smoothtask/patterns"
  log_file_path: "/var/log/smoothtask/smoothtask.log"
  api_listen_addr: "0.0.0.0:8080"

# Пороги и лимиты
thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120

# Правила приоритетов
priority_rules:
  - name: "Critical applications"
    match:
      tags: ["audio", "video", "game"]
    priority: "latency_critical"

# Пользовательские метрики
custom_metrics:
  - id: "cpu_usage"
    name: "CPU Usage"
    description: "Current CPU usage percentage"
    source: "command"
    source_config:
      command: "top -bn1 | grep \"Cpu(s)\" | sed \"s/.*, *\([0-9.]*\)%* id.*/\1/\" | awk '{print 100 - $1}'"
    update_interval: 60

  - id: "memory_free"
    name: "Free Memory"
    description: "Available memory in MB"
    source: "file"
    source_config:
      path: "/proc/meminfo"
      pattern: "MemFree: *(\d+) kB"
    update_interval: 30

# Настройки кэширования
cache_config:
  feature_cache_capacity: 1000
  enable_memory_pressure_aware_caching: true
  memory_pressure_threshold: 0.8
```

## 🎯 Сценарии конфигурации

### 1. Разработка (Development)

**Цель**: Оптимизация для IDE, компиляторов и инструментов разработки

```yaml
# configs/smoothtask-development.yml
polling_interval_ms: 500
enable_snapshot_logging: true

paths:
  snapshot_db_path: "/var/lib/smoothtask/snapshots.db"
  patterns_dir: "/etc/smoothtask/patterns"
  log_file_path: "/var/log/smoothtask/smoothtask.log"

thresholds:
  psi_cpu_some_high: 0.5
  psi_io_some_high: 0.3
  user_idle_timeout_sec: 60

priority_rules:
  - name: "Boost IDE processes"
    match:
      tags: ["ide"]
    priority: "interactive"
    
  - name: "Boost terminal processes"
    match:
      tags: ["terminal"]
    priority: "interactive"
    
  - name: "Lower build processes"
    match:
      command: ["make", "cargo", "npm", "yarn", "gradle"]
    priority: "background"
    
  - name: "Lower background services"
    match:
      type: ["daemon"]
    priority: "idle"

cache_intervals:
  system_metrics_cache_interval: 3
  process_metrics_cache_interval: 1
```

### 2. Игры (Gaming)

**Цель**: Максимальная производительность для игр

```yaml
# configs/smoothtask-gaming.yml
polling_interval_ms: 250
enable_snapshot_logging: false

paths:
  snapshot_db_path: "/var/lib/smoothtask/snapshots.db"
  patterns_dir: "/etc/smoothtask/patterns"
  log_file_path: "/var/log/smoothtask/smoothtask.log"

thresholds:
  psi_cpu_some_high: 0.7
  psi_io_some_high: 0.5
  user_idle_timeout_sec: 30

priority_rules:
  - name: "Maximize game performance"
    match:
      tags: ["game"]
    priority: "latency_critical"
    
  - name: "Boost audio processes"
    match:
      tags: ["audio"]
    priority: "latency_critical"
    
  - name: "Lower background processes"
    match:
      type: ["daemon", "batch"]
    priority: "idle"
    
  - name: "Lower system updates"
    match:
      command: ["apt", "dnf", "pacman", "yum"]
    priority: "idle"

cache_intervals:
  system_metrics_cache_interval: 2
  process_metrics_cache_interval: 1
```

### 3. Сервер (Server)

**Цель**: Оптимизация для серверных приложений

```yaml
# configs/smoothtask-server.yml
polling_interval_ms: 1000
enable_snapshot_logging: true

paths:
  snapshot_db_path: "/var/lib/smoothtask/snapshots.db"
  patterns_dir: "/etc/smoothtask/patterns"
  log_file_path: "/var/log/smoothtask/smoothtask.log"
  api_listen_addr: "0.0.0.0:8080"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 300

priority_rules:
  - name: "Prioritize web services"
    match:
      command: ["nginx", "apache", "node", "php-fpm"]
    priority: "interactive"
    
  - name: "Prioritize database services"
    match:
      command: ["mysqld", "postgres", "mongodb", "redis"]
    priority: "interactive"
    
  - name: "Limit background jobs"
    match:
      type: ["batch"]
    priority: "background"
    
  - name: "Limit system maintenance"
    match:
      command: ["cron", "systemd", "logrotate"]
    priority: "background"

cache_intervals:
  system_metrics_cache_interval: 5
  process_metrics_cache_interval: 2
```

### 4. Ноутбук (Laptop)

**Цель**: Баланс производительности и энергосбережения

```yaml
# configs/smoothtask-laptop.yml
polling_interval_ms: 1500
enable_snapshot_logging: true

paths:
  snapshot_db_path: "/var/lib/smoothtask/snapshots.db"
  patterns_dir: "/etc/smoothtask/patterns"
  log_file_path: "/var/log/smoothtask/smoothtask.log"

thresholds:
  psi_cpu_some_high: 0.5
  psi_io_some_high: 0.3
  user_idle_timeout_sec: 120

priority_rules:
  - name: "Boost interactive applications"
    match:
      tags: ["browser", "office", "media"]
    priority: "interactive"
    
  - name: "Lower background processes"
    match:
      type: ["daemon", "batch"]
    priority: "background"
    
  - name: "Limit energy-intensive processes"
    match:
      tags: ["mining", "rendering"]
    priority: "idle"

cache_intervals:
  system_metrics_cache_interval: 4
  process_metrics_cache_interval: 2
```

### 5. Рабочая станция (Workstation)

**Цель**: Максимальная производительность для профессиональных задач

```yaml
# configs/smoothtask-workstation.yml
polling_interval_ms: 500
enable_snapshot_logging: true

paths:
  snapshot_db_path: "/var/lib/smoothtask/snapshots.db"
  patterns_dir: "/etc/smoothtask/patterns"
  log_file_path: "/var/log/smoothtask/smoothtask.log"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 60

priority_rules:
  - name: "Boost professional applications"
    match:
      tags: ["design", "video", "audio", "3d"]
    priority: "latency_critical"
    
  - name: "Boost development tools"
    match:
      tags: ["ide", "terminal", "debugger"]
    priority: "interactive"
    
  - name: "Lower background services"
    match:
      type: ["daemon"]
    priority: "background"

cache_intervals:
  system_metrics_cache_interval: 3
  process_metrics_cache_interval: 1
```

## 🔧 Расширенные настройки

### Настройка кэширования

```yaml
cache_intervals:
  # Как часто обновлять кэш системных метрик (в итерациях)
  system_metrics_cache_interval: 5
  
  # Как часто обновлять кэш метрик процессов (в итерациях)
  process_metrics_cache_interval: 2

# Расширенная конфигурация кэша
metrics_cache:
  max_cache_size: 200
  cache_ttl_seconds: 3
  max_memory_bytes: 15_000_000
  auto_cleanup_enabled: true
```

### Настройка ML-классификатора

```yaml
ml_classifier:
  enabled: true
  model_path: "/etc/smoothtask/models/ranker.onnx"
  feature_config:
    use_cpu_features: true
    use_memory_features: true
    use_io_features: true
    use_window_features: true
    use_audio_features: true
```

### Настройка уведомлений

```yaml
notifications:
  backend: "dbus"  # или "libnotify", "stub"
  enabled: true
  
  # Настройки для разных типов уведомлений
  notification_types:
    priority_changes:
      enabled: true
      min_priority_level: "interactive"
    
    errors:
      enabled: true
      severity: "warning"
    
    system_health:
      enabled: true
      severity: "critical"
```

### Настройка мониторинга

```yaml
monitoring:
  prometheus:
    enabled: true
    listen_addr: "0.0.0.0:8080"
    
  grafana:
    dashboard_path: "/etc/smoothtask/grafana/dashboards"
    
  alerting:
    enabled: true
    rules_path: "/etc/smoothtask/alerting/rules.yml"
```

## 📊 Примеры сложных правил

### Правила на основе комбинации условий

```yaml
priority_rules:
  - name: "High priority audio processing"
    match:
      tags: ["audio"]
      cpu_usage: ">5"
      memory_usage: ">100MB"
    priority: "latency_critical"
    
  - name: "Interactive GUI applications"
    match:
      has_gui_window: true
      is_focused_window: true
      user_active: true
    priority: "interactive"
```

### Правила с исключениями

```yaml
priority_rules:
  - name: "Critical system processes"
    match:
      command: ["systemd", "dbus", "Xorg"]
    priority: "latency_critical"
    
  - name: "Background processes except critical"
    match:
      type: ["daemon", "batch"]
      command: ["!systemd", "!dbus", "!Xorg"]
    priority: "background"
```

### Правила на основе времени

```yaml
priority_rules:
  - name: "Daytime interactive priority"
    match:
      time_range: "08:00-18:00"
      tags: ["browser", "office"]
    priority: "interactive"
    
  - name: "Nighttime background priority"
    match:
      time_range: "22:00-06:00"
      tags: ["browser", "office"]
    priority: "background"
```

## 🚨 Устранение неполадок конфигурации

### Проверка конфигурации

```bash
# Проверка синтаксиса YAML
smoothtaskd --dry-run --config /etc/smoothtask/smoothtask.yml

# Проверка путей
ls -la /etc/smoothtask/patterns
test -f /var/lib/smoothtask/snapshots.db
```

### Частые ошибки

1. **Неправильные пути**: Убедитесь, что все пути существуют и доступны для записи
2. **Неправильный синтаксис YAML**: Используйте инструменты проверки YAML
3. **Конфликтующие правила**: Проверьте приоритет правил
4. **Недостаточно памяти**: Увеличьте лимиты кэша

### Логирование и отладка

```yaml
debug:
  enabled: true
  log_level: "debug"
  
  # Логирование конкретных компонентов
  component_logging:
    metrics: "debug"
    policy: "info"
    actuator: "warn"
```

## 🎓 Лучшие практики

1. **Начинайте с консервативных настроек** и постепенно оптимизируйте
2. **Используйте dry-run режим** для тестирования новых конфигураций
3. **Мониторьте производительность** с помощью Prometheus и Grafana
4. **Регулярно обновляйте паттерны** для лучшей классификации
5. **Оптимизируйте кэширование** для вашей рабочей нагрузки
6. **Используйте динамическую конфигурацию** через API для быстрой настройки без перезагрузки

## 🔄 Динамическое управление конфигурацией через API

SmoothTask поддерживает динамическое обновление конфигурации через REST API, что позволяет изменять параметры работы демона без перезагрузки.

### Основные возможности

- **Частичное обновление**: Изменение только нужных параметров без затрагивания остальных
- **Немедленное применение**: Все изменения применяются сразу после успешного запроса
- **Валидация**: Автоматическая проверка корректности передаваемых значений
- **Кэш очистка**: Автоматическая очистка кэша конфигурации после обновления

### Поддерживаемые параметры для динамического обновления

| Параметр | Тип | Диапазон/Значения | Описание |
|----------|-----|-------------------|----------|
| `polling_interval_ms` | u64 | 100-60000 | Интервал опроса системы в миллисекундах |
| `max_candidates` | usize | 10-1000 | Максимальное количество кандидатов для обработки |
| `dry_run_default` | bool | true/false | Режим dry-run по умолчанию |
| `policy_mode` | string | "rules-only", "hybrid" | Режим работы Policy Engine |
| `enable_snapshot_logging` | bool | true/false | Включение логирования снапшотов |

### Примеры использования

#### Базовое обновление через curl

```bash
# Увеличение интервала опроса для снижения нагрузки
curl -X POST http://127.0.0.1:8080/api/config \
  -H "Content-Type: application/json" \
  -d '{"polling_interval_ms": 2000}'

# Переключение на гибридный режим (rules + ML)
curl -X POST http://127.0.0.1:8080/api/config \
  -H "Content-Type: application/json" \
  -d '{"policy_mode": "hybrid"}'

# Комплексное обновление нескольких параметров
curl -X POST http://127.0.0.1:8080/api/config \
  -H "Content-Type: application/json" \
  -d '{
    "polling_interval_ms": 1500,
    "max_candidates": 200,
    "dry_run_default": false,
    "policy_mode": "hybrid",
    "enable_snapshot_logging": true
  }'
```

#### Интеграция с системами автоматического масштабирования

```bash
#!/bin/bash
# Скрипт для автоматической настройки SmoothTask в зависимости от нагрузки

# Получение текущей нагрузки CPU
CPU_LOAD=$(top -bn1 | grep "Cpu(s)" | sed "s/.*, *\([0-9.]*\)%* id.*/\1/" | awk '{print 100 - $1}')

if [ $(echo "$CPU_LOAD > 80" | bc) -eq 1 ]; then
    # Высокая нагрузка - оптимизировать для производительности
    echo "Высокая нагрузка CPU ($CPU_LOAD%), оптимизация для производительности"
    curl -X POST http://127.0.0.1:8080/api/config \
      -H "Content-Type: application/json" \
      -d '{
        "polling_interval_ms": 2000,
        "max_candidates": 100
      }'
else
    # Нормальная нагрузка - оптимизировать для отзывчивости
    echo "Нормальная нагрузка CPU ($CPU_LOAD%), оптимизация для отзывчивости"
    curl -X POST http://127.0.0.1:8080/api/config \
      -H "Content-Type: application/json" \
      -d '{
        "polling_interval_ms": 500,
        "max_candidates": 300
      }'
fi
```

#### Python пример с обработкой ответов

```python
import requests
import json

def update_smoothtask_config(new_params):
    """
    Обновляет конфигурацию SmoothTask через API
    
    Args:
        new_params (dict): Словарь с параметрами для обновления
        
    Returns:
        dict: Результат обновления
    """
    url = "http://127.0.0.1:8080/api/config"
    headers = {"Content-Type": "application/json"}
    
    try:
        response = requests.post(url, json=new_params, headers=headers)
        response.raise_for_status()
        
        result = response.json()
        if result.get("status") == "success":
            print(f"✅ Конфигурация успешно обновлена: {result['message']}")
            return result
        else:
            print(f"❌ Ошибка обновления: {result.get('message', 'Неизвестная ошибка')}")
            return None
    except requests.exceptions.RequestException as e:
        print(f"❌ Ошибка соединения: {e}")
        return None

# Пример использования
config_update = {
    "polling_interval_ms": 1000,
    "policy_mode": "hybrid",
    "enable_snapshot_logging": True
}

update_smoothtask_config(config_update)
```

### Лучшие практики

1. **Используйте частичные обновления**: Обновляйте только те параметры, которые нужно изменить
2. **Проверяйте текущую конфигурацию**: Перед обновлением получите текущую конфигурацию через `GET /api/config`
3. **Обрабатывайте ошибки**: Всегда проверяйте статус ответа и обрабатывайте ошибки валидации
4. **Логируйте изменения**: Ведите журнал изменений конфигурации для отладки
5. **Используйте в комбинации с файловой конфигурацией**: Динамические изменения через API дополняют, а не заменяют файловую конфигурацию

### Ограничения

- Не все параметры конфигурации поддерживают динамическое обновление
- Изменения порогов (`thresholds`) и путей (`paths`) требуют перезагрузки демона
- Для сложных изменений рекомендуется использовать комбинацию API и файловой конфигурации

### Перезагрузка конфигурации из файла

Если вам нужно применить изменения из конфигурационного файла:

```bash
# Перезагрузка полной конфигурации из файла
curl -X POST http://127.0.0.1:8080/api/config/reload
```

**Дополнительная информация:**
- Полная документация API доступна в [API.md](API.md)
- Примеры интеграции с системами мониторинга см. в [GETTING_STARTED.md](GETTING_STARTED.md)

## 📝 Примеры полных конфигураций

См. директорию `configs/examples/` для полных примеров конфигураций:
- `smoothtask-development.yml`
- `smoothtask-gaming.yml`
- `smoothtask-laptop.yml`
- `smoothtask-server.yml`
- `smoothtask-workstation.yml`

---

*Последнее обновление: 2025-12-12*
