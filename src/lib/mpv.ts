import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  command,
  destroy,
  init,
  listenEvents,
  observeProperties,
  setProperty,
  type MpvObservableProperty,
} from "tauri-plugin-libmpv-api";

export const OBSERVED_PROPERTIES = [
  ["pause", "flag"],
  ["paused-for-cache", "flag"],
  ["time-pos", "double", "none"],
  ["duration", "double", "none"],
  ["demuxer-cache-duration", "double", "none"],
  ["filename", "string", "none"],
  ["volume", "double"],
  ["mute", "flag"],
  ["video-params/w", "int64", "none"],
  ["video-params/h", "int64", "none"],
  ["file-format", "string", "none"],
  ["video-codec", "string", "none"],
  ["audio-codec", "string", "none"],
] as const satisfies readonly MpvObservableProperty[];

export const mpvOptions = {
  vo: "gpu-next",
  hwdec: "auto-safe",
  "force-window": "yes",
  "keep-open": "yes",
  keepaspect: "yes",
  "video-align-x": 0,
  "video-align-y": 0,
  idle: "yes",
  "sub-auto": "exact",
  "osd-level": 0,
  // Keep mpv's native time-stretching active so speed changes preserve pitch.
  "audio-pitch-correction": "yes",
} as const;

export { command, destroy, init, listenEvents, observeProperties, setProperty };

export async function setEqualizer(enabled: boolean, gains: readonly number[], normalize: boolean): Promise<void> {
  await invoke("set_equalizer", {
    equalizer: { enabled, gains: [...gains], normalize },
    windowLabel: getCurrentWindow().label,
  });
}

export async function setPlaybackSpeed(speed: number): Promise<void> {
  await invoke("set_playback_speed", {
    speed,
    windowLabel: getCurrentWindow().label,
  });
}

export async function syncMpvVideo(
  bounds: { x: number; y: number; width: number; height: number },
  visible: boolean,
): Promise<void> {
  const deviceScaleFactor = window.devicePixelRatio || 1;
  await invoke("sync_mpv_video", {
    bounds: { ...bounds, deviceScaleFactor },
    visible,
    windowLabel: getCurrentWindow().label,
  });
}
