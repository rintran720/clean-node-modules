use serde::Serialize;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize)]
pub struct ProjectInfo {
    pub project_path: PathBuf,
    pub node_modules_path: PathBuf,
    pub size_bytes: u64,
    /// Epoch millis. Newest mtime under project_path, excluding node_modules subtree.
    pub project_modified_ms: u64,
    /// Epoch millis. Newest mtime under node_modules_path.
    pub nm_modified_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteResult {
    pub path: PathBuf,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    pub projects_found: u64,
    pub total_size_bytes: u64,
    pub dirs_skipped: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanDone {
    pub cancelled: bool,
    pub total_projects: u64,
    pub total_size_bytes: u64,
    pub dirs_skipped: u64,
}

pub fn system_time_to_ms(t: SystemTime) -> u64 {
    t.duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
