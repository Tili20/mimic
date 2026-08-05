# Mimic — менеджер приложений по умолчанию (GTK4 + GIO)

Приложение для управления ассоциациями «MIME-тип → приложение» в Linux,
ориентированное на пользователей тайлинговых оконных менеджеров
(клавиатурная навигация, минимализм).

## Контекст: как это работает под капотом

Linux хранит ассоциации «MIME-тип → приложение» в файле `~/.config/mimeapps.list`
(плюс системные дефолты из `/usr/share/applications/`). Этот файл используют
`xdg-open` и все окружения рабочего стола.

GIO — слой из gtk-стека (GLib) — является официальной реализацией xdg-спецификации:

- читает зарегистрированные MIME-типы;
- находит приложения по `.desktop`-файлам;
- записывает дефолты в `mimeapps.list` (атомарно, без ручного парсинга INI).

Мы используем GIO — меньше кода, меньше ошибок, официальное поведение.

## Стек

- **Язык**: Rust (edition 2024)
- **UI**: GTK4 (crate `gtk4` 0.11, feature `v4_22`)
- **Данные**: GIO (`gio` 0.22), `glib` 0.22

## Структура проекта

```
src/
├── main.rs      — точка входа: создание gtk::Application, подключение активации
├── model.rs     — модель данных: свой glib-объект MimeType
├── backend.rs   — вся работа с GIO (данные отделены от UI)
└── ui.rs        — сборка окна, списка, фильтров, диалога выбора
```

`Cargo.toml`: добавить `gio = "0.22"` и `glib = "0.22"`
(версии уже лежат в Cargo.lock — gtk4 на них завязан, конфликтов не будет).

## Шаги

### 1. Backend (`backend.rs`) — слой данных

- `load_mime_types() -> Vec<MimeType>`:
  - `gio::functions::content_types_get_registered()` — все зарегистрированные типы;
  - для каждого: человекочитаемое описание `content_type_get_description()`;
  - текущий дефолт `AppInfo::default_for_type(mime, false)`.
- `apps_for(mime) -> Vec<gio::AppInfo>`: `AppInfo::all_for_type(mime)`.
- `set_default(mime, &app_info) -> Result<(), glib::Error>`:
  `app_info.set_as_default_for_type(mime)` — пишет в `mimeapps.list`.

### 2. Модель (`model.rs`) — собственный glib-объект

- `MimeType` — подкласс `glib::Object` со свойствами:
  - `mime: String`
  - `description: String`
  - `category: String` (по префиксу: `text/`, `image/`, …)
  - `default_app: Option<GString>`
- Реализация: `#[derive(glib::Properties)]` + `#[glib::object_subclass]` +
  `glib::wrapper!`.

Ключевой приём: виджеты GTK биндятся на свойства объекта — обновляешь модель,
UI перерисовывается сам.

### 3. UI (`ui.rs`) — окно

```
ApplicationWindow
├── HeaderBar (заголовок "Mimic")
└── Box (вертикальный)
    ├── SearchEntry  — поиск по типу/описанию
    ├── DropDown     — фильтр по категории (All / text / image / audio / video / application / inode / other)
    └── ScrolledWindow
        └── ListView (SingleSelection)
            └── строка: описание (жирным), mime-тип (моноширинный),
                текущий дефолт, кнопка "…"
```

- **ListView + SignalListItemFactory** — современный способ GTK4
  (вместо устаревшего TreeView): циклы `setup` / `bind` / `unbind`.
- **Фильтрация**: подкласс `gtk::Filter` (текст поиска + категория)
  + `gtk::FilterListModel`. Изменение фильтра → `filter.changed(...)`
  → список перефильтровывается.
- **Диалог выбора приложения**: `gtk::Popover` за строкой — список приложений
  с иконками (`Image::from_gicon`), кнопка "Set Default". После установки —
  обновляем свойство `default_app` модели, биндинг перерисует строку.
- **Хоткеи** (важно для tiling WM): `Ctrl+F` → фокус в поиск, `Ctrl+Q` → выход.

### 4. Полировка

- Сортировка списка (категория, затем описание).
- Отсев «мусорных» типов (`x-scheme-handler/*`, `application/octet-stream`).
- `cargo build`, `cargo clippy`, ручной тест на системе.

## Обучающие темы (разбираем параллельно)

- Модули Rust, видимость (`pub`, `use`), `Result` / `Option`.
- Владение и замыкания в GTK: `clone`, `'static`, время жизни сигналов.
- Подклассы glib: `object_subclass`, свойства, `wrapper!`.
- Модель-представление GTK: GListModel, ListView, FilterListModel, биндинг свойств.
- GIO: AppInfo, mimeapps.list, .desktop файлы.

## Файлы

| Файл | Назначение |
|---|---|
| `plan.md` | этот план |
| `TODO.md` | задачи с чекбоксами, текущий прогресс |
| `src/main.rs` | точка входа |
| `src/model.rs` | модель MimeType |
| `src/backend.rs` | работа с GIO |
| `src/ui.rs` | интерфейс |
