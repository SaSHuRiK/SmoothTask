# Исследование №4: API композиторов и аудио-стеков для метрик латентности

**Анализ доступных API для получения информации об окнах, процессах и метрик аудио-системы**

---

## 0. Цель исследования

Нам нужно понять, **что именно мы можем достать из системы автоматически**:

1. Список окон / приложений, их состояние (фокус, свернуто, на каком workspace), желательно с PID или хотя бы app_id.
2. Метрики аудио-подсистемы, отражающие:
   * текущую/эффективную задержку (latency),
   * XRUN'ы / underrun'ы (хрип, «бульканье», пропуски),
   * загрузку аудио-графа (на уровне драйвера и отдельных клиентов).
3. Нас интересуют **интерфейсы, которые можно дергать из демона SmoothTask (Rust)** с минимальными накладными расходами.

---

## 1. Wayland / X11: что реально можно получить про окна

### 1.1. Ограничения Wayland по дизайну

В Wayland **нет стандартного API «дай список всех окон»**. Это сознательное решение: защита приватности, чтобы любое приложение не могло подглядывать за другими.
Вместо этого каждый композитор (Mutter, KWin, Sway, Hyprland и т.д.) добавляет свои расширения протоколов.

**Вывод:** нам нужен **абстрактный слой `WindowIntrospector` с backend'ами под разные композиторы**, а не одна универсальная «магия».

### 1.2. wlroots / wlr-foreign-toplevel-management

Есть полу-дефакто стандарт для панелей/доков — протокол **`wlr-foreign-toplevel-management-unstable-v1`**:

* даёт:
  * список toplevel-окон;
  * заголовок (`title`),
  * `app_id`,
  * состояние: активировано, свернуто, полноэкранный режим и т.п.
* поддерживается многими композиторами: **Mutter (GNOME), KWin, Sway, Hyprland, Wayfire и др.** (видно в разделе *Compositor Support* в описании протокола).

Но **важный момент**: протокол **не даёт PID**. Там только логическая идентичность приложения (`app_id`) и состояние окна.

**Для SmoothTask практический вывод:**

* Имеет смысл сделать **Wayland-клиент-библиотеку** (на Rust) для `wlr-foreign-toplevel-management` / `ext-foreign-toplevel-list`:
  * собираем: `title`, `app_id`, `state`, workspace, фокус;
  * считаем признаки:
    * `is_focused`,
    * `is_minimized`,
    * `is_fullscreen`,
    * «есть ли вообще window у процесса» (GUI vs CLI/daemon — косвенный признак).
* PID придётся восстанавливать косвенно (см. ниже, через PipeWire и /proc).

### 1.3. KWin (KDE Plasma)

У KWin есть два рабочих подхода:

1. **Скрипты на JS + D-Bus**:
   * В Wayland-режиме можно написать KWin-скрипт, который делает `workspace.windowList()` и печатает `caption` (заголовки окон).
   * Его можно грузить и запускать через D-Bus (`org.kde.KWin /Scripting ...`), а результат читать из `journalctl`.
   * Более новые версии KWin используют `workspace.windowList()`, а не `clientList()`.

2. **Wayland-протоколы KDE + wlr-foreign-toplevel**:
   * KWin поддерживает KDE-расширения (`KDE_plasma_window_management`) и **wlr-foreign-toplevel-management**, поэтому в будущем можно обойтись без скриптов и читать список окон через протокол, как и у wlroots-композиторов.

**Для SmoothTask:**

* Можно сделать отдельный backend `KWinScriptBackend`:
  * Скрипт отдаёт минимум: `caption`, workspace, состояние окна.
  * Если удастся — плюс PID (иногда его можно получить из KWin API, но это зависит от версии).
* Либо сразу опереться на `wlr-foreign-toplevel`, если в целевой версии Plasma он уже есть и стабилен.

### 1.4. Mutter / GNOME Shell

Для GNOME/Mutter ситуация сложнее:

* В X11-режиме GNOME использует стандартный стек EWMH, и всё можно взять как на X.org.
* В Wayland-режиме:
  * есть внутренняя JS-API (`Meta.Window`) в Shell-расширениях, но разработчики GNOME подчёркивают, что **PID под Wayland может быть недоступен или -1**, и API не гарантирует стабильный доступ к нему.
  * GNOME также подтянул поддержку **`wlr-foreign-toplevel-management`**, его видно в списке композиторов для этого протокола.

**Итого для SmoothTask:**

* Базовый путь: **GNOME-backend на `wlr-foreign-toplevel-management`**:
  * получаем `title`, `app_id`, фокус, workspace.
* Продвинутый путь (опциональный): **Shell-extension**:
  * расширение может:
    * перечислить окна через Meta API;
    * попытаться получить PID (если доступен);
    * написать лёгкий D-Bus API, выдающий JSON со списком окон;
  * это уже «deep integration» и требует упаковки расширения под GNOME.

### 1.5. X11 (legacy, но всё ещё важно)

Под X11 всё проще:

* Используем стандарт EWMH (как делает `wmctrl` / `xdotool`), через **Xlib/libxcb**:
  * список окон, их состояния, workspace;
  * через `_NET_WM_PID` почти всегда получается PID.
* Это хороший fallback для:
  * старых систем;
  * случаев, когда пользователь сам выбрал X11-сессию.

**Для SmoothTask:**

* Имеет смысл реализовать **X11-backend** сразу (дёшево и даёт много фич), а Wayland-композиторы накрывать постепенно.

---

## 2. Связка «окна ↔ PID ↔ cgroup»

Для ранкера нам важна именно **процессная сущность**: PID/cgroup, а не окно само по себе.

**Подход:**

1. **PipeWire даёт PID напрямую для аудио-клиентов**:
   * node/stream'ы содержат свойства типа `application.process.id`, `application.process.binary`, `application.name`, и т.д.
   * это идеальный мост: аудио-активный PID → дальше через `/proc/<pid>/cgroup` получаем cgroup и связываем с CPU-метриками.

2. **Окна под Wayland: PID не всегда есть**:
   * если протокол/композитор его не даёт, делаем «best effort»:
     * используем `app_id` (часто совпадает с именем `.desktop`/application id);
     * ищем процессы с совпадающим `argv[0]` / `cmdline` / `Desktop` в окружении;
     * при наличии PipeWire stream'ов с таким `application.name` / `application.id` — склеиваем.
   * при неуверенности даём ранкеру признак `pid_confidence ∈ [0..1]`.

3. **Дерево процессов / cgroup**:
   * финальная сущность для приоритизации — **«группа процессов приложения»**:
     * браузер с кучей воркеров,
     * IDE + компилятор,
     * Wine-игра с дочерними процессами.
   * мы берём:
     * основной PID,
     * всех детей из того же cgroup / родительской цепочки,
     * агрегируем метрики (CPU, IO, audio_lag, window state) для группы.

---

## 3. PipeWire: что можно измерять про задержку и XRUN

### 3.1. Низкоуровневый API (`pw_stream_get_time_n` / `pw_time`)

Документация PipeWire описывает структуру **`pw_time`**, которую можно получить через `pw_stream_get_time_n()`:

* `now` — время обновления (ns),
* `delay` — задержка до устройства (в тиках, можно перевести в мс),
* `queued` — сколько данных в очереди,
* `buffered` — сколько кадров в буферах/ресемплере,
* `ticks` — монотонный счётчик времени графа, по его скачкам можно ловить дискретности (XRUN'ы).

Это позволяет на уровне демона:

* вычислять **общую задержку** нового блока данных до выхода в железо;
* отслеживать стабильность времени (дискретности → возможные XRUN).

### 3.2. `pw-top`, ERR и загрузка графа

Инструмент **`pw-top`** показывает:

* `WAIT` — время ожидания узла до старта обработки;
* `BUSY` — время обработки;
* `W/Q` — отношение WAIT/QUANT, **метрика загрузки графа**;
* `B/Q` — загрузка конкретного узла;
* `ERR` — суммарный счётчик XRUN'ов и ошибок по узлу.

Из ман-страницы и документации видно, что:

* XRUN для драйвера = цикл графа не успел уложиться в дедлайн;
* XRUN для follower-узла = он не успел завершить обработку до конца цикла.

**Практически для SmoothTask:**

* Мы можем **повторить логику `pw-top` внутри демона** через libpipewire:
  * подписаться на события графа,
  * получать WAIT/BUSY/ERR для драйверов и клиентов,
  * считать rolling-метрики:
    * `dsp_load` (по W/Q драйвера),
    * `xruns_per_minute_global`,
    * `xruns_per_minute_per_node`.

### 3.3. `pw-dump` и свойства узлов

`pw-dump` даёт JSON с полным состоянием графа: узлы, порты, устройства, их свойства.

* Там же видны:
  * `node.name`, `node.description`, `media.*`,
  * `application.process.id`, `application.name`, etc.

**Для демона:**

* раз в N секунд можно делать дешёвую синхронизацию — обновлять карту `PipeWire node id → PID → app group`;
* тяжёлый polling `pw-dump` часто делать не надо — лучше подписка на события через API (появился/пропал узел, изменились свойства).

---

## 4. PulseAudio: латентность и underrun'ы

PulseAudio — легаси, но всё ещё встречается или эмулируется через `pipewire-pulse`.

### 4.1. API для измерения латентности

Официальные доки по потокам рекомендуют:

* вызывать `pa_stream_update_timing_info()`,
* затем через `pa_stream_get_timing_info()` получить структуру с индексами буфера,
* или напрямую `pa_stream_get_latency()` / `pa_stream_get_time()` для уже «очищенных» значений.

Пример кода с измерением latency есть в официальных примерах (`pacat`, демо для Async API).

**На уровне CLI:**

* `pactl list sinks` показывает **текущую и сконфигурированную задержку**:
  * `Latency: 103758 usec, configured 100000 usec`,
  * плюс есть параметры `tsched`, `fragments`, `fragment_size`, `fixed_latency_range`, которые сильно влияют на латентность и устойчивость к underrun.

### 4.2. XRUN / underrun

Типично:

* ALSA при underrun'ах пишет в лог сообщения вида `Underrun!` / `XRUN` — это видно по dmesg/journal;
* PulseAudio в логах уровня `info`/`debug` пишет `Underrun`, `Scheduling delay` и т.п.

**Для SmoothTask:**

* На PulseAudio-системах простой вариант:
  * раз в N секунд парсить вывод `pactl list sinks` на предмет latency,
  * при необходимости «продвинутый» режим — собирать логи PulseAudio и считать частоту underrun'ов.
* На PipeWire (через `pipewire-pulse`) **лучше работать напрямую с PipeWire**, а не с эмулируемым Pulse.

---

## 5. Как всё это встраивается в SmoothTask

### 5.1. Слой абстракций

Предлагается явно ввести интерфейсы:

* **`WindowIntrospector`**:
  * реализации:
    * `X11Introspector` (EWMH),
    * `WlrForeignToplevelIntrospector` (Mutter/KWin/Sway/Hyprland/Wayfire…),
    * `KWinScriptIntrospector` (если нужно что-то специфичное KDE),
    * (опционально) `GnomeShellExtensionIntrospector`.
* **`AudioIntrospector`**:
  * `PipeWireIntrospector` (основной),
  * `PulseAudioIntrospector` (fallback там, где PipeWire нет).

Каждый из них выдаёт **нормализованные структуры**:

```rust
pub struct WindowInfo {
    pub app_id: Option<String>,
    pub title: Option<String>,
    pub workspace: Option<u32>,
    pub is_focused: bool,
    pub is_minimized: bool,
    pub is_fullscreen: bool,
    pub pid: Option<u32>,
    pub pid_confidence: f32,
}

pub struct AudioNodeInfo {
    pub pid: Option<u32>,
    pub node_id: u32,
    pub app_name: Option<String>,
    pub latency_ms: Option<f32>,
    pub xruns_total: u64,
    pub xruns_recent: u64,  // за последнее окно времени
    pub dsp_load_local: f32,
}

pub struct AudioGraphInfo {
    pub dsp_load_global: f32,
    pub xruns_global_recent: u64,
}
```

Эти структуры дальше идут в **фич-экстрактор ранкера**.

### 5.2. Метрики «лага» без пользовательских меток

То, о чём спрашивалось ранее: *как оценить «плохую отзывчивость» без явного фидбэка?*

С опорой на найденные API:

* **Аудио-лаг / хрипы**:
  * PipeWire: рост `ERR` (XRUN'ы) по драйверу и конкретным audio-клиентам, всплески WAIT/QUANT > порога;
  * PulseAudio: частота underrun-сообщений + слишком малые буферы (через конфиг/`pactl`).
* **Системный лаг**:
  * при высоком CPU:
    * DSP load (W/Q драйвера) уходит к 1.0;
    * растёт latency аудио-стримов (PipeWire/PulseAudio);
    * одновременно видно активные окна + PID'ы.
* Для SmoothTask это превращается в:
  * три набора метрик:
    * `global_responsiveness_score`,
    * `audio_stability_score`,
    * `per_app_responsiveness_score(pid)`.

Ранкер может использовать их как таргет/feedback-сигнал при offline-обучении:

* «когда мы меняли приоритет так-то, метрика `audio_stability_score` улучшалась → такой паттерн полезен».

---

## 6. Архитектура реализации в Rust

### 6.1. Модуль `smoothtask-window`

```rust
pub trait WindowIntrospector: Send + Sync {
    fn get_windows(&self) -> Result<Vec<WindowInfo>>;
    fn get_focused_window(&self) -> Result<Option<WindowInfo>>;
    fn supports_pid(&self) -> bool;
}

// Реализации
pub struct X11Introspector { /* x11rb-based */ }
pub struct WlrForeignToplevelIntrospector { /* wayland-client-based */ }
pub struct KWinScriptIntrospector { /* dbus-based */ }
```

### 6.2. Модуль `smoothtask-audio`

```rust
pub trait AudioIntrospector: Send + Sync {
    fn get_audio_graph_info(&self) -> Result<AudioGraphInfo>;
    fn get_audio_nodes(&self) -> Result<Vec<AudioNodeInfo>>;
    fn get_node_by_pid(&self, pid: u32) -> Result<Option<AudioNodeInfo>>;
}

// Реализации
pub struct PipeWireIntrospector {
    // libpipewire-rs bindings
    // подписка на события через pw_context
}

pub struct PulseAudioIntrospector {
    // libpulse-binding или pulsectl-rs
    // polling через pactl или прямой API
}
```

### 6.3. Интеграция с Metrics Collector

```rust
// В smoothtask-core/src/metrics/mod.rs
pub struct Snapshot {
    pub global: GlobalMetrics,
    pub windows: Vec<WindowInfo>,
    pub audio: AudioGraphInfo,
    pub audio_nodes: Vec<AudioNodeInfo>,
    pub processes: Vec<ProcessRecord>,
    // ...
}

// WindowInfo и AudioNodeInfo используются для:
// 1. Обогащения ProcessRecord (has_window, is_focused, audio_client)
// 2. Построения AppGroup (группировка по окнам и аудио-связям)
// 3. Экстракции фич для ранкера
```

---

## 7. Риски и ограничения

1. **Wayland-privacy**:
   * нет гарантий, что на всех композиторах можно стабильно получить PID окна;
   * система должна уметь жить с `pid = None` и опираться на CPU+PipeWire.

2. **Браузеры и сложные приложения**:
   * у Chromium-подобных: отдельные процессы под вкладки, GPU, сеть и т.д.;
   * аудио-процесс ≠ CPU-heavy процесс вкладки;
   * придётся объединять по cgroup и/или родителю.

3. **Накладные расходы**:
   * опираться на **событийные API** (PipeWire, Wayland),
   * минимизировать агрессивный polling (`pw-dump`, `pactl` и т.п.).

4. **Совместимость**:
   * PipeWire API может меняться между версиями;
   * Wayland протоколы могут отличаться между композиторами;
   * нужны fallback-механизмы и версионирование API.

---

## 8. Вывод

Для SmoothTask у нас есть **реально используемый путь**:

* **по окнам и фокусу**:
  * `wlr-foreign-toplevel-management` + X11-fallback + точечные интеграции с KWin / GNOME;
* **по аудио**:
  * PipeWire как основной источник **latency + XRUN + PID**, с повторением логики `pw-top` внутри демона;
  * PulseAudio — только как fallback.

**Структура модулей:**

1. `smoothtask-window` — абстракция над X11/Wayland для получения информации об окнах
2. `smoothtask-audio` — абстракция над PipeWire/PulseAudio для метрик аудио
3. Интеграция в `smoothtask-core` через `WindowIntrospector` и `AudioIntrospector` traits

**Следующие шаги:**

1. Реализовать базовый X11-backend для получения окон и PID
2. Реализовать PipeWire-интроспектор для метрик аудио
3. Добавить поддержку `wlr-foreign-toplevel-management` для Wayland
4. Интегрировать метрики окон и аудио в процесс классификации и ранжирования

---

## Источники

- [wlr-foreign-toplevel-management protocol](https://wayland.app/protocols/wlr-foreign-toplevel-management-unstable-v1)
- [PipeWire Documentation](https://docs.pipewire.org/)
- [PipeWire Key Names](https://docs.pipewire.org/group__pw__keys.html)
- [pw-top man page](https://docs.pipewire.org/page_man_pw-top_1.html)
- [pw-dump man page](https://docs.pipewire.org/page_man_pw-dump_1.html)
- [PulseAudio Streams Documentation](https://freedesktop.org/software/pulseaudio/doxygen/streams.html)
- [PulseAudio and Latency - The Blog of Juho](https://juho.tykkala.fi/Pulseaudio-and-latency)
- Обсуждения на форумах KWin, GNOME, Sway



