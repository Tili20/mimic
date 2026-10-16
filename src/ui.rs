// Сборка интерфейса.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use gio::ListStore;
use gio::prelude::*;
use gtk::prelude::*;
use gtk::{
    ActionBar, Align, Box, Button, DropDown, EventControllerKey, Grid, HeaderBar, Image, Label,
    ListView, Notebook, ScrolledWindow, SearchEntry, SignalListItemFactory, SingleSelection,
    StringObject, TreeExpander, TreeListModel, TreeListRow,
};

use crate::model::{MimeGroup, MimeType};

fn matches(mime_type: &MimeType, search: &str, category: &str) -> bool {
    let search = search.to_lowercase();

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
.mimic-group-row { font-size: 12px; font-weight: 700; }
.mimic-type-row { font-size: 12px; font-family: monospace; }
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

    #[cfg(test)]
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

    let selection = SingleSelection::new(Some(build_advanced_tree(&store, "")));
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

    let factory = SignalListItemFactory::new();
    factory.connect_setup(|_, object| {
        let item = object
            .downcast_ref::<gtk::ListItem>()
            .expect("item is a ListItem");
        setup_advanced_row(item);
    });
    factory.connect_bind(|_, object| {
        let item = object
            .downcast_ref::<gtk::ListItem>()
            .expect("item is a ListItem");
        bind_advanced_row(item);
    });
    factory.connect_unbind(|_, object| {
        let item = object
            .downcast_ref::<gtk::ListItem>()
            .expect("item is a ListItem");
        unbind_advanced_row(item);
    });

    let list_view = ListView::new(Some(selection.clone()), Some(factory));
    list_view.set_vexpand(true);

    let activate_ctx = ctx.clone();
    let activate_selection = selection.clone();
    list_view.connect_activate(move |_, position| {
        let Some(row) = activate_selection
            .item(position)
            .and_downcast::<TreeListRow>()
        else {
            return;
        };
        let Some(item) = row.item() else {
            return;
        };
        if let Some(mime_type) = item.downcast_ref::<MimeType>() {
            open_chooser_for(&activate_ctx, mime_type);
        } else {
            row.set_expanded(!row.is_expanded());
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

    let rebuild: Rc<dyn Fn()> = {
        let store = store.clone();
        let selection = selection.clone();
        let search = search.clone();
        Rc::new(move || {
            let tree = build_advanced_tree(&store, &search.text());
            selection.set_model(Some(&tree));
        })
    };

    let search_refresh = rebuild.clone();
    search.connect_search_changed(move |_| search_refresh());

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

    let controls = Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .build();
    controls.append(&search);

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

    let header = HeaderBar::new();
    header.set_title_widget(Some(&gtk::Label::new(Some("Mimic"))));

    let action_bar = ActionBar::new();
    action_bar.pack_start(&status);
    action_bar.pack_end(&ctx.apply);

    let click_ctx = ctx.clone();
    let click_store = store.clone();
    let click_refresh = rebuild.clone();
    ctx.apply.connect_clicked(move |_| {
        apply_pending(&click_ctx, &click_store);
        click_refresh();
    });

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

/// Группирует типы по категориям для дерева вкладки Advanced с учётом
/// поиска. Категории сохраняют порядок появления.
fn advanced_groups(store: &ListStore, search: &str) -> Vec<(String, ListStore)> {
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, ListStore> = HashMap::new();

    for index in 0..store.n_items() {
        let Some(mime_type) = store.item(index).and_downcast::<MimeType>() else {
            continue;
        };
        if !matches(&mime_type, search, "") {
            continue;
        }
        let category = mime_type.category().to_string();
        let children = groups.entry(category.clone()).or_insert_with(|| {
            order.push(category.clone());
            ListStore::new::<MimeType>()
        });
        children.append(&mime_type);
    }

    order
        .into_iter()
        .map(|category| {
            let children = groups.remove(&category).expect("category model exists");
            (category, children)
        })
        .collect()
}

/// Строит дерево «категория → типы». При непустом поиске узлы раскрыты,
/// чтобы найденные типы были видны сразу.
fn build_advanced_tree(store: &ListStore, search: &str) -> TreeListModel {
    let autoexpand = !search.trim().is_empty();
    let groups = advanced_groups(store, search);
    let children: HashMap<String, ListStore> = groups
        .iter()
        .map(|(category, types)| (category.clone(), types.clone()))
        .collect();

    let root = ListStore::new::<MimeGroup>();
    for (category, _) in &groups {
        root.append(&MimeGroup::new(category));
    }

    TreeListModel::new(root, false, autoexpand, move |object| {
        let group = object.downcast_ref::<MimeGroup>()?;
        children
            .get(&group.name())
            .cloned()
            .map(|store| store.upcast())
    })
}

/// Иконка типа: значок приложения по умолчанию, иначе значок самого типа.
fn set_type_icon(icon: &Image, mime_type: &MimeType) {
    let default_icon =
        gio::AppInfo::default_for_type(&mime_type.mime(), false).and_then(|app| app.icon());
    match default_icon {
        Some(gicon) => icon.set_from_gicon(&gicon),
        None => icon.set_from_gicon(&gio::content_type_get_icon(&mime_type.mime())),
    }
}

/// Двойной клик по типу: выбор приложения по умолчанию через диалог «Other…»,
/// результат записывается в pending.
fn open_chooser_for(ctx: &ApplyContext, mime_type: &MimeType) {
    let window = ctx.window.clone();
    let ctx = ctx.clone();
    let mime = mime_type.mime();
    let heading = format!("Select default for {}", mime_type.description());
    crate::chooser::show_other_dialog(
        &window,
        &heading,
        vec![mime.clone()],
        mime.clone(),
        move |picked| {
            let Some(app) = picked else {
                return;
            };
            let current = gio::AppInfo::default_for_type(&mime, false);
            ctx.record(&mime, vec![mime.clone()], app, current.as_ref());
        },
    );
}

fn setup_advanced_row(item: &gtk::ListItem) {
    let expander = TreeExpander::new();

    let icon = Image::new();
    icon.set_pixel_size(12);
    icon.set_valign(Align::Center);

    let label = Label::new(None);
    label.set_xalign(0.0);
    label.set_hexpand(true);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let content = Box::new(gtk::Orientation::Horizontal, 6);
    content.append(&icon);
    content.append(&label);

    expander.set_child(Some(&content));
    item.set_child(Some(&expander));

    unsafe {
        item.set_data("advanced-icon", icon);
        item.set_data("advanced-label", label);
    }
}

fn bind_advanced_row(item: &gtk::ListItem) {
    let Some(row) = item.item().and_downcast::<TreeListRow>() else {
        return;
    };
    let Some(expander) = item.child().and_downcast::<TreeExpander>() else {
        return;
    };
    expander.set_list_row(Some(&row));

    let (icon, label) = unsafe {
        (
            item.data::<Image>("advanced-icon")
                .expect("icon is set up")
                .as_ref(),
            item.data::<Label>("advanced-label")
                .expect("label is set up")
                .as_ref(),
        )
    };

    match row.item() {
        Some(object) if object.is::<MimeGroup>() => {
            let group = object.downcast_ref::<MimeGroup>().expect("group object");
            icon.set_visible(false);
            label.remove_css_class("mimic-type-row");
            label.add_css_class("mimic-group-row");
            label.set_text(&group.name());
            label.set_tooltip_text(None);
        }
        Some(object) => {
            let mime_type = object.downcast_ref::<MimeType>().expect("mime type object");
            icon.set_visible(true);
            label.remove_css_class("mimic-group-row");
            label.add_css_class("mimic-type-row");
            label.set_text(&mime_type.mime());
            label.set_tooltip_text(Some(&mime_type.description()));
            set_type_icon(icon, mime_type);
        }
        None => {
            icon.set_visible(false);
            label.set_text("");
            label.set_tooltip_text(None);
        }
    }
}

fn unbind_advanced_row(item: &gtk::ListItem) {
    let Some(expander) = item.child().and_downcast::<TreeExpander>() else {
        return;
    };
    expander.set_list_row(None);
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
        matches(&mime_type, search, filter_category)
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

    fn mime_store(types: &[(&str, &str, &str)]) -> ListStore {
        let store = ListStore::new::<MimeType>();
        for (mime, description, category) in types {
            store.append(&MimeType::new(mime, description, category, None));
        }
        store
    }

    #[test]
    fn advanced_groups_groups_by_category_in_order() {
        let store = mime_store(&[
            ("image/png", "PNG image", "image"),
            ("text/plain", "Plain text", "text"),
            ("image/jpeg", "JPEG image", "image"),
        ]);

        let groups = advanced_groups(&store, "");

        let categories: Vec<&str> = groups.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(categories, vec!["image", "text"]);
        assert_eq!(groups[0].1.n_items(), 2);
        assert_eq!(groups[1].1.n_items(), 1);
    }

    #[test]
    fn advanced_groups_applies_search() {
        let store = mime_store(&[
            ("image/png", "PNG image", "image"),
            ("text/plain", "Plain text", "text"),
        ]);

        let groups = advanced_groups(&store, "png");

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, "image");
        assert_eq!(groups[0].1.n_items(), 1);
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
