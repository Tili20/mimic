#![allow(dead_code)]

use gio::ListStore;
use gio::prelude::*;
use gtk::prelude::*;
use gtk::{
    Box, Button, EventControllerKey, Label, ListView, ScrolledWindow, SignalListItemFactory,
    SingleSelection, Window,
};

use crate::model::MimeType;

pub fn show_app_chooser(parent_window: &gtk::ApplicationWindow, mime_type: &MimeType) {
    let apps = crate::backend::apps_for(&mime_type.mime());

    let apps_store = ListStore::new::<gio::AppInfo>();
    for app in &apps {
        apps_store.append(app);
    }

    let title = Label::new(Some(&format!("Default for {}", mime_type.mime())));
    title.add_css_class("mimic-description");
    title.set_xalign(0.0);
    title.set_wrap(true);

    let status = Label::new(None);
    status.add_css_class("mimic-error");
    status.set_xalign(0.0);
    status.set_wrap(true);

    let apps_selection = SingleSelection::new(Some(apps_store.clone()));
    apps_selection.set_autoselect(false);

    let factory = SignalListItemFactory::new();
    factory.connect_setup(|_, object| {
        let item = object.downcast_ref::<gtk::ListItem>().expect("item is a ListItem");
        setup_app_row(item);
    });
    factory.connect_bind(|_, object| {
        let item = object.downcast_ref::<gtk::ListItem>().expect("item is a ListItem");
        bind_app_row(item);
    });

    let apps_list = ListView::new(Some(apps_selection.clone()), Some(factory));
    apps_list.set_vexpand(true);

    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .max_content_height(300)
        .child(&apps_list)
        .build();

    let set_button = Button::builder()
        .label("Set Default")
        .css_classes(["suggested-action"])
        .build();

    let content = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(16)
        .margin_end(16)
        .build();
    content.append(&title);
    content.append(&scrolled);
    content.append(&set_button);
    content.append(&status);

    let chooser = Window::builder()
        .title("Choose application")
        .default_width(450)
        .default_height(440)
        .transient_for(parent_window)
        .modal(true)
        .child(&content)
        .build();

    let close_chooser = chooser.clone();
    let controller = EventControllerKey::new();
    controller.connect_key_pressed(move |_, keyval, _, _| {
        if keyval == gtk::gdk::Key::Escape {
            close_chooser.close();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    chooser.add_controller(controller);

    let button_chooser = chooser.clone();
    let button_mime = mime_type.clone();
    let button_status = status.clone();
    let button_selection = apps_selection.clone();
    let button_store = apps_store.clone();
    set_button.connect_clicked(move |_| {
        if let Some(app) = selected_app(&button_selection, &button_store) {
            apply_default(&button_chooser, &button_mime, &app, &button_status);
        }
    });

    let list_chooser = chooser.clone();
    let list_mime = mime_type.clone();
    let list_status = status.clone();
    let list_selection = apps_selection.clone();
    let list_store = apps_store.clone();
    apps_list.connect_activate(move |_, _| {
        if let Some(app) = selected_app(&list_selection, &list_store) {
            apply_default(&list_chooser, &list_mime, &app, &list_status);
        }
    });

    chooser.present();
}

fn selected_app(selection: &SingleSelection, store: &ListStore) -> Option<gio::AppInfo> {
    let position = selection.selected();
    if position == u32::MAX {
        return None;
    }
    store.item(position).and_downcast::<gio::AppInfo>()
}

fn apply_default(
    chooser: &Window,
    mime_type: &MimeType,
    app: &gio::AppInfo,
    status: &Label,
) {
    match crate::backend::set_default(&mime_type.mime(), app) {
        Ok(()) => {
            mime_type.set_default_app(Some(app.name()));
            chooser.close();
        }
        Err(error) => status.set_text(&format!("Failed to set default: {error}")),
    }
}

fn setup_app_row(item: &gtk::ListItem) {
    let image = gtk::Image::new();
    image.set_pixel_size(24);

    let name = Label::new(None);
    name.set_xalign(0.0);
    name.set_hexpand(true);
    name.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let row = Box::new(gtk::Orientation::Horizontal, 8);
    row.set_margin_top(4);
    row.set_margin_bottom(4);
    row.set_margin_start(6);
    row.set_margin_end(6);
    row.append(&image);
    row.append(&name);

    item.set_child(Some(&row));
    unsafe {
        item.set_data("image", image);
        item.set_data("name", name);
    }
}

fn bind_app_row(item: &gtk::ListItem) {
    let app = item
        .item()
        .and_downcast::<gio::AppInfo>()
        .expect("list item holds an AppInfo");

    let (image, name) = unsafe {
        (
            item.data::<gtk::Image>("image").expect("widgets are set up").as_ref(),
            item.data::<Label>("name").expect("widgets are set up").as_ref(),
        )
    };

    name.set_text(&app.name());
    match app.icon() {
        Some(icon) => image.set_from_gicon(&icon),
        None => image.set_icon_name(Some("application-x-executable")),
    }
}