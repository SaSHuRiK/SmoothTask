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

## Конфигурация API

Если вы хотите использовать HTTP API для мониторинга, убедитесь, что в конфигурационном файле `/etc/smoothtask/smoothtask.yml` указан адрес для прослушивания:

```yaml
paths:
  api_listen_addr: "127.0.0.1:8080"  # Адрес для прослушивания API сервера
```

После изменения конфигурации перезапустите сервис:
```bash
sudo systemctl restart smoothtaskd.service
```

## Проверка валидности unit файла

Перед установкой можно проверить синтаксис unit файла:
```bash
systemd-analyze verify systemd/smoothtaskd.service
```

## Systemd Notify

Сервис использует `Type=notify` для интеграции с systemd:

- Демон отправляет `READY=1` после успешной инициализации всех компонентов
- Статус работы демона периодически обновляется через `STATUS=...` (виден в `systemctl status`)
- Это позволяет systemd точно знать, когда сервис готов к работе, и корректно управлять зависимостями

Если демон запущен не под systemd, уведомления безопасно игнорируются.

## Примечания

- Сервис запускается от имени `root`, так как требует доступа к `/proc`, cgroups и другим системным ресурсам
- Сервис использует `Type=notify` для интеграции с systemd через sd-notify
- При ошибках сервис автоматически перезапускается через 5 секунд (`Restart=on-failure`)
- Graceful shutdown происходит через SIGTERM с таймаутом 30 секунд
- Статус работы демона можно увидеть в `systemctl status smoothtaskd.service`
