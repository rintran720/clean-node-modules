use jwalk::WalkDir;
use std::path::Path;

/// Sum of regular-file sizes under `root`, recursively.
/// Symlinks are not followed. Errors (permission denied, missing) are silently ignored.
/// Hidden subdirs are walked too — pnpm stores everything under `node_modules/.pnpm/`.
pub fn dir_size(root: &Path) -> u64 {
    WalkDir::new(root)
        .follow_links(false)
        .skip_hidden(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}
