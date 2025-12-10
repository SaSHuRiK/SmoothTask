# Исследование №1: Паттерн-база приложений для SmoothTask

**Категории приложений под Linux — для приоритизации процессов**

---

## 0. Цель исследования

Задача: набрать список приложений, который покроет 80–90% реальных десктоп-сценариев и удобно ляжет в `patterns/*.yml`.

### Цели:

1. Выделить **ключевые категории приложений**, важные для приоритизации:
   * браузеры;
   * IDE/редакторы кода;
   * терминалы;
   * торрент-клиенты;
   * аудиоплееры;
   * видеоплееры;
   * индексаторы/поисковые демоны;
   * обновлялки/пакетные менеджеры;
   * сборщики/билд-системы (как «дети» IDE/CLI).

2. Для каждой категории:
   * зафиксировать **самые распространённые приложения на Linux к 2024–2025**;
   * понять, **какие идентификаторы** мы можем использовать:
     * `exe`/`comm`,
     * `.desktop`/AppId,
     * systemd-unit/cgroup-путь,
     * иногда — пакет или Snap/Flatpak id.

3. В конце — предложить **скелет YAML-паттерна** под SmoothTask.

---

## 1. Браузеры

### Какие считаем «основными»

По обзорам браузеров для Linux/Ubuntu 2024–2025 и рыночной доле настольных браузеров:

#### Chromium-семейство:
* Google Chrome (`google-chrome`, `chrome`, `google-chrome-stable`);
* Chromium (`chromium`, `chromium-browser`);
* Microsoft Edge (`microsoft-edge`, `msedge`);
* Brave (`brave`, `brave-browser`);
* Vivaldi (`vivaldi`, `vivaldi-bin`);
* Opera (`opera`, `opera-beta`, `opera-developer`);
* различные форки: Thorium, Ungoogled Chromium, etc. (по `exe` и desktop-файлам).

#### Firefox-семейство:
* Mozilla Firefox (`firefox`, `firefox-esr`);
* LibreWolf, Waterfox и др. (обычно свои бинарники / desktop-файлы).

#### Другие:
* GNOME Web / Epiphany (`epiphany`, `org.gnome.Epiphany`);
* Falkon (`falkon`);
* Midori (`midori`);
* Tor Browser (тут сложнее: поверх `firefox` в отдельной директории).

### Полезные признаки для паттернов

* `exe` / `comm`:
  * `chrome`, `chromium`, `firefox`, `vivaldi-bin`, `brave`, `opera`, `microsoft-edge`, `epiphany`, `falkon`…
* `.desktop` / AppId (из `XDG_CURRENT_DESKTOP`+`desktop_id`):
  * `google-chrome.desktop`, `chromium-browser.desktop`, `firefox.desktop`, `brave-browser.desktop`, `vivaldi-stable.desktop`, `org.gnome.Epiphany.desktop`, `org.kde.falkon.desktop`, и т.п.
* cgroup/службы:
  * иногда браузеры в собственных slices, но надёжнее `exe + desktop`.

---

## 2. IDE и редакторы кода

По свежим подборкам и обсуждениям IDE/редакторов 2024–2025:

### GUI IDE/редакторы:

* Visual Studio Code (`code`, `code-insiders`), VSCodium (`vscodium`);
* JetBrains:
  * IntelliJ IDEA (`idea`, `idea64`, `intellij-idea-*`);
  * PyCharm (`pycharm`, `pycharm-community`);
  * CLion, WebStorm, PhpStorm, GoLand, Rider (`clion`, `webstorm`, …);
  * RustRover (`rustrover`);
* Sublime Text (`subl`, `sublime_text`);
* Kate (`kate`);
* KDevelop (`kdevelop`);
* Geany (`geany`);
* Eclipse (`eclipse`).

### Терминальные/«хайбрыды»:

* Vim/Neovim (`vim`, `nvim`);
* Emacs (`emacs`, иногда в GUI-режиме);
* Micro/Helix/Zed и т.п. (по мере необходимости).

### Признаки

* `exe`/`comm`: `code`, `vscodium`, `idea64`, `pycharm`, `clion`, `webstorm`, `kate`, `kdevelop`, `geany`, `eclipse`, `nvim`, `vim`, `emacs`…
* `.desktop`: `code.desktop`, `jetbrains-*`, `org.kde.kate.desktop`, `org.kde.kdevelop.desktop`, `geany.desktop`, и т.д.
* cgroup/unit:
  * JetBrains часто в unit'ах с `jetbrains-` в имени (зависит от пакета).

---

## 3. Терминалы

Сводим из списков терминалов и обзоров 2024–2025:

### Классика DE:

* GNOME Terminal (`gnome-terminal`);
* Konsole (`konsole`);
* Xfce Terminal (`xfce4-terminal`);
* XTerm (`xterm`);
* LXQt / LXDE терминалы.

### Продвинутые/tiling:

* Terminator (`terminator`);
* Tilix (`tilix`);
* Guake (`guake`), Yakuake (`yakuake`);
* Kitty (`kitty`);
* Alacritty (`alacritty`);
* WezTerm (`wezterm`);
* Cool Retro Term (`cool-retro-term`);
* st (`st`);
* urxvt (`urxvt`, `rxvt-unicode`);
* Black Box, GNOME Console (`kgx`, `gnome-console`).

### Признаки

* `exe` по имени терминала;
* `.desktop` (если GUI):
  * `org.gnome.Terminal.desktop`, `konsole.desktop`, `tilix.desktop`, `terminator.desktop`, `org.wezfurlong.wezterm.desktop` и т.п.
* TTY-привязка:
  * важно больше для `cli_interactive`, но терминалы дают контекст для детей.

---

## 4. Торрент-клиенты

По TechRadar, обзорам и спискам BitTorrent-клиентов 2024–2025:

* qBittorrent (`qbittorrent`);
* Transmission (`transmission-gtk`, `transmission-qt`, `transmission-daemon`);
* Deluge (`deluge`, `deluged`);
* Vuze/Azureus (`vuze`, `azureus`);
* Tixati (`tixati`);
* rtorrent (`rtorrent`);
* aria2 (`aria2c`) — больше как универсальный загрузчик, но часто используется для торрентов;
* BitTorrent/uTorrent (под Linux реже, но если вдруг — можно пометить как `torrent_client`).

### Признаки

* `exe`: `qbittorrent`, `transmission-*`, `deluge`, `deluged`, `tixati`, `rtorrent`, `aria2c`, `vuze`, `azureus`…
* `.desktop`: `org.qbittorrent.qBittorrent.desktop`, `transmission-gtk.desktop` и т.д.
* systemd-units:
  * `transmission-daemon.service`, `deluged.service` — как `torrent_daemon`.

---

## 5. Музыкальные плееры

Сводка из подборок Linux-плееров 2023–2025:

* Rhythmbox (`rhythmbox`);
* Elisa (`elisa`);
* Lollypop (`lollypop`);
* Amberol (`amberol`);
* Tauon Music Box (`tauonmb`, иногда `tauon-music-box`);
* Strawberry (`strawberry`);
* Clementine (`clementine`);
* Amarok (`amarok`);
* Audacious (`audacious`);
* DeaDBeeF (`deadbeef`);
* cmus (`cmus`) — TUI;
* mpd + клиенты (`mpd`, `ncmpcpp`, `cantata` и т.п.).

### Признаки

* `exe`: имена плееров;
* `.desktop`: `org.gnome.Rhythmbox3.desktop`, `org.kde.elisa.desktop`, `org.kde.amarok.desktop`, `org.atheme.audacious.desktop`, `strawberry.desktop`, `tauonmb.desktop`…
* `audio_client` тег:
  * cross-проверка с PipeWire/PulseAudio по активным потокам.

---

## 6. Видеоплееры / медиа-центры

По обзорам медиаплееров для Linux и общим рейтингам:

* VLC (`vlc`) — де-факто стандарт;
* mpv (`mpv`);
* SMPlayer (`smplayer`);
* MPlayer (`mplayer`);
* Celluloid (`celluloid`);
* Haruna (`haruna`);
* Kodi/XBMC (`kodi`, `xbmc`);
* QMPlay2 (`qmplay2`);
* Parole (`parole` — Xfce);
* Dragon Player (`dragon`), Kaffeine (`kaffeine`) и др.

### Признаки

* `exe`: `vlc`, `mpv`, `smplayer`, `mplayer`, `celluloid`, `haruna`, `kodi`, `qmplay2`, `parole`, …
* `.desktop`: `vlc.desktop`, `io.github.celluloid_player.Celluloid.desktop`, `org.kde.haruna.desktop`, `org.xfce.Parole.desktop`, `org.kde.dragonplayer.desktop` и т.д.

---

## 7. Индексаторы / десктоп-поиск

По докам KDE/GNOME и обсуждениям:

### KDE Baloo:
* `baloo_file`, `baloo_file_extractor`, `baloo_filemetadata_temp_extractor`, `baloo_fileindexer`;
* Plasma использует Baloo как основной индексатор.

### GNOME Tracker / TinySPARQL:
* демоны `tracker-miner-fs-*`, `tracker3`, `tracker-miner-rss` и т.п.;
* в новых версиях проект переименован в TinySPARQL как фреймворк, но бинарники tracker-* всё ещё используются.

### Recoll:
* `recollindex`, `recoll`, свой индексирующий демон.

### Признаки

* `exe` с `baloo*`, `tracker*`, `recoll*`;
* `systemd`-units:
  * `baloo_file.service`, `tracker-miner-fs-*.service` и др.;
* относим к категории `indexer` → всегда `BACKGROUND`, агрессивно душим при `bad_responsiveness`.

---

## 8. Обновлялки / пакетные менеджеры

По сравнениям менеджеров пакетов и общим обзорам:

### Классические пакетники:

#### APT/DPKG:
* `apt`, `apt-get`, `unattended-upgrades`, `update-manager`, `synaptic`, `dpkg`;

#### DNF/YUM:
* `dnf`, `yum`, `dnf-automatic`, `dnfdragora`;

#### Pacman:
* `pacman`, AUR-helpers (`yay`, `paru`, `pamac` и т.п.);

#### Zypper:
* `zypper`, YaST Software Manager (`yast2`).

### Универсальные форматы:

#### Snap:
* `snapd`, `snap`, `snap-store`;

#### Flatpak:
* `flatpak`, `flatpak-system-helper`, `gnome-software` (как фронтенд).

#### PackageKit:
* `packagekitd`, `pkcon`, различные GUI-фронтенды.

### Признаки

* `exe`: перечисленные имена;
* `systemd`-units:
  * `packagekit.service`, `apt-daily.service`, `apt-daily-upgrade.service`, `dnf-makecache.service`, `snapd.service`, `flatpak-system-helper.service` и т.д.
* Категория `maintenance` с жёстким правилом:
  * при `user_active=true` → максимум `BACKGROUND/IDLE`.

---

## 9. Билд-системы и сборочные инструменты

По обзорам build-систем:

### C/C++-ориентированные:
* `make`, `cmake`, `ninja`, `meson`, `scons`, `bazel`, `qmake`, `waf`, `bear`.

### Языковые экосистемы:

* Rust: `cargo`;
* Go: `go` (`go build`, `go test` как подкоманды);
* Node.js: `npm`, `yarn`, `pnpm`, `turbo`, `nx`;
* Python: `pip`, `pipenv`, `poetry`, `tox`;
* Java/Android: `mvn` (Maven), `gradle`, `gradlew`;
* .NET: `dotnet build`, `dotnet test`.

### Признаки

* `exe` и первая часть `cmdline`;
* контекст:
  * если родитель — IDE/терминал, и процесс долго грузит CPU/IO → тип `build_tool`, ребёнок `ide`/`cli_interactive`.
* Политика:
  * короткие сборки → можно оставить в `INTERACTIVE`;
  * долгие тяжелые build'ы при `bad_responsiveness` → переводить группу/процесс в `batch_heavy`.

---

## 10. Скелет YAML-паттерна для SmoothTask

Один из вариантов структуры (подходит под всё выше):

```yaml
# configs/patterns/browsers.yml
category: browser
apps:
  - id: google-chrome
    label: "Google Chrome"
    match:
      exe:
        - "google-chrome"
        - "google-chrome-stable"
        - "chrome"
      desktop_id:
        - "google-chrome.desktop"
      cgroup_contains:
        - "chrome"
    tags: ["browser", "chromium-family"]

  - id: firefox
    label: "Mozilla Firefox"
    match:
      exe:
        - "firefox"
        - "firefox-esr"
      desktop_id:
        - "firefox.desktop"
    tags: ["browser", "firefox-family"]

  - id: brave
    label: "Brave Browser"
    match:
      exe: ["brave", "brave-browser"]
      desktop_id: ["brave-browser.desktop"]
    tags: ["browser", "chromium-family"]
```

И аналогично:

* `patterns/ide.yml` — `code`, `idea64`, `pycharm`, `clion`, `kate`, `nvim`, `emacs`…
* `patterns/terminals.yml` — `gnome-terminal`, `konsole`, `tilix`, `wezterm`, `alacritty`…
* `patterns/torrent.yml` — `qbittorrent`, `transmission-*`, `deluge`, `tixati`, `rtorrent`…
* `patterns/media_audio.yml` / `media_video.yml` — плееры;
* `patterns/indexers.yml` — `baloo*`, `tracker*`, `recoll*`;
* `patterns/maintenance.yml` — `apt*`, `dnf*`, `pacman`, `zypper`, `snapd`, `flatpak*`, `packagekitd`…
* `patterns/build_tools.yml` — `make`, `ninja`, `cmake`, `cargo`, `npm`, `mvn`, `gradle`…

---

## Краткий вывод

* Для **паттерн-базы SmoothTask** разумно начать именно с этих категорий — они покрывают почти все «узкие места» отзывчивости: браузерные вкладки, IDE+build, медиаплееры, торренты, индексаторы и обновлялки.
* Практическая стратегия:
  1. Взять топ-приложения из списка выше;
  2. Для каждого — добавить минимум `exe` + `.desktop` + (по возможности) `systemd/cgroup`-подписи;
  3. Остальным процессам давать теги и типы через **эвристику** (GUI/CLI/daemon, TTY, Audio, heavy CPU/IO).

---

## Следующие шаги

1. Детализировать паттерны для каждой категории с реальными именами бинарников и desktop-файлов
2. Добавить поддержку Snap/Flatpak AppIds
3. Расширить список systemd-units для индексаторов и обновлялок
4. Создать правила приоритизации для каждой категории

