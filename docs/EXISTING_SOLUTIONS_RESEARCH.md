# Исследование №2: Существующие решения для управления приоритетами процессов

**Анализ Ananicy, uLatencyd, Auto Nice Daemon и их применимость к SmoothTask**

---

## 0. Цель исследования

Проанализировать существующие подходы к автоматическому управлению приоритетами процессов в Linux, выявить лучшие практики, типичные паттерны и потенциальные проблемы для переноса в SmoothTask.

---

## 1. Какие подходы вообще есть

### 1.1. Ananicy / Ananicy-cpp

**Идея:**
простое правило → тип приложения → готовый пресет приоритетов.

* Правила лежат в `/etc/ananicy.d/*.rules`, каждое правило — JSON-объект вида
  `{"name":"gcc","type":"Heavy_CPU","nice":19,"ioclass":"best-effort","ionice":7,"cgroup":"cpu90"}`. Все поля кроме `name` опциональны.
* Есть **глобальный файл типов** `00-types.types`, где для каждого типа задан дефолтный `nice`, `ioclass`, иногда `latency_nice` и т.п.
* Есть набор «crowdsourced» правил: репозитории с типовыми правилами (CachyOS `ananicy-rules` и т.п.) — там уже готовы профили для игр, компиляторов, IDE, архиваторов, cloud-клиентов и прочего.

**Примеры:**

Из обзора Garuda Linux видно, что `00-types.types` содержит, например:

* `Game` — `nice ≈ -7`, `ioclass = "best-effort"`, `latency_nice ≈ -7`
* `Player-Audio` / `Player-Video` — `nice ≈ -4`, повышенный упор на снижение латентности
* `Image-View`, `Doc-View` — тоже `nice ≈ -4` как «приятные» интерактивные задачи

А `BG_CPUIO` используется для тяжёлых фоновых задач (compile, pacman, backup):

```json
{name: "mkinitcpio", type: "BG_CPUIO"}
{name: "makepkg",   type: "BG_CPUIO"}
{name: "pacman",    type: "BG_CPUIO"}
```

А в гайде по Ananicy показывают кастомное правило:

```json
{ "name": "timeshift", "type": "BG_CPUIO", "nice": 17, "ioclass": "idle" }
```

и правило для игры:

```json
{ "name": "BatmanAC.exe", "type": "Game" }
```

то есть:

* **тип** задаёт «базовый» набор приоритетов,
* правило для конкретного процесса может **переопределить** `nice` / `ioclass`.

### 1.2. uLatencyd

**Идея:**
не просто переписывать `nice`, а **динамически раскладывать процессы по cgroup'ам** на основе Lua-евристик.

* Демон на C, внутри — Lua-интерпретатор. Большинство правил и «планировщика» пишутся на Lua.
* Есть два механизма:
  * **timeout callbacks** — периодические эвристики;
  * **filter class** — фильтры, через которые прогоняется дерево процессов.
* Демон экспортирует в Lua инфу о процессах (CPU, IO, дерево процессов и т.д.), а фильтр возвращает флаги и таймаут, определяя:
  * в какой cgroup положить процесс,
  * когда к нему в следующий раз применять фильтр.

Ключевой момент: uLatencyd **ходит по дереву процессов** (от PID 1 вниз) и может принимать решения с учётом **родителя/детей**, а не только по имени процесса.

### 1.3. Auto Nice Daemon (`and`)

Самый простой, «олдскульный» вариант:

* периодически сканирует процессы и **автоматически renice'ит** их по CPU usage; root-процессы не трогает; никогда не повышает приоритет, только «дожимает вниз».
* Никаких типов, только «жирный → сделать nicer».

Полезен как референс того, **чего нам мало**: он видит только CPU и не знает ни про IO, ни про интерактивность.

---

## 2. Что именно делают конфиги с приоритетами

### 2.1. Диапазоны nice / ionice / latency_nice

Сводно по Ananicy / Ananicy-cpp:

* **Игры** (`Game`):
  * `nice` примерно от `-7` до `-5` (в Garuda прямо показано `-7` + `latency_nice = -7`).
  * `ioclass = "best-effort"`.
* **Мультимедиа-плееры** (`Player-Audio`, `Player-Video`):
  * `nice ≈ -4`,
  * акцент на снижении латентности (`latency_nice`).
* **Просмотрщики (Image-View, Doc-View)**:
  * тоже `nice ≈ -4` — чтобы UI не подлагивал при скролле.
* **Фоновые тяжёлые задачи (`BG_CPUIO`)**:
  * крайне «добрые» nice / ionice (почти «пусть всё остальное работает, а это — когда получится»). В статье про timeshift как backup-софт прямо сказано, что дефолтный пресет даёт *«самые низкие nice и ionice»*, и автор поднимает nice до 17 и переводит IO с `idle` на `best-effort`, чтобы бэкап не длился вечность.

По **ionice** (классика Linux):

* классы: `idle (3)`, `best-effort (2, уровни 0–7)`, `realtime (1)`;
* в `best-effort`: `0` — максимум, `7` — минимум.

Типичный паттерн в Ananicy:

* **Game** / **Player** → `best-effort` или `realtime` (аудио/видео);
* `BG_CPUIO` → `idle` или `best-effort` с низким приоритетом.

Плюс новые ядра добавляют `latency_nice` — отдельный «слайдер отзывчивости», который Ananicy-cpp уже использует.

---

## 3. Как именно классифицируют приложения

### 3.1. Ananicy

Там два уровня:

1. **Файл типов (`00-types.types`)**
   «Какие вообще бывают классы»:
   * Game
   * Player-{Audio,Video}
   * {Image-View, Doc-View}
   * BG_CPU, BG_CPUIO
   * Service, Daemon, Web-Browser, IM, Terminal и т.д. (по факту там большой словарь типов).

   Для каждого типа описываются: `nice`, `ioclass`, `latency_nice`, иногда cgroup/лимиты.

2. **Файлы правил (`*.rules` в `00-default/…`)**
   «Кто относится к какому типу»:
   * матчинг по `name` (имя бинаря: `vlc`, `firefox`, `mkinitcpio`);
   * при необходимости — по `cmdline` (чтобы различать, например, разные Java-приложения на одном `java`).
   * пример из CachyOS rules, уже приведённый выше, — Arch-утилиты → `BG_CPUIO`.

**Важные детали дизайна:**

* **Иерархия правил:** есть файлы `00-default`, дистро-специфичные (`archlinux.rules` и т.п.) и пользовательские; в обсуждениях Ananicy всплывает баг «берётся первое совпавшее правило, а не самое приоритетное», что показывает, насколько важно чётко определить порядок применения.
* **Валидация типов:** баг «Error: "type": "BG_CPUIO" not defined» — классический кейс, когда правило ссылается на тип, которого нет в `00-types.types`. Это прямо нам указывает на необходимость строго проверять консистентность rule-сет'а.

### 3.2. uLatencyd

Классификация более «умная», но тяжелее:

* В Lua-правилах есть **фильтры**, которые получают объект процесса (CPU/IO, дерево, флаги, cgroup и т.п.), и возвращают:
  * категорию / cgroup,
  * флаги поведения,
  * таймаут, когда фильтр надо запускать снова.
* Процессы обходятся в **порядке дерева** (от init к детям), поэтому можно, например, опознавать «детей игр», сборки от IDE и т.п.

Это очень похоже на то, что хочет SmoothTask: контекст по дереву + более сложные паттерны (компилятор как подпроцесс IDE, вкладка браузера как ребёнок `chrome` и т.п.).

### 3.3. Auto Nice Daemon

Здесь классификация минимальна:

* daemon периодически смотрит: «этот процесс долго жрёт CPU?»,
* если да — повышает `nice` (понижает приоритет), но:
  * root-процессы не трогает,
  * не занимается IO, cgroup, latency.

Это хороший негативный пример: **таких эвристик мало**, но идея «порогов по CPU» нам всё равно полезна как одна из фич.

---

## 4. Типичные паттерны rule-config'ов, которые стоит украсть

По сути, из Ananicy/Ananicy-cpp + uLatencyd можно собрать «best of»:

### 4.1. Конфиг-слои и приоритеты

У Ananicy:

* directory layout: `00-default/`, дальше сабдиры `games/`, `desktop/`, `net/`, `archlinux.rules` и т.п.;
* на практике есть вопрос: что важнее — правило дистрибутива или пользовательское; в issues прямо всплывает проблема с «first match wins».

Для SmoothTask:

* делаем **явные уровни**:
  * `vendor` (bundled пресеты),
  * `distro` (опционально),
  * `user`,
  * `runtime` (авто-паттерны, выученные ML).
* правила настраивают **приоритет слоя**, а не надеются на порядковый номер файла.

### 4.2. Типы как «шаблоны поведения»

Концепция типов у Ananicy реально удачная:

* Тип — это **не приложение**, а **поведенческий класс**:
  * `Game`, `Player-Audio`, `BG_CPUIO`, `Image-View`, `Service`, …
* Тип задаёт:
  * целевой коридор `nice` (например, `[-10, 0]`),
  * коридор `latency_nice`,
  * дефолтный `ioclass` + диапазон `ionice`,
  * возможные лимиты cgroup (ограничение ядер, cpu.shares и т.п.).

В SmoothTask можно:

* использовать тот же список как **базовую онтологию**;
* добавить специфичные для 2025 года типы:
  * `LLM-Worker`, `GPU-Renderer`, `VM`, `Container`, `Browser-Tab`, …
* и уже **ранкер** будет говорить: «внутри Game этот конкретный экземпляр сейчас в топ-3 по важности».

### 4.3. Матчинг процессов

Ananicy опирается на:

* `name` (бинарь),
* `cmdline` (подразличение по аргументам),
* иногда `cgroup`.

uLatencyd добавляет:

* родителя / дерево процессов,
* тайм-срезы поведения (CPU, IO, swap).

Для SmoothTask имеет смысл:

* **в конфиге** поддержать:
  * `name`, `cmdline`, `cwd`, `user`, `env`, `cgroup`, `parent_name`;
* **в ML-части** использовать:
  * историю CPU/IO,
  * «активность» окна (из внешних хуков),
  * дерево процессов.

---

## 5. Какие грабли у чужих rule-сет'ов и как их учесть

По Ananicy:

* **Неопределённые типы** (`type: BG_CPUIO not defined`) — значит, нужен:
  * строгий валидатор конфигов;
  * режим «не запускать daemon, пока конфиг не проходит валидацию».
* **Ломаная приоритизация правил** (берётся первое совпавшее) — нам нужно:
  * чёткая стратегия merge'а слоёв;
  * лог «какое правило сработало и почему».
* **Гиперактивность** (частый пересчёт и renice) — значит:
  * SmoothTask должен иметь **rate limiting** на пересчёты,
  * и минимум «дёргания» process-priorities (гистерезис).

По uLatencyd:

* сложный стек (Lua + dbus + патченный libprocps) → тяжело поддерживать;
* битрот: проект автором заброшен, хотя идея норм.

Вывод: в SmoothTask лучше:

* **ядро логики — Rust**, без внешнего интерпретатора;
* если нужен скриптинг, делать лёгкий DSL или WASM-плагины, но с очень жёсткими лимитами.

---

## 6. Как это переложить в SmoothTask (конкретика)

С точки зрения «языка правил» и дефолтных конфигов:

### 6.1. Структура типов (`types.yaml`)

Вводим `types.yaml` (аналог `00-types.types`):

```yaml
types:
  - type: Game
    nice_range: [-10, -2]
    latency_nice_range: [-10, -2]
    ioclass: best-effort
    ionice_range: [0, 4]
    cpu_weight: 200
    description: "Игры и игровые лаунчеры"

  - type: Player-Audio
    nice_range: [-6, -2]
    latency_nice_range: [-8, -2]
    ioclass: best-effort
    ionice_range: [0, 3]
    cpu_weight: 150
    description: "Аудиоплееры"

  - type: Player-Video
    nice_range: [-6, -2]
    latency_nice_range: [-8, -2]
    ioclass: best-effort
    ionice_range: [0, 3]
    cpu_weight: 150
    description: "Видеоплееры"

  - type: Web-Browser
    nice_range: [-4, 0]
    latency_nice_range: [-6, 0]
    ioclass: best-effort
    ionice_range: [2, 5]
    cpu_weight: 150
    description: "Веб-браузеры"

  - type: IDE
    nice_range: [-4, 0]
    latency_nice_range: [-6, 0]
    ioclass: best-effort
    ionice_range: [2, 5]
    cpu_weight: 150
    description: "IDE и редакторы кода"

  - type: Image-View
    nice_range: [-4, 0]
    latency_nice_range: [-4, 0]
    ioclass: best-effort
    ionice_range: [3, 5]
    cpu_weight: 150
    description: "Просмотрщики изображений"

  - type: Doc-View
    nice_range: [-4, 0]
    latency_nice_range: [-4, 0]
    ioclass: best-effort
    ionice_range: [3, 5]
    cpu_weight: 150
    description: "Просмотрщики документов"

  - type: Terminal
    nice_range: [-4, 0]
    latency_nice_range: [-4, 0]
    ioclass: best-effort
    ionice_range: [3, 5]
    cpu_weight: 150
    description: "Терминальные эмуляторы"

  - type: Service
    nice_range: [-2, 2]
    latency_nice_range: [-2, 2]
    ioclass: best-effort
    ionice_range: [4, 6]
    cpu_weight: 100
    description: "Системные сервисы"

  - type: Daemon
    nice_range: [0, 5]
    latency_nice_range: [0, 5]
    ioclass: best-effort
    ionice_range: [4, 6]
    cpu_weight: 100
    description: "Фоновые демоны"

  - type: BG_CPU
    nice_range: [5, 19]
    latency_nice_range: [5, 19]
    ioclass: best-effort
    ionice_range: [5, 7]
    cpu_weight: 50
    description: "Фоновые CPU-задачи"

  - type: BG_CPUIO
    nice_range: [10, 19]
    latency_nice_range: [10, 19]
    ioclass: idle
    ionice_range: [6, 7]
    cpu_weight: 25
    description: "Тяжёлые фоновые задачи (компиляция, бэкапы)"
```

### 6.2. Структура правил (`rules.d/*.yaml`)

Вводим `rules.d/*.yaml` (аналог `*.rules`):

```yaml
# vendor/default/rules.yaml
rules:
  - match:
      name: "chrome"
    type: Web-Browser
    priority: 100  # vendor layer

  - match:
      name: "firefox"
    type: Web-Browser
    priority: 100

  - match:
      name: "code"
    type: IDE
    priority: 100

  - match:
      name: "mkinitcpio"
    type: BG_CPUIO
    priority: 100

  - match:
      name: "makepkg"
    type: BG_CPUIO
    priority: 100

  - match:
      name: "pacman"
    type: BG_CPUIO
    priority: 100

  - match:
      name: "timeshift"
    type: BG_CPUIO
    override:
      nice: 17
      ioclass: best-effort  # не idle, чтобы не длилось вечность
    priority: 100

# user/rules.yaml (пример пользовательского правила)
rules:
  - match:
      name: "BatmanAC.exe"
    type: Game
    priority: 200  # user layer выше vendor
```

### 6.3. Расширенный матчинг

Поддержка более сложных условий:

```yaml
rules:
  - match:
      name: "java"
      cmdline_contains: ["intellij"]
    type: IDE
    priority: 100

  - match:
      parent_name: "chrome"
      cmdline_contains: ["--type=renderer"]
    type: Web-Browser
    description: "Renderer процессы Chrome"
    priority: 100

  - match:
      name: "clang"
      parent_name: ["code", "vscode", "clion"]
    type: BG_CPU
    override:
      cpu_weight: 50
      description: "Компилятор под IDE"
    priority: 100

  - match:
      cgroup_contains: "docker"
    type: Container
    priority: 100

  - match:
      user: "user"
      env_has: "DISPLAY"
      has_gui_window: true
    type: GUI-Interactive
    priority: 50  # fallback правило
```

### 6.4. Внутренняя логика демона

1. **Находим процесс** → применяем **конфиг-matcher** → получаем базовый тип.
2. **ML-ранкер** говорит: «внутри текущего типа этот процесс имеет score 0.9, а другой — 0.2».
3. По типу и score выбираем конкретные `nice`, `latency_nice`, `ionice` в разрешённом диапазоне.
4. **Берём идею uLatencyd с деревом процессов:**
   * для compiled-подпроцессов (`clang` под `vscode`) можно делать:
     * родитель `vscode` — интерактивный, его нельзя убивать,
     * дочерние `clang` — `BG_CPU` с сильным ограничением.

---

## 7. Итог по исследованию №2

Если коротко:

* **Ananicy/Ananicy-cpp** — хороший пример:
  * как описывать **типы приложений** и **шаблоны приоритетов**;
  * как хранить правила в виде маленьких файлов;
  * какие диапазоны nice/ionice разумно использовать для игр, плееров, фоновых задач.
* **uLatencyd** показывает, как:
  * использовать **дерево процессов** и cgroups;
  * писать сложные эвристики (но мы не хотим Lua и старый стек).
* **Auto Nice Daemon** полезен как минималистичная идея «простого CPU-порога», но этого явно недостаточно — нам нужны ещё IO, latency, интерактивность.

Для SmoothTask логично:

1. **Утащить схему типов и rule-директории из Ananicy**, но:
   * добавить строгую валидацию;
   * нормальный приоритет слоёв (vendor/distro/user/ML).
2. **Взять из uLatencyd идею фильтров по дереву процессов** и cgroups, но реализовать их как часть Rust-ядра.
3. Использовать эти rule-конфиги как **baseline**, поверх которого уже работает CatBoost/ML-ранкер.

---

## 8. Следующие шаги

1. Реализовать парсер `types.yaml` и валидатор типов
2. Реализовать парсер правил из `rules.d/` с поддержкой слоёв
3. Создать matcher процессов по правилам (name, cmdline, parent, cgroup и т.д.)
4. Интегрировать типы в Policy Engine как baseline для ML-ранкера
5. Добавить гистерезис и rate limiting для предотвращения излишних пересчётов

---

## Источники

- [How to Control App Priorities with Ananicy in Linux - Make Tech Easier](https://www.maketecheasier.com/control-apps-priorities-with-ananicy-linux/)
- [Garuda Linux Review - ordinatechnic.com](https://www.ordinatechnic.com/distribution-reviews/garuda-linux/garuda-linux-review-kde-dragonized-210621)
- [uLatencyd GitHub](https://github.com/poelzi/ulatencyd)
- [Ananicy Issues - GitHub](https://github.com/nefelim4ag/Ananicy/issues)

