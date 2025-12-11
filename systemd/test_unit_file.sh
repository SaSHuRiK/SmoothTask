#!/bin/bash
# Простой скрипт для проверки валидности systemd unit файла

set -e

UNIT_FILE="systemd/smoothtaskd.service"

echo "Проверка systemd unit файла: $UNIT_FILE"

# Проверка существования файла
if [ ! -f "$UNIT_FILE" ]; then
    echo "ОШИБКА: Файл $UNIT_FILE не найден"
    exit 1
fi

# Проверка обязательных секций
echo "Проверка обязательных секций..."

if ! grep -q "^\[Unit\]" "$UNIT_FILE"; then
    echo "ОШИБКА: Отсутствует секция [Unit]"
    exit 1
fi

if ! grep -q "^\[Service\]" "$UNIT_FILE"; then
    echo "ОШИБКА: Отсутствует секция [Service]"
    exit 1
fi

if ! grep -q "^\[Install\]" "$UNIT_FILE"; then
    echo "ОШИБКА: Отсутствует секция [Install]"
    exit 1
fi

# Проверка обязательных полей
echo "Проверка обязательных полей..."

if ! grep -q "^Description=" "$UNIT_FILE"; then
    echo "ОШИБКА: Отсутствует поле Description"
    exit 1
fi

if ! grep -q "^ExecStart=" "$UNIT_FILE"; then
    echo "ОШИБКА: Отсутствует поле ExecStart"
    exit 1
fi

if ! grep -q "^WantedBy=" "$UNIT_FILE"; then
    echo "ОШИБКА: Отсутствует поле WantedBy"
    exit 1
fi

# Проверка синтаксиса через systemd-analyze (если доступен)
if command -v systemd-analyze &> /dev/null; then
    echo "Проверка синтаксиса через systemd-analyze..."
    if systemd-analyze verify "$UNIT_FILE" 2>&1 | grep -q "smoothtaskd.service"; then
        # Если есть ошибки, связанные с нашим файлом (не с отсутствием бинарника)
        if systemd-analyze verify "$UNIT_FILE" 2>&1 | grep -v "is not executable" | grep -q "smoothtaskd.service"; then
            echo "ПРЕДУПРЕЖДЕНИЕ: systemd-analyze обнаружил проблемы (кроме отсутствия бинарника)"
            systemd-analyze verify "$UNIT_FILE" 2>&1 | grep "smoothtaskd.service" || true
        else
            echo "✓ Синтаксис unit файла корректен (предупреждение о бинарнике игнорируется)"
        fi
    else
        echo "✓ Синтаксис unit файла корректен"
    fi
else
    echo "ПРЕДУПРЕЖДЕНИЕ: systemd-analyze не найден, пропуск проверки синтаксиса"
fi

echo "✓ Все проверки пройдены успешно"
