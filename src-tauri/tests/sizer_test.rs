use clean_node_modules_lib::sizer::dir_size;
use std::fs;
use tempfile::tempdir;

#[test]
fn sums_file_sizes_recursively() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), b"hello").unwrap(); // 5 bytes
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("b.txt"), b"world!!").unwrap();      // 7 bytes

    let n = dir_size(dir.path());
    assert_eq!(n, 12);
}

#[test]
fn empty_dir_is_zero() {
    let dir = tempdir().unwrap();
    assert_eq!(dir_size(dir.path()), 0);
}

#[test]
fn missing_dir_is_zero() {
    let bogus = std::path::Path::new("/no/such/path/clean_nm_test");
    assert_eq!(dir_size(bogus), 0);
}

#[test]
fn counts_hidden_subdirs() {
    // pnpm stores actual deps under node_modules/.pnpm/ — must be counted.
    let dir = tempdir().unwrap();
    let pnpm = dir.path().join(".pnpm").join("foo@1.0.0").join("node_modules").join("foo");
    fs::create_dir_all(&pnpm).unwrap();
    fs::write(pnpm.join("index.js"), vec![0u8; 4096]).unwrap();
    assert_eq!(dir_size(dir.path()), 4096);
}

#[cfg(unix)]
#[test]
fn does_not_follow_symlinks() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    std::fs::write(target.path().join("big.bin"), vec![0u8; 1000]).unwrap();
    symlink(target.path(), dir.path().join("link")).unwrap();
    assert_eq!(clean_node_modules_lib::sizer::dir_size(dir.path()), 0);
}
