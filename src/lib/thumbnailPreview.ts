import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type ThumbnailPreview = { left: number; position: number; imageUrl: string | null; loading: boolean };

type ThumbnailPreviewOptions = {
  /** Native thumbnail capture is only available once libmpv is initialized. */
  isEnabled: () => boolean;
  getDuration: () => number;
  /** Identifies the loaded media, so thumbnails of another file are never reused. */
  getMediaKey: () => string | null;
  onChange: (preview: ThumbnailPreview | null) => void;
};

const THUMBNAIL_CACHE_LIMIT = 40;
const THUMBNAIL_CAPTURE_DELAY_MS = 180;
// The thumbnail decoder is a second libmpv instance; stop it once the pointer
// has been away from the seek bar for a while so it does not hold memory.
const THUMBNAIL_RELEASE_DELAY_MS = 10_000;

function formatThumbnailPosition(position: number): number {
  return Math.round(position * 2) / 2;
}

export function createThumbnailPreview(options: ThumbnailPreviewOptions) {
  const cache = new Map<string, string>();
  let timer: number | undefined;
  let releaseTimer: number | undefined;
  let request = 0;
  let inFlightRequest: number | null = null;
  let cancelRequested = false;
  let ownerActive = true;

  function cacheThumbnail(key: string, imageUrl: string): void {
    const previousUrl = cache.get(key);
    if (previousUrl) URL.revokeObjectURL(previousUrl);

    cache.delete(key);
    cache.set(key, imageUrl);

    while (cache.size > THUMBNAIL_CACHE_LIMIT) {
      const oldestKey = cache.keys().next().value;
      if (oldestKey === undefined) break;

      const oldestUrl = cache.get(oldestKey);
      cache.delete(oldestKey);
      if (oldestUrl) URL.revokeObjectURL(oldestUrl);
    }
  }

  function cancelRelease(): void {
    if (releaseTimer) window.clearTimeout(releaseTimer);
    releaseTimer = undefined;
  }

  function releaseSource(): void {
    cancelRelease();
    if (!options.isEnabled()) return;
    void invoke("release_thumbnail_source", {
      windowLabel: getCurrentWindow().label,
    }).catch(() => undefined);
  }

  function cancelCapture(): void {
    if (!options.isEnabled() || inFlightRequest === null || cancelRequested) return;

    cancelRequested = true;
    void invoke("cancel_thumbnail_capture", {
      windowLabel: getCurrentWindow().label,
    }).catch(() => undefined);
  }

  function hover(event: MouseEvent): void {
    const duration = options.getDuration();
    if (!options.isEnabled() || !duration) return;
    const input = event.currentTarget as HTMLInputElement;
    const bounds = input.getBoundingClientRect();
    const left = Math.max(0, Math.min(100, ((event.clientX - bounds.left) / bounds.width) * 100));
    const position = formatThumbnailPosition((left / 100) * duration);
    const key = `${options.getMediaKey() ?? ""}|${position}`;
    const cached = cache.get(key);
    options.onChange({ left, position, imageUrl: cached ?? null, loading: !cached });

    cancelRelease();
    if (timer) window.clearTimeout(timer);
    const currentRequest = ++request;
    cancelCapture();
    if (cached) return;

    timer = window.setTimeout(() => {
      timer = undefined;
      inFlightRequest = currentRequest;
      cancelRequested = false;
      void invoke<ArrayBuffer>("capture_thumbnail", {
        position,
        windowLabel: getCurrentWindow().label,
      }).then((bytes) => {
        const imageUrl = URL.createObjectURL(new Blob([bytes], { type: "image/jpeg" }));
        if (!ownerActive) {
          URL.revokeObjectURL(imageUrl);
          return;
        }
        cacheThumbnail(key, imageUrl);
        if (currentRequest === request) {
          options.onChange({ left, position, imageUrl, loading: false });
        }
      }).catch(() => {
        if (currentRequest === request) options.onChange({ left, position, imageUrl: null, loading: false });
      }).finally(() => {
        if (inFlightRequest === currentRequest) {
          inFlightRequest = null;
          cancelRequested = false;
        }
      });
    }, THUMBNAIL_CAPTURE_DELAY_MS);
  }

  function leave(): void {
    if (timer) window.clearTimeout(timer);
    timer = undefined;
    request += 1;
    cancelCapture();
    options.onChange(null);
    cancelRelease();
    releaseTimer = window.setTimeout(releaseSource, THUMBNAIL_RELEASE_DELAY_MS);
  }

  function dispose(): void {
    ownerActive = false;
    if (timer) window.clearTimeout(timer);
    timer = undefined;
    cancelCapture();
    releaseSource();
    for (const imageUrl of cache.values()) URL.revokeObjectURL(imageUrl);
    cache.clear();
  }

  return { hover, leave, dispose };
}
