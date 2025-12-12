# SmoothTask Control API

SmoothTask предоставляет HTTP API для мониторинга работы демона и просмотра текущего состояния системы.

## Конфигурация

API сервер настраивается через поле `paths.api_listen_addr` в конфигурационном файле:

```yaml
paths:
  api_listen_addr: "127.0.0.1:8080"  # Адрес для прослушивания API сервера
```

Если `api_listen_addr` не указан или равен `null`, API сервер не запускается.

По умолчанию API сервер запускается на `127.0.0.1:8080`.

## Endpoints

### GET /health

Проверка работоспособности API сервера.

**Запрос:**
```bash
curl http://127.0.0.1:8080/health
```

**Ответ:**
```json
{
  "status": "ok",
  "service": "smoothtask-api"
}
```

**Статус коды:**
- `200 OK` - API сервер работает

---

### GET /api/version

Получение версии демона SmoothTask.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/version
```

**Ответ:**
```json
{
  "status": "ok",
  "version": "0.0.1",
  "service": "smoothtaskd"
}
```

**Поля:**
- `status` (string) - статус ответа (всегда "ok")
- `version` (string) - версия демона (соответствует версии из Cargo.toml)
- `service` (string) - имя сервиса (всегда "smoothtaskd")

**Статус коды:**
- `200 OK` - запрос выполнен успешно

---

### GET /api/endpoints

Получение списка всех доступных endpoints API.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/endpoints
```

**Ответ:**
```json
{
  "status": "ok",
  "endpoints": [
    {
      "path": "/health",
      "method": "GET",
      "description": "Проверка работоспособности API сервера"
    },
    {
      "path": "/api/version",
      "method": "GET",
      "description": "Получение версии демона SmoothTask"
    },
    {
      "path": "/api/endpoints",
      "method": "GET",
      "description": "Получение списка всех доступных endpoints"
    },
    {
      "path": "/api/stats",
      "method": "GET",
      "description": "Получение статистики работы демона"
    },
    {
      "path": "/api/metrics",
      "method": "GET",
      "description": "Получение последних системных метрик"
    },
    {
      "path": "/api/responsiveness",
      "method": "GET",
      "description": "Получение последних метрик отзывчивости системы"
    },
    {
      "path": "/api/processes",
      "method": "GET",
      "description": "Получение списка последних процессов"
    },
    {
      "path": "/api/processes/:pid",
      "method": "GET",
      "description": "Получение информации о конкретном процессе по PID"
    },
    {
      "path": "/api/appgroups",
      "method": "GET",
      "description": "Получение списка последних групп приложений"
    },
    {
      "path": "/api/appgroups/:id",
      "method": "GET",
      "description": "Получение информации о конкретной группе приложений по ID"
    },
    {
      "path": "/api/config",
      "method": "GET",
      "description": "Получение текущей конфигурации демона (без секретов)"
    }
  ],
  "count": 12
}
```

**Поля:**
- `status` (string) - статус ответа (всегда "ok")
- `endpoints` (array) - массив объектов с информацией о каждом endpoint
  - `path` (string) - путь к endpoint
  - `method` (string) - HTTP метод (всегда "GET" для текущих endpoints)
  - `description` (string) - описание назначения endpoint
- `count` (integer) - общее количество доступных endpoints

**Статус коды:**
- `200 OK` - запрос выполнен успешно

---

### GET /api/stats

Получение статистики работы демона.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/stats
```

**Ответ (если демон запущен):**
```json
{
  "status": "ok",
  "daemon_stats": {
    "total_iterations": 100,
    "successful_iterations": 98,
    "error_iterations": 2,
    "total_duration_ms": 50000,
    "max_iteration_duration_ms": 600,
    "total_applied_adjustments": 450,
    "total_apply_errors": 5
  }
}
```

**Ответ (если демон не запущен):**
```json
{
  "status": "ok",
  "daemon_stats": null,
  "message": "Daemon stats not available (daemon may not be running)"
}
```

**Поля `daemon_stats`:**
- `total_iterations` (u64) - общее количество итераций (успешных и с ошибками)
- `successful_iterations` (u64) - количество успешных итераций (без ошибок сбора метрик)
- `error_iterations` (u64) - количество итераций с ошибками (ошибки при сборе метрик)
- `total_duration_ms` (u128) - суммарное время выполнения всех успешных итераций (в миллисекундах)
- `max_iteration_duration_ms` (u128) - максимальное время выполнения одной итерации (в миллисекундах)
- `total_applied_adjustments` (u64) - общее количество применённых изменений приоритетов
- `total_apply_errors` (u64) - общее количество ошибок при применении приоритетов

**Статус коды:**
- `200 OK` - запрос выполнен успешно

---

### GET /api/metrics

Получение последних системных метрик.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/metrics
```

**Ответ (если метрики доступны):**
```json
{
  "status": "ok",
  "system_metrics": {
    "cpu_times": {
      "user": 1000,
      "nice": 20,
      "system": 500,
      "idle": 2000,
      "iowait": 10,
      "irq": 5,
      "softirq": 5,
      "steal": 0,
      "guest": 0,
      "guest_nice": 0
    },
    "memory": {
      "mem_total_kb": 16384000,
      "mem_available_kb": 8000000,
      "mem_free_kb": 6000000,
      "buffers_kb": 500000,
      "cached_kb": 1500000,
      "swap_total_kb": 4096000,
      "swap_free_kb": 3500000
    },
    "load_avg": {
      "one": 1.5,
      "five": 1.2,
      "fifteen": 1.0
    },
    "pressure": {
      "cpu": {
        "some": {
          "avg10": 0.05,
          "avg60": 0.03,
          "avg300": 0.02
        },
        "full": null
      },
      "io": {
        "some": {
          "avg10": 0.1,
          "avg60": 0.08,
          "avg300": 0.05
        },
        "full": null
      },
      "memory": {
        "some": {
          "avg10": 0.02,
          "avg60": 0.01,
          "avg300": 0.01
        },
        "full": {
          "avg10": 0.0,
          "avg60": 0.0,
          "avg300": 0.0
        }
      }
    }
  }
}
```

**Ответ (если метрики недоступны):**
```json
{
  "status": "ok",
  "system_metrics": null,
  "message": "System metrics not available (daemon may not be running or no metrics collected yet)"
}
```

**Поля `system_metrics`:**
- `cpu_times` - счётчики CPU из `/proc/stat`
- `memory` - информация о памяти из `/proc/meminfo`
- `load_avg` - средняя нагрузка системы из `/proc/loadavg`
- `pressure` - метрики давления из PSI (`/proc/pressure/*`)

**Статус коды:**
- `200 OK` - запрос выполнен успешно

---

### GET /api/responsiveness

Получение последних метрик отзывчивости системы.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/responsiveness
```

**Ответ (если метрики доступны):**
```json
{
  "status": "ok",
  "responsiveness_metrics": {
    "sched_latency_p95_ms": 15.5,
    "sched_latency_p99_ms": 25.0,
    "audio_xruns_delta": 0,
    "ui_loop_p95_ms": 12.3,
    "frame_jank_ratio": 0.02,
    "bad_responsiveness": false,
    "responsiveness_score": 0.95
  }
}
```

**Ответ (если метрики недоступны):**
```json
{
  "status": "ok",
  "responsiveness_metrics": null,
  "message": "Responsiveness metrics not available (daemon may not be running or no metrics collected yet)"
}
```

**Поля `responsiveness_metrics`:**
- `sched_latency_p95_ms` (f64, optional) - 95-й перцентиль задержки планировщика в миллисекундах
- `sched_latency_p99_ms` (f64, optional) - 99-й перцентиль задержки планировщика в миллисекундах
- `audio_xruns_delta` (u64, optional) - количество новых XRUN событий в аудио подсистеме
- `ui_loop_p95_ms` (f64, optional) - 95-й перцентиль времени цикла UI в миллисекундах
- `frame_jank_ratio` (f64, optional) - отношение пропущенных/задержанных кадров
- `bad_responsiveness` (bool) - флаг, указывающий на плохую отзывчивость системы
- `responsiveness_score` (f64, optional) - общий балл отзывчивости (0.0 - 1.0, где 1.0 - идеальная отзывчивость)

**Статус коды:**
- `200 OK` - запрос выполнен успешно

---

### GET /api/processes

Получение списка последних процессов.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/processes
```

**Ответ (если процессы доступны):**
```json
{
  "status": "ok",
  "processes": [
    {
      "pid": 1234,
      "ppid": 1,
      "uid": 1000,
      "gid": 1000,
      "exe": "/usr/bin/firefox",
      "cmdline": "firefox",
      "cgroup_path": "/user.slice/user-1000.slice/session-2.scope",
      "systemd_unit": "session-2.scope",
      "app_group_id": "firefox-1234",
      "state": "S",
      "start_time": 1234567890,
      "uptime_sec": 3600,
      "tty_nr": 0,
      "has_tty": false,
      "cpu_share_1s": 0.1,
      "cpu_share_10s": 0.05,
      "io_read_bytes": 1000000,
      "io_write_bytes": 500000,
      "rss_mb": 500,
      "swap_mb": 0,
      "voluntary_ctx": 1000,
      "involuntary_ctx": 100,
      "has_gui_window": true,
      "is_focused_window": true,
      "window_state": "Focused",
      "env_has_display": true,
      "env_has_wayland": false,
      "env_term": null,
      "env_ssh": false,
      "is_audio_client": false,
      "has_active_stream": false,
      "process_type": "gui",
      "tags": ["browser"],
      "nice": 0,
      "ionice_class": null,
      "ionice_prio": null,
      "teacher_priority_class": null,
      "teacher_score": null
    }
  ],
  "count": 1
}
```

**Ответ (если процессы недоступны):**
```json
{
  "status": "ok",
  "processes": null,
  "count": 0,
  "message": "Processes not available (daemon may not be running or no processes collected yet)"
}
```

**Поля процесса:**
- `pid` (u32) - идентификатор процесса
- `ppid` (u32) - идентификатор родительского процесса
- `uid` (u32) - идентификатор пользователя
- `gid` (u32) - идентификатор группы
- `exe` (String?) - путь к исполняемому файлу
- `cmdline` (String?) - командная строка процесса
- `cgroup_path` (String?) - путь к cgroup процесса
- `systemd_unit` (String?) - имя systemd unit
- `app_group_id` (String?) - идентификатор группы приложений
- `state` (String) - состояние процесса (R, S, D, Z, T, t, W)
- `start_time` (u64) - время запуска процесса (в тиках)
- `uptime_sec` (u64) - время работы процесса (в секундах)
- `tty_nr` (u32) - номер терминала
- `has_tty` (bool) - есть ли у процесса терминал
- `cpu_share_1s` (f64?) - доля использования CPU за последнюю секунду
- `cpu_share_10s` (f64?) - доля использования CPU за последние 10 секунд
- `io_read_bytes` (u64?) - количество прочитанных байт
- `io_write_bytes` (u64?) - количество записанных байт
- `rss_mb` (u64?) - размер резидентной памяти (в мегабайтах)
- `swap_mb` (u64?) - размер swap памяти (в мегабайтах)
- `voluntary_ctx` (u64?) - количество добровольных переключений контекста
- `involuntary_ctx` (u64?) - количество принудительных переключений контекста
- `has_gui_window` (bool) - есть ли у процесса GUI окно
- `is_focused_window` (bool) - является ли окно процесса фокусным
- `window_state` (String?) - состояние окна (Focused, Minimized, Fullscreen)
- `env_has_display` (bool) - есть ли переменная окружения DISPLAY
- `env_has_wayland` (bool) - есть ли переменная окружения WAYLAND_DISPLAY
- `env_term` (String?) - значение переменной окружения TERM
- `env_ssh` (bool) - запущен ли процесс через SSH
- `is_audio_client` (bool) - является ли процесс аудио-клиентом
- `has_active_stream` (bool) - есть ли у процесса активный аудио-поток
- `process_type` (String?) - тип процесса (gui, cli, daemon, batch, etc.)
- `tags` (Vec<String>) - теги процесса (browser, ide, game, audio, etc.)
- `nice` (i32) - значение nice приоритета
- `ionice_class` (u8?) - класс I/O приоритета
- `ionice_prio` (u8?) - уровень I/O приоритета
- `teacher_priority_class` (String?) - класс приоритета от ML-ранкера
- `teacher_score` (f64?) - оценка от ML-ранкера

**Статус коды:**
- `200 OK` - запрос выполнен успешно

---

### GET /api/processes/:pid

Получение информации о конкретном процессе по PID.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/processes/1234
```

**Ответ (если процесс найден):**
```json
{
  "status": "ok",
  "process": {
    "pid": 1234,
    "ppid": 1,
    "uid": 1000,
    "gid": 1000,
    "exe": "/usr/bin/firefox",
    "cmdline": "firefox",
    "cgroup_path": "/user.slice/user-1000.slice/session-2.scope",
    "systemd_unit": "session-2.scope",
    "app_group_id": "firefox-1234",
    "state": "S",
    "start_time": 1234567890,
    "uptime_sec": 3600,
    "tty_nr": 0,
    "has_tty": false,
    "cpu_share_1s": 0.1,
    "cpu_share_10s": 0.05,
    "io_read_bytes": 1000000,
    "io_write_bytes": 500000,
    "rss_mb": 500,
    "swap_mb": 0,
    "voluntary_ctx": 1000,
    "involuntary_ctx": 100,
    "has_gui_window": true,
    "is_focused_window": true,
    "window_state": "Focused",
    "env_has_display": true,
    "env_has_wayland": false,
    "env_term": null,
    "env_ssh": false,
    "is_audio_client": false,
    "has_active_stream": false,
    "process_type": "gui",
    "tags": ["browser"],
    "nice": 0,
    "ionice_class": null,
    "ionice_prio": null,
    "teacher_priority_class": null,
    "teacher_score": null
  }
}
```

**Ответ (если процесс не найден):**
```json
{
  "status": "error",
  "error": "not_found",
  "message": "Process with PID 1234 not found"
}
```

**Ответ (если процессы недоступны):**
```json
{
  "status": "error",
  "error": "not_available",
  "message": "Processes not available (daemon may not be running or no processes collected yet)"
}
```

**Параметры пути:**
- `pid` (integer) - идентификатор процесса

**Статус коды:**
- `200 OK` - запрос выполнен успешно (процесс найден или недоступен)
- В случае ошибки возвращается JSON с полем `status: "error"` и описанием ошибки

---

### GET /api/appgroups

Получение списка последних групп приложений.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/appgroups
```

**Ответ (если группы доступны):**
```json
{
  "status": "ok",
  "app_groups": [
    {
      "app_group_id": "firefox-1234",
      "root_pid": 1234,
      "process_ids": [1234, 1235, 1236],
      "app_name": "firefox",
      "total_cpu_share": 0.15,
      "total_io_read_bytes": 5000000,
      "total_io_write_bytes": 2000000,
      "total_rss_mb": 1500,
      "has_gui_window": true,
      "is_focused_group": true,
      "tags": ["browser", "gui"],
      "priority_class": "interactive"
    }
  ],
  "count": 1
}
```

**Ответ (если группы недоступны):**
```json
{
  "status": "ok",
  "app_groups": null,
  "count": 0,
  "message": "App groups not available (daemon may not be running or no groups collected yet)"
}
```

**Поля группы приложений:**
- `app_group_id` (String) - идентификатор группы приложений
- `root_pid` (u32) - идентификатор корневого процесса группы
- `process_ids` (Vec<u32>) - список идентификаторов процессов в группе
- `app_name` (String?) - имя приложения
- `total_cpu_share` (f64?) - суммарная доля использования CPU группой
- `total_io_read_bytes` (u64?) - суммарное количество прочитанных байт
- `total_io_write_bytes` (u64?) - суммарное количество записанных байт
- `total_rss_mb` (u64?) - суммарный размер резидентной памяти (в мегабайтах)
- `has_gui_window` (bool) - есть ли у группы GUI окно
- `is_focused_group` (bool) - является ли группа фокусной
- `tags` (Vec<String>) - теги группы (browser, ide, game, audio, etc.)
- `priority_class` (String?) - класс приоритета группы (latency_critical, interactive, normal, background, idle)

**Статус коды:**
- `200 OK` - запрос выполнен успешно

---

### GET /api/appgroups/:id

Получение информации о конкретной группе приложений по ID.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/appgroups/firefox-1234
```

**Ответ (если группа найдена):**
```json
{
  "status": "ok",
  "app_group": {
    "app_group_id": "firefox-1234",
    "root_pid": 1234,
    "process_ids": [1234, 1235, 1236],
    "app_name": "firefox",
    "total_cpu_share": 0.15,
    "total_io_read_bytes": 5000000,
    "total_io_write_bytes": 2000000,
    "total_rss_mb": 1500,
    "has_gui_window": true,
    "is_focused_group": true,
    "tags": ["browser", "gui"],
    "priority_class": "interactive"
  }
}
```

**Ответ (если группа не найдена):**
```json
{
  "status": "error",
  "error": "not_found",
  "message": "App group with ID 'firefox-1234' not found"
}
```

**Ответ (если группы недоступны):**
```json
{
  "status": "error",
  "error": "not_available",
  "message": "App groups not available (daemon may not be running or no groups collected yet)"
}
```

**Параметры пути:**
- `id` (string) - идентификатор группы приложений

**Статус коды:**
- `200 OK` - запрос выполнен успешно (группа найдена или недоступна)
- В случае ошибки возвращается JSON с полем `status: "error"` и описанием ошибки

---

### GET /api/config

Получение текущей конфигурации демона (без секретов).

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/config
```

**Ответ (если конфигурация доступна):**
```json
{
  "status": "ok",
  "config": {
    "polling_interval_ms": 1000,
    "max_candidates": 150,
    "dry_run_default": false,
    "policy_mode": "rules-only",
    "enable_snapshot_logging": true,
    "thresholds": {
      "psi_cpu_some_high": 0.6,
      "psi_io_some_high": 0.4,
      "user_idle_timeout_sec": 120,
      "interactive_build_grace_sec": 10,
      "noisy_neighbour_cpu_share": 0.7,
      "crit_interactive_percentile": 0.9,
      "interactive_percentile": 0.6,
      "normal_percentile": 0.3,
      "background_percentile": 0.1,
      "sched_latency_p99_threshold_ms": 20.0,
      "ui_loop_p95_threshold_ms": 16.67
    },
    "paths": {
      "snapshot_db_path": "/var/lib/smoothtask/snapshots.db",
      "patterns_dir": "/etc/smoothtask/patterns",
      "api_listen_addr": "127.0.0.1:8080"
    },
    "notifications": {
      "enabled": false,
      "backend": "stub",
      "app_name": "SmoothTask",
      "min_level": "warning"
    },
    "ml_ranker": {
      "enabled": true,
      "model_path": "/etc/smoothtask/models/ranker.onnx",
      "use_categorical_features": false
    }
  }
}
```

**Ответ (если конфигурация недоступна):**
```json
{
  "status": "ok",
  "config": null,
  "message": "Config not available (daemon may not be running or config not set)"
}
```

**Поля конфигурации:**
- `polling_interval_ms` (u64) - интервал опроса системы (в миллисекундах)
- `max_candidates` (usize) - максимальное количество кандидатов для обработки
- `dry_run_default` (bool) - режим dry-run по умолчанию
- `policy_mode` (string) - режим работы Policy Engine ("rules-only" или "hybrid")
- `enable_snapshot_logging` (bool) - флаг включения логирования снапшотов
- `thresholds` (object) - пороги для определения приоритетов и метрик отзывчивости
  - `psi_cpu_some_high` (f32) - порог PSI CPU для определения высокого давления
  - `psi_io_some_high` (f32) - порог PSI IO для определения высокого давления
  - `user_idle_timeout_sec` (u64) - таймаут неактивности пользователя (в секундах)
  - `interactive_build_grace_sec` (u64) - период отсрочки для интерактивных сборок (в секундах)
  - `noisy_neighbour_cpu_share` (f32) - доля CPU для определения "шумного соседа"
  - `crit_interactive_percentile` (f32) - перцентиль для критически интерактивных процессов
  - `interactive_percentile` (f32) - перцентиль для интерактивных процессов
  - `normal_percentile` (f32) - перцентиль для обычных процессов
  - `background_percentile` (f32) - перцентиль для фоновых процессов
  - `sched_latency_p99_threshold_ms` (f64) - порог P99 scheduling latency (в миллисекундах)
  - `ui_loop_p95_threshold_ms` (f64) - порог P95 UI loop latency (в миллисекундах)
- `paths` (object) - пути к файлам и директориям
  - `snapshot_db_path` (string) - путь к базе данных снапшотов
  - `patterns_dir` (string) - директория с паттернами приложений
  - `api_listen_addr` (string?) - адрес для прослушивания API сервера
- `notifications` (object) - конфигурация системы уведомлений
  - `enabled` (bool) - флаг включения уведомлений
  - `backend` (string) - тип бэкенда уведомлений ("stub" или "libnotify")
  - `app_name` (string) - имя приложения для уведомлений
  - `min_level` (string) - минимальный уровень важности уведомлений ("critical", "warning" или "info")
- `ml_ranker` (object, опционально) - конфигурация ML-ранкера для гибридного режима
  - `enabled` (bool) - флаг включения ML-ранкера
  - `model_path` (string) - путь к ONNX модели
  - `use_categorical_features` (bool) - использовать категориальные фичи (требуется совместимая модель)

**Примечания:**
- Конфигурация возвращается как есть, так как в SmoothTask нет явных секретов (паролей, токенов и т.д.)
- Все поля конфигурации безопасны для просмотра
- Конфигурация обновляется при перезапуске демона

**Статус коды:**
- `200 OK` - запрос выполнен успешно

---

### POST /api/config/reload

Перезагрузка конфигурации демона из файла.

**Запрос:**
```bash
curl -X POST http://127.0.0.1:8080/api/config/reload
```

**Ответ (если конфигурация успешно перезагружена):**
```json
{
  "status": "success",
  "message": "Configuration successfully reloaded from file and applied",
  "old_config": {
    "polling_interval_ms": 500,
    "max_candidates": 150,
    "dry_run_default": false,
    "policy_mode": "rules-only",
    "enable_snapshot_logging": true,
    "thresholds": {
      "psi_cpu_some_high": 0.6,
      "psi_io_some_high": 0.4,
      "user_idle_timeout_sec": 120,
      "interactive_build_grace_sec": 10,
      "noisy_neighbour_cpu_share": 0.7,
      "crit_interactive_percentile": 0.9,
      "interactive_percentile": 0.6,
      "normal_percentile": 0.3,
      "background_percentile": 0.1,
      "sched_latency_p99_threshold_ms": 20.0,
      "ui_loop_p95_threshold_ms": 16.67
    },
    "paths": {
      "snapshot_db_path": "/var/lib/smoothtask/snapshots.db",
      "patterns_dir": "/etc/smoothtask/patterns",
      "api_listen_addr": "127.0.0.1:8080"
    },
    "notifications": {
      "enabled": false,
      "backend": "stub",
      "app_name": "SmoothTask",
      "min_level": "warning"
    }
  },
  "new_config": {
    "polling_interval_ms": 1000,
    "max_candidates": 200,
    "dry_run_default": true,
    "policy_mode": "rules-only",
    "enable_snapshot_logging": true,
    "thresholds": {
      "psi_cpu_some_high": 0.5,
      "psi_io_some_high": 0.3,
      "user_idle_timeout_sec": 60,
      "interactive_build_grace_sec": 5,
      "noisy_neighbour_cpu_share": 0.5,
      "crit_interactive_percentile": 0.8,
      "interactive_percentile": 0.5,
      "normal_percentile": 0.2,
      "background_percentile": 0.1,
      "sched_latency_p99_threshold_ms": 20.0,
      "ui_loop_p95_threshold_ms": 16.67
    },
    "paths": {
      "snapshot_db_path": "/var/lib/smoothtask/snapshots.db",
      "patterns_dir": "/etc/smoothtask/patterns",
      "api_listen_addr": "127.0.0.1:8080"
    },
    "notifications": {
      "enabled": false,
      "backend": "stub",
      "app_name": "SmoothTask",
      "min_level": "warning"
    }
  },
  "action_required": "Configuration has been updated and is now active.",
  "config_path": "/etc/smoothtask/smoothtask.yml"
}
```

**Ответ (если конфигурация доступна, но путь к файлу неизвестен):**
```json
{
  "status": "warning",
  "message": "Config reload requested but config file path is not available",
  "current_config": {
    "polling_interval_ms": 500,
    "max_candidates": 150,
    "dry_run_default": false,
    "policy_mode": "rules-only",
    "enable_snapshot_logging": true,
    "thresholds": {
      "psi_cpu_some_high": 0.6,
      "psi_io_some_high": 0.4,
      "user_idle_timeout_sec": 120,
      "interactive_build_grace_sec": 10,
      "noisy_neighbour_cpu_share": 0.7,
      "crit_interactive_percentile": 0.9,
      "interactive_percentile": 0.6,
      "normal_percentile": 0.3,
      "background_percentile": 0.1,
      "sched_latency_p99_threshold_ms": 20.0,
      "ui_loop_p95_threshold_ms": 16.67
    },
    "paths": {
      "snapshot_db_path": "/var/lib/smoothtask/snapshots.db",
      "patterns_dir": "/etc/smoothtask/patterns",
      "api_listen_addr": "127.0.0.1:8080"
    },
    "notifications": {
      "enabled": false,
      "backend": "stub",
      "app_name": "SmoothTask",
      "min_level": "warning"
    }
  },
  "action_required": "To enable full config reload, ensure the daemon is running with config path information."
}
```

**Ответ (если произошла ошибка загрузки конфигурации):**
```json
{
  "status": "error",
  "message": "Failed to reload configuration: Config file not found at /etc/smoothtask/smoothtask.yml",
  "current_config": {
    "polling_interval_ms": 500,
    "max_candidates": 150,
    "dry_run_default": false,
    "policy_mode": "rules-only",
    "enable_snapshot_logging": true,
    "thresholds": {
      "psi_cpu_some_high": 0.6,
      "psi_io_some_high": 0.4,
      "user_idle_timeout_sec": 120,
      "interactive_build_grace_sec": 10,
      "noisy_neighbour_cpu_share": 0.7,
      "crit_interactive_percentile": 0.9,
      "interactive_percentile": 0.6,
      "normal_percentile": 0.3,
      "background_percentile": 0.1,
      "sched_latency_p99_threshold_ms": 20.0,
      "ui_loop_p95_threshold_ms": 16.67
    },
    "paths": {
      "snapshot_db_path": "/var/lib/smoothtask/snapshots.db",
      "patterns_dir": "/etc/smoothtask/patterns",
      "api_listen_addr": "127.0.0.1:8080"
    },
    "notifications": {
      "enabled": false,
      "backend": "stub",
      "app_name": "SmoothTask",
      "min_level": "warning"
    }
  },
  "config_path": "/etc/smoothtask/smoothtask.yml",
  "action_required": "Check the configuration file for errors and try again."
}
```

**Ответ (если конфигурация недоступна):**
```json
{
  "status": "error",
  "message": "Config reload not available (daemon may not be running or config not set)"
}
```

**Поля ответа:**
- `status` (string) - статус операции: "success", "warning" или "error"
- `message` (string) - описание результата операции
- `old_config` (object, опционально) - предыдущая конфигурация (при успешной перезагрузке)
- `new_config` (object, опционально) - новая конфигурация (при успешной перезагрузке)
- `current_config` (object, опционально) - текущая конфигурация (при ошибках или предупреждениях)
- `config_path` (string, опционально) - путь к конфигурационному файлу
- `action_required` (string, опционально) - рекомендации по дальнейшим действиям

**Примечания:**

- В текущей реализации, API сервер может напрямую загружать и применять новую конфигурацию из файла
- Конфигурация обновляется в реальном времени через Arc<RwLock<Config>>, что позволяет всем компонентам использовать новую конфигурацию
- Для успешной перезагрузки требуется, чтобы путь к конфигурационному файлу был известен API серверу
- Если конфигурационный файл содержит ошибки, возвращается текущая конфигурация и сообщение об ошибке
- Уведомления настраиваются в секции `notifications` конфигурационного файла

**Статус коды:**
- `200 OK` - Успешный запрос
- `200 OK` с `status: "error"` - Ошибка загрузки конфигурации
- `200 OK` с `status: "warning"` - Конфигурация доступна, но путь к файлу неизвестен
- `200 OK` с `status: "error"` - Конфигурация недоступна

---

### Динамическая перезагрузка конфигурации

SmoothTask поддерживает динамическую перезагрузку конфигурации без необходимости перезапуска демона. Это позволяет изменять параметры работы демона "на лету", что особенно полезно для тонкой настройки и экспериментов.

#### Механизмы перезагрузки

1. **Ручная перезагрузка через API**
   - Используйте endpoint `POST /api/config/reload` для ручной перезагрузки конфигурации
   - Требуется, чтобы путь к конфигурационному файлу был известен API серверу
   - Новая конфигурация загружается из файла и применяется немедленно

2. **Автоматическая перезагрузка через ConfigWatcher**
   - Демон автоматически отслеживает изменения в конфигурационном файле
   - При обнаружении изменений, демон автоматически перезагружает конфигурацию
   - Этот механизм работает независимо от API сервера

3. **Перезапуск демона**
   - Традиционный способ - перезапуск демона для применения новой конфигурации
   - Гарантирует полную перезагрузку всех компонентов

#### Архитектура динамической перезагрузки

- **Arc<RwLock<Config>>** - Конфигурация хранится в потокобезопасном контейнере
- **ConfigWatcher** - Компонент, отслеживающий изменения в конфигурационном файле
- **API Integration** - API сервер имеет доступ к той же конфигурации через Arc<RwLock<Config>>
- **Real-time Updates** - Все компоненты демона используют актуальную конфигурацию

#### Примеры использования

**Ручная перезагрузка через API:**
```bash
# Изменить конфигурационный файл
nano /etc/smoothtask/smoothtask.yml

# Перезагрузить конфигурацию через API
curl -X POST http://127.0.0.1:8080/api/config/reload

# Проверить новую конфигурацию
curl http://127.0.0.1:8080/api/config | jq
```

**Автоматическая перезагрузка через ConfigWatcher:**
```bash
# Изменить конфигурационный файл
nano /etc/smoothtask/smoothtask.yml

# Демон автоматически обнаружит изменения и перезагрузит конфигурацию
# Можно проверить новую конфигурацию через API
curl http://127.0.0.1:8080/api/config | jq
```

**Проверка текущей конфигурации:**
```bash
# Получение текущей конфигурации
curl http://127.0.0.1:8080/api/config | jq '.config'

# Проверка конкретных параметров
curl http://127.0.0.1:8080/api/config | jq '.config.thresholds.psi_cpu_some_high'
```

#### Ограничения и рекомендации

- **Совместимость параметров** - Не все параметры могут быть изменены "на лету" без перезапуска
- **Валидация** - Новая конфигурация проходит валидацию перед применением
- **Fallback** - В случае ошибки, демон сохраняет предыдущую рабочую конфигурацию
- **Логирование** - Все события перезагрузки логируются для отладки

#### Примеры сценариев использования

1. **Тонкая настройка порогов** - Изменение порогов PSI для оптимизации реакции на нагрузку
2. **Эксперименты с политиками** - Переключение между режимами `rules-only` и `hybrid`
3. **Настройка уведомлений** - Включение/отключение уведомлений без перезапуска
4. **Изменение интервалов опроса** - Настройка частоты сбора метрик

---

### Система уведомлений

SmoothTask включает систему уведомлений для информирования пользователей о важных событиях в работе демона. Система поддерживает различные уровни важности и бэкенды для отправки уведомлений.

#### Типы уведомлений

- **Critical** - Критические уведомления, требующие немедленного внимания (например, фатальные ошибки)
- **Warning** - Предупреждения о потенциальных проблемах или неоптимальных состояниях
- **Info** - Информационные уведомления о нормальной работе системы

#### Бэкенды уведомлений

- **Stub** - Заглушка для тестирования (только логирование через tracing)
- **Libnotify** - Desktop уведомления через системную библиотеку libnotify (рекомендуется для production)

#### Конфигурация уведомлений

Уведомления настраиваются в конфигурационном файле в секции `notifications`:

```yaml
notifications:
  # Включить отправку уведомлений
  enabled: true
  
  # Тип бэкенда (stub или libnotify)
  backend: libnotify
  
  # Имя приложения для уведомлений
  app_name: "SmoothTask"
  
  # Минимальный уровень важности
  # - critical: только критические уведомления
  # - warning: предупреждения и критические уведомления
  # - info: все уведомления
  min_level: warning
```

**Примечания:**

- Для использования libnotify требуется наличие системной библиотеки libnotify
- В production рекомендуется использовать уровень `warning` или `critical` для избежания информационного шума
- Уведомления могут быть полностью отключены с помощью `enabled: false`

---

### GET /api/classes

Получение информации о всех доступных классах QoS (Quality of Service) и их параметрах приоритета.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/classes
```

**Ответ:**
```json
{
  "status": "ok",
  "classes": [
    {
      "class": "CRIT_INTERACTIVE",
      "name": "CRIT_INTERACTIVE",
      "description": "Критически интерактивные процессы (фокус + аудио/игра)",
      "params": {
        "nice": -8,
        "latency_nice": -15,
        "ionice": {
          "class": 2,
          "level": 0,
          "class_description": "best-effort"
        },
        "cgroup": {
          "cpu_weight": 200
        }
      }
    },
    {
      "class": "INTERACTIVE",
      "name": "INTERACTIVE",
      "description": "Обычные интерактивные процессы (UI/CLI)",
      "params": {
        "nice": -4,
        "latency_nice": -10,
        "ionice": {
          "class": 2,
          "level": 2,
          "class_description": "best-effort"
        },
        "cgroup": {
          "cpu_weight": 150
        }
      }
    },
    {
      "class": "NORMAL",
      "name": "NORMAL",
      "description": "Дефолтный приоритет",
      "params": {
        "nice": 0,
        "latency_nice": 0,
        "ionice": {
          "class": 2,
          "level": 4,
          "class_description": "best-effort"
        },
        "cgroup": {
          "cpu_weight": 100
        }
      }
    },
    {
      "class": "BACKGROUND",
      "name": "BACKGROUND",
      "description": "Фоновые процессы (batch/maintenance)",
      "params": {
        "nice": 5,
        "latency_nice": 10,
        "ionice": {
          "class": 2,
          "level": 6,
          "class_description": "best-effort"
        },
        "cgroup": {
          "cpu_weight": 50
        }
      }
    },
    {
      "class": "IDLE",
      "name": "IDLE",
      "description": "Процессы, которые можно выполнять \"на остатке\"",
      "params": {
        "nice": 10,
        "latency_nice": 15,
        "ionice": {
          "class": 3,
          "level": 0,
          "class_description": "idle"
        },
        "cgroup": {
          "cpu_weight": 25
        }
      }
    }
  ],
  "count": 5
}
```

**Поля:**
- `status` (string) - статус ответа (всегда "ok")
- `classes` (array) - массив объектов с информацией о классах QoS
  - `class` (string) - имя класса в формате SCREAMING_SNAKE_CASE
  - `name` (string) - строковое представление класса (то же, что и `class`)
  - `description` (string) - описание класса и его назначения
  - `params` (object) - параметры приоритета для класса
    - `nice` (integer) - значение nice (от -20 до +19, в SmoothTask используется диапазон -8..+10)
    - `latency_nice` (integer) - значение latency_nice (от -20 до +19)
      - -20 = максимальная чувствительность к задержке (UI, аудио, игры)
      - +19 = безразличие к задержке (batch, индексация)
    - `ionice` (object) - параметры IO приоритета
      - `class` (integer) - класс IO: 1 (realtime), 2 (best-effort), 3 (idle)
      - `level` (integer) - уровень приоритета внутри класса (0-7 для best-effort)
      - `class_description` (string) - текстовое описание класса IO
    - `cgroup` (object) - параметры cgroup v2
      - `cpu_weight` (integer) - вес CPU для cgroup (от 1 до 10000, в SmoothTask используется диапазон 25-200)
- `count` (integer) - количество классов (всегда 5)

**Примечания:**
- Endpoint всегда возвращает все 5 классов QoS, независимо от состояния демона
- Параметры классов фиксированы и соответствуют настройкам в `policy::classes`
- Классы отсортированы по убыванию важности: CRIT_INTERACTIVE > INTERACTIVE > NORMAL > BACKGROUND > IDLE

**Статус коды:**
- `200 OK` - запрос выполнен успешно

---

### GET /api/patterns

Получение информации о загруженных паттернах для классификации процессов.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/patterns
```

**Ответ (с паттернами):**
```json
{
  "status": "ok",
  "categories": [
    {
      "category": "browser",
      "patterns": [
        {
          "name": "firefox",
          "label": "Mozilla Firefox",
          "exe_patterns": ["firefox", "firefox-*-bin"],
          "desktop_patterns": ["firefox.desktop"],
          "cgroup_patterns": ["*firefox*"],
          "tags": ["browser", "gui"]
        },
        {
          "name": "chromium",
          "label": "Chromium",
          "exe_patterns": ["chromium", "chromium-browser"],
          "desktop_patterns": [],
          "cgroup_patterns": [],
          "tags": ["browser", "gui"]
        }
      ],
      "count": 2
    }
  ],
  "total_patterns": 2,
  "total_categories": 1
}
```

**Ответ (без паттернов):**
```json
{
  "status": "ok",
  "categories": [],
  "total_patterns": 0,
  "total_categories": 0,
  "message": "Pattern database not available (daemon may not be running or patterns not loaded)"
}
```

**Поля:**
- `categories` (array) - массив категорий паттернов
- `total_patterns` (number) - общее количество паттернов
- `total_categories` (number) - общее количество категорий

**Статус коды:**
- `200 OK` - запрос выполнен успешно

---

### GET /api/system

Получение информации о системе (ядро, архитектура, дистрибутив).

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/system
```

**Ответ:**
```json
{
  "status": "ok",
  "system": {
    "kernel": {
      "version_string": "Linux version 6.14.0-36-generic (buildd@lcy02-amd64-001) (gcc (Ubuntu 13.3.0-1ubuntu1) 13.3.0, GNU ld (GNU Binutils for Ubuntu) 2.42) #1-Ubuntu SMP PREEMPT_DYNAMIC Wed Oct 16 10:00:00 UTC 2024",
      "version": "6.14.0-36-generic"
    },
    "architecture": "x86_64",
    "distribution": {
      "name": "Ubuntu",
      "version": "24.04",
      "id": "ubuntu",
      "id_like": "debian",
      "pretty_name": "Ubuntu 24.04 LTS"
    }
  }
}
```

**Поля `system`:**
- `kernel` (object) - информация о ядре Linux
  - `version_string` (string, опционально) - полная строка версии ядра из `/proc/version`
  - `version` (string, опционально) - версия ядра (извлечённая из version_string)
- `architecture` (string | null) - архитектура системы (например, "x86_64", "aarch64")
- `distribution` (object) - информация о дистрибутиве Linux из `/etc/os-release`
  - `name` (string, опционально) - название дистрибутива
  - `version` (string, опционально) - версия дистрибутива
  - `id` (string, опционально) - идентификатор дистрибутива
  - `id_like` (string, опционально) - похожие дистрибутивы
  - `pretty_name` (string, опционально) - красивое название дистрибутива

**Примечания:**
- Информация о ядре читается из `/proc/version`
- Архитектура читается из `/proc/sys/kernel/arch`
- Информация о дистрибутиве читается из `/etc/os-release` (может быть недоступна в контейнерах)
- Если какой-то источник информации недоступен, соответствующие поля могут отсутствовать или быть `null`

**Статус коды:**
- `200 OK` - запрос выполнен успешно

**Поля:**
- `status` (string) - статус ответа (всегда "ok")
- `categories` (array) - массив объектов с категориями паттернов
  - `category` (string) - название категории (например, "browser", "ide", "terminal")
  - `patterns` (array) - массив паттернов в этой категории
    - `name` (string) - имя паттерна (идентификатор)
    - `label` (string) - человекочитаемое название приложения
    - `exe_patterns` (array) - паттерны для сопоставления с именем исполняемого файла (поддерживаются wildcards: `*`, `?`)
    - `desktop_patterns` (array) - паттерны для сопоставления с desktop-файлом
    - `cgroup_patterns` (array) - паттерны для сопоставления с путём cgroup
    - `tags` (array) - теги, которые присваиваются процессу при совпадении
  - `count` (integer) - количество паттернов в категории
- `total_patterns` (integer) - общее количество паттернов во всех категориях
- `total_categories` (integer) - количество категорий
- `message` (string, опционально) - сообщение об отсутствии данных (если паттерны не загружены)

**Примечания:**
- Endpoint возвращает все паттерны, загруженные из YAML файлов в директории `patterns_dir` (указана в конфигурации)
- Паттерны группируются по категориям для удобства просмотра
- Если паттерны не загружены (демон не запущен или произошла ошибка загрузки), endpoint вернёт пустой список с сообщением
- Паттерны используются для классификации процессов и определения их типа и тегов

**Статус коды:**
- `200 OK` - запрос выполнен успешно

---

## Обновление данных

Данные в API обновляются при каждой итерации демона (согласно `polling_interval_ms` в конфигурации). Это означает, что:

- Статистика демона (`/api/stats`) обновляется после каждой итерации
- Системные метрики (`/api/metrics`) обновляются после каждого сбора снапшота
- Метрики отзывчивости (`/api/responsiveness`) обновляются после каждого сбора снапшота
- Список процессов (`/api/processes`) обновляется после каждого сбора снапшота
- Список групп приложений (`/api/appgroups`) обновляется после каждого сбора снапшота
- Конфигурация (`/api/config`) устанавливается при запуске демона и не изменяется во время работы
- Паттерны (`/api/patterns`) загружаются при запуске демона и не изменяются во время работы

## Обработка ошибок

Все endpoints возвращают статус `200 OK` даже если данные недоступны. В этом случае поле с данными будет равно `null`, а в ответе будет присутствовать поле `message` с объяснением причины отсутствия данных.

## Примеры использования

### Мониторинг работы демона

```bash
# Проверка работоспособности API
curl http://127.0.0.1:8080/health

# Получение статистики
curl http://127.0.0.1:8080/api/stats | jq

# Получение системных метрик
curl http://127.0.0.1:8080/api/metrics | jq

# Получение метрик отзывчивости
curl http://127.0.0.1:8080/api/responsiveness | jq

# Получение списка процессов
curl http://127.0.0.1:8080/api/processes | jq

# Получение списка групп приложений
curl http://127.0.0.1:8080/api/appgroups | jq

# Получение конфигурации демона
curl http://127.0.0.1:8080/api/config | jq

# Получение информации о классах QoS
curl http://127.0.0.1:8080/api/classes | jq

# Получение списка всех доступных endpoints
curl http://127.0.0.1:8080/api/endpoints | jq
```

### Автоматический мониторинг

```bash
#!/bin/bash
# Скрипт для периодического мониторинга

while true; do
    echo "=== $(date) ==="
    curl -s http://127.0.0.1:8080/api/stats | jq '.daemon_stats.total_iterations'
    sleep 5
done
```

### Практическое использование API

#### Мониторинг производительности системы

```bash
#!/bin/bash
# Скрипт для мониторинга производительности системы

while true; do
    clear
    echo "=== System Performance Monitor ==="
    echo "Timestamp: $(date)"
    echo
    
    # Получение системных метрик
    echo "CPU Usage:"
    curl -s http://127.0.0.1:8080/api/metrics | jq '.system_metrics.cpu_times'
    
    echo "Memory Usage:"
    curl -s http://127.0.0.1:8080/api/metrics | jq '.system_metrics.memory | {mem_used: (.mem_total_kb - .mem_available_kb)/1024/1024, mem_total: .mem_total_kb/1024/1024}'
    
    echo "Load Average:"
    curl -s http://127.0.0.1:8080/api/metrics | jq '.system_metrics.load_avg'
    
    echo "PSI Pressure:"
    curl -s http://127.0.0.1:8080/api/metrics | jq '.system_metrics.pressure'
    
    sleep 2
done
```

#### Мониторинг отзывчивости системы

```bash
#!/bin/bash
# Скрипт для мониторинга отзывчивости системы

while true; do
    clear
    echo "=== System Responsiveness Monitor ==="
    echo "Timestamp: $(date)"
    echo
    
    # Получение метрик отзывчивости
    curl -s http://127.0.0.1:8080/api/responsiveness | jq '.responsiveness_metrics'
    
    # Проверка статуса отзывчивости
    BAD_RESPONSIVENESS=$(curl -s http://127.0.0.1:8080/api/responsiveness | jq '.responsiveness_metrics.bad_responsiveness')
    RESPONSIVENESS_SCORE=$(curl -s http://127.0.0.1:8080/api/responsiveness | jq '.responsiveness_metrics.responsiveness_score')
    
    echo "Status: $([ "$BAD_RESPONSIVENESS" = "true" ] && echo "BAD" || echo "GOOD")"
    echo "Score: $RESPONSIVENESS_SCORE"
    
    sleep 1
done
```

#### Мониторинг процессов и групп приложений

```bash
#!/bin/bash
# Скрипт для мониторинга процессов и групп приложений

while true; do
    clear
    echo "=== Process and App Group Monitor ==="
    echo "Timestamp: $(date)"
    echo
    
    # Получение списка групп приложений
    echo "App Groups:"
    curl -s http://127.0.0.1:8080/api/appgroups | jq '.app_groups[] | {app_group_id, app_name, priority_class, total_cpu_share}'
    
    echo
    echo "Top Processes by CPU:"
    curl -s http://127.0.0.1:8080/api/processes | jq '.processes | sort_by(.cpu_share_1s) | reverse | .[0:5] | {pid, exe, cpu_share_1s, priority_class: .teacher_priority_class}'
    
    sleep 2
done
```

#### Управление конфигурацией через API

```bash
#!/bin/bash
# Скрипт для управления конфигурацией через API

# Получение текущей конфигурации
curl http://127.0.0.1:8080/api/config | jq '.config'

# Перезагрузка конфигурации из файла
curl -X POST http://127.0.0.1:8080/api/config/reload

# Проверка новой конфигурации
curl http://127.0.0.1:8080/api/config | jq '.config.thresholds'
```

#### Интеграция с Prometheus (пример экспортера)

```python
#!/usr/bin/env python3
# Простой экспортер метрик для Prometheus

import http.server
import socketserver
import requests
import json
import time

class SmoothTaskMetricsExporter(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/metrics':
            # Получение метрик из SmoothTask API
            try:
                response = requests.get('http://127.0.0.1:8080/api/metrics')
                metrics = response.json()
                
                # Форматирование метрик для Prometheus
                output = ""
                output += f"# HELP smoothtask_cpu_user CPU user time\n"
                output += f"# TYPE smoothtask_cpu_user gauge\n"
                output += f"smoothtask_cpu_user {metrics['system_metrics']['cpu_times']['user']}\n"
                
                output += f"# HELP smoothtask_cpu_system CPU system time\n"
                output += f"# TYPE smoothtask_cpu_system gauge\n"
                output += f"smoothtask_cpu_system {metrics['system_metrics']['cpu_times']['system']}\n"
                
                output += f"# HELP smoothtask_memory_used Memory used in MB\n"
                output += f"# TYPE smoothtask_memory_used gauge\n"
                mem_total = metrics['system_metrics']['memory']['mem_total_kb'] / 1024
                mem_available = metrics['system_metrics']['memory']['mem_available_kb'] / 1024
                output += f"smoothtask_memory_used {mem_total - mem_available}\n"
                
                output += f"# HELP smoothtask_load_avg_1 Load average 1 minute\n"
                output += f"# TYPE smoothtask_load_avg_1 gauge\n"
                output += f"smoothtask_load_avg_1 {metrics['system_metrics']['load_avg']['one']}\n"
                
                self.send_response(200)
                self.send_header('Content-Type', 'text/plain; version=0.0.4; charset=utf-8')
                self.end_headers()
                self.wfile.write(output.encode())
                
            except Exception as e:
                self.send_response(500)
                self.end_headers()
                self.wfile.write(f"Error: {str(e)}".encode())
        else:
            self.send_response(404)
            self.end_headers()

if __name__ == '__main__':
    PORT = 9090
    with socketserver.TCPServer(("", PORT), SmoothTaskMetricsExporter) as httpd:
        print(f"SmoothTask Prometheus Exporter running on port {PORT}")
        httpd.serve_forever()
```

#### Интеграция с Grafana (пример дашборда)

```json
{
  "title": "SmoothTask System Monitoring",
  "panels": [
    {
      "title": "CPU Usage",
      "type": "graph",
      "targets": [
        {
          "expr": "rate(smoothtask_cpu_user[1m])",
          "legendFormat": "User"
        },
        {
          "expr": "rate(smoothtask_cpu_system[1m])",
          "legendFormat": "System"
        }
      ]
    },
    {
      "title": "Memory Usage",
      "type": "graph",
      "targets": [
        {
          "expr": "smoothtask_memory_used",
          "legendFormat": "Used Memory"
        }
      ]
    },
    {
      "title": "Load Average",
      "type": "graph",
      "targets": [
        {
          "expr": "smoothtask_load_avg_1",
          "legendFormat": "1 Minute Load"
        }
      ]
    }
  ]
}
```

#### Мониторинг и алертинг

```bash
#!/bin/bash
# Скрипт для мониторинга и алертинга

# Пороговые значения
CPU_THRESHOLD=0.8
MEM_THRESHOLD=0.9
LOAD_THRESHOLD=2.0

while true; do
    # Получение метрик
    METRICS=$(curl -s http://127.0.0.1:8080/api/metrics)
    
    # Расчет использования CPU
    CPU_USER=$(echo $METRICS | jq '.system_metrics.cpu_times.user')
    CPU_SYSTEM=$(echo $METRICS | jq '.system_metrics.cpu_times.system')
    CPU_IDLE=$(echo $METRICS | jq '.system_metrics.cpu_times.idle')
    CPU_TOTAL=$((CPU_USER + CPU_SYSTEM + CPU_IDLE))
    CPU_USAGE=$(echo "scale=2; ($CPU_USER + $CPU_SYSTEM) / $CPU_TOTAL" | bc)
    
    # Расчет использования памяти
    MEM_TOTAL=$(echo $METRICS | jq '.system_metrics.memory.mem_total_kb')
    MEM_AVAILABLE=$(echo $METRICS | jq '.system_metrics.memory.mem_available_kb')
    MEM_USED=$(echo "scale=2; ($MEM_TOTAL - $MEM_AVAILABLE) / $MEM_TOTAL" | bc)
    
    # Получение load average
    LOAD_1=$(echo $METRICS | jq '.system_metrics.load_avg.one')
    
    # Проверка порогов
    if (( $(echo "$CPU_USAGE > $CPU_THRESHOLD" | bc -l) )); then
        echo "ALERT: High CPU usage: ${CPU_USAGE} (threshold: ${CPU_THRESHOLD})"
        # Здесь можно добавить отправку уведомления
    fi
    
    if (( $(echo "$MEM_USED > $MEM_THRESHOLD" | bc -l) )); then
        echo "ALERT: High memory usage: ${MEM_USED} (threshold: ${MEM_THRESHOLD})"
        # Здесь можно добавить отправку уведомления
    fi
    
    if (( $(echo "$LOAD_1 > $LOAD_THRESHOLD" | bc -l) )); then
        echo "ALERT: High load average: ${LOAD_1} (threshold: ${LOAD_THRESHOLD})"
        # Здесь можно добавить отправку уведомления
    fi
    
    sleep 10
done
```

#### Использование API для отладки

```bash
#!/bin/bash
# Скрипт для отладки проблем с производительностью

# Получение информации о процессах с высоким CPU
curl -s http://127.0.0.1:8080/api/processes | \
  jq '.processes | sort_by(.cpu_share_1s) | reverse | .[0:10] | {pid, exe, cpu_share_1s, nice, ionice_class, ionice_prio}'

# Получение информации о процессах с высоким I/O
curl -s http://127.0.0.1:8080/api/processes | \
  jq '.processes | sort_by(.io_read_bytes + .io_write_bytes) | reverse | .[0:10] | {pid, exe, io_read_bytes, io_write_bytes}'

# Получение информации о процессах с высоким использованием памяти
curl -s http://127.0.0.1:8080/api/processes | \
  jq '.processes | sort_by(.rss_mb) | reverse | .[0:10] | {pid, exe, rss_mb, swap_mb}'
```

#### Использование API для тестирования

```bash
#!/bin/bash
# Скрипт для тестирования API

# Тестирование всех endpoints
ENDPOINTS=(
  "/health"
  "/api/version"
  "/api/endpoints"
  "/api/stats"
  "/api/metrics"
  "/api/responsiveness"
  "/api/processes"
  "/api/appgroups"
  "/api/config"
  "/api/classes"
  "/api/patterns"
  "/api/system"
)

for endpoint in "${ENDPOINTS[@]}"; do
  echo "Testing $endpoint..."
  response=$(curl -s -w "%{http_code}" http://127.0.0.1:8080$endpoint)
  status_code=${response: -3}
  
  if [ "$status_code" -eq "200" ]; then
    echo "✓ $endpoint - OK (Status: $status_code)"
  else
    echo "✗ $endpoint - FAILED (Status: $status_code)"
  fi
done
```

## Безопасность

**Важно:** По умолчанию API сервер слушает только на `127.0.0.1` (localhost), что означает, что он доступен только с локальной машины. Это обеспечивает базовую безопасность.

Если необходимо сделать API доступным извне, измените адрес в конфигурации:

```yaml
paths:
  api_listen_addr: "0.0.0.0:8080"  # Доступно извне (не рекомендуется без дополнительной защиты)
```

**Рекомендации по безопасности:**

1. Используйте API только на localhost (по умолчанию)
2. Если необходим доступ извне, используйте reverse proxy (nginx, Caddy) с аутентификацией
3. Не используйте API на публичных серверах без дополнительной защиты
4. Рассмотрите возможность добавления TLS/HTTPS для защиты данных в будущем

---

### POST /api/notifications/test

Отправка тестового уведомления через систему уведомлений.

**Запрос:**
```bash
curl -X POST http://127.0.0.1:8080/api/notifications/test
```

**Ответ (успешная отправка):**
```json
{
  "status": "success",
  "message": "Test notification sent successfully",
  "notification": {
    "type": "info",
    "title": "Test Notification",
    "message": "This is a test notification from SmoothTask API",
    "details": "Sent via /api/notifications/test endpoint",
    "timestamp": "2025-01-01T12:00:00+00:00"
  },
  "backend": "stub"
}
```

**Ответ (ошибка отправки):**
```json
{
  "status": "error",
  "message": "Failed to send test notification: Notification backend error",
  "backend": "libnotify"
}
```

**Ответ (менеджер уведомлений недоступен):**
```json
{
  "status": "error",
  "message": "Notification manager not available (daemon may not be running or notifications not configured)",
  "backend": "none"
}
```

**Поля ответа:**
- `status` (string) - статус операции: "success" или "error"
- `message` (string) - описание результата операции
- `notification` (object, опционально) - информация об отправленном уведомлении
- `backend` (string) - используемый бэкенд уведомлений ("stub", "libnotify" или "none")

**Примечания:**
- Используется для проверки работоспособности системы уведомлений
- Отправляет тестовое уведомление с фиксированным текстом
- Тип уведомления: Info
- Требует наличия notification_manager в состоянии API
- Если уведомления отключены в конфигурации, уведомление не будет отправлено

**Статус коды:**
- `200 OK` - Успешный запрос
- `200 OK` с `status: "error"` - Ошибка отправки уведомления

---

### GET /api/notifications/status

Получение текущего состояния системы уведомлений.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/notifications/status
```

**Ответ (полная информация):**
```json
{
  "status": "ok",
  "notifications": {
    "config": {
      "enabled": true,
      "backend": "stub",
      "app_name": "SmoothTask",
      "min_level": "info"
    },
    "manager": {
      "enabled": true,
      "backend": "stub"
    },
    "available": true
  }
}
```

**Ответ (конфигурация недоступна):**
```json
{
  "status": "ok",
  "notifications": {
    "config": null,
    "manager": null,
    "available": false
  }
}
```

**Поля ответа:**
- `status` (string) - статус ответа (всегда "ok")
- `notifications` (object) - информация о системе уведомлений
  - `config` (object | null) - текущая конфигурация уведомлений
    - `enabled` (bool) - флаг включения уведомлений
    - `backend` (string) - тип бэкенда ("stub" или "libnotify")
    - `app_name` (string) - имя приложения для уведомлений
    - `min_level` (string) - минимальный уровень важности ("critical", "warning" или "info")
  - `manager` (object | null) - информация о менеджере уведомлений
    - `enabled` (bool) - флаг включения уведомлений в менеджере
    - `backend` (string) - используемый бэкенд
  - `available` (bool) - флаг доступности системы уведомлений

**Примечания:**
- Возвращает информацию о конфигурации и состоянии менеджера уведомлений
- Позволяет проверить, включены ли уведомления и какой бэкенд используется
- Полезно для отладки и мониторинга системы уведомлений

**Статус коды:**
- `200 OK` - запрос выполнен успешно

---

### POST /api/notifications/config

Изменение конфигурации уведомлений в runtime.

**Запрос:**
```bash
curl -X POST http://127.0.0.1:8080/api/notifications/config \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true,
    "backend": "libnotify",
    "app_name": "SmoothTask Test",
    "min_level": "info"
  }'
```

**Ответ (успешное обновление):**
```json
{
  "status": "success",
  "message": "Notification configuration updated successfully",
  "config": {
    "enabled": true,
    "backend": "libnotify",
    "app_name": "SmoothTask Test",
    "min_level": "info"
  }
}
```

**Ответ (конфигурация недоступна):**
```json
{
  "status": "error",
  "message": "Config not available (daemon may not be running or config not set)"
}
```

**Поля запроса (JSON):**
- `enabled` (bool, опционально) - флаг включения уведомлений
- `backend` (string, опционально) - тип бэкенда ("stub" или "libnotify")
- `app_name` (string, опционально) - имя приложения для уведомлений
- `min_level` (string, опционально) - минимальный уровень важности ("critical", "warning" или "info")

**Поля ответа:**
- `status` (string) - статус операции: "success" или "error"
- `message` (string) - описание результата операции
- `config` (object, опционально) - обновлённая конфигурация уведомлений

**Примечания:**
- Позволяет изменять параметры уведомлений без перезапуска демона
- Изменения применяются немедленно
- Можно обновлять отдельные параметры (частичное обновление)
- Требует наличия конфигурации в состоянии API
- Если параметр не указан в запросе, он остаётся без изменений
- При изменении параметра `enabled` обновляется как конфигурация, так и менеджер уведомлений

**Примеры использования:**

**Включение уведомлений:**
```bash
curl -X POST http://127.0.0.1:8080/api/notifications/config \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'
```

**Изменение бэкенда:**
```bash
curl -X POST http://127.0.0.1:8080/api/notifications/config \
  -H "Content-Type: application/json" \
  -d '{"backend": "libnotify"}'
```

**Изменение минимального уровня:**
```bash
curl -X POST http://127.0.0.1:8080/api/notifications/config \
  -H "Content-Type: application/json" \
  -d '{"min_level": "warning"}'
```

**Полное обновление конфигурации:**
```bash
curl -X POST http://127.0.0.1:8080/api/notifications/config \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true,
    "backend": "libnotify",
    "app_name": "SmoothTask Production",
    "min_level": "warning"
  }'
```

**Статус коды:**
- `200 OK` - Успешный запрос
- `200 OK` с `status: "error"` - Ошибка обновления конфигурации

---

## Примеры использования API уведомлений

### Проверка работоспособности системы уведомлений

```bash
# Отправить тестовое уведомление
curl -X POST http://127.0.0.1:8080/api/notifications/test

# Проверить статус системы уведомлений
curl http://127.0.0.1:8080/api/notifications/status | jq

# Включить уведомления
curl -X POST http://127.0.0.1:8080/api/notifications/config \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'

# Изменить бэкенд на libnotify
curl -X POST http://127.0.0.1:8080/api/notifications/config \
  -H "Content-Type: application/json" \
  -d '{"backend": "libnotify"}'

# Отправить ещё одно тестовое уведомление для проверки
curl -X POST http://127.0.0.1:8080/api/notifications/test
```

### Настройка уведомлений для production

```bash
# Настроить уведомления для production использования
curl -X POST http://127.0.0.1:8080/api/notifications/config \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": true,
    "backend": "libnotify",
    "app_name": "SmoothTask",
    "min_level": "warning"
  }'

# Проверить текущую конфигурацию
curl http://127.0.0.1:8080/api/notifications/status | jq '.notifications.config'
```

### Отладка проблем с уведомлениями

```bash
# Проверить текущий статус системы уведомлений
curl http://127.0.0.1:8080/api/notifications/status | jq

# Отправить тестовое уведомление для проверки
curl -X POST http://127.0.0.1:8080/api/notifications/test

# Проверить логи демона для деталей
journalctl -u smoothtaskd -n 50

# Включить более подробное логирование (уровень info)
curl -X POST http://127.0.0.1:8080/api/notifications/config \
  -H "Content-Type: application/json" \
  -d '{"min_level": "info"}'
```

---

## Интеграция с внешними системами

API уведомлений SmoothTask может быть интегрировано с внешними системами мониторинга и управления.

### Пример интеграции с Prometheus

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'smoothtask'
    metrics_path: '/api/metrics'
    static_configs:
      - targets: ['localhost:8080']
```

### Пример интеграции с Grafana

```json
// Grafana dashboard JSON
{
  "panels": [
    {
      "title": "SmoothTask Notifications",
      "type": "stat",
      "datasource": "prometheus",
      "targets": [
        {
          "expr": "smoothtask_notifications_enabled",
          "format": "time_series"
        }
      ]
    }
  ]
}
```

### Пример интеграции с системой мониторинга

```python
import requests
import json

def check_smoothtask_notifications():
    try:
        response = requests.get("http://127.0.0.1:8080/api/notifications/status")
        data = response.json()
        
        if data["status"] == "ok":
            notifications = data["notifications"]
            if notifications["available"]:
                config = notifications["config"]
                manager = notifications["manager"]
                
                print(f"Notifications enabled: {config['enabled']}")
                print(f"Backend: {config['backend']}")
                print(f"Min level: {config['min_level']}")
                print(f"Manager status: {manager['enabled']}")
                
                return True
            else:
                print("Notifications not available")
                return False
        else:
            print("Error checking notifications")
            return False
            
    except Exception as e:
        print(f"Error: {e}")
        return False

if __name__ == "__main__":
    check_smoothtask_notifications()
```

---

## Рекомендации по использованию

1. **Настройка уведомлений:**
   - Для production использования рекомендуется использовать бэкенд `libnotify`
   - Установите минимальный уровень `warning` для избежания информационного шума
   - Включайте уведомления только при необходимости мониторинга

2. **Тестирование:**
   - Используйте бэкенд `stub` для тестирования и разработки
   - Отправляйте тестовые уведомления для проверки работоспособности
   - Проверяйте статус системы уведомлений перед использованием

3. **Безопасность:**
   - API уведомлений доступен только на localhost по умолчанию
   - Не открывайте API на публичные интерфейсы без дополнительной защиты
   - Используйте HTTPS и аутентификацию при необходимости внешнего доступа

4. **Мониторинг:**
   - Интегрируйте API уведомлений с вашей системой мониторинга
   - Отслеживайте статус системы уведомлений для своевременного обнаружения проблем
   - Используйте тестовые уведомления для проверки работоспособности

5. **Отладка:**
   - Проверяйте логи демона для деталей о работе системы уведомлений
   - Используйте уровень `info` для более подробного логирования
   - Проверяйте статус системы уведомлений при возникновении проблем

---

## Будущие улучшения

В будущих версиях SmoothTask планируется:

1. **Расширение системы уведомлений:**
   - Добавление поддержки дополнительных бэкендов (email, webhook, etc.)
   - Реализация системы шаблонов для уведомлений
   - Добавление поддержки локализации уведомлений

2. **Улучшение API:**
   - Добавление эндпоинта для получения истории уведомлений
   - Реализация системы фильтрации и поиска уведомлений
   - Добавление поддержки пагинации для списка уведомлений

3. **Интеграция с внешними системами:**
   - Добавление поддержки отправки уведомлений в внешние системы (Slack, Telegram, etc.)
   - Реализация системы вебхуков для уведомлений
   - Добавление поддержки отправки уведомлений по email

4. **Улучшение безопасности:**
   - Добавление аутентификации и авторизации для API
   - Реализация системы ролей и разрешений
   - Добавление поддержки TLS/HTTPS для API

5. **Улучшение документации:**
   - Добавление примеров использования API на различных языках программирования
   - Реализация интерактивной документации (Swagger/OpenAPI)
   - Добавление руководств по интеграции с популярными системами мониторинга

---

## ML-классификатор процессов

SmoothTask поддерживает ML-классификацию процессов для более точного определения типов процессов и назначения приоритетов.

### Конфигурация ML-классификатора

ML-классификатор настраивается через поле `ml_classifier` в конфигурационном файле:

```yaml
ml_classifier:
  enabled: true
  model_path: "models/process_classifier.json"
  confidence_threshold: 0.7
  use_onnx: false
```

**Параметры:**
- `enabled`: Включить ML-классификатор (по умолчанию: `false`)
- `model_path`: Путь к файлу модели (JSON или ONNX)
- `confidence_threshold`: Минимальная уверенность для переопределения паттерн-классификации (0.0-1.0)
- `use_onnx`: Использовать ONNX Runtime для загрузки модели (по умолчанию: `false`)

### Примеры использования

**Загрузка модели CatBoost (JSON формат):**
```yaml
ml_classifier:
  enabled: true
  model_path: "models/process_classifier.json"
  confidence_threshold: 0.7
  use_onnx: false
```

**Загрузка модели ONNX:**
```yaml
ml_classifier:
  enabled: true
  model_path: "models/process_classifier.onnx"
  confidence_threshold: 0.8
  use_onnx: true
```

### Интеграция с системой классификации

ML-классификатор работает в дополнение к паттерн-базированной классификации:
1. Сначала применяется паттерн-классификация
2. Если ML-классификатор включен и его уверенность выше порога, его результат переопределяет паттерн-классификацию
3. Если ML-классификатор отключен или его уверенность ниже порога, используется результат паттерн-классификации

### Практические примеры использования

**Пример 1: Классификация аудио-приложения с высокой уверенностью**

```yaml
ml_classifier:
  enabled: true
  model_path: "models/process_classifier.json"
  confidence_threshold: 0.7
  use_onnx: false
```

В этом примере, если ML-классификатор определяет процесс как "audio" с уверенностью 0.85, этот тип будет использован вместо паттерн-классификации.

**Пример 2: Использование с автообновлением паттернов**

```yaml
ml_classifier:
  enabled: true
  model_path: "models/process_classifier.json"
  confidence_threshold: 0.75
  use_onnx: false

pattern_auto_update:
  enabled: true
  interval_sec: 60
  notify_on_update: true
```

Эта конфигурация позволяет ML-классификатору работать вместе с автообновлением паттернов, обеспечивая наилучшие результаты классификации.

### Обработка ошибок и fallback механизмы

ML-классификатор имеет встроенные механизмы обработки ошибок:
- Если модель не найдена, классификатор автоматически отключается
- Если уверенность ниже порога, используется паттерн-классификация
- Если ML-классификатор отключен, используется только паттерн-классификация

### Производительность и оптимизация

ML-классификатор оптимизирован для работы в реальном времени:
- Быстрое извлечение фич из процессов
- Эффективная загрузка и кэширование моделей
- Минимальное влияние на производительность системы

### Мониторинг и отладка

Для мониторинга работы ML-классификатора используйте API endpoints:
- `/api/stats` - статистика классификации
- `/api/processes` - текущие процессы с информацией о классификации
- `/api/logs` - логи работы классификатора

**Пример запроса для мониторинга:**

```bash
curl http://127.0.0.1:8080/api/stats
```

**Пример ответа:**

```json
{
  "status": "ok",
  "stats": {
    "total_processes": 150,
    "classified_by_ml": 45,
    "classified_by_patterns": 105,
    "ml_average_confidence": 0.82,
    "classification_time_ms": 15
  }
}
```

## Автообновление паттерн-базы

SmoothTask поддерживает автоматическое обновление паттерн-базы для поддержки новых приложений без перезапуска демона.

### Конфигурация автообновления

Автообновление настраивается через поле `pattern_auto_update` в конфигурационном файле:

```yaml
pattern_auto_update:
  enabled: true
  interval_sec: 60
  notify_on_update: true
```

**Параметры:**
- `enabled`: Включить автоматическое обновление паттернов (по умолчанию: `false`)
- `interval_sec`: Интервал проверки изменений в секундах (по умолчанию: `60`)
- `notify_on_update`: Уведомлять об обновлениях паттернов (по умолчанию: `false`)

### Примеры использования

**Базовая конфигурация:**
```yaml
pattern_auto_update:
  enabled: true
  interval_sec: 30
  notify_on_update: true
```

**Отключение автообновления:**
```yaml
pattern_auto_update:
  enabled: false
```

### Мониторинг изменений

PatternWatcher отслеживает изменения в директории с паттернами:
- Обнаружение новых файлов паттернов
- Обнаружение изменений в существующих файлах
- Обнаружение удаленных файлов
- Автоматическая перезагрузка паттерн-базы при обнаружении изменений

### Интеграция с системой классификации

Автообновление паттернов интегрировано с основным циклом классификации:
1. PatternWatcher мониторит директорию с паттернами
2. При обнаружении изменений выполняется перезагрузка паттерн-базы
3. Новые паттерны становятся доступны для классификации без перезапуска демона
4. При включенных уведомлениях, пользователь получает информацию об обновлении

## Мониторинг производительности приложений

SmoothTask предоставляет расширенные метрики производительности для мониторинга отзывчивости и ресурсоемкости приложений.

### Метрики производительности

Модуль `app_performance` собирает следующие метрики:
- Задержка отклика приложения (мс)
- FPS для графических приложений
- Использование CPU на уровне процесса (%)
- Использование памяти на уровне процесса (MB)
- Количество активных потоков
- Время работы процесса

### Примеры использования

**Получение метрик производительности через API:**
```bash
curl http://127.0.0.1:8080/api/metrics/app_performance
```

**Интеграция с Prometheus:**
```yaml
# В конфигурации Prometheus
scrape_configs:
  - job_name: 'smoothtask'
    static_configs:
      - targets: ['localhost:8080']
```

**Пример 1: Мониторинг производительности конкретного приложения**

```bash
#!/bin/bash
# Мониторинг производительности Firefox с использованием SmoothTask API

APP_NAME="firefox"
API_URL="http://127.0.0.1:8080/api/metrics/app_performance"

# Получение метрик производительности
PERF_DATA=$(curl -s "$API_URL")

# Анализ данных для Firefox
FIREFOX_PERF=$(echo "$PERF_DATA" | jq --arg app "$APP_NAME" '.app_performance | .[] | select(.app_name | contains($app))')

if [[ -n "$FIREFOX_PERF" ]]; then
    RESPONSE_TIME=$(echo "$FIREFOX_PERF" | jq '.response_time_ms')
    CPU_USAGE=$(echo "$FIREFOX_PERF" | jq '.cpu_usage_percent')
    MEM_USAGE=$(echo "$FIREFOX_PERF" | jq '.memory_usage_mb')
    
    echo "=== $APP_NAME Performance ==="
    echo "Response Time: ${RESPONSE_TIME} ms"
    echo "CPU Usage: ${CPU_USAGE}%"
    echo "Memory Usage: ${MEM_USAGE} MB"
    
    # Проверка на критическое состояние
    if (( $(echo "$RESPONSE_TIME > 100.0" | bc -l) )); then
        echo "WARNING: High response time for $APP_NAME!" | logger -t smoothtask-perf
    fi
else
    echo "No performance data found for $APP_NAME"
fi
```

**Пример 2: Комплексный мониторинг производительности системы**

```bash
#!/bin/bash
# Комплексный скрипт мониторинга производительности с использованием SmoothTask API

API_URL="http://127.0.0.1:8080/api/metrics/app_performance"
SYSTEM_API="http://127.0.0.1:8080/api/metrics"

# Получение метрик производительности приложений
APP_PERF=$(curl -s "$API_URL")

# Получение системных метрик
SYSTEM_PERF=$(curl -s "$SYSTEM_API")

# Анализ производительности приложений
TOTAL_APPS=$(echo "$APP_PERF" | jq '.app_performance | length')
HIGH_RESPONSE_APPS=0
HIGH_CPU_APPS=0

for app in $(echo "$APP_PERF" | jq -c '.app_performance | .[]'); do
    RESPONSE_TIME=$(echo "$app" | jq '.response_time_ms')
    CPU_USAGE=$(echo "$app" | jq '.cpu_usage_percent')
    APP_NAME=$(echo "$app" | jq -r '.app_name')
    
    if (( $(echo "$RESPONSE_TIME > 50.0" | bc -l) )); then
        HIGH_RESPONSE_APPS=$((HIGH_RESPONSE_APPS + 1))
        echo "WARNING: High response time for $APP_NAME: ${RESPONSE_TIME} ms"
    fi
    
    if (( $(echo "$CPU_USAGE > 80.0" | bc -l) )); then
        HIGH_CPU_APPS=$((HIGH_CPU_APPS + 1))
        echo "WARNING: High CPU usage for $APP_NAME: ${CPU_USAGE}%"
    fi
done

# Анализ системной производительности
CPU_USAGE=$(echo "$SYSTEM_PERF" | jq '.cpu_usage.total')
MEM_USAGE=$(echo "$SYSTEM_PERF" | jq '.memory.used_kb / 1024')

# Вывод комплексного отчета
echo "=== System Performance Report ==="
echo "Total Applications: $TOTAL_APPS"
echo "High Response Time Apps: $HIGH_RESPONSE_APPS"
echo "High CPU Usage Apps: $HIGH_CPU_APPS"
echo "System CPU Usage: ${CPU_USAGE}%"
echo "System Memory Usage: ${MEM_USAGE} MB"

# Проверка на критическое состояние системы
if (( HIGH_RESPONSE_APPS > 3 )); then
    echo "CRITICAL: Multiple applications with high response time!" | logger -t smoothtask-perf
fi

if (( HIGH_CPU_APPS > 2 )); then
    echo "CRITICAL: Multiple applications with high CPU usage!" | logger -t smoothtask-perf
fi
```

**Пример 3: Интеграция с Grafana для визуализации производительности**

```yaml
# Конфигурация источника данных Grafana для SmoothTask API
apiVersion: 1

datasources:
  - name: SmoothTask
    type: json-api
    access: proxy
    url: http://localhost:8080
    jsonData:
      tlsSkipVerify: true
      oauthPassThru: false
    secureJsonData:
      # Если требуется аутентификация
      # basicAuthPassword: "your_password"
```

**Пример 4: Создание дашборда Grafana для мониторинга производительности**

```json
{
  "title": "SmoothTask Performance Dashboard",
  "panels": [
    {
      "title": "Application Response Times",
      "type": "timeseries",
      "datasource": "SmoothTask",
      "targets": [
        {
          "refId": "A",
          "url": "/api/metrics/app_performance",
          "query": "$.app_performance[*].response_time_ms",
          "format": "time_series",
          "legendFormat": "{{app_name}}"
        }
      ]
    },
    {
      "title": "Application CPU Usage",
      "type": "timeseries",
      "datasource": "SmoothTask",
      "targets": [
        {
          "refId": "B",
          "url": "/api/metrics/app_performance",
          "query": "$.app_performance[*].cpu_usage_percent",
          "format": "time_series",
          "legendFormat": "{{app_name}}"
        }
      ]
    },
    {
      "title": "System Overview",
      "type": "stat",
      "datasource": "SmoothTask",
      "targets": [
        {
          "refId": "C",
          "url": "/api/metrics",
          "query": "$.cpu_usage.total",
          "format": "time_series"
        }
      ]
    }
  ]
}
```

**Пример 5: Автоматическое оповещение о проблемах производительности**

```bash
#!/bin/bash
# Скрипт для автоматического оповещения о проблемах производительности

API_URL="http://127.0.0.1:8080/api/metrics/app_performance"
THRESHOLD_RESPONSE=100  # ms
THRESHOLD_CPU=90        # %

# Получение метрик производительности
PERF_DATA=$(curl -s "$API_URL")

# Проверка каждого приложения
for app in $(echo "$PERF_DATA" | jq -c '.app_performance | .[]'); do
    APP_NAME=$(echo "$app" | jq -r '.app_name')
    RESPONSE_TIME=$(echo "$app" | jq '.response_time_ms')
    CPU_USAGE=$(echo "$app" | jq '.cpu_usage_percent')
    
    # Проверка порогов
    if (( $(echo "$RESPONSE_TIME > $THRESHOLD_RESPONSE" | bc -l) )); then
        MESSAGE="ALERT: High response time for $APP_NAME: ${RESPONSE_TIME} ms"
        echo "$MESSAGE" | logger -t smoothtask-alert
        
        # Отправка уведомления через API (если настроено)
        curl -X POST "http://127.0.0.1:8080/api/notifications" \
            -H "Content-Type: application/json" \
            -d "{\"level\": \"warning\", \"message\": \"$MESSAGE\"}"
    fi
    
    if (( $(echo "$CPU_USAGE > $THRESHOLD_CPU" | bc -l) )); then
        MESSAGE="ALERT: High CPU usage for $APP_NAME: ${CPU_USAGE}%"
        echo "$MESSAGE" | logger -t smoothtask-alert
        
        # Отправка уведомления через API
        curl -X POST "http://127.0.0.1:8080/api/notifications" \
            -H "Content-Type: application/json" \
            -d "{\"level\": \"warning\", \"message\": \"$MESSAGE\"}"
    fi
done
```

**Пример 6: Интеграция с системой мониторинга Zabbix**

```bash
#!/bin/bash
# Скрипт для интеграции SmoothTask с Zabbix

API_URL="http://127.0.0.1:8080/api/metrics/app_performance"

# Получение метрик производительности
PERF_DATA=$(curl -s "$API_URL")

# Экспорт метрик в формате Zabbix
echo "$PERF_DATA" | jq -r '.app_performance | .[] | "smoothtask.app.perf[\" + .app_name + \"].response_time \" + (.response_time_ms | tostring)'
echo "$PERF_DATA" | jq -r '.app_performance | .[] | "smoothtask.app.perf[\" + .app_name + \"].cpu_usage \" + (.cpu_usage_percent | tostring)'
echo "$PERF_DATA" | jq -r '.app_performance | .[] | "smoothtask.app.perf[\" + .app_name + \"].memory_usage \" + (.memory_usage_mb | tostring)'
```

## Заключение

SmoothTask предоставляет мощный инструмент для мониторинга и управления системой. С помощью API и конфигурации вы можете:

- Проверять работоспособность системы
- Использовать ML-классификацию для более точного определения типов процессов
- Автоматически обновлять паттерн-базу без перезапуска демона
- Мониторить производительность приложений на уровне отдельных процессов
- Интегрировать систему с внешними инструментами мониторинга
- Настраивать уведомления и оповещения

Новые функции ML-классификации, автообновления паттернов и мониторинга производительности приложений значительно расширяют возможности SmoothTask по оптимизации работы системы и обеспечению лучшего пользовательского опыта.
