use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::chat_panel::{ChatMessage, ChatPanel};
use crate::nats::{DEFAULT_TOPIC, TOPIC_KEY};
use crate::settings_modal::{
    SettingsModal, DEFAULT_MODEL, DEFAULT_NATS_URL, MODEL_KEY, NATS_URL_KEY,
};
use crate::store::{get_setting, set_setting};
use crate::tauri_commands::{invoke_start_game, invoke_stop_game};
use crate::toolbar::{GameState, Theme, Toolbar, THEME_KEY};

pub const OPEN_SETTINGS_EVENT: &str = "open-settings";
const CHAT_MESSAGE_EVENT: &str = "chat-message";

#[component]
pub fn App() -> impl IntoView {
    let object = RwSignal::new(String::new());
    let players = RwSignal::new(1u32);
    let theme = RwSignal::new(Theme::Light);
    let show_settings = RwSignal::new(false);
    let nats_url = RwSignal::new(DEFAULT_NATS_URL.to_string());
    let model = RwSignal::new(DEFAULT_MODEL.to_string());
    let topic = RwSignal::new(DEFAULT_TOPIC.to_string());
    let game_state = RwSignal::new(GameState::Idle);
    let messages: RwSignal<Vec<ChatMessage>> = RwSignal::new(Vec::new());

    spawn_local(async move {
        if let Some(saved) = get_setting(THEME_KEY).await {
            if let Some(t) = Theme::from_str(&saved) {
                theme.set(t);
                Theme::apply(t);
            }
        } else {
            let t = Theme::from_system();
            theme.set(t);
            Theme::apply(t);
        }
        if let Some(url) = get_setting(NATS_URL_KEY).await {
            nats_url.set(url);
        }
        if let Some(m) = get_setting(MODEL_KEY).await {
            model.set(m);
        }
        if let Some(t) = get_setting(TOPIC_KEY).await {
            topic.set(t);
        }
    });

    // Register Tauri menu event listener (app-lifetime, intentional forget()).
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
                let listen_fn: js_sys::Function = listen.unchecked_into();

                // Menu event
                let _ = listen_fn.call2(
                    &wasm_bindgen::JsValue::NULL,
                    &wasm_bindgen::JsValue::from_str(OPEN_SETTINGS_EVENT),
                    handler.as_ref().unchecked_ref(),
                );
                handler.forget();

                // Chat message event
                let chat_handler = Closure::wrap(Box::new(move |event: wasm_bindgen::JsValue| {
                    let payload = js_sys::Reflect::get(&event, &"payload".into()).ok();
                    let sender = payload
                        .as_ref()
                        .and_then(|p| js_sys::Reflect::get(p, &"sender".into()).ok())
                        .and_then(|v| v.as_string())
                        .unwrap_or_default();
                    let content = payload
                        .as_ref()
                        .and_then(|p| js_sys::Reflect::get(p, &"content".into()).ok())
                        .and_then(|v| v.as_string())
                        .unwrap_or_default();
                    messages.update(|msgs| msgs.push(ChatMessage::new(sender, content)));
                })
                    as Box<dyn Fn(wasm_bindgen::JsValue)>);
                let _ = listen_fn.call2(
                    &wasm_bindgen::JsValue::NULL,
                    &wasm_bindgen::JsValue::from_str(CHAT_MESSAGE_EVENT),
                    chat_handler.as_ref().unchecked_ref(),
                );
                chat_handler.forget();
            }
        }
    }

    let on_settings = Callback::new(move |()| show_settings.set(true));

    let on_start = Callback::new(move |()| {
        let url = nats_url.get();
        let t = topic.get();
        let p = players.get();
        let m = model.get();
        let o = object.get();
        messages.set(Vec::new());
        game_state.set(GameState::Running);
        spawn_local(async move {
            if let Err(e) = invoke_start_game(&url, &t, p, &m, &o).await {
                web_sys::console::error_1(&format!("start_game failed: {e}").into());
                game_state.set(GameState::Idle);
            }
        });
    });

    let on_stop = Callback::new(move |()| {
        let url = nats_url.get();
        let t = topic.get();
        game_state.set(GameState::Idle);
        spawn_local(async move {
            if let Err(e) = invoke_stop_game(&url, &t).await {
                web_sys::console::error_1(&format!("stop_game failed: {e}").into());
            }
        });
    });

    let on_topic_change = Callback::new(move |new_topic: String| {
        topic.set(new_topic.clone());
        spawn_local(async move {
            set_setting(TOPIC_KEY, &new_topic).await;
        });
    });

    view! {
        <main class="flex flex-col h-screen bg-gray-50 dark:bg-gray-950 text-gray-900 dark:text-gray-100">
            <Toolbar object=object players=players theme=theme on_settings=on_settings topic=topic on_topic_change=on_topic_change game_state=game_state on_start=on_start on_stop=on_stop />
            <div class="flex flex-col flex-1 overflow-hidden p-4">
                <ChatPanel messages=messages />
            </div>
            <SettingsModal show=show_settings nats_url=nats_url model=model />
        </main>
    }
}
