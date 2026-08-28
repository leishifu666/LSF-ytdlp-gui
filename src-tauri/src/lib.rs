mod downloader;
mod updater;

use downloader::JobManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .manage(JobManager::default())
        .invoke_handler(tauri::generate_handler![
            downloader::start_download,
            downloader::cancel_download,
            downloader::list_jobs,
            downloader::clear_finished,
            downloader::fetch_info,
            downloader::ytdlp_version,
            downloader::update_ytdlp_now,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
