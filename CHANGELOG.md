# Changelog

## v0.1.0 — 2026-05-06

First public release. Tauri 2 desktop app that scans a chosen folder for `node_modules` directories and lets you bulk-delete them to reclaim disk space.

### Features

- **Folder picker + scan** — native macOS folder dialog, scans for top-level `node_modules` recursively (skips hidden dirs like `.git`, `.cache`).
- **Two delete modes**
  - **Move to Trash** — uses `trash` crate, recoverable from OS Trash, single-click confirm.
  - **Delete permanently** — `std::fs::remove_dir_all`, bypasses the Trash. Requires typing `DELETE` in the confirm dialog before the OK button enables.
- **Per-item progress**
  - Bottom progress bar with current path and `i / N` count.
  - Each row pulses red while being deleted, then fades + slides out on success. Failed rows stay marked with a ⚠ and error tooltip.
  - Trash deletes (which complete in <1ms) are throttled per-frame so the cascade is visible instead of all flushing in one paint.
- **Sort & select** — by project path, size, project mtime, or `node_modules` mtime; bulk select-all; selection summary shows reclaimable size.
- **Cancel-able scan** — click Cancel mid-scan; "New scan…" in the topbar resets and restarts at any time.

### UI

- **Aurora Glass** — gradient teal→cyan background matching the app icon, frosted glass panels (topbar, results, footer, modal) with backdrop blur.
- Monospace typography app-wide (JetBrains Mono / SF Mono / Menlo fallback) for clean alignment of paths, sizes, and dates.
- Pill buttons; primary actions (New scan, Choose folder) use the gradient with glow; danger uses red gradient.
- Custom `<dialog>` modal replaces `window.confirm` / `window.prompt`, which silently no-op in Tauri 2 WKWebView.

### Fixes / engineering

- **pnpm sizing** — `dir_size` now walks hidden subdirs. pnpm stores all packages under `node_modules/.pnpm/`, which jwalk skipped by default, causing pnpm-managed projects to report 0 Bytes. Fixed with `.skip_hidden(false)` on the size walk; project-mtime walk still skips hidden so `.git` activity doesn't pollute "last edited".
- Tauri bundler now ships icons — `tauri.conf.json` was missing the `bundle.icon` array, so `.app` had no `Resources/icon.icns`. Now bundled correctly.
- App identity — `productName` and window title set to "Clean Node Modules" (was kebab-case).

### Install

- **Apple Silicon (M1/M2/M3/M4)** — download `Clean Node Modules_0.1.0_aarch64.dmg`, open, drag to Applications, launch.
- **Intel** — build from source: `rustup target add x86_64-apple-darwin && cargo tauri build --target x86_64-apple-darwin`.

The build is **unsigned and unnotarized**. First launch may require *System Settings → Privacy & Security → "Open Anyway"*, or right-click the app → Open → confirm.

### Limitations

- Cancel only checks between projects, not inside `dir_size` / `newest_mtime`. A multi-GB `node_modules` will delay cancel by a few seconds.
- Single root folder per session.
- Permission errors during scan/size are silently swallowed (`dirs_skipped` is plumbed but always `0`).
