// Сборка интерфейса.

use std::cell::Cell;
use std::rc::Rc;

use gio::ListStore;
use gio::prelude::*;
use gtk::prelude::*;
use gtk::{
    Align, Box, Button, CheckButton, DropDown, Entry, EventControllerKey, FileDialog, FileFilter,
    Grid, HeaderBar, Label, ListView, MenuButton, Notebook, Popover, ScrolledWindow, SearchEntry,
    SignalListItemFactory, SingleSelection, StringList, Window,
};

use crate::model::MimeType;

type ApplyFn = Rc<dyn Fn(&gio::AppInfo) -> Result<(), glib::Error>>;

const CUSTOM_APP_LABEL: &str = "Custom application…";

mod imp {
    use std::cell::RefCell;

    use glib::Properties;
    use glib::prelude::*;
    use glib::subclass::prelude::*;
    use gtk::subclass::prelude::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::MimicFilter)]
    pub struct MimicFilter {
        #[property(get, set)]
        pub search: RefCell<String>,
        #[property(get, set)]
        pub category: RefCell<String>,
        #[property(get, set)]
        pub hide_rare: RefCell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MimicFilter {
        const NAME: &'static str = "MimicFilter";
        type Type = super::MimicFilter;
        type ParentType = gtk::Filter;
    }

    impl ObjectImpl for MimicFilter {
        fn constructed(&self) {
            self.parent_constructed();
            self.hide_rare.replace(true);
        }

        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }

    impl FilterImpl for MimicFilter {
        fn match_(&self, item: &glib::Object) -> bool {
            let mime_type = item
                .downcast_ref::<crate::model::MimeType>()
                .expect("filtered items are MimeType objects");

            super::matches(
                mime_type,
                self.search.borrow().as_str(),
                self.category.borrow().as_str(),
                *self.hide_rare.borrow(),
            )
        }
    }
}

glib::wrapper! {
    pub struct MimicFilter(ObjectSubclass<imp::MimicFilter>) @extends gtk::Filter;
}

impl MimicFilter {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

fn matches(mime_type: &MimeType, search: &str, category: &str, hide_rare: bool) -> bool {
    let search = search.to_lowercase();

    if hide_rare && crate::backend::is_rare(&mime_type.mime()) {
        return false;
    }

    let category_matches =
        category.is_empty() || category == "All" || mime_type.category() == category;

    let search_matches = search.is_empty()
        || mime_type.mime().to_lowercase().contains(&search)
        || mime_type.description().to_lowercase().contains(&search);

    category_matches && search_matches
}

const CSS: &str = "
.mimic-description { font-weight: 700; }
.mimic-error { color: red; }
";

pub fn build_ui(app: &gtk::Application) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Mimic")
        .default_width(900)
        .default_height(600)
        .build();

    load_css(&window);
    setup_window(&window);
}

fn load_css(window: &gtk::ApplicationWindow) {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(CSS);
    let display = gtk::prelude::WidgetExt::display(window);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn setup_window(window: &gtk::ApplicationWindow) {
    let mime_types = crate::backend::load_mime_types();

    let store = ListStore::new::<MimeType>();
    for mime_type in &mime_types {
        store.append(mime_type);
    }

    let filter = MimicFilter::new();
    let filtered = gtk::FilterListModel::new(Some(store.clone()), Some(filter.clone()));

    let selection = SingleSelection::new(Some(filtered));
    selection.set_autoselect(false);

    let status = Label::new(None);
    status.add_css_class("mimic-error");
    status.set_xalign(0.0);
    status.set_wrap(true);

    let setup_status = status.clone();
    let setup_window = window.clone();
    let factory = SignalListItemFactory::new();
    factory.connect_setup(move |_, object| {
        let item = object.downcast_ref::<gtk::ListItem>().expect("item is a ListItem");
        setup_row(item, &setup_status, &setup_window);
    });
    factory.connect_bind(|_, object| {
        let item = object.downcast_ref::<gtk::ListItem>().expect("item is a ListItem");
        bind_row(item);
    });

    let list_view = ListView::new(Some(selection.clone()), Some(factory));
    list_view.set_vexpand(true);

    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&list_view)
        .build();

    let search = SearchEntry::builder()
        .placeholder_text("Search MIME types…")
        .hexpand(true)
        .build();

    let mut categories: Vec<String> = mime_types
        .iter()
        .map(|m| m.category())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    categories.insert(0, "All".to_string());
    let category_refs: Vec<&str> = categories.iter().map(String::as_str).collect();
    let category_model = StringList::new(&category_refs);

    let dropdown = DropDown::new(Some(category_model), None::<&gtk::Expression>);

    let filter_search = filter.clone();
    search.connect_search_changed(move |search| {
        filter_search.set_search(search.text().to_string());
        filter_search.changed(gtk::FilterChange::Different);
    });

    let search_is_focused = Rc::new(Cell::new(false));
    let flag_enter = search_is_focused.clone();
    let focus_tracker = gtk::EventControllerFocus::new();
    focus_tracker.set_propagation_phase(gtk::PropagationPhase::Capture);
    focus_tracker.connect_enter(move |_| flag_enter.set(true));
    let flag_leave = search_is_focused.clone();
    focus_tracker.connect_leave(move |_| flag_leave.set(false));
    search.add_controller(focus_tracker);

    let search_for_keys = search.clone();
    let search_focused = search_is_focused.clone();
    let keys = EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    keys.connect_key_pressed(move |_, keyval, _, state| {
        if keyval == gtk::gdk::Key::Escape {
            search_for_keys.set_text("");
            return glib::Propagation::Stop;
        }
        let modifiers = gtk::gdk::ModifierType::CONTROL_MASK
            | gtk::gdk::ModifierType::ALT_MASK
            | gtk::gdk::ModifierType::SUPER_MASK;
        if !search_focused.get()
            && !state.intersects(modifiers)
            && keyval.to_unicode().is_some()
        {
            search_for_keys.grab_focus();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });

    let focus_search = search.clone();
    window.connect_show(move |_| {
        focus_search.grab_focus();
    });

    let filter_category = filter.clone();
    dropdown.connect_selected_notify(move |dropdown| {
        let index = dropdown.selected() as usize;
        let category = categories.get(index).cloned().unwrap_or_default();
        filter_category.set_category(category);
        filter_category.changed(gtk::FilterChange::Different);
    });

    let controls = Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .build();
    controls.append(&search);
    controls.append(&dropdown);

    let advanced_root = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    advanced_root.append(&controls);
    advanced_root.append(&scrolled);
    advanced_root.append(&status);
    advanced_root.add_controller(keys);

    let general_page = build_general_page(window, &store);

    let notebook = Notebook::new();
    notebook.append_page(&general_page, Some(&Label::new(Some("General"))));
    notebook.append_page(&advanced_root, Some(&Label::new(Some("Advanced"))));
    notebook.set_show_border(false);

    let focus_on_switch = search.clone();
    notebook.connect_switch_page(move |_, _, page_num| {
        if page_num == 1 {
            focus_on_switch.grab_focus();
        }
    });

    let menu_button = build_menu_button(&filter);

    let header = HeaderBar::new();
    header.set_title_widget(Some(&gtk::Label::new(Some("Mimic"))));
    header.pack_end(&menu_button);

    window.set_titlebar(Some(&header));
    window.set_child(Some(&notebook));
    window.present();
}

fn build_general_page(window: &gtk::ApplicationWindow, store: &ListStore) -> gtk::Box {
    let status = Label::new(None);
    status.add_css_class("mimic-error");
    status.set_xalign(0.0);
    status.set_wrap(true);

    let grid = Grid::new();
    grid.set_column_spacing(8);
    grid.set_row_spacing(6);
    grid.set_margin_top(12);
    grid.set_margin_bottom(12);
    grid.set_margin_start(16);
    grid.set_margin_end(16);

    let mut row = 0;
    for group in crate::backend::groups() {
        if group.mimes.is_empty() {
            continue;
        }
        attach_group_row(&grid, row, &group, store, &status, window);
        row += 1;
    }

    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&grid)
        .build();
    scrolled.set_vexpand(true);

    let page = Box::builder().orientation(gtk::Orientation::Vertical).build();
    page.append(&scrolled);
    page.append(&status);
    page
}

fn attach_group_row(
    grid: &Grid,
    row: i32,
    group: &crate::backend::Group,
    store: &ListStore,
    status: &Label,
    window: &gtk::ApplicationWindow,
) {
    let mut apps = crate::backend::apps_for_group(&group.app_mimes);
    if apps.is_empty()
        && let Some(default) = gio::AppInfo::default_for_type(&group.app_mimes[0], false)
    {
        apps.push(default);
    }

    let name_strings: Vec<String> = apps
        .iter()
        .map(|app| app.name().to_string())
        .chain(std::iter::once(CUSTOM_APP_LABEL.to_string()))
        .collect();
    let names: Vec<&str> = name_strings.iter().map(String::as_str).collect();
    let model = StringList::new(&names);
    let dropdown = DropDown::new(Some(model), None::<&gtk::Expression>);
    dropdown.set_hexpand(true);
    dropdown.set_valign(Align::Center);

    if let Some(default) = gio::AppInfo::default_for_type(&group.app_mimes[0], false)
        && let Some(index) = apps.iter().position(|app| app.id() == default.id())
    {
        dropdown.set_selected(index as u32);
    }

    let name_label = Label::new(Some(group.name));
    name_label.add_css_class("mimic-description");
    name_label.set_xalign(0.0);
    name_label.set_valign(Align::Center);

    let button = Button::with_label("Set Default");
    button.set_halign(Align::Start);
    button.set_valign(Align::Center);

    let click_dropdown = dropdown.clone();
    let click_status = status.clone();
    let click_mimes = group.mimes.clone();
    let click_apps = apps.clone();
    let click_store = store.clone();
    button.connect_clicked(move |_| {
        let index = click_dropdown.selected();
        if index == u32::MAX || index as usize == click_apps.len() {
            return;
        }
        let Some(app) = click_apps.get(index as usize) else {
            return;
        };
        match crate::backend::set_default_group(&click_mimes, app) {
            Ok(()) => {
                click_status.set_text("");
                for i in 0..click_store.n_items() {
                    if let Some(mime_type) = click_store.item(i).and_downcast::<MimeType>()
                        && click_mimes.iter().any(|m| m.as_str() == mime_type.mime().as_str())
                    {
                        mime_type.set_default_app(Some(app.name()));
                    }
                }
            }
            Err(error) => click_status.set_text(&format!("Failed to set default: {error}")),
        }
    });

    grid.attach(&name_label, 0, row, 1, 1);
    grid.attach(&dropdown, 1, row, 1, 1);
    grid.attach(&button, 2, row, 1, 1);

    let notify_dropdown = dropdown.clone();
    let notify_apps = apps.clone();
    let notify_mimes = group.mimes.clone();
    let notify_store = store.clone();
    let notify_window = window.clone();
    let notify_title = group.name.to_string();
    dropdown.connect_selected_notify(move |_| {
        let index = notify_dropdown.selected();
        if index == u32::MAX || index as usize != notify_apps.len() {
            return;
        }
        let apply_mimes = notify_mimes.clone();
        let apply_store = notify_store.clone();
        show_custom_app_dialog(&notify_window, &notify_title, Rc::new(move |app| {
            crate::backend::set_default_group(&apply_mimes, app)?;
            for i in 0..apply_store.n_items() {
                if let Some(mime_type) = apply_store.item(i).and_downcast::<MimeType>()
                    && apply_mimes.iter().any(|m| m.as_str() == mime_type.mime().as_str())
                {
                    mime_type.set_default_app(Some(app.name()));
                }
            }
            Ok(())
        }));
    });
}

fn build_menu_button(filter: &MimicFilter) -> MenuButton {
    let check = CheckButton::builder()
        .label("Hide rarely used MIME types")
        .active(true)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();

    let filter_hide = filter.clone();
    check.connect_toggled(move |check| {
        filter_hide.set_hide_rare(check.is_active());
        filter_hide.changed(gtk::FilterChange::Different);
    });

    let popover = Popover::new();
    popover.set_child(Some(&check));

    MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .popover(&popover)
        .build()
}

fn apps_for_dropdown(mime: &str) -> Vec<gio::AppInfo> {
    let mut apps = crate::backend::apps_for(mime);
    if apps.is_empty()
        && let Some(default) = gio::AppInfo::default_for_type(mime, false)
    {
        apps.push(default);
    }
    apps
}

fn show_custom_app_dialog(
    parent_window: &gtk::ApplicationWindow,
    title: &str,
    apply: ApplyFn,
) {
    let title = Label::new(Some(title));
    title.add_css_class("mimic-description");
    title.set_xalign(0.0);
    title.set_wrap(true);

    let status = Label::new(None);
    status.add_css_class("mimic-error");
    status.set_xalign(0.0);
    status.set_wrap(true);

    let command = Entry::new();
    command.set_placeholder_text(Some("/path/to/app"));
    command.set_hexpand(true);

    let browse = Button::builder()
        .label("Browse…")
        .valign(Align::Center)
        .build();

    let command_row = Box::new(gtk::Orientation::Horizontal, 8);
    command_row.append(&command);
    command_row.append(&browse);

    let set_button = Button::builder()
        .label("Set Default")
        .css_classes(["suggested-action"])
        .valign(Align::Center)
        .halign(Align::End)
        .build();

    let content = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();
    content.append(&title);
    content.append(&command_row);
    content.append(&status);
    content.append(&set_button);

    let dialog = Window::builder()
        .title("Custom application")
        .default_width(480)
        .modal(true)
        .transient_for(parent_window)
        .child(&content)
        .build();

    let close_dialog = dialog.clone();
    let controller = EventControllerKey::new();
    controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            close_dialog.close();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    dialog.add_controller(controller);

    let file_dialog = FileDialog::builder()
        .title("Choose executable")
        .modal(true)
        .build();
    let filter = FileFilter::new();
    filter.set_name(Some("Executables"));
    filter.add_mime_type("application/x-executable");
    let filters = gio::ListStore::new::<FileFilter>();
    filters.append(&filter);
    file_dialog.set_filters(Some(&filters));

    let dialog_for_browse = dialog.clone();
    let command_for_browse = command.clone();
    browse.connect_clicked(move |_| {
        let command = command_for_browse.clone();
        file_dialog.clone().open(
            Some(&dialog_for_browse),
            None::<&gio::Cancellable>,
            move |result| {
                if let Ok(file) = result {
                    command.set_text(&file_path(&file));
                }
            },
        );
    });

    let dialog_for_set = dialog.clone();
    let command_for_set = command.clone();
    let status_for_set = status.clone();
    set_button.connect_clicked(move |_| {
        let text = command_for_set.text();
        let value = text.trim();
        if value.is_empty() {
            status_for_set.set_text("Enter a command");
            return;
        }
        match crate::backend::app_from_command(value) {
            Ok(app) => match apply(&app) {
                Ok(()) => dialog_for_set.close(),
                Err(error) => {
                    status_for_set.set_text(&format!("Failed to set default: {error}"))
                }
            },
            Err(error) => status_for_set.set_text(&format!("Invalid command: {error}")),
        }
    });

    dialog.present();
}

fn file_path(file: &gio::File) -> String {
    file.path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| file.uri().to_string())
}

fn setup_row(item: &gtk::ListItem, status: &Label, window: &gtk::ApplicationWindow) {
    let description = Label::new(None);
    description.add_css_class("mimic-description");
    description.set_xalign(0.0);
    description.set_valign(Align::Center);
    description.set_hexpand(true);
    description.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let dropdown = DropDown::new(None::<StringList>, None::<&gtk::Expression>);
    dropdown.set_valign(Align::Center);
    dropdown.set_width_request(280);

    let set_button = Button::with_label("Set Default");
    set_button.set_valign(Align::Center);

    let first = Box::new(gtk::Orientation::Horizontal, 8);
    first.append(&description);
    first.append(&dropdown);
    first.append(&set_button);

    let mime = Label::new(None);
    mime.add_css_class("monospace");
    mime.add_css_class("dim-label");
    mime.set_xalign(0.0);
    mime.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let row = Box::new(gtk::Orientation::Vertical, 2);
    row.set_margin_top(4);
    row.set_margin_bottom(4);
    row.set_margin_start(6);
    row.set_margin_end(6);
    row.append(&first);
    row.append(&mime);

    let click_item = item.clone();
    let click_dropdown = dropdown.clone();
    let click_status = status.clone();
    set_button.connect_clicked(move |_| {
        let index = click_dropdown.selected();
        if index == u32::MAX {
            return;
        }
        let Some(mime_type) = click_item.item().and_downcast::<MimeType>() else {
            return;
        };
        let apps = apps_for_dropdown(&mime_type.mime());
        if index as usize == apps.len() {
            return;
        }
        let click_apps = apps.clone();
        let Some(app) = click_apps.get(index as usize).cloned() else {
            return;
        };
        match crate::backend::set_default(&mime_type.mime(), &app) {
            Ok(()) => {
                click_status.set_text("");
                mime_type.set_default_app(Some(app.name()));
            }
            Err(error) => click_status.set_text(&format!("Failed to set default: {error}")),
        }
    });

    let notify_item = item.clone();
    let notify_window = window.clone();
    let notify_dropdown = dropdown.clone();
    let notify_binding = Rc::new(Cell::new(false));
    let binding = notify_binding.clone();
    dropdown.connect_selected_notify(move |_| {
        if notify_binding.get() {
            return;
        }
        let index = notify_dropdown.selected();
        if index == u32::MAX {
            return;
        }
        let Some(mime_type) = notify_item.item().and_downcast::<MimeType>() else {
            return;
        };
        let apps = apps_for_dropdown(&mime_type.mime());
        if index as usize != apps.len() {
            return;
        }
        let apply_mime = mime_type.clone();
        show_custom_app_dialog(&notify_window, &apply_mime.mime(), Rc::new(move |app| {
            let mime = apply_mime.mime();
            crate::backend::set_default(&mime, app)?;
            apply_mime.set_default_app(Some(app.name()));
            Ok(())
        }));
    });

    item.set_child(Some(&row));
    unsafe {
        item.set_data("description", description);
        item.set_data("mime", mime);
        item.set_data("dropdown", dropdown);
        item.set_data("binding", binding);
    }
}

fn bind_row(item: &gtk::ListItem) {
    let mime_type = item
        .item()
        .and_downcast::<MimeType>()
        .expect("list item holds a MimeType");

    let (description, mime, dropdown) = unsafe {
        (
            item.data::<Label>("description").expect("widgets are set up").as_ref(),
            item.data::<Label>("mime").expect("widgets are set up").as_ref(),
            item.data::<DropDown>("dropdown").expect("widgets are set up").as_ref(),
        )
    };

    description.set_text(&mime_type.description());
    mime.set_text(&mime_type.mime());

    let apps = apps_for_dropdown(&mime_type.mime());

    let name_strings: Vec<String> = apps.iter().map(|app| app.name().to_string()).collect();
    let mut names: Vec<&str> = name_strings.iter().map(String::as_str).collect();
    names.push(CUSTOM_APP_LABEL);
    let model = StringList::new(&names);
    let binding = unsafe { item.data::<Rc<Cell<bool>>>("binding").expect("flag is set up").as_ref() };
    binding.set(true);
    dropdown.set_model(Some(&model));

    dropdown.set_selected(u32::MAX);
    if let Some(default) = gio::AppInfo::default_for_type(&mime_type.mime(), false)
        && let Some(index) = apps.iter().position(|app| app.id() == default.id())
    {
        dropdown.set_selected(index as u32);
    }
    binding.set(false);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches_case(
        mime: &str,
        description: &str,
        item_category: &str,
        filter_category: &str,
        search: &str,
    ) -> bool {
        let mime_type = MimeType::new(mime, description, item_category, None);
        matches(&mime_type, search, filter_category, false)
    }

    #[test]
    fn hide_rare_hides_junk_types() {
        let rare = MimeType::new(
            "x-scheme-handler/http",
            "URL handler for http",
            "x-scheme-handler",
            None,
        );
        assert!(!matches(&rare, "", "", true));
        assert!(matches(&rare, "", "", false));
    }

    #[test]
    fn empty_filter_matches_everything() {
        assert!(matches_case("image/png", "PNG image", "image", "", ""));
        assert!(matches_case("text/plain", "Plain text", "text", "", ""));
    }

    #[test]
    fn category_filters_out_other_categories() {
        assert!(matches_case("image/png", "PNG image", "image", "image", ""));
        assert!(!matches_case("text/plain", "Plain text", "text", "image", ""));
    }

    #[test]
    fn all_category_matches_everything() {
        assert!(matches_case("image/png", "PNG image", "image", "All", ""));
        assert!(matches_case("text/plain", "Plain text", "text", "All", ""));
    }

    #[test]
    fn search_matches_mime_type() {
        assert!(matches_case("image/png", "PNG image", "image", "", "PNG"));
        assert!(!matches_case("image/png", "PNG image", "image", "", "zip"));
    }

    #[test]
    fn search_matches_description() {
        assert!(matches_case("text/plain", "Plain text", "text", "", "plain"));
    }

    #[test]
    fn search_is_case_insensitive() {
        assert!(matches_case("image/png", "PNG image", "image", "", "png"));
        assert!(matches_case("text/plain", "Plain text", "text", "", "PLAIN"));
    }
}