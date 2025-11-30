//! BlockAndFocus Tauri Application Library
//!
//! This module contains the Tauri commands and state management for the menu bar app.

mod commands;
mod ipc_client;

use ipc_client::IpcClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};
use tokio::sync::Mutex;

/// Application state shared across Tauri commands
pub struct AppState {
    pub client: Arc<Mutex<IpcClient>>,
}

/// Status information returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusInfo {
    pub blocking_active: bool,
    pub schedule_enabled: bool,
    pub schedule_active: bool,
    pub bypass_active: bool,
    pub bypass_remaining_seconds: Option<i64>,
    pub blocked_count: u64,
    pub daemon_connected: bool,
}

/// Quiz information for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizInfo {
    pub challenge_id: String,
    pub questions: Vec<String>,
    pub expires_at: i64,
}

/// Result of quiz submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizResult {
    pub success: bool,
    pub message: String,
}

// ============================================================================
// Tray Icon Setup
// ============================================================================

/// Set up the system tray icon and menu
pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show", "Show BlockAndFocus", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

// ============================================================================
// App Runner
// ============================================================================

/// Run the Tauri application
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            client: Arc::new(Mutex::new(IpcClient::new())),
        })
        .setup(|app| {
            setup_tray(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::get_blocklist,
            commands::add_domain,
            commands::remove_domain,
            commands::get_schedule,
            commands::set_schedule_enabled,
            commands::request_bypass,
            commands::submit_quiz_answers,
            commands::cancel_bypass,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
