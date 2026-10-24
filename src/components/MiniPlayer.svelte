<script lang="ts">
  import { onMount } from "svelte";
  import { emit } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    command,
    destroy,
    embeddedMpvOptions,
    init,
    listenEvents,
    observeProperties,
    setEqualizer,
    setPlaybackSpeed,
    setProperty,
    syncMpvVideo,
    OBSERVED_PROPERTIES,
  } from "../lib/mpv";
  import { formatTime } from "../lib/time";

  const LABEL = "mini-player";
  const query = new URLSearchParams(window.location.search);
  const file = query.get("file") ?? "";
  const startPosition = Math.max(0, Number(query.get("position") ?? 0) || 0);
  const initialVolume = Math.max(0, Math.min(100, Number(query.get("volume") ?? 100) || 100));
  const initialMuted = query.get("muted") === "true";
  const initialSpeed = Math.max(0.5, Math.min(2, Number(query.get("speed") ?? 1) || 1));
  const equalizerEnabled = query.get("equalizer") === "1";
  const normalizeEnabled = query.get("normalize") === "1";
  const equalizerGains = (query.get("gains") ?? "0,0,0,0,0").split(",").map(Number).filter((gain) => Number.isFinite(gain));

  let position = startPosition;
  let duration = 0;
  let paused = false;
  let closing = false;
  let unobserve: (() => void) | undefined;
  let unlistenEvents: (() => void) | undefined;
  let closeUnlisten: (() => void) | undefined;
  let videoViewport: HTMLElement | undefined;
  let syncFrame: number | undefined;

  // The native video is an X11 window above the webview, so it gets its own
  // area between the bars instead of filling the whole mini window.
  function syncVideo(): void {
    if (closing || !videoViewport || syncFrame !== undefined) return;
    syncFrame = window.requestAnimationFrame(() => {
      syncFrame = undefined;
      if (closing || !videoViewport) return;
      const rect = videoViewport.getBoundingClientRect();
      void syncMpvVideo({ x: rect.left, y: rect.top, width: rect.width, height: rect.height }, true).catch(() => undefined);
    });
  }

  async function togglePlayback(): Promise<void> {
    paused = !paused;
    await setProperty("pause", paused, LABEL).catch(() => undefined);
  }

  async function closeMini(): Promise<void> {
    if (closing) return;
    closing = true;
    try {
      await emit("mini-player-closed", { position });
    } catch {
      // The main window may already be closing; cleanup must still continue.
    } finally {
      unobserve?.();
      unlistenEvents?.();
      closeUnlisten?.();
      await destroy(LABEL).catch(() => undefined);
      await getCurrentWindow().close().catch(() => undefined);
    }
  }

  onMount(() => {
    let active = true;
    const start = async () => {
      if (!file) return;

      try {
        const initialOptions = await embeddedMpvOptions();
        await init({ initialOptions, observedProperties: OBSERVED_PROPERTIES }, LABEL);
        if (!active) return;
        syncVideo();

        await setProperty("volume", initialVolume, LABEL);
        await setProperty("mute", initialMuted, LABEL);
        await setPlaybackSpeed(initialSpeed);
        await setEqualizer(equalizerEnabled, equalizerGains.length === 5 ? equalizerGains : [0, 0, 0, 0, 0], normalizeEnabled);
        unobserve = await observeProperties(OBSERVED_PROPERTIES, ({ name, data }) => {
          if (!active) return;
          if (name === "time-pos" && typeof data === "number") position = Math.max(0, data);
          if (name === "duration" && typeof data === "number") duration = Math.max(0, data);
          if (name === "pause" && typeof data === "boolean") paused = data;
        }, LABEL);
        unlistenEvents = await listenEvents((event) => {
          if (event.event === "end-file" && event.reason === "eof") paused = true;
        }, LABEL);
        const args: (string | number)[] = [file, "replace"];
        if (startPosition > 0) args.push(`start=${startPosition}`);
        await command("loadfile", args, LABEL);
      } catch {
        await closeMini();
      }
    };

    const resizeObserver = typeof ResizeObserver !== "undefined" ? new ResizeObserver(syncVideo) : undefined;
    if (videoViewport) resizeObserver?.observe(videoViewport);
    void start().catch(() => undefined);
    void getCurrentWindow().onCloseRequested((event) => {
      if (closing) return;
      event.preventDefault();
      void closeMini().catch(() => undefined);
    }).then((unlisten) => {
      if (active) closeUnlisten = unlisten;
      else unlisten();
    }).catch(() => undefined);

    return () => {
      active = false;
      resizeObserver?.disconnect();
      if (syncFrame !== undefined) window.cancelAnimationFrame(syncFrame);
      unobserve?.();
      unlistenEvents?.();
      closeUnlisten?.();
      if (!closing) void destroy(LABEL).catch(() => undefined);
    };
  });
</script>

<svelte:head><title>Atiga Cine Mini</title></svelte:head>

<main class="mini-player">
  <div class="mini-dragbar" data-tauri-drag-region>
    <span data-tauri-drag-region>ATIGA CINE · MINI</span>
    <button type="button" aria-label="Close mini player" title="Return to main window" onclick={closeMini}>×</button>
  </div>
  <div bind:this={videoViewport} class="mini-video" aria-hidden="true"></div>
  <div class="mini-controls">
    <button class="mini-play" type="button" aria-label={paused ? "Play" : "Pause"} onclick={() => void togglePlayback()}>{paused ? "▶" : "Ⅱ"}</button>
    <span>{formatTime(position)} / {formatTime(duration)}</span>
  </div>
</main>

<style>
  :global(html),
  :global(body),
  :global(#app) {
    width: 100%;
    height: 100%;
    margin: 0;
    overflow: hidden;
    background: transparent;
  }

  .mini-player {
    position: relative;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    color: #f5d28b;
    background: rgb(8 10 10 / 84%);
    border: 1px solid #936536;
    box-shadow: inset 0 0 0 1px rgb(255 214 133 / 25%), 0 0 18px rgb(0 0 0 / 55%);
    font: 700 0.62rem/1 monospace;
  }

  .mini-dragbar,
  .mini-controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.35rem 0.45rem;
    background: linear-gradient(180deg, rgb(12 13 12 / 92%), rgb(12 13 12 / 65%));
  }

  .mini-video {
    min-height: 0;
    background: #000;
  }

  .mini-controls { justify-content: flex-start; gap: 0.45rem; }

  .mini-dragbar button,
  .mini-play {
    color: inherit;
    background: #292116;
    border: 1px solid #a36f36;
    cursor: pointer;
    font: inherit;
  }

  .mini-dragbar button { width: 20px; height: 18px; }
  .mini-play { width: 24px; height: 20px; }
  button:hover,
  button:focus-visible { color: #fff2c8; border-color: #f0b658; outline: none; }
</style>
