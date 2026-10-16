// Сборка интерфейса.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gio::ListStore;
use gio::prelude::*;
use gtk::prelude::*;
use gtk::{
    Align, ActionBar, Box, Button, CheckButton, DropDown, EventControllerKey, Grid, HeaderBar,
    Label, ListView, MenuButton, Notebook, Popover, ScrolledWindow, SearchEntry,
    SignalListItemFactory, SingleSelection, StringList,
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

struct PendingChange {
    key: String,
    mimes: Vec<String>,
    app: gio::AppInfo,
}

/// Выборы, сделанные в UI, но ещё не записанные на диск.
/// Записи хранятся в порядке появления, ключ уникален
/// (имя группы на вкладке General, mime-тип на вкладке Advanced).
#[derive(Default)]
struct PendingChanges {
    changes: Vec<PendingChange>,
}

impl PendingChanges {
    fn select(
        &mut self,
        key: &str,
        mimes: Vec<String>,
        app: gio::AppInfo,
        current_default: Option<&gio::AppInfo>,
    ) {
        if current_default.is_some_and(|current| same_app(&app, current)) {
            self.remove(key);
            return;
        }
        if let Some(change) = self.changes.iter_mut().find(|change| change.key == key) {
            change.mimes = mimes;
            change.app = app;
            return;
        }
        self.changes.push(PendingChange {
            key: key.to_string(),
            mimes,
            app,
        });
    }

    fn get(&self, key: &str) -> Option<&gio::AppInfo> {
        self.changes
            .iter()
            .find(|change| change.key == key)
            .map(|change| &change.app)
    }

    fn entries(&self) -> Vec<(String, Vec<String>, gio::AppInfo)> {
        self.changes
            .iter()
            .map(|change| (change.key.clone(), change.mimes.clone(), change.app.clone()))
            .collect()
    }

    fn remove(&mut self, key: &str) {
        self.changes.retain(|change| change.key != key);
    }

    fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

fn same_app(one: &gio::AppInfo, other: &gio::AppInfo) -> bool {
    match (one.id(), other.id()) {
        (Some(one_id), Some(other_id)) => one_id == other_id,
        _ => one.name() == other.name(),
    }
}

/// Общее состояние для обеих вкладок: ожидающие применения выборы,
/// кнопка Apply и флаг программного обновления dropdown'ов.
#[derive(Clone)]
struct ApplyContext {
    pending: Rc<RefCell<PendingChanges>>,
    apply: Button,
    status: Label,
    rebinding: Rc<Cell<bool>>,
}

impl ApplyContext {
    fn record(
        &self,
        key: &str,
        mimes: Vec<String>,
        app: gio::AppInfo,
        current_default: Option<&gio::AppInfo>,
    ) {
        self.pending
            .borrow_mut()
            .select(key, mimes, app, current_default);
        self.apply
            .set_sensitive(!self.pending.borrow().is_empty());
    }
}

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

    let apply = Button::builder()
        .label("Apply")
        .css_classes(["suggested-action"])
        .sensitive(false)
        .build();

    let ctx = ApplyContext {
        pending: Rc::new(RefCell::new(PendingChanges::default())),
        apply,
        status: status.clone(),
        rebinding: Rc::new(Cell::new(false)),
    };

    let setup_ctx = ctx.clone();
    let factory = SignalListItemFactory::new();
    factory.connect_setup(move |_, object| {
        let item = object.downcast_ref::<gtk::ListItem>().expect("item is a ListItem");
        setup_row(item, &setup_ctx);
    });
    let bind_ctx = ctx.clone();
    factory.connect_bind(move |_, object| {
        let item = object.downcast_ref::<gtk::ListItem>().expect("item is a ListItem");
        bind_row(item, &bind_ctx);
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
    advanced_root.add_controller(keys);

    let general_page = build_general_page(&ctx);

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

    let action_bar = ActionBar::new();
    action_bar.pack_start(&status);
    action_bar.pack_end(&ctx.apply);

    let click_ctx = ctx.clone();
    let click_store = store.clone();
    ctx.apply.connect_clicked(move |_| apply_pending(&click_ctx, &click_store));

    let body = Box::builder().orientation(gtk::Orientation::Vertical).build();
    body.append(&notebook);
    body.append(&action_bar);

    window.set_titlebar(Some(&header));
    window.set_child(Some(&body));
    window.present();
}

fn build_general_page(ctx: &ApplyContext) -> gtk::Box {
    let grid = Grid::new();
    grid.set_column_spacing(8);
    grid.set_row_spacing(6);
    grid.set_margin_top(12);
    grid.set_margin_bottom(12);
    grid.set_margin_start(16);
    grid.set_margin_end(16);

    let mut row = 0;
    let mut last_section: Option<&str> = None;
    for group in crate::backend::groups() {
        if group.mimes.is_empty() {
            continue;
        }
        if last_section != Some(group.section) {
            let header = Label::new(Some(group.section));
            header.add_css_class("heading");
            header.set_halign(Align::Center);
            header.set_margin_top(if row == 0 { 0 } else { 16 });
            header.set_margin_bottom(4);
            grid.attach(&header, 0, row, 2, 1);
            row += 1;
            last_section = Some(group.section);
        }
        attach_group_row(&grid, row, &group, ctx);
        row += 1;
    }

    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&grid)
        .build();
    scrolled.set_vexpand(true);

    let page = Box::builder().orientation(gtk::Orientation::Vertical).build();
    page.append(&scrolled);
    page
}

fn attach_group_row(grid: &Grid, row: i32, group: &crate::backend::Group, ctx: &ApplyContext) {
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

    let row_ctx = ctx.clone();
    let key = group.name.to_string();
    let mimes = group.mimes.clone();
    let row_apps = apps;
    let default_mime = group.app_mimes[0].clone();
    dropdown.connect_selected_notify(move |dropdown| {
        let index = dropdown.selected();
        if index == u32::MAX {
            return;
        }
        let Some(app) = row_apps.get(index as usize).cloned() else {
            return;
        };
        let current = gio::AppInfo::default_for_type(&default_mime, false);
        row_ctx.record(&key, mimes.clone(), app, current.as_ref());
    });

    grid.attach(&name_label, 0, row, 1, 1);
    grid.attach(&dropdown, 1, row, 1, 1);
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

fn setup_row(item: &gtk::ListItem, ctx: &ApplyContext) {
    let description = Label::new(None);
    description.add_css_class("mimic-description");
    description.set_xalign(0.0);
    description.set_valign(Align::Center);
    description.set_hexpand(true);
    description.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let dropdown = DropDown::new(None::<StringList>, None::<&gtk::Expression>);
    dropdown.set_valign(Align::Center);
    dropdown.set_width_request(280);

    let first = Box::new(gtk::Orientation::Horizontal, 8);
    first.append(&description);
    first.append(&dropdown);

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

    let notify_item = item.clone();
    let notify_ctx = ctx.clone();
    dropdown.connect_selected_notify(move |dropdown| {
        if notify_ctx.rebinding.get() {
            return;
        }
        let index = dropdown.selected();
        if index == u32::MAX {
            return;
        }
        let Some(mime_type) = notify_item.item().and_downcast::<MimeType>() else {
            return;
        };
        let mime = mime_type.mime();
        let Some(app) = apps_for_dropdown(&mime).get(index as usize).cloned() else {
            return;
        };
        let current = gio::AppInfo::default_for_type(&mime, false);
        notify_ctx.record(&mime, vec![mime.clone()], app, current.as_ref());
    });

    item.set_child(Some(&row));
    unsafe {
        item.set_data("description", description);
        item.set_data("mime", mime);
        item.set_data("dropdown", dropdown);
    }
}

fn bind_row(item: &gtk::ListItem, ctx: &ApplyContext) {
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
    let names: Vec<&str> = name_strings.iter().map(String::as_str).collect();
    let model = StringList::new(&names);

    ctx.rebinding.set(true);
    dropdown.set_model(Some(&model));

    let selected = ctx
        .pending
        .borrow()
        .get(&mime_type.mime())
        .cloned()
        .or_else(|| gio::AppInfo::default_for_type(&mime_type.mime(), false));
    if let Some(selected) = selected
        && let Some(index) = apps.iter().position(|app| same_app(app, &selected))
    {
        dropdown.set_selected(index as u32);
    }
    ctx.rebinding.set(false);
}

fn apply_pending(ctx: &ApplyContext, store: &ListStore) {
    let entries = ctx.pending.borrow().entries();
    for (key, mimes, app) in entries {
        if let Err(error) = crate::backend::set_default_group(&mimes, &app) {
            ctx.status.set_text(&format!("Failed to set default: {error}"));
            return;
        }
        for i in 0..store.n_items() {
            if let Some(mime_type) = store.item(i).and_downcast::<MimeType>()
                && mimes.iter().any(|m| m.as_str() == mime_type.mime().as_str())
            {
                mime_type.set_default_app(Some(app.name()));
            }
        }
        ctx.pending.borrow_mut().remove(&key);
    }
    ctx.status.set_text("");
    ctx.apply.set_sensitive(!ctx.pending.borrow().is_empty());
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

    fn app(name: &str) -> gio::AppInfo {
        gio::AppInfo::create_from_commandline(name, Some(name), gio::AppInfoCreateFlags::NONE)
            .expect("commandline app info")
    }

    #[test]
    fn pending_select_adds_change() {
        let mut pending = PendingChanges::default();

        pending.select("image/png", vec!["image/png".to_string()], app("Foo"), None);

        assert!(!pending.is_empty());
        assert_eq!(pending.get("image/png").map(|app| app.name()).as_deref(), Some("Foo"));
        let entries = pending.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "image/png");
        assert_eq!(entries[0].1, vec!["image/png".to_string()]);
    }

    #[test]
    fn pending_select_same_as_current_default_discards() {
        let mut pending = PendingChanges::default();
        let foo = app("Foo");

        pending.select("image/png", vec!["image/png".to_string()], foo.clone(), Some(&foo));

        assert!(pending.is_empty());
    }

    #[test]
    fn pending_reselect_of_current_default_removes_change() {
        let mut pending = PendingChanges::default();
        let foo = app("Foo");
        let bar = app("Bar");

        pending.select("text/plain", vec!["text/plain".to_string()], bar, None);
        pending.select("text/plain", vec!["text/plain".to_string()], foo.clone(), Some(&foo));

        assert!(pending.is_empty());
    }

    #[test]
    fn pending_same_key_replaces_and_keeps_order() {
        let mut pending = PendingChanges::default();

        pending.select("a", vec!["a".to_string()], app("A1"), None);
        pending.select("b", vec!["b".to_string()], app("B"), None);
        pending.select("a", vec!["a".to_string(), "a2".to_string()], app("A2"), None);

        let entries = pending.entries();
        let keys: Vec<&str> = entries.iter().map(|entry| entry.0.as_str()).collect();
        assert_eq!(keys, vec!["a", "b"]);
        assert_eq!(pending.get("a").map(|app| app.name()).as_deref(), Some("A2"));
        assert_eq!(entries[0].1, vec!["a".to_string(), "a2".to_string()]);
    }

    #[test]
    fn pending_remove_clears_entry() {
        let mut pending = PendingChanges::default();

        pending.select("image/png", vec!["image/png".to_string()], app("Foo"), None);
        pending.remove("image/png");

        assert!(pending.is_empty());
    }

    #[test]
    fn same_app_matches_identical_commandline_apps() {
        assert!(same_app(&app("Foo"), &app("Foo")));
        assert!(!same_app(&app("Foo"), &app("Bar")));
    }
}