use crate::types::DeleteResult;
use std::path::PathBuf;

pub fn delete_many(paths: Vec<PathBuf>) -> Vec<DeleteResult> {
    paths
        .into_iter()
        .map(|p| match trash::delete(&p) {
            Ok(()) => DeleteResult { path: p, ok: true, error: None },
            Err(e) => DeleteResult { path: p, ok: false, error: Some(e.to_string()) },
        })
        .collect()
}
