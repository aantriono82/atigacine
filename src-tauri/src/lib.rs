mod commands;
mod playlist;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::ThumbnailCaptureState::default())
        .plugin(tauri_plugin_libmpv::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::load_playlist_session,
            commands::save_playlist_session,
            commands::path_exists,
            commands::expand_drop_paths,
            commands::set_equalizer,
            commands::set_playback_speed,
            commands::cancel_thumbnail_capture,
            commands::capture_thumbnail,
            commands::prepare_mpv_container,
            commands::sync_mpv_video
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
