mod app;
mod chat_panel;
mod nats;
mod settings_modal;
mod store;
mod tauri_commands;
mod toolbar;

use app::*;
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! {
            <App/>
        }
    })
}
