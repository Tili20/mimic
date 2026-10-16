// База данных MIME-типов через GIO.

use std::collections::BTreeSet;

use gio::AppInfo;
use gio::prelude::*;

use crate::model::MimeType;

pub fn load_mime_types() -> Vec<MimeType> {
    let registered: BTreeSet<_> = gio::functions::content_types_get_registered()
        .into_iter()
        .collect();
    let mut mime_types: Vec<_> = registered
        .into_iter()
        .map(|mime| {
            let description = gio::functions::content_type_get_description(&mime);
            let category = category_of(&mime);
            let default_app =
                AppInfo::default_for_type(&mime, false).map(|app| app.name().to_string());
            MimeType::new(&mime, &description, &category, default_app)
        })
        .collect();
    mime_types.sort_by_key(|mime_type| {
        (
            mime_type.category().to_string(),
            mime_type.description().to_lowercase(),
        )
    });
    mime_types
}

fn category_of(mime: &str) -> String {
    mime.split_once('/')
        .map(|(category, _)| category.to_string())
        .unwrap_or_else(|| "other".to_string())
}

pub fn apps_for(mime: &str) -> Vec<AppInfo> {
    let mut apps = AppInfo::all_for_type(mime)
        .into_iter()
        .filter(should_list)
        .collect::<Vec<_>>();
    apps.sort_by_key(|app| app.name());
    apps
}

pub fn is_rare(mime: &str) -> bool {
    mime.starts_with("x-scheme-handler/")
        || mime.starts_with("inode/")
        || mime.starts_with("application/x-")
        || mime == "application/octet-stream"
}

pub struct Group {
    pub name: &'static str,
    /// Подзаголовок-секция на вкладке General (группы с одинаковой
    /// секцией идут подряд и объединяются общим заголовком).
    pub section: &'static str,
    /// Типы, на которые применяется выбор приложения при Set Default.
    pub mimes: Vec<String>,
    /// «Представительные» типы: из них строится список приложений
    /// и определяется текущий дефолт группы.
    pub app_mimes: Vec<String>,
}

pub fn groups() -> Vec<Group> {
    let registered = gio::functions::content_types_get_registered();
    let with_prefix = |prefix: &str| -> Vec<String> {
        registered
            .iter()
            .filter(|mime| mime.starts_with(prefix))
            .map(|mime| mime.to_string())
            .collect()
    };

    let explicit = |mimes: &[&str]| mimes.iter().map(|m| m.to_string()).collect::<Vec<_>>();

    let terminal = vec![
        "x-scheme-handler/terminal".to_string(),
        "application/x-terminal-emulator".to_string(),
    ];

    let archives = explicit(&[
        "application/x-tar",
        "application/x-compressed-tar",
        "application/x-bzip-compressed-tar",
        "application/x-tarz",
        "application/x-xz-compressed-tar",
        "application/x-lzma-compressed-tar",
        "application/x-lzip-compressed-tar",
        "application/x-tzo",
        "application/x-lrzip-compressed-tar",
        "application/x-lz4-compressed-tar",
        "application/x-zstd-compressed-tar",
        "application/x-cd-image",
        "application/x-bcpio",
        "application/x-cpio",
        "application/x-cpio-compressed",
        "application/x-sv4cpio",
        "application/x-sv4crc",
        "application/x-source-rpm",
        "application/vnd.ms-cab-compressed",
        "application/x-xar",
        "application/x-iso9660-appimage",
        "application/x-archive",
        "application/vnd.rar",
        "application/x-rar",
        "application/x-7z-compressed",
        "application/zip",
        "application/x-compress",
        "application/gzip",
        "application/x-bzip",
        "application/x-bzip2",
        "application/x-lzma",
        "application/x-xz",
        "application/zstd",
        "application/x-lha",
    ]);

    let mut text_mimes = with_prefix("text/");
    text_mimes.extend(explicit(&[
        "application/json",
        "application/xml",
        "application/javascript",
        "application/x-docbook+xml",
        "application/x-yaml",
    ]));

    let mut video_mimes = with_prefix("video/");
    video_mimes.push("application/x-matroska".to_string());

    let office = explicit(&[
        "application/vnd.oasis.opendocument.text",
        "application/vnd.oasis.opendocument.spreadsheet",
        "application/vnd.oasis.opendocument.presentation",
        "application/vnd.oasis.opendocument.graphics",
        "application/msword",
        "application/vnd.ms-excel",
        "application/vnd.ms-powerpoint",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "application/rtf",
    ]);

    vec![
        Group {
            name: "Web Browser",
            section: "Internet",
            mimes: explicit(&["x-scheme-handler/http", "x-scheme-handler/https"]),
            app_mimes: explicit(&["x-scheme-handler/http", "x-scheme-handler/https"]),
        },
        Group {
            name: "Calendar",
            section: "Internet",
            mimes: explicit(&["text/calendar"]),
            app_mimes: explicit(&["text/calendar"]),
        },
        Group {
            name: "Mail Client",
            section: "Internet",
            mimes: explicit(&["x-scheme-handler/mailto"]),
            app_mimes: explicit(&["x-scheme-handler/mailto"]),
        },
        Group {
            name: "Map",
            section: "Internet",
            mimes: explicit(&[
                "x-scheme-handler/map",
                "x-scheme-handler/geo",
                "x-scheme-handler/maps",
            ]),
            app_mimes: explicit(&["x-scheme-handler/map", "x-scheme-handler/geo"]),
        },
        Group {
            name: "Image Viewer",
            section: "Multimedia",
            mimes: with_prefix("image/"),
            app_mimes: explicit(&["image/jpeg", "image/png", "image/gif", "image/webp"]),
        },
        Group {
            name: "Audio Player",
            section: "Multimedia",
            mimes: with_prefix("audio/"),
            app_mimes: explicit(&["audio/mpeg", "audio/ogg", "audio/flac", "audio/mp4"]),
        },
        Group {
            name: "Video Player",
            section: "Multimedia",
            mimes: video_mimes,
            app_mimes: explicit(&[
                "video/mp4",
                "video/webm",
                "video/x-matroska",
                "application/x-matroska",
            ]),
        },
        Group {
            name: "Text Editor",
            section: "Documents",
            mimes: text_mimes,
            app_mimes: explicit(&["text/plain", "text/markdown", "application/json"]),
        },
        Group {
            name: "PDF Viewer",
            section: "Documents",
            mimes: explicit(&["application/pdf"]),
            app_mimes: explicit(&["application/pdf"]),
        },
        Group {
            name: "Office documents",
            section: "Documents",
            mimes: office,
            app_mimes: explicit(&[
                "application/vnd.oasis.opendocument.text",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            ]),
        },
        Group {
            name: "Terminal",
            section: "System",
            mimes: terminal.clone(),
            app_mimes: terminal,
        },
        Group {
            name: "File Manager",
            section: "System",
            mimes: explicit(&["inode/directory"]),
            app_mimes: explicit(&["inode/directory"]),
        },
        Group {
            name: "Archiver",
            section: "System",
            mimes: archives.clone(),
            app_mimes: archives,
        },
    ]
}

pub fn apps_for_group(mimes: &[String]) -> Vec<AppInfo> {
    let mut seen = std::collections::HashSet::new();
    let mut apps = Vec::new();
    for mime in mimes {
        for app in AppInfo::all_for_type(mime) {
            if !should_list(&app) {
                continue;
            }
            if let Some(id) = app.id()
                && !seen.insert(id.to_string())
            {
                continue;
            }
            apps.push(app);
        }
    }
    apps.sort_by_key(|app| app.name());
    apps
}

pub fn set_default_group(mimes: &[String], app: &AppInfo) -> Result<(), glib::Error> {
    for mime in mimes {
        app.set_as_default_for_type(mime)?;
    }
    Ok(())
}

/// Префикс desktop-файлов, которые создаёт «Other…».
const DESKTOP_PREFIX: &str = "mimic-";

/// NoDisplay-приложения, созданные mimic, должны оставаться видимы
/// в списках самого mimic (в приложениях меню они не светятся).
fn listable(id: Option<&str>, should_show: bool) -> bool {
    should_show || id.is_some_and(|id| id.starts_with(DESKTOP_PREFIX))
}

pub fn should_list(app: &AppInfo) -> bool {
    listable(app.id().as_deref(), app.should_show())
}

/// Отображаемое имя для созданного приложения: базовое имя бинарника + аргументы.
pub fn display_name(command: &str) -> String {
    let mut parts = command.split_whitespace();
    let program = parts.next().unwrap_or(command);
    let base = std::path::Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(program);
    let mut name = base.to_string();
    for part in parts {
        name.push(' ');
        name.push_str(part);
    }
    name.replace(['\n', '\r'], "")
}

/// Имя desktop-файла для команды: `mimic-<базовое имя>.desktop`.
pub fn desktop_file_name(command: &str) -> String {
    let program = command.split_whitespace().next().unwrap_or("app");
    let base = std::path::Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("app");
    let sanitized: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let sanitized = if sanitized.is_empty() {
        "app".to_string()
    } else {
        sanitized
    };
    format!("{DESKTOP_PREFIX}{sanitized}.desktop")
}

/// Первая программа команды: либо весь путь с пробелами, если он существует,
/// либо первый пробельный токен.
pub fn command_program(command: &str) -> &str {
    let trimmed = command.trim();
    if trimmed.contains(' ') && std::path::Path::new(trimmed).exists() {
        trimmed
    } else {
        trimmed.split_whitespace().next().unwrap_or(trimmed)
    }
}

pub fn program_exists(program: &str) -> bool {
    glib::find_program_in_path(program).is_some()
}

fn quote_program(command: &str) -> String {
    let trimmed = command.trim();
    if !trimmed.starts_with('"')
        && trimmed.contains(' ')
        && std::path::Path::new(trimmed).exists()
    {
        format!("\"{trimmed}\"")
    } else {
        trimmed.to_string()
    }
}

/// Для scheme-хендлеров в конец Exec дописывается `%u` (как в KIO),
/// если команда ещё не содержит плейсхолдеров.
fn format_exec(command: &str, primary_mime: &str) -> String {
    let mut exec = quote_program(command);
    if primary_mime.starts_with("x-scheme-handler/") && !command.contains('%') {
        exec.push_str(" %u");
    }
    exec
}

fn desktop_file_contents(name: &str, exec: &str, mimes: &[String]) -> String {
    let mut contents = String::from("[Desktop Entry]\nType=Application\n");
    contents.push_str(&format!("Name={}\n", name.replace(['\n', '\r'], "")));
    contents.push_str(&format!("Exec={exec}\n"));
    contents.push_str("NoDisplay=true\n");
    if !mimes.is_empty() {
        contents.push_str(&format!("MimeType={};\n", mimes.join(";")));
    }
    contents
}

fn exec_line(contents: &str) -> Option<&str> {
    contents
        .lines()
        .find_map(|line| line.strip_prefix("Exec="))
}

/// Дописывает недостающие MIME-типы в существующий desktop-файл.
/// Возвращает (новое содержимое, было ли изменение).
fn merge_mimetype_line(existing: &str, mimes: &[String]) -> (String, bool) {
    let mut lines: Vec<String> = existing.lines().map(str::to_string).collect();
    let position = lines.iter().position(|line| line.starts_with("MimeType="));
    let mut current: Vec<String> = match position {
        Some(index) => lines[index]["MimeType=".len()..]
            .split(';')
            .filter(|entry| !entry.is_empty())
            .map(str::to_string)
            .collect(),
        None => Vec::new(),
    };

    let mut changed = false;
    for mime in mimes {
        if !current.iter().any(|entry| entry == mime) {
            current.push(mime.clone());
            changed = true;
        }
    }
    if !changed {
        return (existing.to_string(), false);
    }

    let line = format!("MimeType={};", current.join(";"));
    match position {
        Some(index) => lines[index] = line,
        None => lines.push(line),
    }
    let mut merged = lines.join("\n");
    merged.push('\n');
    (merged, true)
}

fn desktop_names(dir: &std::path::Path) -> std::io::Result<std::collections::HashSet<String>> {
    let mut names = std::collections::HashSet::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str()
            && name.ends_with(".desktop")
        {
            names.insert(name.to_string());
        }
    }
    Ok(names)
}

/// Первое свободное имя вида `base`, `stem-2.desktop`, `stem-3.desktop`, …
fn unique_desktop_name(
    base: &str,
    taken: &std::collections::HashSet<String>,
) -> String {
    if !taken.contains(base) {
        return base.to_string();
    }
    let stem = base.strip_suffix(".desktop").unwrap_or(base);
    let mut number = 2;
    loop {
        let candidate = format!("{stem}-{number}.desktop");
        if !taken.contains(&candidate) {
            return candidate;
        }
        number += 1;
    }
}

/// Создаёт (или переиспользует) desktop-файл в `~/.local/share/applications`
/// и возвращает его id (= имя файла).
pub fn write_desktop_file(
    command: &str,
    mimes: &[String],
    primary_mime: &str,
) -> std::io::Result<String> {
    let name = desktop_file_name(command);
    let exec = format_exec(command, primary_mime);
    let contents = desktop_file_contents(&display_name(command), &exec, mimes);

    let dir = glib::user_data_dir().join("applications");
    std::fs::create_dir_all(&dir)?;

    let path = dir.join(&name);
    if path.exists() {
        let existing = std::fs::read_to_string(&path)?;
        if exec_line(&existing) == Some(exec.as_str()) {
            let (merged, changed) = merge_mimetype_line(&existing, mimes);
            if changed {
                std::fs::write(&path, merged)?;
            }
            return Ok(name);
        }
        let taken = desktop_names(&dir)?;
        let unique = unique_desktop_name(&name, &taken);
        std::fs::write(dir.join(&unique), contents)?;
        return Ok(unique);
    }

    std::fs::write(&path, contents)?;
    Ok(name)
}

pub fn find_app(id: &str) -> Option<AppInfo> {
    AppInfo::all()
        .into_iter()
        .find(|app| app.id().as_deref() == Some(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group<'a>(groups: &'a [Group], name: &str) -> &'a Group {
        groups
            .iter()
            .find(|group| group.name == name)
            .unwrap_or_else(|| panic!("group {name:?} exists"))
    }

    #[test]
    fn office_group_contains_core_types_and_no_pdf() {
        let groups = groups();
        let office = group(&groups, "Office documents");

        for mime in [
            "application/vnd.oasis.opendocument.text",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "application/rtf",
        ] {
            assert!(
                office.mimes.iter().any(|m| m == mime),
                "office group covers {mime}"
            );
        }
        assert!(!office.mimes.iter().any(|m| m == "application/pdf"));
    }

    #[test]
    fn office_group_representative_mimes_are_subset() {
        let groups = groups();
        let office = group(&groups, "Office documents");

        assert!(!office.app_mimes.is_empty());
        assert!(
            office
                .app_mimes
                .iter()
                .all(|mime| office.mimes.contains(mime)),
            "app_mimes must be a subset of mimes"
        );
    }

    #[test]
    fn pdf_group_is_pdf_only() {
        let groups = groups();
        let pdf = group(&groups, "PDF Viewer");

        assert_eq!(pdf.mimes, vec!["application/pdf".to_string()]);
        assert_eq!(pdf.app_mimes, vec!["application/pdf".to_string()]);
    }

    #[test]
    fn archiver_covers_kde_types() {
        let groups = groups();
        let archiver = group(&groups, "Archiver");

        for mime in [
            "application/x-bzip-compressed-tar",
            "application/x-lz4-compressed-tar",
            "application/x-cpio",
            "application/vnd.ms-cab-compressed",
            "application/x-source-rpm",
            "application/x-iso9660-appimage",
            "application/vnd.rar",
            "application/x-lha",
            "application/x-bzip",
            "application/x-bzip2",
        ] {
            assert!(
                archiver.mimes.iter().any(|m| m == mime),
                "archiver covers {mime}"
            );
        }
        assert_eq!(archiver.app_mimes, archiver.mimes);
    }

    #[test]
    fn video_group_covers_matroska() {
        let groups = groups();
        let video = group(&groups, "Video Player");

        assert!(video.mimes.iter().any(|m| m == "application/x-matroska"));
        assert!(video.app_mimes.iter().any(|m| m == "application/x-matroska"));
    }

    #[test]
    fn text_editor_covers_kde_extra_types() {
        let groups = groups();
        let editor = group(&groups, "Text Editor");

        for mime in ["application/x-docbook+xml", "application/x-yaml"] {
            assert!(
                editor.mimes.iter().any(|m| m == mime),
                "text editor covers {mime}"
            );
        }
    }

    #[test]
    fn groups_follow_section_scheme() {
        let expected = [
            ("Web Browser", "Internet"),
            ("Calendar", "Internet"),
            ("Mail Client", "Internet"),
            ("Map", "Internet"),
            ("Image Viewer", "Multimedia"),
            ("Audio Player", "Multimedia"),
            ("Video Player", "Multimedia"),
            ("Text Editor", "Documents"),
            ("PDF Viewer", "Documents"),
            ("Office documents", "Documents"),
            ("Terminal", "System"),
            ("File Manager", "System"),
            ("Archiver", "System"),
        ];

        let actual: Vec<_> = groups()
            .iter()
            .map(|group| (group.name, group.section))
            .collect();
        assert_eq!(actual, expected, "groups must follow the section scheme");
    }

    #[test]
    fn desktop_file_contents_sets_required_fields() {
        let mimes = vec!["application/pdf".to_string(), "text/html".to_string()];
        let contents = desktop_file_contents("Foo", "foo", &mimes);

        assert!(contents.starts_with("[Desktop Entry]\nType=Application\n"));
        assert!(contents.contains("Name=Foo\n"));
        assert!(contents.contains("Exec=foo\n"));
        assert!(contents.contains("NoDisplay=true\n"));
        assert!(contents.contains("MimeType=application/pdf;text/html;\n"));
    }

    #[test]
    fn desktop_file_contents_skips_empty_mimetype() {
        let contents = desktop_file_contents("Foo", "foo", &[]);
        assert!(!contents.contains("MimeType="));
    }

    #[test]
    fn exec_appends_u_for_scheme_handlers() {
        assert_eq!(format_exec("foo", "x-scheme-handler/http"), "foo %u");
        assert_eq!(format_exec("foo", "x-scheme-handler/tel"), "foo %u");
    }

    #[test]
    fn exec_keeps_existing_placeholders() {
        assert_eq!(format_exec("foo %f", "x-scheme-handler/http"), "foo %f");
        assert_eq!(format_exec("foo", "application/pdf"), "foo");
    }

    #[test]
    fn display_name_strips_directory_and_keeps_args() {
        assert_eq!(display_name("/usr/bin/foo --bar"), "foo --bar");
        assert_eq!(display_name("foo"), "foo");
    }

    #[test]
    fn desktop_file_name_sanitizes_basename() {
        assert_eq!(desktop_file_name("/usr/bin/foo"), "mimic-foo.desktop");
        assert_eq!(desktop_file_name("my tool"), "mimic-my.desktop");
        assert_eq!(desktop_file_name("/usr/bin/my tool"), "mimic-my.desktop");
        assert_eq!(desktop_file_name("/"), "mimic-app.desktop");
    }

    #[test]
    fn unique_desktop_name_appends_suffix() {
        let mut taken = std::collections::HashSet::new();
        taken.insert("mimic-foo.desktop".to_string());

        assert_eq!(
            unique_desktop_name("mimic-foo.desktop", &taken),
            "mimic-foo-2.desktop"
        );
        taken.insert("mimic-foo-2.desktop".to_string());
        assert_eq!(
            unique_desktop_name("mimic-foo.desktop", &taken),
            "mimic-foo-3.desktop"
        );
        assert_eq!(
            unique_desktop_name("mimic-bar.desktop", &taken),
            "mimic-bar.desktop"
        );
    }

    #[test]
    fn merge_mimetype_line_adds_missing_types() {
        let existing = "[Desktop Entry]\nMimeType=application/pdf;\n";
        let mimes = vec![
            "application/pdf".to_string(),
            "application/x-fictionbook".to_string(),
        ];

        let (merged, changed) = merge_mimetype_line(existing, &mimes);
        assert!(changed);
        assert!(merged.contains("MimeType=application/pdf;application/x-fictionbook;\n"));

        let (unchanged, changed_again) = merge_mimetype_line(&merged, &mimes);
        assert!(!changed_again);
        assert_eq!(unchanged, merged);
    }

    #[test]
    fn listable_keeps_mimic_apps_visible() {
        assert!(listable(Some("mimic-foo.desktop"), false));
        assert!(listable(Some("other.desktop"), true));
        assert!(!listable(Some("other.desktop"), false));
        assert!(!listable(None, false));
        assert!(listable(None, true));
    }

    #[test]
    fn command_program_prefers_existing_path_with_spaces() {
        assert_eq!(command_program("firefox --new-window"), "firefox");
        assert_eq!(command_program("/nonexistent path/with space"), "/nonexistent");
        assert_eq!(command_program("   "), "");
    }
}
