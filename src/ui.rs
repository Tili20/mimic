// Сборка интерфейса.

use gio::ListStore;
use gtk::prelude::*;
use gtk::{
    Align, Box, DropDown, HeaderBar, Label, ListView, ScrolledWindow, SearchEntry,
    SignalListItemFactory, SingleSelection, StringList,
};

use crate::model::MimeType;

const CSS: &str = "
.mimic-description { font-weight: 700; }
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

    let selection = SingleSelection::new(Some(store));
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

    let list_view = ListView::new(Some(selection), Some(factory));
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

    let root = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    root.append(&controls);
    root.append(&scrolled);

    let header = HeaderBar::new();
    header.set_title_widget(Some(&gtk::Label::new(Some("Mimic"))));

    window.set_titlebar(Some(&header));
    window.set_child(Some(&root));
    window.present();
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
}