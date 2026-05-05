use crate::types::DeleteResult;
use std::path::{Path, PathBuf};

pub fn delete_one(p: &Path) -> DeleteResult {
    match trash::delete(p) {
        Ok(()) => DeleteResult { path: p.to_path_buf(), ok: true, error: None },
        Err(e) => DeleteResult { path: p.to_path_buf(), ok: false, error: Some(e.to_string()) },
    }
}

pub fn delete_one_permanent(p: &Path) -> DeleteResult {
    match std::fs::remove_dir_all(p) {
        Ok(()) => DeleteResult { path: p.to_path_buf(), ok: true, error: None },
        Err(e) => DeleteResult { path: p.to_path_buf(), ok: false, error: Some(e.to_string()) },
    }
}

pub fn delete_many(paths: Vec<PathBuf>) -> Vec<DeleteResult> {
    paths.iter().map(|p| delete_one(p)).collect()
}

pub fn delete_many_permanent(paths: Vec<PathBuf>) -> Vec<DeleteResult> {
    paths.iter().map(|p| delete_one_permanent(p)).collect()
}
