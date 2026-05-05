use jwalk::WalkDir;
use std::path::Path;

/// Sum of regular-file sizes under `root`, recursively.
/// Symlinks are not followed. Errors (permission denied, missing) are silently ignored.
pub fn dir_size(root: &Path) -> u64 {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}
