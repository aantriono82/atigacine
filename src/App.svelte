<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { SvelteMap } from "svelte/reactivity";
  import { get } from "svelte/store";
  import {
    command,
    destroy,
    init,
    listenEvents,
    mpvOptions,
    observeProperties,
    setEqualizer,
    setPlaybackSpeed,
    setProperty,
    syncMpvVideo,
    OBSERVED_PROPERTIES,
  } from "./lib/mpv";
  import { loadSession, saveSession } from "./lib/session";
  import { createRecentItemId, loadRecentFiles, recordRecentFiles, type RecentFile } from "./lib/recent";
  import {
    createPlaybackPersistence,
    loadPlaybackPersistence,
    playbackPositionKey,
    savePlaybackPersistence,
    type PlaybackPersistence,
  } from "./lib/playback";
  import {
    filterSupportedVideoPaths,
    isSupportedVideoPath,
    isSupportedSubtitlePath,
    SUPPORTED_SUBTITLE_EXTENSIONS,
    SUPPORTED_VIDEO_EXTENSIONS,
  } from "./lib/media";
  import PlayerRetroTV from "./components/PlayerRetroTV.svelte";
  import MiniPlayer from "./components/MiniPlayer.svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import {
    activePlaylistId,
    activePlaylistStore,
    addPaths,
    clearPlaylist,
    createPlaylist,
    currentItemId,
    deletePlaylist,
    getActiveItems,
    getSession,
    moveItem,
    playlistStore,
    removeItem,
    renamePlaylist,
    replaceSession,
    setActivePlaylist,
    setCurrentItem,
  } from "./stores/playlistStore";
  import { playerStore, updatePlayer } from "./stores/playerStore";
  import "./stores/themeStore";
  import {
    equalizerStore,
    setEqualizerEnabled,
    setEqualizerGain,
    setNormalizationEnabled,
    setEqualizerPreset,
    setEqualizerVisible,
  } from "./stores/equalizerStore";
  import type { PlaylistItem, RepeatMode } from "./types";

  const smokeVideo = new URLSearchParams(window.location.search).get("video") ?? import.meta.env.VITE_SMOKE_VIDEO;
  const miniMode = new URLSearchParams(window.location.search).get("mini") === "1";
  const SPEED_PRESETS = [1, 1.25, 1.5, 2, 0.5, 0.75] as const;

  let isDropActive = false;
  let isFullscreen = false;
  let initialized = false;
  const browserMode = !isTauriRuntime();
  const browserVideoAccept = SUPPORTED_VIDEO_EXTENSIONS.map((extension) => `.${extension}`).join(",");
  const browserSubtitleAccept = SUPPORTED_SUBTITLE_EXTENSIONS.map((extension) => `.${extension}`).join(",");
  let browserVideoInput: HTMLInputElement | undefined;
  let browserSubtitleInput: HTMLInputElement | undefined;
  let browserVideo: HTMLVideoElement | null = null;
  let browserSource: string | null = null;
  const browserSources = new SvelteMap<string, string>();
  let runtimeMessage = "Starting native player...";
  let importProgress: { current: number; total: number } | null = null;
  type DropZone = "screen" | "playlist";
  let dropZone: DropZone = "screen";
  let browserOpenShouldPlay = true;
  let unobserve: (() => void) | undefined;
  let unlistenEvents: (() => void) | undefined;
  let unlistenDrag: (() => void) | undefined;
  let seekTimer: number | undefined;
  let volumeTimer: number | undefined;
  let pendingVolume: number | null = null;
  let positionTimer: number | undefined;
  let pendingPosition: number | null = null;
  let equalizerTimer: number | undefined;
  let thumbnailTimer: number | undefined;
  let thumbnailRequest = 0;
  let thumbnailInFlightRequest: number | null = null;
  let thumbnailCancelRequested = false;
  let thumbnailOwnerActive = true;
  let nativeVideoSyncFrame: number | undefined;
  let nativeVideoSyncTimer: number | undefined;
  const nativeVideoRetryTimers: number[] = [];
  const THUMBNAIL_CACHE_LIMIT = 40;
  const thumbnailCache = new SvelteMap<number, string>();
  let thumbnailPreview: { left: number; position: number; imageUrl: string | null; loading: boolean } | null = null;
  let sessionSaveQueue = Promise.resolve();
  let playbackPersistence: PlaybackPersistence = createPlaybackPersistence();
  let lastPlaybackSaveAt = 0;
  let pendingResume: { itemId: string; position: number } | null = null;
  let closeUnlisten: (() => void) | undefined;
  let miniEventUnlisten: (() => void) | undefined;
  let miniPlayerOpen = false;
  let closing = false;
  let speedValue = 1;
  let recentFiles: RecentFile[] = loadRecentFiles();
  const recentAvailability = new SvelteMap<string, boolean>();
  let recentAvailabilityRequest = 0;
  let standaloneItem: PlaylistItem | null = null;
  const shuffleQueues = new SvelteMap<string, string[]>();
  let showCrtEffect = true;
  let windowFocused = typeof document === "undefined" || document.hasFocus();
  let documentVisible = typeof document === "undefined" || document.visibilityState !== "hidden";
  let windowMinimized = false;
  let windowFocusUnlisten: (() => void) | undefined;

  $: isPlaying = $playerStore.status === "playing";
  $: windowActive = windowFocused && documentVisible && !windowMinimized;
  $: resolution = $playerStore.width && $playerStore.height ? `${$playerStore.width}×${$playerStore.height}` : "—";
  $: mediaFormat = [$playerStore.format, $playerStore.videoCodec, $playerStore.audioCodec].filter(Boolean).join(" · ") || "—";
  $: currentPlaylistIndex = standaloneItem ? -1 : $activePlaylistStore?.items.findIndex((item) => item.id === $currentItemId) ?? -1;
  $: currentChannel = currentPlaylistIndex >= 0 ? currentPlaylistIndex + 1 : 0;
  $: currentTrackName = standaloneItem?.name ?? (currentPlaylistIndex >= 0 ? $activePlaylistStore?.items[currentPlaylistIndex]?.name ?? "" : "");
  $: repeatMode = playbackPersistence.repeatModes[$activePlaylistId] ?? "off";
  $: shuffleEnabled = playbackPersistence.shuffleModes[$activePlaylistId] ?? false;

  function isTauriRuntime(): boolean {
    return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  }

  function selectedPaths(selection: string | string[] | null): string[] {
    if (!selection) return [];
    return Array.isArray(selection) ? selection : [selection];
  }

  function speedLabel(speed: number): string {
    return `${speed}x`;
  }

  function formatThumbnailPosition(position: number): number {
    return Math.round(position * 2) / 2;
  }

  function cacheThumbnail(position: number, imageUrl: string): void {
    const previousUrl = thumbnailCache.get(position);
    if (previousUrl) URL.revokeObjectURL(previousUrl);

    thumbnailCache.delete(position);
    thumbnailCache.set(position, imageUrl);

    while (thumbnailCache.size > THUMBNAIL_CACHE_LIMIT) {
      const oldestPosition = thumbnailCache.keys().next().value;
      if (oldestPosition === undefined) break;

      const oldestUrl = thumbnailCache.get(oldestPosition);
      thumbnailCache.delete(oldestPosition);
      if (oldestUrl) URL.revokeObjectURL(oldestUrl);
    }
  }

  function cancelThumbnailCapture(): void {
    if (browserMode || !initialized || thumbnailInFlightRequest === null || thumbnailCancelRequested) return;

    thumbnailCancelRequested = true;
    void invoke("cancel_thumbnail_capture", {
      windowLabel: getCurrentWindow().label,
    }).catch(() => undefined);
  }

  function handleSeekHover(event: MouseEvent): void {
    if (browserMode || !initialized || !get(playerStore).duration) return;
    const input = event.currentTarget as HTMLInputElement;
    const bounds = input.getBoundingClientRect();
    const left = Math.max(0, Math.min(100, ((event.clientX - bounds.left) / bounds.width) * 100));
    const position = formatThumbnailPosition((left / 100) * get(playerStore).duration);
    const cached = thumbnailCache.get(position);
    thumbnailPreview = { left, position, imageUrl: cached ?? null, loading: !cached };

    if (thumbnailTimer) window.clearTimeout(thumbnailTimer);
    const request = ++thumbnailRequest;
    cancelThumbnailCapture();
    if (cached) return;

    thumbnailTimer = window.setTimeout(() => {
      thumbnailTimer = undefined;
      thumbnailInFlightRequest = request;
      thumbnailCancelRequested = false;
      void invoke<number[]>("capture_thumbnail", {
        position,
        windowLabel: getCurrentWindow().label,
      }).then((bytes) => {
        const imageUrl = URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: "image/png" }));
        if (!thumbnailOwnerActive) {
          URL.revokeObjectURL(imageUrl);
          return;
        }
        cacheThumbnail(position, imageUrl);
        if (request === thumbnailRequest) {
          thumbnailPreview = { left, position, imageUrl, loading: false };
        }
      }).catch(() => {
        if (request === thumbnailRequest) thumbnailPreview = { left, position, imageUrl: null, loading: false };
      }).finally(() => {
        if (thumbnailInFlightRequest === request) {
          thumbnailInFlightRequest = null;
          thumbnailCancelRequested = false;
        }
      });
    }, 180);
  }

  function handleSeekLeave(): void {
    if (thumbnailTimer) window.clearTimeout(thumbnailTimer);
    thumbnailTimer = undefined;
    thumbnailRequest += 1;
    cancelThumbnailCapture();
    thumbnailPreview = null;
  }

  async function resetPlaybackSpeed(): Promise<void> {
    speedValue = 1;
    if (!initialized) return;

    try {
      await setPlaybackSpeed(1);
    } catch (error) {
      updatePlayer({ message: `Speed reset failed: ${String(error)}` });
    }
  }

  async function cyclePlaybackSpeed(): Promise<void> {
    if (!initialized) {
      updatePlayer({ message: "Playback speed requires native libmpv playback." });
      return;
    }

    const currentIndex = SPEED_PRESETS.indexOf(speedValue as (typeof SPEED_PRESETS)[number]);
    const nextSpeed = SPEED_PRESETS[(currentIndex + 1) % SPEED_PRESETS.length];
    try {
      await setPlaybackSpeed(nextSpeed);
      speedValue = nextSpeed;
      updatePlayer({ message: `Playback speed ${speedLabel(nextSpeed)}` });
    } catch (error) {
      updatePlayer({ message: `Speed update failed: ${String(error)}` });
    }
  }

  function persistSession(): Promise<void> {
    const snapshot = getSession();
    sessionSaveQueue = sessionSaveQueue
      .catch(() => undefined)
      .then(async () => {
        try {
          await saveSession(snapshot);
        } catch (error) {
          runtimeMessage = `Could not save playlist: ${String(error)}`;
        }
      });
    return sessionSaveQueue;
  }

  function currentPlaybackItem(): PlaylistItem | undefined {
    if (standaloneItem) return standaloneItem;
    const itemId = get(currentItemId);
    return getActiveItems().find((item) => item.id === itemId);
  }

  function persistCurrentPlaybackPosition(force = false): void {
    if (standaloneItem) return;
    const playlistId = get(activePlaylistId);
    const item = currentPlaybackItem();
    if (!playlistId || !item) return;

    const position = get(playerStore).position;
    if (!Number.isFinite(position)) return;

    const key = playbackPositionKey(playlistId, item.id);
    playbackPersistence.positions[key] = {
      playlistId,
      itemId: item.id,
      path: item.path,
      position: Math.max(0, position),
      updatedAt: Date.now(),
    };
    playbackPersistence.lastPlaylistId = playlistId;
    playbackPersistence.lastItemId = item.id;

    const now = Date.now();
    if (force || now - lastPlaybackSaveAt >= 5_000) {
      savePlaybackPersistence(playbackPersistence);
      lastPlaybackSaveAt = now;
    }
  }

  function resetCurrentPlaybackPosition(): void {
    if (standaloneItem) return;
    const playlistId = get(activePlaylistId);
    const item = currentPlaybackItem();
    if (!playlistId || !item) return;

    const key = playbackPositionKey(playlistId, item.id);
    playbackPersistence.positions[key] = {
      playlistId,
      itemId: item.id,
      path: item.path,
      position: 0,
      updatedAt: Date.now(),
    };
    savePlaybackPersistence(playbackPersistence);
    lastPlaybackSaveAt = Date.now();
  }

  function rememberPlaybackItem(item: PlaylistItem, position = 0): void {
    const playlistId = get(activePlaylistId);
    if (!playlistId) return;

    const key = playbackPositionKey(playlistId, item.id);
    const previous = playbackPersistence.positions[key];
    playbackPersistence = {
      ...playbackPersistence,
      lastPlaylistId: playlistId,
      lastItemId: item.id,
      positions: {
        ...playbackPersistence.positions,
        [key]: {
          playlistId,
          itemId: item.id,
          path: item.path,
          position: Number.isFinite(position) ? Math.max(0, position) : previous?.position ?? 0,
          updatedAt: previous?.updatedAt ?? Date.now(),
        },
      },
    };
    savePlaybackPersistence(playbackPersistence);
    lastPlaybackSaveAt = Date.now();
  }

  function removePlaybackPosition(playlistId: string, itemId: string): void {
    const key = playbackPositionKey(playlistId, itemId);
    if (!playbackPersistence.positions[key]) return;

    const positions = Object.fromEntries(
      Object.entries(playbackPersistence.positions).filter(([entryKey]) => entryKey !== key),
    );
    playbackPersistence = { ...playbackPersistence, positions };
    savePlaybackPersistence(playbackPersistence);
  }

  function removePlaylistPlaybackState(playlistId: string): void {
    const positions = Object.fromEntries(
      Object.entries(playbackPersistence.positions).filter(([, entry]) => entry.playlistId !== playlistId),
    );
    const repeatModes = Object.fromEntries(
      Object.entries(playbackPersistence.repeatModes).filter(([entryId]) => entryId !== playlistId),
    );
    const shuffleModes = Object.fromEntries(
      Object.entries(playbackPersistence.shuffleModes).filter(([entryId]) => entryId !== playlistId),
    );

    playbackPersistence = {
      ...playbackPersistence,
      lastPlaylistId: playbackPersistence.lastPlaylistId === playlistId ? null : playbackPersistence.lastPlaylistId,
      lastItemId: playbackPersistence.lastPlaylistId === playlistId ? null : playbackPersistence.lastItemId,
      positions,
      repeatModes,
      shuffleModes,
    };
    shuffleQueues.delete(playlistId);
    savePlaybackPersistence(playbackPersistence);
  }

  function removeActivePlaylistPositions(): void {
    const playlistId = get(activePlaylistId);
    const positions = Object.fromEntries(
      Object.entries(playbackPersistence.positions).filter(([, entry]) => entry.playlistId !== playlistId),
    );
    if (Object.keys(positions).length === Object.keys(playbackPersistence.positions).length) return;

    playbackPersistence = { ...playbackPersistence, positions };
    savePlaybackPersistence(playbackPersistence);
  }

  function setRepeatMode(mode: RepeatMode): void {
    const playlistId = get(activePlaylistId);
    playbackPersistence = {
      ...playbackPersistence,
      repeatModes: { ...playbackPersistence.repeatModes, [playlistId]: mode },
    };
    savePlaybackPersistence(playbackPersistence);
  }

  function currentRepeatMode(): RepeatMode {
    return playbackPersistence.repeatModes[get(activePlaylistId)] ?? "off";
  }

  function toggleRepeat(): void {
    const next: Record<RepeatMode, RepeatMode> = { off: "one", one: "all", all: "off" };
    setRepeatMode(next[currentRepeatMode()]);
  }

  function resetShuffleQueue(): void {
    const playlistId = get(activePlaylistId);
    if (!playbackPersistence.shuffleModes[playlistId]) {
      shuffleQueues.delete(playlistId);
      return;
    }

    const current = get(currentItemId);
    const ids = getActiveItems().map((item) => item.id).filter((id) => id !== current);
    for (let index = ids.length - 1; index > 0; index -= 1) {
      const swapIndex = Math.floor(Math.random() * (index + 1));
      [ids[index], ids[swapIndex]] = [ids[swapIndex], ids[index]];
    }
    shuffleQueues.set(playlistId, ids);
  }

  function toggleShuffle(): void {
    const playlistId = get(activePlaylistId);
    playbackPersistence = {
      ...playbackPersistence,
      shuffleModes: { ...playbackPersistence.shuffleModes, [playlistId]: !playbackPersistence.shuffleModes[playlistId] },
    };
    if (playbackPersistence.shuffleModes[playlistId]) resetShuffleQueue();
    else shuffleQueues.delete(playlistId);
    savePlaybackPersistence(playbackPersistence);
  }

  function nextAutomaticItem(): PlaylistItem | undefined {
    if (standaloneItem) return undefined;
    const items = getActiveItems();
    if (items.length === 0) return undefined;

    const current = get(currentItemId);
    const currentItem = items.find((item) => item.id === current);
    const mode = currentRepeatMode();
    if (mode === "one") return currentItem ?? items[0];

    const playlistId = get(activePlaylistId);
    if (playbackPersistence.shuffleModes[playlistId]) {
      let queue = shuffleQueues.get(playlistId);
      if (!queue || queue.length === 0) {
        if (mode !== "all") return undefined;
        resetShuffleQueue();
        queue = shuffleQueues.get(playlistId);
      }

      const nextId = queue?.shift();
      if (queue) shuffleQueues.set(playlistId, queue);
      return items.find((item) => item.id === nextId);
    }

    const currentIndex = items.findIndex((item) => item.id === current);
    const nextIndex = currentIndex + 1;
    if (nextIndex < items.length) return items[nextIndex];
    return mode === "all" ? items[0] : undefined;
  }

  async function fileExists(path: string): Promise<boolean> {
    try {
      return await invoke<boolean>("path_exists", { path });
    } catch {
      // Keep playback usable with older development binaries that lack this command.
      return true;
    }
  }

  async function refreshRecentAvailability(): Promise<void> {
    const request = ++recentAvailabilityRequest;
    const entries = [...recentFiles];

    if (browserMode) {
      recentAvailability.clear();
      entries.forEach((entry) => recentAvailability.set(entry.path, true));
      return;
    }

    const availability = await Promise.all(entries.map(async (entry) => ({
      path: entry.path,
      available: await fileExists(entry.path),
    })));
    if (request !== recentAvailabilityRequest) return;

    recentAvailability.clear();
    availability.forEach((entry) => recentAvailability.set(entry.path, entry.available));
  }

  function rememberRecentPaths(paths: readonly string[]): void {
    recentFiles = recordRecentFiles(paths);
    void refreshRecentAvailability().catch((error) => {
      runtimeMessage = `Recent files check failed: ${String(error)}`;
    });
  }

  type LoadItemOptions = {
    autoPlay?: boolean;
    autoAdvance?: boolean;
    resumePosition?: number;
    standalone?: boolean;
  };

  async function loadItem(item: PlaylistItem, options: LoadItemOptions = {}): Promise<void> {
    const autoPlay = options.autoPlay ?? true;
    const autoAdvance = options.autoAdvance ?? false;
    const standalone = options.standalone ?? false;
    const playlistId = get(activePlaylistId);
    const key = playbackPositionKey(playlistId, item.id);
    const storedPosition = playbackPersistence.positions[key];
    const requestedPosition = Math.max(0, options.resumePosition ?? (autoAdvance ? 0 : storedPosition?.position ?? 0));

    clearPositionUpdate();
    standaloneItem = standalone ? item : null;
    setCurrentItem(item.id);
    if (!autoAdvance) resetShuffleQueue();
    if (!standalone) rememberPlaybackItem(item, autoAdvance ? 0 : storedPosition?.position ?? requestedPosition);
    pendingResume = requestedPosition > 0 ? { itemId: item.id, position: requestedPosition } : null;
    updatePlayer({
      currentFile: item.path,
      status: "loading",
      buffering: false,
      bufferDuration: 0,
      position: 0,
      duration: 0,
      format: null,
      videoCodec: null,
      audioCodec: null,
      width: null,
      height: null,
      message: `Loading ${item.name}`,
    });

    await resetPlaybackSpeed();

    if (!initialized && browserMode) {
      const source = browserSources.get(item.id);
      if (!source) {
        browserSource = null;
        pendingResume = null;
        updatePlayer({ status: "error", message: "Select this file again to play it in the browser." });
        return;
      }

      browserVideo?.pause();
      browserSource = source;
      updatePlayer({ status: "loading", message: `Preparing ${item.name}` });
      return;
    }

    if (!initialized) {
      updatePlayer({
        status: "error",
        buffering: false,
        message: "Native libmpv is unavailable. Check the installed libmpv package and restart Atiga Cine.",
      });
      return;
    }

    try {
      if (!(await fileExists(item.path))) {
        pendingResume = null;
        updatePlayer({ status: "error", buffering: false, message: `File tidak ditemukan: ${item.name}` });
        return;
      }

      const loadArguments: (string | boolean | number)[] = [item.path, "replace"];
      if (requestedPosition > 0) loadArguments.push(`start=${requestedPosition}`);
      await command("loadfile", loadArguments);
      scheduleNativeVideoLayout();
      if (!autoPlay) {
        await setProperty("pause", true);
        if (requestedPosition > 0) await command("seek", [requestedPosition, "absolute"]);
        pendingResume = null;
        updatePlayer({ status: "paused", position: requestedPosition, message: "Ready to resume" });
      }
      scheduleEqualizerApply();
    } catch (error) {
      pendingResume = null;
      const details = String(error);
      updatePlayer({
        status: "error",
        message: /not found|no such file|cannot open/i.test(details)
          ? `File tidak ditemukan: ${item.name}`
          : `Could not load ${item.name}: ${details}`,
      });
    }
  }

  async function addAndPlay(paths: string[], shouldPlay = true): Promise<void> {
    const validPaths = filterSupportedVideoPaths(paths);
    const added: PlaylistItem[] = [];
    const batchSize = 50;

    if (validPaths.length === 0) {
      runtimeMessage = "No supported video files were selected.";
      return;
    }

    rememberRecentPaths(validPaths);

    importProgress = { current: 0, total: validPaths.length };
    for (let index = 0; index < validPaths.length; index += batchSize) {
      added.push(...addPaths(validPaths.slice(index, index + batchSize)));
      importProgress = { current: Math.min(index + batchSize, validPaths.length), total: validPaths.length };
      await new Promise<void>((resolve) => window.setTimeout(resolve, 0));
    }
    importProgress = null;

    if (added.length === 0) {
      runtimeMessage = "Those files are already in the playlist.";
      return;
    }

    await persistSession();
    if (shouldPlay) await loadItem(added[0]);
  }

  async function addBrowserFiles(fileList: FileList | readonly File[], shouldPlay = true): Promise<void> {
    const files = Array.from(fileList).filter((file) => isSupportedVideoPath(file.name));
    const added: PlaylistItem[] = [];
    const batchSize = 50;

    if (files.length === 0) {
      runtimeMessage = "No supported video files were selected.";
      return;
    }

    rememberRecentPaths(files.map((file) => file.name));

    importProgress = { current: 0, total: files.length };
    for (let index = 0; index < files.length; index += batchSize) {
      const batch = files.slice(index, index + batchSize);
      const batchItems = addPaths(batch.map((file) => file.name));
      const itemsByName = new SvelteMap<string, PlaylistItem[]>();

      for (const item of batchItems) {
        const items = itemsByName.get(item.name) ?? [];
        items.push(item);
        itemsByName.set(item.name, items);
      }

      for (const file of batch) {
        const items = itemsByName.get(file.name);
        const item = items?.shift();
        if (!item) continue;

        browserSources.set(item.id, URL.createObjectURL(file));
        added.push(item);
      }

      importProgress = { current: Math.min(index + batchSize, files.length), total: files.length };
      await new Promise<void>((resolve) => window.setTimeout(resolve, 0));
    }
    importProgress = null;

    if (added.length === 0) {
      runtimeMessage = "Those files are already in the playlist.";
      return;
    }

    await persistSession();
    if (shouldPlay) await loadItem(added[0]);
  }

  function playRecentFile(entry: RecentFile): void {
    const item: PlaylistItem = {
      id: createRecentItemId(entry.path),
      path: entry.path,
      name: entry.name,
    };
    void loadItem(item, { standalone: true }).catch((error) => {
      updatePlayer({ status: "error", message: `Could not load ${entry.name}: ${String(error)}` });
    });
  }

  async function addRecentToPlaylist(entry: RecentFile): Promise<void> {
    const added = addPaths([entry.path]);
    if (added.length > 0) {
      await persistSession();
      runtimeMessage = `${entry.name} added to ${get(activePlaylistStore)?.name ?? "playlist"}.`;
    } else {
      runtimeMessage = `${entry.name} is already in the active playlist.`;
    }
  }

  function handleBrowserVideoInput(event: Event): void {
    const input = event.currentTarget as HTMLInputElement;
    if (input.files) {
      void addBrowserFiles(input.files, browserOpenShouldPlay).catch((error) => {
        runtimeMessage = `Browser import failed: ${String(error)}`;
      });
    }
    input.value = "";
  }

  function handleBrowserSubtitleInput(event: Event): void {
    const input = event.currentTarget as HTMLInputElement;
    const subtitle = input.files?.[0];
    input.value = "";
    if (!subtitle || !isSupportedSubtitlePath(subtitle.name)) return;

    updatePlayer({ message: "Subtitles are loaded by libmpv in the desktop app." });
  }

  async function openFiles(): Promise<void> {
    browserOpenShouldPlay = true;
    if (browserMode) {
      browserVideoInput?.click();
      return;
    }

    try {
      const selection = await open({
        multiple: true,
        directory: false,
        filters: [{ name: "Video", extensions: [...SUPPORTED_VIDEO_EXTENSIONS] }],
      });
      await addAndPlay(selectedPaths(selection));
    } catch (error) {
      runtimeMessage = `File dialog failed: ${String(error)}`;
    }
  }

  async function openPlaylistFiles(): Promise<void> {
    browserOpenShouldPlay = false;
    if (browserMode) {
      browserVideoInput?.click();
      return;
    }

    try {
      const selection = await open({
        multiple: true,
        directory: false,
        filters: [{ name: "Video", extensions: [...SUPPORTED_VIDEO_EXTENSIONS] }],
      });
      await addAndPlay(selectedPaths(selection), false);
    } catch (error) {
      runtimeMessage = `File dialog failed: ${String(error)}`;
    }
  }

  async function openSubtitle(): Promise<void> {
    if ((!browserMode && !initialized) || !get(currentItemId)) {
      runtimeMessage = browserMode ? "Select a video before adding a subtitle." : "Load a video before adding a subtitle.";
      return;
    }

    if (browserMode) {
      browserSubtitleInput?.click();
      return;
    }

    try {
      const selection = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "Subtitle", extensions: [...SUPPORTED_SUBTITLE_EXTENSIONS] }],
      });
      const subtitlePath = selectedPaths(selection).find(isSupportedSubtitlePath);
      if (!subtitlePath) return;

      await command("sub-add", [subtitlePath, "select"]);
      updatePlayer({ message: `Subtitle loaded: ${subtitlePath.split(/[\\/]/).pop() ?? subtitlePath}` });
    } catch (error) {
      updatePlayer({ message: `Could not load subtitle: ${String(error)}` });
    }
  }

  function nextIndex(direction: -1 | 1): number | undefined {
    const items = getActiveItems();
    if (items.length === 0) return undefined;

    const current = get(currentItemId);
    const currentIndex = items.findIndex((item) => item.id === current);
    const target = currentIndex < 0 ? (direction > 0 ? 0 : items.length - 1) : currentIndex + direction;
    if (target < 0 || target >= items.length) return undefined;
    return target;
  }

  function playAdjacent(direction: -1 | 1): void {
    const index = nextIndex(direction);
    if (index === undefined) return;
    const item = getActiveItems()[index];
    if (item) {
      void loadItem(item).catch((error) => {
        updatePlayer({ status: "error", message: `Could not load ${item.name}: ${String(error)}` });
      });
    }
  }

  function playAutomaticNext(): void {
    persistCurrentPlaybackPosition(true);
    resetCurrentPlaybackPosition();
    const next = nextAutomaticItem();
    if (next) {
      void loadItem(next, { autoAdvance: true }).catch((error) => {
        updatePlayer({ status: "error", message: `Could not load ${next.name}: ${String(error)}` });
      });
      return;
    }

    updatePlayer({ status: "stopped", buffering: false, message: "Playback finished" });
  }

  async function playPlayback(): Promise<void> {
    if (!get(currentItemId)) {
      const first = getActiveItems()[0];
      if (first) {
        await loadItem(first);
      }
      return;
    }

    if (!initialized && browserMode) {
      if (!browserVideo) return;

      try {
        await browserVideo.play();
      } catch (error) {
        updatePlayer({ status: "error", message: `Browser playback failed: ${String(error)}` });
      }
      return;
    }

    if (!initialized) {
      updatePlayer({ message: "Run the desktop build to start libmpv playback." });
      return;
    }

    try {
      await setProperty("pause", false);
    } catch (error) {
      updatePlayer({ status: "error", message: `Playback command failed: ${String(error)}` });
    }
  }

  async function pausePlayback(): Promise<void> {
    if (!get(currentItemId)) return;

    if (!initialized && browserMode) {
      browserVideo?.pause();
      return;
    }

    if (!initialized) {
      updatePlayer({ message: "Run the desktop build to start libmpv playback." });
      return;
    }

    try {
      await setProperty("pause", true);
    } catch (error) {
      updatePlayer({ status: "error", message: `Pause command failed: ${String(error)}` });
    }
  }

  async function togglePlay(): Promise<void> {
    if ($playerStore.status === "playing") await pausePlayback();
    else await playPlayback();
  }

  async function stopPlayback(): Promise<void> {
    persistCurrentPlaybackPosition(true);
    const item = currentPlaybackItem();
    if (item && !standaloneItem) {
      const playlistId = get(activePlaylistId);
      const key = playbackPositionKey(playlistId, item.id);
      playbackPersistence.positions[key] = {
        playlistId,
        itemId: item.id,
        path: item.path,
        position: 0,
        updatedAt: Date.now(),
      };
      savePlaybackPersistence(playbackPersistence);
    }

    if (!initialized) {
      if (browserMode && browserVideo) {
        browserVideo.pause();
        browserVideo.currentTime = 0;
      }
      clearPositionUpdate();
      updatePlayer({ status: "stopped", buffering: false, position: 0, message: "Stopped" });
      return;
    }
    try {
      await command("stop");
      clearPositionUpdate();
      updatePlayer({ status: "stopped", buffering: false, position: 0, message: "Stopped" });
    } catch (error) {
      updatePlayer({ status: "error", message: `Stop failed: ${String(error)}` });
    }
  }

  function seek(position: number): void {
    clearPositionUpdate();
    updatePlayer({ position });
    if (!initialized) {
      if (browserMode && browserVideo) browserVideo.currentTime = position;
      return;
    }

    if (seekTimer) window.clearTimeout(seekTimer);
    seekTimer = window.setTimeout(() => {
      void command("seek", [position, "absolute"]).catch((error) => {
        updatePlayer({ message: `Seek failed: ${String(error)}` });
      });
    }, 100);
  }

  async function changeVolume(value: number): Promise<void> {
    const volume = Math.min(100, Math.max(0, value));
    const wasMuted = $playerStore.muted;
    updatePlayer({ volume, muted: volume === 0 ? $playerStore.muted : false });
    if (!initialized) {
      if (browserMode && browserVideo) {
        browserVideo.volume = volume / 100;
        if (volume > 0 && wasMuted) browserVideo.muted = false;
      }
      return;
    }

    try {
      await setProperty("volume", volume);
      if (volume > 0 && wasMuted) await setProperty("mute", false);
    } catch (error) {
      updatePlayer({ message: `Volume update failed: ${String(error)}` });
    }
  }

  function scheduleVolumeChange(value: number): void {
    pendingVolume = value;
    if (volumeTimer !== undefined) window.clearTimeout(volumeTimer);

    volumeTimer = window.setTimeout(() => {
      volumeTimer = undefined;
      const nextVolume = pendingVolume;
      pendingVolume = null;
      if (nextVolume !== null) void changeVolume(nextVolume);
    }, 100);
  }

  async function toggleMute(): Promise<void> {
    const muted = !$playerStore.muted;
    updatePlayer({ muted });
    if (!initialized) {
      if (browserMode && browserVideo) browserVideo.muted = muted;
      return;
    }

    try {
      await setProperty("mute", muted);
    } catch (error) {
      updatePlayer({ message: `Mute update failed: ${String(error)}` });
    }
  }

  function scheduleEqualizerApply(): void {
    if (!initialized) {
      if (browserMode) updatePlayer({ message: "Equalizer requires native libmpv playback" });
      return;
    }

    if (equalizerTimer) window.clearTimeout(equalizerTimer);
    equalizerTimer = window.setTimeout(() => {
      equalizerTimer = undefined;
      const state = get(equalizerStore);
      void setEqualizer(state.enabled, state.gains, state.normalize)
        .then(() => {
          updatePlayer({ message: state.normalize ? "Equalizer + auto volume active" : state.enabled ? "Equalizer active" : "Audio filter bypassed" });
        })
        .catch((error) => {
          updatePlayer({ message: `Equalizer update failed: ${String(error)}` });
        });
    }, 100);
  }

  function toggleEqualizer(): void {
    setEqualizerVisible(!$equalizerStore.visible);
  }

  function toggleEqualizerEnabled(): void {
    setEqualizerEnabled(!$equalizerStore.enabled);
    scheduleEqualizerApply();
  }

  function toggleNormalization(): void {
    setNormalizationEnabled(!$equalizerStore.normalize);
    scheduleEqualizerApply();
  }

  function chooseEqualizerPreset(preset: "flat" | "bass-boost" | "vocal-boost"): void {
    setEqualizerPreset(preset);
    setEqualizerEnabled(true);
    scheduleEqualizerApply();
  }

  function changeEqualizerGain(index: number, gain: number): void {
    setEqualizerGain(index, gain);
    setEqualizerEnabled(true);
    scheduleEqualizerApply();
  }

  async function toggleFullscreen(): Promise<void> {
    if (browserMode) {
      try {
        if (document.fullscreenElement) await document.exitFullscreen();
        else await document.documentElement.requestFullscreen();
        isFullscreen = Boolean(document.fullscreenElement);
      } catch (error) {
        runtimeMessage = `Fullscreen failed: ${String(error)}`;
      }
      return;
    }

    try {
      const window = getCurrentWindow();
      const nextFullscreen = !isFullscreen;
      await window.setFullscreen(nextFullscreen);
      isFullscreen = nextFullscreen;
    } catch (error) {
      runtimeMessage = `Fullscreen failed: ${String(error)}`;
    }
  }

  async function enterMiniPlayer(): Promise<void> {
    if (browserMode || !initialized || miniPlayerOpen) {
      if (browserMode) runtimeMessage = "Mini player requires native libmpv playback.";
      return;
    }

    const item = currentPlaybackItem();
    if (!item) {
      runtimeMessage = "Load a video before opening the mini player.";
      return;
    }

    const position = Math.max(0, get(playerStore).position);
    try {
      await setProperty("pause", true);
      const miniQuery = new URLSearchParams({
        mini: "1",
        file: item.path,
        position: String(position),
        volume: String(get(playerStore).volume),
        muted: String(get(playerStore).muted),
        speed: String(speedValue),
        equalizer: $equalizerStore.enabled ? "1" : "0",
        normalize: $equalizerStore.normalize ? "1" : "0",
        gains: $equalizerStore.gains.join(","),
      }).toString();
      const miniUrl = `${window.location.href.split("?")[0]}?${miniQuery}`;

      new WebviewWindow("mini-player", {
        url: miniUrl.toString(),
        title: "Atiga Cine Mini",
        width: 360,
        height: 240,
        minWidth: 280,
        minHeight: 190,
        decorations: false,
        transparent: true,
        alwaysOnTop: true,
        resizable: true,
        focus: true,
      });
      miniPlayerOpen = true;
      await getCurrentWindow().hide();
    } catch (error) {
      miniPlayerOpen = false;
      runtimeMessage = `Mini player failed: ${String(error)}`;
      await setProperty("pause", false).catch(() => undefined);
    }
  }

  function removeFromPlaylist(id: string): void {
    const items = getActiveItems();
    const wasCurrent = get(currentItemId) === id;
    const removedIndex = items.findIndex((item) => item.id === id);
    const source = browserSources.get(id);
    if (source) {
      URL.revokeObjectURL(source);
      browserSources.delete(id);
    }
    removeItem(id);
    removePlaybackPosition(get(activePlaylistId), id);

    if (wasCurrent) {
      const remaining = getActiveItems();
      const next = remaining[Math.min(removedIndex, remaining.length - 1)];
      if (next) {
        void loadItem(next).catch((error) => {
          updatePlayer({ status: "error", message: `Could not load ${next.name}: ${String(error)}` });
        });
      }
      else {
        browserSource = null;
        updatePlayer({ currentFile: null, status: "idle", buffering: false, bufferDuration: 0, position: 0, duration: 0, format: null, videoCodec: null, audioCodec: null, message: "Playlist empty" });
      }
    }

    void persistSession();
  }

  function clearActivePlaylist(): void {
    void stopPlayback();
    browserVideo?.pause();
    browserSource = null;
    for (const item of getActiveItems()) {
      const source = browserSources.get(item.id);
      if (source) {
        URL.revokeObjectURL(source);
        browserSources.delete(item.id);
      }
    }
    removeActivePlaylistPositions();
    clearPlaylist();
    updatePlayer({ currentFile: null, status: "idle", buffering: false, bufferDuration: 0, position: 0, duration: 0, width: null, height: null, format: null, videoCodec: null, audioCodec: null, message: "Playlist empty" });
    void persistSession();
  }

  function reorder(id: string, direction: -1 | 1): void {
    moveItem(id, direction);
    void persistSession();
  }

  function loadActivePlaylistSelection(): void {
    standaloneItem = null;
    const item = getActiveItems()[0];
    browserVideo?.pause();
    browserSource = null;

    if (!item) {
      setCurrentItem(null);
      updatePlayer({ currentFile: null, status: "idle", message: "Playlist empty" });
      return;
    }

    if (!browserMode || browserSources.has(item.id)) {
      void loadItem(item).catch((error) => {
        updatePlayer({ status: "error", message: `Could not load ${item.name}: ${String(error)}` });
      });
      return;
    }

    updatePlayer({ currentFile: null, status: "idle", message: "Select this file again to play it in the browser." });
  }

  function selectPlaylist(id: string): void {
    if (id === get(activePlaylistId)) return;
    setActivePlaylist(id);
    loadActivePlaylistSelection();
    void persistSession();
  }

  function newPlaylist(): void {
    const name = window.prompt("Playlist name", `Playlist ${get(playlistStore).length + 1}`);
    if (name === null || !name.trim()) return;

    createPlaylist(name);
    loadActivePlaylistSelection();
    void persistSession();
  }

  function renameExistingPlaylist(id: string): void {
    const playlist = get(playlistStore).find((entry) => entry.id === id);
    if (!playlist) return;

    const name = window.prompt("Rename playlist", playlist.name);
    if (name === null || !name.trim()) return;

    renamePlaylist(id, name);
    void persistSession();
  }

  function deleteExistingPlaylist(id: string): void {
    const playlist = get(playlistStore).find((entry) => entry.id === id);
    if (!playlist || get(playlistStore).length <= 1) {
      runtimeMessage = "Keep at least one playlist.";
      return;
    }

    if (!window.confirm(`Delete playlist “${playlist.name}” and its ${playlist.items.length} track(s)?`)) return;

    const wasActive = get(activePlaylistId) === id;
    for (const item of playlist.items) {
      const source = browserSources.get(item.id);
      if (source) {
        URL.revokeObjectURL(source);
        browserSources.delete(item.id);
      }
    }
    removePlaylistPlaybackState(id);
    deletePlaylist(id);
    if (wasActive) loadActivePlaylistSelection();
    void persistSession();
  }

  function toggleCrtEffect(): void {
    showCrtEffect = !showCrtEffect;
  }

  function resolveDropZone(position: { x: number; y: number }): DropZone {
    const drawer = document.querySelector<HTMLElement>(".playlist-drawer");
    if (!drawer) return "screen";

    const deviceScale = window.devicePixelRatio || 1;
    const points = [
      { x: position.x / deviceScale, y: position.y / deviceScale },
      { x: position.x, y: position.y },
    ];
    const drawerBounds = drawer.getBoundingClientRect();
    return points.some((point) => point.x >= drawerBounds.left && point.x <= drawerBounds.right && point.y >= drawerBounds.top && point.y <= drawerBounds.bottom)
      ? "playlist"
      : "screen";
  }

  async function expandDroppedPaths(paths: string[]): Promise<string[]> {
    try {
      return await invoke<string[]>("expand_drop_paths", { paths });
    } catch {
      // Keep direct-file drops compatible with development binaries from before folder support.
      return filterSupportedVideoPaths(paths);
    }
  }

  async function handleDroppedPaths(paths: string[], zone: DropZone): Promise<void> {
    try {
      const expandedPaths = await expandDroppedPaths(paths);
      await addAndPlay(expandedPaths, zone === "screen");
    } catch (error) {
      runtimeMessage = `Drop import failed: ${String(error)}`;
    }
  }

  function handleBrowserDragOver(event: DragEvent): void {
    if (!browserMode) return;
    event.preventDefault();
    dropZone = resolveDropZone({ x: event.clientX, y: event.clientY });
    isDropActive = true;
  }

  function handleBrowserDrop(event: DragEvent): void {
    if (!browserMode) return;
    event.preventDefault();
    const targetZone = resolveDropZone({ x: event.clientX, y: event.clientY });
    isDropActive = false;
    if (event.dataTransfer?.files) {
      void addBrowserFiles(event.dataTransfer.files, targetZone === "screen").catch((error) => {
        runtimeMessage = `Browser drop failed: ${String(error)}`;
      });
    }
  }

  function handleBrowserDragLeave(event: DragEvent): void {
    if (!browserMode || event.currentTarget !== event.target) return;
    isDropActive = false;
  }

  function browserVideoHasFrames(): boolean {
    return Boolean(browserVideo && browserVideo.videoWidth > 0 && browserVideo.videoHeight > 0);
  }

  function onBrowserLoadedMetadata(): void {
    if (!browserVideo) return;
    browserVideo.volume = $playerStore.volume / 100;
    browserVideo.muted = $playerStore.muted;
    if (pendingResume?.itemId === get(currentItemId)) {
      browserVideo.currentTime = Math.min(pendingResume.position, browserVideo.duration || pendingResume.position);
      updatePlayer({ position: browserVideo.currentTime });
      pendingResume = null;
    }

    if (!browserVideoHasFrames()) {
      updatePlayer({
        duration: browserVideo.duration,
        status: "error",
        message: "Browser preview hanya dapat memutar audio file ini. Jalankan aplikasi desktop untuk video.",
      });
      return;
    }

    updatePlayer({ duration: browserVideo.duration, status: "paused", message: "Browser preview ready" });
  }

  function onBrowserTimeUpdate(): void {
    if (browserVideo) {
      updatePlayer({ position: browserVideo.currentTime });
      persistCurrentPlaybackPosition();
    }
  }

  function onBrowserPlay(): void {
    if (!browserVideoHasFrames()) {
      browserVideo?.pause();
      updatePlayer({
        status: "error",
        message: "Codec video ini tidak didukung browser preview. Jalankan aplikasi desktop untuk memutar video.",
      });
      return;
    }
    updatePlayer({ status: "playing", message: "Playing in browser preview" });
  }

  function onBrowserPause(): void {
    if ($playerStore.status !== "stopped") updatePlayer({ status: "paused" });
  }

  function onBrowserEnded(): void {
    playAutomaticNext();
  }

  function onBrowserError(): void {
    updatePlayer({ status: "error", message: "Browser preview tidak dapat merender video ini. Jalankan aplikasi desktop untuk codec lengkap." });
  }

  function handleKeydown(event: KeyboardEvent): void {
    const target = event.target as HTMLElement | null;
    if (target?.matches("input, select, textarea, button")) return;

    if (event.code === "Space") {
      event.preventDefault();
      void togglePlay();
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      seek(Math.max(0, $playerStore.position - 5));
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      seek(Math.min($playerStore.duration, $playerStore.position + 5));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      void changeVolume($playerStore.volume + 5);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      void changeVolume($playerStore.volume - 5);
    } else if (event.key.toLowerCase() === "f") {
      event.preventDefault();
      void toggleFullscreen();
    }
  }

  function schedulePositionUpdate(position: number | null): void {
    pendingPosition = position ?? 0;
    if (positionTimer !== undefined) return;

    positionTimer = window.setTimeout(() => {
      positionTimer = undefined;
      if (pendingPosition !== null) updatePlayer({ position: pendingPosition });
      pendingPosition = null;
    }, 100);
  }

  function clearPositionUpdate(): void {
    if (positionTimer !== undefined) window.clearTimeout(positionTimer);
    positionTimer = undefined;
    pendingPosition = null;
  }

  function restoreTarget(savedSession: Awaited<ReturnType<typeof loadSession>>): { item: PlaylistItem; position: number } | undefined {
    if (!playbackPersistence.lastPlaylistId || !playbackPersistence.lastItemId) return undefined;

    const playlist = savedSession.playlists.find((entry) => entry.id === playbackPersistence.lastPlaylistId);
    const item = playlist?.items.find((entry) => entry.id === playbackPersistence.lastItemId);
    if (!playlist || !item) return undefined;

    const position = playbackPersistence.positions[playbackPositionKey(playlist.id, item.id)];
    if (position && position.path !== item.path) return undefined;
    return { item, position: position?.position ?? 0 };
  }

  function handleBeforeUnload(): void {
    persistCurrentPlaybackPosition(true);
  }

  const NATIVE_VIDEO_SYNC_INTERVAL_MS = 900;

  function syncNativeVideoLayout(requirePlaying = false): void {
    if (browserMode || !windowActive || (requirePlaying && !isPlaying)) return;

    const screen = document.querySelector<HTMLElement>(".screen-inner");
    if (!screen) return;

    const rect = screen.getBoundingClientRect();
    const pixelRatio = window.devicePixelRatio || 1;
    const bounds = {
      x: rect.left * pixelRatio,
      y: rect.top * pixelRatio,
      width: rect.width * pixelRatio,
      height: rect.height * pixelRatio,
    };
    void syncMpvVideo(bounds, Boolean(get(playerStore).currentFile)).catch(() => undefined);
  }

  function scheduleNativeVideoLayout(withRetry = false, requirePlaying = false): void {
    if (browserMode || nativeVideoSyncFrame !== undefined) return;
    nativeVideoSyncFrame = window.requestAnimationFrame(() => {
      nativeVideoSyncFrame = undefined;
      syncNativeVideoLayout(requirePlaying);
    });

    if (withRetry) {
      for (const delay of [50, 200, 500]) {
        let timer = 0;
        timer = window.setTimeout(() => {
          const timerIndex = nativeVideoRetryTimers.indexOf(timer);
          if (timerIndex >= 0) nativeVideoRetryTimers.splice(timerIndex, 1);
          scheduleNativeVideoLayout(false, requirePlaying);
        }, delay);
        nativeVideoRetryTimers.push(timer);
      }
    }
  }

  function startNativeVideoSyncLoop(): void {
    if (browserMode || nativeVideoSyncTimer !== undefined) return;

    nativeVideoSyncTimer = window.setInterval(() => {
      if (isPlaying && windowActive && get(playerStore).currentFile) {
        scheduleNativeVideoLayout(false, true);
      }
    }, NATIVE_VIDEO_SYNC_INTERVAL_MS);
    if (get(playerStore).currentFile) scheduleNativeVideoLayout(false, true);
  }

  function stopNativeVideoSyncLoop(): void {
    if (nativeVideoSyncTimer === undefined) return;
    window.clearInterval(nativeVideoSyncTimer);
    nativeVideoSyncTimer = undefined;
  }

  async function syncWindowMinimizedState(): Promise<void> {
    if (!isTauriRuntime()) return;

    try {
      windowMinimized = await getCurrentWindow().isMinimized();
    } catch {
      windowMinimized = false;
    }
  }

  $: if (initialized && isPlaying && windowActive) {
    startNativeVideoSyncLoop();
  } else {
    stopNativeVideoSyncLoop();
  }

  onMount(() => {
    if (miniMode) return;
    let active = true;
    const handleVisibilityChange = (): void => {
      documentVisible = document.visibilityState !== "hidden";
    };
    const handleWindowFocus = (): void => {
      windowFocused = true;
      void syncWindowMinimizedState();
    };
    const handleWindowBlur = (): void => {
      windowFocused = false;
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);
    window.addEventListener("focus", handleWindowFocus);
    window.addEventListener("blur", handleWindowBlur);

    if (isTauriRuntime()) {
      void syncWindowMinimizedState();
      void getCurrentWindow()
        .onFocusChanged(({ payload }) => {
          windowFocused = payload;
          if (payload) void syncWindowMinimizedState();
        })
        .then((unlisten) => {
          if (active) windowFocusUnlisten = unlisten;
          else unlisten();
        })
        .catch((error) => {
          if (active) runtimeMessage = `Window state listener unavailable: ${String(error)}`;
        });
    }

    const start = async () => {
      const sessionPromise = loadSession().catch(() => ({ playlists: [], activePlaylistId: null }));
      const nativeInitPromise = isTauriRuntime()
        ? init({ initialOptions: mpvOptions, observedProperties: OBSERVED_PROPERTIES })
        : Promise.resolve();
      let nativeInitError: unknown;
      try {
        await nativeInitPromise;
      } catch (error) {
        nativeInitError = error;
      }
      const savedSession = await sessionPromise;
      if (!active) {
        if (isTauriRuntime()) await destroy().catch(() => undefined);
        return;
      }
      playbackPersistence = loadPlaybackPersistence();
      replaceSession(savedSession);
      void refreshRecentAvailability().catch((error) => {
        if (active) runtimeMessage = `Recent files check failed: ${String(error)}`;
      });
      const savedTrackCount = savedSession.playlists.reduce((count, playlist) => count + playlist.items.length, 0);
      if (browserMode) setCurrentItem(null);

      if (!isTauriRuntime()) {
        runtimeMessage = savedTrackCount > 0
          ? "Browser preview ready. Choose the files again to restore playback."
          : "Browser preview ready. Choose a local video to begin.";
        return;
      }

      if (nativeInitError) {
        runtimeMessage = `libmpv initialization failed: ${String(nativeInitError)}`;
        updatePlayer({ initialized: false, status: "error", buffering: false, message: runtimeMessage });
        return;
      }

      try {
        if (!active) {
          await destroy().catch(() => undefined);
          return;
        }
        initialized = true;
        updatePlayer({ initialized: true, message: "Native libmpv ready" });
        scheduleNativeVideoLayout(true);

        scheduleEqualizerApply();

        const nextUnobserve = await observeProperties(OBSERVED_PROPERTIES, ({ name, data }) => {
          if (!active) return;

          switch (name) {
            case "filename":
              updatePlayer({ currentFile: data, message: data ? "Loaded" : "Ready" });
              scheduleNativeVideoLayout(true);
              if (data) scheduleEqualizerApply();
              break;
            case "time-pos":
              persistCurrentPlaybackPosition();
              schedulePositionUpdate(data);
              break;
            case "duration":
              updatePlayer({ duration: data ?? 0 });
              scheduleNativeVideoLayout(true);
              break;
            case "demuxer-cache-duration":
              updatePlayer({ bufferDuration: Math.max(0, data ?? 0) });
              break;
            case "pause":
              updatePlayer({ status: data ? ($playerStore.buffering ? "buffering" : "paused") : "playing" });
              break;
            case "paused-for-cache":
              updatePlayer({
                buffering: data,
                status: data ? "buffering" : $playerStore.status === "paused" ? "paused" : "playing",
                message: data ? "Buffering…" : $playerStore.status === "paused" ? "Ready to resume" : "Playing",
              });
              break;
            case "volume":
              updatePlayer({ volume: data });
              break;
            case "mute":
              updatePlayer({ muted: data });
              break;
            case "video-params/w":
              updatePlayer({ width: data });
              scheduleNativeVideoLayout(true);
              break;
            case "video-params/h":
              updatePlayer({ height: data });
              break;
            case "file-format":
              updatePlayer({ format: data });
              break;
            case "video-codec":
              updatePlayer({ videoCodec: data });
              break;
            case "audio-codec":
              updatePlayer({ audioCodec: data });
              break;
          }
        });
        if (!active) {
          nextUnobserve();
          await destroy().catch(() => undefined);
          return;
        }
        unobserve = nextUnobserve;

        const nextUnlistenEvents = await listenEvents((event) => {
          if (!active) return;
          if (event.event !== "end-file") return;

          if (event.reason === "eof") {
            playAutomaticNext();
          } else if (event.reason === "error") {
            const item = currentPlaybackItem();
            void (async () => {
              const missing = item ? !(await fileExists(item.path)) : false;
              updatePlayer({
                status: "error",
                buffering: false,
                message: missing ? `File tidak ditemukan: ${item?.name ?? ""}` : `Playback error (${event.error})`,
              });
            })();
          } else if (event.reason === "stop") {
            updatePlayer({ status: "stopped", buffering: false });
          }
        });
        if (!active) {
          nextUnlistenEvents();
          await destroy().catch(() => undefined);
          return;
        }
        unlistenEvents = nextUnlistenEvents;

        const target = restoreTarget(savedSession);
        if (target && get(activePlaylistId) !== playbackPersistence.lastPlaylistId) {
          setActivePlaylist(playbackPersistence.lastPlaylistId ?? get(activePlaylistId));
        }

        if (smokeVideo) {
          await addAndPlay([smokeVideo]);
        } else if (target) {
          await loadItem(target.item, { autoPlay: false, resumePosition: target.position });
        } else if (savedTrackCount > 0) {
          updatePlayer({ message: "Session restored" });
        }
      } catch (error) {
        runtimeMessage = `libmpv initialization failed: ${String(error)}`;
        updatePlayer({ status: "error", message: runtimeMessage });
      }
    };

    void start().catch((error) => {
      if (!active) return;
      runtimeMessage = `Startup failed: ${String(error)}`;
      updatePlayer({ status: "error", message: runtimeMessage });
    });
    window.addEventListener("keydown", handleKeydown);
    window.addEventListener("beforeunload", handleBeforeUnload);

    if (isTauriRuntime()) {
      void listen<{ position?: number }>("mini-player-closed", async ({ payload }) => {
        if (!miniPlayerOpen) return;
        miniPlayerOpen = false;
        const position = Number.isFinite(payload?.position) ? Math.max(0, payload?.position ?? 0) : get(playerStore).position;
        try {
          await setProperty("time-pos", position);
          updatePlayer({ position, status: "playing", message: "Playing" });
          await setProperty("pause", false);
          await getCurrentWindow().show();
          await getCurrentWindow().setFocus();
        } catch (error) {
          runtimeMessage = `Could not return from mini player: ${String(error)}`;
          await getCurrentWindow().show().catch(() => undefined);
        }
      }).then((unlisten) => {
        if (active) miniEventUnlisten = unlisten;
        else unlisten();
      }).catch((error) => {
        if (active) runtimeMessage = `Mini player listener unavailable: ${String(error)}`;
      });

      void getCurrentWindow()
        .onCloseRequested(async (event) => {
          if (closing) return;
          event.preventDefault();
          closing = true;
          persistCurrentPlaybackPosition(true);
          await persistSession();
          await getCurrentWindow().close();
        })
        .then((unlisten) => {
          if (active) closeUnlisten = unlisten;
          else unlisten();
        })
        .catch((error) => {
          if (active) runtimeMessage = `Close handler unavailable: ${String(error)}`;
        });
    }

    if (isTauriRuntime()) {
      void getCurrentWindow()
        .onDragDropEvent(({ payload }) => {
          if (payload.type === "enter" || payload.type === "over") {
            dropZone = resolveDropZone(payload.position);
            isDropActive = true;
          } else if (payload.type === "drop") {
            const targetZone = resolveDropZone(payload.position);
            isDropActive = false;
            void handleDroppedPaths(payload.paths, targetZone);
          } else {
            isDropActive = false;
          }
        })
        .then((unlisten) => {
          if (active) unlistenDrag = unlisten;
          else unlisten();
        })
        .catch((error) => {
          if (active) runtimeMessage = `Drag-and-drop unavailable: ${String(error)}`;
        });
    }

    return () => {
      active = false;
      thumbnailOwnerActive = false;
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      window.removeEventListener("focus", handleWindowFocus);
      window.removeEventListener("blur", handleWindowBlur);
      windowFocusUnlisten?.();
      window.removeEventListener("keydown", handleKeydown);
      window.removeEventListener("beforeunload", handleBeforeUnload);
      if (seekTimer) window.clearTimeout(seekTimer);
      if (volumeTimer !== undefined) window.clearTimeout(volumeTimer);
      volumeTimer = undefined;
      pendingVolume = null;
      if (thumbnailTimer) window.clearTimeout(thumbnailTimer);
      cancelThumbnailCapture();
      if (nativeVideoSyncFrame !== undefined) window.cancelAnimationFrame(nativeVideoSyncFrame);
      nativeVideoSyncFrame = undefined;
      stopNativeVideoSyncLoop();
      for (const timer of nativeVideoRetryTimers) window.clearTimeout(timer);
      nativeVideoRetryTimers.length = 0;
      for (const imageUrl of thumbnailCache.values()) URL.revokeObjectURL(imageUrl);
      thumbnailCache.clear();
      clearPositionUpdate();
      if (equalizerTimer) window.clearTimeout(equalizerTimer);
      unobserve?.();
      unlistenEvents?.();
      unlistenDrag?.();
      closeUnlisten?.();
      miniEventUnlisten?.();
      if (!closing) persistCurrentPlaybackPosition(true);
      if (initialized) void destroy().catch(() => undefined);
      for (const source of browserSources.values()) URL.revokeObjectURL(source);
      browserSources.clear();
    };
  });
</script>

<svelte:head>
  <title>{$playerStore.currentFile ? `${$playerStore.currentFile.split(/[\\/]/).pop()} — Atiga Cine` : "Atiga Cine"}</title>
</svelte:head>

{#if miniMode}
  <MiniPlayer />
{:else}
<input
  bind:this={browserVideoInput}
  class="browser-file-input"
  type="file"
  accept={browserVideoAccept}
  multiple
  onchange={handleBrowserVideoInput}
  aria-label="Choose video files"
/> 
<input
  bind:this={browserSubtitleInput}
  class="browser-file-input"
  type="file"
  accept={browserSubtitleAccept}
  onchange={handleBrowserSubtitleInput}
  aria-label="Choose subtitle file"
/> 

<main class:browser-preview={browserMode} class="app-shell" ondragover={handleBrowserDragOver} ondrop={handleBrowserDrop} ondragleave={handleBrowserDragLeave}>
  <TitleBar />
  {#if browserMode}
    <div class="app-menu-strip">
      <span class="runtime-led" class:ready={initialized}></span>
      <span>WEBVIEW PREVIEW</span>
      {#if importProgress}
        <span class="import-progress" aria-label={`Importing ${importProgress.current} of ${importProgress.total}`}>
          <span style={`width: ${(importProgress.current / importProgress.total) * 100}%`}></span>
        </span>
        <span class="strip-message">IMPORTING {importProgress.current}/{importProgress.total}</span>
      {:else}
        <span class="strip-message">{runtimeMessage}</span>
      {/if}
    </div>
  {/if}

  <div class="workspace retro-workspace">
    <PlayerRetroTV
      currentFile={$playerStore.currentFile}
      status={$playerStore.status}
      message={$playerStore.message}
      {browserMode}
      {browserSource}
      bind:videoElement={browserVideo}
      browserVolume={$playerStore.volume / 100}
      browserMuted={$playerStore.muted}
      onBrowserLoadedMetadata={onBrowserLoadedMetadata}
      onBrowserTimeUpdate={onBrowserTimeUpdate}
      onBrowserPlay={onBrowserPlay}
      onBrowserPause={onBrowserPause}
      onBrowserEnded={onBrowserEnded}
      onBrowserError={onBrowserError}
      {isDropActive}
      {dropZone}
      onOpen={openFiles}
      onPlaylistOpen={openPlaylistFiles}
      onOpenSubtitle={openSubtitle}
      onPlay={playPlayback}
      onPause={pausePlayback}
      onStop={stopPlayback}
      onPrevious={() => playAdjacent(-1)}
      onNext={() => playAdjacent(1)}
      onSeek={seek}
      onSeekHover={handleSeekHover}
      onSeekLeave={handleSeekLeave}
      {thumbnailPreview}
      speed={speedValue}
      onSpeedCycle={cyclePlaybackSpeed}
      onVolume={scheduleVolumeChange}
      windowActive={windowActive}
      onMute={toggleMute}
      {isPlaying}
      position={$playerStore.position}
      duration={$playerStore.duration}
      volume={$playerStore.volume}
      muted={$playerStore.muted}
      {currentChannel}
      channelCount={$activePlaylistStore?.items.length ?? 0}
      {currentTrackName}
      playlists={$playlistStore}
      activePlaylistId={$activePlaylistId}
      playlistItems={$activePlaylistStore?.items ?? []}
      playlistCurrentId={$currentItemId}
      onPlaylistChange={selectPlaylist}
      onPlaylistCreate={newPlaylist}
      onPlaylistRename={renameExistingPlaylist}
      onPlaylistDelete={deleteExistingPlaylist}
      onPlaylistSelect={loadItem}
      onPlaylistRemove={removeFromPlaylist}
      onPlaylistMove={reorder}
      onPlaylistClear={clearActivePlaylist}
      {repeatMode}
      {shuffleEnabled}
      onRepeatToggle={toggleRepeat}
      onShuffleToggle={toggleShuffle}
      {recentFiles}
      {recentAvailability}
      onRecentSelect={playRecentFile}
      onRecentAdd={addRecentToPlaylist}
      equalizerVisible={$equalizerStore.visible}
      equalizerEnabled={$equalizerStore.enabled}
      equalizerNormalize={$equalizerStore.normalize}
      equalizerPreset={$equalizerStore.preset}
      equalizerGains={$equalizerStore.gains}
      onToggleEqualizer={toggleEqualizer}
      onToggleEqualizerEnabled={toggleEqualizerEnabled}
      onToggleEqualizerNormalize={toggleNormalization}
      onEqualizerPreset={chooseEqualizerPreset}
      onEqualizerGain={changeEqualizerGain}
      {isFullscreen}
      {resolution}
      {mediaFormat}
      showCrtEffect={showCrtEffect}
      onToggleCrtEffect={toggleCrtEffect}
      onToggleMiniPlayer={enterMiniPlayer}
      onToggleFullscreen={() => void toggleFullscreen()}
      onNativeVideoLayout={() => scheduleNativeVideoLayout()}
    />
  </div>

  {#if browserMode}
    <footer class="status-bar">
      <span class="status-message" title={$playerStore.message}>{$playerStore.message}</span>
      <span>{$playerStore.currentFile ? $playerStore.currentFile.split(/[\\/]/).pop() : "No file"}</span>
      <span>{resolution}</span>
      <span title={mediaFormat}>{mediaFormat}</span>
    </footer>
  {/if}
</main>
{/if}
