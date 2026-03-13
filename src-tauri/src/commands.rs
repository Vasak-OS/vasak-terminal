use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::{
    env,
    io::{BufRead, Write},
    sync::Arc,
    thread,
};

use tauri::State;

use crate::structs::{AppState, TerminalSession};

fn create_terminal_session(rows: u16, cols: u16) -> Result<Arc<TerminalSession>, String> {
    let pty_pair = native_pty_system()
        .openpty(PtySize {
            rows: rows.max(1),
            cols: cols.max(1),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|err| err.to_string())?;

    let reader = pty_pair
        .master
        .try_clone_reader()
        .map_err(|err| err.to_string())?;

    let writer = pty_pair
        .master
        .take_writer()
        .map_err(|err| err.to_string())?;

    Ok(Arc::new(TerminalSession {
        pty_pair: tauri::async_runtime::Mutex::new(pty_pair),
        writer: tauri::async_runtime::Mutex::new(writer),
        reader: tauri::async_runtime::Mutex::new(std::io::BufReader::new(reader)),
        shell_started: tauri::async_runtime::Mutex::new(false),
    }))
}

async fn get_or_create_session(
    state: &State<'_, AppState>,
    session_id: &str,
    rows: u16,
    cols: u16,
) -> Result<Arc<TerminalSession>, String> {
    let mut sessions = state.sessions.lock().await;

    if let Some(session) = sessions.get(session_id) {
        return Ok(Arc::clone(session));
    }

    let new_session = create_terminal_session(rows, cols)?;
    sessions.insert(session_id.to_string(), Arc::clone(&new_session));
    Ok(new_session)
}

async fn get_session(state: &State<'_, AppState>, session_id: &str) -> Result<Arc<TerminalSession>, String> {
    let sessions = state.sessions.lock().await;
    sessions
        .get(session_id)
        .cloned()
        .ok_or_else(|| format!("Session not found: {}", session_id))
}

#[tauri::command]
// create a shell and add to it the $TERM env variable so we can use clear and other commands
pub async fn async_create_shell(
    session_id: &str,
    rows: u16,
    cols: u16,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let session = get_or_create_session(&state, session_id, rows, cols).await?;

    // This command can be invoked more than once on remount/HMR.
    // If a shell already exists for this PTY, treat it as success.
    {
        let mut shell_started = session.shell_started.lock().await;
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

        match session.pty_pair.lock().await.slave.spawn_command(cmd) {
            Ok(mut child) => {
                thread::spawn(move || {
                    let _ = child.wait();
                });
                return Ok(());
            }
            Err(err) => {
                spawn_errors.push(format!("{}: {}", shell, err));
            }
        }
    }

    let mut shell_started = session.shell_started.lock().await;
    *shell_started = false;

    Err(format!(
        "No se pudo crear la shell. Intentos: {}",
        spawn_errors.join(" | ")
    ))
}

#[tauri::command]
pub async fn async_write_to_pty(session_id: &str, data: &str, state: State<'_, AppState>) -> Result<(), String> {
    let session = get_session(&state, session_id).await?;
    let mut writer = session.writer.lock().await;
    write!(writer, "{}", data).map_err(|err| err.to_string())?;
    writer.flush().map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn async_read_from_pty(session_id: &str, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let session = get_session(&state, session_id).await?;
    let mut reader = session.reader.lock().await;
    let data = {
        let data = reader.fill_buf().map_err(|err| err.to_string())?;

        if data.len() > 0 {
            std::str::from_utf8(data)
                .map(|v| Some(v.to_string()))
                .map_err(|err| err.to_string())?
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
pub async fn async_resize_pty(
    session_id: &str,
    rows: u16,
    cols: u16,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let session = get_session(&state, session_id).await?;
    let resize_result = session
        .pty_pair
        .lock()
        .await
        .master
        .resize(PtySize {
            rows,
            cols,
            ..Default::default()
        });

    resize_result.map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn async_close_shell(session_id: &str, state: State<'_, AppState>) -> Result<(), String> {
    let mut sessions = state.sessions.lock().await;
    sessions.remove(session_id);
    Ok(())
}
