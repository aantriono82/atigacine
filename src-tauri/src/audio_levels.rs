//! Real-time audio levels for the visualizer.
//!
//! An `astats` filter at the end of mpv's audio filter chain measures per-channel
//! RMS levels, and `ametadata` prints them to a FIFO that a reader thread drains.
//! Levels are deliberately not read through mpv's `af-metadata` property: on
//! libmpv 0.37 that query never returns and stalls the playback core.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Readings older than this mean audio stopped flowing (paused, ended, no track).
const STALE_AFTER: Duration = Duration::from_millis(300);

const SILENT_LEVEL_DB: f64 = -120.0;

#[derive(Default)]
struct LevelMeter {
    latest: Mutex<Option<(Instant, Vec<f64>)>>,
}

impl LevelMeter {
    fn store(&self, levels: Vec<f64>) {
        if let Ok(mut latest) = self.latest.lock() {
            *latest = Some((Instant::now(), levels));
        }
    }

    fn current(&self) -> Vec<f64> {
        match self.latest.lock().ok().as_deref() {
            Some(Some((at, levels))) if at.elapsed() < STALE_AFTER => levels.clone(),
            _ => Vec::new(),
        }
    }
}

/// One FIFO and reader thread per mpv instance, created on first use.
#[derive(Default)]
pub struct AudioLevelState {
    meters: Mutex<HashMap<String, (PathBuf, Arc<LevelMeter>)>>,
}

impl AudioLevelState {
    /// FIFO path for the window's level meter, or `None` when no reader could be
    /// started. Without a reader, opening the FIFO would block mpv's audio.
    pub fn meter_path(&self, window_label: &str) -> Option<PathBuf> {
        let mut meters = self.meters.lock().ok()?;
        if let Some((path, _)) = meters.get(window_label) {
            return Some(path.clone());
        }

        let path = fifo_path(window_label)?;
        let meter = Arc::new(LevelMeter::default());
        start_reader(&path, Arc::clone(&meter))?;
        meters.insert(window_label.to_string(), (path.clone(), meter));
        Some(path)
    }

    /// Latest per-channel RMS levels in dBFS, or empty when audio is not flowing.
    pub fn levels(&self, window_label: &str) -> Vec<f64> {
        self.meters
            .lock()
            .ok()
            .and_then(|meters| meters.get(window_label).map(|(_, meter)| meter.current()))
            .unwrap_or_default()
    }
}

fn fifo_path(window_label: &str) -> Option<PathBuf> {
    let directory = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|directory| directory.is_dir())
        .unwrap_or_else(std::env::temp_dir);
    let label: String = window_label
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    let path = directory.join(format!("atiga-cine-{}-{label}.levels", std::process::id()));
    // The path is embedded in an mpv/lavfi filter string, where these characters
    // are separators.
    let text = path.to_str()?;
    if text.contains([':', ',', '[', ']', '\'', '\\', '=', ';']) {
        return None;
    }
    Some(path)
}

#[cfg(target_os = "linux")]
fn start_reader(path: &Path, meter: Arc<LevelMeter>) -> Option<()> {
    use std::ffi::CString;
    use std::io::{BufRead, BufReader};
    use std::os::unix::ffi::OsStrExt;

    if let Some(directory) = path.parent() {
        remove_stale_fifos(directory);
    }
    let _ = std::fs::remove_file(path);
    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    if unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) } != 0 {
        return None;
    }

    // Opening read-write never blocks and never sees end-of-file when mpv closes
    // and reopens its end, which happens on every audio filter change.
    let fifo = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .ok()?;

    std::thread::Builder::new()
        .name("audio-levels".into())
        .spawn(move || {
            let mut reader = BufReader::new(fifo);
            let mut frame = Vec::new();
            let mut line = Vec::new();
            loop {
                line.clear();
                match reader.read_until(b'\n', &mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                let line = String::from_utf8_lossy(&line);
                if line.starts_with("frame:") {
                    if !frame.is_empty() {
                        meter.store(channel_levels(std::mem::take(&mut frame)));
                    }
                } else if let Some(level) = parse_rms_line(&line) {
                    frame.push(level);
                }
            }
        })
        .ok()?;
    Some(())
}

/// Removes FIFOs left behind by earlier runs that have since exited.
#[cfg(target_os = "linux")]
fn remove_stale_fifos(directory: &Path) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name
            .to_str()
            .filter(|name| name.ends_with(".levels"))
            .and_then(|name| name.strip_prefix("atiga-cine-"))
            .and_then(|rest| rest.split('-').next())
            .and_then(|pid| pid.parse::<u32>().ok())
        else {
            continue;
        };
        if !Path::new("/proc").join(pid.to_string()).exists() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn start_reader(_path: &Path, _meter: Arc<LevelMeter>) -> Option<()> {
    None
}

/// Parses an `ametadata` print line such as `lavfi.astats.1.RMS_level=-20.5`
/// into `(channel, dBFS)`. Silence is printed as `-inf`.
fn parse_rms_line(line: &str) -> Option<(u32, f64)> {
    let (key, value) = line.trim().split_once('=')?;
    let channel = key
        .strip_prefix("lavfi.astats.")?
        .strip_suffix(".RMS_level")?
        .parse::<u32>()
        .ok()?;
    let level = value.trim().parse::<f64>().ok()?;
    let level = if level.is_nan() {
        SILENT_LEVEL_DB
    } else {
        level.clamp(SILENT_LEVEL_DB, 6.0)
    };
    Some((channel, level))
}

fn channel_levels(mut frame: Vec<(u32, f64)>) -> Vec<f64> {
    frame.sort_by_key(|(channel, _)| *channel);
    frame.into_iter().map(|(_, level)| level).collect()
}

/// Level meter filters for mpv's `af` list, cheapest first. FFmpeg before 4.4
/// lacks `astats`' `measure_*` options and gets the full stats set instead.
pub fn meter_filters(path: &Path) -> Vec<String> {
    let Some(path) = path.to_str() else {
        return Vec::new();
    };
    let print = format!("ametadata=mode=print:file={path}:direct=1");
    vec![
        format!(
            "lavfi=[astats=metadata=1:reset=1:measure_perchannel=RMS_level:measure_overall=none,{print}]"
        ),
        format!("lavfi=[astats=metadata=1:reset=1,{print}]"),
    ]
}

#[cfg(test)]
mod tests {
    use super::{channel_levels, meter_filters, parse_rms_line};
    use std::path::Path;

    #[test]
    fn parses_per_channel_rms_lines() {
        assert_eq!(
            parse_rms_line("lavfi.astats.2.RMS_level=-18.5\n"),
            Some((2, -18.5))
        );
        assert_eq!(
            parse_rms_line("lavfi.astats.1.RMS_level=-inf"),
            Some((1, -120.0))
        );
        assert_eq!(parse_rms_line("lavfi.astats.1.Peak_level=-3.0"), None);
        assert_eq!(parse_rms_line("lavfi.astats.Overall.RMS_level=-19.0"), None);
        assert_eq!(
            parse_rms_line("frame:12   pts:512     pts_time:0.0106"),
            None
        );
    }

    #[test]
    fn orders_levels_by_channel() {
        assert_eq!(
            channel_levels(vec![(2, -18.5), (1, -20.25)]),
            vec![-20.25, -18.5]
        );
    }

    #[test]
    fn prints_meter_stats_to_the_fifo() {
        let filters = meter_filters(Path::new("/run/user/1000/atiga-cine-1-main.levels"));
        assert_eq!(
            filters[0],
            "lavfi=[astats=metadata=1:reset=1:measure_perchannel=RMS_level:measure_overall=none,ametadata=mode=print:file=/run/user/1000/atiga-cine-1-main.levels:direct=1]"
        );
    }
}
