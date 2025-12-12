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
sudo apt install -y libwayland-dev libpipewire-0.3-dev libpulse-dev

# Для Python тренера (опционально)
sudo apt install -y python3 python3-pip python3-venv
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
sudo dnf install -y wayland-devel pipewire-devel pulseaudio-libs-devel

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
sudo pacman -S --needed wayland pipewire pulseaudio

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
sudo zypper install -y wayland-devel pipewire-devel libpulse-devel

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

### Конфигурация для ноутбуков

Для ноутбуков рекомендуется более агрессивная настройка для экономии батареи:

```yaml
polling_interval_ms: 1000
max_candidates: 100

thresholds:
  psi_cpu_some_high: 0.5
  psi_io_some_high: 0.3
  user_idle_timeout_sec: 60
```

### Конфигурация для серверов

Для серверов без графического интерфейса:

```yaml
polling_interval_ms: 2000
max_candidates: 200

thresholds:
  psi_cpu_some_high: 0.7
  psi_io_some_high: 0.5
  user_idle_timeout_sec: 300
```

### Конфигурация для рабочих станций

Для мощных рабочих станций с высокой нагрузкой:

```yaml
polling_interval_ms: 250
max_candidates: 200

thresholds:
  psi_cpu_some_high: 0.4
  psi_io_some_high: 0.2
  interactive_build_grace_sec: 15
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

## Поддержка

Если у вас возникли проблемы:

1. Проверьте раздел [Устранение неполадок](README.md#Устранение-неполадок)
2. Проверьте логи: `sudo journalctl -u smoothtaskd.service`
3. Создайте issue на GitHub с подробным описанием проблемы
