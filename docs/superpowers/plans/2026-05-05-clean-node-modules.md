# clean-node-modules Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Tauri-based desktop app that scans a chosen folder for `node_modules` directories and lets the user bulk-move them to the OS trash.

**Architecture:** Rust backend exposes Tauri commands (`pick_root_folder`, `scan`, `cancel_scan`, `delete_many`). Scanner streams `ProjectInfo` rows to the frontend via Tauri events. Vanilla-JS frontend renders an incremental table with sorting, selection, and a confirmation dialog. Deletes go through the cross-platform `trash` crate.

**Tech Stack:** Rust 1.75+, Tauri 2.x, `jwalk` (parallel walker), `trash`, `serde`, `tauri-plugin-dialog`. Frontend: HTML + CSS + vanilla JS (no bundler). Distribution: `.dmg`, `.msi`, `.AppImage`/`.deb`.

**File structure (final shape):**

```
clean-node-modules/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs        # Tauri setup, command registration, AppState
│   │   ├── scanner.rs     # walk root, find node_modules (no recursion into)
│   │   ├── sizer.rs       # compute folder size in parallel
│   │   ├── mtime.rs       # newest-mtime helpers (project + nm)
│   │   ├── deleter.rs     # move to trash
│   │   └── types.rs       # ProjectInfo, DeleteResult, events
│   ├── Cargo.toml
│   ├── build.rs
│   └── tauri.conf.json
├── src/
│   ├── index.html
│   ├── main.js
│   └── style.css
└── package.json
```

Each Rust file has one responsibility. `main.rs` is glue only; logic lives in submodules so it can be unit-tested without Tauri.

---

## Task 1: Scaffold Tauri project

**Files:**
- Create: `clean-node-modules/package.json`
- Create: `clean-node-modules/src/index.html`
- Create: `clean-node-modules/src/main.js`
- Create: `clean-node-modules/src/style.css`
- Create: `clean-node-modules/src-tauri/Cargo.toml`
- Create: `clean-node-modules/src-tauri/build.rs`
- Create: `clean-node-modules/src-tauri/tauri.conf.json`
- Create: `clean-node-modules/src-tauri/src/main.rs`
- Create: `clean-node-modules/.gitignore`

- [ ] **Step 1: Create `.gitignore`**

```
/src-tauri/target
/node_modules
.DS_Store
```

- [ ] **Step 2: Create `package.json`** (used only as a place to record the Tauri CLI version; no JS deps required)

```json
{
  "name": "clean-node-modules",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "tauri": "tauri"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0"
  }
}
```

- [ ] **Step 3: Create `src/index.html`** (placeholder, real UI added in Task 10+)

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>clean-node-modules</title>
  <link rel="stylesheet" href="style.css" />
</head>
<body>
  <main id="app">
    <h1>clean-node-modules</h1>
  </main>
  <script type="module" src="main.js"></script>
</body>
</html>
```

- [ ] **Step 4: Create `src/main.js`** (empty stub)

```js
// UI logic added in later tasks
```

- [ ] **Step 5: Create `src/style.css`** (empty stub)

```css
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; margin: 0; }
```

- [ ] **Step 6: Create `src-tauri/Cargo.toml`**

```toml
[package]
name = "clean-node-modules"
version = "0.1.0"
edition = "2021"

[lib]
name = "clean_node_modules_lib"
path = "src/lib.rs"

[[bin]]
name = "clean-node-modules"
path = "src/main.rs"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
jwalk = "0.8"
trash = "5"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 7: Create `src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 8: Create `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "clean-node-modules",
  "version": "0.1.0",
  "identifier": "dev.local.clean-node-modules",
  "build": {
    "frontendDist": "../src",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "",
    "beforeBuildCommand": ""
  },
  "app": {
    "windows": [
      { "title": "clean-node-modules", "width": 900, "height": 600 }
    ],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": "all"
  },
  "plugins": {}
}
```

- [ ] **Step 9: Create `src-tauri/src/main.rs`** (will be replaced; just enough to compile)

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    clean_node_modules_lib::run()
}
```

- [ ] **Step 10: Create `src-tauri/src/lib.rs`**

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 11: Verify it compiles**

Run: `cd clean-node-modules/src-tauri && cargo build`
Expected: builds successfully (will download many crates first run).

- [ ] **Step 12: Commit**

```bash
git add .gitignore package.json src/ src-tauri/Cargo.toml src-tauri/build.rs src-tauri/tauri.conf.json src-tauri/src/main.rs src-tauri/src/lib.rs
git commit -m "chore: scaffold Tauri project skeleton"
```

---

## Task 2: Types module (`ProjectInfo`, `DeleteResult`)

**Files:**
- Create: `clean-node-modules/src-tauri/src/types.rs`
- Modify: `clean-node-modules/src-tauri/src/lib.rs`

- [ ] **Step 1: Write `src-tauri/src/types.rs`**

```rust
use serde::Serialize;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize)]
pub struct ProjectInfo {
    pub project_path: PathBuf,
    pub node_modules_path: PathBuf,
    pub size_bytes: u64,
    /// Epoch millis. Newest mtime under project_path, excluding node_modules subtree.
    pub project_modified_ms: u64,
    /// Epoch millis. Newest mtime under node_modules_path.
    pub nm_modified_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteResult {
    pub path: PathBuf,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    pub projects_found: u64,
    pub total_size_bytes: u64,
    pub dirs_skipped: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanDone {
    pub cancelled: bool,
    pub total_projects: u64,
    pub total_size_bytes: u64,
    pub dirs_skipped: u64,
}

pub fn system_time_to_ms(t: SystemTime) -> u64 {
    t.duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
```

- [ ] **Step 2: Add module to `lib.rs`**

```rust
pub mod types;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd src-tauri && cargo build`
Expected: builds; one or two unused-import warnings are OK at this stage.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/types.rs src-tauri/src/lib.rs
git commit -m "feat: add shared types for scan/delete payloads"
```

---

## Task 3: Sizer — compute folder size

**Files:**
- Create: `clean-node-modules/src-tauri/src/sizer.rs`
- Create: `clean-node-modules/src-tauri/tests/sizer_test.rs`
- Modify: `clean-node-modules/src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing test in `src-tauri/tests/sizer_test.rs`**

```rust
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
```

- [ ] **Step 2: Run the test, verify it fails**

Run: `cd src-tauri && cargo test --test sizer_test`
Expected: FAIL — `dir_size` not found.

- [ ] **Step 3: Implement `src-tauri/src/sizer.rs`**

```rust
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
```

- [ ] **Step 4: Add module to `lib.rs`**

Add `pub mod sizer;` next to `pub mod types;`.

- [ ] **Step 5: Run the test, verify it passes**

Run: `cd src-tauri && cargo test --test sizer_test`
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/sizer.rs src-tauri/tests/sizer_test.rs src-tauri/src/lib.rs
git commit -m "feat: parallel directory-size helper"
```

---

## Task 4: Mtime helper — newest mtime under a tree, with optional skip

**Files:**
- Create: `clean-node-modules/src-tauri/src/mtime.rs`
- Create: `clean-node-modules/src-tauri/tests/mtime_test.rs`
- Modify: `clean-node-modules/src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing tests in `src-tauri/tests/mtime_test.rs`**

```rust
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
```

Add to `src-tauri/Cargo.toml` `[dev-dependencies]`:

```toml
filetime = "0.2"
```

- [ ] **Step 2: Run tests, verify they fail**

Run: `cd src-tauri && cargo test --test mtime_test`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement `src-tauri/src/mtime.rs`**

```rust
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
```

- [ ] **Step 4: Add module to `lib.rs`**

Add `pub mod mtime;`.

- [ ] **Step 5: Run tests, verify they pass**

Run: `cd src-tauri && cargo test --test mtime_test`
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/mtime.rs src-tauri/src/lib.rs src-tauri/tests/mtime_test.rs
git commit -m "feat: newest-mtime helper with subtree skip"
```

---

## Task 5: Scanner — find node_modules without recursing into them

**Files:**
- Create: `clean-node-modules/src-tauri/src/scanner.rs`
- Create: `clean-node-modules/src-tauri/tests/scanner_test.rs`
- Modify: `clean-node-modules/src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing tests in `src-tauri/tests/scanner_test.rs`**

```rust
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
```

- [ ] **Step 2: Run tests, verify they fail**

Run: `cd src-tauri && cargo test --test scanner_test`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement `src-tauri/src/scanner.rs`**

```rust
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

    // Collect node_modules paths (no recursion into them, no dot-dirs, no symlinks).
    let nm_paths: Vec<PathBuf> = WalkDir::new(root)
        .follow_links(false)
        .skip_hidden(true)
        .process_read_dir(move |_depth, _path, _state, children| {
            // Drop entries whose own name is "node_modules" *after* we record them
            // by inspecting in the iteration below. To avoid descending into them,
            // we mark them as "do not recurse" here:
            for res in children.iter_mut() {
                if let Ok(entry) = res {
                    if entry.file_type.is_dir()
                        && entry.file_name() == std::ffi::OsStr::new("node_modules")
                    {
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
```

- [ ] **Step 4: Add module to `lib.rs`**

Add `pub mod scanner;`. Order: `types`, `sizer`, `mtime`, `scanner`.

- [ ] **Step 5: Run tests, verify they pass**

Run: `cd src-tauri && cargo test --test scanner_test`
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/scanner.rs src-tauri/src/lib.rs src-tauri/tests/scanner_test.rs
git commit -m "feat: scanner finds node_modules without recursing into them"
```

---

## Task 6: Scanner cancellation

**Files:**
- Modify: `clean-node-modules/src-tauri/tests/scanner_test.rs`

The cancel flag is already plumbed; this task adds a regression test.

- [ ] **Step 1: Add the failing test**

Append to `tests/scanner_test.rs`:

```rust
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
```

- [ ] **Step 2: Run, verify it passes**

Run: `cd src-tauri && cargo test --test scanner_test`
Expected: 4 passed.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/scanner_test.rs
git commit -m "test: scanner respects cancellation flag"
```

---

## Task 7: Deleter — move to OS trash

**Files:**
- Create: `clean-node-modules/src-tauri/src/deleter.rs`
- Create: `clean-node-modules/src-tauri/tests/deleter_test.rs`
- Modify: `clean-node-modules/src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing test in `src-tauri/tests/deleter_test.rs`**

```rust
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
```

- [ ] **Step 2: Run, verify it fails**

Run: `cd src-tauri && cargo test --test deleter_test`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement `src-tauri/src/deleter.rs`**

```rust
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
```

- [ ] **Step 4: Add module to `lib.rs`**

Add `pub mod deleter;`.

- [ ] **Step 5: Run, verify it passes**

Run: `cd src-tauri && cargo test --test deleter_test`
Expected: 2 passed. **Note:** on a headless Linux CI without a desktop session, the `trash` crate may fail; this is acceptable — run locally on your dev machine.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/deleter.rs src-tauri/src/lib.rs src-tauri/tests/deleter_test.rs
git commit -m "feat: cross-platform move-to-trash helper"
```

---

## Task 8: Tauri commands — wire backend to frontend

**Files:**
- Modify: `clean-node-modules/src-tauri/src/lib.rs`

This task plumbs four commands and the global cancel flag. No new tests — covered by manual end-to-end run in Task 13.

- [ ] **Step 1: Replace `lib.rs` with the wired-up version**

```rust
pub mod deleter;
pub mod mtime;
pub mod scanner;
pub mod sizer;
pub mod types;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::scanner::{scan_blocking, ScanCallbacks};
use crate::types::{DeleteResult, ScanDone};

#[derive(Default)]
struct AppState {
    cancel: Arc<AtomicBool>,
}

#[tauri::command]
async fn pick_root_folder(window: tauri::Window) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    window.dialog().file().pick_folder(move |path| {
        let _ = tx.send(path);
    });
    // FilePath from tauri-plugin-dialog implements Display; if your installed
    // version returns a different type, convert via
    // `p.into_path().ok().map(|pb| pb.to_string_lossy().into_owned())`.
    rx.recv().ok().flatten().map(|p| p.to_string())
}

#[tauri::command]
async fn scan(
    window: tauri::Window,
    state: State<'_, AppState>,
    root: String,
) -> Result<(), String> {
    state.cancel.store(false, Ordering::Relaxed);
    let cancel = state.cancel.clone();
    let win_proj = window.clone();
    let win_prog = window.clone();
    let win_done = window.clone();
    let root_path = PathBuf::from(root);

    tauri::async_runtime::spawn_blocking(move || {
        let cbs = ScanCallbacks {
            on_project: Box::new(move |p| {
                let _ = win_proj.emit("scan:project", p);
            }),
            on_progress: Box::new(move |p| {
                let _ = win_prog.emit("scan:progress", p);
            }),
            cancel,
        };
        let s = scan_blocking(&root_path, cbs);
        let _ = win_done.emit(
            "scan:done",
            ScanDone {
                cancelled: s.cancelled,
                total_projects: s.projects_found,
                total_size_bytes: s.total_size_bytes,
                dirs_skipped: s.dirs_skipped,
            },
        );
    });

    Ok(())
}

#[tauri::command]
fn cancel_scan(state: State<'_, AppState>) {
    state.cancel.store(true, Ordering::Relaxed);
}

#[tauri::command]
async fn delete_many(paths: Vec<String>) -> Vec<DeleteResult> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    tauri::async_runtime::spawn_blocking(move || crate::deleter::delete_many(paths))
        .await
        .unwrap_or_default()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            pick_root_folder,
            scan,
            cancel_scan,
            delete_many
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo build`
Expected: builds. Fix any compile errors by adjusting the `pick_root_folder` body to match the dialog plugin API on your installed version.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: expose pick_root_folder, scan, cancel_scan, delete_many commands"
```

---

## Task 9: Frontend — empty state and "Choose folder" button

**Files:**
- Modify: `clean-node-modules/src/index.html`
- Modify: `clean-node-modules/src/main.js`
- Modify: `clean-node-modules/src/style.css`

- [ ] **Step 1: Replace `src/index.html`**

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>clean-node-modules</title>
  <link rel="stylesheet" href="style.css" />
</head>
<body>
  <main id="app">
    <header id="topbar">
      <h1>clean-node-modules</h1>
      <div id="status"></div>
      <button id="cancel-btn" hidden>Cancel</button>
    </header>

    <section id="empty-state">
      <p>Choose a folder to scan for <code>node_modules</code> directories.</p>
      <button id="choose-btn">Choose folder…</button>
    </section>

    <section id="results" hidden>
      <table id="results-table">
        <thead>
          <tr>
            <th><input type="checkbox" id="select-all" /></th>
            <th data-sort="path">Project</th>
            <th data-sort="size">Size</th>
            <th data-sort="project_modified">Project modified</th>
            <th data-sort="nm_modified">node_modules modified</th>
          </tr>
        </thead>
        <tbody id="results-body"></tbody>
      </table>
    </section>

    <footer id="footer" hidden>
      <span id="selection-summary">0 selected · 0 B</span>
      <button id="delete-btn" disabled>Delete selected (move to Trash)</button>
    </footer>
  </main>
  <script type="module" src="main.js"></script>
</body>
</html>
```

- [ ] **Step 2: Replace `src/style.css`**

```css
* { box-sizing: border-box; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; margin: 0; color: #222; }
#app { display: flex; flex-direction: column; height: 100vh; }
#topbar { display: flex; align-items: center; gap: 12px; padding: 10px 16px; border-bottom: 1px solid #ddd; }
#topbar h1 { font-size: 16px; margin: 0; }
#status { flex: 1; color: #666; font-size: 13px; }
#empty-state { padding: 40px; text-align: center; }
#results { flex: 1; overflow: auto; padding: 0 16px; }
#results-table { width: 100%; border-collapse: collapse; font-size: 13px; }
#results-table th, #results-table td { padding: 6px 8px; border-bottom: 1px solid #eee; text-align: left; }
#results-table th[data-sort] { cursor: pointer; user-select: none; }
#results-table th[data-sort].active::after { content: " ▾"; }
#results-table th[data-sort].active.asc::after { content: " ▴"; }
#footer { display: flex; gap: 12px; align-items: center; padding: 10px 16px; border-top: 1px solid #ddd; }
#footer #selection-summary { flex: 1; color: #444; }
button { padding: 6px 12px; cursor: pointer; }
button:disabled { opacity: 0.5; cursor: default; }
```

- [ ] **Step 3: Replace `src/main.js`** with the empty-state wiring

```js
import { invoke } from "https://esm.sh/@tauri-apps/api@2/core";

const $ = (sel) => document.querySelector(sel);

$("#choose-btn").addEventListener("click", async () => {
  const root = await invoke("pick_root_folder");
  if (!root) return;
  startScan(root);
});

async function startScan(root) {
  $("#empty-state").hidden = true;
  $("#status").textContent = `Scanning ${root}…`;
  // Wired up in Task 10
  await invoke("scan", { root });
}
```

- [ ] **Step 4: Run the app**

Run: `cd clean-node-modules && cargo tauri dev`

(Use `cargo install tauri-cli --version '^2'` once if `cargo tauri` isn't installed.)

Expected: a window opens; clicking "Choose folder…" opens the OS folder picker; after selecting, the empty state disappears and status shows "Scanning ...". No results render yet.

- [ ] **Step 5: Commit**

```bash
git add src/index.html src/main.js src/style.css
git commit -m "feat(ui): empty state + folder picker wiring"
```

---

## Task 10: Frontend — incremental results table

**Files:**
- Modify: `clean-node-modules/src/main.js`

- [ ] **Step 1: Replace `src/main.js`**

```js
import { invoke } from "https://esm.sh/@tauri-apps/api@2/core";
import { listen } from "https://esm.sh/@tauri-apps/api@2/event";

const $ = (sel) => document.querySelector(sel);

const state = {
  rows: [],            // ProjectInfo[]
  selected: new Set(), // node_modules_path
  sortKey: "size",
  sortDir: "desc",
};

function fmtSize(n) {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = 0; let v = n;
  while (v >= 1024 && i < units.length - 1) { v /= 1024; i++; }
  return `${v.toFixed(v < 10 && i > 0 ? 1 : 0)} ${units[i]}`;
}

function fmtDate(ms) {
  if (!ms) return "—";
  const d = new Date(ms);
  return d.toISOString().slice(0, 10);
}

function compare(a, b) {
  const k = state.sortKey;
  let av, bv;
  if (k === "size") { av = a.size_bytes; bv = b.size_bytes; }
  else if (k === "project_modified") { av = a.project_modified_ms; bv = b.project_modified_ms; }
  else if (k === "nm_modified") { av = a.nm_modified_ms; bv = b.nm_modified_ms; }
  else { av = a.project_path; bv = b.project_path; }
  if (av < bv) return state.sortDir === "asc" ? -1 : 1;
  if (av > bv) return state.sortDir === "asc" ? 1 : -1;
  return 0;
}

function render() {
  const body = $("#results-body");
  body.innerHTML = "";
  const sorted = [...state.rows].sort(compare);
  for (const r of sorted) {
    const tr = document.createElement("tr");
    const checked = state.selected.has(r.node_modules_path) ? "checked" : "";
    tr.innerHTML = `
      <td><input type="checkbox" data-path="${r.node_modules_path}" ${checked}></td>
      <td title="${r.project_path}">${r.project_path}</td>
      <td>${fmtSize(r.size_bytes)}</td>
      <td>${fmtDate(r.project_modified_ms)}</td>
      <td>${fmtDate(r.nm_modified_ms)}</td>
    `;
    body.appendChild(tr);
  }
  document.querySelectorAll("#results-table th[data-sort]").forEach((th) => {
    th.classList.toggle("active", th.dataset.sort === state.sortKey);
    th.classList.toggle("asc", state.sortDir === "asc");
  });
  updateSelectionSummary();
}

function updateSelectionSummary() {
  const sel = state.rows.filter((r) => state.selected.has(r.node_modules_path));
  const total = sel.reduce((s, r) => s + r.size_bytes, 0);
  $("#selection-summary").textContent = `${sel.length} selected · ${fmtSize(total)}`;
  $("#delete-btn").disabled = sel.length === 0;
}

$("#results-body").addEventListener("change", (e) => {
  const cb = e.target;
  if (cb.matches('input[type="checkbox"]')) {
    const p = cb.dataset.path;
    if (cb.checked) state.selected.add(p); else state.selected.delete(p);
    updateSelectionSummary();
  }
});

$("#select-all").addEventListener("change", (e) => {
  if (e.target.checked) state.rows.forEach((r) => state.selected.add(r.node_modules_path));
  else state.selected.clear();
  render();
});

document.querySelectorAll("#results-table th[data-sort]").forEach((th) => {
  th.addEventListener("click", () => {
    const k = th.dataset.sort;
    if (state.sortKey === k) state.sortDir = state.sortDir === "asc" ? "desc" : "asc";
    else { state.sortKey = k; state.sortDir = (k === "path") ? "asc" : "desc"; }
    render();
  });
});

$("#choose-btn").addEventListener("click", async () => {
  const root = await invoke("pick_root_folder");
  if (!root) return;
  startScan(root);
});

$("#cancel-btn").addEventListener("click", () => invoke("cancel_scan"));

async function startScan(root) {
  state.rows = [];
  state.selected.clear();
  $("#empty-state").hidden = true;
  $("#results").hidden = false;
  $("#footer").hidden = false;
  $("#cancel-btn").hidden = false;
  $("#status").textContent = `Scanning ${root}…`;
  render();
  await invoke("scan", { root });
}

await listen("scan:project", (event) => {
  state.rows.push(event.payload);
  render();
});

await listen("scan:progress", (event) => {
  const p = event.payload;
  $("#status").textContent =
    `Scanning… ${p.projects_found} projects, ${fmtSize(p.total_size_bytes)}`;
});

await listen("scan:done", (event) => {
  const d = event.payload;
  $("#cancel-btn").hidden = true;
  $("#status").textContent = d.cancelled
    ? `Cancelled · ${d.total_projects} projects, ${fmtSize(d.total_size_bytes)}`
    : `Done · ${d.total_projects} projects, ${fmtSize(d.total_size_bytes)}`;
});
```

- [ ] **Step 2: Run the app and verify**

Run: `cargo tauri dev`
Expected: pick a folder containing at least one Node project; rows appear incrementally; clicking column headers sorts; checkboxes update the footer summary.

- [ ] **Step 3: Commit**

```bash
git add src/main.js
git commit -m "feat(ui): incremental results table with sort and selection"
```

---

## Task 11: Frontend — confirm dialog and bulk delete

**Files:**
- Modify: `clean-node-modules/src/main.js`

- [ ] **Step 1: Append delete-button handler to `src/main.js`** (place near the other event listeners)

```js
$("#delete-btn").addEventListener("click", async () => {
  const sel = state.rows.filter((r) => state.selected.has(r.node_modules_path));
  if (sel.length === 0) return;
  const totalBytes = sel.reduce((s, r) => s + r.size_bytes, 0);
  const ok = confirm(
    `Move ${sel.length} node_modules folders (${fmtSize(totalBytes)}) to Trash?\n\n` +
    `You can restore them from your OS trash, or run \`npm install\` to recreate them.`
  );
  if (!ok) return;

  $("#delete-btn").disabled = true;
  const paths = sel.map((r) => r.node_modules_path);
  const results = await invoke("delete_many", { paths });

  const okPaths = new Set(results.filter((r) => r.ok).map((r) => r.path));
  state.rows = state.rows.filter((r) => !okPaths.has(r.node_modules_path));
  okPaths.forEach((p) => state.selected.delete(p));

  const failed = results.filter((r) => !r.ok);
  if (failed.length === 0) {
    $("#status").textContent = `Deleted ${results.length} folders.`;
  } else {
    $("#status").textContent = `Deleted ${results.length - failed.length}, ${failed.length} failed.`;
    console.warn("Failed deletes:", failed);
  }
  render();
});
```

- [ ] **Step 2: Run and verify**

Run: `cargo tauri dev`
On a real folder:
1. Scan, select one small node_modules.
2. Click "Delete selected".
3. Confirm dialog appears.
4. After accept: the row disappears, status shows "Deleted 1 folders.", and the OS trash contains the folder.

- [ ] **Step 3: Commit**

```bash
git add src/main.js
git commit -m "feat(ui): bulk delete with confirmation"
```

---

## Task 12: Manual end-to-end smoke + bundle

**Files:**
- (No code changes; this is a verification task.)

- [ ] **Step 1: Run a full smoke test**

Pick a real dev directory containing several Node projects (your own machine).

- [ ] Pick folder → results stream in.
- [ ] Sort by size desc — largest first.
- [ ] Sort by "Project modified" asc — oldest first.
- [ ] Select-all → footer shows correct total.
- [ ] Cancel mid-scan on a deep tree → status reads "Cancelled · …".
- [ ] Delete one small folder → confirm → folder is in OS trash, row gone.

- [ ] **Step 2: Build a release bundle**

Run: `cargo tauri build`
Expected: produces `.dmg` (macOS) or platform-equivalent installer in `src-tauri/target/release/bundle/`.

- [ ] **Step 3: Run all unit tests once more**

Run: `cd src-tauri && cargo test`
Expected: all tests pass.

- [ ] **Step 4: Commit any incidental fixes; tag v0.1.0**

```bash
# (only if there were changes)
git add -A
git commit -m "chore: smoke-test fixes"
git tag v0.1.0
```

---

## Notes & known limitations (carry into v2)

- No permanent-delete fallback when trash isn't supported by the filesystem.
- No multi-root configuration UI.
- Permission-denied counter (`dirs_skipped`) is wired through types but not yet incremented by the scanner (the `jwalk` errors are silently dropped). When v2 adds proper error visibility, increment `dirs_skipped` from `process_read_dir`.
- Symlinks are skipped; if a user wants symlink support later, add an opt-in setting.
