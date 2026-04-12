use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::Serialize;
use std::{
    env,
    fs,
    io::{BufRead, Write},
    path::Path,
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
        shell_pid: tauri::async_runtime::Mutex::new(None),
    }))
}

#[derive(Serialize)]
pub struct ShellStatus {
    pub cwd: Option<String>,
    pub running_command: Option<String>,
}

fn default_shell_status() -> ShellStatus {
    ShellStatus {
        cwd: None,
        running_command: None,
    }
}

#[cfg(target_os = "linux")]
fn read_proc_stat_pgrp_tpgid(pid: u32) -> Option<(i32, i32)> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let end = stat.rfind(')')?;
    let rest = stat.get(end + 2..)?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    if fields.len() < 6 {
        return None;
    }

    let pgrp = fields.get(2)?.parse::<i32>().ok()?;
    let tpgid = fields.get(5)?.parse::<i32>().ok()?;
    Some((pgrp, tpgid))
}

#[cfg(target_os = "linux")]
fn read_proc_cmdline(pid: u32) -> Option<String> {
    let data = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    if data.is_empty() {
        return None;
    }

    let parts: Vec<String> = data
        .split(|b| *b == 0)
        .filter(|v| !v.is_empty())
        .map(|v| String::from_utf8_lossy(v).to_string())
        .collect();

    if parts.is_empty() {
        return None;
    }

    let joined = parts.join(" ");
    if joined.len() > 120 {
        return Some(format!("{}...", &joined[..117]));
    }
    Some(joined)
}

#[cfg(target_os = "linux")]
fn read_proc_comm(pid: u32) -> Option<String> {
    fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[cfg(target_os = "linux")]
fn basename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| path.to_string())
}

#[cfg(target_os = "linux")]
fn read_shell_status_linux(shell_pid: u32) -> ShellStatus {
    let cwd = fs::read_link(format!("/proc/{shell_pid}/cwd"))
        .ok()
        .map(|path| path.to_string_lossy().to_string());

    let (_, tpgid) = match read_proc_stat_pgrp_tpgid(shell_pid) {
        Some(values) => values,
        None => {
            return ShellStatus {
                cwd,
                running_command: None,
            };
        }
    };

    if tpgid <= 0 {
        return ShellStatus {
            cwd,
            running_command: None,
        };
    }

    let mut best_pid: Option<u32> = None;
    let mut shell_name = read_proc_comm(shell_pid).unwrap_or_default().to_lowercase();
    if shell_name.is_empty() {
        shell_name = basename(
            &read_proc_cmdline(shell_pid)
                .unwrap_or_default()
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string(),
        )
        .to_lowercase();
    }

    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let pid = match file_name.to_string_lossy().parse::<u32>() {
                Ok(pid) => pid,
                Err(_) => continue,
            };

            let (pgrp, _) = match read_proc_stat_pgrp_tpgid(pid) {
                Some(values) => values,
                None => continue,
            };

            if pgrp != tpgid {
                continue;
            }

            if pid == shell_pid {
                continue;
            }

            let comm = read_proc_comm(pid).unwrap_or_default().to_lowercase();
            if !comm.is_empty() && comm == shell_name {
                continue;
            }

            best_pid = Some(best_pid.map_or(pid, |best| best.max(pid)));
        }
    }

    let running_command = best_pid.and_then(read_proc_cmdline).or_else(|| {
        best_pid.and_then(read_proc_comm)
    });

    ShellStatus {
        cwd,
        running_command,
    }
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
                let pid = child.process_id();
                if let Some(shell_pid) = pid {
                    let mut session_shell_pid = session.shell_pid.lock().await;
                    *session_shell_pid = Some(shell_pid);
                }
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

#[tauri::command]
pub async fn async_get_shell_status(
    session_id: &str,
    state: State<'_, AppState>,
) -> Result<ShellStatus, String> {
    let session = get_session(&state, session_id).await?;
    let shell_pid = *session.shell_pid.lock().await;

    let Some(shell_pid) = shell_pid else {
        return Ok(default_shell_status());
    };

    #[cfg(target_os = "linux")]
    {
        return Ok(read_shell_status_linux(shell_pid));
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(default_shell_status())
    }
}

#[tauri::command]
pub async fn async_take_startup_command(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let mut startup_command = state.startup_command.lock().await;
    Ok(startup_command.take())
}
