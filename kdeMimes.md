# KDE Plasma: Component Chooser (Default Applications) — MIME-типы и логика определения дефолта

> **Источник реализации:** `plasma-workspace/kcms/componentchooser/` (KCM `kcm_componentchooser`). Репозиторий `systemsettings` — только оболочка для поиска и встраивания KCM'ов.

## 1. Общая информация

- **KCM:** `kcm_componentchooser` (QML KCM, `KQuickManagedConfigModule`)
- **Расположение кода:** `plasma-workspace/kcms/componentchooser/`
- **Базовые классы:**
  - `ComponentChooser` (`componentchooser.h/cpp`) — логика загрузки/сохранения для каждого типа компонентов
  - `ApplicationModel` (`applicationmodel.h/cpp`) — модель списка кандидатов (через `KApplicationTrader::query()`), с пунктом «Other…»
- **UI:** `ui/main.qml`, `ui/ComponentComboBox.qml`, `ui/ComponentOverlay.qml`

---

## 2. Какие типы указаны для каждой подкатегории

Каждый компонент — подкласс `ComponentChooser` в `components/`. Указывается:
- **MIME-типы / URI-схемы** (`m_mimeTypes`)
- **Категория `.desktop`-приложений** для фильтрации (`applicationCategory`, используется `KApplicationTrader`)
- **KDE-дефолт** (`defaultApplication`)

### 2.1 Веб-браузер (Web Browser)
**Файлы:** `components/componentchooserbrowser.h`, `components/componentchooserbrowser.cpp`

| Параметр | Значение |
|---|---|
| **MIME/Схемы** | `x-scheme-handler/http`, `x-scheme-handler/https` |
| **Категория** | `KService::WebBrowser` (desktop-category `WebBrowser`) |
| **KDE-дефолт** | `org.kde.falkon.desktop` |
| **Особенность сохранения** | Переопределён `save()` — записывает **обе** схемы (`http` и `https`) через `KApplicationTrader::setPreferredService()` |

---

### 2.2 Электронная почта (Email Client)
**Файлы:** `components/componentchooseremail.h`, `components/componentchooseremail.cpp`

| Параметр | Значение |
|---|---|
| **MIME/Схемы** | `x-scheme-handler/mailto` |
| **Категория** | `KService::Email` (desktop-category `Email`) |
| **KDE-дефолт** | `org.kde.kmail2.desktop` |
| **Особенность сохранения** | Переопределён `save()` — помимо установки `mailto` через `KApplicationTrader`, **дополнительно** записывает `KEMailSettings` (`ClientProgram`/`ClientTerminal`) в `~/.config/emaildefaults` (для совместимости с KDE-инфраструктурой почты) |

---

### 2.3 Файловый менеджер (File Manager)
**Файлы:** `components/componentchooserfilemanager.h`, `components/componentchooserfilemanager.cpp`

| Параметр | Значение |
|---|---|
| **MIME** | `inode/directory` |
| **Категория** | `KService::FileManager` (desktop-category `FileManager`) |
| **KDE-дефолт** | `org.kde.dolphin.desktop` |
| **Сохранение** | Использует базовый `ComponentChooser::saveMimeTypeAssociations()` |

---

### 2.4 Терминальный эмулятор (Terminal Emulator)
**Файлы:** `components/componentchooserterminal.h`, `components/componentchooserterminal.cpp`, `terminal_settings.kcfg`, `terminal_settings.kcfgc`

| Параметр | Значение |
|---|---|
| **MIME/Схемы** | **Отсутствуют.** Работает **только по desktop-категории** приложений. |
| **Категория** | `KService::TerminalEmulator` (desktop-category `TerminalEmulator`) |
| **KDE-дефолт** | `org.kde.konsole.desktop` |
| **Хранение** | **Не в `mimeapps.list`**. KConfigXT: `kdeglobals` → `[General]` `TerminalApplication` и `TerminalService`. |
| **load()/save()** | **Полностью переопределены** (читают/пишут `TerminalSettings`). |

---

### 2.5 Текстовый редактор (Text Editor)
**Файлы:** `components/componentchoosertexteditor.h`, `components/componentchoosertexteditor.cpp`

| Параметр | Значение |
|---|---|
| **MIME-типы** | `text/plain`, `text/x-cmake`, `text/markdown`, `application/x-docbook+xml`, `application/json`, `application/x-yaml` |
| **Категория** | `KService::TextEditor` (desktop-category `TextEditor`) |
| **KDE-дефолт** | `org.kde.kate.desktop` |
| **Сохранение** | Базовый механизм — сохраняет **все** перечисленные MIME-типы |

---

### 2.6 Просмотр изображений (Image Viewer)
**Файлы:** `components/componentchooserimageviewer.h`, `components/componentchooserimageviewer.cpp`

| Параметр | Значение |
|---|---|
| **MIME-типы** | `image/png`, `image/jpeg`, `image/webp`, `image/avif`, `image/tiff`, `image/bmp`, `image/x-psd`, `image/x-eps`, `image/x-xcf`, `image/svg+xml`, `image/x-icb`, `image/x-portable-anymap`, `image/x-portable-bitmap`, `image/x-portable-graymap`, `image/x-portable-pixmap`, `image/x-xbitmap`, `image/x-xpixmap` |
| **Категория** | `KService::Viewer` (desktop-category `Viewer`) |
| **KDE-дефолт** | `org.kde.gwenview.desktop` |

---

### 2.7 Музыкальный плеер (Music Player)
**Файлы:** `components/componentchoosermusicplayer.h`, `components/componentchoosermusicplayer.cpp`

| Параметр | Значение |
|---|---|
| **MIME-типы** | `audio/mpeg`, `audio/x-mpegurl`, `audio/aac`, `audio/mp4`, `audio/x-ms-wma`, `audio/x-ms-asx`, `audio/x-ogg`, `audio/ogg`, `audio/x-vorbis+ogg`, `audio/wav`, `audio/x-wav`, `audio/flac`, `audio/x-flac`, `audio/x-opus+ogg` |
| **Категория** | `KService::Player` (desktop-category `Player`) |
| **KDE-дефолт** | `org.kde.elisa.desktop` |

---

### 2.8 Видеоплеер (Video Player)
**Файлы:** `components/componentchooservideoplayer.h`, `components/componentchooservideoplayer.cpp`

| Параметр | Значение |
|---|---|
| **MIME-типы** | `video/mpeg`, `video/x-mpeg`, `video/mp4`, `video/x-m4v`, `video/quicktime`, `video/x-msvideo`, `video/x-ms-asf`, `video/x-ms-wmv`, `video/x-ms-wmx`, `video/avi`, `video/x-avi`, `video/x-ogm`, `video/x-ogm+ogg`, `video/ogg`, `video/x-theora`, `video/x-theora+ogg`, `video/webm`, `video/x-matroska`, `video/x-matroska-3d` |
| **Категория** | `KService::Video` (desktop-category `Video`) |
| **KDE-дефолт** | `org.kde.haruna.desktop` |

---

### 2.9 PDF-просмотрщик (PDF Viewer)
**Файлы:** `components/componentchooserpdfviewer.h`, `components/componentchooserpdfviewer.cpp`

| Параметр | Значение |
|---|---|
| **MIME** | `application/pdf` |
| **Категория** | `KService::Viewer` (desktop-category `Viewer`) |
| **KDE-дефолт** | `okularApplication_pdf.desktop` |

---

### 2.10 Архиватор (Archive Manager)
**Файлы:** `components/componentchooserarchivemanager.h`, `components/componentchooserarchivemanager.cpp`

| Параметр | Значение |
|---|---|
| **MIME-типы** | `application/zip`, `application/x-tar`, `application/x-compressed-tar`, `application/x-bzip-compressed-tar`, `application/x-lzma-compressed-tar`, `application/x-xz-compressed-tar`, `application/x-lzip-compressed-tar`, `application/x-cd-image`, `application/x-bcpio`, `application/x-cpio`, `application/x-cpio-compressed`, `application/vnd.ms-cab-compressed`, `application/x-7z-compressed`, `application/x-java-archive`, `application/rar`, `application/x-rar`, `application/x-iso9660-image` |
| **Категория** | `KService::Archiving` (desktop-category `Archiving`) |
| **KDE-дефолт** | `org.kde.ark.desktop` |

---

### 2.11 Карты (Geo URI)
**Файлы:** `components/componentchoosergeo.h`, `components/componentchoosergeo.cpp`

| Параметр | Значение |
|---|---|
| **MIME/Схема** | `x-scheme-handler/geo` |
| **Категория** | `KService::Application` (без жёсткой фильтрации только по Geo-категории) |
| **KDE-дефолт** | `marble_geo.desktop` |
| **Примечание** | Для схем проверка поддержки идёт через `KService::schemeHandlers()`, а не только `mimeTypes()` (см. `ComponentChooser::serviceSupportsMimeType()`) |

---

### 2.12 Телефония (Tel URI)
**Файлы:** `components/componentchoosertel.h`, `components/componentchoosertel.cpp`

| Параметр | Значение |
|---|---|
| **MIME/Схема** | `x-scheme-handler/tel` |
| **Категория** | `KService::Dialer` (desktop-category `Dialer`) |
| **KDE-дефолт** | `org.kde.kdeconnect.handler.desktop` |
| **Примечание** | Проверка поддержки схемы — через `KService::schemeHandlers()` |

---

## 3. Как именно реализована проверка, что используется по умолчанию

Определение «текущее приложение по умолчанию» происходит **в `ComponentChooser` и `ApplicationModel`**.

### 3.1 Базовый `ComponentChooser` (все компоненты, кроме терминала)
**Файлы:** `componentchooser.h`, `componentchooser.cpp`

#### Загрузка (определение текущего дефолта)
1. Берётся список `m_mimeTypes` (обычно **первый** — основной для данного компонента).
2. Текущее предпочитаемое приложение читается через:
   ```cpp
   m_preferredService = KApplicationTrader::preferredService(m_mimeTypes.first());
   ```
   `KApplicationTrader::preferredService()` резолвит текущее значение по цепочке XDG: `~/.config/mimeapps.list` (`[Default Applications]`) → системные defaults и т.д. (KF6-обёртка над XDG).
3. `ApplicationModel::load()` заполняет список кандидатов, **первым (или в списке) добавляет `m_preferredService`** и помечает его `isSelected = true` (даже если он не проходит фильтры — защита от случая, когда текущий дефолт не найден по фильтру).
4. После загрузки модели: `m_index = m_applicationsModel->findSelected()` (поиск по `isSelected`). Это значение экспортируется в QML как `index` (текущий выбранный пункт в комбобоксе).

#### Сохранение
- `save()` вызывает `saveMimeTypeAssociations()`, который **для всех** MIME из `m_mimeTypes` делает:
  ```cpp
  KApplicationTrader::setPreferredService(mimeType, selectedService);
  ```
  Запись идёт в `~/.config/mimeapps.list`, секция `[Default Applications]`.
- Браузер и почта **переопределяют** `save()` (браузер пишет обе схемы, почта — дополнительно пишет `KEMailSettings`).

#### isDefaults() и isSaveNeeded()
- **`isDefaults()`** — сравнивает текущий `m_index` с `m_applicationsModel->defaultIndex()` (индекс KDE-дефолта, определённый моделью по `defaultAppStorageId`).
- **`isSaveNeeded()`** — сравнивает **текущий выбранный `storageId`** (из модели по текущему `index`) с тем, что вернёт `KApplicationTrader::preferredService(m_mimeTypes.first())->storageId()` **прямо в момент проверки**. Т.е. `isSaveNeeded()` отвечает на вопрос: «Отличается ли текущее выбранное значение от реально записанного в mimeapps.list?»

### 3.2 ApplicationModel — как строится список и определяется выбранный
**Файлы:** `applicationmodel.h`, `applicationmodel.cpp`

`load(preferredService, mimeTypes, applicationCategory, defaultAppStorageId)` делает следующее:

1. **Добавляет текущий предпочитаемый сервис (`preferredService`)** — помечается `isSelected = true`. Добавляется даже с учётом возможных расхождений с фильтрами (защита от некорректно записанного дефолта).
2. **Запрашивает кандидатов** через `KApplicationTrader::query()`:
   - Фильтр по категории: `KApplicationTrader::query(applicationCategory, ...)` (WebBrowser, Email, FileManager, TerminalEmulator, TextEditor, Viewer, Player, Video, Archiving, Dialer, Application)
   - Доп. фильтр по поддержке MIME/схем: `serviceSupportsMimeType(service, mimeTypes)` (реализован в `ComponentChooser`):
     - **Обычные MIME:** `service.mimeTypes().contains(mime)` (с учётом наследования MIME через `QMimeDatabase`)
     - **URI-схемы `x-scheme-handler/*`:** `service.schemeHandlers().contains(schemeName)` (`http`, `https`, `mailto`, `geo`, `tel`)
3. У каждого кандидата `storageId = service->storageId()` (`*.desktop` файл).
4. **Определяется KDE-дефолт:** ищется элемент с `storageId == defaultAppStorageId` → запоминается `defaultIndex()` (используется в `isDefaults()`).
5. **В конец списка всегда добавляется «Other…»** (специальная строка модели). При выборе её UI открывает `KOpenWithDialog`.

**Методы определения текущего выбора:**
- `findSelected()` — ищет элемент с `isSelected == true` (роль `IsSelectedRole`).
- `setSelected(const KService::Ptr &service)` — помечает выбранный сервис.
- `index()` компонента = `m_applicationsModel->findSelected()`.

### 3.3 Терминальный эмулятор — особый случай
**`ComponentChooserTerminal`** (`components/componentchooserterminal.cpp`)

| Аспект | Реализация |
|---|---|
| **load()** | Создаёт `TerminalSettings` (KConfigXT). Читает `settings.terminalService()` (storageId `.desktop`). Формирует `preferredService` через `KService::serviceByStorageId()` (или передаёт storageId в модель). Затем `m_applicationsModel->load(...)` с этим preferred. |
| **save()** | Берёт выбранный `storageId` из модели, пишет `settings.setTerminalService(selectedStorageId)` и `settings.setTerminalApplication(selectedName)`, вызывает `settings.save()`. |
| **defaults()** | Выбирает `defaultIndex()` (дефолт `org.kde.konsole.desktop`). |
| **isDefaults()/isSaveNeeded()** | Сравнивают текущий выбор/индекс с тем, что возвращают `TerminalSettings` (текущее сохранённое значение в `kdeglobals`), и с KDE-дефолтом. |
| **Ключевое отличие** | Определение «используется по умолчанию» **исключительно по `kdeglobals`** `[General] TerminalApplication/TerminalService`, **а не по `mimeapps.list`**. |

### 3.4 Применение изменений после сохранения
После успешного `save()` всех изменившихся компонентов (в `kcm_componentchooser.cpp`):

1. **D-Bus:** вызывается `org.kde.klauncher5 /KLauncher reparseConfiguration()` — KLauncher перечитывает конфигурацию.
2. **Форсится пересбор `ksycoca`:** временно выставляется `ksycoca_ms_between_checks = 0`, вызывается `KService::allServices()` (через `KBuildSycocaProgressDialog::rebuildKSycoca()`), чтобы `KApplicationTrader` и запросы сервисов **сразу** увидели новые предпочтения.

---

## 4. Краткое резюме (куда что пишется)

| Компонент | Механизм хранения | Файл |
|---|---|---|
| **Большинство (http/https, mailto, inode/directory, text/*, image/*, audio/*, video/*, application/pdf/zip/tar..., geo, tel)** | `KApplicationTrader::setPreferredService(mimeType, service)` | `~/.config/mimeapps.list` → `[Default Applications]` |
| **Терминал (TerminalEmulator)** | KConfigXT `TerminalSettings` | `~/.config/kdeglobals` → `[General]` `TerminalApplication`, `TerminalService` |
| **Email (дополнительно)** | `KEMailSettings` | `~/.config/emaildefaults` → `[General]` `ClientProgram`, `ClientTerminal` |

**Проверка «текущий дефолт»:** в основном через `KApplicationTrader::preferredService(mimeType/схема)` (XDG), для терминала — через `TerminalSettings` (KConfig).

---

## 5. Механизм «Other…» (выбор стороннего приложения)

Позволяет пользователю выбрать приложение, которого нет в отфильтрованном списке модели (или ввести произвольную команду). Реализация состоит из трёх частей: sentinel-строка в модели, обработчик в `ComponentChooser` и стандартный диалог `KOpenWithDialog` из KF6 KIOWidgets.

### 5.1 Sentinel-строка в модели

`ApplicationModel::load()` **всегда** добавляет специальную последнюю строку (`applicationmodel.cpp:64`):

```cpp
addApplication(i18n("Other…"), QStringLiteral("application-x-shellscript"),
               /*storageId*/ QString(), /*isSelected*/ false, /*execLine*/ QString());
```

- `storageId` и `execLine` **пустые** — это маркер «не настоящее приложение».
- Всегда **последней строкой** (функция пушит в конец списка).
- В QML специальной обработки нет: `ui/ComponentComboBox.qml:24` при выборе любого пункта вызывает `component.select(currentIndex)`.

### 5.2 Обработчик выбора — `ComponentChooser::select()` (`componentchooser.cpp:64`)

```cpp
void ComponentChooser::select(int index)
{
    // Other selection
    if (index == m_model->rowCount() - 1) {   // последняя строка == "Other…"
        auto *dialog = new KOpenWithDialog(QList<QUrl>(), m_mimeType, m_dialogText,
                                           QString(), QApplication::activeWindow());
        dialog->setSaveNewApplications(true);
        dialog->setAttribute(Qt::WA_DeleteOnClose);
        connect(dialog, &KOpenWithDialog::finished, this, [this, dialog](int result) {
            if (result == QDialog::Rejected) {
                Q_EMIT indexChanged();       // combobox откатится через биндинг currentIndex: component.index
                Q_EMIT isDefaultsChanged();
                return;
            }
            const auto serviceStorageId = dialog->service()->storageId();
            QModelIndex modelIndex = m_model->findByStorageId(serviceStorageId);
            if (modelIndex.isValid()) {      // дедупликация: уже есть в списке — просто выделить
                select(modelIndex.row());
                return;
            }
            auto newIndex = m_model->addApplicationBeforeLast(dialog->service());
            select(newIndex);                // вставить ПЕРЕД "Other…" и выделить
        });
        dialog->open();                      // асинхронно, не exec()
    } else {
        m_index = index;                     // обычный выбор
        m_model->setData(m_model->index(index, 0), true, ApplicationModel::Selected);
        Q_EMIT indexChanged();
        Q_EMIT isDefaultsChanged();
        Q_EMIT modelChanged();
    }
}
```

Ключевые моменты:

- Аргументы диалога: `urls={}`, **`mimeType = m_mimeType` (основной/первый MIME компонента)** — `x-scheme-handler/http` у браузера, `x-scheme-handler/mailto` у почты, `text/plain` у редактора, **пустая строка у терминала**; `text = m_dialogText` («Select default browser» и т.п.).
- `setSaveNewApplications(true)` — разрешает создать новый `.desktop` для введённой вручную команды (см. 5.4).
- Диалог **асинхронный** (`open()`), объект уничтожается сам (`WA_DeleteOnClose`).
- **Дедупликация:** выбранное приложение уже в модели → новая строка не создаётся.
- **`ApplicationModel::addApplicationBeforeLast()`** (`applicationmodel.cpp:95`): вставляет новое приложение **перед** строкой «Other…» (swap последних двух элементов), чтобы «Other…» оставался последним.
- **Отмена:** в C++ `m_index` не менялся, комбобокс откатывается сам через биндинг `currentIndex: component.index`.
- **Запись на диск не происходит сразу** — приложение только попадает в модель; сохранение — по Apply (см. 5.5).

### 5.3 Что делает `KOpenWithDialog` (KF6 KIOWidgets, `kio/src/widgets/kopenwithdialog.cpp`)

- **Строка ввода команды:** `KHistoryComboBox` с историей (state-конфиг, группа `Open-with settings`) и автодополнением путей к бинарникам (`KUrlCompletion::ExeCompletion`).
- **Дерево «Known Applications»:** `KApplicationModel` строится из **всего** application menu (`KServiceGroup::group("")`) — **все установленные приложения, без фильтрации** по категории/MIME. Живой поиск: рекурсивный, case-insensitive, авто-разворот узлов и автовыбор единственного совпадения при ≥3 символах.
- Чекбокс **«Remember application association for all files of type …»** — появляется только если передан `mimeType` (для терминала — нет).
- Сворачиваемая группа **«Terminal options»** (Run in terminal / Do not close when command exits).
- Кнопка **«Get more Apps from Discover»** (если есть MIME и установлен `org.kde.discover`).

### 5.4 Что происходит при OK — `KIO::OpenWith::accept()` (`kio/src/core/openwith.cpp`)

Сигнатура: `accept(KService::Ptr &service, typedExec, remember, mimeType, openInTerminal, lingerTerminal, saveNewApps)` — `service` **по ссылке**, поэтому после OK `dialog->service()` возвращает валидный сервис даже для введённой вручную команды (отсюда `dialog->service()->storageId()` в колбэке не падает).

| Ситуация | Результат |
|---|---|
| Выбрано приложение из дерева, `remember` включён | Немедленно пишет ассоциацию в `mimeapps.list` (`[Default Applications]` + `[Added Associations]`, выбранный — первый в списке) и выключает auto-embed (`filetypesrc`, `embed-<mime>=false`). |
| Введена команда, **без** `remember` и **без** `saveNewApps` | Временный in-memory `KService` (на диск не сохраняется). В нашем случае `saveNewApps=true`, эта ветка не используется. |
| Введена команда + `saveNewApps=true` (случай componentchooser) | Проверка существования бинарника (`QStandardPaths::findExecutable`, иначе ошибка). Создаётся **реальный `.desktop`**: `KService::newServicePath()` → `~/.local/share/applications/<name>.desktop` с `Type=Application`, `Name` (имя бинарника + аргументы), `Exec`, **`NoDisplay=true`** (не светится в меню), `MimeType=<mime>` (если MIME передан), и для `x-scheme-handler/*` к `Exec` **автоматически дописывается `%u`**. Затем `addToMimeAppsList(menuId, mimeType)` и пересбор ksycoca. |

### 5.5 Нюансы сохранения

- Выбор из «Other…» — это просто ещё один `storageId` в модели. Дальше обычный путь: Apply → `ComponentChooser::save()` → `saveMimeTypeAssociations(storageId, mimeTypes())` → `KApplicationTrader::setPreferredService()` для каждого MIME компонента (**с пропуском** MIME, которые приложение не поддерживает — `serviceSupportsMimeType`, `componentchooser.cpp:116`).
- **Квирк:** диалог при создании нового `.desktop` (`saveNewApps`) вызывает `addToMimeAppsList()` **безусловно**, если `mimeType` не пуст — т.е. ассоциация может записаться **раньше Apply**. Галка «Remember…» для существующего приложения — тоже немедленная запись.
- Для браузера диалог получает только `http` (первый MIME); `https` допишется только при Apply (`ComponentChooserBrowser::save()` пишет обе схемы).
- Для терминала `m_mimeType` пуст → ни чекбокса, ни записи в mimeapps; новый `.desktop` создастся без `MimeType`, а выбор сохранится в `kdeglobals` через `TerminalSettings` по Apply.

---

## 6. Аналог «Other…» в GTK4/GIO (для mimic)

`GtkAppChooserDialog`/`GtkAppChooserWidget` в GTK4 **deprecated с 4.10** — на них не рассчитывать. Рабочий вариант:

1. Sentinel-строка «Other…» в конце списка (как в KDE): `storageId == ""` как маркер.
2. При выборе — ввод команды (поле) или выбор бинарника через `FileDialog`.
3. Создать `~/.local/share/applications/mimic-<name>.desktop` вручную:
   - `Type=Application`, `Name`, `Exec=<команда>` (**дописать `%u`/`%U` для scheme-handler`ов`**, `NoDisplay=true`);
   - `MimeType=` — через запятую, если MIME известен.
4. Создать `gio::DesktopAppInfo` из пути к файлу → `set_as_default_for_type()` / `set_as_default_for_scheme()` — GIO сам запишет `~/.config/mimeapps.list`.
5. Новый сервис добавить в модель **перед** sentinel-строкой и выделить; при отмене — просто откатить выделение (в C++/GTK: не менять активный индекс до подтверждения).

Альтернатива без создания `.desktop`: `gio::AppInfo::create_from_commandline()` — но даёт in-memory сервис **без storageId**, в `mimeapps.list` такую запись вписать нельзя, поэтому вариант с ручным `.desktop` надёжнее.