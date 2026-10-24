import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { writable } from "svelte/store";

/** Latest measured loudness per side, 0 (silent) to 1 (loud). */
export type AudioLevels = { left: number; right: number };

const SILENT: AudioLevels = { left: 0, right: 0 };
// Ten reads per second keeps the meter lively without loading low-end CPUs.
const POLL_INTERVAL_MS = 100;
// Short-frame RMS of typical program audio sits between these bounds; loud,
// compressed music lands around -20 to -12 dBFS.
const FLOOR_DB = -36;
const CEILING_DB = -6;

function normalizeLevel(db: number | undefined): number {
  if (db === undefined || !Number.isFinite(db)) return 0;
  return Math.max(0, Math.min(1, (db - FLOOR_DB) / (CEILING_DB - FLOOR_DB)));
}

let enabled = false;
let subscribed = false;
let timer: number | undefined;

const store = writable<AudioLevels>(SILENT, () => {
  subscribed = true;
  schedule();
  return () => {
    subscribed = false;
    stop();
  };
});

async function poll(): Promise<void> {
  timer = undefined;
  try {
    const channels = await invoke<number[]>("get_audio_levels", {
      windowLabel: getCurrentWindow().label,
    });
    // Mono feeds both sides; multichannel audio uses front left and right.
    if (enabled) store.set({ left: normalizeLevel(channels[0]), right: normalizeLevel(channels[1] ?? channels[0]) });
  } catch {
    if (enabled) store.set(SILENT);
  }
  schedule();
}

function schedule(): void {
  if (!enabled || !subscribed || timer !== undefined) return;
  timer = window.setTimeout(() => void poll(), POLL_INTERVAL_MS);
}

function stop(): void {
  if (timer !== undefined) window.clearTimeout(timer);
  timer = undefined;
}

/** Real audio levels from libmpv's `astats` meter, read only while polling is enabled. */
export const audioLevels = { subscribe: store.subscribe };

/** Polls libmpv while native playback is running in a visible window. */
export function setAudioLevelPolling(active: boolean): void {
  if (active === enabled) return;
  enabled = active;
  if (active) {
    schedule();
  } else {
    stop();
    store.set(SILENT);
  }
}
