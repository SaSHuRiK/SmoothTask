# Руководство по установке SmoothTask

Это руководство поможет вам установить и настроить SmoothTask на различных дистрибутивах Linux.

## Требования системы

- **Linux дистрибутив** с ядром 5.4+ (рекомендуется 5.10+)
- **cgroups v2** (обязательно)
- **Rust 1.70+** для сборки
- **Python 3.9+** для тренера (опционально)
- **Wayland** или **X11** для обнаружения окон
- **PipeWire** или **PulseAudio** для аудио метрик

## Поддерживаемые дистрибутивы

- Ubuntu 22.04 LTS и новее
- Debian 11 и новее
- Fedora 36 и новее
- Arch Linux и производные (Manjaro, EndeavourOS)
- openSUSE Tumbleweed и Leap 15.4+

## Установка на Ubuntu/Debian

### 1. Установка зависимостей

```bash
# Обновите систему
sudo apt update && sudo apt upgrade -y

# Установите базовые зависимости
sudo apt install -y build-essential curl git pkg-config libssl-dev

# Установите Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Установите зависимости для сборки
sudo apt install -y libwayland-dev libpipewire-0.3-dev libpulse-dev libglib2.0-dev

# Для Python тренера (опционально)
sudo apt install -y python3 python3-pip python3-venv

# Для ONNX Runtime (опционально, для ML-ранжирования)
sudo apt install -y libonnxruntime-dev
```

### 2. Настройка cgroups v2

```bash
# Проверьте версию cgroups
stat -fc %T /sys/fs/cgroup/
# Должно вернуть: cgroup2fs

# Если у вас cgroups v1, переключитесь на v2
# Отредактируйте /etc/default/grub и добавьте:
# GRUB_CMDLINE_LINUX="systemd.unified_cgroup_hierarchy=1"
# Затем выполните:
sudo update-grub
sudo reboot
```

### 3. Сборка и установка

```bash
# Клонируйте репозиторий
git clone https://github.com/SaSHuRiK/SmoothTask.git
cd SmoothTask

# Соберите проект
cargo build --release

# Установите бинарник
sudo cp target/release/smoothtaskd /usr/local/bin/

# Создайте конфигурационные директории
sudo mkdir -p /etc/smoothtask/patterns
sudo cp -r configs/patterns/* /etc/smoothtask/patterns/

# Создайте конфигурационный файл
sudo mkdir -p /etc/smoothtask
sudo cp configs/smoothtask.example.yml /etc/smoothtask/smoothtask.yml

# Создайте директорию для данных
sudo mkdir -p /var/lib/smoothtask
sudo chown root:root /var/lib/smoothtask
```

### 4. Настройка systemd

```bash
# Установите systemd unit файл
sudo cp systemd/smoothtaskd.service /etc/systemd/system/

# Перезагрузите конфигурацию systemd
sudo systemctl daemon-reload

# Включите автозапуск
sudo systemctl enable smoothtaskd.service

# Запустите сервис
sudo systemctl start smoothtaskd.service

# Проверьте статус
sudo systemctl status smoothtaskd.service
```

## Установка на Fedora

### 1. Установка зависимостей

```bash
# Обновите систему
sudo dnf upgrade -y

# Установите базовые зависимости
sudo dnf install -y @development-tools curl git pkg-config openssl-devel

# Установите Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Установите зависимости для сборки
sudo dnf install -y wayland-devel pipewire-devel pulseaudio-libs-devel glib2-devel

# Для Python тренера (опционально)
sudo dnf install -y python3 python3-pip
```

### 2. Настройка cgroups v2

```bash
# Проверьте версию cgroups
stat -fc %T /sys/fs/cgroup/

# Fedora по умолчанию использует cgroups v2
```

### 3. Сборка и установка

Следуйте тем же шагам, что и для Ubuntu/Debian, начиная с шага 3.

## Установка на Arch Linux

### 1. Установка зависимостей

```bash
# Обновите систему
sudo pacman -Syu

# Установите базовые зависимости
sudo pacman -S --needed base-devel curl git pkgconf openssl

# Установите Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Установите зависимости для сборки
sudo pacman -S --needed wayland pipewire pulseaudio glib2

# Для Python тренера (опционально)
sudo pacman -S --needed python python-pip
```

### 2. Настройка cgroups v2

```bash
# Проверьте версию cgroups
stat -fc %T /sys/fs/cgroup/

# Arch Linux по умолчанию использует cgroups v2
```

### 3. Сборка и установка

Следуйте тем же шагам, что и для Ubuntu/Debian, начиная с шага 3.

## Установка на openSUSE

### 1. Установка зависимостей

```bash
# Обновите систему
sudo zypper refresh
sudo zypper update -y

# Установите базовые зависимости
sudo zypper install -y -t pattern devel_basis
sudo zypper install -y curl git pkg-config libopenssl-devel

# Установите Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Установите зависимости для сборки
sudo zypper install -y wayland-devel pipewire-devel libpulse-devel glib2-devel

# Для Python тренера (опционально)
sudo zypper install -y python3 python3-pip
```

### 2. Настройка cgroups v2

```bash
# Проверьте версию cgroups
stat -fc %T /sys/fs/cgroup/

# openSUSE Tumbleweed использует cgroups v2 по умолчанию
```

### 3. Сборка и установка

Следуйте тем же шагам, что и для Ubuntu/Debian, начиная с шага 3.

## Настройка ONNX моделей (опционально)

Для использования ML-ранжирования с ONNX моделями выполните следующие шаги:

### 1. Установка зависимостей для ONNX

```bash
# Для Python тренера
pip install onnxruntime catboost

# Для Rust ONNX Runtime
# Уже включено в зависимости smoothtask-core
```

### 2. Обучение и экспорт модели

```bash
cd smoothtask-trainer

# Обучение модели и экспорт в ONNX
python -m smoothtask_trainer.train_ranker \
    --db snapshots.db \
    --model-json models/ranker.json \
    --model-onnx models/ranker.onnx
```

### 3. Настройка демона для использования ONNX

Добавьте в конфигурацию `smoothtask.yml`:

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

### 4. Проверка интеграции

```bash
# Запуск демона с ONNX моделью
cargo run --bin smoothtaskd -- --config configs/smoothtask.yml

# Проверка логов
journalctl -u smoothtaskd -f
```

## Настройка конфигурации

### Базовая конфигурация

Редактируйте `/etc/smoothtask/smoothtask.yml`:

```yaml
# Интервал опроса системы в миллисекундах
polling_interval_ms: 500

# Максимальное количество кандидатов для обработки
max_candidates: 150

# Режим "сухого прогона" (для тестирования)
dry_run_default: false

# Режим работы Policy Engine
policy_mode: rules-only

paths:
  # Путь к базе данных снапшотов
  snapshot_db_path: "/var/lib/smoothtask/snapshots.sqlite"
  
  # Директория с паттернами приложений
  patterns_dir: "/etc/smoothtask/patterns"
  
  # Адрес для API сервера (опционально)
  api_listen_addr: "127.0.0.1:8080"

thresholds:
  # Пороги PSI для определения нагрузки
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  
  # Таймауты неактивности
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  
  # Пороги приоритетов
  noisy_neighbour_cpu_share: 0.7
  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
```

### Примеры конфигурации

В репозитории предоставлены готовые примеры конфигураций для различных сценариев использования:

- **Ноутбуки**: `configs/examples/smoothtask-laptop.yml`
- **Серверы**: `configs/examples/smoothtask-server.yml`
- **Рабочие станции**: `configs/examples/smoothtask-workstation.yml`

Эти примеры оптимизированы для различных типов систем и могут быть использованы как основа для вашей конфигурации.

#### Конфигурация для ноутбуков

Для ноутбуков рекомендуется более агрессивная настройка для экономии батареи:

```yaml
# Увеличенный интервал опроса для экономии батареи
polling_interval_ms: 1000

# Уменьшенное количество кандидатов для снижения нагрузки
max_candidates: 100

# Более чувствительные настройки для ноутбуков
thresholds:
  psi_cpu_some_high: 0.5
  psi_io_some_high: 0.3
  user_idle_timeout_sec: 60
  
  # Более агрессивное распределение для экономии ресурсов
  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1

# Конфигурация системы уведомлений для ноутбуков
notifications:
  enabled: true
  backend: libnotify
  min_level: warning
```

**Особенности:**
- Увеличенный интервал опроса (1000мс) для экономии батареи
- Более чувствительные пороги PSI для раннего обнаружения нагрузки
- Уменьшенный таймаут неактивности (60с) для быстрого перехода в режим экономии
- Включенные уведомления для мониторинга работы демона

#### Конфигурация для серверов

Для серверов без графического интерфейса:

```yaml
# Увеличенный интервал опроса для серверов
polling_interval_ms: 2000

# Увеличенное количество кандидатов для серверных нагрузок
max_candidates: 200

# Менее чувствительные настройки для серверов
thresholds:
  psi_cpu_some_high: 0.7
  psi_io_some_high: 0.5
  user_idle_timeout_sec: 300
  
  # Менее агрессивное распределение для серверных нагрузок
  crit_interactive_percentile: 0.95
  interactive_percentile: 0.7
  normal_percentile: 0.4
  background_percentile: 0.2

# Конфигурация системы уведомлений для серверов
notifications:
  enabled: false
  backend: stub
  min_level: warning
```

**Особенности:**
- Увеличенный интервал опроса (2000мс) для снижения нагрузки
- Менее чувствительные пороги PSI для серверных нагрузок
- Увеличенный таймаут неактивности (300с) для серверных задач
- Отключенные уведомления (нет GUI)

#### Конфигурация для рабочих станций

Для мощных рабочих станций с высокой нагрузкой:

```yaml
# Уменьшенный интервал опроса для быстрой реакции
polling_interval_ms: 250

# Увеличенное количество кандидатов для рабочих нагрузок
max_candidates: 200

# Более чувствительные настройки для рабочих станций
thresholds:
  psi_cpu_some_high: 0.4
  psi_io_some_high: 0.2
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 15
  
  # Более агрессивное распределение для рабочих нагрузок
  crit_interactive_percentile: 0.85
  interactive_percentile: 0.5
  normal_percentile: 0.25
  background_percentile: 0.05

# Конфигурация системы уведомлений для рабочих станций
notifications:
  enabled: true
  backend: libnotify
  min_level: warning
```

**Особенности:**
- Уменьшенный интервал опроса (250мс) для быстрой реакции
- Более чувствительные пороги PSI для раннего обнаружения нагрузки
- Увеличенный период отсрочки для интерактивных сборок (15с)
- Более агрессивное распределение приоритетов для рабочих нагрузок

### Лучшие практики конфигурации

1. **Настройка интервалов опроса:**
   - Ноутбуки: 800-1200мс для экономии батареи
   - Рабочие станции: 200-500мс для быстрой реакции
   - Серверы: 1500-3000мс для снижения нагрузки

2. **Настройка порогов PSI:**
   - Более низкие значения (0.3-0.5) для чувствительных систем
   - Более высокие значения (0.6-0.8) для серверов и стабильных нагрузок

3. **Настройка перцентилей приоритетов:**
   - Более агрессивное распределение для интерактивных систем
   - Менее агрессивное распределение для серверов

4. **Уведомления:**
   - Включайте уведомления для рабочих станций и ноутбуков
   - Отключайте уведомления для серверов без GUI

### Использование примеров конфигурации

Чтобы использовать готовые примеры:

```bash
# Для ноутбуков
sudo cp configs/examples/smoothtask-laptop.yml /etc/smoothtask/smoothtask.yml

# Для серверов
sudo cp configs/examples/smoothtask-server.yml /etc/smoothtask/smoothtask.yml

# Для рабочих станций
sudo cp configs/examples/smoothtask-workstation.yml /etc/smoothtask/smoothtask.yml
```

После копирования не забудьте перезапустить демон:

```bash
sudo systemctl restart smoothtaskd.service
```

## Настройка паттернов приложений

### Добавление новых паттернов

1. Создайте новый файл в `/etc/smoothtask/patterns/`:
   ```bash
   sudo nano /etc/smoothtask/patterns/myapp.yml
   ```

2. Добавьте паттерн:
   ```yaml
   category: myapp
   apps:
     - name: "myapp"
       label: "My Application"
       exe_patterns: ["myapp", "myapp-bin"]
       desktop_patterns: ["myapp.desktop"]
       tags: ["gui_interactive", "custom"]
   ```

3. Перезапустите демон:
   ```bash
   sudo systemctl restart smoothtaskd.service
   ```

### Примеры паттернов

См. существующие паттерны в `/etc/smoothtask/patterns/`:

- `browsers.yml` - браузеры (Firefox, Chrome, Brave и др.)
- `ide.yml` - среды разработки (VS Code, IntelliJ, PyCharm и др.)
- `audio.yml` - аудио приложения (Audacity, Ardour, LMMS и др.)
- `video.yml` - видео приложения (Kdenlive, OBS, Blender и др.)
- `games.yml` - игры (Steam, Wine, Lutris и др.)
- `build_tools.yml` - инструменты сборки (make, cmake, ninja и др.)

## Настройка API

### Включение API

Редактируйте конфигурационный файл:

```yaml
paths:
  api_listen_addr: "127.0.0.1:8080"
```

### Безопасность API

Для защиты API:

1. **Используйте брандмауэр:**
   ```bash
   sudo ufw allow from 127.0.0.1 to any port 8080
   sudo ufw enable
   ```

2. **Используйте nginx как reverse proxy:**
   ```nginx
   server {
       listen 80;
       server_name smoothtask.example.com;
       
       location / {
           proxy_pass http://127.0.0.1:8080;
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
           
           # Авторизация
           auth_basic "Restricted Access";
           auth_basic_user_file /etc/nginx/.htpasswd;
       }
   }
   ```

3. **Создайте .htpasswd файл:**
   ```bash
   sudo apt install apache2-utils
   sudo htpasswd -c /etc/nginx/.htpasswd username
   ```

## Мониторинг и логирование

### Просмотр логов

```bash
# Логи systemd
sudo journalctl -u smoothtaskd.service -f

# Логи демона
sudo tail -f /var/log/syslog | grep smoothtaskd

# Логи API
curl http://127.0.0.1:8080/api/stats
```

### Мониторинг производительности

```bash
# Использование CPU
 top -p $(pidof smoothtaskd)

# Использование памяти
 ps -p $(pidof smoothtaskd) -o %mem,cmd

# Статистика демона
 curl http://127.0.0.1:8080/api/stats
```

## Обновление

### Обновление до новой версии

```bash
# Остановите сервис
sudo systemctl stop smoothtaskd.service

# Обновите код
cd /path/to/SmoothTask
git pull origin main

# Пересоберите
cargo build --release

# Обновите бинарник
sudo cp target/release/smoothtaskd /usr/local/bin/

# Перезапустите сервис
sudo systemctl start smoothtaskd.service
```

## Удаление

### Полное удаление

```bash
# Остановите и отключите сервис
sudo systemctl stop smoothtaskd.service
sudo systemctl disable smoothtaskd.service

# Удалите unit файл
sudo rm /etc/systemd/system/smoothtaskd.service

# Удалите бинарник
sudo rm /usr/local/bin/smoothtaskd

# Удалите конфигурационные файлы
sudo rm -rf /etc/smoothtask

# Удалите данные
sudo rm -rf /var/lib/smoothtask

# Перезагрузите systemd
sudo systemctl daemon-reload
```

## Часто задаваемые вопросы

### Как проверить, что демон работает?

```bash
sudo systemctl status smoothtaskd.service
curl http://127.0.0.1:8080/health
```

### Как изменить приоритет конкретного процесса?

Редактируйте паттерны в `/etc/smoothtask/patterns/` или используйте API для временного изменения приоритетов.

### Как отключить ML-ранкер?

Установите `policy_mode: rules-only` в конфигурационном файле.

### Как увеличить производительность?

Увеличьте `polling_interval_ms` и уменьшите `max_candidates` в конфигурации.

### Как добавить поддержку нового приложения?

Создайте новый паттерн в `/etc/smoothtask/patterns/` и перезапустите демон.

## Устранение неполадок при установке

### Ошибка сборки: glib-2.0 не найден

**Проблема:** Ошибка сборки с сообщением "The system library `glib-2.0` required by crate `glib-2-0-sys` was not found."

**Решения:**

1. **Установите glib-2.0 для вашего дистрибутива:**
   ```bash
   # Ubuntu/Debian
   sudo apt install -y libglib2.0-dev
   
   # Fedora
   sudo dnf install -y glib2-devel
   
   # Arch Linux
   sudo pacman -S --needed glib2
   
   # openSUSE
   sudo zypper install -y glib2-devel
   ```

2. **Убедитесь, что pkg-config может найти glib-2.0:**
   ```bash
   pkg-config --libs --cflags glib-2.0 'glib-2.0 >= 2.46'
   ```

3. **Если pkg-config не может найти библиотеку, установите PKG_CONFIG_PATH:**
   ```bash
   export PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig:/usr/share/pkgconfig
   ```

4. **Проверьте, что файл glib-2.0.pc существует:**
   ```bash
   find /usr -name "glib-2.0.pc"
   ```

5. **Пересоберите проект:**
   ```bash
   cargo clean
   cargo build --release
   ```

### Проблемы с pkg-config

**Проблема:** pkg-config не установлен или не может найти библиотеки.

**Решения:**

1. **Установите pkg-config:**
   ```bash
   # Ubuntu/Debian
   sudo apt install -y pkg-config
   
   # Fedora
   sudo dnf install -y pkgconf-pkg-config
   
   # Arch Linux
   sudo pacman -S --needed pkgconf
   
   # openSUSE
   sudo zypper install -y pkg-config
   ```

2. **Проверьте, что pkg-config работает:**
   ```bash
   pkg-config --version
   ```

### Отсутствующие зависимости для Wayland

**Проблема:** Ошибки сборки, связанные с Wayland.

**Решения:**

1. **Убедитесь, что установлены все зависимости Wayland:**
   ```bash
   # Ubuntu/Debian
   sudo apt install -y libwayland-dev wayland-protocols
   
   # Fedora
   sudo dnf install -y wayland-devel
   
   # Arch Linux
   sudo pacman -S --needed wayland
   
   # openSUSE
   sudo zypper install -y wayland-devel
   ```

2. **Проверьте, что Wayland доступен:**
   ```bash
   echo $WAYLAND_DISPLAY
   ```

## Поддержка

Если у вас возникли проблемы:

1. Проверьте раздел [Устранение неполадок](README.md#Устранение-неполадок)
2. Проверьте логи: `sudo journalctl -u smoothtaskd.service`
3. Создайте issue на GitHub с подробным описанием проблемы
