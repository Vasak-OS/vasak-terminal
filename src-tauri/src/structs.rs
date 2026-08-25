use portable_pty::PtyPair;
use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::{Arc, Mutex},
};
use tauri::{async_runtime::Mutex as AsyncMutex, ipc::Channel, ipc::InvokeResponseBody};

pub struct TerminalSession {
    pub pty_pair: AsyncMutex<PtyPair>,
    pub writer: AsyncMutex<Box<dyn Write + Send>>,
    // Taken by the push reader thread once the shell starts (see spawn_reader).
    pub reader: AsyncMutex<Option<Box<dyn Read + Send>>>,
    pub shell_started: AsyncMutex<bool>,
    pub shell_pid: AsyncMutex<Option<u32>>,
    /// Por dónde sale lo que escribe el PTY.
    ///
    /// Guardado en la sesión y no capturado por el hilo lector porque un canal
    /// es punto a punto: cuando la vista se vuelve a montar —recarga, HMR—
    /// crea un canal nuevo, y el lector tiene que empezar a escribir en ése. Un
    /// `Mutex` común y no el asíncrono: el lector es un hilo bloqueante y no
    /// puede esperar en un `await`.
    pub output: Mutex<Option<Channel<InvokeResponseBody>>>,
}

pub struct AppState {
    pub sessions: Arc<AsyncMutex<HashMap<String, Arc<TerminalSession>>>>,
    pub startup_command_state: Arc<AsyncMutex<StartupCommandState>>,
    pub is_overlay: bool,
}

pub struct StartupCommandState {
    pub command: Option<String>,
    pub claim: Option<String>,
}