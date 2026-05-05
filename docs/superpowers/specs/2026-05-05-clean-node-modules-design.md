# clean-node-modules — Design

**Date:** 2026-05-05
**Status:** Approved

## Goal

A cross-platform desktop app (macOS, Windows, Linux) that scans a user-chosen
folder for projects containing `node_modules` directories and lets the user
reclaim disk space by moving selected `node_modules` folders to the OS trash.

UI language: **English**.

## Non-goals (v1)

- Scheduled / automatic cleanup.
- Cleaning pnpm store, yarn cache, or other package-manager caches.
- Special handling for monorepo workspace tooling beyond "don't recurse into
  node_modules".
- Multi-root configuration (single chosen root for v1; can be extended later).
- Permanent delete fallback when trash fails (v1 surfaces the error only).

## Tech stack

- **Tauri 2.x** — Rust backend, web frontend, single binary per OS.
- **Frontend**: HTML + CSS + vanilla JS. No framework — UI is small enough that
  added complexity isn't justified.
- **Rust crates**:
  - `jwalk` — parallel directory walking.
  - `trash` — cross-platform move-to-trash.
  - `serde` / `serde_json` — Tauri command payloads.
  - `tauri-plugin-dialog` — native folder picker.
- **Distribution**: `.dmg` (macOS), `.msi` (Windows), `.AppImage` and `.deb` (Linux).

## Repository layout

```
clean-node-modules/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs       # Tauri setup, command registration
│   │   ├── scanner.rs    # find node_modules dirs
│   │   ├── sizer.rs      # compute folder size
│   │   └── deleter.rs    # move to trash
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── index.html
│   ├── main.js
│   └── style.css
├── package.json          # frontend build glue (Tauri CLI)
└── docs/superpowers/specs/2026-05-05-clean-node-modules-design.md
```

## Data model

```rust
struct ProjectInfo {
    project_path: PathBuf,       // parent directory of node_modules
    node_modules_path: PathBuf,
    size_bytes: u64,
    project_modified: SystemTime, // newest mtime in project, excluding node_modules
    nm_modified: SystemTime,      // newest mtime inside node_modules
}
```

Frontend receives this serialized as JSON (paths as strings, times as ISO-8601
or epoch millis — pick at implementation time).

## Tauri commands (backend → frontend boundary)

1. **`pick_root_folder() -> Option<String>`**
   Opens native folder-picker dialog. Returns chosen path or None on cancel.

2. **`scan(root: String)`**
   Streams results via Tauri events. Three event types:
   - `scan:project` — payload `ProjectInfo`, one per discovered node_modules.
   - `scan:progress` — `{ projects_found, total_size_bytes, dirs_skipped }`.
   - `scan:done` — final summary, fired once.
   - `scan:error` — surfaced (e.g., root unreadable).
   Frontend can call `cancel_scan()` (atomic flag) to stop early.

3. **`cancel_scan()`** — sets cancel flag for current scan.

4. **`delete_many(paths: Vec<String>) -> Vec<DeleteResult>`**
   `DeleteResult { path: String, ok: bool, error: Option<String> }`.
   Uses `trash` crate. Returns per-item results so the UI can update.

## Scanner behaviour

- Walks from chosen root using `jwalk` for parallelism.
- When a directory named `node_modules` is encountered, record it and **do not
  recurse** into it. This keeps scans fast and prevents accidentally finding
  nested `node_modules` belonging to dependencies.
- Skip dot-directories (`.git`, `.cache`, etc.) by default.
- **Do not follow symlinks** — avoids loops and avoids deleting files outside
  the chosen tree.
- Permission-denied errors during walking are counted (shown in UI) but do not
  abort the scan.
- Size computation walks the contents of each `node_modules` and sums file
  sizes (also via `jwalk`, parallel).
- `project_modified` excludes anything inside `node_modules` so it reflects
  user code activity, not dependency installs.

## Frontend flow

1. **Empty state**: a single "Choose folder…" button.
2. **Scanning state**: progress strip at top — "Scanning… 12 projects found,
   3.4 GB total, 8 folders skipped". Results list populates incrementally as
   `scan:project` events arrive. "Cancel" button visible.
3. **Results state**: table/list with columns:
   - checkbox
   - project path (relative to chosen root)
   - size (human-readable)
   - project last modified
   - node_modules last modified
4. **Sorting**: clickable column headers. Default sort: size desc.
   Other useful sorts: project modified asc (oldest first → likely safe to nuke).
5. **Selection footer**: "N projects selected · X GB · [Delete selected (move
   to Trash)]". Disabled when nothing selected.
6. **Confirmation dialog**: "Move N node_modules folders (X GB) to Trash?
   You can restore them from your OS trash, or run `npm install` to recreate
   them." [Cancel] [Move to Trash].
7. **Post-delete**: deleted rows fade out and are removed from the list. Toast
   summarises results — e.g., "12 deleted, 1 failed (see details)". Failed
   rows stay with an error indicator.

## Error handling

| Situation | Behaviour |
|---|---|
| Permission denied during walk | Skip, increment `dirs_skipped` counter shown in UI. |
| Symlink encountered | Skip, do not follow. |
| Trash operation fails (e.g., unsupported FS) | Per-item error in `DeleteResult`, surfaced in toast and inline; v1 does **not** offer permanent-delete fallback. |
| Scan cancelled by user | Scanner checks atomic flag between dirs; emits `scan:done` with `cancelled: true`. |
| node_modules locked (Windows) | Trash returns error; surface to user. |
| Root path no longer exists | `scan:error` event, return to empty state. |

## Testing

**Rust unit tests** (`src-tauri/src/`):

- `scanner`:
  - finds top-level `node_modules`
  - does not recurse into a `node_modules`
  - skips dot-dirs
  - does not follow symlinks
  - tolerates permission-denied subtree
- `sizer`:
  - sums file sizes correctly across nested files
  - handles empty directory
- `deleter`: integration-style test using a temp dir; assert path no longer
  exists at original location (don't assert OS trash internals).

**Manual UI tests**:

- Run on a real dev folder containing several Node projects.
- Verify cancel mid-scan works.
- Verify bulk delete of 10+ entries.
- Verify a locked `node_modules` (held open in another process) surfaces an
  error rather than silently failing.

## Open questions resolved during brainstorming

- Framework: Tauri (over egui / iced).
- Scan scope: user picks a single root folder per session.
- Display: full info (size + project mtime + nm mtime) with checkboxes for bulk.
- Delete safety: confirm dialog + move to OS trash via `trash` crate.
- UI language: English.
