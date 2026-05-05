# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`clean-node-modules` is a Tauri 2 desktop app (macOS / Windows / Linux) that scans a chosen folder for `node_modules` directories and lets the user bulk-move them to the OS trash to reclaim disk space. The full design and task-by-task implementation plan are in `docs/superpowers/specs/` and `docs/superpowers/plans/` — read them first if you need context the code alone doesn't give.

## Commands

All run from the repo root unless noted.

```bash
# Dev loop (opens a window with hot reload of Rust + frontend)
cargo tauri dev

# Production bundle (.app + .dmg on macOS)
cargo tauri build
# Output: src-tauri/target/release/bundle/{macos,dmg}/

# Unit + integration tests (Rust only — frontend is plain JS, no test runner)
cd src-tauri && cargo test

# Run a single integration test file
cd src-tauri && cargo test --test scanner_test
cd src-tauri && cargo test --test scanner_test -- finds_top_level_node_modules

# Plain build (compile check, no bundling)
cd src-tauri && cargo build
```

`cargo tauri` requires `tauri-cli` 2.x (`cargo install tauri-cli --version "^2" --locked`).

## Architecture

### Two halves connected by Tauri IPC

**Backend (`src-tauri/src/`)** — pure Rust, broken into single-responsibility modules. Each one is independently unit-testable; integration tests in `src-tauri/tests/` exercise them via the `clean_node_modules_lib` library crate.

- `types.rs` — `Serialize`-only structs that form the wire contract with the frontend (`ProjectInfo`, `DeleteResult`, `ScanProgress`, `ScanDone`).
- `sizer.rs` — `dir_size(&Path) -> u64`, parallel walk via `jwalk`, errors silently swallowed.
- `mtime.rs` — `newest_mtime` / `newest_mtime_skipping`. The skipping variant is how the scanner gets a project's "last activity" without traversing the heavy `node_modules` subtree.
- `scanner.rs` — `scan_blocking(root, ScanCallbacks)`. Two-phase: (1) walk to discover all top-level `node_modules` paths, marking them as leaves via `entry.read_children_path = None` so `jwalk` doesn't descend into them; (2) for each, compute size + mtimes and fire `on_project` then `on_progress`. Cancel checked between projects.
- `deleter.rs` — `delete_many(Vec<PathBuf>) -> Vec<DeleteResult>` via the `trash` crate.
- `lib.rs` — Tauri command registration, `AppState { cancel: Arc<AtomicBool> }`, `spawn_blocking` plumbing.

**Frontend (`src/`)** — vanilla HTML/CSS/JS, no bundler. ESM imports `@tauri-apps/api` directly from `esm.sh`. All UI logic lives in `src/main.js`.

### Wire contract

JS calls four commands via `invoke`:

| Command | Returns | Notes |
|---|---|---|
| `pick_root_folder` | `string \| null` | Native folder picker via `tauri-plugin-dialog`. |
| `scan({ root })` | `Ok` immediately | Real work runs in `spawn_blocking`; results stream as events. |
| `cancel_scan` | — | Sets the atomic cancel flag. |
| `delete_many({ paths })` | `Result<DeleteResult[], string>` | Per-path failures encoded inside `DeleteResult`; rejects only on join-error/panic. |

Rust → JS events:
- `scan:project` — one `ProjectInfo` per discovered `node_modules`.
- `scan:progress` — cumulative running totals (one per project).
- `scan:done` — fired exactly once at end, with `cancelled` flag.

`PathBuf` serializes as a JSON string. All time values are epoch milliseconds (`u64`). Field names are snake_case on both sides — do not add `#[serde(rename_all = "camelCase")]` without also updating `src/main.js`.

### Tauri 2 ACL gotcha

`tauri-plugin-dialog`'s `pick_folder` is gated by capability ACLs at runtime — `cargo build` won't catch a missing permission. The capability file lives at `src-tauri/capabilities/default.json`; it grants `core:default` and `dialog:allow-open` to the window labeled `"main"`. The `tauri.conf.json` window block has `"label": "main"` so this match is explicit. Adding a new plugin command means updating both files.

## v1 deliberate limitations

These are documented decisions, not bugs — don't "fix" without checking the spec:

- `dirs_skipped` is plumbed through types but always reported as `0` (no permission-error counter yet).
- Cancel only checks between projects, not inside `dir_size` or `newest_mtime`. A 5 GB `node_modules` will delay cancel by seconds.
- No permanent-delete fallback when trash fails — the error surfaces, that's it.
- Single root folder per session (not multi-root).
- macOS aarch64 only by default; for universal: `cargo tauri build --target universal-apple-darwin` plus `rustup target add x86_64-apple-darwin`.

## When changing the scanner

The two-phase walk is intentional: we collect all `node_modules` paths first because `jwalk`'s parallel traversal can't easily interleave with per-project metadata work. If you change to streaming discovery, watch out for:
- `process_read_dir` runs on jwalk's worker threads — your callback closure capture must be `Send + Sync`.
- The `entry.read_children_path = None` trick is what prevents recursion into a `node_modules` once found. Removing it will explode `dir_size` calls and discovery into nested deps.
- `skip_hidden(true)` is what skips `.cache/`, `.git/`, etc.
