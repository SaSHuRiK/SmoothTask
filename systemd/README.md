# Systemd unit файл для SmoothTask

Этот каталог содержит systemd unit файл для автоматического запуска `smoothtaskd` при загрузке системы.

## Установка

1. Скопируйте unit файл в директорию systemd:
   ```bash
   sudo cp systemd/smoothtaskd.service /etc/systemd/system/
   ```

2. Перезагрузите конфигурацию systemd:
   ```bash
   sudo systemctl daemon-reload
   ```

3. Включите автозапуск сервиса:
   ```bash
   sudo systemctl enable smoothtaskd.service
   ```

4. Запустите сервис:
   ```bash
   sudo systemctl start smoothtaskd.service
   ```

## Управление сервисом

- Проверить статус:
  ```bash
  sudo systemctl status smoothtaskd.service
  ```

- Остановить:
  ```bash
  sudo systemctl stop smoothtaskd.service
  ```

- Перезапустить:
  ```bash
  sudo systemctl restart smoothtaskd.service
  ```

- Просмотр логов:
  ```bash
  sudo journalctl -u smoothtaskd.service -f
  ```

- Отключить автозапуск:
  ```bash
  sudo systemctl disable smoothtaskd.service
  ```

## Настройка

Перед установкой убедитесь, что:

1. Бинарник `smoothtaskd` установлен в `/usr/local/bin/smoothtaskd` (или измените путь в unit файле)
2. Конфигурационный файл находится в `/etc/smoothtask/smoothtask.yml`
3. Директория `/var/lib/smoothtask/` существует и доступна для записи:
   ```bash
   sudo mkdir -p /var/lib/smoothtask
   sudo chown root:root /var/lib/smoothtask
   ```

## Проверка валидности unit файла

Перед установкой можно проверить синтаксис unit файла:
```bash
systemd-analyze verify systemd/smoothtaskd.service
```

## Примечания

- Сервис запускается от имени `root`, так как требует доступа к `/proc`, cgroups и другим системным ресурсам
- Сервис использует `Type=simple`, так как демон сам управляет своим жизненным циклом
- При ошибках сервис автоматически перезапускается через 5 секунд (`Restart=on-failure`)
- Graceful shutdown происходит через SIGTERM с таймаутом 30 секунд
