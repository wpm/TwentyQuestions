use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::chat_panel::ChatPanel;
use crate::settings_modal::{
    SettingsModal, DEFAULT_MODEL, DEFAULT_NATS_URL, MODEL_KEY, NATS_URL_KEY,
};
use crate::store::{load_store, store_get_string, STORE_PATH};
use crate::toolbar::{Theme, Toolbar, THEME_KEY};

pub const OPEN_SETTINGS_EVENT: &str = "open-settings";

#[component]
pub fn App() -> impl IntoView {
    let object = RwSignal::new(String::new());
    let players = RwSignal::new(1u32);
    let theme = RwSignal::new(Theme::Light);
    let show_settings = RwSignal::new(false);
    let nats_url = RwSignal::new(DEFAULT_NATS_URL.to_string());
    let model = RwSignal::new(DEFAULT_MODEL.to_string());

    // Load all persisted settings in a single store session
    spawn_local(async move {
        if let Some(rid) = load_store(STORE_PATH).await {
            if let Some(saved) = store_get_string(rid, THEME_KEY).await {
                if let Some(t) = Theme::from_str(&saved) {
                    theme.set(t);
                    Theme::apply(t);
                }
            } else {
                let t = Theme::from_system();
                theme.set(t);
                Theme::apply(t);
            }
            if let Some(url) = store_get_string(rid, NATS_URL_KEY).await {
                nats_url.set(url);
            }
            if let Some(m) = store_get_string(rid, MODEL_KEY).await {
                model.set(m);
            }
        } else {
            let t = Theme::from_system();
            theme.set(t);
            Theme::apply(t);
        }
    });

    // Register the Tauri menu event listener (app-lifetime, intentional forget())
    let handler = Closure::wrap(Box::new(move || {
        show_settings.set(true);
    }) as Box<dyn Fn()>);
    let window = web_sys::window().unwrap();
    if let Ok(tauri) = js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("__TAURI__"))
    {
        if let Ok(event_mod) =
            js_sys::Reflect::get(&tauri, &wasm_bindgen::JsValue::from_str("event"))
        {
            if let Ok(listen) =
                js_sys::Reflect::get(&event_mod, &wasm_bindgen::JsValue::from_str("listen"))
            {
                let listen: js_sys::Function = listen.unchecked_into();
                let _ = listen.call2(
                    &wasm_bindgen::JsValue::NULL,
                    &wasm_bindgen::JsValue::from_str(OPEN_SETTINGS_EVENT),
                    handler.as_ref().unchecked_ref(),
                );
                handler.forget();
            }
        }
    }

    let on_settings = Callback::new(move |()| show_settings.set(true));

    view! {
        <main class="flex flex-col h-screen bg-gray-50 dark:bg-gray-950 text-gray-900 dark:text-gray-100">
            <Toolbar object=object players=players theme=theme on_settings=on_settings />
            <div class="flex flex-col flex-1 overflow-hidden p-4">
                <ChatPanel messages=vec![] />
            </div>
            <SettingsModal show=show_settings nats_url=nats_url model=model />
        </main>
    }
}
