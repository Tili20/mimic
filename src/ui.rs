// Сборка интерфейса.

use std::cell::Cell;
use std::rc::Rc;

use gio::ListStore;
use gio::prelude::*;
use gtk::prelude::*;
use gtk::{
    Align, Box, Button, CheckButton, DropDown, EventControllerKey, Grid, HeaderBar, Label,
    ListView, MenuButton, Notebook, Popover, ScrolledWindow, SearchEntry,
    SignalListItemFactory, SingleSelection, StringList, Window,
};

use crate::model::MimeType;

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

    let factory = SignalListItemFactory::new();
    factory.connect_setup(|_, object| {
        let item = object.downcast_ref::<gtk::ListItem>().expect("item is a ListItem");
        setup_row(item);
    });
    factory.connect_bind(|_, object| {
        let item = object.downcast_ref::<gtk::ListItem>().expect("item is a ListItem");
        bind_row(item);
    });
    factory.connect_unbind(|_, object| {
        let item = object.downcast_ref::<gtk::ListItem>().expect("item is a ListItem");
        let handler = unsafe { item.data::<glib::SignalHandlerId>("default_notify") };
        let mime_type = item.item().and_downcast::<MimeType>();
        if let (Some(handler), Some(mime_type)) = (handler, mime_type) {
            let handler = unsafe { handler.read() };
            mime_type.disconnect(handler);
        }
    });

    let list_view = ListView::new(Some(selection.clone()), Some(factory));
    list_view.set_vexpand(true);
    list_view.set_single_click_activate(true);

    let window_clone = window.clone();
    list_view.connect_activate(move |_, position| {
        let mime_type = selection.item(position).and_downcast::<MimeType>();
        if let Some(mime_type) = mime_type {
            show_app_chooser(&window_clone, &mime_type);
        }
    });

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
    advanced_root.add_controller(keys);

    let general_page = build_general_page(&store);

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

fn build_general_page(store: &ListStore) -> gtk::Box {
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
        attach_group_row(&grid, row, &group, store, &status);
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
) {
    let mut apps = crate::backend::apps_for_group(&group.app_mimes);
    if apps.is_empty()
        && let Some(default) = gio::AppInfo::default_for_type(&group.app_mimes[0], false)
    {
        apps.push(default);
    }

    let name_strings: Vec<String> = apps.iter().map(|app| app.name().to_string()).collect();
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
        if index == u32::MAX {
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

fn show_app_chooser(parent_window: &gtk::ApplicationWindow, mime_type: &MimeType) {
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

fn setup_row(item: &gtk::ListItem) {
    let description = Label::new(None);
    description.add_css_class("mimic-description");
    description.set_xalign(0.0);
    description.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let mime = Label::new(None);
    mime.add_css_class("monospace");
    mime.add_css_class("dim-label");
    mime.set_xalign(0.0);
    mime.set_hexpand(true);
    mime.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let default = Label::new(None);
    default.add_css_class("dim-label");
    default.set_halign(Align::End);
    default.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let second = Box::new(gtk::Orientation::Horizontal, 12);
    second.append(&mime);
    second.append(&default);

    let row = Box::new(gtk::Orientation::Vertical, 2);
    row.set_margin_top(4);
    row.set_margin_bottom(4);
    row.set_margin_start(6);
    row.set_margin_end(6);
    row.append(&description);
    row.append(&second);

    item.set_child(Some(&row));
    unsafe {
        item.set_data("description", description);
        item.set_data("mime", mime);
        item.set_data("default_app", default);
    }
}

fn bind_row(item: &gtk::ListItem) {
    let mime_type = item
        .item()
        .and_downcast::<MimeType>()
        .expect("list item holds a MimeType");

    let (description, mime, default) = unsafe {
        (
            item.data::<Label>("description").expect("widgets are set up").as_ref(),
            item.data::<Label>("mime").expect("widgets are set up").as_ref(),
            item.data::<Label>("default_app").expect("widgets are set up").as_ref(),
        )
    };

    description.set_text(&mime_type.description());
    mime.set_text(&mime_type.mime());
    default.set_text(mime_type.default_app().as_deref().unwrap_or("—"));

    if unsafe { item.data::<glib::SignalHandlerId>("default_notify") }.is_none() {
        let default_label = default.clone();
        let handler = mime_type.connect_notify_local(Some("default-app"), move |mime_type, _| {
            default_label.set_text(mime_type.default_app().as_deref().unwrap_or("—"));
        });
        unsafe {
            item.set_data("default_notify", handler);
        }
    }
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