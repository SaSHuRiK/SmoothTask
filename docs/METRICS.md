# Метрики SmoothTask

## Глобальные метрики

### CPU
- Процент использования CPU (user%, system%, idle%, iowait%)
- Источник: `/proc/stat`

### Memory
- Используемая и доступная память
- Используемый swap
- Источник: `/proc/meminfo`

### Load Average
- Средняя нагрузка системы
- Источник: `/proc/loadavg`

### PSI (Pressure Stall Information)
- `cpu_some_avg10`, `cpu_some_avg60` — давление на CPU
- `io_some_avg10` — давление на IO
- `mem_some_avg10`, `mem_full_avg10` — давление на память
- Источник: `/proc/pressure/{cpu,io,memory}`

### User Activity
- `user_active` (bool) — активен ли пользователь
- `time_since_last_input` — время с последнего ввода
- Источник: события evdev

## Per-process метрики

### Легкие метрики (для всех процессов)
- CPU usage (дельты за окно)
- Состояние процесса (R/S/D/Z/T)
- Приоритеты (nice, latency_nice, ionice)
- RSS
- Источник: `/proc/[pid]/stat`, `/proc/[pid]/sched`, `/proc/[pid]/io`

### Тяжелые метрики (только для кандидатов)
- Детальная информация о памяти (VmRSS, VmSwap)
- Контекстные переключения
- IO статистика (read_bytes, write_bytes)
- Переменные окружения
- Источники: `/proc/[pid]/status`, `/proc/[pid]/io`, `/proc/[pid]/environ`

## Метрики отзывчивости

### Scheduling Latency
- P95 и P99 латентности планировщика
- Измеряется через probe-thread (mini-cyclictest)

### Аудио XRUN
- Количество XRUN событий в аудио-стеке
- Источник: PipeWire/PulseAudio

### UI Latency (опционально)
- P95 латентности event loop GUI
- Frame jank ratio

### Интегральный score
- `bad_responsiveness` — флаг плохой отзывчивости
- `responsiveness_score` — нормированная комбинация метрик

## Частота сбора

- Глобальные метрики: 500–1000 мс
- Per-process (легкие): 1–2 Гц для всех процессов
- Per-process (тяжелые): 500–1000 мс, только для топ-N кандидатов
- Ввод пользователя: в реальном времени через evdev
- Окна/фокус: синхронизировано с циклом метрик
- Аудио: синхронизировано с циклом метрик

