use tauri::menu::{Menu, MenuItem, Submenu};
use tauri::{AppHandle, Emitter};

const OPEN_SETTINGS_EVENT: &str = "open-settings";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let preferences_item = MenuItem::with_id(
                app,
                "preferences",
                "Preferences...",
                true,
                Some("CmdOrCtrl+,"),
            )?;
            let file_menu = Submenu::with_items(app, "File", true, &[&preferences_item])?;
            let menu = Menu::with_items(app, &[&file_menu])?;
            app.set_menu(menu)?;

            let app_handle: AppHandle = app.handle().clone();
            app.on_menu_event(move |_app, event| {
                if event.id() == "preferences" {
                    let _ = app_handle.emit(OPEN_SETTINGS_EVENT, ());
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
