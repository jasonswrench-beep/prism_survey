//! The Prism Survey desktop app: a window around the React interface, with one door to the engine.
//!
//! The interface calls `invoke('call', { cmd, args })`; this file hands it to the same command layer the tests use, so
//! what ships is what was tested. Nothing here decides anything.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use prismsurvey_api::{Api, ApiError};
use serde_json::Value;
use std::sync::Mutex;
use tauri::State;

struct Engine(Mutex<Api>);

#[tauri::command]
async fn call(cmd: String, args: Option<Value>, engine: State<'_, Engine>) -> Result<Value, ApiError> {
    let mut api = engine.0.lock().map_err(|_| ApiError::new("internal", "the engine stopped after an earlier failure; close and reopen Prism Survey"))?;
    api.call(&cmd, args.unwrap_or(Value::Null))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Engine(Mutex::new(Api::from_env())))
        .invoke_handler(tauri::generate_handler![call])
        .run(tauri::generate_context!())
        .expect("Prism Survey could not start its window");
}
