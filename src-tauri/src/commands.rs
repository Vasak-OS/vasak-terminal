use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::Serialize;
use std::{
    env,
    fs,
    io::{Read, Write},
    path::Path,
    sync::Arc,
    thread,
};

use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Emitter, State};

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
        reader: tauri::async_runtime::Mutex::new(Some(reader)),
        shell_started: tauri::async_runtime::Mutex::new(false),
        shell_pid: tauri::async_runtime::Mutex::new(None),
        output: std::sync::Mutex::new(None),
    }))
}

/// Un hilo hace lectura bloqueante del PTY y manda los bytes tal cual por el
/// canal, y al final un evento de salida.
///
/// **Los bytes van crudos, sin base64.** Antes se codificaban acá y el frontend
/// los decodificaba con `atob` más un bucle por byte en JavaScript. Eso costaba
/// tres cosas: un tercio más de bytes en el IPC (8 KB de salida viajaban como
/// 10,9 KB), la serialización JSON de esa cadena en cada evento, y un
/// decodificado once veces más lento que el nativo —medido: 267 MB/s contra
/// 3117 MB/s—. Con un canal de Tauri los trozos de más de 1 KB viajan como
/// binario real por la vía de `fetch`, sin codificar nada.
///
/// Se sigue mandando el trozo entero y no texto decodificado: xterm.js
/// necesita los bytes para rearmar secuencias UTF-8 partidas entre trozos.
fn spawn_reader(
    app: AppHandle,
    session: Arc<TerminalSession>,
    session_id: String,
    mut reader: Box<dyn Read + Send>,
) {
    thread::spawn(move || {
        let exit_event = format!("pty://exit/{session_id}");
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break, // EOF: la shell cerró el PTY
                Ok(n) => {
                    // El canal se lee de la sesión en cada trozo, no se captura
                    // una vez: al remontarse la vista hay un canal nuevo y hay
                    // que escribir en ése. El candado está sin contención, así
                    // que cuesta nanosegundos por trozo.
                    let canal = match session.output.lock() {
                        Ok(guard) => guard.clone(),
                        Err(envenenado) => envenenado.into_inner().clone(),
                    };
                    let Some(canal) = canal else {
                        // Nadie escuchando todavía. Se descarta este trozo en
                        // lugar de acumularlo: la alternativa es un búfer que
                        // crece sin techo si la vista nunca vuelve.
                        continue;
                    };
                    if canal.send(InvokeResponseBody::Raw(buf[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        let _ = app.emit(&exit_event, ());
    });
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
    parse_stat_pgrp_tpgid(&stat)
}

/// Saca el grupo de procesos y el grupo en primer plano de `/proc/PID/stat`.
///
/// Separado de la lectura para poder probarlo, porque el formato tiene una
/// trampa: el segundo campo es el nombre del ejecutable entre paréntesis, y
/// **puede contener espacios y paréntesis**. Por eso se busca el *último* `)` y
/// no se parte por espacios desde el principio; un proceso llamado
/// `(raro) cosa)` rompería cualquier otra lectura.
#[cfg(target_os = "linux")]
fn parse_stat_pgrp_tpgid(stat: &str) -> Option<(i32, i32)> {
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
    parse_cmdline(&data)
}

/// Largo máximo del comando que se muestra en la pestaña.
#[cfg(target_os = "linux")]
const CMDLINE_MAX: usize = 120;

/// Arma el comando visible a partir de `/proc/PID/cmdline`, que viene con los
/// argumentos separados por bytes nulos.
///
/// El recorte es por **caracteres y no por bytes**. Cortaba con
/// `&joined[..117]`, que paniquea si ese byte cae en medio de un carácter
/// UTF-8: bastaba un comando de 116 caracteres seguido de una `ñ` —una ruta con
/// acento, algo cotidiano— para tirar el hilo que atiende el estado de la
/// pestaña.
#[cfg(target_os = "linux")]
fn parse_cmdline(data: &[u8]) -> Option<String> {
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
    if joined.chars().count() > CMDLINE_MAX {
        let recortado: String = joined.chars().take(CMDLINE_MAX - 3).collect();
        return Some(format!("{recortado}..."));
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
fn read_shell_status_linux(shell_pid: u32) -> ShellStatus {
    let cwd = fs::read_link(format!("/proc/{shell_pid}/cwd"))
        .ok()
        .map(|path| path.to_string_lossy().to_string());

    let (pgrp, tpgid) = match read_proc_stat_pgrp_tpgid(shell_pid) {
        Some(values) => values,
        None => return ShellStatus { cwd, running_command: None },
    };

    // The controlling terminal's foreground process-group id (tpgid) equals the
    // pid of that group's leader. When it matches the shell's own group the
    // shell itself is in the foreground (no command running). Otherwise read
    // the foreground command directly — no full /proc scan needed.
    let running_command = if tpgid > 0 && tpgid != pgrp {
        read_proc_cmdline(tpgid as u32).or_else(|| read_proc_comm(tpgid as u32))
    } else {
        None
    };

    ShellStatus { cwd, running_command }
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
    app: AppHandle,
    session_id: &str,
    rows: u16,
    cols: u16,
    on_output: Channel<InvokeResponseBody>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let session = get_or_create_session(&state, session_id, rows, cols).await?;

    // El canal se guarda antes de arrancar la shell, así que no hay ventana por
    // la que se pueda perder la primera salida. Antes esto dependía de que el
    // frontend se suscribiera a los eventos primero.
    {
        let mut salida = session
            .output
            .lock()
            .unwrap_or_else(|envenenado| envenenado.into_inner());
        *salida = Some(on_output);
    }

    // This command can be invoked more than once on remount/HMR.
    // If a shell already exists for this PTY, treat it as success — pero con el
    // canal ya reemplazado arriba, que es lo que hace que la vista remontada
    // vuelva a recibir la salida.
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
                // Start streaming PTY output to the frontend (push model).
                if let Some(reader) = session.reader.lock().await.take() {
                    spawn_reader(
                        app.clone(),
                        session.clone(),
                        session_id.to_string(),
                        reader,
                    );
                }
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

fn is_process_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
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

    if !is_process_alive(shell_pid) {
        return Err("Shell process has exited".to_string());
    }

    #[cfg(target_os = "linux")]
    {
        Ok(read_shell_status_linux(shell_pid))
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(default_shell_status())
    }
}

#[tauri::command]
pub async fn async_take_startup_command(
    session_id: &str,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let mut startup_command_state = state.startup_command_state.lock().await;

    let Some(command) = startup_command_state.command.clone() else {
        return Ok(None);
    };

    match startup_command_state.claim.as_deref() {
        None => {
            startup_command_state.claim = Some(session_id.to_string());
            Ok(Some(command))
        }
        Some(claimed_by) if claimed_by == session_id => Ok(Some(command)),
        Some(_) => Ok(None),
    }
}

#[tauri::command]
pub async fn async_confirm_startup_command_delivered(
    session_id: &str,
    delivered_command: &str,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut startup_command_state = state.startup_command_state.lock().await;

    let claim_matches = startup_command_state
        .claim
        .as_ref()
        .is_some_and(|claimed_by| claimed_by == session_id);
    let command_matches = startup_command_state
        .command
        .as_ref()
        .is_some_and(|value| value == delivered_command);

    if claim_matches && command_matches {
        startup_command_state.command.take();
        startup_command_state.claim.take();
    }

    Ok(())
}

use std::cell::RefCell;

use gtk::prelude::WidgetExt;

thread_local! {
    pub static OVERLAY_WIN: RefCell<Option<gtk::Window>> = const { RefCell::new(None) };
}

#[tauri::command]
pub fn is_overlay_mode(state: tauri::State<'_, AppState>) -> bool {
    state.is_overlay
}

#[tauri::command]
pub fn show_overlay() -> Result<(), String> {
    OVERLAY_WIN.with(|win| {
        win.borrow()
            .as_ref()
            .ok_or_else(|| "Overlay window not initialized".to_string())
            .map(|w| w.show_all())
    })
}

#[tauri::command]
pub fn hide_overlay() -> Result<(), String> {
    OVERLAY_WIN.with(|win| {
        win.borrow()
            .as_ref()
            .ok_or_else(|| "Overlay window not initialized".to_string())
            .map(|w| w.hide())
    })
}

/// Portapapeles del sistema.
///
/// El webview no puede leer el portapapeles: WebKitGTK no implementa el permiso
/// «clipboard-read», así que `navigator.clipboard.readText()` no sirve y sin
/// leerlo no hay «Pegar» en el menú. GTK sí puede, porque es el que ya tiene la
/// conexión con el compositor, pero sus funciones de portapapeles sólo se pueden
/// llamar desde el hilo principal: de ahí el salto y el canal para traer la
/// respuesta.
fn with_clipboard<T, F>(app: &AppHandle, action: F) -> Result<T, String>
where
    F: FnOnce(&gtk::Clipboard) -> T + Send + 'static,
    T: Send + 'static,
{
    let (sender, receiver) = std::sync::mpsc::channel();

    app.run_on_main_thread(move || {
        let clipboard = gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD);
        let _ = sender.send(action(&clipboard));
    })
    .map_err(|error| error.to_string())?;

    receiver.recv().map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn clipboard_read_text(app: AppHandle) -> Result<String, String> {
    with_clipboard(&app, |clipboard| {
        clipboard
            .wait_for_text()
            .map(|text| text.to_string())
            .unwrap_or_default()
    })
}

#[tauri::command]
pub async fn clipboard_write_text(app: AppHandle, text: String) -> Result<(), String> {
    with_clipboard(&app, move |clipboard| {
        clipboard.set_text(&text);
        // Pedirle al gestor de portapapeles que se quede con el texto: sin
        // esto, lo copiado se pierde al cerrar la ventana del terminal.
        clipboard.store();
    })
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    /// Una línea de `/proc/PID/stat` real, de una zsh.
    const STAT_ZSH: &str =
        "1234 (zsh) S 1200 1234 1234 34816 1300 4194304 900 0 0 0 1 2 0 0 20 0 1 0 999 0 0";

    #[test]
    fn se_leen_el_grupo_y_el_primer_plano() {
        // Campos tras el `)`: estado, ppid, pgrp, sid, tty, tpgid -> pgrp=1234,
        // tpgid=1300.
        let (pgrp, tpgid) = parse_stat_pgrp_tpgid(STAT_ZSH).unwrap();
        assert_eq!(pgrp, 1234);
        assert_eq!(tpgid, 1300);
    }

    #[test]
    fn un_nombre_de_proceso_con_espacios_no_corre_los_campos() {
        // El nombre está entre paréntesis y puede tener espacios. Partir por
        // espacios desde el principio daría los campos corridos y un comando en
        // primer plano inventado.
        let stat = "77 (Web Content) S 70 77 77 0 -1 4194304 1 0 0 0 1 2 0 0 20 0 1 0 5 0 0";
        let (pgrp, tpgid) = parse_stat_pgrp_tpgid(stat).unwrap();
        assert_eq!(pgrp, 77);
        assert_eq!(tpgid, -1);
    }

    #[test]
    fn un_nombre_con_parentesis_se_lee_por_el_ultimo() {
        // Un ejecutable puede llamarse así, y es la razón por la que se busca el
        // último `)` y no el primero.
        let stat = "88 (raro) cosa) S 80 88 88 0 91 4194304 1 0 0 0 1 2 0 0 20 0 1 0 5 0 0";
        let (pgrp, tpgid) = parse_stat_pgrp_tpgid(stat).unwrap();
        assert_eq!(pgrp, 88);
        assert_eq!(tpgid, 91);
    }

    #[test]
    fn una_linea_truncada_no_inventa_valores() {
        assert!(parse_stat_pgrp_tpgid("1 (init) S 0 1").is_none());
        assert!(parse_stat_pgrp_tpgid("basura sin parentesis").is_none());
        assert!(parse_stat_pgrp_tpgid("").is_none());
    }

    #[test]
    fn los_argumentos_se_juntan_con_espacios() {
        // /proc/PID/cmdline separa con bytes nulos y suele terminar en uno.
        let data = b"git\0commit\0-m\0mensaje\0";
        assert_eq!(parse_cmdline(data).unwrap(), "git commit -m mensaje");
    }

    #[test]
    fn un_cmdline_vacio_no_da_comando() {
        // Los procesos de kernel tienen el cmdline vacío.
        assert!(parse_cmdline(b"").is_none());
        assert!(parse_cmdline(b"\0\0\0").is_none());
    }

    #[test]
    fn un_comando_largo_con_acentos_no_paniquea() {
        // Este es el caso que rompía: el recorte era por bytes con
        // `&joined[..117]`, así que 116 caracteres seguidos de una `ñ` dejaban
        // el corte en medio del carácter y el hilo se caía. Una ruta con acento
        // alcanza para llegar acá.
        let mut comando = "a".repeat(116);
        comando.push('ñ');
        comando.push_str(&"b".repeat(200));

        let resultado = parse_cmdline(comando.as_bytes()).expect("tiene que devolver algo");
        assert!(resultado.ends_with("..."));
        assert!(resultado.chars().count() <= CMDLINE_MAX);
    }

    #[test]
    fn el_recorte_cuenta_caracteres_y_no_bytes() {
        // Con 200 caracteres multibyte el largo en bytes es el triple; recortar
        // por bytes habría dejado bastante menos texto del que cabe.
        let comando = "ñ".repeat(200);
        let resultado = parse_cmdline(comando.as_bytes()).unwrap();
        assert_eq!(resultado.chars().count(), CMDLINE_MAX);
        assert!(resultado.starts_with('ñ'));
    }

    #[test]
    fn un_comando_corto_pasa_entero() {
        assert_eq!(parse_cmdline(b"ls\0-la\0").unwrap(), "ls -la");
        // Justo en el límite tampoco se toca.
        let justo = "a".repeat(CMDLINE_MAX);
        assert_eq!(parse_cmdline(justo.as_bytes()).unwrap(), justo);
    }
}
