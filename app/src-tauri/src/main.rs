//! The Prism Survey desktop app: a window around the React interface, with one door to the engine.
//!
//! The interface calls `invoke('call', { cmd, args })`; this file hands it to the same command layer the tests use, so
//! what ships is what was tested. Nothing here decides anything. The `update.` commands are the window's own (the Help
//! tab's Check for Updates, see `updates.rs`) and never reach the engine, which makes no network connection for them.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod updates;

use prismsurvey_api::{Api, ApiError};
use serde_json::Value;
use std::sync::Mutex;
use tauri::State;

struct Engine(Mutex<Api>);

#[tauri::command]
async fn call(app: tauri::AppHandle, cmd: String, args: Option<Value>, engine: State<'_, Engine>) -> Result<Value, ApiError> {
    if let Some(what) = cmd.strip_prefix("update.") {
        let feed = updates::Feed::built_in();
        let back = updates::Feed::rollback();
        let answer = match what {
            "state" => Ok(updates::state(&app, &feed, &back)),
            "check" => updates::check(&app, &feed).await,
            "install" => updates::install(&app, &feed).await,
            "rollbackCheck" => updates::rollback(&app, &back, false).await,
            "rollback" => updates::rollback(&app, &back, true).await,
            other => Err(format!("There is no command update.{other}.")),
        };
        return answer.map_err(|m| ApiError::new("update_failed", m));
    }
    let mut api = engine.0.lock().map_err(|_| ApiError::new("internal", "the engine stopped after an earlier failure; close and reopen Prism Survey"))?;
    api.call(&cmd, args.unwrap_or(Value::Null))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Engine(Mutex::new(Api::from_env())))
        .invoke_handler(tauri::generate_handler![call])
        .run(tauri::generate_context!())
        .expect("Prism Survey could not start its window");
}
