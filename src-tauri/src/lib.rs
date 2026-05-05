pub mod deleter;
pub mod mtime;
pub mod scanner;
pub mod sizer;
pub mod types;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{Emitter, State};
use tauri_plugin_dialog::DialogExt;

use crate::scanner::{scan_blocking, ScanCallbacks};
use crate::types::{DeleteProgress, DeleteResult, DeleteStart, ScanDone};

#[derive(Default)]
struct AppState {
    cancel: Arc<AtomicBool>,
}

#[tauri::command]
async fn pick_root_folder(window: tauri::Window) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    window.dialog().file().pick_folder(move |path| {
        let _ = tx.send(path);
    });
    rx.recv()
        .ok()
        .flatten()
        .and_then(|p| p.into_path().ok())
        .map(|pb| pb.to_string_lossy().into_owned())
}

#[tauri::command]
async fn scan(
    window: tauri::Window,
    state: State<'_, AppState>,
    root: String,
) -> Result<(), String> {
    state.cancel.store(false, Ordering::Relaxed);
    let cancel = state.cancel.clone();
    let win_proj = window.clone();
    let win_prog = window.clone();
    let win_done = window.clone();
    let root_path = PathBuf::from(root);

    tauri::async_runtime::spawn_blocking(move || {
        let cbs = ScanCallbacks {
            on_project: Box::new(move |p| {
                let _ = win_proj.emit("scan:project", p);
            }),
            on_progress: Box::new(move |p| {
                let _ = win_prog.emit("scan:progress", p);
            }),
            cancel,
        };
        let s = scan_blocking(&root_path, cbs);
        let _ = win_done.emit(
            "scan:done",
            ScanDone {
                cancelled: s.cancelled,
                total_projects: s.projects_found,
                total_size_bytes: s.total_size_bytes,
                dirs_skipped: s.dirs_skipped,
            },
        );
    });

    Ok(())
}

#[tauri::command]
fn cancel_scan(state: State<'_, AppState>) {
    state.cancel.store(true, Ordering::Relaxed);
}

async fn run_delete<F>(
    window: tauri::Window,
    paths: Vec<String>,
    one: F,
) -> Result<Vec<DeleteResult>, String>
where
    F: Fn(&std::path::Path) -> DeleteResult + Send + Sync + 'static,
{
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let total = paths.len() as u64;
    tauri::async_runtime::spawn_blocking(move || {
        let mut results = Vec::with_capacity(paths.len());
        for (i, p) in paths.iter().enumerate() {
            let _ = window.emit(
                "delete:item-start",
                DeleteStart { index: (i + 1) as u64, total, path: p.clone() },
            );
            let r = one(p);
            let _ = window.emit(
                "delete:progress",
                DeleteProgress { index: (i + 1) as u64, total, result: r.clone() },
            );
            results.push(r);
        }
        results
    })
    .await
    .map_err(|e| format!("delete task failed: {e}"))
}

#[tauri::command]
async fn delete_many(window: tauri::Window, paths: Vec<String>) -> Result<Vec<DeleteResult>, String> {
    run_delete(window, paths, crate::deleter::delete_one).await
}

#[tauri::command]
async fn delete_many_permanent(window: tauri::Window, paths: Vec<String>) -> Result<Vec<DeleteResult>, String> {
    run_delete(window, paths, crate::deleter::delete_one_permanent).await
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            pick_root_folder,
            scan,
            cancel_scan,
            delete_many,
            delete_many_permanent
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
