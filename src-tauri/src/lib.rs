mod commands;
mod structs;

use crate::commands::{
    async_close_shell, async_create_shell, async_read_from_pty, async_resize_pty, async_write_to_pty,
};
use crate::structs::AppState;
use std::{collections::HashMap, sync::Arc};
use tauri::async_runtime::Mutex as AsyncMutex;

pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            sessions: Arc::new(AsyncMutex::new(HashMap::new())),
        })
        .invoke_handler(tauri::generate_handler![
            async_write_to_pty,
            async_resize_pty,
            async_create_shell,
            async_read_from_pty,
            async_close_shell
        ])
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_config_manager::init())
        .plugin(tauri_plugin_vicons::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
