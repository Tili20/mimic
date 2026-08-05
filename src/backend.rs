// База данных MIME-типов через GIO.

use std::collections::BTreeSet;

use gio::AppInfo;
use gio::prelude::*;

use crate::model::MimeType;

pub fn load_mime_types() -> Vec<MimeType> {
    let registered: BTreeSet<_> =
        gio::functions::content_types_get_registered().into_iter().collect();
    let mut mime_types: Vec<_> = registered
        .into_iter()
        .map(|mime| {
            let description = gio::functions::content_type_get_description(&mime);
            let category = category_of(&mime);
            let default_app = AppInfo::default_for_type(&mime, false)
                .map(|app| app.name().to_string());
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
    text_mimes.extend(explicit(&["application/json", "application/xml", "application/javascript"]));

    vec![
        Group {
            name: "Web Browser",
            mimes: explicit(&["x-scheme-handler/http", "x-scheme-handler/https"]),
            app_mimes: explicit(&["x-scheme-handler/http", "x-scheme-handler/https"]),
        },
        Group {
            name: "Text Editor",
            mimes: text_mimes,
            app_mimes: explicit(&["text/plain", "text/markdown", "application/json"]),
        },
        Group {
            name: "Image Viewer",
            mimes: with_prefix("image/"),
            app_mimes: explicit(&["image/jpeg", "image/png", "image/gif", "image/webp"]),
        },
        Group {
            name: "Audio Player",
            mimes: with_prefix("audio/"),
            app_mimes: explicit(&["audio/mpeg", "audio/ogg", "audio/flac", "audio/mp4"]),
        },
        Group {
            name: "Video Player",
            mimes: with_prefix("video/"),
            app_mimes: explicit(&["video/mp4", "video/webm", "video/x-matroska"]),
        },
        Group {
            name: "Mail Client",
            mimes: explicit(&["x-scheme-handler/mailto"]),
            app_mimes: explicit(&["x-scheme-handler/mailto"]),
        },
        Group {
            name: "Calendar",
            mimes: explicit(&["text/calendar"]),
            app_mimes: explicit(&["text/calendar"]),
        },
        Group {
            name: "File Manager",
            mimes: explicit(&["inode/directory"]),
            app_mimes: explicit(&["inode/directory"]),
        },
        Group {
            name: "Terminal",
            mimes: terminal.clone(),
            app_mimes: terminal,
        },
        Group {
            name: "Archiver",
            mimes: archives.clone(),
            app_mimes: archives,
        },
        Group {
            name: "Map",
            mimes: explicit(&[
                "x-scheme-handler/map",
                "x-scheme-handler/geo",
                "x-scheme-handler/maps",
            ]),
            app_mimes: explicit(&["x-scheme-handler/map", "x-scheme-handler/geo"]),
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
