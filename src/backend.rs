// База данных MIME-типов через GIO.

use std::collections::BTreeSet;

use gio::AppInfo;
use gio::prelude::*;

use crate::model::MimeType;

pub fn load_mime_types() -> Vec<MimeType> {
    let mime_types = gio::functions::content_types_get_registered();
    let mime_types: BTreeSet<_> = mime_types.into_iter().collect();
    let mime_types = mime_types
        .into_iter()
        .map(|mime| {
            let description = gio::functions::content_type_get_description(&mime);
            let category = category_of(&mime);
            let default_app = AppInfo::default_for_type(&mime, false)
                .map(|app| app.name().to_string());
            MimeType::new(&mime, &description, &category, default_app)
        })
        .collect();
    mime_types
}

fn category_of(mime: &str) -> String {
    mime.split_once('/')
        .map(|(category, _)| category.to_string())
        .unwrap_or_else(|| "other".to_string())
}
