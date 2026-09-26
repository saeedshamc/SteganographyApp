//! Tauri desktop shell for Open Stego.
//! Hide/Extract UI is wired in stages 7–8.

use stego_core::VERSION;

#[tauri::command]
fn app_version() -> String {
    VERSION.to_string()
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Open Stego {VERSION} — hello, {name}!")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, app_version])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
