use portable_pty::{CommandBuilder, PtySize};
use std::{
    env,
    io::{BufRead, Write},
    process::exit,
    thread::{self},
};

use tauri::State;

use crate::structs::AppState;

#[tauri::command]
// create a shell and add to it the $TERM env variable so we can use clear and other commands
pub async fn async_create_shell(state: State<'_, AppState>) -> Result<(), String> {
    // This command can be invoked more than once on remount/HMR.
    // If a shell already exists for this PTY, treat it as success.
    {
        let mut shell_started = state.shell_started.lock().await;
        if *shell_started {
            return Ok(());
        }
        *shell_started = true;
    }

    let mut candidates: Vec<String> = Vec::new();

    if let Ok(shell_env) = env::var("SHELL") {
        let shell_env = shell_env.trim();
        if !shell_env.is_empty() {
            candidates.push(shell_env.to_string());
        }
    }

    for shell in ["/bin/bash", "/bin/sh", "bash", "sh"] {
        if !candidates.iter().any(|s| s == shell) {
            candidates.push(shell.to_string());
        }
    }

    let mut spawn_errors: Vec<String> = Vec::new();

    for shell in candidates {
        let mut cmd = CommandBuilder::new(shell.as_str());
        cmd.env("TERM", "xterm-256color");

        match state.pty_pair.lock().await.slave.spawn_command(cmd) {
            Ok(mut child) => {
                thread::spawn(move || {
                    let status = child.wait().unwrap();
                    exit(status.exit_code() as i32)
                });
                return Ok(());
            }
            Err(err) => {
                spawn_errors.push(format!("{}: {}", shell, err));
            }
        }
    }

    let mut shell_started = state.shell_started.lock().await;
    *shell_started = false;

    Err(format!(
        "No se pudo crear la shell. Intentos: {}",
        spawn_errors.join(" | ")
    ))
}

#[tauri::command]
pub async fn async_write_to_pty(data: &str, state: State<'_, AppState>) -> Result<(), ()> {
    let mut writer = state.writer.lock().await;
    write!(writer, "{}", data).map_err(|_| ())?;
    writer.flush().map_err(|_| ())
}

#[tauri::command]
pub async fn async_read_from_pty(state: State<'_, AppState>) -> Result<Option<String>, ()> {
    let mut reader = state.reader.lock().await;
    let data = {
        let data = reader.fill_buf().map_err(|_| ())?;

        if data.len() > 0 {
            std::str::from_utf8(data)
                .map(|v| Some(v.to_string()))
                .map_err(|_| ())?
        } else {
            None
        }
    };

    if let Some(data) = &data {
        reader.consume(data.len());
    }

    Ok(data)
}

#[tauri::command]
pub async fn async_resize_pty(rows: u16, cols: u16, state: State<'_, AppState>) -> Result<(), ()> {
    state
        .pty_pair
        .lock()
        .await
        .master
        .resize(PtySize {
            rows,
            cols,
            ..Default::default()
        })
        .map_err(|_| ())
}
