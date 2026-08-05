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
