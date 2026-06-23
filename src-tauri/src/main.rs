// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let is_overlay = std::env::args().any(|a| a == "--overlay");
    vasak_terminal_lib::run(is_overlay)
}
