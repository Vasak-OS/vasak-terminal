mod commands;
mod structs;

use crate::commands::{
    async_close_shell, async_confirm_startup_command_delivered, async_create_shell,
    async_get_shell_status, async_read_from_pty, async_resize_pty, async_take_startup_command,
    async_write_to_pty,
};
use crate::structs::AppState;
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    sync::Arc,
};
use tauri::async_runtime::Mutex as AsyncMutex;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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

    if let Ok(mut file) = File::open(path) {
        let mut first_two_bytes = [0_u8; 2];
        if file.read_exact(&mut first_two_bytes).is_ok() {
            return first_two_bytes == *b"#!";
        }
    }

    false
}

fn read_shebang_interpreter(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    let bytes_read = reader.read_line(&mut first_line).ok()?;

    if bytes_read == 0 || !first_line.starts_with("#!") {
        return None;
    }

    let interpreter = first_line
        .trim_end_matches(['\n', '\r'])
        .trim_start_matches("#!")
        .trim();

    if interpreter.is_empty() {
        None
    } else {
        Some(interpreter.to_string())
    }
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    false
}

fn fallback_interpreter_from_extension(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "sh" | "command" => Some("sh"),
        "bash" => Some("bash"),
        "zsh" => Some("zsh"),
        "ksh" => Some("ksh"),
        "fish" => Some("fish"),
        "py" => Some("python3"),
        "rb" => Some("ruby"),
        "pl" => Some("perl"),
        _ => None,
    }
}

fn build_script_command(path: &Path) -> Option<String> {
    let escaped_file = shell_escape_single_quoted(&path.to_string_lossy());

    if let Some(interpreter) = read_shebang_interpreter(path) {
        return Some(format!("{interpreter} {escaped_file}\n"));
    }

    if is_executable(path) {
        return Some(format!("{escaped_file}\n"));
    }

    if let Some(interpreter) = fallback_interpreter_from_extension(path) {
        return Some(format!("{interpreter} {escaped_file}\n"));
    }

    None
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

    build_script_command(&launch_target)
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
            async_take_startup_command,
            async_confirm_startup_command_delivered
        ])
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_config_manager::init())
        .plugin(tauri_plugin_vicons::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
