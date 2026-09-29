//! Desktop shell. The whole application lives in `pw-view`; this file only hands
//! the interface a way to call it and gives the world a place to keep its saves.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use pw_view::Api;
use serde::Serialize;
use serde_json::Value;
use tauri::Manager;

/// What the interface receives when a call fails: the same `{code, message}` the browser transport reads.
#[derive(Serialize)]
struct CommandError {
    code: String,
    kind: pw_view::ErrorKind,
    message: String,
    retryable: bool,
}

/// One entry point for everything the interface asks of the simulation.
/// Runs on a blocking thread so a long call cannot stall the window.
#[tauri::command]
async fn api(state: tauri::State<'_, Api>, method: String, args: Option<Value>) -> Result<Value, CommandError> {
    let api = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || api.call(&method, args.unwrap_or(Value::Null)))
        .await
        .map_err(|e| CommandError { code: "state".into(), kind: pw_view::ErrorKind::InternalError, message: e.to_string(), retryable: false })?
        .map_err(|e| { let b = pw_view::ErrorBody::of(&e); CommandError { code: b.code, kind: b.kind, message: b.message, retryable: b.retryable } })
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(Api::new(dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![api])
        .run(tauri::generate_context!())
        .expect("the application failed to start");
}
