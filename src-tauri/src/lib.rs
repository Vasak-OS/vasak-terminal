mod commands;
mod structs;
mod wayland_layer;

use crate::commands::{
    async_close_shell, async_confirm_startup_command_delivered, async_create_shell,
    async_get_shell_status, async_read_from_pty, async_resize_pty, async_take_startup_command,
    async_write_to_pty, hide_overlay, is_overlay_mode, show_overlay,
};
use crate::structs::{AppState, StartupCommandState};
use shell_words::split as split_shell_words;
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    sync::Arc,
};
use tauri::{async_runtime::Mutex as AsyncMutex, Emitter, Manager};

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

fn read_shebang_tokens(path: &Path) -> Option<Vec<String>> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    let bytes_read = reader.read_line(&mut first_line).ok()?;

    if bytes_read == 0 || !first_line.starts_with("#!") {
        return None;
    }

    let shebang = first_line
        .trim_end_matches(['\n', '\r'])
        .trim_start_matches("#!")
        .trim();

    if shebang.is_empty() {
        None
    } else {
        split_shell_words(shebang)
            .ok()
            .filter(|tokens| !tokens.is_empty())
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

    if let Some(shebang_tokens) = read_shebang_tokens(path) {
        let escaped_tokens = shebang_tokens
            .iter()
            .map(|token| shell_escape_single_quoted(token))
            .collect::<Vec<_>>()
            .join(" ");
        return Some(format!("{escaped_tokens} {escaped_file}\n"));
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
pub fn run(is_overlay: bool) {
    let startup_command = if !is_overlay {
        resolve_startup_command_from_args()
    } else {
        None
    };

    let mut builder = tauri::Builder::default()
        .manage(AppState {
            sessions: Arc::new(AsyncMutex::new(HashMap::new())),
            startup_command_state: Arc::new(AsyncMutex::new(StartupCommandState {
                command: startup_command,
                claim: None,
            })),
            is_overlay,
        })
        .invoke_handler(tauri::generate_handler![
            async_write_to_pty,
            async_resize_pty,
            async_create_shell,
            async_read_from_pty,
            async_close_shell,
            async_get_shell_status,
            async_take_startup_command,
            async_confirm_startup_command_delivered,
            is_overlay_mode,
            show_overlay,
            hide_overlay
        ])
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_config_manager::init())
        .plugin(tauri_plugin_vicons::init())
        .plugin(tauri_plugin_i18n_vsk::init(None));

    if is_overlay {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let is_overlay_request = argv.iter().any(|a| a == "--overlay");
            if is_overlay_request {
                let _ = crate::commands::show_overlay();
                if let Some(window) = app.get_webview_window("main") {
                    window.emit("vterminal:toggle-overlay", ()).ok();
                }
            }
        }));

        builder = builder.setup(|app| {
                let window =
                    app.get_webview_window("main").ok_or("main window not found")?;

                if let Ok(gtk_tauri_win) = window.gtk_window() {
                    use gtk::prelude::*;
                    use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

                    // Compute logical dimensions
                    let (w, h, logical_width) = if let Some(monitor) =
                        window.primary_monitor()?
                    {
                        let phys = monitor.size();
                        let scale = window.scale_factor()?;
                        let lw = (phys.width as f64 / scale) as i32;
                        let lh = (phys.height as f64 / scale) as i32;
                        let w = (lw as f64 * 2.0 / 3.0) as i32;
                        let h = (lh as f64 / 3.0).max(150.0) as i32;
                        (w, h, lw)
                    } else {
                        (800, 300, 800)
                    };

                    // Create a fresh GTK window for layer-shell
                    let layer_win = gtk::Window::new(gtk::WindowType::Toplevel);
                    layer_win.set_decorated(false);
                    layer_win.set_default_size(w, h);
                    layer_win.set_size_request(w, h);

                    // Configure layer-shell
                    layer_win.init_layer_shell();
                    layer_win.set_namespace("vasak-terminal");
                    layer_win.set_layer(Layer::Overlay);
                    layer_win.set_anchor(Edge::Bottom, true);
                    layer_win.set_anchor(Edge::Left, true);
                    layer_win.set_anchor(Edge::Right, true);
                    layer_win.set_exclusive_zone(0);
                    let margin = (logical_width - w) / 2;
                    layer_win.set_layer_shell_margin(Edge::Left, margin);
                    layer_win.set_layer_shell_margin(Edge::Right, margin);
                    layer_win.set_keyboard_mode(KeyboardMode::OnDemand);

                    // RGBA visual + transparent background so the webview's
                    // rounded‑corner CSS shows through at the window level
                    if let Some(screen) = gtk::gdk::Screen::default() {
                        if let Some(visual) = screen.rgba_visual() {
                            layer_win.set_visual(Some(&visual));
                        }
                        let provider = gtk::CssProvider::new();
                        provider
                            .load_from_data(
                                b"window { background: transparent; }",
                            )
                            .ok();
                        gtk::StyleContext::add_provider_for_screen(
                            &screen,
                            &provider,
                            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                        );
                    }

                    // Reparent the WebKitWebView from the xdg window into the
                    // layer-shell window
                    if let Some(child) = gtk_tauri_win.child() {
                        if let Ok(container) =
                            child.dynamic_cast::<gtk::Container>()
                        {
                            if let Some(webview) = container.children().first() {
                                container.remove(webview);
                                layer_win.add(webview);
                                gtk_tauri_win.hide();

                            crate::commands::OVERLAY_WIN.with(
                                |win| {
                                    *win.borrow_mut() = Some(layer_win);
                                },
                            );
                                eprintln!(
                                    "[overlay] reparented webview: \
                                     {}x{} margin={}",
                                    w, h, margin
                                );
                            } else {
                                eprintln!(
                                    "[overlay] no children in container"
                                );
                            }
                        } else {
                            eprintln!("[overlay] child is not a Container");
                        }
                    } else {
                        eprintln!("[overlay] no child in Tauri window");
                    }
                }

                Ok(())
            });
    } else {
        builder = builder.setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
            }
            Ok(())
        });
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
