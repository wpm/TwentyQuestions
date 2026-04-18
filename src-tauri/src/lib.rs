mod entity;
mod language_model;
mod message;

use entity::GameSession;
use free_agent::Envelope;
use futures::StreamExt;
use message::Message;
use serde::Serialize;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_store::StoreExt;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

const OPEN_SETTINGS_EVENT: &str = "open-settings";
const CHAT_MESSAGE_EVENT: &str = "chat-message";
const STORE_PATH: &str = "settings.json";

#[derive(Clone, Serialize)]
struct ChatMessageEvent {
    sender: String,
    content: String,
}

#[tauri::command]
fn get_setting(app: AppHandle, key: String) -> Option<String> {
    let store = app.store(STORE_PATH).ok()?;
    store.get(&key).and_then(|v| v.as_str().map(String::from))
}

#[tauri::command]
fn set_setting(app: AppHandle, key: String, value: String) -> Result<(), String> {
    let store = app.store(STORE_PATH).map_err(|e| e.to_string())?;
    store.set(key, serde_json::json!(value));
    store.save().map_err(|e| e.to_string())
}

struct AppState {
    session: Mutex<Option<GameSession>>,
    chat_listener: Mutex<Option<JoinHandle<()>>>,
}

#[tauri::command]
async fn start_game(
    app: AppHandle,
    state: State<'_, AppState>,
    nats_url: String,
    topic: String,
    player_count: u32,
    model: String,
    object: String,
) -> Result<(), String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| "ANTHROPIC_API_KEY environment variable not set".to_string())?;

    let session =
        GameSession::start(&nats_url, &topic, player_count, &model, &object, &api_key).await?;
    *state.session.lock().await = Some(session);

    // Independent NATS subscriber that forwards every Message to the frontend.
    let nats = async_nats::connect(&nats_url)
        .await
        .map_err(|e| e.to_string())?;
    let mut sub = nats
        .subscribe(topic.clone())
        .await
        .map_err(|e| e.to_string())?;

    let handle = tokio::spawn(async move {
        while let Some(raw) = sub.next().await {
            if let Ok(Envelope::Message(msg)) =
                serde_json::from_slice::<Envelope<Message>>(&raw.payload)
            {
                let _ = app.emit(
                    CHAT_MESSAGE_EVENT,
                    ChatMessageEvent {
                        sender: msg.sender,
                        content: msg.content,
                    },
                );
            }
        }
    });

    *state.chat_listener.lock().await = Some(handle);

    Ok(())
}

#[tauri::command]
async fn stop_game(
    state: State<'_, AppState>,
    nats_url: String,
    topic: String,
) -> Result<(), String> {
    let mut session = state.session.lock().await.take();
    if let Some(ref mut s) = session {
        s.stop(&nats_url, &topic).await?;
    }
    if let Some(handle) = state.chat_listener.lock().await.take() {
        handle.abort();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(AppState {
            session: Mutex::new(None),
            chat_listener: Mutex::new(None),
        })
        .setup(|app| {
            let quit_item = PredefinedMenuItem::quit(app, None)?;
            let app_menu = Submenu::with_items(app, "TwentyQuestions", true, &[&quit_item])?;
            let preferences_item = MenuItem::with_id(
                app,
                "preferences",
                "Preferences...",
                true,
                Some("CmdOrCtrl+,"),
            )?;
            let file_menu = Submenu::with_items(app, "File", true, &[&preferences_item])?;
            let menu = Menu::with_items(app, &[&app_menu, &file_menu])?;
            app.set_menu(menu)?;

            let app_handle: AppHandle = app.handle().clone();
            app.on_menu_event(move |_app, event| {
                if event.id() == "preferences" {
                    let _ = app_handle.emit(OPEN_SETTINGS_EVENT, ());
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_game,
            stop_game,
            get_setting,
            set_setting
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
