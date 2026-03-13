mod commands;
mod structs;

use crate::commands::{
    async_create_shell, async_read_from_pty, async_resize_pty, async_write_to_pty,
};
use crate::structs::AppState;
use portable_pty::{native_pty_system, PtySize};
use std::{io::BufReader, sync::Arc};
use tauri::async_runtime::Mutex as AsyncMutex;

pub fn run() {
    let pty_system = native_pty_system();

    let pty_pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();

    let reader = pty_pair.master.try_clone_reader().unwrap();
    let writer = pty_pair.master.take_writer().unwrap();

    tauri::Builder::default()
        .manage(AppState {
            pty_pair: Arc::new(AsyncMutex::new(pty_pair)),
            writer: Arc::new(AsyncMutex::new(writer)),
            reader: Arc::new(AsyncMutex::new(BufReader::new(reader))),
            shell_started: Arc::new(AsyncMutex::new(false)),
        })
        .invoke_handler(tauri::generate_handler![
            async_write_to_pty,
            async_resize_pty,
            async_create_shell,
            async_read_from_pty
        ])
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_config_manager::init())
        .plugin(tauri_plugin_vicons::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
