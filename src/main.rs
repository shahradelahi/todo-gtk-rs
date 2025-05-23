mod config;
mod utils;
mod widgets;

use adw::prelude::*;
use gtk::{gio, glib};
use std::env;

use crate::config::APP_ID;
use crate::widgets::Window;

fn main() -> glib::ExitCode {
    gio::resources_register_include!("resources.gresource").expect("Failed to register resources.");

    // println!("{}", env!("GSETTINGS_SCHEMA_DIR"));

    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_startup(on_startup);
    app.connect_activate(build_ui);

    app.run()
}

fn on_startup(app: &adw::Application) {
    let css_provider = gtk::CssProvider::new();
    css_provider.load_from_resource("/com/github/shahradelahi/Todo/style.css");

    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("Could not connect to a display."),
        &css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(app: &adw::Application) {
    let window = Window::new(app);

    app.set_accels_for_action("win.filter('All')", &["<Ctrl>a"]);
    app.set_accels_for_action("win.filter('Open')", &["<Ctrl>o"]);
    app.set_accels_for_action("win.filter('Done')", &["<Ctrl>d"]);
    app.set_accels_for_action("win.close", &["<Ctrl>W"]);

    window.present();
}
