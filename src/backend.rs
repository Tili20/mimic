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
        .filter(|app| app.should_show())
        .collect::<Vec<_>>();
    apps.sort_by_key(|app| app.name());
    apps
}

pub fn set_default(mime: &str, app: &AppInfo) -> Result<(), glib::Error> {
    app.set_as_default_for_type(mime)
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
        "application/zip",
        "application/gzip",
        "application/x-tar",
        "application/x-compressed-tar",
        "application/x-7z-compressed",
        "application/x-rar",
        "application/x-bzip2",
        "application/x-xz",
        "application/zstd",
    ]);

    let mut text_mimes = with_prefix("text/");
    text_mimes.extend(explicit(&[
        "application/json",
        "application/xml",
        "application/javascript",
    ]));

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
            mimes: with_prefix("video/"),
            app_mimes: explicit(&["video/mp4", "video/webm", "video/x-matroska"]),
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
            if !app.should_show() {
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
}
