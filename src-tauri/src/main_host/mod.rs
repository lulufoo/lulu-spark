//! Main Host: general business entry on loopback (`GET/POST /api/notes-*`, `/api/archive-*`, `/api/read-later*`, `/api/todo-tasks`, `POST /api/bind-complete`, `/api/status`).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use tiny_http::Server;

use dispatch::handle_request;

pub const DEFAULT_HTTP_PORT: u16 = crate::config::settings::DEFAULT_PROD_HTTP_PORT;

/// Locked master-task `status` wire values for `/api/todo-tasks`, `/api/todo-task`, `/api/todo-task-create`.
pub(crate) const TODO_TASK_MASTER_STATUS_WIRE: &[&str] = &["incomplete", "complete", "abandoned"];

mod archive;
mod bind;
mod dispatch;
mod notes_cat;
mod read_later;
mod respond;
mod todo;

pub(crate) use respond::{
    map_value_to_response, todo_task_get_response_body, todo_tasks_list_response_body,
};

pub struct MainHostHandle {
    server: Arc<Server>,
    join: JoinHandle<()>,
}

#[derive(Debug)]
pub enum MainHostError {
    BindFailed(String),
}

pub struct MainHostState {
    handle: Mutex<Option<MainHostHandle>>,
    http_ready: Arc<AtomicBool>,
}

impl MainHostState {
    pub fn new() -> Self {
        Self {
            handle: Mutex::new(None),
            http_ready: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.http_ready.load(Ordering::SeqCst)
    }

    pub fn try_start(&self, repo_root: PathBuf, port: u16) {
        match start(repo_root, port) {
            Ok(handle) => {
                if let Ok(mut guard) = self.handle.lock() {
                    *guard = Some(handle);
                    self.http_ready.store(true, Ordering::SeqCst);
                }
            }
            Err(MainHostError::BindFailed(msg)) => {
                eprintln!("[main_host] bind failed on port {port}: {msg}");
                self.http_ready.store(false, Ordering::SeqCst);
            }
        }
    }

    pub fn stop(&self) {
        if let Ok(mut guard) = self.handle.lock() {
            if let Some(handle) = guard.take() {
                stop(handle);
            }
        }
        self.http_ready.store(false, Ordering::SeqCst);
    }
}

pub fn start(repo_root: PathBuf, port: u16) -> Result<MainHostHandle, MainHostError> {
    let addr = format!("127.0.0.1:{port}");
    let server = Server::http(&addr).map_err(|e| MainHostError::BindFailed(e.to_string()))?;
    let server = Arc::new(server);
    let server_for_thread = Arc::clone(&server);
    let join = thread::spawn(move || {
        for request in server_for_thread.incoming_requests() {
            handle_request(&repo_root, port, request);
        }
    });
    Ok(MainHostHandle { server, join })
}

pub fn stop(handle: MainHostHandle) {
    handle.server.unblock();
    let _ = handle.join.join();
}

#[cfg(test)]
#[path = "../unit-tests/main_host.rs"]
mod tests;
