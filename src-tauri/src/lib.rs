pub mod deleter;
pub mod mtime;
pub mod scanner;
pub mod sizer;
pub mod types;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
