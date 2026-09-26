use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

#[cfg(target_os = "linux")]
use std::ffi::CStr;
#[cfg(target_os = "linux")]
use std::os::raw::c_void;
#[cfg(target_os = "linux")]
use std::ptr;

#[cfg(target_os = "linux")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tauri::{AppHandle, Manager};
use tauri_plugin_libmpv::MpvExt;

use crate::playlist::{sanitize_session, PlaylistItem, PlaylistSession};

pub struct ThumbnailCaptureState {
    generation: Arc<AtomicU64>,
    capture_lock: Arc<Mutex<()>>,
}

impl Default for ThumbnailCaptureState {
    fn default() -> Self {
        Self {
            generation: Arc::new(AtomicU64::new(0)),
            capture_lock: Arc::new(Mutex::new(())),
        }
    }
}

#[tauri::command]
pub fn path_exists(path: String) -> bool {
    std::path::Path::new(&path).is_file()
}

const SUPPORTED_VIDEO_EXTENSIONS: [&str; 14] = [
    "3gp", "avi", "flac", "m4v", "mkv", "mov", "mp4", "mpeg", "mpg", "ogg", "ogv", "ts", "webm",
    "wmv",
];

#[tauri::command]
pub fn cancel_thumbnail_capture(state: tauri::State<'_, ThumbnailCaptureState>) {
    state.generation.fetch_add(1, Ordering::SeqCst);
}

#[cfg(target_os = "linux")]
fn is_mpv_window(display: *mut x11::xlib::Display, window: x11::xlib::Window) -> bool {
    unsafe {
        let mut class_hint = x11::xlib::XClassHint {
            res_name: ptr::null_mut(),
            res_class: ptr::null_mut(),
        };
        if x11::xlib::XGetClassHint(display, window, &mut class_hint) == 0 {
            return false;
        }

        let class_name = if class_hint.res_class.is_null() {
            String::new()
        } else {
            CStr::from_ptr(class_hint.res_class)
                .to_string_lossy()
                .to_ascii_lowercase()
        };
        let resource_name = if class_hint.res_name.is_null() {
            String::new()
        } else {
            CStr::from_ptr(class_hint.res_name)
                .to_string_lossy()
                .to_ascii_lowercase()
        };

        if !class_hint.res_name.is_null() {
            x11::xlib::XFree(class_hint.res_name as *mut c_void);
        }
        if !class_hint.res_class.is_null() {
            x11::xlib::XFree(class_hint.res_class as *mut c_void);
        }

        class_name == "mpv" || resource_name == "mpv"
    }
}

#[cfg(target_os = "linux")]
fn find_mpv_window(
    display: *mut x11::xlib::Display,
    window: x11::xlib::Window,
) -> Option<(x11::xlib::Window, x11::xlib::Window)> {
    unsafe {
        let mut root: x11::xlib::Window = 0;
        let mut parent: x11::xlib::Window = 0;
        let mut children: *mut x11::xlib::Window = ptr::null_mut();
        let mut child_count = 0;
        if x11::xlib::XQueryTree(
            display,
            window,
            &mut root,
            &mut parent,
            &mut children,
            &mut child_count,
        ) == 0
        {
            return None;
        }

        let found = if children.is_null() {
            None
        } else {
            let child_slice = std::slice::from_raw_parts(children, child_count as usize);
            child_slice.iter().find_map(|&child| {
                if is_mpv_window(display, child) {
                    Some((child, window))
                } else {
                    find_mpv_window(display, child)
                }
            })
        };

        if !children.is_null() {
            x11::xlib::XFree(children as *mut c_void);
        }

        found
    }
}

#[cfg(target_os = "linux")]
fn top_level_window(
    display: *mut x11::xlib::Display,
    window: x11::xlib::Window,
) -> x11::xlib::Window {
    unsafe {
        let mut current = window;
        loop {
            let mut root: x11::xlib::Window = 0;
            let mut parent: x11::xlib::Window = 0;
            let mut children: *mut x11::xlib::Window = ptr::null_mut();
            let mut child_count = 0;
            let queried = x11::xlib::XQueryTree(
                display,
                current,
                &mut root,
                &mut parent,
                &mut children,
                &mut child_count,
            );
            if !children.is_null() {
                x11::xlib::XFree(children as *mut c_void);
            }
            if queried == 0 || parent == 0 || parent == root || parent == current {
                return current;
            }
            current = parent;
        }
    }
}

fn is_supported_video_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            SUPPORTED_VIDEO_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
}

#[tauri::command]
pub fn expand_drop_paths(paths: Vec<String>) -> Vec<String> {
    let mut expanded = Vec::new();

    for raw_path in paths {
        let path = std::path::PathBuf::from(&raw_path);
        if path.is_file() {
            if is_supported_video_path(&path) {
                expanded.push(raw_path);
            }
            continue;
        }

        if !path.is_dir() {
            continue;
        }

        let Ok(entries) = std::fs::read_dir(path) else {
            continue;
        };

        for entry in entries.flatten() {
            let child = entry.path();
            if child.is_file() && is_supported_video_path(&child) {
                expanded.push(child.to_string_lossy().into_owned());
            }
        }
    }

    expanded
}

const EQUALIZER_FREQUENCIES: [u32; 5] = [60, 250, 1_000, 4_000, 12_000];

#[tauri::command]
pub async fn set_playback_speed<R: tauri::Runtime>(
    app: AppHandle<R>,
    speed: f64,
    window_label: String,
) -> Result<(), String> {
    if !speed.is_finite() || !(0.5..=2.0).contains(&speed) {
        return Err(format!(
            "playback speed must be between 0.5 and 2.0, received {speed}"
        ));
    }

    tauri::async_runtime::spawn_blocking(move || {
        app.mpv()
            .set_property("speed", &serde_json::Value::from(speed), &window_label)
            .map_err(|error| format!("libmpv rejected playback speed: {error}"))
    })
    .await
    .map_err(|error| format!("playback speed command failed: {error}"))?
}

#[tauri::command]
pub async fn capture_thumbnail<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, ThumbnailCaptureState>,
    position: f64,
    window_label: String,
) -> Result<Vec<u8>, String> {
    if !position.is_finite() || position < 0.0 {
        return Err("thumbnail position must be a non-negative number".into());
    }

    let generation = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let generations = Arc::clone(&state.generation);
    let capture_lock = Arc::clone(&state.capture_lock);

    tauri::async_runtime::spawn_blocking(move || {
        let _capture_guard = capture_lock
            .lock()
            .map_err(|_| "thumbnail capture lock was poisoned".to_string())?;
        let is_current = || generations.load(Ordering::SeqCst) == generation;
        if !is_current() {
            return Err("thumbnail capture cancelled".to_string());
        }

        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!(
            "atiga-cine-thumbnail-{}-{stamp}.png",
            std::process::id()
        ));
        let mpv = app.mpv();
        let original_position = mpv
            .get_property("time-pos".into(), "double".into(), &window_label)
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(position);
        let original_pause = mpv
            .get_property("pause".into(), "flag".into(), &window_label)
            .ok()
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

        let result = (|| {
            if !is_current() {
                return Err("thumbnail capture cancelled".to_string());
            }
            mpv.set_property("pause", &serde_json::Value::Bool(true), &window_label)
                .map_err(|error| format!("could not pause for thumbnail: {error}"))?;
            if !is_current() {
                return Err("thumbnail capture cancelled".to_string());
            }
            mpv.set_property(
                "time-pos",
                &serde_json::Value::from(position),
                &window_label,
            )
            .map_err(|error| format!("could not seek for thumbnail: {error}"))?;
            mpv.command(
                "screenshot-to-file",
                &vec![
                    serde_json::Value::String(path.to_string_lossy().into_owned()),
                    serde_json::Value::String("video".into()),
                ],
                &window_label,
            )
            .map_err(|error| format!("could not capture thumbnail: {error}"))?;

            for _ in 0..10 {
                if !is_current() {
                    return Err("thumbnail capture cancelled".to_string());
                }
                if let Ok(bytes) = std::fs::read(&path) {
                    if !bytes.is_empty() {
                        if !is_current() {
                            return Err("thumbnail capture cancelled".to_string());
                        }
                        return Ok(bytes);
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(15));
            }

            Err("libmpv did not produce a thumbnail".into())
        })();

        let restore_position = mpv.set_property(
            "time-pos",
            &serde_json::Value::from(original_position),
            &window_label,
        );
        let restore_pause = mpv.set_property(
            "pause",
            &serde_json::Value::Bool(original_pause),
            &window_label,
        );
        let _ = std::fs::remove_file(&path);

        if let Err(error) = restore_position {
            return Err(format!(
                "thumbnail captured but position restore failed: {error}"
            ));
        }
        if let Err(error) = restore_pause {
            return Err(format!(
                "thumbnail captured but playback restore failed: {error}"
            ));
        }
        result
    })
    .await
    .map_err(|error| format!("thumbnail task failed: {error}"))?
}

#[tauri::command]
pub fn sync_mpv_video<R: tauri::Runtime>(
    app: AppHandle<R>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    visible: bool,
    window_label: String,
) -> Result<(), String> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (app, x, y, width, height, visible, window_label);
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        if ![x, y, width, height].iter().all(|value| value.is_finite()) {
            return Err("native video bounds must be finite".into());
        }

        let window = app
            .get_webview_window(&window_label)
            .ok_or_else(|| format!("window '{window_label}' not found"))?;
        let handle = window
            .window_handle()
            .map_err(|error| format!("could not get native window handle: {error}"))?;
        let parent = match handle.as_raw() {
            RawWindowHandle::Xlib(handle) => handle.window,
            RawWindowHandle::Xcb(handle) => handle.window.get() as x11::xlib::Window,
            RawWindowHandle::Wayland(_) => return Ok(()),
            _ => return Ok(()),
        };

        let display = unsafe { x11::xlib::XOpenDisplay(ptr::null()) };
        if display.is_null() {
            return Ok(());
        }

        unsafe {
            let top_level = top_level_window(display, parent);
            let Some((mpv_window, mpv_parent)) = find_mpv_window(display, top_level) else {
                x11::xlib::XCloseDisplay(display);
                return Ok(());
            };

            if visible {
                let mut x = x.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32;
                let mut y = y.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32;
                if mpv_parent != top_level {
                    let mut translated_x = 0;
                    let mut translated_y = 0;
                    let mut translated_child = 0;
                    if x11::xlib::XTranslateCoordinates(
                        display,
                        top_level,
                        mpv_parent,
                        x,
                        y,
                        &mut translated_x,
                        &mut translated_y,
                        &mut translated_child,
                    ) != 0
                    {
                        x = translated_x;
                        y = translated_y;
                    }
                }
                let width = width.max(1.0).round().min(u32::MAX as f64) as u32;
                let height = height.max(1.0).round().min(u32::MAX as f64) as u32;
                x11::xlib::XMoveResizeWindow(display, mpv_window, x, y, width, height);
                x11::xlib::XMapRaised(display, mpv_window);
            } else {
                x11::xlib::XUnmapWindow(display, mpv_window);
            }
            x11::xlib::XFlush(display);
            x11::xlib::XCloseDisplay(display);
            Ok(())
        }
    }
}

#[derive(serde::Deserialize)]
pub struct EqualizerRequest {
    pub enabled: bool,
    pub gains: Vec<f64>,
    #[serde(default)]
    pub normalize: bool,
}

fn clamp_gain(gain: f64) -> f64 {
    if gain.is_finite() {
        gain.clamp(-12.0, 12.0)
    } else {
        0.0
    }
}

fn build_audio_filter(
    equalizer_enabled: bool,
    gains: &[f64],
    normalize: bool,
) -> Result<String, String> {
    let mut chain = Vec::new();
    if equalizer_enabled {
        if gains.len() != EQUALIZER_FREQUENCIES.len() {
            return Err(format!(
                "equalizer expected {} bands, received {}",
                EQUALIZER_FREQUENCIES.len(),
                gains.len()
            ));
        }

        chain.extend(
            EQUALIZER_FREQUENCIES
                .iter()
                .zip(gains)
                .map(|(frequency, gain)| {
                    format!(
                        "equalizer=f={frequency}:width_type=o:w=1:g={:.1}",
                        clamp_gain(*gain)
                    )
                }),
        );
    }

    if normalize {
        chain.push("dynaudnorm".to_string());
    }

    if chain.is_empty() {
        return Ok(String::new());
    }

    Ok(format!("lavfi=[{}]", chain.join(",")))
}

#[tauri::command]
pub async fn set_equalizer<R: tauri::Runtime>(
    app: AppHandle<R>,
    equalizer: EqualizerRequest,
    window_label: String,
) -> Result<(), String> {
    let filter = build_audio_filter(equalizer.enabled, &equalizer.gains, equalizer.normalize)?;

    tauri::async_runtime::spawn_blocking(move || {
        app.mpv()
            .set_property("af", &serde_json::Value::String(filter), &window_label)
            .map_err(|error| format!("libmpv rejected equalizer filter: {error}"))
    })
    .await
    .map_err(|error| format!("equalizer command failed: {error}"))?
}

fn session_path<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|directory| directory.join("playlist.json"))
        .map_err(|error| format!("could not resolve app data directory: {error}"))
}

#[tauri::command]
pub async fn load_playlist_session<R: tauri::Runtime>(
    app: AppHandle<R>,
) -> Result<PlaylistSession, String> {
    let path = session_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(PlaylistSession {
                    playlists: Vec::new(),
                    active_playlist_id: None,
                })
            }
            Err(error) => return Err(format!("could not read playlist session: {error}")),
        };

        if let Ok(session) = serde_json::from_str::<PlaylistSession>(&content) {
            return Ok(sanitize_session(session));
        }

        let legacy_items = serde_json::from_str::<Vec<PlaylistItem>>(&content)
            .map_err(|error| format!("could not parse playlist session: {error}"))?;
        Ok(PlaylistSession {
            playlists: vec![crate::playlist::Playlist {
                id: "default-playlist".into(),
                name: "My Playlist".into(),
                items: crate::playlist::sanitize(legacy_items),
            }],
            active_playlist_id: Some("default-playlist".into()),
        })
    })
    .await
    .map_err(|error| format!("playlist load task failed: {error}"))?
}

#[tauri::command]
pub async fn save_playlist_session<R: tauri::Runtime>(
    app: AppHandle<R>,
    session: PlaylistSession,
) -> Result<(), String> {
    let path = session_path(&app)?;
    let session = sanitize_session(session);

    tauri::async_runtime::spawn_blocking(move || {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("could not create app data directory: {error}"))?;
        }

        let content = serde_json::to_vec_pretty(&session)
            .map_err(|error| format!("could not encode playlist session: {error}"))?;
        let temporary_path = path.with_extension("json.tmp");
        std::fs::write(&temporary_path, content)
            .map_err(|error| format!("could not write playlist session: {error}"))?;
        std::fs::rename(&temporary_path, &path)
            .map_err(|error| format!("could not commit playlist session: {error}"))
    })
    .await
    .map_err(|error| format!("playlist save task failed: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::{build_audio_filter, expand_drop_paths};

    #[test]
    fn builds_one_lavfi_chain_for_all_bands() {
        let filter =
            build_audio_filter(true, &[7.0, 5.0, 2.0, 0.0, -3.5], false).expect("valid bands");

        assert_eq!(
            filter,
            "lavfi=[equalizer=f=60:width_type=o:w=1:g=7.0,equalizer=f=250:width_type=o:w=1:g=5.0,equalizer=f=1000:width_type=o:w=1:g=2.0,equalizer=f=4000:width_type=o:w=1:g=0.0,equalizer=f=12000:width_type=o:w=1:g=-3.5]"
        );
    }

    #[test]
    fn chains_normalization_after_equalizer() {
        let filter = build_audio_filter(true, &[0.0; 5], true).expect("valid bands");
        assert!(filter.ends_with(",dynaudnorm]"));
    }

    #[test]
    fn can_normalize_without_enabling_equalizer() {
        assert_eq!(
            build_audio_filter(false, &[], true).expect("normalization only"),
            "lavfi=[dynaudnorm]"
        );
    }

    #[test]
    fn rejects_partial_band_arrays() {
        assert!(build_audio_filter(true, &[0.0, 0.0], false).is_err());
    }

    #[test]
    fn expands_only_supported_top_level_video_files() {
        let root = std::env::temp_dir().join(format!("atiga-cine-drop-{}", std::process::id()));
        let nested = root.join("nested");
        std::fs::create_dir_all(&nested).expect("create test folders");
        let mp4 = root.join("lesson.mp4");
        let uppercase = root.join("LECTURE.MKV");
        let subtitle = root.join("lesson.srt");
        let nested_video = nested.join("hidden.mp4");
        for path in [&mp4, &uppercase, &subtitle, &nested_video] {
            std::fs::write(path, b"test").expect("create test file");
        }

        let result = expand_drop_paths(vec![root.to_string_lossy().into_owned()]);

        assert_eq!(result.len(), 2);
        assert!(result.contains(&mp4.to_string_lossy().into_owned()));
        assert!(result.contains(&uppercase.to_string_lossy().into_owned()));
        assert!(!result.contains(&nested_video.to_string_lossy().into_owned()));
        std::fs::remove_dir_all(root).expect("remove test folders");
    }
}
