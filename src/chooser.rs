// Окно выбора стороннего приложения для пункта «Other…».

use std::cell::RefCell;
use std::rc::Rc;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::{
    Align, Box, Button, Entry, EventControllerKey, FileDialog, Label, ListBox, ListBoxRow,
    SearchEntry, ScrolledWindow, Window,
};

type OnClose = Rc<dyn Fn(Option<gio::AppInfo>)>;

/// Показывает модальный диалог выбора приложения.
///
/// `mimes`/`primary_mime` описывают контекст выбора: они нужны, чтобы
/// создать desktop-файл для введённой команды. `on_close` вызывается ровно
/// один раз: `Some(app)` — подтверждённый выбор, `None` — отмена.
pub fn show_other_dialog<F>(
    parent: &gtk::ApplicationWindow,
    heading: &str,
    mimes: Vec<String>,
    primary_mime: String,
    on_close: F,
) where
    F: FnOnce(Option<gio::AppInfo>) + 'static,
{
    let on_close_slot: Rc<RefCell<Option<F>>> = Rc::new(RefCell::new(Some(on_close)));

    let window = Window::builder()
        .title("Choose application")
        .default_width(480)
        .default_height(460)
        .transient_for(parent)
        .modal(true)
        .build();

    let finish: OnClose = {
        let slot = on_close_slot.clone();
        let window = window.clone();
        Rc::new(move |picked| {
            let callback = slot.borrow_mut().take();
            if let Some(callback) = callback {
                callback(picked);
                window.close();
            }
        })
    };

    let request_slot = on_close_slot.clone();
    window.connect_close_request(move |_| {
        let callback = request_slot.borrow_mut().take();
        if let Some(callback) = callback {
            callback(None);
        }
        glib::Propagation::Proceed
    });

    let heading_label = Label::new(Some(heading));
    heading_label.add_css_class("mimic-description");
    heading_label.set_xalign(0.0);
    heading_label.set_wrap(true);

    let search = SearchEntry::builder()
        .placeholder_text("Search applications…")
        .hexpand(true)
        .build();

    let all_apps = gio::AppInfo::all();
    let all_apps: Rc<Vec<gio::AppInfo>> = {
        let mut apps: Vec<_> = all_apps
            .into_iter()
            .filter(crate::backend::should_list)
            .collect();
        apps.sort_by_key(|app| app.name());
        Rc::new(apps)
    };

    let list = ListBox::new();
    let visible: Rc<RefCell<Vec<gio::AppInfo>>> = Rc::new(RefCell::new(Vec::new()));

    let rebuild = {
        let list = list.clone();
        let visible = visible.clone();
        let all_apps = all_apps.clone();
        Rc::new(move |query: &str| {
            let query = query.to_lowercase();
            let filtered: Vec<gio::AppInfo> = all_apps
                .iter()
                .filter(|app| {
                    query.is_empty() || app.name().to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            for app in &filtered {
                list.append(&make_row(app));
            }
            *visible.borrow_mut() = filtered;
        })
    };
    rebuild("");

    let filter_rebuild = rebuild.clone();
    search.connect_search_changed(move |search| {
        filter_rebuild(&search.text());
    });

    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .max_content_height(320)
        .child(&list)
        .build();
    scrolled.set_vexpand(true);

    let command_hint = Label::new(Some("or enter a command:"));
    command_hint.add_css_class("dim-label");
    command_hint.set_xalign(0.0);

    let entry = Entry::builder()
        .placeholder_text("Program path or command…")
        .hexpand(true)
        .build();

    let browse = Button::with_label("Browse…");

    let error = Label::new(None);
    error.add_css_class("mimic-error");
    error.set_xalign(0.0);
    error.set_wrap(true);

    let select_button = Button::builder()
        .label("Select")
        .css_classes(["suggested-action"])
        .build();
    let cancel_button = Button::with_label("Cancel");

    let update_sensitivity = {
        let list = list.clone();
        let entry = entry.clone();
        let select_button = select_button.clone();
        Rc::new(move || {
            let has_command = !entry.text().trim().is_empty();
            select_button.set_sensitive(has_command || list.selected_row().is_some());
        })
    };
    list.connect_selected_rows_changed({
        let update = update_sensitivity.clone();
        move |_| update()
    });
    entry.connect_changed({
        let update = update_sensitivity.clone();
        move |_| update()
    });

    let submit_selection = {
        let list = list.clone();
        let visible = visible.clone();
        let finish = finish.clone();
        move || {
            let Some(row) = list.selected_row() else {
                return;
            };
            let picked = visible.borrow().get(row.index() as usize).cloned();
            if let Some(app) = picked {
                finish(Some(app));
            }
        }
    };

    let submit_command = {
        let entry = entry.clone();
        let error = error.clone();
        let finish = finish.clone();
        let mimes = mimes.clone();
        let primary = primary_mime.clone();
        Rc::new(move || {
            let command = entry.text().to_string();
            let command = command.trim();
            if command.is_empty() {
                error.set_text("Enter a command or pick an application");
                return;
            }
            let program = crate::backend::command_program(command);
            if !crate::backend::program_exists(program) {
                error.set_text(&format!("Not found or not executable: {program}"));
                return;
            }
            match crate::backend::write_desktop_file(command, &mimes, &primary) {
                Err(error_message) => {
                    error.set_text(&format!("Failed to save application: {error_message}"));
                }
                Ok(id) => match crate::backend::find_app(&id) {
                    Some(app) => finish(Some(app)),
                    None => retry_find(id, finish.clone(), error.clone()),
                },
            }
        })
    };

    select_button.connect_clicked({
        let entry = entry.clone();
        let submit_command = submit_command.clone();
        let submit_selection = submit_selection.clone();
        move |_| {
            if entry.text().trim().is_empty() {
                submit_selection();
            } else {
                submit_command();
            }
        }
    });
    entry.connect_activate({
        let submit_command = submit_command.clone();
        move |_| submit_command()
    });
    list.connect_row_activated({
        let submit_selection = submit_selection.clone();
        move |_, _| submit_selection()
    });
    cancel_button.connect_clicked({
        let finish = finish.clone();
        move |_| finish(None)
    });

    let browse_dialog = FileDialog::new();
    browse.connect_clicked({
        let window = window.clone();
        let entry = entry.clone();
        move |_| {
            let entry = entry.clone();
            browse_dialog.open(
                Some(&window),
                None::<&gio::Cancellable>,
                move |result| {
                    if let Ok(file) = result
                        && let Some(path) = file.path()
                    {
                        entry.set_text(&path.to_string_lossy());
                    }
                },
            );
        }
    });

    let buttons = Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .halign(Align::End)
        .margin_top(4)
        .build();
    buttons.append(&cancel_button);
    buttons.append(&select_button);

    let command_row = Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();
    command_row.append(&entry);
    command_row.append(&browse);

    let content = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(16)
        .margin_end(16)
        .build();
    content.append(&heading_label);
    content.append(&search);
    content.append(&scrolled);
    content.append(&command_hint);
    content.append(&command_row);
    content.append(&error);
    content.append(&buttons);

    window.set_child(Some(&content));

    let escape_window = window.clone();
    let controller = EventControllerKey::new();
    controller.connect_key_pressed(move |_, keyval, _, _| {
        if keyval == gtk::gdk::Key::Escape {
            escape_window.close();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    window.add_controller(controller);

    let focus_search = search.clone();
    window.connect_show(move |_| {
        focus_search.grab_focus();
    });

    update_sensitivity();
    window.present();
}

fn retry_find(id: String, finish: OnClose, error: Label) {
    glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
        match crate::backend::find_app(&id) {
            Some(app) => finish(Some(app)),
            None => error.set_text("Application was saved but not recognized; try again"),
        }
    });
}

fn make_row(app: &gio::AppInfo) -> ListBoxRow {
    let image = gtk::Image::new();
    image.set_pixel_size(24);
    match app.icon() {
        Some(icon) => image.set_from_gicon(&icon),
        None => image.set_icon_name(Some("application-x-executable")),
    }

    let name = Label::new(Some(&app.name()));
    name.set_xalign(0.0);
    name.set_hexpand(true);
    name.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let row_box = Box::new(gtk::Orientation::Horizontal, 8);
    row_box.set_margin_top(4);
    row_box.set_margin_bottom(4);
    row_box.set_margin_start(6);
    row_box.set_margin_end(6);
    row_box.append(&image);
    row_box.append(&name);

    let row = ListBoxRow::new();
    row.set_child(Some(&row_box));
    row
}
