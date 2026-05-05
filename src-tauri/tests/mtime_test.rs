use clean_node_modules_lib::mtime::{newest_mtime, newest_mtime_skipping};
use filetime::{set_file_mtime, FileTime};
use std::fs;
use tempfile::tempdir;

fn touch(p: &std::path::Path, secs: i64) {
    fs::write(p, b"x").unwrap();
    set_file_mtime(p, FileTime::from_unix_time(secs, 0)).unwrap();
}

#[test]
fn newest_returns_latest_mtime() {
    let dir = tempdir().unwrap();
    touch(&dir.path().join("a"), 100);
    touch(&dir.path().join("b"), 500);
    touch(&dir.path().join("c"), 200);
    let t = newest_mtime(dir.path()).unwrap();
    let secs = t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    assert_eq!(secs, 500);
}

#[test]
fn skipping_excludes_subtree() {
    let dir = tempdir().unwrap();
    touch(&dir.path().join("a"), 100);
    let nm = dir.path().join("node_modules");
    fs::create_dir(&nm).unwrap();
    touch(&nm.join("dep"), 999); // newer, but should be skipped
    let t = newest_mtime_skipping(dir.path(), &nm).unwrap();
    let secs = t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    assert_eq!(secs, 100);
}

#[test]
fn empty_dir_returns_none() {
    let dir = tempdir().unwrap();
    assert!(newest_mtime(dir.path()).is_none());
}
