use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

#[cfg(target_os = "linux")]
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::ffi::CStr;
#[cfg(target_os = "linux")]
use std::os::raw::c_void;
#[cfg(target_os = "linux")]
use std::ptr;

#[cfg(target_os = "linux")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tauri::{AppHandle, Listener, Manager};
use tauri_plugin_libmpv::MpvExt;

use crate::audio_levels::{self, AudioLevelState};
use crate::playlist::{sanitize_session, PlaylistItem, PlaylistSession};

pub struct ThumbnailCaptureState {
    generation: Arc<AtomicU64>,
    capture_lock: Arc<Mutex<()>>,
    /// Window label -> media path loaded in that window's thumbnail decoder.
    sources: Arc<Mutex<std::collections::HashMap<String, String>>>,
}

impl Default for ThumbnailCaptureState {
    fn default() -> Self {
        Self {
            generation: Arc::new(AtomicU64::new(0)),
            capture_lock: Arc::new(Mutex::new(())),
            sources: Arc::new(Mutex::new(std::collections::HashMap::new())),
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

#[cfg(target_os = "linux")]
fn physical_video_bounds(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    device_scale_factor: f64,
) -> Result<(i32, i32, u32, u32), String> {
    if ![x, y, width, height, device_scale_factor]
        .iter()
        .all(|value| value.is_finite())
        || width <= 0.0
        || height <= 0.0
        || device_scale_factor <= 0.0
    {
        return Err(
            "native video bounds and device scale factor must be finite and positive".into(),
        );
    }

    let clamp_coordinate =
        |value: f64| value.clamp(i32::MIN as f64, i32::MAX as f64).round() as i32;
    // DOMRect is expressed in post-transform CSS pixels. X11 expects physical
    // pixels, so use the WebView's DPR from the same layout snapshot. Expanding
    // the outer edges avoids a one-pixel seam at fractional TV scale factors.
    let left = clamp_coordinate((x * device_scale_factor).floor());
    let top = clamp_coordinate((y * device_scale_factor).floor());
    let right = clamp_coordinate(((x + width) * device_scale_factor).ceil());
    let bottom = clamp_coordinate(((y + height) * device_scale_factor).ceil());
    let physical_width = (i64::from(right) - i64::from(left))
        .max(1)
        .min(i64::from(u32::MAX)) as u32;
    let physical_height = (i64::from(bottom) - i64::from(top))
        .max(1)
        .min(i64::from(u32::MAX)) as u32;

    Ok((left, top, physical_width, physical_height))
}

#[cfg(target_os = "linux")]
#[link(name = "Xext")]
extern "C" {
    fn XShapeQueryExtension(
        display: *mut x11::xlib::Display,
        event_base: *mut std::os::raw::c_int,
        error_base: *mut std::os::raw::c_int,
    ) -> x11::xlib::Bool;
    fn XShapeCombineRectangles(
        display: *mut x11::xlib::Display,
        dest: x11::xlib::Window,
        dest_kind: std::os::raw::c_int,
        x_off: std::os::raw::c_int,
        y_off: std::os::raw::c_int,
        rectangles: *mut x11::xlib::XRectangle,
        n_rects: std::os::raw::c_int,
        op: std::os::raw::c_int,
        ordering: std::os::raw::c_int,
    );
}

/// Clips the native video host to a rounded rectangle so the TV screen's
/// curved rim, drawn by the webview underneath, stays visible.
#[cfg(target_os = "linux")]
unsafe fn shape_video_host(
    display: *mut x11::xlib::Display,
    host: x11::xlib::Window,
    width: u32,
    height: u32,
    radii: [f64; 4],
) {
    const SHAPE_BOUNDING: std::os::raw::c_int = 0;
    const SHAPE_SET: std::os::raw::c_int = 0;
    const YX_BANDED: std::os::raw::c_int = 3;

    let (mut event_base, mut error_base) = (0, 0);
    if XShapeQueryExtension(display, &mut event_base, &mut error_base) == 0 {
        return;
    }
    let mut rectangles: Vec<x11::xlib::XRectangle> = rounded_rect_bands(width, height, radii)
        .into_iter()
        .map(|(x, y, width, height)| x11::xlib::XRectangle {
            x: x.clamp(0, i16::MAX as u32) as i16,
            y: y.clamp(0, i16::MAX as u32) as i16,
            width: width.min(u16::MAX as u32) as u16,
            height: height.min(u16::MAX as u32) as u16,
        })
        .collect();
    XShapeCombineRectangles(
        display,
        host,
        SHAPE_BOUNDING,
        0,
        0,
        rectangles.as_mut_ptr(),
        rectangles.len() as std::os::raw::c_int,
        SHAPE_SET,
        YX_BANDED,
    );
}

/// Splits a rounded rectangle into horizontal bands `(x, y, width, height)`,
/// merging consecutive rows that share the same horizontal extent. Radii are
/// top-left, top-right, bottom-right, bottom-left in physical pixels.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn rounded_rect_bands(width: u32, height: u32, radii: [f64; 4]) -> Vec<(u32, u32, u32, u32)> {
    let limit = f64::from(width.min(height)) / 2.0;
    let [top_left, top_right, bottom_right, bottom_left] = radii.map(|radius| {
        if radius.is_finite() {
            radius.clamp(0.0, limit)
        } else {
            0.0
        }
    });
    // Horizontal inset of a quarter circle at a row whose pixel center lies
    // `distance` pixels from the straight edge.
    let inset = |radius: f64, distance: f64| -> u32 {
        if distance >= radius {
            return 0;
        }
        let dy = radius - distance;
        (radius - (radius * radius - dy * dy).sqrt()).round() as u32
    };

    let mut bands: Vec<(u32, u32, u32, u32)> = Vec::new();
    for row in 0..height {
        let from_top = f64::from(row) + 0.5;
        let from_bottom = f64::from(height - row) - 0.5;
        let left = inset(top_left, from_top).max(inset(bottom_left, from_bottom));
        let right = inset(top_right, from_top).max(inset(bottom_right, from_bottom));
        if left + right >= width {
            continue;
        }
        let span = (left, width - left - right);
        match bands.last_mut() {
            Some(band) if (band.0, band.2) == span && band.1 + band.3 == row => band.3 += 1,
            _ => bands.push((span.0, row, span.1, 1)),
        }
    }
    bands
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

/// Seek-bar thumbnails come from a hidden libmpv instance per window, so
/// hovering never pauses or seeks the main player. It decodes in software and
/// scales frames down before they are encoded, which keeps each capture small.
const THUMBNAIL_INSTANCE_SUFFIX: &str = "-thumbnails";
const THUMBNAIL_OPTIONS: [(&str, &str); 14] = [
    ("vo", "null"),
    ("ao", "null"),
    ("aid", "no"),
    ("sid", "no"),
    ("pause", "yes"),
    ("idle", "yes"),
    ("keep-open", "always"),
    ("hwdec", "no"),
    // One decoding thread and a small demuxer buffer keep the decoder light;
    // it only ever decodes the keyframe it seeks to.
    ("vd-lavc-threads", "1"),
    ("cache", "no"),
    ("demuxer-max-bytes", "4MiB"),
    ("vf", "scale=w=320:h=-2"),
    // The format is picked from the file extension; JPEG encodes far faster than PNG.
    ("screenshot-jpeg-quality", "75"),
    ("config", "no"),
];
const THUMBNAIL_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

fn thumbnail_label(window_label: &str) -> String {
    format!("{window_label}{THUMBNAIL_INSTANCE_SUFFIX}")
}

/// Runs `action`, then waits until the thumbnail instance reports
/// `playback-restart`, which mpv emits once a load or seek shows its frame.
fn run_and_wait_for_frame<R: tauri::Runtime>(
    app: &AppHandle<R>,
    label: &str,
    is_current: &dyn Fn() -> bool,
    action: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    let listener = app.listen_any(format!("mpv-event-{label}"), move |event| {
        let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) else {
            return;
        };
        match payload.get("event").and_then(serde_json::Value::as_str) {
            Some("playback-restart") => {
                let _ = sender.send(Ok(()));
            }
            Some("end-file")
                if payload.get("reason").and_then(serde_json::Value::as_str) == Some("error") =>
            {
                let _ = sender.send(Err("thumbnail source could not be opened".to_string()));
            }
            _ => {}
        }
    });

    let result = (|| {
        action()?;
        let deadline = std::time::Instant::now() + THUMBNAIL_WAIT_TIMEOUT;
        loop {
            if !is_current() {
                return Err("thumbnail capture cancelled".to_string());
            }
            match receiver.recv_timeout(std::time::Duration::from_millis(20)) {
                Ok(result) => return result,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout)
                    if std::time::Instant::now() < deadline => {}
                Err(_) => return Err("thumbnail frame did not arrive in time".to_string()),
            }
        }
    })();
    app.unlisten(listener);
    result
}

#[tauri::command]
pub async fn capture_thumbnail<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, ThumbnailCaptureState>,
    position: f64,
    window_label: String,
) -> Result<tauri::ipc::Response, String> {
    if !position.is_finite() || position < 0.0 {
        return Err("thumbnail position must be a non-negative number".into());
    }

    let generation = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let generations = Arc::clone(&state.generation);
    let capture_lock = Arc::clone(&state.capture_lock);
    let sources = Arc::clone(&state.sources);

    tauri::async_runtime::spawn_blocking(move || {
        let _capture_guard = capture_lock
            .lock()
            .map_err(|_| "thumbnail capture lock was poisoned".to_string())?;
        let is_current = || generations.load(Ordering::SeqCst) == generation;
        if !is_current() {
            return Err("thumbnail capture cancelled".to_string());
        }

        let mpv = app.mpv();
        let media_path = mpv
            .get_property("path".into(), "string".into(), &window_label)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .ok_or_else(|| "no media is loaded".to_string())?;
        let label = thumbnail_label(&window_label);

        let loaded_path = sources
            .lock()
            .map_err(|_| "thumbnail sources lock was poisoned".to_string())?
            .get(&window_label)
            .cloned();
        if loaded_path.as_deref() != Some(media_path.as_str()) {
            if loaded_path.is_none() {
                let config = tauri_plugin_libmpv::MpvConfig {
                    initial_options: THUMBNAIL_OPTIONS
                        .iter()
                        .map(|(key, value)| (key.to_string(), serde_json::Value::from(*value)))
                        .collect(),
                    observed_properties: Default::default(),
                };
                mpv.init(config, &label)
                    .map_err(|error| format!("could not start thumbnail decoder: {error}"))?;
            }
            // Forget the source first, so a failed load is retried next time.
            if let Ok(mut sources) = sources.lock() {
                sources.remove(&window_label);
            }
            run_and_wait_for_frame(&app, &label, &is_current, || {
                mpv.command(
                    "loadfile",
                    &vec![serde_json::Value::String(media_path.clone())],
                    &label,
                )
                .map_err(|error| format!("could not open thumbnail source: {error}"))
            })?;
            if let Ok(mut sources) = sources.lock() {
                sources.insert(window_label.clone(), media_path);
            }
        }

        // The nearest keyframe is close enough for a preview and needs no
        // decoding of the frames in between.
        run_and_wait_for_frame(&app, &label, &is_current, || {
            mpv.command(
                "seek",
                &vec![
                    serde_json::Value::from(position),
                    "absolute+keyframes".into(),
                ],
                &label,
            )
            .map_err(|error| format!("could not seek for thumbnail: {error}"))
        })?;

        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!(
            "atiga-cine-thumbnail-{}-{stamp}.jpg",
            std::process::id()
        ));
        let captured = mpv
            .command(
                "screenshot-to-file",
                &vec![
                    serde_json::Value::String(path.to_string_lossy().into_owned()),
                    "video".into(),
                ],
                &label,
            )
            .map_err(|error| format!("could not capture thumbnail: {error}"))
            .and_then(|()| {
                std::fs::read(&path).map_err(|error| format!("could not read thumbnail: {error}"))
            });
        let _ = std::fs::remove_file(&path);

        let bytes = captured?;
        if bytes.is_empty() {
            return Err("libmpv did not produce a thumbnail".into());
        }
        if !is_current() {
            return Err("thumbnail capture cancelled".to_string());
        }
        Ok(bytes)
    })
    .await
    .map_err(|error| format!("thumbnail task failed: {error}"))?
    // Raw bytes reach the webview as an ArrayBuffer instead of a JSON number array.
    .map(tauri::ipc::Response::new)
}

/// Stops the window's thumbnail decoder; the next capture starts a new one.
#[tauri::command]
pub async fn release_thumbnail_source<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, ThumbnailCaptureState>,
    window_label: String,
) -> Result<(), String> {
    state.generation.fetch_add(1, Ordering::SeqCst);
    let capture_lock = Arc::clone(&state.capture_lock);
    let sources = Arc::clone(&state.sources);

    tauri::async_runtime::spawn_blocking(move || {
        let _capture_guard = capture_lock
            .lock()
            .map_err(|_| "thumbnail capture lock was poisoned".to_string())?;
        let started = sources
            .lock()
            .map_err(|_| "thumbnail sources lock was poisoned".to_string())?
            .remove(&window_label)
            .is_some();
        if started {
            app.mpv()
                .destroy(&thumbnail_label(&window_label))
                .map_err(|error| format!("could not stop thumbnail decoder: {error}"))?;
        }
        Ok(())
    })
    .await
    .map_err(|error| format!("thumbnail release failed: {error}"))?
}

/// Native X11 windows that libmpv embeds into through `wid`, one per webview
/// window. The display connection stays open for the lifetime of the app:
/// X destroys a client's windows when its connection closes.
#[cfg(target_os = "linux")]
struct VideoHosts {
    display: *mut x11::xlib::Display,
    /// Window label -> (webview X window, host window). A host is destroyed
    /// with its parent, and windows such as the mini player get a new X
    /// window each time they open, so a host is only reused for its parent.
    windows: HashMap<String, (x11::xlib::Window, x11::xlib::Window)>,
}

// The display pointer is only used while holding the VIDEO_HOSTS lock.
#[cfg(target_os = "linux")]
unsafe impl Send for VideoHosts {}

#[cfg(target_os = "linux")]
static VIDEO_HOSTS: Mutex<Option<VideoHosts>> = Mutex::new(None);

#[cfg(target_os = "linux")]
fn x11_window<R: tauri::Runtime>(
    app: &AppHandle<R>,
    window_label: &str,
) -> Result<Option<x11::xlib::Window>, String> {
    let window = app
        .get_webview_window(window_label)
        .ok_or_else(|| format!("window '{window_label}' not found"))?;
    let handle = window
        .window_handle()
        .map_err(|error| format!("could not get native window handle: {error}"))?;
    Ok(match handle.as_raw() {
        RawWindowHandle::Xlib(handle) => Some(handle.window),
        RawWindowHandle::Xcb(handle) => Some(handle.window.get() as x11::xlib::Window),
        _ => None,
    })
}

/// Creates the native window libmpv renders into. libmpv sizes its embedded
/// video to the `wid` parent, so the TV viewport must be that parent: moving
/// libmpv's own child window leaves its render size at the parent's size and
/// crops the video. Returns `None` where X11 embedding is unavailable, in which
/// case the plugin falls back to embedding into the webview window.
#[tauri::command]
pub fn create_mpv_video_host<R: tauri::Runtime>(
    app: AppHandle<R>,
    window_label: String,
) -> Result<Option<u64>, String> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (app, window_label);
        return Ok(None);
    }

    #[cfg(target_os = "linux")]
    {
        let Some(parent) = x11_window(&app, &window_label)? else {
            return Ok(None);
        };
        let mut hosts = VIDEO_HOSTS
            .lock()
            .map_err(|_| "video host lock was poisoned".to_string())?;
        if hosts.is_none() {
            let display = unsafe { x11::xlib::XOpenDisplay(ptr::null()) };
            if display.is_null() {
                return Ok(None);
            }
            *hosts = Some(VideoHosts {
                display,
                windows: HashMap::new(),
            });
        }
        let Some(hosts) = hosts.as_mut() else {
            return Ok(None);
        };
        if let Some(&(host_parent, host)) = hosts.windows.get(&window_label) {
            if host_parent == parent {
                return Ok(Some(host));
            }
        }

        unsafe {
            let display = hosts.display;
            let black = x11::xlib::XBlackPixel(display, x11::xlib::XDefaultScreen(display));
            // Start mapped but outside the parent so libmpv sees a viewable
            // window; sync_mpv_video moves it into the TV viewport.
            let host =
                x11::xlib::XCreateSimpleWindow(display, parent, -1, -1, 1, 1, 0, black, black);
            if host == 0 {
                return Err("could not create native video host window".into());
            }
            x11::xlib::XMapWindow(display, host);
            x11::xlib::XFlush(display);
            hosts.windows.insert(window_label, (parent, host));
            Ok(Some(host))
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeVideoBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    device_scale_factor: f64,
    /// Top-left, top-right, bottom-right, bottom-left radii in CSS pixels.
    #[serde(default)]
    corner_radii: Option<[f64; 4]>,
}

#[tauri::command]
pub fn sync_mpv_video<R: tauri::Runtime>(
    app: AppHandle<R>,
    bounds: NativeVideoBounds,
    visible: bool,
    window_label: String,
) -> Result<(), String> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (app, bounds, visible, window_label);
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let (mut x, mut y, width, height) = physical_video_bounds(
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            bounds.device_scale_factor,
        )?;

        let Some(parent) = x11_window(&app, &window_label)? else {
            return Ok(());
        };

        // The host is a direct child of the webview window, so viewport
        // coordinates need no translation. Resizing it makes libmpv follow.
        if let Ok(hosts) = VIDEO_HOSTS.lock() {
            if let Some(hosts) = hosts.as_ref() {
                if let Some(&(_, host)) = hosts
                    .windows
                    .get(&window_label)
                    .filter(|(host_parent, _)| *host_parent == parent)
                {
                    unsafe {
                        if visible {
                            x11::xlib::XMoveResizeWindow(hosts.display, host, x, y, width, height);
                            let radii = bounds
                                .corner_radii
                                .unwrap_or_default()
                                .map(|radius| radius * bounds.device_scale_factor);
                            shape_video_host(hosts.display, host, width, height, radii);
                            x11::xlib::XMapRaised(hosts.display, host);
                        } else {
                            x11::xlib::XUnmapWindow(hosts.display, host);
                        }
                        x11::xlib::XFlush(hosts.display);
                    }
                    return Ok(());
                }
            }
        }

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

            // Keep libmpv's native child attached to the window created by the
            // plugin. Reparenting that child after initialization can stop
            // libmpv's X11 video and audio pipeline on some Linux/WebKitGTK
            // combinations, leaving the frontend stuck at LOADING. Translate
            // the viewport coordinates into the child's current parent and
            // resize the child directly instead.
            if mpv_parent != parent {
                let mut translated_x = 0;
                let mut translated_y = 0;
                let mut translated_child = 0;
                if x11::xlib::XTranslateCoordinates(
                    display,
                    parent,
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

            if visible {
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

/// Appends the level meter after the equalizer, so the visualizer shows what is heard.
fn with_level_meter(filter: &str, meter: &str) -> String {
    if filter.is_empty() {
        meter.to_string()
    } else {
        format!("{filter},{meter}")
    }
}

#[tauri::command]
pub async fn set_equalizer<R: tauri::Runtime>(
    app: AppHandle<R>,
    levels: tauri::State<'_, AudioLevelState>,
    equalizer: EqualizerRequest,
    window_label: String,
) -> Result<(), String> {
    let filter = build_audio_filter(equalizer.enabled, &equalizer.gains, equalizer.normalize)?;
    let meters = levels
        .meter_path(&window_label)
        .map(|path| audio_levels::meter_filters(&path))
        .unwrap_or_default();

    tauri::async_runtime::spawn_blocking(move || {
        let mpv = app.mpv();
        // The level meter only feeds the visualizer; if this FFmpeg rejects it,
        // keep the equalizer working without it.
        for meter in meters {
            let metered = serde_json::Value::String(with_level_meter(&filter, &meter));
            if mpv.set_property("af", &metered, &window_label).is_ok() {
                return Ok(());
            }
        }
        mpv.set_property("af", &serde_json::Value::String(filter), &window_label)
            .map_err(|error| format!("libmpv rejected equalizer filter: {error}"))
    })
    .await
    .map_err(|error| format!("equalizer command failed: {error}"))?
}

/// Latest per-channel RMS levels in dBFS, or an empty list while no audio is
/// flowing through the level meter.
#[tauri::command]
pub fn get_audio_levels(
    levels: tauri::State<'_, AudioLevelState>,
    window_label: String,
) -> Vec<f64> {
    levels.levels(&window_label)
}

fn session_path<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|directory| directory.join(concat!("playlist-v", env!("CARGO_PKG_VERSION"), ".json")))
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
    #[cfg(target_os = "linux")]
    use super::physical_video_bounds;
    use super::{build_audio_filter, expand_drop_paths, rounded_rect_bands, with_level_meter};

    #[test]
    fn square_corners_produce_one_band() {
        assert_eq!(rounded_rect_bands(100, 50, [0.0; 4]), vec![(0, 0, 100, 50)]);
    }

    #[test]
    fn rounded_corners_inset_only_the_corner_rows() {
        let bands = rounded_rect_bands(100, 60, [10.0, 10.0, 4.0, 4.0]);
        assert_eq!(bands.first().map(|band| band.1), Some(0));
        assert!(bands[0].0 > 0 && bands[0].0 + bands[0].2 < 100);
        assert!(bands
            .iter()
            .any(|&(x, y, width, height)| (x, width) == (0, 100) && y <= 10 && y + height >= 56));
        assert_eq!(bands.last().map(|band| band.1 + band.3), Some(60));
        let total_rows: u32 = bands.iter().map(|band| band.3).sum();
        assert_eq!(total_rows, 60);
    }

    #[test]
    fn oversized_or_invalid_radii_are_clamped() {
        let bands = rounded_rect_bands(20, 20, [f64::NAN, 500.0, -3.0, 0.0]);
        assert!(bands.iter().all(|&(x, _, width, _)| x + width <= 20));
        assert_eq!(bands.iter().map(|band| band.3).sum::<u32>(), 20);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn converts_logical_video_edges_without_leaving_fractional_gaps() {
        assert_eq!(
            physical_video_bounds(10.25, 20.5, 100.5, 50.25, 1.25)
                .expect("valid native video bounds"),
            (12, 25, 127, 64)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn converts_transformed_css_bounds_at_common_device_scales() {
        let css_bounds = (107.25, 82.5, 593.75, 438.25);

        assert_eq!(
            physical_video_bounds(css_bounds.0, css_bounds.1, css_bounds.2, css_bounds.3, 1.0),
            Ok((107, 82, 594, 439))
        );
        assert_eq!(
            physical_video_bounds(css_bounds.0, css_bounds.1, css_bounds.2, css_bounds.3, 1.25),
            Ok((134, 103, 743, 548))
        );
        assert_eq!(
            physical_video_bounds(css_bounds.0, css_bounds.1, css_bounds.2, css_bounds.3, 2.0),
            Ok((214, 165, 1188, 877))
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn rejects_invalid_layout_snapshots() {
        assert!(physical_video_bounds(0.0, 0.0, 0.0, 100.0, 1.0).is_err());
        assert!(physical_video_bounds(0.0, 0.0, 100.0, 100.0, 0.0).is_err());
        assert!(physical_video_bounds(f64::NAN, 0.0, 100.0, 100.0, 1.0).is_err());
    }

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
    fn appends_level_meter_after_equalizer() {
        assert_eq!(
            with_level_meter("lavfi=[dynaudnorm]", "lavfi=[astats]"),
            "lavfi=[dynaudnorm],lavfi=[astats]"
        );
        assert_eq!(with_level_meter("", "lavfi=[astats]"), "lavfi=[astats]");
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
