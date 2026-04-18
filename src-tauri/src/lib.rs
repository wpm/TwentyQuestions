mod entity;

use entity::GameSession;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

const OPEN_SETTINGS_EVENT: &str = "open-settings";

struct AppState {
    session: Mutex<Option<GameSession>>,
}

#[tauri::command]
async fn start_game(
    state: State<'_, AppState>,
    nats_url: String,
    topic: String,
    player_count: u32,
) -> Result<(), String> {
    let session = GameSession::start(&nats_url, &topic, player_count).await?;
    *state.session.lock().await = Some(session);
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
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(AppState {
            session: Mutex::new(None),
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
        .invoke_handler(tauri::generate_handler![start_game, stop_game])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
