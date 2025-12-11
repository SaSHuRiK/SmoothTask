# SmoothTask

**SmoothTask — чтобы система оставалась отзывчивой даже на 100% CPU.**

Системный демон для Linux, который автоматически управляет приоритетами процессов (nice, latency_nice, IO, cgroups), чтобы интерактивные приложения оставались максимально отзывчивыми, а фоновые задачи не «убивали» систему.

[![GitHub](https://img.shields.io/badge/GitHub-SmoothTask-blue)](https://github.com/SaSHuRiK/SmoothTask)

## Архитектура

- **Rust-демон** (`smoothtaskd`) — быстрый демон для сбора метрик, применения правил и ML-ранкера
- **Python-тренер** (`smoothtask-trainer`) — офлайн-обучение CatBoostRanker на основе собранных снапшотов

## Быстрый старт

### Сборка

```bash
cargo build --release
```

### Запуск

```bash
sudo ./target/release/smoothtaskd --config configs/smoothtask.example.yml
```

## Документация

См. [docs/tz.md](docs/tz.md) для полного технического задания.

## Статус проекта

🚧 **Проект в активной разработке** — MVP в стадии реализации.

Текущий этап: создана базовая структура проекта, модули подготовлены к реализации.

## Ссылки

- 📖 [Техническое задание](docs/tz.md)
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

