# Архитектура SmoothTask

## Обзор

SmoothTask состоит из двух основных компонентов:

1. **Rust-демон** (`smoothtaskd`) — системный демон, работающий в реальном времени
2. **Python-тренер** (`smoothtask-trainer`) — инструменты для офлайн-обучения ML-моделей

## Компоненты Rust-демона

### Metrics Collector
- Сбор глобальных метрик из `/proc`, PSI
- Per-process метрики (CPU, IO, память)
- Информация о вводе пользователя (evdev)
- Состояние окон и фокуса (X11/Wayland)
- Метрики аудио (PipeWire/PulseAudio, XRUN)

### Process Grouper
- Построение AppGroup (групп процессов одного приложения)
- Определение корневого процесса и потомков

### Process Classifier
- Классификация процессов по типам (GUI/CLI/daemon/batch/...)
- Использование паттерн-базы приложений
- Определение тегов (browser/ide/game/audio/...)

### Policy Engine
- Жёсткие правила (guardrails)
- Семантические правила
- Интеграция с ML-ранкером (опционально)
- Определение целевого класса приоритета

### Actuator
- Применение приоритетов через `nice`, `ionice`
- Управление cgroups v2 (cpu.weight, cpu.max, IO-лимиты)
- Гистерезис для предотвращения частых изменений

### Snapshot Logger
- Сохранение снапшотов в SQLite
- Формирование датасета для обучения

## Компоненты Python-тренера

### Data Preparator
- Чтение снапшотов из SQLite
- Формирование датасета для CatBoostRanker

### CatBoost Trainer
- Обучение CatBoostRanker на собранных снапшотах
- Валидация и экспорт моделей (ONNX, JSON)

### Policy Tuner
- Оффлайн-оптимизация параметров политики
- Тюнинг порогов по метрикам отзывчивости

## Модель данных

См. раздел 4 в [tz.md](tz.md) для подробного описания структур данных:
- `Snapshot`
- `GlobalMetrics`
- `ProcessRecord`
- `AppGroupRecord`
- `ResponsivenessMetrics`

## Поток данных

1. Metrics Collector собирает метрики
2. Process Grouper группирует процессы в AppGroup
3. Process Classifier определяет типы и теги
4. Policy Engine применяет правила и ML-ранкер
5. Actuator применяет приоритеты
6. Snapshot Logger сохраняет снапшоты (опционально)

