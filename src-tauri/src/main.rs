// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let argumentos: Vec<String> = std::env::args().skip(1).collect();
    vasak_terminal_lib::run(vasak_terminal_lib::argumentos::leer(&argumentos))
}
