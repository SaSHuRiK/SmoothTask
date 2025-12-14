# Начало работы с SmoothTask

Добро пожаловать в SmoothTask! Это руководство поможет вам быстро начать работу с системой интеллектуального управления приоритетами процессов.

## 🚀 Быстрый старт

### Предварительные требования

Перед началом работы убедитесь, что у вас установлены следующие компоненты:

- **Linux система** (SmoothTask работает только на Linux)
- **Rust** (версия 1.70 или новее)
- **Python 3.8+** (для тренера)
- **Git** (для клонирования репозитория)

### Установка

#### 1. Клонируйте репозиторий

```bash
git clone https://github.com/your-repo/SmoothTask.git
cd SmoothTask
```

#### 2. Соберите проект

```bash
# Собрать Rust-компоненты
cargo build --release

# Установить Python-зависимости для тренера
cd smoothtask-trainer
pip install -r requirements.txt
cd ..
```

#### 3. Настройте конфигурацию

Скопируйте пример конфигурации и отредактируйте его:

```bash
cp configs/smoothtask.example.yml /etc/smoothtask/smoothtask.yml
# Отредактируйте конфигурацию по вашим нуждам
```

#### 4. Запустите демон

```bash
# Запуск в режиме dry-run (без применения изменений)
./target/release/smoothtaskd --dry-run

# Запуск в производственном режиме
./target/release/smoothtaskd
```

## 📖 Основные концепции

### Архитектура

SmoothTask состоит из двух основных компонентов:

1. **Демон** (`smoothtaskd`) - работает в реальном времени, собирает метрики и управляет приоритетами
2. **Тренер** (`smoothtask-trainer`) - офлайн-инструменты для обучения ML-моделей

### Основные компоненты

- **Metrics Collector**: Сбор системных и процессных метрик, включая GPU, виртуальные машины и контейнеры
- **Process Grouper**: Группировка процессов по приложениям
- **Process Classifier**: Классификация процессов по типам с использованием ML
- **Policy Engine**: Определение целевых приоритетов с поддержкой пользовательских метрик
- **Actuator**: Применение изменений приоритетов
- **Custom Metrics Manager**: Управление пользовательскими метриками через API

### Новые возможности

- **Пользовательские метрики**: Возможность определять собственные метрики через файлы, команды, HTTP API или статические значения
- **Расширенный мониторинг**: Поддержка GPU (NVML, AMDGPU), виртуальных машин и контейнеров (Kubernetes, CRI-O, Rkt)
- **Улучшенная ML-классификация**: Кэширование фич и оптимизация производительности
- **API управления**: Полный REST API для управления пользовательскими метриками

## 🎯 Типичные сценарии использования

### Для разработчиков

```yaml
# configs/smoothtask-development.yml
priority_rules:
  - name: "Boost IDE processes"
    match:
      tags: ["ide"]
    priority: "interactive"
  
  - name: "Lower build processes"
    match:
      command: ["make", "cargo", "npm"]
    priority: "background"
```

### Для геймеров

```yaml
# configs/smoothtask-gaming.yml
priority_rules:
  - name: "Maximize game performance"
    match:
      tags: ["game"]
    priority: "latency_critical"
  
  - name: "Lower background processes"
    match:
      type: ["daemon", "batch"]
    priority: "idle"
```

### Для серверов

```yaml
# configs/smoothtask-server.yml
priority_rules:
  - name: "Prioritize web services"
    match:
      command: ["nginx", "apache", "node"]
    priority: "interactive"
  
  - name: "Limit background jobs"
    match:
      type: ["batch"]
    priority: "background"
```

## 🔧 Настройка

### Основные параметры конфигурации

```yaml
# Основные настройки
polling_interval_ms: 1000  # Интервал опроса метрик
enable_snapshot_logging: true  # Логирование снапшотов

# Пути
paths:
  snapshot_db_path: "/var/lib/smoothtask/snapshots.db"
  patterns_dir: "/etc/smoothtask/patterns"
  log_file_path: "/var/log/smoothtask/smoothtask.log"

# Пороги
thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
```

### Настройка приоритетов

```yaml
priority_rules:
  - name: "Critical applications"
    match:
      tags: ["audio", "video", "game"]
    priority: "latency_critical"
    
  - name: "Interactive applications"
    match:
      tags: ["browser", "ide", "terminal"]
    priority: "interactive"
    
  - name: "Background processes"
    match:
      type: ["daemon", "batch"]
    priority: "background"
```

## 📊 Мониторинг и логирование

### Просмотр логов

```bash
# Просмотр основных логов
journalctl -u smoothtaskd -f

# Просмотр снапшотов (SQLite)
sqlite3 /var/lib/smoothtask/snapshots.db "SELECT * FROM snapshots LIMIT 10;"
```

### Prometheus и Grafana

SmoothTask поддерживает интеграцию с Prometheus и Grafana:

```yaml
# Включение Prometheus метрик
paths:
  api_listen_addr: "0.0.0.0:8080"

# Настройка Grafana дашборда
# См. monitoring/grafana/dashboards/
```

## 🖥️ Использование API

SmoothTask предоставляет REST API для динамического управления конфигурацией и мониторинга состояния системы.

### Основные API endpoints

#### Получение текущей конфигурации

```bash
# Получение полной конфигурации
curl http://127.0.0.1:8080/api/config | jq

# Получение только основных параметров
curl http://127.0.0.1:8080/api/config | jq '.config | {polling_interval_ms, max_candidates, policy_mode}'
```

#### Динамическое обновление конфигурации

Новые возможности позволяют обновлять конфигурацию без перезагрузки демона:

```bash
# Обновление интервала опроса
curl -X POST http://127.0.0.1:8080/api/config \
  -H "Content-Type: application/json" \
  -d '{"polling_interval_ms": 2000}'

# Переключение режима политики на гибридный (rules + ML)
curl -X POST http://127.0.0.1:8080/api/config \
  -H "Content-Type: application/json" \
  -d '{"policy_mode": "hybrid"}'

# Включение логирования снапшотов
curl -X POST http://127.0.0.1:8080/api/config \
  -H "Content-Type: application/json" \
  -d '{"enable_snapshot_logging": true}'

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

#### Перезагрузка конфигурации из файла

```bash
# Перезагрузка конфигурации из файла
curl -X POST http://127.0.0.1:8080/api/config/reload
```

#### Управление кэшем процессов

```bash
# Получение текущей конфигурации кэша
curl http://127.0.0.1:8080/api/cache/config

# Обновление параметров кэша
curl -X POST http://127.0.0.1:8080/api/cache/config \
  -H "Content-Type: application/json" \
  -d '{
    "cache_ttl_seconds": 300,
    "max_cached_processes": 500,
    "enable_caching": true
  }'
```

### Примеры использования в скриптах

#### Автоматическая настройка для игрового режима

```bash
#!/bin/bash
# Настройка SmoothTask для игрового режима (максимальная отзывчивость)

# Установить высокий приоритет для интерактивных процессов
curl -X POST http://127.0.0.1:8080/api/config \
  -H "Content-Type: application/json" \
  -d '{
    "policy_mode": "hybrid",
    "max_candidates": 300
  }'

# Увеличить интервал опроса для снижения нагрузки
curl -X POST http://127.0.0.1:8080/api/config \
  -H "Content-Type: application/json" \
  -d '{"polling_interval_ms": 500}'

echo "SmoothTask настроен для игрового режима"
```

#### Мониторинг и настройка через Python

```python
import requests
import json

# Получение текущей конфигурации
response = requests.get("http://127.0.0.1:8080/api/config")
config = response.json()

print(f"Текущий интервал опроса: {config['config']['polling_interval_ms']}ms")
print(f"Текущий режим политики: {config['config']['policy_mode']}")

# Обновление конфигурации
new_config = {
    "polling_interval_ms": 2000,
    "policy_mode": "hybrid"
}

update_response = requests.post(
    "http://127.0.0.1:8080/api/config",
    json=new_config,
    headers={"Content-Type": "application/json"}
)

print(f"Обновление: {update_response.json()['message']}")
```

### Интеграция с системами мониторинга

Вы можете интегрировать SmoothTask API с Prometheus, Zabbix или другими системами мониторинга:

```bash
# Получение метрик через API (для Prometheus exporter)
curl http://127.0.0.1:8080/api/stats | jq '.metrics.system'

# Проверка состояния демона
curl http://127.0.0.1:8080/api/health
```

**Дополнительная информация:**
- Полная документация API доступна в [API.md](API.md)
- Все изменения конфигурации применяются немедленно
- Для сложных сценариев используйте комбинацию API и конфигурационных файлов

## 💻 Мониторинг аппаратных устройств

SmoothTask предоставляет расширенные возможности мониторинга аппаратных устройств, которые можно использовать для диагностики и оптимизации системы.

### Включение мониторинга аппаратных устройств

По умолчанию мониторинг аппаратных устройств включен. Вы можете настроить его в конфигурационном файле:

```yaml
metrics:
  hardware:
    enable_pci_monitoring: true
    enable_usb_monitoring: true
    enable_storage_monitoring: true
    enable_temperature_monitoring: true
    enable_power_monitoring: true
```

### Примеры использования

#### Просмотр информации о PCI устройствах

```bash
# Получение информации о всех аппаратных устройствах
curl http://127.0.0.1:8080/api/system | jq '.system_metrics.hardware.pci_devices'

# Фильтрация по температуре
curl http://127.0.0.1:8080/api/system | jq '.system_metrics.hardware.pci_devices[] | select(.temperature_c > 70)'
```

#### Мониторинг температуры устройств

```bash
#!/bin/bash

# Мониторинг температуры устройств
while true; do
    clear
    echo "=== Device Temperature Monitor ==="
    
    # PCI устройства
    echo -e "\nPCI Devices:"
    curl -s http://127.0.0.1:8080/api/system | \
        jq -r '.system_metrics.hardware.pci_devices[] | select(.temperature_c) | "  \(.device_id): \(.temperature_c)°C"'
    
    # USB устройства
    echo -e "\nUSB Devices:"
    curl -s http://127.0.0.1:8080/api/system | \
        jq -r '.system_metrics.hardware.usb_devices[] | select(.temperature_c) | "  \(.device_id): \(.temperature_c)°C"'
    
    # Устройства хранения
    echo -e "\nStorage Devices:"
    curl -s http://127.0.0.1:8080/api/system | \
        jq -r '.system_metrics.hardware.storage_devices[] | select(.temperature_c) | "  \(.device_id): \(.temperature_c)°C"'
    
    sleep 5
done
```

#### Проверка состояния здоровья устройств хранения

```bash
#!/bin/bash

# Проверка состояния здоровья устройств хранения
response=$(curl -s http://127.0.0.1:8080/api/system)

if [ $? -eq 0 ]; then
    echo "Storage Device Health Check:"
    echo "$response" | jq -r '.system_metrics.hardware.storage_devices[] | "\(.device_id) (\(.model)): \(.health_status // "unknown")"'
    
    # Проверка на проблемы
    unhealthy=$(echo "$response" | jq -r '.system_metrics.hardware.storage_devices[] | select(.health_status != "good" and .health_status != null) | .device_id')
    
    if [ -n "$unhealthy" ]; then
        echo -e "\nWARNING: Unhealthy devices detected:"
        echo "$unhealthy"
    else
        echo -e "\nAll devices are healthy!"
    fi
else
    echo "Failed to fetch storage health information"
fi
```

### Интеграция с системами мониторинга

#### Prometheus + Grafana

1. **Настройка Prometheus** (`prometheus.yml`):

```yaml
scrape_configs:
  - job_name: 'smoothtask'
    scrape_interval: 15s
    metrics_path: '/api/system'
    static_configs:
      - targets: ['localhost:8080']
```

2. **Создание дашборда Grafana**:

- Добавьте панель для отображения температуры устройств
- Создайте алерты для высоких температур (например, > 80°C)
- Настройте панель для отображения состояния здоровья устройств

#### Python скрипт для мониторинга

```python
import requests
import time
import json

def monitor_hardware():
    """Мониторинг аппаратных устройств"""
    
    while True:
        try:
            response = requests.get("http://127.0.0.1:8080/api/system")
            if response.status_code == 200:
                data = response.json()
                hardware = data.get("system_metrics", {}).get("hardware", {})
                
                # Проверка температуры
                devices = []
                
                for pci in hardware.get("pci_devices", []):
                    if "temperature_c" in pci:
                        devices.append({
                            "type": "PCI",
                            "id": pci["device_id"],
                            "temp": pci["temperature_c"],
                            "critical": pci["temperature_c"] > 80
                        })
                
                for usb in hardware.get("usb_devices", []):
                    if "temperature_c" in usb:
                        devices.append({
                            "type": "USB",
                            "id": usb["device_id"],
                            "temp": usb["temperature_c"],
                            "critical": usb["temperature_c"] > 60
                        })
                
                for storage in hardware.get("storage_devices", []):
                    if "temperature_c" in storage:
                        devices.append({
                            "type": "Storage",
                            "id": storage["device_id"],
                            "temp": storage["temperature_c"],
                            "critical": storage["temperature_c"] > 65
                        })
                
                # Вывод информации
                print(f"\n=== Hardware Monitor ({time.strftime('%H:%M:%S')}) ===")
                for device in sorted(devices, key=lambda x: x["temp"], reverse=True):
                    status = "⚠️ CRITICAL" if device["critical"] else "✅ OK"
                    print(f"{device['type']} {device['id']}: {device['temp']}°C {status}")
                
                # Проверка на критическое состояние
                critical_devices = [d for d in devices if d["critical"]]
                if critical_devices:
                    print(f"\n⚠️  WARNING: {len(critical_devices)} devices in critical state!")
                else:
                    print("\n✅ All devices are within safe temperature ranges")
            else:
                print(f"Error: HTTP {response.status_code}")
        except Exception as e:
            print(f"Monitoring error: {e}")
        
        time.sleep(10)

if __name__ == "__main__":
    monitor_hardware()
```

### Рекомендации по использованию

1. **Регулярный мониторинг**: Настройте регулярный мониторинг температуры устройств для предотвращения перегрева
2. **Алерты**: Создайте алерты для критических значений температуры (например, > 80°C для PCI, > 65°C для хранилища)
3. **Анализ трендов**: Храните исторические данные для анализа трендов и прогнозирования проблем
4. **Интеграция**: Интегрируйте мониторинг аппаратных устройств с существующими системами мониторинга
5. **Оптимизация**: Используйте информацию о температуре и состоянии здоровья для оптимизации размещения рабочих нагрузок

### Устранение проблем с аппаратным мониторингом

#### Нет данных о температуре

```bash
# Проверьте доступность sysfs
ls /sys/class/thermal/

# Проверьте права доступа
sudo chmod a+r /sys/class/thermal/thermal_zone*/temp
```

#### Нет данных о PCI устройствах

```bash
# Проверьте доступность PCI информации
lspci -v

# Проверьте права доступа
sudo chmod a+r /sys/bus/pci/devices/*/power
```

#### Нет данных о устройствах хранения

```bash
# Проверьте доступность SMART данных
sudo smartctl --info /dev/sda

# Установите необходимые пакеты
sudo apt install smartmontools
```

## 🚨 Устранение неполадок

### Частые проблемы

#### Демон не запускается

```bash
# Проверьте права доступа
chmod +x /usr/local/bin/smoothtaskd

# Проверьте конфигурацию
smoothtaskd --dry-run --config /etc/smoothtask/smoothtask.yml
```

#### Нет метрик

```bash
# Проверьте доступность /proc
ls /proc/stat

# Проверьте права доступа
sudo chmod a+r /proc/stat
```

#### Ошибки приоритетов

```bash
# Проверьте доступность cgroups v2
ls /sys/fs/cgroup/cgroup.controllers

# Проверьте права доступа
sudo chmod a+rw /sys/fs/cgroup/cpu.weight
```

## 🎓 Дополнительные ресурсы

- [Архитектура SmoothTask](ARCHITECTURE.md)
- [Документация API](API.md)
- [Руководство по метрикам](METRICS.md)
- [Руководство по политикам](POLICY.md)

## 🤝 Сообщество и поддержка

- **Issues**: Сообщайте о багах и предлагайте новые функции
- **Pull Requests**: Приветствуются вклад в проект
- **Обсуждения**: Обсуждайте идеи и архитектурные решения

## 📝 Лицензия

SmoothTask распространяется под лицензией MIT. См. файл LICENSE для подробностей.

---

*Последнее обновление: 2025-12-12*
