import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type ThumbnailPreview = { left: number; position: number; imageUrl: string | null; loading: boolean };

type ThumbnailPreviewOptions = {
  /** Native thumbnail capture is only available once libmpv is initialized. */
  isEnabled: () => boolean;
  getDuration: () => number;
  onChange: (preview: ThumbnailPreview | null) => void;
};

const THUMBNAIL_CACHE_LIMIT = 40;
const THUMBNAIL_CAPTURE_DELAY_MS = 180;

function formatThumbnailPosition(position: number): number {
  return Math.round(position * 2) / 2;
}

export function createThumbnailPreview(options: ThumbnailPreviewOptions) {
  const cache = new Map<number, string>();
  let timer: number | undefined;
  let request = 0;
  let inFlightRequest: number | null = null;
  let cancelRequested = false;
  let ownerActive = true;

  function cacheThumbnail(position: number, imageUrl: string): void {
    const previousUrl = cache.get(position);
    if (previousUrl) URL.revokeObjectURL(previousUrl);

    cache.delete(position);
    cache.set(position, imageUrl);

    while (cache.size > THUMBNAIL_CACHE_LIMIT) {
      const oldestPosition = cache.keys().next().value;
      if (oldestPosition === undefined) break;

      const oldestUrl = cache.get(oldestPosition);
      cache.delete(oldestPosition);
      if (oldestUrl) URL.revokeObjectURL(oldestUrl);
    }
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
    const cached = cache.get(position);
    options.onChange({ left, position, imageUrl: cached ?? null, loading: !cached });

    if (timer) window.clearTimeout(timer);
    const currentRequest = ++request;
    cancelCapture();
    if (cached) return;

    timer = window.setTimeout(() => {
      timer = undefined;
      inFlightRequest = currentRequest;
      cancelRequested = false;
      void invoke<number[]>("capture_thumbnail", {
        position,
        windowLabel: getCurrentWindow().label,
      }).then((bytes) => {
        const imageUrl = URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: "image/png" }));
        if (!ownerActive) {
          URL.revokeObjectURL(imageUrl);
          return;
        }
        cacheThumbnail(position, imageUrl);
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
  }

  function dispose(): void {
    ownerActive = false;
    if (timer) window.clearTimeout(timer);
    timer = undefined;
    cancelCapture();
    for (const imageUrl of cache.values()) URL.revokeObjectURL(imageUrl);
    cache.clear();
  }

  return { hover, leave, dispose };
}
