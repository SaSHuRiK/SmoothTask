# SmoothTask

**SmoothTask — чтобы система оставалась отзывчивой даже на 100% CPU.**

Системный демон для Linux, который автоматически управляет приоритетами процессов (nice, latency_nice, IO, cgroups), чтобы интерактивные приложения оставались максимально отзывчивыми, а фоновые задачи не «убивали» систему.

[![GitHub](https://img.shields.io/badge/GitHub-SmoothTask-blue)](https://github.com/SaSHuRiK/SmoothTask)

## Архитектура

- **Rust-демон** (`smoothtaskd`) — быстрый демон для сбора метрик, применения правил и ML-ранкера
- **Python-тренер** (`smoothtask-trainer`) — офлайн-обучение CatBoostRanker на основе собранных снапшотов

## Основные возможности

### ML-классификация процессов

SmoothTask поддерживает ML-классификацию процессов для более точного определения типов процессов:
- **CatBoost JSON модели** — простой формат для тестирования и отладки
- **ONNX модели** — оптимизированный формат для production использования
- **Гибкая конфигурация** — настройка порога уверенности и приоритетов
- **Автоматическое переопределение** — ML-результаты могут переопределять паттерн-классификацию

### Автообновление паттерн-базы

Автоматическое обновление паттернов без перезапуска демона:
- **Мониторинг изменений** — отслеживание добавления/изменения/удаления паттернов
- **Горячая перезагрузка** — новые паттерны применяются без перезапуска
- **Уведомления** — оповещения об обновлениях паттерн-базы
- **Периодическая проверка** — регулярное сканирование директории с паттернами

### Расширенный мониторинг производительности

Детальные метрики производительности на уровне приложений:
- **Задержка отклика** — мониторинг отзывчивости приложений
- **FPS для графических приложений** — контроль производительности графики
- **Использование ресурсов** — CPU, память, потоки на уровне процессов
- **История метрик** — сохранение и анализ временных рядов

## Тестирование

SmoothTask включает comprehensive тесты для обеспечения надежности:

### Интеграционные тесты для ML-классификатора

```bash
# Запуск интеграционных тестов для ML-классификатора
cargo test --test ml_classifier_integration_test
```

Тесты покрывают:
- Интеграцию ML-классификатора с системой паттерн-классификации
- Взаимодействие с PatternWatcher
- Обработку ошибок и fallback механизмы
- Тестирование порогов уверенности
- Извлечение фич и объединение тегов
- Производительность и надежность

### Unit-тесты

```bash
# Запуск всех unit-тестов
cargo test
```

### Интеграционные тесты

```bash
# Запуск всех интеграционных тестов
cargo test --tests
```

## Быстрый старт

### Сборка

```bash
cargo build --release
```

### Запуск

```bash
sudo ./target/release/smoothtaskd --config configs/smoothtask.example.yml
```

### Настройка systemd (для автозапуска)

Для автоматического запуска демона при загрузке системы:

1. Установите бинарник в `/usr/local/bin/`:
   ```bash
   sudo cp target/release/smoothtaskd /usr/local/bin/
   ```

2. Создайте конфигурационную директорию:
   ```bash
   sudo mkdir -p /etc/smoothtask/
   sudo cp configs/smoothtask.example.yml /etc/smoothtask/smoothtask.yml
   ```

3. Создайте директорию для данных:
   ```bash
   sudo mkdir -p /var/lib/smoothtask/
   sudo chown root:root /var/lib/smoothtask
   ```

4. Установите systemd unit файл:
   ```bash
   sudo cp systemd/smoothtaskd.service /etc/systemd/system/
   sudo systemctl daemon-reload
   ```

5. Включите и запустите сервис:
   ```bash
   sudo systemctl enable smoothtaskd.service
   sudo systemctl start smoothtaskd.service
   ```

6. Проверьте статус:
   ```bash
   sudo systemctl status smoothtaskd.service
   ```

Подробная документация по systemd доступна в [systemd/README.md](systemd/README.md).

## Примеры использования

### Использование ML-классификатора

Запуск с ML-классификатором (CatBoost JSON):
```bash
sudo ./target/release/smoothtaskd --config configs/examples/smoothtask-ml-enabled.yml
```

Запуск с ONNX моделью:
```bash
sudo ./target/release/smoothtaskd --config configs/examples/smoothtask-ml-onnx.yml
```

### Настройка автообновления паттернов

Пример конфигурации с автообновлением:
```yaml
pattern_auto_update:
  enabled: true
  interval_sec: 60
  notify_on_update: true
```

### Мониторинг производительности

Получение метрик производительности через API:
```bash
curl http://127.0.0.1:8080/api/metrics/app_performance
```

### Обучение ML-модели

Обучение модели с использованием тренера:
```bash
cd smoothtask-trainer
python -m smoothtask_trainer.train_ranker --input data/snapshots --output models/process_classifier.json
```

Экспорт модели в ONNX формат:
```bash
python -m smoothtask_trainer.export_model --input models/process_classifier.json --output models/process_classifier.onnx
```

### Пример использования ML-классификатора с PatternWatcher

Запуск с ML-классификатором и автоматической перезагрузкой паттернов:
```bash
sudo ./target/release/smoothtaskd --config configs/examples/smoothtask-ml-patternwatcher.yml
```

Этот пример демонстрирует:
- Автоматическую загрузку и использование ML-модели
- Мониторинг изменений в директории паттернов
- Автоматическую перезагрузку паттернов без перезапуска демона
- Интеграцию ML-классификации с паттерн-классификацией

### Пример мониторинга производительности приложений

Комплексный мониторинг производительности с использованием API:
```bash
#!/bin/bash
# Мониторинг производительности приложений с использованием SmoothTask API

API_URL="http://127.0.0.1:8080/api/metrics/app_performance"

# Получение метрик производительности
PERF_DATA=$(curl -s "$API_URL")

# Анализ производительности приложений
echo "$PERF_DATA" | jq -c '.app_performance | .[]' | while read app; do
    APP_NAME=$(echo "$app" | jq -r '.app_name')
    RESPONSE_TIME=$(echo "$app" | jq '.response_time_ms')
    CPU_USAGE=$(echo "$app" | jq '.cpu_usage_percent')
    
    echo "App: $APP_NAME, Response: ${RESPONSE_TIME}ms, CPU: ${CPU_USAGE}%"
    
    # Проверка на критическое состояние
    if (( $(echo "$RESPONSE_TIME > 100.0" | bc -l) )); then
        echo "WARNING: High response time for $APP_NAME!" | logger -t smoothtask-perf
    fi
done
```

### Пример интеграции с системой мониторинга

Интеграция SmoothTask с Prometheus для мониторинга:
```yaml
# Конфигурация Prometheus для сбора метрик SmoothTask
scrape_configs:
  - job_name: 'smoothtask'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/api/metrics/app_performance'
    scrape_interval: 15s
```

### Пример использования API для управления конфигурацией

Получение и обновление текущей конфигурации:
```bash
# Получение текущей конфигурации
curl http://127.0.0.1:8080/api/config

# Обновление конфигурации (если поддерживается)
curl -X POST http://127.0.0.1:8080/api/config \
  -H "Content-Type: application/json" \
  -d '{"ml_classifier": {"enabled": true}}'
```

## Документация

См. [docs/tz.md](docs/tz.md) для полного технического задания.

- 📖 [Руководство по CatBoost v1](docs/CATBOOST_V1_GUIDE.md) - обучение моделей, ONNX интеграция и использование ML-ранкера

## Статус проекта

✅ **MVP (rules-only) завершено** — базовая функциональность реализована и работает.

✅ **CatBoost v1 завершено** — ML-ранкер реализован и протестирован.

Текущий этап: проект находится в активной разработке. Реализованы:
- Метрики системы и процессов
- Группировка процессов и классификация по правилам
- Применение приоритетов через cgroups v2, latency_nice, nice и ionice
- Snapshot Logger для сбора данных
- CatBoost Ranker с ONNX Runtime интеграцией
- Режим dry-run и hybrid режим

См. [Roadmap](docs/ROADMAP.md) для подробной информации о текущем состоянии и планах.

## API для мониторинга

SmoothTask предоставляет HTTP API для мониторинга работы демона и просмотра текущего состояния системы. API позволяет получать информацию о процессах, метриках, группах приложений и многом другом.

**Основные возможности:**

- 📊 Получение системных метрик (CPU, память, PSI)
- 🔍 Просмотр списка процессов и их приоритетов
- 🎯 Мониторинг групп приложений и их приоритетов
- ⚙️ Просмотр текущей конфигурации демона
- 📋 Получение информации о классах QoS и паттернах

**Примеры использования:**

```bash
# Проверка работоспособности API
curl http://127.0.0.1:8080/health

# Получение статистики демона
curl http://127.0.0.1:8080/api/stats

# Получение системных метрик
curl http://127.0.0.1:8080/api/metrics

# Получение списка процессов
curl http://127.0.0.1:8080/api/processes

# Получение информации о конкретном процессе
curl http://127.0.0.1:8080/api/processes/1234

# Получение списка групп приложений
curl http://127.0.0.1:8080/api/appgroups

# Получение информации о конкретной группе
curl http://127.0.0.1:8080/api/appgroups/firefox-1234

# Получение текущей конфигурации
curl http://127.0.0.1:8080/api/config

# Получение информации о классах QoS
curl http://127.0.0.1:8080/api/classes

# Получение списка всех доступных endpoints
curl http://127.0.0.1:8080/api/endpoints

# Получение информации о системе
curl http://127.0.0.1:8080/api/system

# Получение информации о загруженных паттернах
curl http://127.0.0.1:8080/api/patterns
```

**Практические примеры использования:**

```bash
# Мониторинг загрузки системы с выводом в формате для Grafana
curl -s http://127.0.0.1:8080/api/metrics | jq '.cpu_usage'

# Получение информации о топ-5 процессов по использованию CPU
curl -s http://127.0.0.1:8080/api/processes | jq '.processes | sort_by(.cpu_usage) | reverse | .[0:5]'

# Проверка отзывчивости системы
curl -s http://127.0.0.1:8080/api/responsiveness | jq '.latency_stats'

# Мониторинг групп приложений с фильтрацией по приоритету
curl -s http://127.0.0.1:8080/api/appgroups | jq '.groups | .[] | select(.priority_class == "LATENCY_CRITICAL")'
```

**Интеграция с системами мониторинга:**

API можно использовать для интеграции с системами мониторинга, такими как Prometheus, Grafana, Zabbix и другими. Для этого можно создать простые скрипты, которые будут опрашивать API и предоставлять данные в нужном формате.

**Пример скрипта для Prometheus:**

```bash
#!/bin/bash
# smoothtask_exporter.sh - экспортер метрик для Prometheus

# Получение метрик системы
SMOOTHTASK_METRICS=$(curl -s http://127.0.0.1:8080/api/metrics)
CPU_USAGE=$(echo "$SMOOTHTASK_METRICS" | jq '.cpu_usage.total')
MEM_USAGE=$(echo "$SMOOTHTASK_METRICS" | jq '.memory.used_kb')

# Вывод в формате Prometheus
cat <<EOF
# HELP smoothtask_cpu_usage_total Total CPU usage percentage
# TYPE smoothtask_cpu_usage_total gauge
smoothtask_cpu_usage_total $CPU_USAGE

# HELP smoothtask_memory_used_kb Memory used in KB
# TYPE smoothtask_memory_used_kb gauge
smoothtask_memory_used_kb $MEM_USAGE
EOF
```

**Пример использования в автоматизации:**

```bash
#!/bin/bash
# Автоматическое обнаружение и мониторинг критичных процессов

# Получение списка процессов с высоким приоритетом
HIGH_PRIO_PROCESSES=$(curl -s http://127.0.0.1:8080/api/processes | \
    jq '.processes | .[] | select(.priority_class == "LATENCY_CRITICAL") | .name')

# Логирование и оповещение
echo "High priority processes: $HIGH_PRIO_PROCESSES"
if [[ -n "$HIGH_PRIO_PROCESSES" ]]; then
    # Отправка оповещения в систему мониторинга
    echo "Critical processes detected: $HIGH_PRIO_PROCESSES" | logger -t smoothtask-monitor
fi
```

**Пример комплексного мониторинга системы:**

```bash
#!/bin/bash
# Комплексный скрипт мониторинга системы с использованием SmoothTask API

# Получение системных метрик
SYSTEM_METRICS=$(curl -s http://127.0.0.1:8080/api/metrics)
CPU_USAGE=$(echo "$SYSTEM_METRICS" | jq '.cpu_usage.total')
MEM_USAGE=$(echo "$SYSTEM_METRICS" | jq '.memory.used_kb')
SWAP_USAGE=$(echo "$SYSTEM_METRICS" | jq '.memory.swap_used_kb')

# Получение метрик отзывчивости
RESPONSIVENESS=$(curl -s http://127.0.0.1:8080/api/responsiveness)
LATENCY_P99=$(echo "$RESPONSIVENESS" | jq '.latency_stats.p99_ms')

# Получение статистики демона
DAEMON_STATS=$(curl -s http://127.0.0.1:8080/api/stats)
TOTAL_ITERATIONS=$(echo "$DAEMON_STATS" | jq '.daemon_stats.total_iterations')

# Вывод комплексного отчета
echo "=== System Health Report ==="
echo "CPU Usage: ${CPU_USAGE}%"
echo "Memory Used: ${MEM_USAGE} KB"
echo "Swap Used: ${SWAP_USAGE} KB"
echo "Latency P99: ${LATENCY_P99} ms"
echo "Daemon Iterations: ${TOTAL_ITERATIONS}"

# Проверка на критическое состояние
if (( $(echo "$CPU_USAGE > 90.0" | bc -l) )); then
    echo "WARNING: High CPU usage detected!" | logger -t smoothtask-monitor
fi

if (( $(echo "$LATENCY_P99 > 50.0" | bc -l) )); then
    echo "WARNING: High system latency detected!" | logger -t smoothtask-monitor
fi
```

**Пример интеграции с Prometheus для расширенного мониторинга:**

```bash
#!/bin/bash
# Расширенный экспортер метрик для Prometheus

# Получение полных системных метрик
METRICS=$(curl -s http://127.0.0.1:8080/api/metrics)

# Экспорт метрик CPU
CPU_USER=$(echo "$METRICS" | jq '.cpu_usage.user')
CPU_SYSTEM=$(echo "$METRICS" | jq '.cpu_usage.system')
CPU_IDLE=$(echo "$METRICS" | jq '.cpu_usage.idle')

# Экспорт метрик памяти
MEM_TOTAL=$(echo "$METRICS" | jq '.memory.mem_total_kb')
MEM_USED=$(echo "$METRICS" | jq '.memory.mem_used_kb')
MEM_AVAILABLE=$(echo "$METRICS" | jq '.memory.mem_available_kb')

# Экспорт метрик PSI
PSI_CPU_SOME=$(echo "$METRICS" | jq '.pressure.cpu.some.avg10')
PSI_IO_SOME=$(echo "$METRICS" | jq '.pressure.io.some.avg10')
PSI_MEM_SOME=$(echo "$METRICS" | jq '.pressure.memory.some.avg10')

# Вывод в формате Prometheus
cat <<EOF
# HELP smoothtask_cpu_user CPU user usage percentage
# TYPE smoothtask_cpu_user gauge
smoothtask_cpu_user ${CPU_USER}

# HELP smoothtask_cpu_system CPU system usage percentage
# TYPE smoothtask_cpu_system gauge
smoothtask_cpu_system ${CPU_SYSTEM}

# HELP smoothtask_cpu_idle CPU idle percentage
# TYPE smoothtask_cpu_idle gauge
smoothtask_cpu_idle ${CPU_IDLE}

# HELP smoothtask_memory_total Total memory in KB
# TYPE smoothtask_memory_total gauge
smoothtask_memory_total ${MEM_TOTAL}

# HELP smoothtask_memory_used Used memory in KB
# TYPE smoothtask_memory_used gauge
smoothtask_memory_used ${MEM_USED}

# HELP smoothtask_memory_available Available memory in KB
# TYPE smoothtask_memory_available gauge
smoothtask_memory_available ${MEM_AVAILABLE}

# HELP smoothtask_psi_cpu_some CPU pressure (some) avg10
# TYPE smoothtask_psi_cpu_some gauge
smoothtask_psi_cpu_some ${PSI_CPU_SOME}

# HELP smoothtask_psi_io_some IO pressure (some) avg10
# TYPE smoothtask_psi_io_some gauge
smoothtask_psi_io_some ${PSI_IO_SOME}

# HELP smoothtask_psi_mem_some Memory pressure (some) avg10
# TYPE smoothtask_psi_mem_some gauge
smoothtask_psi_mem_some ${PSI_MEM_SOME}
EOF
```

**Пример использования API для анализа производительности приложений:**

```bash
#!/bin/bash
# Анализ производительности конкретного приложения

APP_NAME="firefox"

# Получение информации о процессах приложения
PROCESSES=$(curl -s http://127.0.0.1:8080/api/processes | \
    jq --arg app "$APP_NAME" '.processes | .[] | select(.cmdline | contains($app))')

# Анализ использования ресурсов
TOTAL_CPU=0
TOTAL_MEM=0
PROCESS_COUNT=0

for process in $(echo "$PROCESSES" | jq -c '.'); do
    CPU=$(echo "$process" | jq '.cpu_share_1s')
    MEM=$(echo "$process" | jq '.rss_mb')
    
    if [[ "$CPU" != "null" ]]; then
        TOTAL_CPU=$(echo "$TOTAL_CPU + $CPU" | bc)
    fi
    
    if [[ "$MEM" != "null" ]]; then
        TOTAL_MEM=$(echo "$TOTAL_MEM + $MEM" | bc)
    fi
    
    PROCESS_COUNT=$((PROCESS_COUNT + 1))
done

echo "=== $APP_NAME Performance Analysis ==="
echo "Process Count: $PROCESS_COUNT"
echo "Total CPU Usage: ${TOTAL_CPU}%"
echo "Total Memory Usage: ${TOTAL_MEM} MB"

# Получение информации о группе приложения
APP_GROUP=$(curl -s http://127.0.0.1:8080/api/appgroups | \
    jq --arg app "$APP_NAME" '.app_groups | .[] | select(.app_name | contains($app)) | .priority_class')

echo "Priority Class: ${APP_GROUP:-Not found}"
```

**Документация API:**

Подробная документация API доступна в [docs/API.md](docs/API.md).

## Устранение неполадок

### Демон не запускается

**Проблема:** Демон не запускается или сразу завершается.

**Решения:**

1. **Проверьте права доступа:**
   ```bash
   sudo chmod +x /usr/local/bin/smoothtaskd
   sudo chown root:root /usr/local/bin/smoothtaskd
   ```

2. **Проверьте конфигурационный файл:**
   ```bash
   sudo /usr/local/bin/smoothtaskd --config /etc/smoothtask/smoothtask.yml --validate-config
   ```

3. **Проверьте логи:**
   ```bash
   sudo journalctl -u smoothtaskd.service -f
   ```

4. **Запустите вручную для отладки:**
   ```bash
   sudo /usr/local/bin/smoothtaskd --config /etc/smoothtask/smoothtask.yml --debug
   ```

### Ошибки доступа к /proc

**Проблема:** Ошибки "Permission denied" при доступе к /proc.

**Решения:**

1. **Запускайте демон от root:**
   ```bash
   sudo systemctl restart smoothtaskd.service
   ```

2. **Проверьте монтирование /proc:**
   ```bash
   mount | grep proc
   ```

3. **Проверьте права доступа:**
   ```bash
   ls -la /proc/1
   ```

### Проблемы с cgroups v2

**Проблема:** Ошибки при работе с cgroups v2.

**Решения:**

1. **Проверьте версию cgroups:**
   ```bash
   stat -fc %T /sys/fs/cgroup/
   ```
   Должно вернуть `cgroup2fs` для cgroups v2.

2. **Проверьте монтирование cgroups:**
   ```bash
   mount | grep cgroup2
   ```

3. **Проверьте права доступа:**
   ```bash
   ls -la /sys/fs/cgroup/
   ```

### Проблемы с Wayland

**Проблема:** Ошибки при работе с Wayland.

**Решения:**

1. **Проверьте переменную окружения WAYLAND_DISPLAY:**
   ```bash
   echo $WAYLAND_DISPLAY
   ```

2. **Проверьте доступность Wayland:**
   ```bash
   ls -la /run/user/$(id -u)/wayland-1
   ```

3. **Проверьте поддержку композитором:**
   ```bash
   echo $XDG_CURRENT_DESKTOP
   ```

### Проблемы с API

**Проблема:** API сервер не отвечает.

**Решения:**

1. **Проверьте, что API включен в конфигурации:**
   ```yaml
   paths:
     api_listen_addr: "127.0.0.1:8080"
   ```

2. **Проверьте порт:**
   ```bash
   ss -tulnp | grep 8080
   ```

3. **Проверьте статус демона:**
   ```bash
   sudo systemctl status smoothtaskd.service
   ```

4. **Проверьте брандмауэр:**
   ```bash
   sudo ufw status
   sudo iptables -L
   ```

### Проблемы с производительностью

**Проблема:** Высокая нагрузка от демона.

**Решения:**

1. **Увеличьте интервал опроса:**
   ```yaml
   polling_interval_ms: 1000  # вместо 500
   ```

2. **Уменьшите количество кандидатов:**
   ```yaml
   max_candidates: 100  # вместо 150
   ```

3. **Проверьте логи:**
   ```bash
   sudo journalctl -u smoothtaskd.service | grep "performance"
   ```

### Общие советы по отладке

1. **Включите режим отладки:**
   ```bash
   sudo /usr/local/bin/smoothtaskd --config /etc/smoothtask/smoothtask.yml --debug
   ```

2. **Проверьте системные логи:**
   ```bash
   sudo dmesg | tail -20
   ```

3. **Проверьте использование ресурсов:**
   ```bash
   top -p $(pidof smoothtaskd)
   ```

## Ссылки

- 📖 [Техническое задание](docs/tz.md)
- 🔧 [Руководство по установке](docs/SETUP_GUIDE.md)
- 🔍 [Исследование паттерн-базы приложений](docs/PATTERNS_RESEARCH.md)
- 🔬 [Исследование существующих решений](docs/EXISTING_SOLUTIONS_RESEARCH.md)
- ⚡ [Исследование низко-латентных практик](docs/LOW_LATENCY_RESEARCH.md)
- 🪟 [Исследование API композиторов и аудио-стеков](docs/API_INTROSPECTION_RESEARCH.md)
- 📈 [Исследование поведенческих паттернов приложений](docs/BEHAVIORAL_PATTERNS_RESEARCH.md)
- 🏗️ [Архитектура](docs/ARCHITECTURE.md)
- 📊 [Метрики](docs/METRICS.md)
- ⚙️ [Политика приоритетов](docs/POLICY.md)
- 🗺️ [Roadmap](docs/ROADMAP.md)

## Лицензия

MIT License

Copyright (c) 2025 SmoothTask Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

