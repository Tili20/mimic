use gtk::prelude::*;
use gtk::{Application, glib};

mod backend;
mod chooser;
mod model;
mod ui;

const APP_ID: &str = "org.mimic.Mimic";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(ui::build_ui);
    app.run()
}