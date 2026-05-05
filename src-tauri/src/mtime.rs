use jwalk::WalkDir;
use std::path::Path;
use std::time::SystemTime;

/// Newest mtime among regular files under `root` (recursive). None if no files.
pub fn newest_mtime(root: &Path) -> Option<SystemTime> {
    newest_mtime_skipping(root, Path::new(""))
}

/// Same as `newest_mtime` but any path equal to `skip` (and its subtree) is excluded.
pub fn newest_mtime_skipping(root: &Path, skip: &Path) -> Option<SystemTime> {
    let skip = skip.to_path_buf();
    WalkDir::new(root)
        .follow_links(false)
        .process_read_dir(move |_depth, _path, _state, children| {
            children.retain(|res| match res {
                Ok(entry) => entry.path() != skip,
                Err(_) => true,
            });
        })
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .filter_map(|m| m.modified().ok())
        .max()
}
