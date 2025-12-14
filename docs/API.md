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

### Конфигурация eBPF

Для работы с сетевыми соединениями через eBPF, необходимо настроить соответствующие параметры в конфигурационном файле:

```yaml
metrics:
  ebpf:
    enable_network_connections: true  # Включить мониторинг сетевых соединений
    enable_network_monitoring: true    # Включить общий мониторинг сети
    enable_high_performance_mode: true # Использовать оптимизированные eBPF программы
    
    # Конфигурация фильтрации (опционально)
    filter_config:
      enable_kernel_filtering: true
      active_connections_threshold: 10  # Минимальное количество активных соединений для уведомлений
      network_traffic_threshold: 1024   # Минимальный трафик в байтах для отслеживания
      
      # Фильтрация по протоколам (опционально)
      enable_network_protocol_filtering: false
      filtered_network_protocols: [6, 17]  # TCP=6, UDP=17
      
      # Фильтрация по диапазону портов (опционально)
      enable_port_range_filtering: false
      min_port: 1024
      max_port: 65535
```

**Требования для eBPF:**
- Ядро Linux версии 5.4 или новее
- Права CAP_BPF или запуск от root
- Установленные заголовки ядра для компиляции eBPF программ
- Библиотека `libbpf` и `libbpf-rs`

**Ограничения:**
- eBPF программы требуют компиляции при первом запуске
- Максимальное количество отслеживаемых соединений: 2048 (задается в eBPF программе)
- Данные собираются только для активных соединений

**Рекомендации:**
- Для высоконагруженных систем используйте `enable_high_performance_mode: true`
- Настраивайте фильтрацию для уменьшения накладных расходов
- Мониторьте использование памяти eBPF карт через `/api/stats`

## Унифицированная система обработки ошибок

API сервер использует улучшенную унифицированную систему обработки ошибок с использованием различных HTTP статусов и структурированных JSON ответов. Все ошибки теперь обрабатываются через единый тип `ApiError`, который обеспечивает консистентность и предсказуемость.

### Типы ошибок

#### 1. Ошибки валидации (400 Bad Request)

Возникают при некорректных входных данных:

```json
{
  "status": "error",
  "error": "invalid_input",
  "message": "Invalid PID value: -1. PID must be a positive integer",
  "timestamp": "2023-12-12T12:34:56.789Z",
  "details": {
    "type": "validation",
    "suggestion": "Check your request parameters and try again"
  }
}
```

**Примеры:**
- Недопустимый PID (<= 0 или слишком большой)
- Пустой или слишком длинный идентификатор группы приложений
- Некорректный формат параметров запроса

#### 2. Ошибки "Не найдено" (404 Not Found)

Возникают при запросе несуществующих ресурсов:

```json
{
  "status": "error",
  "error": "not_found",
  "message": "Process with PID 12345 not found (available processes: 42)",
  "timestamp": "2023-12-12T12:34:56.789Z",
  "details": {
    "type": "not_found",
    "suggestion": "Check the resource identifier and try again"
  }
}
```

**Примеры:**
- Процесс с указанным PID не найден
- Группа приложений с указанным ID не существует
- Запрошенный ресурс недоступен

#### 3. Ошибки недоступности сервиса (503 Service Unavailable)

Возникают когда демон не запущен или данные еще не собраны:

```json
{
  "status": "error",
  "error": "not_available",
  "message": "Processes data not available - daemon may not be running or no processes collected yet",
  "timestamp": "2023-12-12T12:34:56.789Z",
  "details": {
    "type": "service_unavailable",
    "suggestion": "Check if the daemon is running and has collected data"
  }
}
```

**Примеры:**
- Демон не запущен
- Данные еще не собраны
- Компонент временно недоступен

#### 4. Ошибки доступа к данным (500 Internal Server Error)

Возникают при проблемах доступа к данным:

```json
{
  "status": "error",
  "error": "data_access_error",
  "message": "Failed to access process data: permission denied",
  "timestamp": "2023-12-12T12:34:56.789Z",
  "details": {
    "type": "data_access",
    "suggestion": "Check if the daemon is running and has collected data"
  }
}
```

**Примеры:**
- Ошибки доступа к данным процессов
- Проблемы с чтением системных метрик
- Сбои при работе с кэшем

#### 5. Ошибки конфигурации (500 Internal Server Error)

Возникают при проблемах с конфигурацией:

```json
{
  "status": "error",
  "error": "configuration_error",
  "message": "Invalid configuration: missing required field 'api_listen_addr'",
  "timestamp": "2023-12-12T12:34:56.789Z",
  "details": {
    "type": "configuration",
    "suggestion": "Check your configuration file and restart the daemon"
  }
}
```

**Примеры:**
- Неправильная конфигурация API
- Отсутствующие обязательные поля
- Некорректные значения параметров

#### 6. Внутренние ошибки сервера (500 Internal Server Error)

Возникают при внутренних сбоях:

```json
{
  "status": "error",
  "error": "internal_error",
  "message": "Failed to access system metrics: permission denied",
  "timestamp": "2023-12-12T12:34:56.789Z",
  "details": {
    "type": "internal",
    "suggestion": "This is a bug, please report it with logs"
  }
}
```

**Примеры:**
- Ошибки доступа к системным ресурсам
- Сбои при обработке данных
- Внутренние исключения

### Улучшения системы обработки ошибок

Новая унифицированная система обработки ошибок включает следующие улучшения:

1. **Консистентные HTTP статусы**: Каждый тип ошибки соответствует определенному HTTP статусу
2. **Структурированные JSON ответы**: Все ошибки возвращают одинаковый формат JSON
3. **Полезные предложения**: Каждая ошибка содержит поле `suggestion` с рекомендациями по устранению
4. **Типизированные ошибки**: Поле `error` содержит машинно-читаемый идентификатор типа ошибки
5. **Дополнительный контекст**: Поле `details` предоставляет дополнительную информацию для отладки

### Graceful Degradation

Когда данные временно недоступны, API возвращает статус `"ok"` с пустыми данными вместо ошибки:

```json
{
  "status": "ok",
  "system_metrics": null,
  "message": "System metrics not available (daemon may not be running or no metrics collected yet)"
}
```

Это позволяет клиентам продолжать работу даже при временной недоступности данных.

### Рекомендации по обработке ошибок

1. **Проверяйте HTTP статус код** для определения типа ошибки
2. **Анализируйте поле `error`** для точного определения проблемы
3. **Используйте поле `details`** для получения дополнительной информации
4. **Следуйте предложениям** из поля `suggestion` для устранения проблем
5. **Обрабатывайте graceful degradation** для временной недоступности данных
6. **Логируйте ошибки** для отладки и мониторинга
7. **Используйте машинно-читаемые идентификаторы** для автоматической обработки ошибок

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

### GET /api/network/connections

Получение информации о текущих сетевых соединениях, собранных через eBPF.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/network/connections
```

**Успешный ответ:**
```json
{
  "status": "ok",
  "timestamp": "2025-01-01T12:00:00+00:00",
  "active_connections": 42,
  "total_connections": 150,
  "connections": [
    {
      "src_ip": "192.168.1.100",
      "dst_ip": "93.184.216.34",
      "src_port": 54321,
      "dst_port": 443,
      "protocol": "TCP",
      "state": 1,
      "packets": 1250,
      "bytes": 875000,
      "start_time": 1672531200000000000,
      "last_activity": 1672531260000000000,
      "active": true
    },
    {
      "src_ip": "192.168.1.100",
      "dst_ip": "8.8.8.8",
      "src_port": 45678,
      "dst_port": 53,
      "protocol": "UDP",
      "state": 0,
      "packets": 45,
      "bytes": 3200,
      "start_time": 1672531180000000000,
      "last_activity": 1672531195000000000,
      "active": false
    }
  ],
  "network_stats": {
    "packets": 15000,
    "bytes": 10500000
  }
}
```

**Ошибка (если eBPF не доступен):**
```json
{
  "status": "error",
  "error": "Failed to collect network connection metrics: eBPF not available",
  "timestamp": "2025-01-01T12:00:00+00:00"
}
```

**Поля ответа:**
- `status`: Статус запроса (`ok` или `error`)
- `timestamp`: Время генерации ответа в формате RFC3339
- `active_connections`: Количество активных соединений (активность в последние 30 секунд)
- `total_connections`: Общее количество соединений в ответе
- `connections`: Массив объектов с информацией о соединениях:
  - `src_ip`: IP адрес источника
  - `dst_ip`: IP адрес назначения
  - `src_port`: Порт источника
  - `dst_port`: Порт назначения
  - `protocol`: Протокол (`TCP`, `UDP`, или `Unknown(X)`)
  - `state`: Состояние соединения (зависит от протокола)
  - `packets`: Количество пакетов
  - `bytes`: Количество байт
  - `start_time`: Время начала соединения в наносекундах
  - `last_activity`: Время последней активности в наносекундах
  - `active`: Флаг активности (true если активность была в последние 30 секунд)
- `network_stats`: Общая статистика сети:
  - `packets`: Общее количество пакетов
  - `bytes`: Общее количество байт

**Требования:**
- eBPF должен быть доступен и настроен
- Демон должен быть запущен с флагом `enable_network_connections: true`
- Требуются права CAP_BPF или root для работы eBPF

**Статус коды:**
- `200 OK` - Успешный запрос
- `500 Internal Server Error` - Ошибка при сборе метрик

---

### GET /api/cpu/temperature

Получение информации о температуре CPU, собранной через eBPF.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/cpu/temperature
```

**Успешный ответ:**
```json
{
  "status": "ok",
  "timestamp": "2025-01-01T12:00:00+00:00",
  "system_status": "normal",
  "average_temperature_celsius": 45,
  "max_temperature_celsius": 55,
  "cpu_count": 8,
  "temperature_details": [
    {
      "cpu_id": 0,
      "temperature_celsius": 45,
      "max_temperature_celsius": 85,
      "critical_temperature_celsius": 95,
      "timestamp": 1672531200000000000,
      "update_count": 1250,
      "error_count": 0,
      "status": "normal"
    },
    {
      "cpu_id": 1,
      "temperature_celsius": 48,
      "max_temperature_celsius": 85,
      "critical_temperature_celsius": 95,
      "timestamp": 1672531200000000000,
      "update_count": 1250,
      "error_count": 0,
      "status": "normal"
    }
  ],
  "recommendations": "System temperature is normal."
}
```

**Ответ при отсутствии метрик:**
```json
{
  "status": "error",
  "error": "Metrics collector not available",
  "timestamp": "2025-01-01T12:00:00+00:00"
}
```

**Поля ответа:**
- `status` (string) - Статус ответа ("ok" или "error")
- `timestamp` (string) - Временная метка ответа в формате RFC3339
- `system_status` (string) - Общий статус системы ("normal", "warning", "critical")
- `average_temperature_celsius` (integer) - Средняя температура CPU по всем ядрам
- `max_temperature_celsius` (integer) - Максимальная температура CPU среди всех ядер
- `cpu_count` (integer) - Количество CPU ядер с доступными метриками
- `temperature_details` (array) - Детализированная информация по каждому CPU ядру
- `recommendations` (string) - Рекомендации по текущему состоянию температуры

**Поля `temperature_details`:**
- `cpu_id` (integer) - Идентификатор CPU ядра
- `temperature_celsius` (integer) - Текущая температура CPU в градусах Цельсия
- `max_temperature_celsius` (integer) - Максимальная температура CPU в градусах Цельсия
- `critical_temperature_celsius` (integer) - Критическая температура CPU в градусах Цельсия
- `timestamp` (integer) - Временная метка последнего обновления в наносекундах
- `update_count` (integer) - Количество обновлений температуры
- `error_count` (integer) - Количество ошибок при сборе температуры
- `status` (string) - Статус температуры ("normal", "warning", "critical")

**Статус коды:**
- `200 OK` - запрос выполнен успешно

**Ошибки:**
- `error` (string) - Сообщение об ошибке, если метрики недоступны
- `timestamp` (string) - Временная метка ответа

**Примечания:**
- Требуется включенный мониторинг температуры CPU в конфигурации eBPF (`enable_cpu_temperature_monitoring: true`)
- В тестовой среде без реальной eBPF поддержки значения температуры могут быть по умолчанию
- Статус "warning" устанавливается при температуре >= 85°C, "critical" при температуре >= 95°C
- Детализированная информация доступна только при успешном сборе метрик с каждого CPU ядра

---

### GET /api/health

Получение расширенной информации о состоянии демона, включая время работы, статус компонентов и метрики производительности.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/health
```

**Ответ:**
```json
{
  "status": "ok",
  "service": "smoothtaskd",
  "uptime_seconds": 12345,
  "components": {
    "daemon_stats": true,
    "system_metrics": true,
    "processes": true,
    "app_groups": true,
    "config": true,
    "pattern_database": true
  },
  "performance": {
    "total_requests": 42,
    "cache_hit_rate": 75.0,
    "average_processing_time_us": 123.45
  },
  "timestamp": "2023-12-12T12:34:56.789Z"
}
```

**Поля ответа:**
- `status` (string): Статус работы демона (`"ok"` или `"error"`)
- `service` (string): Название сервиса (`"smoothtaskd"`)
- `uptime_seconds` (number): Время работы демона в секундах
- `components` (object): Статус доступности основных компонентов:
  - `daemon_stats` (boolean): Доступна ли статистика демона
  - `system_metrics` (boolean): Доступны ли системные метрики
  - `processes` (boolean): Доступны ли данные о процессах
  - `app_groups` (boolean): Доступны ли данные о группах приложений
  - `config` (boolean): Доступна ли конфигурация
  - `pattern_database` (boolean): Доступна ли база данных паттернов
- `performance` (object): Метрики производительности API:
  - `total_requests` (number): Общее количество обработанных запросов
  - `cache_hit_rate` (number): Процент кэш-хитов (0-100)
  - `average_processing_time_us` (number): Среднее время обработки запроса в микросекундах
- `timestamp` (string): Временная метка ответа в формате RFC3339

**Статус коды:**
- `200 OK` - Демон работает нормально
- `500 Internal Server Error` - Ошибка при обработке запроса

**Использование:**
Этот endpoint полезен для мониторинга состояния демона и диагностики проблем. Он предоставляет более детальную информацию, чем базовый `/health` endpoint.

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

### GET /api/cache/stats

Получение статистики кэша метрик процессов.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/cache/stats
```

**Ответ:**
```json
{
  "status": "ok",
  "cache_stats": {
    "total_entries": 150,
    "active_entries": 120,
    "stale_entries": 30,
    "max_capacity": 1000,
    "cache_ttl_seconds": 300,
    "average_age_seconds": 123.45,
    "hit_rate": 75.0,
    "utilization_rate": 15.0
  },
  "timestamp": "2023-12-12T12:34:56.789Z"
}
```

**Поля:**
- `status` (string) - статус ответа (всегда "ok")
- `cache_stats` (object) - статистика кэша:
  - `total_entries` (number) - общее количество записей в кэше
  - `active_entries` (number) - количество актуальных записей
  - `stale_entries` (number) - количество устаревших записей
  - `max_capacity` (number) - максимальная емкость кэша
  - `cache_ttl_seconds` (number) - время жизни кэша в секундах
  - `average_age_seconds` (number) - средний возраст записей в секундах
  - `hit_rate` (number) - процент попаданий в кэш (0-100)
  - `utilization_rate` (number) - процент использования кэша (0-100)
- `timestamp` (string) - временная метка ответа в формате RFC3339

**Статус коды:**
- `200 OK` - запрос выполнен успешно

**Использование:**
Этот endpoint полезен для мониторинга состояния кэша процессов и диагностики проблем с производительностью. Он предоставляет детальную информацию о текущем состоянии кэша, что помогает оптимизировать настройки кэширования.

---

### POST /api/cache/clear

Очистка кэша метрик процессов.

**Запрос:**
```bash
curl -X POST http://127.0.0.1:8080/api/cache/clear
```

**Ответ:**
```json
{
  "status": "success",
  "message": "Process cache cleared successfully",
  "cleared_entries": 150,
  "previous_stats": {
    "total_entries": 150,
    "active_entries": 120,
    "stale_entries": 30
  },
  "current_stats": {
    "total_entries": 0,
    "active_entries": 0,
    "stale_entries": 0
  },
  "timestamp": "2023-12-12T12:34:56.789Z"
}
```

**Поля:**
- `status` (string) - статус операции (всегда "success")
- `message` (string) - сообщение об успешном выполнении
- `cleared_entries` (number) - количество удаленных записей
- `previous_stats` (object) - статистика кэша до очистки
- `current_stats` (object) - статистика кэша после очистки
- `timestamp` (string) - временная метка ответа в формате RFC3339

**Статус коды:**
- `200 OK` - операция выполнена успешно

**Использование:**
Этот endpoint используется для принудительной очистки кэша процессов. Это может быть полезно при отладке, тестировании или когда нужно обеспечить получение свежих данных о процессах.

---

### GET /api/cache/config

Получение текущей конфигурации кэша метрик процессов.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/cache/config
```

**Ответ:**
```json
{
  "status": "ok",
  "cache_config": {
    "cache_ttl_seconds": 300,
    "max_cached_processes": 1000,
    "enable_caching": true,
    "enable_parallel_processing": true,
    "max_parallel_threads": null
  },
  "timestamp": "2023-12-12T12:34:56.789Z"
}
```

**Поля:**
- `status` (string) - статус ответа (всегда "ok")
- `cache_config` (object) - текущая конфигурация кэша:
  - `cache_ttl_seconds` (number) - время жизни кэша в секундах
  - `max_cached_processes` (number) - максимальное количество кэшируемых процессов
  - `enable_caching` (boolean) - включено ли кэширование
  - `enable_parallel_processing` (boolean) - включена ли параллельная обработка
  - `max_parallel_threads` (number/null) - максимальное количество параллельных потоков
- `timestamp` (string) - временная метка ответа в формате RFC3339

**Статус коды:**
- `200 OK` - запрос выполнен успешно

**Использование:**
Этот endpoint позволяет получить текущие настройки кэша процессов. Это полезно для мониторинга конфигурации и отладки проблем с производительностью.

---

### POST /api/cache/config

Обновление конфигурации кэша метрик процессов.

**Запрос:**
```bash
curl -X POST "http://127.0.0.1:8080/api/cache/config" \
  -H "Content-Type: application/json" \
  -d '{"cache_ttl_seconds": 30, "max_cached_processes": 5000}'
```

**Параметры запроса (JSON):**
- `cache_ttl_seconds` (опционально, number) - новое время жизни кэша в секундах
- `max_cached_processes` (опционально, number) - новое максимальное количество кэшируемых процессов
- `enable_caching` (опционально, boolean) - включить/отключить кэширование
- `enable_parallel_processing` (опционально, boolean) - включить/отключить параллельную обработку
- `max_parallel_threads` (опционально, number) - максимальное количество параллельных потоков

**Ответ:**
```json
{
  "status": "success",
  "message": "Process cache configuration updated successfully",
  "cache_config": {
    "cache_ttl_seconds": 30,
    "max_cached_processes": 5000,
    "enable_caching": true,
    "enable_parallel_processing": true,
    "max_parallel_threads": null
  },
  "timestamp": "2023-12-12T12:34:56.789Z"
}
```

**Поля:**
- `status` (string) - статус операции (всегда "success")
- `message` (string) - сообщение об успешном выполнении
- `cache_config` (object) - обновленная конфигурация кэша
- `timestamp` (string) - временная метка ответа в формате RFC3339

**Статус коды:**
- `200 OK` - операция выполнена успешно

**Использование:**
Этот endpoint позволяет динамически изменять конфигурацию кэша процессов без перезапуска демона. Это полезно для оптимизации производительности в реальном времени и адаптации к изменяющимся условиям работы системы.

**Примеры:**

Обновление TTL кэша:
```bash
curl -X POST "http://127.0.0.1:8080/api/cache/config" \
  -H "Content-Type: application/json" \
  -d '{"cache_ttl_seconds": 30}'
```

Обновление максимального количества процессов:
```bash
curl -X POST "http://127.0.0.1:8080/api/cache/config" \
  -H "Content-Type: application/json" \
  -d '{"max_cached_processes": 5000}'
```

Отключение кэширования:
```bash
curl -X POST "http://127.0.0.1:8080/api/cache/config" \
  -H "Content-Type: application/json" \
  -d '{"enable_caching": false}'
```

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
      "path": "/api/network/connections",
      "method": "GET",
      "description": "Получение информации о текущих сетевых соединениях"
    },
    {
      "path": "/api/cpu/temperature",
      "method": "GET",
      "description": "Получение информации о температуре CPU"
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
  "count": 14
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

### GET /api/processes/gpu

Получение информации об использовании GPU процессами. Требует включения `enable_process_gpu_monitoring` в конфигурации eBPF.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/processes/gpu
```

**Успешный ответ (если мониторинг GPU доступен):**
```json
{
  "status": "ok",
  "process_gpu": [
    {
      "pid": 1234,
      "tgid": 1234,
      "gpu_time_ns": 1500000000,
      "memory_usage_bytes": 268435456,
      "compute_units_used": 8,
      "last_update_ns": 1234567890123456789,
      "gpu_id": 0,
      "temperature_celsius": 55,
      "name": "firefox",
      "gpu_usage_percent": 45.2
    },
    {
      "pid": 5678,
      "tgid": 5678,
      "gpu_time_ns": 400000000,
      "memory_usage_bytes": 67108864,
      "compute_units_used": 2,
      "last_update_ns": 1234567890123456789,
      "gpu_id": 0,
      "temperature_celsius": 48,
      "name": "blender",
      "gpu_usage_percent": 12.8
    }
  ],
  "count": 2,
  "total_gpu_time_ns": 1900000000,
  "total_memory_bytes": 335544320,
  "total_compute_units": 10,
  "message": "Process GPU monitoring data retrieved successfully",
  "component_status": {
    "daemon_stats": true,
    "system_metrics": true,
    "processes": true,
    "app_groups": true,
    "config": true,
    "pattern_database": true
  },
  "cache_info": {
    "cached": false,
    "ttl_seconds": 300
  },
  "timestamp": "2025-01-01T12:00:00+00:00"
}
```

**Ответ (если мониторинг GPU недоступен):**
```json
{
  "status": "degraded",
  "process_gpu": null,
  "count": 0,
  "message": "Process GPU monitoring not available",
  "suggestion": "Enable process GPU monitoring in configuration and ensure eBPF support",
  "component_status": {
    "daemon_stats": true,
    "system_metrics": true,
    "processes": true,
    "app_groups": true,
    "config": true,
    "pattern_database": true
  },
  "cache_info": {
    "cached": false,
    "ttl_seconds": 300
  },
  "timestamp": "2025-01-01T12:00:00+00:00"
}
```

**Поля ответа:**
- `status` (string) - статус запроса (`ok` или `degraded`)
- `process_gpu` (array?) - массив объектов с информацией об использовании GPU процессами
- `count` (integer) - количество процессов с данными о GPU
- `total_gpu_time_ns` (u64?) - общее время использования GPU всеми процессами в наносекундах
- `total_memory_bytes` (u64?) - общее использование памяти GPU всеми процессами в байтах
- `total_compute_units` (u64?) - общее количество использованных вычислительных единиц
- `message` (string) - сообщение о статусе запроса
- `suggestion` (string, опционально) - рекомендации по устранению проблем
- `component_status` (object) - статус доступности основных компонентов
- `cache_info` (object) - информация о кэшировании
- `timestamp` (string) - время генерации ответа в формате RFC3339

**Поля объекта process_gpu:**
- `pid` (u32) - идентификатор процесса
- `tgid` (u32) - идентификатор потока группы
- `gpu_time_ns` (u64) - время использования GPU в наносекундах
- `memory_usage_bytes` (u64) - использование памяти GPU в байтах
- `compute_units_used` (u64) - количество использованных вычислительных единиц
- `last_update_ns` (u64) - время последнего обновления в наносекундах
- `gpu_id` (u32) - идентификатор GPU устройства
- `temperature_celsius` (u32) - температура GPU в градусах Цельсия
- `name` (string) - имя процесса
- `gpu_usage_percent` (f32) - процент использования GPU процессом

**Требования:**
- Включенный `enable_process_gpu_monitoring` в конфигурации eBPF
- Поддержка eBPF в ядре Linux (5.4+)
- Права CAP_BPF или запуск от root
- Доступ к GPU устройствам

**Ограничения:**
- Данные собираются только для процессов, активно использующих GPU
- Точность измерений зависит от драйвера GPU и поддержки eBPF
- Максимальное количество отслеживаемых процессов: 4096

**Примеры использования:**

**Получение топ процессов по использованию GPU:**
```bash
curl -s http://127.0.0.1:8080/api/processes/gpu | \
  jq '.process_gpu | sort_by(.gpu_usage_percent) | reverse | .[0:5] | {pid, name, gpu_usage_percent, memory_usage_bytes}'
```

**Мониторинг общего использования GPU:**
```bash
curl -s http://127.0.0.1:8080/api/processes/gpu | \
  jq '{total_gpu_time: .total_gpu_time_ns, total_memory: .total_memory_bytes, processes: .count}'
```

**Проверка доступности мониторинга GPU:**
```bash
curl -s http://127.0.0.1:8080/api/processes/gpu | \
  jq '{available: (.process_gpu != null), message: .message, suggestion: .suggestion}'
```

**Статус коды:**
- `200 OK` - запрос выполнен успешно
- В случае ошибки возвращается JSON с полем `status: "error"` и описанием ошибки

---

### GET /api/processes/memory

Получение информации об использовании памяти процессами. Требует включения `enable_process_memory_monitoring` в конфигурации eBPF.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/processes/memory
```

**Успешный ответ:**
```json
{
  "status": "ok",
  "process_memory": [
    {
      "pid": 1234,
      "tgid": 1234,
      "last_update_ns": 1234567890123456789,
      "rss_bytes": 268435456,
      "vms_bytes": 536870912,
      "shared_bytes": 134217728,
      "swap_bytes": 67108864,
      "heap_usage": 201326592,
      "stack_usage": 33554432,
      "anonymous_memory": 167772160,
      "file_backed_memory": 100663296,
      "major_faults": 15,
      "minor_faults": 75,
      "name": "firefox"
    },
    {
      "pid": 5678,
      "tgid": 5678,
      "last_update_ns": 1234567890123456789,
      "rss_bytes": 134217728,
      "vms_bytes": 268435456,
      "shared_bytes": 67108864,
      "swap_bytes": 33554432,
      "heap_usage": 100663296,
      "stack_usage": 16777216,
      "anonymous_memory": 83886080,
      "file_backed_memory": 50331648,
      "major_faults": 8,
      "minor_faults": 42,
      "name": "blender"
    }
  ],
  "count": 2,
  "total_rss_bytes": 402653184,
  "total_vms_bytes": 805306368,
  "total_shared_bytes": 201326592,
  "total_swap_bytes": 100663296,
  "total_heap_usage": 301989888,
  "total_stack_usage": 50331648,
  "total_anonymous_memory": 251658240,
  "total_file_backed_memory": 150994944,
  "total_major_faults": 23,
  "total_minor_faults": 117,
  "message": "Process memory monitoring data retrieved successfully",
  "component_status": {
    "daemon_stats": true,
    "system_metrics": true,
    "processes": true,
    "app_groups": true,
    "config": true,
    "pattern_database": true
  },
  "cache_info": {
    "cached": false,
    "ttl_seconds": 300
  },
  "timestamp": "2025-01-01T12:00:00+00:00"
}
```

**Поля ответа:**
- `status` (string) - статус запроса (`ok` или `degraded`)
- `process_memory` (array?) - массив объектов с информацией об использовании памяти процессами
- `count` (integer) - количество процессов с данными о памяти
- `total_rss_bytes` (u64?) - общее использование резидентной памяти всеми процессами в байтах
- `total_vms_bytes` (u64?) - общее использование виртуальной памяти всеми процессами в байтах
- `total_shared_bytes` (u64?) - общее использование разделяемой памяти всеми процессами в байтах
- `total_swap_bytes` (u64?) - общее использование swap памяти всеми процессами в байтах
- `total_heap_usage` (u64?) - общее использование heap памяти всеми процессами в байтах
- `total_stack_usage` (u64?) - общее использование stack памяти всеми процессами в байтах
- `total_anonymous_memory` (u64?) - общее использование анонимной памяти всеми процессами в байтах
- `total_file_backed_memory` (u64?) - общее использование памяти, поддерживаемой файлами, всеми процессами в байтах
- `total_major_faults` (u64?) - общее количество major page faults всех процессов
- `total_minor_faults` (u64?) - общее количество minor page faults всех процессов
- `message` (string) - сообщение о статусе запроса
- `component_status` (object) - статус доступности основных компонентов
- `cache_info` (object) - информация о кэшировании
- `timestamp` (string) - время генерации ответа в формате RFC3339

**Поля объекта process_memory:**
- `pid` (u32) - идентификатор процесса
- `tgid` (u32) - идентификатор потока группы
- `last_update_ns` (u64) - время последнего обновления в наносекундах
- `rss_bytes` (u64) - резидентное использование памяти в байтах
- `vms_bytes` (u64) - использование виртуальной памяти в байтах
- `shared_bytes` (u64) - использование разделяемой памяти в байтах
- `swap_bytes` (u64) - использование swap памяти в байтах
- `heap_usage` (u64) - использование heap памяти в байтах
- `stack_usage` (u64) - использование stack памяти в байтах
- `anonymous_memory` (u64) - использование анонимной памяти в байтах
- `file_backed_memory` (u64) - использование памяти, поддерживаемой файлами, в байтах
- `major_faults` (u64) - количество major page faults
- `minor_faults` (u64) - количество minor page faults
- `name` (string) - имя процесса

**Требования:**
- Требуется включенный мониторинг памяти процессов в конфигурации eBPF (`enable_process_memory_monitoring: true`)
- Требуется поддержка eBPF в ядре Linux (версия 5.4 или новее)
- Требуются права CAP_BPF или запуск от root

---

### GET /api/processes/energy

Получение информации об энергопотреблении процессами. Требует включения `enable_process_energy_monitoring` в конфигурации eBPF.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/processes/energy
```

**Успешный ответ:**
```json
{
  "status": "ok",
  "process_energy": [
    {
      "pid": 1234,
      "tgid": 1234,
      "energy_uj": 1500000000,
      "power_uw": 250000,
      "last_update_ns": 1234567890123456789,
      "name": "firefox",
      "energy_usage_percent": 45.2
    },
    {
      "pid": 5678,
      "tgid": 5678,
      "energy_uj": 400000000,
      "power_uw": 180000,
      "last_update_ns": 1234567890123456789,
      "name": "blender",
      "energy_usage_percent": 12.8
    }
  ],
  "count": 2,
  "total_energy_uj": 1900000000,
  "total_power_uw": 430000,
  "message": "Process energy monitoring data retrieved successfully",
  "component_status": {
    "daemon_stats": true,
    "system_metrics": true,
    "processes": true,
    "app_groups": true,
    "config": true,
    "pattern_database": true
  },
  "cache_info": {
    "cached": false,
    "ttl_seconds": 300
  },
  "timestamp": "2025-01-01T12:00:00+00:00"
}
```

**Поля ответа:**
- `status` (string) - статус запроса (`ok` или `degraded`)
- `process_energy` (array?) - массив объектов с информацией об энергопотреблении процессами
- `count` (integer) - количество процессов с данными об энергопотреблении
- `total_energy_uj` (u64?) - общее энергопотребление всеми процессами в микроджоулях
- `total_power_uw` (u64?) - общая мощность потребления всеми процессами в микроваттах
- `message` (string) - сообщение о статусе запроса
- `component_status` (object) - статус доступности основных компонентов
- `cache_info` (object) - информация о кэшировании
- `timestamp` (string) - время генерации ответа в формате RFC3339

**Поля объекта process_energy:**
- `pid` (u32) - идентификатор процесса
- `tgid` (u32) - идентификатор потока группы
- `energy_uj` (u64) - энергопотребление в микроджоулях
- `power_uw` (u64) - мощность потребления в микроваттах
- `last_update_ns` (u64) - время последнего обновления в наносекундах
- `name` (string) - имя процесса
- `energy_usage_percent` (f32) - процент энергопотребления

**Требования:**
- Требуется включенный мониторинг энергопотребления процессов в конфигурации eBPF (`enable_process_energy_monitoring: true`)
- Требуется поддержка eBPF в ядре Linux (версия 5.4 или новее)
- Требуются права CAP_BPF или запуск от root

---

### GET /api/processes/network

Получение информации об использовании сети процессами. Требует включения `enable_process_network_monitoring` в конфигурации eBPF.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/processes/network
```

**Успешный ответ:**
```json
{
  "status": "ok",
  "process_network": [
    {
      "pid": 1234,
      "tgid": 1234,
      "rx_bytes": 268435456,
      "tx_bytes": 134217728,
      "rx_packets": 1500,
      "tx_packets": 800,
      "last_update_ns": 1234567890123456789,
      "name": "firefox",
      "network_usage_percent": 45.2
    },
    {
      "pid": 5678,
      "tgid": 5678,
      "rx_bytes": 67108864,
      "tx_bytes": 33554432,
      "rx_packets": 400,
      "tx_packets": 200,
      "last_update_ns": 1234567890123456789,
      "name": "blender",
      "network_usage_percent": 12.8
    }
  ],
  "count": 2,
  "total_rx_bytes": 335544320,
  "total_tx_bytes": 167772160,
  "total_rx_packets": 1900,
  "total_tx_packets": 1000,
  "message": "Process network monitoring data retrieved successfully",
  "component_status": {
    "daemon_stats": true,
    "system_metrics": true,
    "processes": true,
    "app_groups": true,
    "config": true,
    "pattern_database": true
  },
  "cache_info": {
    "cached": false,
    "ttl_seconds": 300
  },
  "timestamp": "2025-01-01T12:00:00+00:00"
}
```

**Поля ответа:**
- `status` (string) - статус запроса (`ok` или `degraded`)
- `process_network` (array?) - массив объектов с информацией об использовании сети процессами
- `count` (integer) - количество процессов с данными о сети
- `total_rx_bytes` (u64?) - общее количество принятых байт всеми процессами
- `total_tx_bytes` (u64?) - общее количество отправленных байт всеми процессами
- `total_rx_packets` (u64?) - общее количество принятых пакетов всеми процессами
- `total_tx_packets` (u64?) - общее количество отправленных пакетов всеми процессами
- `message` (string) - сообщение о статусе запроса
- `component_status` (object) - статус доступности основных компонентов
- `cache_info` (object) - информация о кэшировании
- `timestamp` (string) - время генерации ответа в формате RFC3339

**Поля объекта process_network:**
- `pid` (u32) - идентификатор процесса
- `tgid` (u32) - идентификатор потока группы
- `rx_bytes` (u64) - количество принятых байт
- `tx_bytes` (u64) - количество отправленных байт
- `rx_packets` (u64) - количество принятых пакетов
- `tx_packets` (u64) - количество отправленных пакетов
- `last_update_ns` (u64) - время последнего обновления в наносекундах
- `name` (string) - имя процесса
- `network_usage_percent` (f32) - процент использования сети

**Требования:**
- Требуется включенный мониторинг сети процессов в конфигурации eBPF (`enable_process_network_monitoring: true`)
- Требуется поддержка eBPF в ядре Linux (версия 5.4 или новее)
- Требуются права CAP_BPF или запуск от root

---

### GET /api/processes/disk

Получение информации об использовании диска процессами. Требует включения `enable_process_disk_monitoring` в конфигурации eBPF.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/processes/disk
```

**Успешный ответ:**
```json
{
  "status": "ok",
  "process_disk": [
    {
      "pid": 1234,
      "tgid": 1234,
      "read_bytes": 268435456,
      "write_bytes": 134217728,
      "read_ops": 1500,
      "write_ops": 800,
      "last_update_ns": 1234567890123456789,
      "name": "firefox",
      "disk_usage_percent": 45.2
    },
    {
      "pid": 5678,
      "tgid": 5678,
      "read_bytes": 67108864,
      "write_bytes": 33554432,
      "read_ops": 400,
      "write_ops": 200,
      "last_update_ns": 1234567890123456789,
      "name": "blender",
      "disk_usage_percent": 12.8
    }
  ],
  "count": 2,
  "total_read_bytes": 335544320,
  "total_write_bytes": 167772160,
  "total_read_ops": 1900,
  "total_write_ops": 1000,
  "message": "Process disk monitoring data retrieved successfully",
  "component_status": {
    "daemon_stats": true,
    "system_metrics": true,
    "processes": true,
    "app_groups": true,
    "config": true,
    "pattern_database": true
  },
  "cache_info": {
    "cached": false,
    "ttl_seconds": 300
  },
  "timestamp": "2025-01-01T12:00:00+00:00"
}
```

**Поля ответа:**
- `status` (string) - статус запроса (`ok` или `degraded`)
- `process_disk` (array?) - массив объектов с информацией об использовании диска процессами
- `count` (integer) - количество процессов с данными о диске
- `total_read_bytes` (u64?) - общее количество прочитанных байт всеми процессами
- `total_write_bytes` (u64?) - общее количество записанных байт всеми процессами
- `total_read_ops` (u64?) - общее количество операций чтения всеми процессами
- `total_write_ops` (u64?) - общее количество операций записи всеми процессами
- `message` (string) - сообщение о статусе запроса
- `component_status` (object) - статус доступности основных компонентов
- `cache_info` (object) - информация о кэшировании
- `timestamp` (string) - время генерации ответа в формате RFC3339

**Поля объекта process_disk:**
- `pid` (u32) - идентификатор процесса
- `tgid` (u32) - идентификатор потока группы
- `read_bytes` (u64) - количество прочитанных байт
- `write_bytes` (u64) - количество записанных байт
- `read_ops` (u64) - количество операций чтения
- `write_ops` (u64) - количество операций записи
- `last_update_ns` (u64) - время последнего обновления в наносекундах
- `name` (string) - имя процесса
- `disk_usage_percent` (f32) - процент использования диска

**Требования:**
- Требуется включенный мониторинг диска процессов в конфигурации eBPF (`enable_process_disk_monitoring: true`)
- Требуется поддержка eBPF в ядре Linux (версия 5.4 или новее)
- Требуются права CAP_BPF или запуск от root

---

### GET /api/performance

Получение информации о производительности API сервера.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/performance
```

**Успешный ответ:**
```json
{
  "status": "ok",
  "performance_metrics": {
    "total_requests": 1500,
    "cache_hits": 1200,
    "cache_misses": 300,
    "cache_hit_rate": 0.8,
    "average_processing_time_us": 1500,
    "total_processing_time_us": 2250000,
    "last_request_time": 123.45,
    "requests_per_second": 15.5
  },
  "cache_info": {
    "enabled": true,
    "ttl_seconds": null
  }
}
```

**Поля ответа:**
- `status` (string) - статус запроса (`ok`)
- `performance_metrics` (object) - метрики производительности API сервера
- `cache_info` (object) - информация о кэшировании

**Поля объекта performance_metrics:**
- `total_requests` (u64) - общее количество запросов
- `cache_hits` (u64) - количество кэш-попаданий
- `cache_misses` (u64) - количество кэш-промахов
- `cache_hit_rate` (f32) - коэффициент попадания в кэш
- `average_processing_time_us` (u64) - среднее время обработки запроса в микросекундах
- `total_processing_time_us` (u64) - общее время обработки всех запросов в микросекундах
- `last_request_time` (f64, optional) - время с последнего запроса в секундах
- `requests_per_second` (f32) - количество запросов в секунду

**Поля объекта cache_info:**
- `enabled` (bool) - включено ли кэширование
- `ttl_seconds` (u64, optional) - время жизни кэша в секундах (может быть null)

**Требования:**
- Нет специальных требований, доступно всегда

**Примеры использования:**

Получение текущих метрик производительности:
```bash
curl -s http://127.0.0.1:8080/api/performance | jq
```

Мониторинг производительности API:
```bash
watch -n 1 'curl -s http://127.0.0.1:8080/api/performance | jq ".performance_metrics | {total_requests, cache_hit_rate, requests_per_second}"'
```

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

### POST /api/notifications/custom

Отправка пользовательского уведомления через систему уведомлений.

**Запрос:**
```bash
curl -X POST http://127.0.0.1:8080/api/notifications/custom \
  -H "Content-Type: application/json" \
  -d '{
    "type": "info",
    "title": "Custom Notification",
    "message": "This is a custom notification message",
    "details": "Additional details about the notification"
  }'
```

**Ответ (успешная отправка):**
```json
{
  "status": "success",
  "message": "Custom notification sent successfully",
  "notification": {
    "type": "info",
    "title": "Custom Notification",
    "message": "This is a custom notification message",
    "details": "Additional details about the notification"
  }
}
```

**Ответ (уведомления недоступны):**
```json
{
  "status": "error",
  "message": "Notification manager not available",
  "available": false
}
```

**Поля запроса:**
- `type` (string, опционально) - тип уведомления: "critical", "warning" или "info" (по умолчанию: "info")
- `title` (string, опционально) - заголовок уведомления (по умолчанию: "Custom Notification")
- `message` (string, опционально) - основное сообщение уведомления (по умолчанию: "Custom notification message")
- `details` (string, опционально) - дополнительные детали уведомления

**Поля ответа:**
- `status` (string) - статус операции: "success" или "error"
- `message` (string) - описание результата операции
- `notification` (object, опционально) - информация об отправленном уведомлении
  - `type` (string) - тип уведомления
  - `title` (string) - заголовок уведомления
  - `message` (string) - сообщение уведомления
  - `details` (string, опционально) - дополнительные детали
- `available` (bool, опционально) - флаг доступности системы уведомлений

**Примечания:**
- Позволяет отправлять пользовательские уведомления через API
- Требует наличия менеджера уведомлений в состоянии API
- Поддерживает три типа уведомлений: critical, warning и info
- Все поля являются опциональными и имеют значения по умолчанию
- Полезно для интеграции с внешними системами мониторинга

**Примеры использования:**

**Отправка критического уведомления:**
```bash
curl -X POST http://127.0.0.1:8080/api/notifications/custom \
  -H "Content-Type: application/json" \
  -d '{
    "type": "critical",
    "title": "Critical Alert",
    "message": "System is running out of memory!"
  }'
```

**Отправка уведомления с деталями:**
```bash
curl -X POST http://127.0.0.1:8080/api/notifications/custom \
  -H "Content-Type: application/json" \
  -d '{
    "type": "warning",
    "title": "High CPU Usage",
    "message": "CPU usage is above 90%",
    "details": "Process: smoothtaskd, CPU: 95%, Duration: 5 minutes"
  }'
```

**Отправка минимального уведомления:**
```bash
curl -X POST http://127.0.0.1:8080/api/notifications/custom \
  -H "Content-Type: application/json" \
  -d '{}'
```

**Статус коды:**
- `200 OK` - Успешная отправка уведомления
- `200 OK` с `status: "error"` - Ошибка отправки уведомления

---

### GET /api/logs

Получение последних логов приложения с возможностью фильтрации по уровню и ограничения количества записей.

**Запрос:**
```bash
# Получение последних 100 логов (по умолчанию)
curl "http://127.0.0.1:8080/api/logs"

# Получение последних 50 логов
curl "http://127.0.0.1:8080/api/logs?limit=50"

# Получение только ошибок и предупреждений
curl "http://127.0.0.1:8080/api/logs?level=warn"

# Получение последних 20 информационных сообщений
curl "http://127.0.0.1:8080/api/logs?level=info&limit=20"
```

**Параметры запроса:**
- `level` (опционально): Минимальный уровень логирования. Возможные значения: `error`, `warn`, `info`, `debug`, `trace`. По умолчанию: `trace` (все уровни)
- `limit` (опционально): Максимальное количество возвращаемых записей. Диапазон: 1-1000. По умолчанию: 100

**Ответ (с логами):**
```json
{
  "status": "ok",
  "logs": [
    {
      "timestamp": "2023-12-12T12:34:56.789Z",
      "level": "INFO",
      "target": "smoothtask_core::api::server",
      "message": "API server started on 127.0.0.1:8080",
      "fields": {
        "port": 8080,
        "address": "127.0.0.1"
      }
    },
    {
      "timestamp": "2023-12-12T12:35:01.234Z",
      "level": "WARN",
      "target": "smoothtask_core::config::watcher",
      "message": "Configuration file not found, using defaults",
      "fields": null
    }
  ],
  "count": 2,
  "total_available": 42,
  "max_capacity": 1000,
  "filter": {
    "min_level": "TRACE",
    "limit": 100
  }
}
```

**Ответ (без хранилища логов):**
```json
{
  "status": "ok",
  "logs": [],
  "count": 0,
  "message": "Log storage not available (daemon may not be running or logs not configured)",
  "filter": {
    "min_level": "TRACE",
    "limit": 100
  }
}
```

**Поля ответа:**
- `status` (string) - статус ответа (всегда "ok")
- `logs` (array) - массив записей логов
  - `timestamp` (string) - временная метка в формате RFC3339
  - `level` (string) - уровень логирования: "ERROR", "WARN", "INFO", "DEBUG", "TRACE"
  - `target` (string) - модуль или компонент, создавший запись
  - `message` (string) - сообщение лога
  - `fields` (object | null) - дополнительные поля (опционально)
- `count` (number) - количество возвращённых записей
- `total_available` (number) - общее количество доступных записей в хранилище
- `max_capacity` (number) - максимальная ёмкость хранилища логов
- `filter` (object) - информация о применённых фильтрах
  - `min_level` (string) - минимальный уровень логирования, использованный для фильтрации
  - `limit` (number) - максимальное количество записей, использованное для ограничения
- `message` (string, опционально) - сообщение об отсутствии данных (если хранилище не доступно)

**Уровни логирования:**
- `ERROR` - Критические ошибки, требующие немедленного внимания
- `WARN` - Предупреждения о потенциальных проблемах
- `INFO` - Информационные сообщения о нормальной работе
- `DEBUG` - Отладочная информация
- `TRACE` - Очень подробная отладочная информация

**Примеры использования:**

**Мониторинг ошибок:**
```bash
# Получение только ошибок
curl "http://127.0.0.1:8080/api/logs?level=error"

# Получение ошибок и предупреждений
curl "http://127.0.0.1:8080/api/logs?level=warn"
```

**Отладка:**
```bash
# Получение последних 100 отладочных сообщений
curl "http://127.0.0.1:8080/api/logs?level=debug&limit=100"

# Получение всех сообщений (до максимальной ёмкости хранилища)
curl "http://127.0.0.1:8080/api/logs?level=trace&limit=1000"
```

**Мониторинг в реальном времени:**
```bash
#!/bin/bash
# Скрипт для мониторинга логов в реальном времени

while true; do
    clear
    echo "=== SmoothTask Logs Monitor ==="
    echo "Timestamp: $(date)"
    echo
    
    # Получение последних 20 логов
    curl -s "http://127.0.0.1:8080/api/logs?limit=20" | jq '.logs[] | "[\(.timestamp)] \(.level) \(.target): \(.message)"'
    
    sleep 2
 done
```

**Интеграция с системами мониторинга:**
```python
#!/usr/bin/env python3
# Простой мониторинг ошибок для интеграции с системами алертинга

import requests
import json

def check_for_errors():
    response = requests.get("http://127.0.0.1:8080/api/logs?level=error")
    data = response.json()
    
    if data["count"] > 0:
        print(f"Found {data['count']} errors!")
        for log in data["logs"]:
            print(f"[{log['timestamp']}] {log['level']} {log['target']}: {log['message']}")
        return True
    else:
        print("No errors found")
        return False

if __name__ == "__main__":
    if check_for_errors():
        # Отправить уведомление в систему мониторинга
        exit(1)
    else:
        exit(0)
```

**Статус коды:**
- `200 OK` - запрос выполнен успешно

**Примечания:**
- Хранилище логов имеет ограниченную ёмкость (по умолчанию 1000 записей)
- Старые записи автоматически удаляются при достижении максимальной ёмкости
- Логи хранятся в памяти и не сохраняются на диск
- Для долговременного хранения логов рекомендуется использовать внешние системы логирования

---

### GET /api/logging/monitoring

Получение статистики и мониторинга системы логирования.

**Запрос:**
```bash
curl "http://127.0.0.1:8080/api/logging/monitoring"
```

**Ответ (с хранилищем логов):**
```json
{
  "status": "ok",
  "logging_monitoring": {
    "statistics": {
      "total_entries": 42,
      "max_capacity": 1000,
      "usage_percentage": 4.2,
      "error_count": 2,
      "warning_count": 5,
      "last_error_time": "2023-12-12T12:34:56.789Z"
    },
    "health": {
      "status": "warning",
      "message": "Обнаружены ошибки в логах (количество: 2)",
      "timestamp": "2023-12-12T12:35:01.234Z"
    },
    "recent_logs": {
      "count": 10,
      "entries": [
        {
          "timestamp": "2023-12-12T12:34:56.789Z",
          "level": "ERROR",
          "target": "smoothtask_core::api::server",
          "message": "Failed to connect to database"
        },
        {
          "timestamp": "2023-12-12T12:34:57.123Z",
          "level": "WARN",
          "target": "smoothtask_core::config::watcher",
          "message": "Configuration file not found, using defaults"
        }
      ]
    }
  },
  "availability": {
    "log_storage_available": true,
    "timestamp": "2023-12-12T12:35:01.234Z"
  }
}
```

**Ответ (без хранилища логов):**
```json
{
  "status": "ok",
  "logging_monitoring": null,
  "message": "Log storage not available (may not be configured)",
  "availability": {
    "log_storage_available": false,
    "timestamp": "2023-12-12T12:35:01.234Z"
  }
}
```

**Поля ответа:**
- `status` (string) - статус ответа (всегда "ok")
- `logging_monitoring` (object | null) - объект с данными мониторинга или null, если хранилище не доступно
  - `statistics` (object) - статистика использования хранилища логов
    - `total_entries` (number) - общее количество записей в хранилище
    - `max_capacity` (number) - максимальная ёмкость хранилища
    - `usage_percentage` (number) - процент использования хранилища
    - `error_count` (number) - количество записей с уровнем ERROR
    - `warning_count` (number) - количество записей с уровнем WARN
    - `last_error_time` (string | null) - временная метка последней ошибки
  - `health` (object) - информация о состоянии здоровья системы логирования
    - `status` (string) - статус здоровья: "healthy", "warning", или "critical"
    - `message` (string) - описание текущего состояния
    - `timestamp` (string) - временная метка проверки
  - `recent_logs` (object) - последние записи логов для мониторинга
    - `count` (number) - количество последних записей
    - `entries` (array) - массив последних записей логов
- `availability` (object) - информация о доступности компонентов
  - `log_storage_available` (boolean) - доступность хранилища логов
  - `timestamp` (string) - временная метка проверки доступности
- `message` (string, опционально) - сообщение об отсутствии данных

**Статусы здоровья:**
- `healthy` - Система логирования работает нормально (нет ошибок, достаточно свободного места)
- `warning` - Обнаружены проблемы (есть ошибки или хранилище почти заполнено)
- `critical` - Критические проблемы (хранилище переполнено или другие серьезные ошибки)

**Примеры использования:**

**Мониторинг состояния системы логирования:**
```bash
# Получение текущего состояния системы логирования
curl -s "http://127.0.0.1:8080/api/logging/monitoring" | jq '.logging_monitoring.health'

# Проверка наличия ошибок
curl -s "http://127.0.0.1:8080/api/logging/monitoring" | jq '.logging_monitoring.statistics.error_count'
```

**Интеграция с системами мониторинга:**
```python
#!/usr/bin/env python3
# Мониторинг системы логирования для интеграции с Prometheus

import requests
import time

def collect_logging_metrics():
    try:
        response = requests.get("http://127.0.0.1:8080/api/logging/monitoring")
        data = response.json()
        
        if data["logging_monitoring"]:
            stats = data["logging_monitoring"]["statistics"]
            health = data["logging_monitoring"]["health"]
            
            print(f"smoothtask_logging_total_entries {stats['total_entries']}")
            print(f"smoothtask_logging_error_count {stats['error_count']}")
            print(f"smoothtask_logging_warning_count {stats['warning_count']}")
            print(f"smoothtask_logging_usage_percentage {stats['usage_percentage']}")
            print(f"smoothtask_logging_health_status {1 if health['status'] == 'healthy' else 0}")
    except Exception as e:
        print(f"# ERROR: {e}")

if __name__ == "__main__":
    while True:
        collect_logging_metrics()
        time.sleep(15)
```

---

### GET /api/cache/monitoring

Получение статистики и мониторинга системы кэширования API.

**Запрос:**
```bash
curl "http://127.0.0.1:8080/api/cache/monitoring"
```

**Ответ (с кэшем):**
```json
{
  "status": "ok",
  "cache_monitoring": {
    "api_cache": {
      "enabled": true,
      "cache_type": "api_response_cache",
      "statistics": {
        "total_cached_items": 8,
        "active_items": 8,
        "stale_items": 0,
        "active_percentage": 100.0,
        "cache_ttl_seconds": 60
      },
      "health": {
        "status": "healthy",
        "message": "Кэш работает эффективно",
        "timestamp": "2023-12-12T12:35:01.234Z"
      }
    },
    "performance": {
      "total_requests": 100,
      "cache_hits": 75,
      "cache_misses": 25,
      "cache_hit_rate": 75.0,
      "cache_miss_rate": 25.0,
      "average_processing_time_us": 1234
    },
    "overall_health": {
      "status": "healthy",
      "message": "Кэширование работает отлично",
      "timestamp": "2023-12-12T12:35:01.234Z"
    }
  },
  "availability": {
    "cache_available": true,
    "performance_metrics_available": true,
    "timestamp": "2023-12-12T12:35:01.234Z"
  }
}
```

**Ответ (без кэша):**
```json
{
  "status": "ok",
  "cache_monitoring": {
    "api_cache": {
      "enabled": false,
      "cache_type": "api_response_cache",
      "statistics": {
        "total_cached_items": 0,
        "active_items": 0,
        "stale_items": 0,
        "active_percentage": 0.0,
        "cache_ttl_seconds": 0
      },
      "health": {
        "status": "disabled",
        "message": "Кэш API отключен",
        "timestamp": "2023-12-12T12:35:01.234Z"
      }
    },
    "performance": {
      "total_requests": 0,
      "cache_hits": 0,
      "cache_misses": 0,
      "cache_hit_rate": 0.0,
      "cache_miss_rate": 0.0,
      "average_processing_time_us": 0
    },
    "overall_health": {
      "status": "disabled",
      "message": "Кэширование отключено",
      "timestamp": "2023-12-12T12:35:01.234Z"
    }
  },
  "availability": {
    "cache_available": false,
    "performance_metrics_available": true,
    "timestamp": "2023-12-12T12:35:01.234Z"
  }
}
```

**Поля ответа:**
- `status` (string) - статус ответа (всегда "ok")
- `cache_monitoring` (object) - объект с данными мониторинга кэша
  - `api_cache` (object) - информация о кэше API
    - `enabled` (boolean) - включен ли кэш
    - `cache_type` (string) - тип кэша (всегда "api_response_cache")
    - `statistics` (object) - статистика использования кэша
      - `total_cached_items` (number) - общее количество кэшированных элементов
      - `active_items` (number) - количество активных (не устаревших) элементов
      - `stale_items` (number) - количество устаревших элементов
      - `active_percentage` (number) - процент активных элементов
      - `cache_ttl_seconds` (number) - время жизни кэша в секундах
    - `health` (object) - информация о состоянии здоровья кэша
      - `status` (string) - статус здоровья: "healthy", "warning", "idle", или "disabled"
      - `message` (string) - описание текущего состояния
      - `timestamp` (string) - временная метка проверки
  - `performance` (object) - информация о производительности кэширования
    - `total_requests` (number) - общее количество запросов
    - `cache_hits` (number) - количество кэш-хитов
    - `cache_misses` (number) - количество кэш-миссов
    - `cache_hit_rate` (number) - процент кэш-хитов
    - `cache_miss_rate` (number) - процент кэш-миссов
    - `average_processing_time_us` (number) - среднее время обработки запросов в микросекундах
  - `overall_health` (object) - общая информация о состоянии здоровья системы кэширования
    - `status` (string) - общий статус здоровья: "healthy", "warning", "critical", или "disabled"
    - `message` (string) - описание общего состояния
    - `timestamp` (string) - временная метка проверки
- `availability` (object) - информация о доступности компонентов
  - `cache_available` (boolean) - доступность кэша
  - `performance_metrics_available` (boolean) - доступность метрик производительности
  - `timestamp` (string) - временная метка проверки доступности

**Статусы здоровья кэша:**
- `healthy` - Кэш работает эффективно (большинство элементов активны)
- `warning` - Много устаревших элементов в кэше
- `idle` - Кэш не используется (нет кэшированных элементов)
- `disabled` - Кэш отключен

**Общие статусы здоровья:**
- `healthy` - Кэширование работает отлично (cache_hit_rate > 70%)
- `warning` - Кэширование может быть улучшено (30% < cache_hit_rate ≤ 70%)
- `critical` - Кэширование неэффективно (cache_hit_rate ≤ 30%)
- `disabled` - Кэширование отключено

**Примеры использования:**

**Мониторинг состояния кэша:**
```bash
# Получение текущего состояния кэша
curl -s "http://127.0.0.1:8080/api/cache/monitoring" | jq '.cache_monitoring.api_cache.health'

# Проверка эффективности кэширования
curl -s "http://127.0.0.1:8080/api/cache/monitoring" | jq '.cache_monitoring.performance.cache_hit_rate'
```

**Оптимизация производительности:**
```bash
#!/bin/bash
# Скрипт для мониторинга и оптимизации кэша

while true; do
    clear
    echo "=== Cache Monitoring Dashboard ==="
    echo "Timestamp: $(date)"
    echo
    
    # Получение данных о кэше
    cache_data=$(curl -s "http://127.0.0.1:8080/api/cache/monitoring")
    
    # Отображение ключевых метрик
    echo "Cache Health: $(echo $cache_data | jq -r '.cache_monitoring.api_cache.health.status')"
    echo "Cache Hit Rate: $(echo $cache_data | jq '.cache_monitoring.performance.cache_hit_rate')%"
    echo "Active Items: $(echo $cache_data | jq '.cache_monitoring.api_cache.statistics.active_items')"
    echo "Total Items: $(echo $cache_data | jq '.cache_monitoring.api_cache.statistics.total_cached_items')"
    echo
    
    # Рекомендации
    hit_rate=$(echo $cache_data | jq '.cache_monitoring.performance.cache_hit_rate')
    if (( $(echo "$hit_rate < 50" | bc -l) )); then
        echo "⚠️  WARNING: Low cache hit rate. Consider increasing cache TTL or optimizing queries."
    else
        echo "✅ Cache performance is good."
    fi
    
    sleep 5
 done
```

**Интеграция с Prometheus:**
```python
#!/usr/bin/env python3
# Экспортер метрик кэша для Prometheus

import requests
from prometheus_client import start_http_server, Gauge
import time

# Создаем метрики
cache_health = Gauge('smoothtask_cache_health', 'Cache health status (1=healthy, 0=unhealthy)')
cache_hit_rate = Gauge('smoothtask_cache_hit_rate', 'Cache hit rate percentage')
cache_active_items = Gauge('smoothtask_cache_active_items', 'Number of active cache items')
cache_total_items = Gauge('smoothtask_cache_total_items', 'Total number of cached items')
cache_processing_time = Gauge('smoothtask_cache_processing_time_us', 'Average processing time in microseconds')

def collect_cache_metrics():
    try:
        response = requests.get("http://127.0.0.1:8080/api/cache/monitoring")
        data = response.json()
        
        cache_data = data["cache_monitoring"]
        api_cache = cache_data["api_cache"]
        performance = cache_data["performance"]
        
        # Обновляем метрики
        health_status = api_cache["health"]["status"]
        cache_health.set(1 if health_status == "healthy" else 0)
        cache_hit_rate.set(performance["cache_hit_rate"])
        cache_active_items.set(api_cache["statistics"]["active_items"])
        cache_total_items.set(api_cache["statistics"]["total_cached_items"])
        cache_processing_time.set(performance["average_processing_time_us"])
        
    except Exception as e:
        print(f"Error collecting cache metrics: {e}")

if __name__ == "__main__":
    # Запускаем HTTP сервер для Prometheus на порту 8000
    start_http_server(8000)
    
    # Собираем метрики каждые 15 секунд
    while True:
        collect_cache_metrics()
        time.sleep(15)
```

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

### POST /api/config/reload

Перезагрузка конфигурации демона.

**Запрос:**
```bash
curl -X POST http://127.0.0.1:8080/api/config/reload
```

**Успешный ответ:**
```json
{
  "status": "success",
  "message": "Configuration reloaded successfully",
  "timestamp": "2023-12-12T12:34:56.789Z"
}
```

**Ответ при ошибке:**
```json
{
  "status": "error",
  "error": "config_reload_failed",
  "message": "Failed to reload configuration: Invalid config file",
  "timestamp": "2023-12-12T12:34:56.789Z"
}
```

**Статус коды:**
- `200 OK` - Конфигурация успешно перезагружена
- `500 Internal Server Error` - Ошибка при перезагрузке конфигурации

**Использование:**
Этот endpoint позволяет перезагрузить конфигурацию демона без перезапуска. Полезно при изменении конфигурационного файла или параметров работы.

---

### GET /api/classes

Получение списка классов приоритетов и их характеристик.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/classes
```

**Ответ:**
```json
{
  "status": "ok",
  "classes": {
    "LATENCY_CRITICAL": {
      "description": "Критичные к задержкам процессы",
      "nice": -20,
      "latency_nice": -20,
      "cpu_weight": 100,
      "io_class": 1,
      "io_priority": 0
    },
    "INTERACTIVE": {
      "description": "Интерактивные процессы",
      "nice": -10,
      "latency_nice": -10,
      "cpu_weight": 50,
      "io_class": 2,
      "io_priority": 4
    },
    "NORMAL": {
      "description": "Нормальные процессы",
      "nice": 0,
      "latency_nice": 0,
      "cpu_weight": 25,
      "io_class": 2,
      "io_priority": 4
    },
    "BACKGROUND": {
      "description": "Фоновые процессы",
      "nice": 10,
      "latency_nice": 10,
      "cpu_weight": 10,
      "io_class": 3,
      "io_priority": 7
    },
    "IDLE": {
      "description": "Процессы простоя",
      "nice": 19,
      "latency_nice": 19,
      "cpu_weight": 5,
      "io_class": 3,
      "io_priority": 7
    }
  },
  "timestamp": "2023-12-12T12:34:56.789Z"
}
```

**Статус коды:**
- `200 OK` - Запрос выполнен успешно

**Использование:**
Этот endpoint предоставляет информацию о доступных классах приоритетов и их характеристиках. Полезно для понимания, как SmoothTask классифицирует и управляет процессами.

---

### GET /api/patterns

Получение информации о паттерн-базе для классификации процессов.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/patterns
```

**Ответ:**
```json
{
  "status": "ok",
  "pattern_database": {
    "total_patterns": 150,
    "categories": ["browser", "ide", "game", "audio", "video", "build_tool"],
    "last_updated": "2023-12-12T12:34:56.789Z",
    "version": "1.0.0"
  },
  "timestamp": "2023-12-12T12:34:56.789Z"
}
```

**Статус коды:**
- `200 OK` - Запрос выполнен успешно

**Использование:**
Этот endpoint предоставляет информацию о паттерн-базе, используемой для классификации процессов. Полезно для мониторинга и отладки системы классификации.

---

### GET /api/system

Получение информации о системе и окружении.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/system
```

**Ответ:**
```json
{
  "status": "ok",
  "system_info": {
    "os": "Linux",
    "arch": "x86_64",
    "kernel_version": "5.15.0-86-generic",
    "host_name": "my-host",
    "cpu_cores": 8,
    "memory_total_mb": 16384,
    "swap_total_mb": 8192,
    "uptime_seconds": 3600,
    "load_avg": [1.5, 1.2, 1.0],
    "features": {
      "ebpf_available": true,
      "cgroups_v2_available": true,
      "x11_available": false,
      "wayland_available": true,
      "pipewire_available": true
    }
  },
  "timestamp": "2023-12-12T12:34:56.789Z"
}
```

**Статус коды:**
- `200 OK` - Запрос выполнен успешно

**Использование:**
Этот endpoint предоставляет информацию о системе и доступных функциях. Полезно для диагностики и проверки совместимости.

---

### POST /api/notifications/test

Отправка тестового уведомления.

**Запрос:**
```bash
curl -X POST "http://127.0.0.1:8080/api/notifications/test" \
  -H "Content-Type: application/json" \
  -d '{"title": "Test Notification", "message": "This is a test notification", "level": "info"}'
```

**Параметры запроса (JSON):**
- `title` (string) - заголовок уведомления
- `message` (string) - текст уведомления
- `level` (string, optional) - уровень уведомления (info, warning, error)

**Ответ:**
```json
{
  "status": "success",
  "message": "Test notification sent successfully",
  "notification_id": "abc123",
  "timestamp": "2023-12-12T12:34:56.789Z"
}
```

**Статус коды:**
- `200 OK` - Уведомление успешно отправлено
- `400 Bad Request` - Некорректные параметры запроса

**Использование:**
Этот endpoint позволяет отправить тестовое уведомление для проверки работы системы уведомлений.

---

### GET /api/notifications/status

Получение статуса системы уведомлений.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/notifications/status
```

**Ответ:**
```json
{
  "status": "ok",
  "notifications": {
    "enabled": true,
    "backend": "dbus",
    "total_sent": 42,
    "last_notification": "2023-12-12T12:34:56.789Z",
    "error_count": 0
  },
  "timestamp": "2023-12-12T12:34:56.789Z"
}
```

**Статус коды:**
- `200 OK` - Запрос выполнен успешно

**Использование:**
Этот endpoint предоставляет информацию о статусе системы уведомлений и статистике отправленных уведомлений.

---

### GET /api/notifications/config

Получение текущей конфигурации уведомлений.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/notifications/config
```

**Ответ:**
```json
{
  "status": "ok",
  "notifications_config": {
    "enabled": true,
    "backend": "dbus",
    "min_level": "info",
    "rate_limit": {
      "max_per_minute": 10,
      "burst_size": 5
    },
    "formats": {
      "title_template": "SmoothTask: {title}",
      "message_template": "{message}"
    }
  },
  "timestamp": "2023-12-12T12:34:56.789Z"
}
```

**Статус коды:**
- `200 OK` - Запрос выполнен успешно

**Использование:**
Этот endpoint предоставляет текущую конфигурацию системы уведомлений.

---

### POST /api/notifications/config

Обновление конфигурации уведомлений.

**Запрос:**
```bash
curl -X POST "http://127.0.0.1:8080/api/notifications/config" \
  -H "Content-Type: application/json" \
  -d '{"enabled": true, "min_level": "warning", "rate_limit": {"max_per_minute": 5}}'
```

**Параметры запроса (JSON):**
- `enabled` (boolean, optional) - включить/отключить уведомления
- `min_level` (string, optional) - минимальный уровень уведомлений (info, warning, error)
- `rate_limit` (object, optional) - ограничение частоты уведомлений

**Ответ:**
```json
{
  "status": "success",
  "message": "Notifications configuration updated successfully",
  "notifications_config": {
    "enabled": true,
    "backend": "dbus",
    "min_level": "warning",
    "rate_limit": {
      "max_per_minute": 5,
      "burst_size": 5
    }
  },
  "timestamp": "2023-12-12T12:34:56.789Z"
}
```

**Статус коды:**
- `200 OK` - Конфигурация успешно обновлена
- `400 Bad Request` - Некорректные параметры запроса

**Использование:**
Этот endpoint позволяет динамически изменять конфигурацию системы уведомлений.

---

### GET /api/performance

Получение информации о производительности API сервера.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/performance
```

**Успешный ответ:**
```json
{
  "status": "ok",
  "performance_metrics": {
    "total_requests": 1500,
    "cache_hits": 1200,
    "cache_misses": 300,
    "cache_hit_rate": 0.8,
    "average_processing_time_us": 1500,
    "total_processing_time_us": 2250000,
    "last_request_time": 123.45,
    "requests_per_second": 15.5
  },
  "cache_info": {
    "enabled": true,
    "ttl_seconds": null
  }
}
```

**Поля ответа:**
- `status` (string) - статус запроса (`ok`)
- `performance_metrics` (object) - метрики производительности API сервера
- `cache_info` (object) - информация о кэшировании

**Поля объекта performance_metrics:**
- `total_requests` (u64) - общее количество запросов
- `cache_hits` (u64) - количество кэш-попаданий
- `cache_misses` (u64) - количество кэш-промахов
- `cache_hit_rate` (f32) - коэффициент попадания в кэш
- `average_processing_time_us` (u64) - среднее время обработки запроса в микросекундах
- `total_processing_time_us` (u64) - общее время обработки всех запросов в микросекундах
- `last_request_time` (f64, optional) - время с последнего запроса в секундах
- `requests_per_second` (f32) - количество запросов в секунду

**Поля объекта cache_info:**
- `enabled` (bool) - включено ли кэширование
- `ttl_seconds` (u64, optional) - время жизни кэша в секундах (может быть null)

**Требования:**
- Нет специальных требований, доступно всегда

**Примеры использования:**

Получение текущих метрик производительности:
```bash
curl -s http://127.0.0.1:8080/api/performance | jq
```

Мониторинг производительности API:
```bash
watch -n 1 'curl -s http://127.0.0.1:8080/api/performance | jq ".performance_metrics | {total_requests, cache_hit_rate, requests_per_second}"'
```

---

### GET /api/app/performance

Получение информации о производительности приложений.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/app/performance
```

**Успешный ответ:**
```json
{
  "status": "ok",
  "app_performance": [
    {
      "app_group_id": "firefox-1234",
      "app_name": "firefox",
      "sched_latency_p95_ms": 15.5,
      "sched_latency_p99_ms": 25.0,
      "audio_xruns_delta": 0,
      "ui_loop_p95_ms": 12.3,
      "frame_jank_ratio": 0.02,
      "responsiveness_score": 0.95,
      "bad_responsiveness": false,
      "timestamp": "2023-12-12T12:34:56.789Z"
    },
    {
      "app_group_id": "blender-5678",
      "app_name": "blender",
      "sched_latency_p95_ms": 20.0,
      "sched_latency_p99_ms": 35.0,
      "audio_xruns_delta": 2,
      "ui_loop_p95_ms": 18.5,
      "frame_jank_ratio": 0.05,
      "responsiveness_score": 0.88,
      "bad_responsiveness": true,
      "timestamp": "2023-12-12T12:34:56.789Z"
    }
  ],
  "count": 2,
  "timestamp": "2023-12-12T12:34:56.789Z"
}
```

**Поля ответа:**
- `status` (string) - статус запроса (`ok`)
- `app_performance` (array) - массив объектов с информацией о производительности приложений
- `count` (integer) - количество приложений с данными о производительности
- `timestamp` (string) - время генерации ответа в формате RFC3339

**Поля объекта app_performance:**
- `app_group_id` (string) - идентификатор группы приложений
- `app_name` (string) - имя приложения
- `sched_latency_p95_ms` (f64, optional) - 95-й перцентиль задержки планировщика в миллисекундах
- `sched_latency_p99_ms` (f64, optional) - 99-й перцентиль задержки планировщика в миллисекундах
- `audio_xruns_delta` (u64, optional) - количество новых XRUN событий в аудио подсистеме
- `ui_loop_p95_ms` (f64, optional) - 95-й перцентиль времени цикла UI в миллисекундах
- `frame_jank_ratio` (f64, optional) - отношение пропущенных/задержанных кадров
- `responsiveness_score` (f64, optional) - общий балл отзывчивости (0.0 - 1.0, где 1.0 - идеальная отзывчивость)
- `bad_responsiveness` (bool) - флаг, указывающий на плохую отзывчивость приложения
- `timestamp` (string) - временная метка последнего обновления

**Требования:**
- Требуется включенный мониторинг производительности приложений в конфигурации

**Примеры использования:**

Получение текущих метрик производительности приложений:
```bash
curl -s http://127.0.0.1:8080/api/app/performance | jq
```

Мониторинг производительности конкретного приложения:
```bash
curl -s http://127.0.0.1:8080/api/app/performance | \
  jq '.app_performance | map(select(.app_name == "firefox"))'
```

---

### GET /api/logs

Получение логов демона.

**Запрос:**
```bash
curl http://127.0.0.1:8080/api/logs
```

**Параметры запроса (опционально):**
- `level` (string) - фильтрация по уровню логирования (info, warning, error)
- `limit` (integer) - ограничение количества записей
- `fields` (string) - выбор конкретных полей

**Пример с параметрами:**
```bash
curl "http://127.0.0.1:8080/api/logs?level=error&limit=10"
```

**Успешный ответ:**
```json
{
  "status": "ok",
  "logs": [
    {
      "timestamp": "2023-12-12T12:34:56.789Z",
      "level": "info",
      "message": "Daemon started successfully",
      "component": "daemon",
      "details": {
        "version": "0.0.1",
        "config_path": "/etc/smoothtask/config.yml"
      }
    },
    {
      "timestamp": "2023-12-12T12:35:00.123Z",
      "level": "warning",
      "message": "High CPU usage detected",
      "component": "metrics",
      "details": {
        "cpu_usage": 0.95,
        "threshold": 0.9
      }
    }
  ],
  "count": 2,
  "total_available": 150,
  "timestamp": "2023-12-12T12:34:56.789Z"
}
```

**Поля ответа:**
- `status` (string) - статус запроса (`ok`)
- `logs` (array) - массив объектов с логами
- `count` (integer) - количество возвращенных записей
- `total_available` (integer) - общее количество доступных записей
- `timestamp` (string) - время генерации ответа в формате RFC3339

**Поля объекта logs:**
- `timestamp` (string) - временная метка записи
- `level` (string) - уровень логирования
- `message` (string) - сообщение
- `component` (string) - компонент, сгенерировавший запись
- `details` (object, optional) - дополнительные детали

**Требования:**
- Требуется включенное логирование в конфигурации

**Примеры использования:**

Получение последних 10 ошибок:
```bash
curl -s "http://127.0.0.1:8080/api/logs?level=error&limit=10" | jq
```

Мониторинг логов в реальном времени:
```bash
watch -n 1 'curl -s "http://127.0.0.1:8080/api/logs?level=warning&limit=5" | jq .logs'
```

Новые функции ML-классификации, автообновления паттернов и мониторинга производительности приложений значительно расширяют возможности SmoothTask по оптимизации работы системы и обеспечению лучшего пользовательского опыта.

## Примеры интеграции

### Мониторинг системы с Prometheus

SmoothTask API можно интегрировать с Prometheus для долговременного мониторинга:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'smoothtask'
    scrape_interval: 15s
    metrics_path: '/api/metrics'
    static_configs:
      - targets: ['localhost:8080']
```

### Интеграция с Grafana

Создайте дашборд в Grafana с использованием SmoothTask API как источника данных:

```json
{
  "title": "SmoothTask System Monitoring",
  "panels": [
    {
      "title": "CPU Usage",
      "type": "graph",
      "datasource": "Prometheus",
      "targets": [
        {
          "expr": "smoothtask_cpu_usage",
          "legendFormat": "CPU {{cpu_id}}"
        }
      ]
    }
  ]
}
```

### Автоматизация с помощью скриптов

Пример скрипта на Python для мониторинга критических процессов:

```python
import requests
import time

def monitor_critical_processes():
    while True:
        try:
            response = requests.get('http://127.0.0.1:8080/api/processes')
            data = response.json()
            
            if data['status'] == 'ok':
                critical_processes = [p for p in data['processes'] 
                                    if p.get('priority_class') == 'LATENCY_CRITICAL']
                print(f"Critical processes: {len(critical_processes)}")
                for proc in critical_processes:
                    print(f"  PID {proc['pid']}: {proc['name']} ({proc['cpu_usage']:.1f}% CPU)")
        except Exception as e:
            print(f"Error: {e}")
        
        time.sleep(10)

if __name__ == "__main__":
    monitor_critical_processes()
```

### Интеграция с системными уведомлениями

Пример использования API для отправки уведомлений:

```bash
#!/bin/bash

# Получение текущего состояния системы
SYSTEM_STATUS=$(curl -s http://127.0.0.1:8080/api/system | jq -r '.status')

if [ "$SYSTEM_STATUS" != "ok" ]; then
    # Отправка уведомления
    curl -X POST http://127.0.0.1:8080/api/notifications/test 
    -H "Content-Type: application/json" 
    -d '{"message": "System status is not OK!"}'
fi
```

## Расширенные сценарии использования

### Мониторинг производительности приложений

```bash
# Получение производительности конкретного приложения
curl -s "http://127.0.0.1:8080/api/app/performance?app_name=firefox" | jq '.performance_metrics'

# Сравнение производительности нескольких приложений
APPS=("firefox" "chrome" "code")
for app in "${APPS[@]}"; do
    echo "Performance for $app:"
    curl -s "http://127.0.0.1:8080/api/app/performance?app_name=$app" | jq '.performance_metrics.cpu_usage'
done
```

### Анализ сетевой активности

```bash
# Получение топ-10 процессов по сетевой активности
curl -s http://127.0.0.1:8080/api/processes/network | 
  jq '.processes | sort_by(.network_bytes) | reverse | .[0:10] | .[] | "PID \(.pid): \(.name) - \(.network_bytes) bytes"'

# Мониторинг сетевых соединений в реальном времени
watch -n 2 'curl -s http://127.0.0.1:8080/api/network/connections | jq ".connections | length"'
```

### Управление конфигурацией через API

```bash
# Получение текущей конфигурации
curl -s http://127.0.0.1:8080/api/config | jq '.config'

# Перезагрузка конфигурации
curl -X POST http://127.0.0.1:8080/api/config/reload

# Проверка статуса уведомлений
curl -s http://127.0.0.1:8080/api/notifications/status | jq '.status'
```

## Лучшие практики

### Оптимизация производительности API

1. **Используйте кэширование**: Включите кэширование в конфигурации для часто запрашиваемых данных
2. **Ограничивайте количество данных**: Используйте параметры `limit` для больших наборов данных
3. **Фильтруйте данные**: Используйте параметры фильтрации для получения только необходимых данных
4. **Используйте сжатие**: Настройте прокси-сервер (nginx, apache) для сжатия JSON-ответов

### Обработка ошибок

```python
import requests

def safe_api_call(endpoint):
    try:
        response = requests.get(f'http://127.0.0.1:8080{endpoint}', timeout=5)
        response.raise_for_status()
        data = response.json()
        
        if data.get('status') == 'error':
            print(f"API Error: {data.get('error', 'Unknown error')}")
            return None
        
        return data
    except requests.exceptions.RequestException as e:
        print(f"Request failed: {e}")
        return None
    except ValueError as e:
        print(f"JSON parsing failed: {e}")
        return None
```

### Безопасность

1. **Ограничьте доступ**: Настройте фаервол для разрешения доступа только с доверенных IP
2. **Используйте HTTPS**: Настройте обратный прокси с SSL-терминацией
3. **Ограничьте привилегии**: Запускайте демон с минимально необходимыми правами
4. **Мониторьте доступ**: Включите логирование доступа к API

## Конфигурационные примеры

### Базовая конфигурация для разработки

```yaml
# configs/smoothtask-development.yml
paths:
  api_listen_addr: "127.0.0.1:8080"
  config_path: "configs/smoothtask-development.yml"
  patterns_path: "configs/patterns"
  snapshot_db_path: "data/snapshots.db"
  log_path: "logs/smoothtask.log"

polling_interval_ms: 1000
max_candidates: 1000

metrics:
  ebpf:
    enable_network_connections: true
    enable_network_monitoring: true
    enable_high_performance_mode: false

ml_classifier:
  enabled: false
  model_path: "models/process_classifier.json"
  confidence_threshold: 0.7

logging:
  max_file_size_bytes: 10485760
  max_files: 5
  compress_old_files: true
```

### Конфигурация для производственной среды

```yaml
# configs/smoothtask-production.yml
paths:
  api_listen_addr: "0.0.0.0:8080"
  config_path: "/etc/smoothtask/smoothtask.yml"
  patterns_path: "/etc/smoothtask/patterns"
  snapshot_db_path: "/var/lib/smoothtask/snapshots.db"
  log_path: "/var/log/smoothtask/smoothtask.log"

polling_interval_ms: 500
max_candidates: 5000

metrics:
  ebpf:
    enable_network_connections: true
    enable_network_monitoring: true
    enable_high_performance_mode: true
    filter_config:
      enable_kernel_filtering: true
      active_connections_threshold: 50
      network_traffic_threshold: 10240

ml_classifier:
  enabled: true
  model_path: "/usr/share/smoothtask/models/process_classifier.onnx"
  confidence_threshold: 0.8
  model_type: "Onnx"

pattern_auto_update:
  enabled: true
  interval_sec: 3600
  notify_on_update: true

notifications:
  enabled: true
  backend: "Libnotify"
  app_name: "SmoothTask"
  min_level: "Warning"

logging:
  max_file_size_bytes: 52428800
  max_files: 10
  compress_old_files: true

cache_intervals:
  system_metrics: 60
  process_metrics: 30
  app_groups: 60
  responsiveness_metrics: 120
```

### Конфигурация для игровых систем

```yaml
# configs/smoothtask-gaming.yml
paths:
  api_listen_addr: "127.0.0.1:8080"
  config_path: "configs/smoothtask-gaming.yml"
  patterns_path: "configs/patterns"
  snapshot_db_path: "data/snapshots.db"
  log_path: "logs/smoothtask.log"

polling_interval_ms: 250
max_candidates: 2000

metrics:
  ebpf:
    enable_network_connections: true
    enable_network_monitoring: true
    enable_high_performance_mode: true

ml_classifier:
  enabled: true
  model_path: "models/process_classifier_gaming.json"
  confidence_threshold: 0.65

thresholds:
  crit_interactive_percentile: 95
  interactive_percentile: 85
  normal_percentile: 70
  background_percentile: 50

priority_classes:
  - name: "GAME"
    latency_nice: -20
    nice: -20
    cpu_weight: 200
    io_class: "BE"
    io_priority: 0
  - name: "LATENCY_CRITICAL"
    latency_nice: -10
    nice: -10
    cpu_weight: 150
    io_class: "BE"
    io_priority: 1
```

## Устранение неполадок

### Общие проблемы и решения

**Проблема**: API не отвечает на порту 8080

**Решения**:
1. Проверьте, запущен ли демон: `systemctl status smoothtaskd`
2. Проверьте конфигурацию: `grep api_listen_addr /etc/smoothtask/smoothtask.yml`
3. Проверьте логи: `journalctl -u smoothtaskd -n 50`
4. Проверьте фаервол: `sudo ufw status`

**Проблема**: Ошибки доступа при запросе к API

**Решения**:
1. Проверьте права доступа к сокету/порту
2. Убедитесь, что демон запущен от правильного пользователя
3. Проверьте SELinux/AppArmor настройки

**Проблема**: Пустые ответы от API

**Решения**:
1. Дайте демону время для сбора данных (1-2 цикла опроса)
2. Проверьте, что демон имеет доступ к системным ресурсам (/proc, /sys)
3. Проверьте логи демона на ошибки сбора данных

### Диагностические команды

```bash
# Проверка доступности API
curl -I http://127.0.0.1:8080/health

# Проверка сбора метрик
curl -s http://127.0.0.1:8080/api/stats | jq '.metrics_collection'

# Проверка состояния демона
curl -s http://127.0.0.1:8080/api/health | jq '.daemon_status'

# Проверка доступности eBPF
curl -s http://127.0.0.1:8080/api/health | jq '.ebpf_status'
```

## Миграция и обновление

### Обновление с предыдущих версий

```bash
# Остановка старого демона
sudo systemctl stop smoothtaskd

# Резервное копирование конфигурации
sudo cp /etc/smoothtask/smoothtask.yml /etc/smoothtask/smoothtask.yml.bak

# Обновление бинарника
sudo cp target/release/smoothtaskd /usr/local/bin/

# Обновление конфигурации
# Сравните старую и новую конфигурацию, перенесите необходимые настройки

# Запуск нового демона
sudo systemctl start smoothtaskd

# Проверка обновления
curl -s http://127.0.0.1:8080/api/version | jq '.version'
```

### Перенос данных

```bash
# Экспорт данных из старой версии
curl -s http://127.0.0.1:8080/api/logs?limit=1000 > old_logs.json

# Импорт в новую версию
# Используйте API новой версии для импорта данных

# Перенос базы данных снапшотов
sudo cp /var/lib/smoothtask/snapshots.db /var/lib/smoothtask/snapshots.db.bak
```

## Разработка и тестирование

### Локальная разработка

```bash
# Запуск демона в режиме разработки
RUST_LOG=debug cargo run --bin smoothtaskd -- --config configs/smoothtask-development.yml

# Тестирование API
curl -v http://127.0.0.1:8080/api/health

# Запуск тестов
cargo test --package smoothtask-core
```

### Тестирование производительности

```bash
# Тестирование времени ответа API
for i in {1..100}; do
    curl -s -o /dev/null -w "%{time_total}\n" http://127.0.0.1:8080/api/health
done | awk '{sum+=$1; count++} END {print "Average: " sum/count " seconds"}'

# Тестирование под нагрузкой
ab -n 1000 -c 50 http://127.0.0.1:8080/api/health
```

### Отладка

```bash
# Включение детального логирования
RUST_LOG=trace cargo run --bin smoothtaskd

# Просмотр логов в реальном времени
journalctl -u smoothtaskd -f

# Анализ производительности
perf top -p $(pgrep smoothtaskd)
```

## Сообщество и поддержка

### Получение помощи

1. **Документация**: Проверьте официальную документацию и примеры
2. **Исходный код**: Изучите код и комментарии для понимания внутренней работы
3. **Логи**: Анализируйте логи демона для диагностики проблем
4. **Сообщество**: Задайте вопрос в issue tracker проекта

### Сообщение об ошибках

При сообщении об ошибках предоставьте:
1. Версию SmoothTask (`/api/version`)
2. Конфигурационный файл (без чувствительных данных)
3. Логи демона (`journalctl -u smoothtaskd`)
4. Шаги для воспроизведения
5. Ожидаемое и фактическое поведение

### Вклад в проект

1. **Документация**: Улучшение и расширение документации
2. **Тесты**: Добавление новых тестов и улучшение покрытия
3. **Фичи**: Реализация новых функций и улучшений
4. **Баги**: Исправление ошибок и улучшение стабильности

## Заключение

SmoothTask Control API предоставляет мощный и гибкий интерфейс для мониторинга и управления системой. С помощью этого API вы можете интегрировать SmoothTask с существующими системами мониторинга, создавать кастомные дашборды и автоматизировать управление системными ресурсами.

Для получения дополнительной информации обратитесь к официальной документации и исходному коду проекта.
