Ниже — собранное и доработанное ТЗ целиком, под текущую договорённость:

> **Python = CatBoost (офлайн-обучение)**
> **Rust = быстрый демон (сбор метрик, правила, ранкер, применение).**

---

# 1. Цель проекта

Разработать системный демон для Linux, который **автоматически управляет приоритетами процессов** (CPU, IO, cgroups), чтобы:

* интерактивные GUI/CLI-приложения (IDE, браузер, терминал, игры, аудио) оставались максимально отзывчивыми;
* фоновые и batch-задачи (рендеры, сборки, торренты, апдейтеры, индексаторы) не «убивали» систему;
* поведение адаптировалось под реальные метрики латентности и стиль работы пользователя.

---

# 2. Общие требования

## 2.1. Функциональные

1. Сбор системных и per-process метрик с низким overhead.
2. Автоматическая классификация процессов:

   * `gui_interactive`, `cli_interactive`, `system_service`, `user_daemon`, `batch_heavy`, `maintenance`, `audio_client`, `browser`, `ide`, `game` и др.
3. Учёт **AppGroup** (группа процессов одного приложения: root GUI + дети).
4. Учёт состояния приложения:

   * `FOCUSED`, `VISIBLE_BACKGROUND`, `MINIMIZED/HIDDEN`, `HEADLESS/BACKGROUND`.
5. Выявление паттернов:

   * heavy вкладка в браузере;
   * компиляция внутри IDE;
   * автообновления во время активной работы;
   * «noisy neighbour» (одна группа ломает отзывчивость).
6. На основе правил + ML-ранкера:

   * присвоение **класса приоритета** (`CRIT_INTERACTIVE`, `INTERACTIVE`, `NORMAL`, `BACKGROUND`, `IDLE`);
   * установка для процессов/AppGroup:

     * `nice`,
     * `ionice`,
     * `cpu.weight` / `cpu.max` / IO-лимитов (cgroups v2).
7. Сбор **объективных метрик отзывчивости**:

   * PSI (CPU/IO/mem pressure);
   * scheduling latency (mini-cyclictest);
   * аудио XRUN;
   * (опция) GUI-loop latency / jank.
8. Логирование снапшотов для офлайн-аналитики и обучения CatBoost.
9. Режимы работы:

   * `rules-only` (только правила, без ML);
   * `hybrid` (правила + CatBoostRanker);
   * `dry-run` (ML считает, но не применяет).

## 2.2. Нефункциональные

* Overhead демона: **≤ 1–2% CPU** в обычных сценариях.
* Никаких RT-приоритетов (`SCHED_FIFO/RR`) для юзерских процессов.
* Надёжность:

  * при падении демона система работает как без него;
  * все действия обратимы (при отключении — возврат к дефолтным классам/лимитам).
* Конфигурируемость:

  * YAML/TOML конфиги;
  * паттерны приложений;
  * per-user overrides.

---

# 3. Архитектура

## 3.1. Компоненты

### Rust-демон

1. **Metrics Collector**

   * Глобальные метрики: `/proc`, PSI.
   * Per-process: CPU/IO/mem, дерево процессов, cgroups.
   * Ввод/активность пользователя (evdev).
   * Окна и фокус (X11/Wayland).
   * Аудио (PipeWire/PulseAudio).

2. **Process Grouper**

   * Строит **AppGroup** (по `ppid`, `cgroup_path`, systemd unit).
   * Помечает процессы `app_group_id`.

3. **Process Classifier (rules + ML, опционально)**

   * Определяет тип процесса и AppGroup (GUI/CLI/daemon/batch/…).
   * Использует паттерн-базу и контекст.

4. **Policy Engine**

   * Жёсткие правила (guardrails + семантика).
   * Параметризованные правила (пороги, тайминги).
   * Вызов ML-ранкера для упорядочивания кандидатов.
   * Выдача target-класса приоритета для процессов/AppGroup.

5. **Actuator**

   * Применение:

     * `nice` (`setpriority`);
     * `ionice` (`ioprio_set`);
     * cgroups v2 (`cpu.weight`, `cpu.max`, IO-лимиты, перенос pid между cgroups).
   * Гистерезис: не дёргать приоритеты при мелких колебаниях.

6. **Snapshot Logger**

   * Пишет снапшоты в SQLite/файлы (для обучения и отладки).

7. **Control API (опционально)**

   * HTTP/gRPC API для просмотра состояния, ручных override и отладки.

### Python-стек (офлайн)

1. **Data Preparator**

   * Читает снапшоты (SQLite/Parquet).
   * Формирует датасеты `CatBoostRanker`:

     * `query_id` = `snapshot_id`;
     * объекты = процессы/AppGroup внутри снапшота;
     * фичи = глобальные + per-process + типы/теги;
     * таргеты = teacher-score / класс / `responsiveness_score`.

2. **CatBoost Trainer**

   * Обучение:

     * `CatBoostRanker` (основной);
     * (опция) `CatBoostClassifier` для типов процессов.
   * Валидация по NDCG + off-policy-метрикам.
   * Экспорт моделей:

     * ONNX (`model.onnx`);
     * JSON (`model.json`) для резервного инференса.

3. **Policy Tuner**

   * Оффлайн-тюнинг параметров правил (PSI-пороги, границы percentiles, idle-timeouts и т.п.) по логам и метрикам латентности.

---

# 4. Модель данных

## 4.1. Snapshot

```text
Snapshot {
  snapshot_id: u64 (timestamp, ms),
  global: GlobalMetrics,
  processes: Vec<ProcessRecord>,
  app_groups: Vec<AppGroupRecord>,
  responsiveness: ResponsivenessMetrics
}
```

### GlobalMetrics

* CPU: user%, system%, idle%, iowait%.
* Memory: used, available, swap_used.
* Loadavg.
* PSI:

  * `cpu_some_avg10`, `cpu_some_avg60`;
  * `io_some_avg10`;
  * `mem_some_avg10`, `mem_full_avg10`.
* User-activity:

  * `user_active` (bool),
  * `time_since_last_input`.

### ResponsivenessMetrics

* `sched_latency_p95`, `sched_latency_p99` (probe-thread).
* `audio_xruns_delta`.
* (опция) `ui_loop_p95`, `frame_jank_ratio`.
* Флаг `bad_responsiveness` (по порогам).

## 4.2. ProcessRecord

Для каждого процесса:

* Идентификация:

  * `pid`, `ppid`, `uid`, `gid`;
  * `exe`, `cmdline`;
  * `cgroup_path`, `systemd_unit`;
  * `app_group_id`.
* Состояние:

  * `state` (R/S/D/Z/T…);
  * `start_time`, `uptime`;
  * `tty_nr`, `has_tty`.
* Ресурсы (дельты за окно):

  * `cpu_share_1s`, `cpu_share_10s`;
  * `io_read_bytes`, `io_write_bytes`;
  * `rss_mb`, `swap_mb`;
  * `voluntary_ctx`, `involuntary_ctx`.
* Интерактивность/контекст:

  * `has_gui_window`, `is_focused_window`, `window_state`;
  * `env_has_display`, `env_has_wayland`, `env_term`, `env_ssh`;
  * `is_audio_client`, `has_active_stream`.
* Классификация:

  * `process_type` (enum);
  * `tags` (множество: browser/game/ide/updater/indexer/…).
* Приоритет:

  * текущие `nice`, `ionice_class`, `ionice_prio`;
  * текущие cgroup-параметры.
* Для обучения:

  * `teacher_priority_class`;
  * `teacher_score` (если есть);
  * `responsiveness_score`/`bad_responsiveness`.

## 4.3. AppGroupRecord

* `app_group_id`;
* `root_pid`;
* `process_ids: Vec<pid_t>`;
* `app_name`/`guess` (по exe/unit);
* агрегированные метрики:

  * суммарный CPU/IO/RSS;
  * флаг `has_gui_window`, `is_focused_group`;
  * флаги типов (browser/ide/game/…);
* итоговый класс приоритета, применённый к группе.

---

# 5. Сбор метрик

## 5.1. Глобальные

Частота: 500–1000 мс.

* `/proc/stat` → CPU usage.
* `/proc/meminfo` → память/Swap.
* `/proc/loadavg` → контекст.
* PSI:

  * `cat /proc/pressure/{cpu,io,memory}` → `some/full avg10/60`.

## 5.2. Per-process (легкие)

Частота: 1–2 Гц для всех процессов.

* `/proc/[pid]/stat`:

  * `comm`, `state`, `ppid`, `tty_nr`;
  * `utime`, `stime` → CPU дельты;
  * `priority`, `nice`;
  * `num_threads`;
  * `starttime`, `rss`.
* `/proc/[pid]/cgroup`:

  * slice/unit/container.

Используем для:

* предварительного отбора кандидатов;
* грубой классификации типов.

## 5.3. Per-process (тяжёлые — только для кандидатов)

Частота: 500–1000 мс, но **только для топ-N** (например, N=100–200).

* `/proc/[pid]/status`:

  * `Uid/Gid`, `VmRSS`, `VmSwap`;
  * `voluntary_ctxt_switches`, `nonvoluntary_ctxt_switches`.
* `/proc/[pid]/io`:

  * `read_bytes`, `write_bytes`, `rchar`, `wchar` (дельты).
* `/proc/[pid]/environ`:

  * `DISPLAY`, `WAYLAND_DISPLAY`, `TERM`, `SSH_*` и т.п. (по возможности кэшировать).

## 5.4. Ввод пользователя

* `evdev`:

  * события `/dev/input/event*`;
  * обновляем `last_input_time`.
* `user_active = now - last_input_time < user_idle_timeout`.

## 5.5. Окна / фокус

### X11

* `x11rb`:

  * список окон, `_NET_WM_PID`, `_NET_ACTIVE_WINDOW`.
* Строим:

  * `pid → {has_window, is_focused, window_state}`.

### Wayland

* `smithay-client-toolkit` + `wayland-client` + специфичные протоколы композитора (минимум: focused app, список окон).
* Если API нет — fallback по паттернам (app-units, env).

## 5.6. Аудио / XRUN

### PipeWire

* `pipewire`:

  * список клиентов и потоков;
  * счётчики XRUN (или статистика ошибок);
  * текущие настройки latency/buffer.

### PulseAudio (если используется)

* `libpulse-binding` / `pulsectl-rs`:

  * XRUN-счётчики;
  * клиенты, потоки.

---

# 6. Автоопределение типов процессов / AppGroup

## 6.1. AppGroup

* Корень — GUI-процесс (или systemd unit).
* Все потомки (по `ppid`, `cgroup_path`, unit) → один `app_group_id`.
* Для контейнеров — отдельная logика (один контейнер = отдельный AppGroup).

## 6.2. CLI-интерактивные

Условия:

* `tty_nr != 0` или `/proc/[pid]/fd/0` → `/dev/tty*`/`/dev/pts/*`;
* родитель — shell/terminal (`bash`, `zsh`, `fish`, `tmux`, `gnome-terminal-*`, `kitty`, `wezterm`, etc.);
* env: `TERM` не `dumb`, `SSH_CONNECTION`/`SSH_TTY` возможно.

→ `process_type = cli_interactive`.

Особый паттерн:

* если CLI-процесс запускает heavy build (`make`, `cargo`, `npm run build` и т.п.):

  * первые `interactive_build_grace_sec` → оставляем `INTERACTIVE/NORMAL`;
  * после истечения и при `bad_responsiveness` → переводим в `batch_heavy`.

## 6.3. GUI-интерактивные

Условия:

* есть окно (`has_gui_window=true`);
* env: `DISPLAY`/`WAYLAND_DISPLAY` есть;
* окно в фокусе:

  * `window_state = FOCUSED` → `CRIT_INTERACTIVE`;
* `VISIBLE_BACKGROUND` → `INTERACTIVE`;
* `MINIMIZED/HIDDEN` → не выше `NORMAL`, если нет аудио/особых тегов.

Типы:

* `browser`, `ide`, `game`, `player` и т.п. по exe/паттернам.

## 6.4. Демоны / сервисы

Условия:

* нет TTY;
* `cgroup_path` в `system.slice` или `system-*.slice`;
* root-родитель `systemd`/init;
* тип unit: `*.service`, `*.socket`, `*.timer`.

→ `process_type = system_service`
Такие процессы **по умолчанию не трогаем**, кроме мягкой подстройки (ограничения batch-сервисов при необходимости).

## 6.5. Batch / heavy background

Условия:

* нет TTY и GUI;
* не `system_service`;
* `cmdline`/`exe` в паттернах:

  * `ffmpeg`, `HandBrake`, `rsync`, backup-cli, архиваторы;
  * `python`/`node`/`java` с известными batch-скриптами;
  * торренты (`qbittorrent`, `transmission`, …);
* длительно высокий CPU/IO.

→ `process_type = batch_heavy`.

## 6.6. Maintenance / автообновления / индексаторы

Условия:

* паттерны: `*update*`, `*updater*`, `apt`, `dnf`, `snapd`, `flatpak`, `packagekitd`, `tracker`, `baloo` и т.п.;
* нет TTY;
* активный IO.

→ `process_type = maintenance`.
При `user_active=true` → максимум `BACKGROUND/IDLE`.

## 6.7. Специальные теги

По паттернам и контексту:

* `audio_client` — активный поток в PipeWire/PA;
* `browser`, `game`, `ide`, `player`, `torrent`, `build_tool`, `indexer`, `updater` и т.п.

Эти теги:

* участвуют в правилах;
* становятся фичами для ранкера.

---

# 7. Политика, ранкер и приоритеты

## 7.1. Жёсткие правила (guardrails)

Не подлежат авто-тюнингу:

* Не менять:

  * `systemd`, `journald`, `udevd`, сетевые/дисковые критичные демоны.
* Не выдавать:

  * `SCHED_FIFO/RR` юзерским процессам;
  * `nice < -10`.
* Не опускать `audio_client` ниже `INTERACTIVE`, если есть XRUN на низком буфере.
* Не превышать суммарный вес batch-групп (`max_batch_cpu_weight`) относительно total CPU.

## 7.2. Семантические правила

Примеры:

* Фокусный GUI-AppGroup всегда ≥ `INTERACTIVE` и ≥ свернутых приложений.
* Активный терминал с недавним вводом ≥ свернутым batch-процессам.
* Updater/indexer при активном пользователе ≤ `BACKGROUND/IDLE`.
* Если вкладка/renderer в фоне жрёт CPU, а у юзера падает отзывчивость → душим эту группу, а не всё приложение.

Правила задаются как отдельный модуль:

* в виде `if/else` и параметров (порогов);
* часть параметров доступна для оффлайн-тюнинга.

## 7.3. Ранкер (CatBoostRanker)

### Входные данные для ранкера

На каждый snapshot:

* список **кандидатов**:

  * все интерактивные (`gui/cli/audio`);
  * AppGroup с высоким CPU/IO;
  * (опция) другие интересные процессы.
* Для каждого кандидата:

  * глобальные фичи (PSI, load, mem, responsiveness);
  * пер-процессные фичи (CPU/IO/RSS, контекст);
  * тип и теги (gui/cli/batch/…);
  * состояние окна (focus/background/minimized);
  * принадлежность к AppGroup и aggregated features AppGroup.

`query_id = snapshot_id`.

### Выход

* `score` для каждого процесса/AppGroup.
* По score считаем:

  * `rank` и `percentile p`.

### Маппинг score → класс

Параметризуемые пороги:

* `p >= p_crit` → `CRIT_INTERACTIVE`;
* `p_crit > p >= p_inter` → `INTERACTIVE`;
* `p_inter > p >= p_norm` → `NORMAL`;
* `p_norm > p >= p_back` → `BACKGROUND`;
* `< p_back` → `IDLE`.

Правила дополняют:

* аудио, системные демоны и т.п. могут «поднимать/опускать» класс вне ранкера в рамках guardrails.

## 7.4. Классы → nice / ionice / cgroup

Пример базовой таблицы:

| Class            | nice | ionice class/level | cpu.weight | Примечания                         |
| ---------------- | ---- | ------------------ | ---------: | ---------------------------------- |
| CRIT_INTERACTIVE | -8   | 2 / 0–1            |        200 | фокус + аудио/игра                 |
| INTERACTIVE      | -4   | 2 / 2–3            |        150 | обычный UI/CLI                     |
| NORMAL           | 0    | 2 / 4              |        100 | дефолт                             |
| BACKGROUND       | +5   | 2 / 6              |         50 | batch / maintenance                |
| IDLE             | +10  | 3 (idle)           |         25 | всё, что можно делать «на остатке» |

**Гистерезис:**

* класс меняем только если:

  * условие держится N снапшотов подряд;
  * разница классов ≥ 1 (не мельтешим между соседями);
* можно ввести «min_time_in_class».

---

# 8. Метрики отзывчивости и таргеты

## 8.1. OS / scheduling latency

* mini-`cyclictest` поток(и) (SCHED_OTHER, нормальный nice):

  * sleep на 5–10 мс;
  * меряем `dt = wakeup_delay`;
  * собираем `p95`, `p99` за окно.

## 8.2. PSI

Используем `cpu_some`, `io_some`, `mem_some/full` как индикаторы «давки».

## 8.3. Аудио

* `audio_xruns_delta` за окно;
* `audio_latency_ms` (из PipeWire/PA).

## 8.4. UI

(Опционально, если есть возможность):

* probe-GUI, считающий latency event loop;
* jank/frametime из композитора.

## 8.5. Интегральный score

Определяем:

```text
bad_responsiveness =
    psi_cpu_some_avg10 > T_cpu
 || psi_io_some_avg10  > T_io
 || sched_p99          > T_sched
 || audio_xruns_delta  > 0
 || ui_loop_p95        > T_ui (если есть)
```

`responsiveness_score` – нормированная комбинация этих метрик.

Использование:

* как таргет/лейбл при обучении:

  * либо бинарный (`bad`/`ok`);
  * либо непрерывный.
* как критерий для оффлайн-тюнинга параметров policy.

---

# 9. Логирование и обучение

## 9.1. Формат логов

* Храним в SQLite/Parquet:

  * `snapshot_id` + `GlobalMetrics` + `ResponsivenessMetrics`;
  * `ProcessRecord` и `AppGroupRecord` (для кандидатов).

## 9.2. Подготовка датасета (Python)

* Для каждого `snapshot_id`:

  * query = список кандидатов (процессов/AppGroup);
  * X = фичи;
  * y:

    * сначала — teacher-score/класс;
    * затем — скорректированный с учётом `responsiveness_score`.

## 9.3. Обучение

* `CatBoostRanker`:

  * loss: YetiRank/PairLogit;
  * метрики: NDCG@k, RMSE по target-score.
* (опция) `CatBoostClassifier` для типов.

Экспорт:

* `model.onnx` (основной формат);
* `model.json` (резервное инференс-решение).

---

# 10. Стек библиотек

## 10.1. Python (обучение)

* `catboost`
* `numpy`
* `pandas`
* (опция) `scikit-learn`, `matplotlib`, `jupyterlab`

## 10.2. Rust — демон

**Инфраструктура:**

* `tokio`
* `tracing`, `tracing-subscriber`
* `serde`, `serde_yaml`, `serde_json`, `toml`
* `anyhow`/`eyre`, `thiserror`
* `clap`/`argh`

**Система и метрики:**

* `procfs`
* `psi`
* `nix`

**Cgroups / приоритет:**

* `cgroups-rs` (+ при необходимости прямой доступ к `/sys/fs/cgroup`)

**GUI/фокус:**

* X11: `x11rb`
* Wayland: `smithay-client-toolkit`, `wayland-client`

**Аудио:**

* PipeWire: `pipewire`
* PulseAudio (опция): `libpulse-binding` или `pulsectl-rs`

**Ввод/evdev:**

* `evdev` (или аналог) + `nix`

**ML-инференс:**

* основной: `onnxruntime` или `ort` (ONNX Runtime)
* резервный: `wafer-catboost` / `catboost` (JSON-инференс)

**Логирование/хранение:**

* `rusqlite` или `sqlx` (SQLite)
* `sled` (быстрый K/V, если потребуется)

**API (опция):**

* `axum` (или `warp`)
* `tonic` (gRPC)

---

# 11. Этапы внедрения

1. **MVP (rules-only)**

   * Метрики и классификация процессов/AppGroup по правилам.
   * Применение фиксированных классов → `nice`/`ionice`/cgroups.
   * Без ML, без логов (только runtime-логирование).

2. **Logging Mode**

   * Включение Snapshot Logger.
   * Расчёт `bad_responsiveness` и `responsiveness_score`.
   * Teacher-policy = существующая rules-логика.

3. **CatBoost v1**

   * Подготовка датасета, обучение Ranker’а на teacher-политике.
   * Инференс через ONNX/JSON в режиме `dry-run`.

4. **Hybrid Mode**

   * Использование Ranker’а для score внутри зон/классов.
   * Сохранение guardrails и семантических правил.
   * Мониторинг метрик отзывчивости до/после.

5. **Auto-tuning параметров**

   * Offline-оптимизация порогов policy по логам.
   * Постепенное улучшение классов и порогов.

6. **Расширения**

   * eBPF-метрики (при необходимости);
   * ML-классификатор типов процессов;
   * автообновление паттерн-базы приложений.

---

# 12. Вопросы к интернету для паттерн-базы и интеграций

При старте нужно собрать доп.данные/ресурсы:

1. **Списки приложений по категориям**

   * Браузеры, IDE, игры, плееры, терминалы, торрент-клиенты, билд-системы, индексаторы, апдейтеры.
   * Формат: имя бинарника / возможные пути / systemd units / snap/flatpak id.

2. **Существующие rule-config’и**

   * Конфиги Ananicy / Ananicy-CPP / uLatencyd:

     * как они группируют приложения;
     * какие классы и диапазоны nice используют;
     * какие паттерны уже охвачены.

3. **Низко-латентные гайды**

   * Статьи и конфиги по:

     * low-latency для PipeWire/JACK;
     * gaming-тюнинг (какие процессы важно поднимать, какие душить);
     * best-practice по background-сервисам.

4. **API композиторов и аудио-стеков**

   * Документация Mutter/KWin/wlroots по получению списка окон и PIDs (Wayland).
   * PipeWire/PulseAudio API для XRUN и latency.

5. **Поведенческие паттерны приложений**

   * Есть ли исследования/блоги по:

     * типичным CPU/IO-паттернам IDE, браузеров, игр;
     * их реакции на изменение nice/cgroups;
     * best-practice по ограничению «шумных соседей».

Получив эти данные, можно заполнить начальную паттерн-базу (`*.yml`) и сделать первый **teacher-ruleset**, на котором уже будет смысленно учить ранкер.

---

Это и есть финальное, собранное ТЗ: от целей и архитектуры до конкретного стека и шагов внедрения.


---


Красиво звучит 😎
**SmoothTask** — официально зафиксировано.

Чтобы дальше всё было ровно и консистентно, накину лёгкий «нейминг-пакет», без теории:

---

### 1. Как называть компоненты

* Демон:

  * бинарник: `smoothtaskd`
  * сервис: `smoothtaskd.service`
* Конфиг:

  * `/etc/smoothtask/smoothtask.yml`
* Логи/метрики:

  * `~/.local/share/smoothtask/` или `/var/lib/smoothtask/`
* Python-тренер:

  * репо/папка: `smoothtask-trainer`
  * основной скрипт: `smoothtask_train.py`

---

### 2. Внутренние названия модулей

Можно так:

* `smoothtask-core` — ядро правил и политики.
* `smoothtask-metrics` — сбор метрик (`/proc`, PSI, XRUN, окна, ввод).
* `smoothtask-ranker` — обёртка над ONNX/CatBoost.
* `smoothtask-agent` / `smoothtask-cli` — небольшая утилита для просмотра состояния.

---

### 3. Девиз (если захочешь куда-то написать)

Что-то в духе:

* **SmoothTask — keeps your desktop responsive under load.**
* или по-русски:
  **SmoothTask — чтобы система оставалась отзывчивой даже на 100% CPU.**

---


Окей, запускаем **SmoothTask v0.0.1 (skeleton edition)** 😄
Сделаю сразу нормальный монореп: Rust-демон + Python-тренер.

---

## 1. Структура репозитория

```text
smoothtask/
├── Cargo.toml                # workspace для Rust
├── rust-toolchain.toml       # (опционально) фиксируем версию Rust
├── README.md
├── LICENSE                   # позже
│
├── smoothtaskd/              # бинарник-демон
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
│
├── smoothtask-core/          # библиотека с логикой
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── config.rs
│       ├── metrics/
│       │   ├── mod.rs
│       │   ├── system.rs      # /proc, PSI, cgroups
│       │   ├── input.rs       # evdev
│       │   ├── windows.rs     # X11/Wayland
│       │   └── audio.rs       # PipeWire/PA
│       ├── model/
│       │   ├── mod.rs
│       │   ├── features.rs    # построение фич
│       │   └── ranker.rs      # обёртка над ONNX/JSON-моделью
│       ├── classify/
│       │   ├── mod.rs
│       │   └── rules.rs       # типы процессов / AppGroup
│       ├── policy/
│       │   ├── mod.rs
│       │   ├── classes.rs     # PRIORITY_CLASS → nice/ionice/cgroup
│       │   └── engine.rs      # применение правил + вызовы ранкера
│       ├── actuator.rs        # setpriority, ioprio_set, cgroups-rs
│       └── logging/
│           ├── mod.rs
│           └── snapshots.rs   # SQLite/файлы для обучения
│
├── configs/
│   ├── smoothtask.example.yml
│   └── patterns/
│       ├── browsers.yml
│       ├── ide.yml
│       ├── games.yml
│       ├── audio.yml
│       ├── batch.yml
│       └── maintenance.yml
│
├── smoothtask-trainer/       # Python-часть для обучения CatBoost
│   ├── pyproject.toml
│   ├── README.md
│   └── smoothtask_trainer/
│       ├── __init__.py
│       ├── dataset.py        # чтение логов, формирование выборки
│       ├── features.py       # трансформация/нормализация фич
│       ├── train_ranker.py   # обучение CatBoostRanker
│       ├── tune_policy.py    # оффлайн-тюнинг порогов
│       └── export_model.py   # сохранение ONNX/JSON
│
└── docs/
    ├── ARCHITECTURE.md
    ├── METRICS.md
    ├── POLICY.md
    └── ROADMAP.md
```

---

## 2. Корневой `Cargo.toml` (workspace)

```toml
[workspace]
members = [
    "smoothtaskd",
    "smoothtask-core",
]

resolver = "2"
```

---

## 3. `smoothtaskd/Cargo.toml` (демон)

```toml
[package]
name = "smoothtaskd"
version = "0.0.1"
edition = "2021"

[dependencies]
smoothtask-core = { path = "../smoothtask-core" }

tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
clap = { version = "4", features = ["derive"] }
anyhow = "1"
```

### `smoothtaskd/src/main.rs`

```rust
use anyhow::Result;
use clap::Parser;
use smoothtask_core::{config::Config, run_daemon};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "smoothtaskd", about = "SmoothTask daemon")]
struct Args {
    /// Путь к конфигу
    #[arg(short, long, default_value = "/etc/smoothtask/smoothtask.yml")]
    config: String,

    /// Dry-run: считать приоритеты, но не применять
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::load(&args.config)?;

    tracing::info!("Starting SmoothTask daemon (dry_run = {})", args.dry_run);

    run_daemon(config, args.dry_run).await
}
```

---

## 4. `smoothtask-core/Cargo.toml` (lib)

```toml
[package]
name = "smoothtask-core"
version = "0.0.1"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
tracing = "0.1"
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1.0"
toml = "0.8"

# системные штуки
procfs = "0.16"
psi = "0.1"              # если понадобится, можно скорректировать
nix = { version = "0.28", features = ["process", "signal"] }

# cgroups
cgroups-rs = "0.3"

# X11 / Wayland / audio будем добавлять по мере реализации:
# x11rb = "0.13"
# smithay-client-toolkit = "0.18"
# pipewire = "0.6"
# libpulse-binding = "2.28.1"

# ML-инференс через ONNX Runtime (вариант)
onnxruntime = { version = "0.19", features = ["download-binaries"] }

# SQLite / хранение снапшотов
rusqlite = { version = "0.31", features = ["bundled", "chrono"] }
chrono = { version = "0.4", features = ["serde"] }
```

### `smoothtask-core/src/lib.rs`

```rust
pub mod config;
pub mod metrics;
pub mod model;
pub mod classify;
pub mod policy;
pub mod actuator;
pub mod logging;

use anyhow::Result;
use config::Config;

/// Главный цикл демона: опрос метрик, ранжирование, применение.
pub async fn run_daemon(config: Config, dry_run: bool) -> Result<()> {
    // TODO:
    // 1. инициализация подсистем (cgroups, БД, model-инференс)
    // 2. основной loop:
    //    - metrics::collect_snapshot()
    //    - classify::apply_rules(...)
    //    - policy::evaluate_snapshot(...)
    //    - actuator::apply_changes(...)
    //    - logging::snapshots::maybe_log(...)
    loop {
        // временный заглушечный loop
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        tracing::debug!("SmoothTask tick (stub)");
        if dry_run {
            // в будущем сюда можно добавить отладочный вывод
        }
    }
}
```

### `smoothtask-core/src/config.rs`

```rust
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub polling_interval_ms: u64,
    pub max_candidates: usize,
    pub dry_run_default: bool,

    pub thresholds: Thresholds,
    pub paths: Paths,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Thresholds {
    pub psi_cpu_some_high: f32,
    pub psi_io_some_high: f32,
    pub user_idle_timeout_sec: u64,
    pub interactive_build_grace_sec: u64,
    pub noisy_neighbour_cpu_share: f32,

    pub crit_interactive_percentile: f32,
    pub interactive_percentile: f32,
    pub normal_percentile: f32,
    pub background_percentile: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Paths {
    pub snapshot_db_path: String,
    pub patterns_dir: String,
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let data = fs::read_to_string(path)?;
        let cfg: Config = serde_yaml::from_str(&data)?;
        Ok(cfg)
    }
}
```

---

## 5. Пример конфига `configs/smoothtask.example.yml`

```yaml
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "/var/lib/smoothtask/snapshots.sqlite"
  patterns_dir: "/etc/smoothtask/patterns"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
```

---

## 6. Python-тренер: `smoothtask-trainer/pyproject.toml`

```toml
[project]
name = "smoothtask-trainer"
version = "0.0.1"
description = "Trainer tools for SmoothTask (CatBoostRanker, policy tuning)"
authors = [
    { name = "Your Name", email = "you@example.com" }
]
requires-python = ">=3.10"

dependencies = [
    "catboost>=1.2",
    "numpy>=1.26",
    "pandas>=2.0",
    "scikit-learn>=1.5",
    "pyarrow>=16.0",
    "matplotlib>=3.8"
]

[project.optional-dependencies]
dev = [
    "jupyterlab",
    "black",
    "isort",
    "mypy",
]
```

### `smoothtask_trainer/train_ranker.py` (очень грубый каркас)

```python
from pathlib import Path

import pandas as pd
from catboost import CatBoostRanker, Pool

from .dataset import load_snapshots_as_frame
from .features import build_feature_matrix

def train_ranker(db_path: Path, model_out: Path, onnx_out: Path | None = None):
    df = load_snapshots_as_frame(db_path)
    X, y, group_id, cat_features = build_feature_matrix(df)

    train_pool = Pool(
        data=X,
        label=y,
        group_id=group_id,
        cat_features=cat_features,
    )

    model = CatBoostRanker(
        loss_function="YetiRank",
        depth=6,
        learning_rate=0.1,
        iterations=500,
        random_state=42,
    )
    model.fit(train_pool, verbose=True)

    model.save_model(model_out.as_posix(), format="json")

    if onnx_out is not None:
        model.save_model(onnx_out.as_posix(), format="onnx")


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--model-json", type=Path, required=True)
    parser.add_argument("--model-onnx", type=Path)
    args = parser.parse_args()

    train_ranker(args.db, args.model_json, args.model_onnx)
```

---

