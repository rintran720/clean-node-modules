use clean_node_modules_lib::deleter::delete_many;
use std::fs;
use tempfile::tempdir;

#[test]
fn moves_existing_paths_to_trash() {
    let dir = tempdir().unwrap();
    let nm = dir.path().join("my-app/node_modules");
    fs::create_dir_all(&nm).unwrap();
    fs::write(nm.join("file"), b"x").unwrap();

    let results = delete_many(vec![nm.clone()]);
    assert_eq!(results.len(), 1);
    assert!(results[0].ok, "delete should report ok=true: {:?}", results[0].error);
    assert!(!nm.exists(), "path should be gone from original location");
}

#[test]
fn reports_error_for_missing_path() {
    let bogus = std::path::PathBuf::from("/no/such/path/clean_nm_missing");
    let results = delete_many(vec![bogus]);
    assert_eq!(results.len(), 1);
    assert!(!results[0].ok);
    assert!(results[0].error.is_some());
}
