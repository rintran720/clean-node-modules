use clean_node_modules_lib::scanner::{scan_blocking, ScanCallbacks};
use std::fs;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

fn collect(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let found = Arc::new(Mutex::new(Vec::new()));
    let f2 = found.clone();
    let cbs = ScanCallbacks {
        on_project: Box::new(move |p| f2.lock().unwrap().push(p.node_modules_path.clone())),
        on_progress: Box::new(|_| {}),
        cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    };
    scan_blocking(root, cbs);
    let mut v = found.lock().unwrap().clone();
    v.sort();
    v
}

#[test]
fn finds_top_level_node_modules() {
    let dir = tempdir().unwrap();
    let proj = dir.path().join("my-app");
    fs::create_dir_all(proj.join("node_modules/some-dep")).unwrap();
    fs::write(proj.join("node_modules/some-dep/index.js"), b"x").unwrap();

    let v = collect(dir.path());
    assert_eq!(v.len(), 1);
    assert!(v[0].ends_with("my-app/node_modules"));
}

#[test]
fn does_not_recurse_into_node_modules() {
    let dir = tempdir().unwrap();
    let proj = dir.path().join("my-app");
    let nested = proj.join("node_modules/dep/node_modules"); // nested NM inside dep
    fs::create_dir_all(&nested).unwrap();

    let v = collect(dir.path());
    assert_eq!(v.len(), 1, "should report only the top-level node_modules");
}

#[test]
fn skips_dot_directories() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".cache/node_modules")).unwrap();
    let v = collect(dir.path());
    assert!(v.is_empty(), "node_modules under .cache must be ignored");
}

#[test]
fn respects_cancel_before_processing() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("a/node_modules")).unwrap();
    fs::create_dir_all(dir.path().join("b/node_modules")).unwrap();

    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(true)); // cancelled up-front
    let found = Arc::new(Mutex::new(Vec::<std::path::PathBuf>::new()));
    let f2 = found.clone();
    let cbs = ScanCallbacks {
        on_project: Box::new(move |p| f2.lock().unwrap().push(p.node_modules_path.clone())),
        on_progress: Box::new(|_| {}),
        cancel,
    };
    let summary = scan_blocking(dir.path(), cbs);
    assert!(summary.cancelled);
    assert_eq!(summary.projects_found, 0);
    assert!(found.lock().unwrap().is_empty());
}
