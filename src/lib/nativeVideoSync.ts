import { syncMpvVideo } from "./mpv";

type NativeVideoSyncOptions = {
  /** False in the browser preview, where there is no native mpv window. */
  enabled: boolean;
  isWindowActive: () => boolean;
  isPlaying: () => boolean;
  hasMedia: () => boolean;
};

const NATIVE_VIDEO_SYNC_INTERVAL_MS = 900;
const NATIVE_VIDEO_RETRY_DELAYS_MS = [50, 200, 500];

/** Keeps the native mpv child window aligned with the TV screen viewport. */
export function createNativeVideoSync(options: NativeVideoSyncOptions) {
  let syncFrame: number | undefined;
  let syncTimer: number | undefined;
  const retryTimers: number[] = [];

  function sync(requirePlaying = false): void {
    if (!options.enabled || !options.isWindowActive() || (requirePlaying && !options.isPlaying())) return;

    const viewport = document.querySelector<HTMLElement>("[data-native-video-viewport]");
    if (!viewport) return;

    const rect = viewport.getBoundingClientRect();
    const bounds = {
      x: rect.left,
      y: rect.top,
      width: rect.width,
      height: rect.height,
    };
    void syncMpvVideo(bounds, options.hasMedia()).catch(() => undefined);
  }

  function schedule(withRetry = false, requirePlaying = false): void {
    if (!options.enabled || syncFrame !== undefined) return;
    syncFrame = window.requestAnimationFrame(() => {
      syncFrame = undefined;
      sync(requirePlaying);
    });

    if (withRetry) {
      for (const delay of NATIVE_VIDEO_RETRY_DELAYS_MS) {
        let timer = 0;
        timer = window.setTimeout(() => {
          const timerIndex = retryTimers.indexOf(timer);
          if (timerIndex >= 0) retryTimers.splice(timerIndex, 1);
          schedule(false, requirePlaying);
        }, delay);
        retryTimers.push(timer);
      }
    }
  }

  function startLoop(): void {
    if (!options.enabled || syncTimer !== undefined) return;

    syncTimer = window.setInterval(() => {
      if (options.isPlaying() && options.isWindowActive() && options.hasMedia()) {
        schedule(false, true);
      }
    }, NATIVE_VIDEO_SYNC_INTERVAL_MS);
    if (options.hasMedia()) schedule(false, true);
  }

  function stopLoop(): void {
    if (syncTimer === undefined) return;
    window.clearInterval(syncTimer);
    syncTimer = undefined;
  }

  function dispose(): void {
    if (syncFrame !== undefined) window.cancelAnimationFrame(syncFrame);
    syncFrame = undefined;
    stopLoop();
    for (const timer of retryTimers) window.clearTimeout(timer);
    retryTimers.length = 0;
  }

  return { schedule, startLoop, stopLoop, dispose };
}
