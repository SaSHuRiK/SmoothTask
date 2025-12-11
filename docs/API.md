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

## Обновление данных

Данные в API обновляются при каждой итерации демона (согласно `polling_interval_ms` в конфигурации). Это означает, что:

- Статистика демона (`/api/stats`) обновляется после каждой итерации
- Системные метрики (`/api/metrics`) обновляются после каждого сбора снапшота
- Список процессов (`/api/processes`) обновляется после каждого сбора снапшота
- Список групп приложений (`/api/appgroups`) обновляется после каждого сбора снапшота

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

# Получение списка процессов
curl http://127.0.0.1:8080/api/processes | jq

# Получение списка групп приложений
curl http://127.0.0.1:8080/api/appgroups | jq
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
