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
- [x] Кастомный `gtk::Filter` + `FilterListModel` — фильтрация по тексту и категории
- [x] Отдельное окно выбора приложения (450×440, transient, Esc закрывает)
- [x] Кнопка "Set Default" — установка дефолта и обновление модели
- [x] Юнит-тесты логики фильтра
- [x] Хоткеи: `Ctrl+H` (фокус в поиск), `Esc` (очистка поиска)
- [x] Печать переносит фокус в поиск (флаг фокуса через `EventControllerFocus`)
- [x] Две вкладки: «Общее» (группы приложений) и «Расширенное» (все типы)
- [x] MenuButton в хеадере: галочка «Скрыть редкоиспользуемые» (по умолчанию включена)
- [x] Отсев мусорных типов: `x-scheme-handler/*`, `application/x-*`, `inode/*`, `application/octet-stream`

## Полировка

- [ ] Сортировка списка по категории и описанию
- [x] `cargo build` и `cargo clippy` без предупреждений
- [x] Ручное тестирование на системе
