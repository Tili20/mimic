# TODO — Mimic

Прогресс проекта. Статусы: `[ ]` — не начато, `[x]` — готово.

## Подготовка

- [x] Добавить зависимости `gio` и `glib` в Cargo.toml
- [x] Создать структуру модулей (`main.rs`, `backend.rs`, `model.rs`, `ui.rs`)

## Backend (слой данных)

- [x] `load_mime_types()` — получить все зарегистрированные MIME-типы
- [x] Описание типа через `content_type_get_description()`
- [x] Текущий дефолт через `AppInfo::default_for_type()`
- [x] `apps_for(mime)` — список приложений через `AppInfo::all_for_type()`
- [x] `set_default(mime, app)` — установка дефолта через `set_as_default_for_type()`

## Модель

- [x] `MimeType` — подкласс `glib::Object` с полями mime/description/category/default_app
- [x] `#[derive(glib::Properties)]` + `#[glib::object_subclass]` + `glib::wrapper!`

## UI

- [x] Главное окно: `ApplicationWindow` + `HeaderBar`
- [x] `SearchEntry` — поиск по типу/описанию
- [x] `DropDown` — фильтр по категориям
- [x] `ListView` + `SignalListItemFactory` — список MIME-типов
- [ ] Кастомный `gtk::Filter` + `FilterListModel` — фильтрация по тексту и категории
- [ ] `Popover` — выбор приложения из списка с иконками
- [ ] Кнопка "Set Default" — установка дефолта и обновление модели
- [ ] Хоткеи: `Ctrl+F` (поиск), `Ctrl+Q` (выход)

## Полировка

- [ ] Сортировка списка по категории и описанию
- [ ] Отсев мусорных типов (`x-scheme-handler/*`, `application/octet-stream`)
- [ ] `cargo build` и `cargo clippy` без предупреждений
- [ ] Ручное тестирование на системе
