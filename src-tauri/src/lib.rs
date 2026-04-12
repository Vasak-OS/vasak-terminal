mod commands;
mod structs;

use crate::commands::{
    async_close_shell, async_create_shell, async_get_shell_status, async_read_from_pty,
    async_resize_pty, async_take_startup_command, async_write_to_pty,
};
use crate::structs::AppState;
use std::{
    collections::HashMap,
    env,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tauri::async_runtime::Mutex as AsyncMutex;

fn shell_escape_single_quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn is_probable_script(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|v| v.to_str()) {
        let ext = ext.to_ascii_lowercase();
        if matches!(ext.as_str(), "sh" | "bash" | "zsh" | "ksh" | "fish" | "command") {
            return true;
        }
    }

    if let Ok(bytes) = fs::read(path) {
        return bytes.starts_with(b"#!");
    }

    false
}

fn resolve_startup_command_from_args() -> Option<String> {
    let launch_target = env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .find(|arg| arg.exists())?;

    if launch_target.is_dir() {
        let escaped_dir = shell_escape_single_quoted(&launch_target.to_string_lossy());
        return Some(format!("cd {escaped_dir}\n"));
    }

    if !launch_target.is_file() || !is_probable_script(&launch_target) {
        return None;
    }

    let escaped_file = shell_escape_single_quoted(&launch_target.to_string_lossy());
    Some(format!("bash {escaped_file}\n"))
}

pub fn run() {
    let startup_command = resolve_startup_command_from_args();

    tauri::Builder::default()
        .manage(AppState {
            sessions: Arc::new(AsyncMutex::new(HashMap::new())),
            startup_command: Arc::new(AsyncMutex::new(startup_command)),
        })
        .invoke_handler(tauri::generate_handler![
            async_write_to_pty,
            async_resize_pty,
            async_create_shell,
            async_read_from_pty,
            async_close_shell,
            async_get_shell_status,
            async_take_startup_command
        ])
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_config_manager::init())
        .plugin(tauri_plugin_vicons::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
