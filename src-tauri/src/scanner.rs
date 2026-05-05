use crate::mtime::{newest_mtime, newest_mtime_skipping};
use crate::sizer::dir_size;
use crate::types::{system_time_to_ms, ProjectInfo, ScanProgress};
use jwalk::WalkDir;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct ScanCallbacks {
    pub on_project: Box<dyn Fn(ProjectInfo) + Send + Sync>,
    pub on_progress: Box<dyn Fn(ScanProgress) + Send + Sync>,
    pub cancel: Arc<AtomicBool>,
}

pub struct ScanSummary {
    pub projects_found: u64,
    pub total_size_bytes: u64,
    pub dirs_skipped: u64,
    pub cancelled: bool,
}

pub fn scan_blocking(root: &Path, cbs: ScanCallbacks) -> ScanSummary {
    let cancel = cbs.cancel.clone();

    // Find every node_modules dir without recursing into them, skipping hidden dirs.
    let nm_paths: Vec<PathBuf> = WalkDir::new(root)
        .follow_links(false)
        .skip_hidden(true)
        .process_read_dir(|_depth, _path, _state, children| {
            for res in children.iter_mut() {
                if let Ok(entry) = res {
                    if entry.file_type.is_dir()
                        && entry.file_name() == std::ffi::OsStr::new("node_modules")
                    {
                        // Mark the node_modules entry as a leaf — do not recurse.
                        entry.read_children_path = None;
                    }
                }
            }
        })
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_dir() && e.file_name() == std::ffi::OsStr::new("node_modules"))
        .map(|e| e.path())
        .collect();

    let mut projects_found = 0u64;
    let mut total_size = 0u64;

    for nm in nm_paths {
        if cancel.load(Ordering::Relaxed) {
            return ScanSummary {
                projects_found,
                total_size_bytes: total_size,
                dirs_skipped: 0,
                cancelled: true,
            };
        }
        let project_path = match nm.parent() {
            Some(p) => p.to_path_buf(),
            None => continue,
        };
        let size_bytes = dir_size(&nm);
        let nm_modified = newest_mtime(&nm).unwrap_or(std::time::UNIX_EPOCH);
        let project_modified =
            newest_mtime_skipping(&project_path, &nm).unwrap_or(std::time::UNIX_EPOCH);

        let info = ProjectInfo {
            project_path,
            node_modules_path: nm,
            size_bytes,
            project_modified_ms: system_time_to_ms(project_modified),
            nm_modified_ms: system_time_to_ms(nm_modified),
        };

        projects_found += 1;
        total_size += size_bytes;
        (cbs.on_project)(info);
        (cbs.on_progress)(ScanProgress {
            projects_found,
            total_size_bytes: total_size,
            dirs_skipped: 0,
        });
    }

    ScanSummary {
        projects_found,
        total_size_bytes: total_size,
        dirs_skipped: 0,
        cancelled: false,
    }
}
