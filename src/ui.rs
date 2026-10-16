// Сборка интерфейса.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gio::ListStore;
use gio::prelude::*;
use gtk::prelude::*;
use gtk::{
    ActionBar, Align, Box, Button, CheckButton, DropDown, EventControllerKey, Grid, HeaderBar,
    Image, Label, ListView, MenuButton, Notebook, Popover, ScrolledWindow, SearchEntry,
    SignalListItemFactory, SingleSelection, StringList, StringObject,
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

const OTHER_LABEL: &str = "Other…";

/// Строка-заглушка, когда приложений нет: занимает позицию autoselect
/// (GtkSingleSelection всегда выбирает элемент 0 и не позволяет снять
/// выбор), чтобы клик по «Other…» менял `selected` и срабатывал.
const EMPTY_LABEL: &str = "No application found";

/// Индекс sentinel-строки «Other…» в модели из `app_count` приложений.
fn sentinel_index(app_count: usize) -> u32 {
    app_count as u32 + u32::from(app_count == 0)
}

/// Элемент модели dropdown'а: приложения и служебные строки.
#[derive(Debug, PartialEq, Eq)]
enum DropdownRow {
    Placeholder,
    App(usize),
    Other,
}

/// Порядок строк dropdown'а: при пустом списке — строка-заглушка,
/// далее приложения, в конце всегда sentinel-строка «Other…».
fn dropdown_rows(app_count: usize) -> Vec<DropdownRow> {
    if app_count == 0 {
        return vec![DropdownRow::Placeholder, DropdownRow::Other];
    }
    let mut rows: Vec<DropdownRow> = (0..app_count).map(DropdownRow::App).collect();
    rows.push(DropdownRow::Other);
    rows
}

/// Кастомная фабрика строк dropdown'а: иконка приложения + его имя.
/// Служебные строки (заглушка, «Other…») идут без иконки.
fn dropdown_factory() -> SignalListItemFactory {
    let factory = SignalListItemFactory::new();
    factory.connect_setup(|_, object| {
        let item = object
            .downcast_ref::<gtk::ListItem>()
            .expect("item is a ListItem");
        let image = Image::new();
        image.set_pixel_size(16);
        let label = Label::new(None);
        label.set_xalign(0.0);
        let row = Box::new(gtk::Orientation::Horizontal, 6);
        row.append(&image);
        row.append(&label);
        item.set_child(Some(&row));
    });
    factory.connect_bind(|_, object| {
        let item = object
            .downcast_ref::<gtk::ListItem>()
            .expect("item is a ListItem");
        let Some(row) = item.child().and_downcast::<Box>() else {
            return;
        };
        let image = row
            .first_child()
            .and_downcast::<Image>()
            .expect("icon is set up");
        let label = image
            .next_sibling()
            .and_downcast::<Label>()
            .expect("label is set up");
        let Some(item_object) = item.item() else {
            return;
        };
        if let Some(app) = item_object.downcast_ref::<gio::AppInfo>() {
            label.set_text(&app.display_name());
            match app.icon() {
                Some(icon) => image.set_from_gicon(&icon),
                None => image.set_icon_name(Some("application-x-executable")),
            }
            image.set_visible(true);
        } else if let Some(string) = item_object.downcast_ref::<StringObject>() {
            label.set_text(&string.string());
            image.set_visible(false);
        }
    });
    factory
}

/// Пересобирает модель dropdown'а по `dropdown_rows`. Возвращает индекс
/// sentinel-строки «Other…».
fn rebuild_dropdown(dropdown: &DropDown, apps: &[gio::AppInfo]) -> u32 {
    let store = ListStore::new::<glib::Object>();
    for row in dropdown_rows(apps.len()) {
        match row {
            DropdownRow::Placeholder => {
                store.append(&StringObject::new(EMPTY_LABEL));
            }
            DropdownRow::App(index) => {
                store.append(&apps[index]);
            }
            DropdownRow::Other => {
                store.append(&StringObject::new(OTHER_LABEL));
            }
        }
    }
    dropdown.set_model(Some(&store));
    sentinel_index(apps.len())
}

/// Позиция приложения в списке: уже есть — его индекс, нет — добавление
/// в конец (перед sentinel-строкой «Other…»).
fn position_or_push(apps: &mut Vec<gio::AppInfo>, app: gio::AppInfo) -> usize {
    if let Some(position) = apps.iter().position(|existing| same_app(existing, &app)) {
        return position;
    }
    apps.push(app);
    apps.len() - 1
}

/// Контекст строки для диалога «Other…»: всё, что нужно, чтобы встроить
/// выбранное приложение в список и записать pending.
struct OtherTarget {
    key: String,
    mimes: Vec<String>,
    primary: String,
    heading: String,
    dropdown: DropDown,
    apps: Rc<RefCell<Vec<gio::AppInfo>>>,
    revert: Rc<Cell<u32>>,
}

fn open_other(ctx: &ApplyContext, target: OtherTarget) {
    let window = ctx.window.clone();
    let ctx = ctx.clone();
    let heading = target.heading.clone();
    let mimes = target.mimes.clone();
    let primary = target.primary.clone();
    crate::chooser::show_other_dialog(&window, &heading, mimes, primary, move |picked| {
        finish_other(&ctx, &target, picked);
    });
}

/// Итог диалога «Other…»: подтверждённый выбор встраивается в список
/// и записывается в pending, отмена возвращает предыдущее выделение.
fn finish_other(ctx: &ApplyContext, target: &OtherTarget, picked: Option<gio::AppInfo>) {
    let OtherTarget {
        key,
        mimes,
        primary,
        dropdown,
        apps,
        revert,
        ..
    } = target;
    match picked {
        None => {
            ctx.rebinding.set(true);
            dropdown.set_selected(revert.get());
            ctx.rebinding.set(false);
        }
        Some(app) => {
            let mut list = apps.borrow().clone();
            let position = position_or_push(&mut list, app);
            let chosen = list[position].clone();

            ctx.rebinding.set(true);
            *apps.borrow_mut() = list;
            rebuild_dropdown(dropdown, &apps.borrow());
            dropdown.set_selected(position as u32);
            ctx.rebinding.set(false);
            revert.set(position as u32);

            let current = gio::AppInfo::default_for_type(primary, false);
            ctx.record(key, mimes.to_vec(), chosen, current.as_ref());
        }
    }
}

/// Общее состояние для обеих вкладок: ожидающие применения выборы,
/// кнопка Apply, флаг программного обновления dropdown'ов и родительское
/// окно для модальных диалогов.
#[derive(Clone)]
struct ApplyContext {
    pending: Rc<RefCell<PendingChanges>>,
    apply: Button,
    status: Label,
    rebinding: Rc<Cell<bool>>,
    window: gtk::ApplicationWindow,
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
        self.apply.set_sensitive(!self.pending.borrow().is_empty());
    }
}

pub fn build_ui(app: &gtk::Application) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Mimic")
        .default_width(720)
        .default_height(840)
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
        window: window.clone(),
    };

    let setup_ctx = ctx.clone();
    let factory = SignalListItemFactory::new();
    factory.connect_setup(move |_, object| {
        let item = object
            .downcast_ref::<gtk::ListItem>()
            .expect("item is a ListItem");
        setup_row(item, &setup_ctx);
    });
    let bind_ctx = ctx.clone();
    factory.connect_bind(move |_, object| {
        let item = object
            .downcast_ref::<gtk::ListItem>()
            .expect("item is a ListItem");
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
        if !search_focused.get() && !state.intersects(modifiers) && keyval.to_unicode().is_some() {
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
    ctx.apply
        .connect_clicked(move |_| apply_pending(&click_ctx, &click_store));

    let body = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
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
    grid.set_halign(Align::Center);
    grid.set_valign(Align::Start);
    grid.set_width_request(560);

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

    let centered = Box::new(gtk::Orientation::Vertical, 0);
    centered.append(&grid);

    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&centered)
        .build();
    scrolled.set_vexpand(true);

    let page = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
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
    let apps = Rc::new(RefCell::new(apps));

    let dropdown = DropDown::builder().factory(&dropdown_factory()).build();
    dropdown.set_hexpand(true);
    dropdown.set_valign(Align::Center);

    let revert = Rc::new(Cell::new(u32::MAX));
    rebuild_dropdown(&dropdown, &apps.borrow());
    let initial = gio::AppInfo::default_for_type(&group.app_mimes[0], false)
        .and_then(|default| apps.borrow().iter().position(|app| same_app(app, &default)));
    match initial {
        Some(index) => {
            dropdown.set_selected(index as u32);
            revert.set(index as u32);
        }
        None => revert.set(dropdown.selected()),
    }

    let name_label = Label::new(Some(group.name));
    name_label.add_css_class("mimic-description");
    name_label.set_xalign(0.5);
    name_label.set_valign(Align::Center);

    let key = group.name.to_string();
    let mimes = group.mimes.clone();
    let primary = group.app_mimes[0].clone();
    let heading = format!("Select default for {}", group.name);

    let notify_ctx = ctx.clone();
    let notify_apps = apps.clone();
    let notify_revert = revert.clone();
    let notify_dropdown = dropdown.clone();
    dropdown.connect_selected_notify(move |dropdown| {
        if notify_ctx.rebinding.get() {
            return;
        }
        let index = dropdown.selected();
        if index == u32::MAX {
            return;
        }
        let apps_now = notify_apps.borrow().clone();
        if index == sentinel_index(apps_now.len()) {
            open_other(
                &notify_ctx,
                OtherTarget {
                    key: key.clone(),
                    mimes: mimes.clone(),
                    primary: primary.clone(),
                    heading: heading.clone(),
                    dropdown: notify_dropdown.clone(),
                    apps: notify_apps.clone(),
                    revert: notify_revert.clone(),
                },
            );
            return;
        }
        let Some(app) = apps_now.get(index as usize).cloned() else {
            return;
        };
        notify_revert.set(index);
        let current = gio::AppInfo::default_for_type(&primary, false);
        notify_ctx.record(&key, mimes.clone(), app, current.as_ref());
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

    let dropdown = DropDown::builder().factory(&dropdown_factory()).build();
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

    let apps: Rc<RefCell<Vec<gio::AppInfo>>> = Rc::new(RefCell::new(Vec::new()));
    let revert = Rc::new(Cell::new(u32::MAX));

    let notify_item = item.clone();
    let notify_ctx = ctx.clone();
    let notify_apps = apps.clone();
    let notify_revert = revert.clone();
    let notify_dropdown = dropdown.clone();
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
        let apps_now = notify_apps.borrow().clone();
        if index == sentinel_index(apps_now.len()) {
            let heading = format!("Select default for {}", mime_type.description());
            open_other(
                &notify_ctx,
                OtherTarget {
                    key: mime.clone(),
                    mimes: vec![mime.clone()],
                    primary: mime.clone(),
                    heading,
                    dropdown: notify_dropdown.clone(),
                    apps: notify_apps.clone(),
                    revert: notify_revert.clone(),
                },
            );
            return;
        }
        let Some(app) = apps_now.get(index as usize).cloned() else {
            return;
        };
        notify_revert.set(index);
        let current = gio::AppInfo::default_for_type(&mime, false);
        notify_ctx.record(&mime, vec![mime.clone()], app, current.as_ref());
    });

    item.set_child(Some(&row));
    unsafe {
        item.set_data("description", description);
        item.set_data("mime", mime);
        item.set_data("dropdown", dropdown);
        item.set_data("apps", apps);
        item.set_data("revert", revert);
    }
}

fn bind_row(item: &gtk::ListItem, ctx: &ApplyContext) {
    let mime_type = item
        .item()
        .and_downcast::<MimeType>()
        .expect("item holds a MimeType");

    let (description, mime, dropdown, apps, revert) = unsafe {
        (
            item.data::<Label>("description")
                .expect("widgets are set up")
                .as_ref(),
            item.data::<Label>("mime")
                .expect("widgets are set up")
                .as_ref(),
            item.data::<DropDown>("dropdown")
                .expect("widgets are set up")
                .as_ref(),
            item.data::<Rc<RefCell<Vec<gio::AppInfo>>>>("apps")
                .expect("widgets are set up")
                .as_ref(),
            item.data::<Rc<Cell<u32>>>("revert")
                .expect("widgets are set up")
                .as_ref(),
        )
    };

    description.set_text(&mime_type.description());
    mime.set_text(&mime_type.mime());

    let apps_now = apps_for_dropdown(&mime_type.mime());
    let selected = ctx
        .pending
        .borrow()
        .get(&mime_type.mime())
        .cloned()
        .or_else(|| gio::AppInfo::default_for_type(&mime_type.mime(), false));
    let index =
        selected.and_then(|selected| apps_now.iter().position(|app| same_app(app, &selected)));

    ctx.rebinding.set(true);
    *apps.borrow_mut() = apps_now;
    rebuild_dropdown(dropdown, &apps.borrow());
    if let Some(position) = index {
        dropdown.set_selected(position as u32);
    }
    ctx.rebinding.set(false);
    revert.set(dropdown.selected());
}

fn apply_pending(ctx: &ApplyContext, store: &ListStore) {
    let entries = ctx.pending.borrow().entries();
    for (key, mimes, app) in entries {
        if let Err(error) = crate::backend::set_default_group(&mimes, &app) {
            ctx.status
                .set_text(&format!("Failed to set default: {error}"));
            return;
        }
        for i in 0..store.n_items() {
            if let Some(mime_type) = store.item(i).and_downcast::<MimeType>()
                && mimes
                    .iter()
                    .any(|m| m.as_str() == mime_type.mime().as_str())
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
        assert!(!matches_case(
            "text/plain",
            "Plain text",
            "text",
            "image",
            ""
        ));
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
        assert!(matches_case(
            "text/plain",
            "Plain text",
            "text",
            "",
            "plain"
        ));
    }

    #[test]
    fn search_is_case_insensitive() {
        assert!(matches_case("image/png", "PNG image", "image", "", "png"));
        assert!(matches_case(
            "text/plain",
            "Plain text",
            "text",
            "",
            "PLAIN"
        ));
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
        assert_eq!(
            pending.get("image/png").map(|app| app.name()).as_deref(),
            Some("Foo")
        );
        let entries = pending.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "image/png");
        assert_eq!(entries[0].1, vec!["image/png".to_string()]);
    }

    #[test]
    fn pending_select_same_as_current_default_discards() {
        let mut pending = PendingChanges::default();
        let foo = app("Foo");

        pending.select(
            "image/png",
            vec!["image/png".to_string()],
            foo.clone(),
            Some(&foo),
        );

        assert!(pending.is_empty());
    }

    #[test]
    fn pending_reselect_of_current_default_removes_change() {
        let mut pending = PendingChanges::default();
        let foo = app("Foo");
        let bar = app("Bar");

        pending.select("text/plain", vec!["text/plain".to_string()], bar, None);
        pending.select(
            "text/plain",
            vec!["text/plain".to_string()],
            foo.clone(),
            Some(&foo),
        );

        assert!(pending.is_empty());
    }

    #[test]
    fn pending_same_key_replaces_and_keeps_order() {
        let mut pending = PendingChanges::default();

        pending.select("a", vec!["a".to_string()], app("A1"), None);
        pending.select("b", vec!["b".to_string()], app("B"), None);
        pending.select(
            "a",
            vec!["a".to_string(), "a2".to_string()],
            app("A2"),
            None,
        );

        let entries = pending.entries();
        let keys: Vec<&str> = entries.iter().map(|entry| entry.0.as_str()).collect();
        assert_eq!(keys, vec!["a", "b"]);
        assert_eq!(
            pending.get("a").map(|app| app.name()).as_deref(),
            Some("A2")
        );
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

    #[test]
    fn position_or_push_appends_new_and_reuses_existing() {
        let mut apps = Vec::new();

        assert_eq!(position_or_push(&mut apps, app("Foo")), 0);
        assert_eq!(apps.len(), 1);

        assert_eq!(position_or_push(&mut apps, app("Foo")), 0);
        assert_eq!(apps.len(), 1, "same app must not be duplicated");

        assert_eq!(position_or_push(&mut apps, app("Bar")), 1);
        assert_eq!(apps.len(), 2);
    }

    #[test]
    fn sentinel_index_leaves_room_for_placeholder() {
        assert_eq!(
            sentinel_index(0),
            1,
            "empty list keeps 'Other…' off position 0"
        );
        assert_eq!(sentinel_index(1), 1);
        assert_eq!(sentinel_index(4), 4);
    }

    #[test]
    fn dropdown_rows_place_other_last_and_placeholder_only_when_empty() {
        use DropdownRow::{App, Other, Placeholder};

        assert_eq!(dropdown_rows(0), vec![Placeholder, Other]);
        assert_eq!(dropdown_rows(3), vec![App(0), App(1), App(2), Other]);
        assert_eq!(
            *dropdown_rows(2).last().unwrap(),
            Other,
            "sentinel row is always last"
        );
    }
}
