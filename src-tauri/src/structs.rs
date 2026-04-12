use portable_pty::PtyPair;
use std::{
    collections::HashMap,
    io::{BufReader, Read, Write},
    sync::Arc,
};
use tauri::{async_runtime::Mutex as AsyncMutex};

pub struct TerminalSession {
    pub pty_pair: AsyncMutex<PtyPair>,
    pub writer: AsyncMutex<Box<dyn Write + Send>>,
    pub reader: AsyncMutex<BufReader<Box<dyn Read + Send>>>,
    pub shell_started: AsyncMutex<bool>,
    pub shell_pid: AsyncMutex<Option<u32>>,
}

pub struct AppState {
    pub sessions: Arc<AsyncMutex<HashMap<String, Arc<TerminalSession>>>>,
    pub startup_command_state: Arc<AsyncMutex<StartupCommandState>>,
}

pub struct StartupCommandState {
    pub command: Option<String>,
    pub claim: Option<String>,
}