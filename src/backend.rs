// База данных MIME-типов через GIO.

use std::collections::BTreeSet;

use gio::AppInfo;
use gio::prelude::*;

use crate::model::MimeType;

pub fn load_mime_types() -> Vec<MimeType> {
    let registered: BTreeSet<_> =
        gio::functions::content_types_get_registered().into_iter().collect();
    registered
        .into_iter()
        .map(|mime| {
            let description = gio::functions::content_type_get_description(&mime);
            let category = category_of(&mime);
            let default_app = AppInfo::default_for_type(&mime, false)
                .map(|app| app.name().to_string());
            MimeType::new(&mime, &description, &category, default_app)
        })
        .collect()
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
    pub mimes: Vec<String>,
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

    vec![
        Group {
            name: "Web Browser",
            mimes: vec!["x-scheme-handler/http".into(), "x-scheme-handler/https".into()],
        },
        Group {
            name: "Text Editor",
            mimes: with_prefix("text/"),
        },
        Group {
            name: "Image Viewer",
            mimes: with_prefix("image/"),
        },
        Group {
            name: "Audio Player",
            mimes: with_prefix("audio/"),
        },
        Group {
            name: "Video Player",
            mimes: with_prefix("video/"),
        },
        Group {
            name: "Mail Client",
            mimes: vec!["x-scheme-handler/mailto".into()],
        },
        Group {
            name: "Calendar",
            mimes: vec!["text/calendar".into()],
        },
        Group {
            name: "File Manager",
            mimes: vec!["inode/directory".into()],
        },
        Group {
            name: "Terminal",
            mimes: vec!["application/x-terminal-emulator".into()],
        },
        Group {
            name: "Archiver",
            mimes: vec![
                "application/zip".into(),
                "application/gzip".into(),
                "application/x-tar".into(),
                "application/x-compressed-tar".into(),
                "application/x-7z-compressed".into(),
                "application/x-rar".into(),
                "application/x-bzip2".into(),
                "application/x-xz".into(),
                "application/zstd".into(),
            ],
        },
        Group {
            name: "Map",
            mimes: vec![
                "x-scheme-handler/map".into(),
                "x-scheme-handler/geo".into(),
                "x-scheme-handler/maps".into(),
            ],
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
