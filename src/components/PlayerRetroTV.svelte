<script lang="ts">
  import { cubicOut } from "svelte/easing";
  import { onMount, tick } from "svelte";
  import { fly } from "svelte/transition";
  import DialScale from "./DialScale.svelte";
  import Equalizer from "./Equalizer.svelte";
  import Playlist from "./Playlist.svelte";
  import PlaybackIcon from "./PlaybackIcon.svelte";
  import type { EqualizerPreset } from "../stores/equalizerStore";
  import type { RecentFile } from "../lib/recent";
  import type { PlaybackStatus, Playlist as PlaylistModel, PlaylistItem, RepeatMode } from "../types";

  export let currentFile: string | null;
  export let status: PlaybackStatus;
  export let message: string;
  export let browserMode: boolean;
  export let browserSource: string | null;
  export let videoElement: HTMLVideoElement | null = null;
  export let browserVolume: number;
  export let browserMuted: boolean;
  export let onBrowserLoadedMetadata: () => void;
  export let onBrowserTimeUpdate: () => void;
  export let onBrowserPlay: () => void;
  export let onBrowserPause: () => void;
  export let onBrowserEnded: () => void;
  export let onBrowserError: () => void;
  export let isDropActive: boolean;
  export let dropZone: "screen" | "playlist";
  export let onOpen: () => void;
  export let onPlaylistOpen: () => void;
  export let onOpenSubtitle: () => void;
  export let onPlay: () => void;
  export let onPause: () => void;
  export let onStop: () => void;
  export let onPrevious: () => void;
  export let onNext: () => void;
  export let onSeek: (position: number) => void;
  export let onSeekHover: (event: MouseEvent) => void;
  export let onSeekLeave: () => void;
  export let thumbnailPreview: { left: number; position: number; imageUrl: string | null; loading: boolean } | null = null;
  export let speed: number;
  export let onSpeedCycle: () => void;
  export let onVolume: (volume: number) => void;
  export let onMute: () => void;
  export let isPlaying: boolean;
  export let windowActive: boolean;
  export let position: number;
  export let duration: number;
  export let volume: number;
  export let muted: boolean;
  export let currentChannel: number;
  export let channelCount: number;
  export let currentTrackName: string;
  export let playlists: PlaylistModel[];
  export let activePlaylistId: string;
  export let playlistItems: PlaylistItem[];
  export let playlistCurrentId: string | null;
  export let onPlaylistChange: (id: string) => void;
  export let onPlaylistCreate: () => void;
  export let onPlaylistRename: (id: string) => void;
  export let onPlaylistDelete: (id: string) => void;
  export let onPlaylistSelect: (item: PlaylistItem) => void;
  export let onPlaylistRemove: (id: string) => void;
  export let onPlaylistMove: (id: string, direction: -1 | 1) => void;
  export let onPlaylistClear: () => void;
  export let repeatMode: RepeatMode;
  export let shuffleEnabled: boolean;
  export let onRepeatToggle: () => void;
  export let onShuffleToggle: () => void;
  export let recentFiles: RecentFile[] = [];
  export let recentAvailability: Map<string, boolean> = new Map();
  export let onRecentSelect: (entry: RecentFile) => void;
  export let onRecentAdd: (entry: RecentFile) => void;
  export let equalizerVisible: boolean;
  export let equalizerEnabled: boolean;
  export let equalizerNormalize: boolean;
  export let equalizerPreset: EqualizerPreset;
  export let equalizerGains: number[];
  export let onToggleEqualizer: () => void;
  export let onToggleEqualizerEnabled: () => void;
  export let onToggleEqualizerNormalize: () => void;
  export let onEqualizerPreset: (preset: Exclude<EqualizerPreset, "custom">) => void;
  export let onEqualizerGain: (index: number, gain: number) => void;
  export let isFullscreen: boolean;
  export let showCrtEffect: boolean;
  export let onToggleCrtEffect: () => void;
  export let onToggleMiniPlayer: () => void;
  export let onToggleFullscreen: () => void;
  export let onNativeVideoLayout: () => void = () => undefined;

  type DialKind = "channel" | "volume";
  type DialDrag = {
    kind: DialKind;
    pointerId: number;
    startY: number;
    startValue: number;
    startedAt: number;
    lastStep: number;
    lastValue: number;
    interactionScale: number;
  };

  // The TV's CSS is authored against this fixed canvas. The outer frame is resized
  // to the rendered size so the transformed wrapper still participates in centering.
  const BASE_DESIGN_WIDTH = 1216;
  const BASE_DESIGN_HEIGHT = 860;
  const BASE_DRAWER_WIDTH = 336;
  const BASE_DRAWER_GAP = 16;
  const SCALE_MIN = 0.65;
  const SCALE_MAX = 1.9;
  const SAFE_VIEWPORT_MARGIN = 24;

  let dialDrag: DialDrag | null = null;
  let playlistOpen = false;
  let drawerLayoutOpen = false;
  let stageElement: HTMLElement | undefined;
  let screenElement: HTMLElement | undefined;
  let nativeVideoViewportElement: HTMLElement | undefined;
  let scaleFactor = 1;
  let resizeFrame: number | undefined;
  let speakerVisualizerTimer: number | undefined;
  let mounted = false;
  const speakerHoles = Array.from({ length: 40 }, (_, index) => index);
  const speakerLitCount = Math.round(speakerHoles.length * 0.25);
  let speakerLit = speakerHoles.map(() => false);

  function updateSpeakerVisualizer(): void {
    if (!isPlaying) {
      if (speakerLit.some(Boolean)) speakerLit = speakerHoles.map(() => false);
      return;
    }

    const shuffledHoles = [...speakerHoles];
    for (let index = shuffledHoles.length - 1; index > 0; index -= 1) {
      const swapIndex = Math.floor(Math.random() * (index + 1));
      const currentHole = shuffledHoles[index];
      shuffledHoles[index] = shuffledHoles[swapIndex];
      shuffledHoles[swapIndex] = currentHole;
    }

    // TODO: Replace this random placeholder with real audio levels from libmpv.
    const litHoles = new Set(shuffledHoles.slice(0, speakerLitCount));
    speakerLit = speakerHoles.map((index) => {
      return litHoles.has(index);
    });
  }

  function startSpeakerVisualizer(): void {
    if (!mounted || !isPlaying || !windowActive || speakerVisualizerTimer !== undefined) return;
    speakerVisualizerTimer = window.setInterval(updateSpeakerVisualizer, 180);
  }

  function stopSpeakerVisualizer(): void {
    if (speakerVisualizerTimer === undefined) return;
    window.clearInterval(speakerVisualizerTimer);
    speakerVisualizerTimer = undefined;
  }

  $: if (isPlaying && windowActive) {
    startSpeakerVisualizer();
  } else {
    stopSpeakerVisualizer();
  }

  onMount(() => {
    mounted = true;
    startSpeakerVisualizer();
    const scheduleLayoutUpdate = (): void => {
      if (resizeFrame !== undefined) return;
      resizeFrame = window.requestAnimationFrame(() => {
        resizeFrame = undefined;
        updateScale();
        void tick().then(onNativeVideoLayout);
      });
    };
    const resizeObserver = !browserMode && typeof ResizeObserver !== "undefined"
      ? new ResizeObserver(scheduleLayoutUpdate)
      : undefined;

    updateScale();
    if (stageElement) resizeObserver?.observe(stageElement);
    if (screenElement) resizeObserver?.observe(screenElement);
    if (nativeVideoViewportElement) resizeObserver?.observe(nativeVideoViewportElement);
    scheduleLayoutUpdate();
    window.addEventListener("resize", scheduleLayoutUpdate, { passive: true });
    return () => {
      mounted = false;
      stopSpeakerVisualizer();
      window.removeEventListener("resize", scheduleLayoutUpdate);
      resizeObserver?.disconnect();
      if (resizeFrame !== undefined) window.cancelAnimationFrame(resizeFrame);
      resizeFrame = undefined;
    };
  });

  $: if (equalizerVisible) drawerLayoutOpen = true;
  $: drawerIsOpen = drawerLayoutOpen || playlistOpen || equalizerVisible;
  $: layoutWidth = drawerIsOpen ? BASE_DESIGN_WIDTH + BASE_DRAWER_GAP + BASE_DRAWER_WIDTH : BASE_DESIGN_WIDTH;
  $: if (stageElement && layoutWidth) updateScale();

  $: safeDuration = Number.isFinite(duration) && duration > 0 ? duration : 0;
  $: safePosition = Math.min(Math.max(position, 0), safeDuration || 1);
  $: channelValue = channelCount > 0 ? Math.min(Math.max(currentChannel, 1), channelCount) : 0;
  $: channelLabelStep = channelCount > 12 ? Math.max(1, Math.ceil((channelCount - 1) / 6)) : 1;
  $: channelAngle = channelCount > 1 ? -135 + ((channelValue - 1) / (channelCount - 1)) * 270 : 0;
  $: volumeValue = Math.min(Math.max(volume, 0), 100);
  $: volumeAngle = -135 + (volumeValue / 100) * 270;

  function clamp(value: number, minimum: number, maximum: number): number {
    return Math.min(Math.max(value, minimum), maximum);
  }

  function updateScale(): void {
    if (!stageElement) return;

    const availableWidth = Math.max(stageElement.clientWidth - SAFE_VIEWPORT_MARGIN * 2, 1);
    const availableHeight = Math.max(stageElement.clientHeight - SAFE_VIEWPORT_MARGIN * 2, 1);
    const nextScale = Math.min(availableWidth / layoutWidth, availableHeight / BASE_DESIGN_HEIGHT);
    scaleFactor = clamp(nextScale, SCALE_MIN, SCALE_MAX);
  }

  function formatTime(seconds: number): string {
    if (!Number.isFinite(seconds) || seconds < 0) return "00:00";
    const total = Math.floor(seconds);
    const minutes = Math.floor(total / 60);
    const remainder = total % 60;
    return `${String(minutes).padStart(2, "0")}:${String(remainder).padStart(2, "0")}`;
  }

  function formatSpeed(value: number): string {
    return `${value}x`;
  }

  function startDialDrag(kind: DialKind, event: PointerEvent): void {
    if (event.button !== 0) return;
    event.preventDefault();
    const dial = event.currentTarget as HTMLElement;
    const renderedHeight = dial.getBoundingClientRect().height;
    const designHeight = Math.max(dial.offsetHeight, 1);
    const interactionScale = renderedHeight / designHeight || 1;
    try {
      dial.setPointerCapture(event.pointerId);
    } catch {
      // Pointer capture can reject synthetic events used by browser automation.
    }
    dialDrag = {
      kind,
      pointerId: event.pointerId,
      startY: event.clientY,
      startValue: kind === "channel" ? channelValue : volumeValue,
      startedAt: Date.now(),
      lastStep: 0,
      lastValue: volumeValue,
      interactionScale,
    };
  }

  function moveDial(event: PointerEvent): void {
    if (!dialDrag || event.pointerId !== dialDrag.pointerId) return;

    const delta = (dialDrag.startY - event.clientY) / dialDrag.interactionScale;
    // TODO: upgrade this vertical drag interaction to full mouse-angle rotation.
    if (dialDrag.kind === "channel") {
      const step = Math.trunc(delta / 28);
      if (step > dialDrag.lastStep) {
        for (let index = dialDrag.lastStep; index < step; index += 1) onNext();
      } else if (step < dialDrag.lastStep) {
        for (let index = dialDrag.lastStep; index > step; index -= 1) onPrevious();
      }
      dialDrag.lastStep = step;
      return;
    }

    const nextVolume = clamp(dialDrag.startValue + delta / 2, 0, 100);
    if (Math.round(nextVolume) !== Math.round(dialDrag.lastValue)) {
      dialDrag.lastValue = nextVolume;
      onVolume(nextVolume);
    }
  }

  function finishDialDrag(event: PointerEvent, shouldToggle = true): void {
    if (!dialDrag || event.pointerId !== dialDrag.pointerId) return;
    const activeDial = dialDrag;
    const target = event.currentTarget as HTMLElement;
    if (target.hasPointerCapture(event.pointerId)) target.releasePointerCapture(event.pointerId);
    dialDrag = null;

    const wasClick = Math.abs(event.clientY - activeDial.startY) < 8 * activeDial.interactionScale && Date.now() - activeDial.startedAt < 450;
    if (!shouldToggle || !wasClick) return;
    if (activeDial.kind === "channel") togglePlaylistDrawer();
    else toggleEqualizerDrawer();
  }

  function handleDialKey(kind: DialKind, event: KeyboardEvent): void {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (kind === "channel") togglePlaylistDrawer();
      else toggleEqualizerDrawer();
      return;
    }
    if (event.key !== "ArrowUp" && event.key !== "ArrowDown" && event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();

    const increase = event.key === "ArrowUp" || event.key === "ArrowRight";
    if (kind === "channel") {
      if (increase) onNext();
      else onPrevious();
      return;
    }

    onVolume(clamp(volumeValue + (increase ? 5 : -5), 0, 100));
  }

  function seekFromInput(event: Event): void {
    onSeek(Number((event.currentTarget as HTMLInputElement).value));
  }

  function closeDrawers(): void {
    playlistOpen = false;
    if (equalizerVisible) onToggleEqualizer();
  }

  function toggleEqualizerDrawer(): void {
    drawerLayoutOpen = true;
    if (playlistOpen) playlistOpen = false;
    onToggleEqualizer();
  }

  function togglePlaylistDrawer(): void {
    drawerLayoutOpen = true;
    playlistOpen = !playlistOpen;
    if (playlistOpen && equalizerVisible) onToggleEqualizer();
  }

  function handleDrawerOutro(): void {
    if (!playlistOpen && !equalizerVisible) drawerLayoutOpen = false;
  }

  function handleCasingPointerDown(event: PointerEvent): void {
    if (!playlistOpen && !equalizerVisible) return;
    const target = event.target instanceof Element ? event.target : null;
    if (target?.closest(".side-drawer, .channel-dial, .volume-dial")) return;
    closeDrawers();
  }
</script>

<section
  bind:this={stageElement}
  class:drop-active={isDropActive && dropZone === "screen"}
  class="retro-tv-stage"
  aria-label="Retro television player"
  onpointerdown={handleCasingPointerDown}
>
  <div class="retro-tv-scale-frame" style={`width: ${layoutWidth * scaleFactor}px; height: ${BASE_DESIGN_HEIGHT * scaleFactor}px`}>
    <div
      class="retro-tv-scale-wrapper"
      style={`width: ${layoutWidth}px; height: ${BASE_DESIGN_HEIGHT}px; transform: translate(-50%, -50%) scale(${scaleFactor})`}
    >
  <div class:drawer-open={drawerIsOpen} class="retro-tv-layout" style={`width: ${layoutWidth}px`}>
    <div class="retro-tv-casing">
      <div class="tv-body">
        <div class="screen-column">
          <div class="screen-bezel">
            <div bind:this={screenElement} class="screen-inner">
              <div bind:this={nativeVideoViewportElement} class="native-video-viewport" data-native-video-viewport aria-hidden="true"></div>

              {#if browserMode && browserSource}
                <video
                  bind:this={videoElement}
                  class="browser-video"
                  src={browserSource}
                  volume={browserVolume}
                  muted={browserMuted}
                  playsinline
                  preload="metadata"
                  onloadedmetadata={onBrowserLoadedMetadata}
                  ontimeupdate={onBrowserTimeUpdate}
                  onplay={onBrowserPlay}
                  onpause={onBrowserPause}
                  onended={onBrowserEnded}
                  onerror={onBrowserError}
                ></video>
                {#if status === "error"}
                  <div class="media-overlay browser-error-overlay">
                    <span class="media-status" role="status" aria-live="polite">{message}</span>
                  </div>
                {/if}
              {:else if !currentFile}
                <div class="screen-empty">
                  <span class="screen-empty-mark">◎</span>
                  <span class="screen-empty-label">NO SIGNAL</span>
                  <button class="screen-open-button" type="button" onclick={onOpen}>OPEN VIDEO</button>
                </div>
              {/if}

              {#if showCrtEffect}
                <div class="screen-scanlines" aria-hidden="true"></div>
                <div class="crt-vignette" aria-hidden="true"></div>
              {/if}
            </div>
          </div>

          <div class="bezel-info" aria-label="Playback metadata">
            <span class="bezel-status">{isFullscreen ? "FULLSCREEN" : status.toUpperCase()}</span>
            <span class="bezel-track" title={currentTrackName}>{currentTrackName || "NO MEDIA"}</span>
            <button
              class:active={showCrtEffect}
              class="crt-status-toggle"
              type="button"
              aria-pressed={showCrtEffect}
              title="Toggle CRT effect"
              onclick={onToggleCrtEffect}
            >CRT {showCrtEffect ? "ON" : "OFF"}</button>
          </div>

          <div class="tv-lower-strip">
            <div class="physical-controls physical-controls-left" aria-label="Playback controls left">
              <button class="physical-button" type="button" aria-label="Previous track" title="Previous track" onclick={onPrevious}>
                <PlaybackIcon name="previous" />
              </button>
              <button class:active={status === "playing"} class="physical-button" type="button" aria-label="Play" aria-pressed={status === "playing"} title="Play" disabled={status === "playing"} onclick={onPlay}>
                <PlaybackIcon name="play" />
              </button>
              <button class="physical-button" type="button" aria-label="Stop" title="Stop" onclick={onStop}>
                <PlaybackIcon name="stop" />
              </button>
            </div>

            <div class="seek-module">
              {#if thumbnailPreview}
                <div class="seek-preview" style={`left: ${thumbnailPreview.left}%`} aria-hidden="true">
                  <div class="seek-preview-frame">
                    {#if thumbnailPreview.imageUrl}
                      <img src={thumbnailPreview.imageUrl} alt="" />
                    {:else}
                      <span class:loading={thumbnailPreview.loading} class="seek-preview-placeholder">{thumbnailPreview.loading ? "LOADING" : "NO FRAME"}</span>
                    {/if}
                  </div>
                  <span>{formatTime(thumbnailPreview.position)}</span>
                </div>
              {/if}
              <div class="seek-labels">
                <span>{formatTime(position)}</span>
                <span title={currentTrackName}>{currentTrackName || "TUNER READY"}</span>
                <button class="speed-button" type="button" title="Cycle playback speed" aria-label={`Playback speed ${formatSpeed(speed)}`} onclick={onSpeedCycle} disabled={browserMode}>{formatSpeed(speed)}</button>
                <span>{formatTime(duration)}</span>
              </div>
              <input
                aria-label="Seek position"
                type="range"
                min="0"
                max={safeDuration || 1}
                step="0.1"
                value={safePosition}
                style={`--tv-seek-progress: ${safeDuration ? (safePosition / safeDuration) * 100 : 0}%`}
                oninput={seekFromInput}
                onmousemove={onSeekHover}
                onmouseleave={onSeekLeave}
              />
            </div>

            <div class="physical-controls physical-controls-right" aria-label="Playback controls right">
              <button class="physical-button" type="button" aria-label="Next track" title="Next track" onclick={onNext}>
                <PlaybackIcon name="next" />
              </button>
              <button class:active={status === "paused"} class="physical-button" type="button" aria-label="Pause" aria-pressed={status === "paused"} title="Pause" disabled={!isPlaying} onclick={onPause}>
                <PlaybackIcon name="pause" />
              </button>
              <button class:active={muted} class="physical-button" type="button" aria-label={muted ? "Unmute" : "Mute"} aria-pressed={muted} title={muted ? "Unmute" : "Mute"} onclick={onMute}>
                <PlaybackIcon name={muted ? "mute" : "volume"} />
              </button>
            </div>
          </div>
        </div>

        <aside class="tv-control-panel" aria-label="Television controls">
          <div class="dial-unit channel-unit">
            <div class="dial-heading">
              <span>CHANNEL</span>
              <output>{channelValue ? String(channelValue).padStart(2, "0") : "--"}</output>
            </div>
            <div class="dial-wrap">
              <DialScale minValue={1} maxValue={Math.max(channelCount, 1)} labelStep={channelLabelStep} activeValue={channelValue || null} showLabels={channelCount > 0} />
              <div
                class="dial channel-dial"
                role="slider"
                tabindex="0"
                aria-label="Channel selector"
                aria-valuemin="1"
                aria-valuemax={Math.max(channelCount, 1)}
                aria-valuenow={channelValue || 1}
                style={`--dial-angle: ${channelAngle}deg`}
                onpointerdown={(event) => startDialDrag("channel", event)}
                onpointermove={moveDial}
                onpointerup={finishDialDrag}
                onpointercancel={(event) => finishDialDrag(event, false)}
                onkeydown={(event) => handleDialKey("channel", event)}
              >
                <span class="dial-indicator" aria-hidden="true"></span>
                <span class="dial-center" aria-hidden="true">{channelValue || "—"}</span>
              </div>
            </div>
            <span class="dial-help">CLICK LIST · DRAG ↑ / ↓</span>
          </div>

          <div class="dial-unit volume-unit">
            <div class="dial-heading">
              <span>VOLUME</span>
              <output>{Math.round(volumeValue)}%</output>
            </div>
            <div class="dial-wrap volume-wrap">
              <DialScale minValue={0} maxValue={100} labelStep={20} />
              <div class="dial volume-dial" role="slider" tabindex="0" aria-label="Volume" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(volumeValue)} style={`--dial-angle: ${volumeAngle}deg`} onpointerdown={(event) => startDialDrag("volume", event)} onpointermove={moveDial} onpointerup={finishDialDrag} onpointercancel={(event) => finishDialDrag(event, false)} onkeydown={(event) => handleDialKey("volume", event)}>
                <span class="dial-indicator" aria-hidden="true"></span>
                <span class="dial-center" aria-hidden="true">{muted ? "×" : ""}</span>
              </div>
            </div>
            <span class="dial-help">CLICK EQ · DRAG ↑ / ↓</span>
          </div>

          <div class="speaker-grille" aria-label="Speaker grille decoration">
            <div class="speaker-holes" aria-hidden="true">
              {#each speakerHoles as index (index)}
                <span class:lit={isPlaying && speakerLit[index]} class="speaker-hole"></span>
              {/each}
            </div>
            <span class="speaker-label">ATIGA SOUND</span>
          </div>

          <div class="utility-controls" aria-label="TV utility controls">
            <div class="utility-buttons">
              <button class="physical-button utility-button" type="button" title="Open video" onclick={onOpen}>OPEN</button>
              <button class="physical-button utility-button" type="button" title="Load subtitle" onclick={onOpenSubtitle}>SUB</button>
              <button class="physical-button utility-button" type="button" title="Toggle mini player" onclick={onToggleMiniPlayer}>MINI</button>
              <button class="physical-button utility-button" type="button" title="Toggle fullscreen" onclick={onToggleFullscreen}>{isFullscreen ? "FULL" : "MAX"}</button>
            </div>

            <div class="audio-jacks" aria-hidden="true">
              <div class="audio-jack-row">
                <div class="audio-jack-port">
                  <span class="audio-jack-hole"></span>
                  <span>L</span>
                </div>
                <div class="audio-jack-port">
                  <span class="audio-jack-hole"></span>
                  <span>R</span>
                </div>
                <div class="audio-jack-port">
                  <span class="audio-jack-hole"></span>
                  <span>BASS</span>
                </div>
              </div>
            </div>
          </div>

          <div class="panel-branding" aria-label="Atiga Cine branding">
            <div class="panel-brand-emblem" aria-label="Atiga Cine monogram">AC</div>
            <div class="panel-brand-copy">
              <span class="panel-brand-name">ATIGA CINE</span>
              <span class="panel-brand-subtitle">NATIVE MEDIA PLAYER</span>
            </div>
          </div>
        </aside>
      </div>

      <div class="tv-feet" aria-hidden="true">
        <span></span>
        <span></span>
      </div>

      {#if isDropActive && dropZone === "screen"}
        <div class="drop-overlay">DROP TO ADD &amp; PLAY VIDEO</div>
      {/if}
    </div>

    {#if playlistOpen}
      <aside class:drop-target={isDropActive && dropZone === "playlist"} class="side-drawer playlist-drawer" aria-label="Playlist drawer" transition:fly={{ x: 320, duration: 240, easing: cubicOut }} onoutroend={handleDrawerOutro}>
        <Playlist
          {playlists}
          {activePlaylistId}
          items={playlistItems}
          currentId={playlistCurrentId}
          onPlaylistSelect={onPlaylistChange}
          onNewPlaylist={onPlaylistCreate}
          onRenamePlaylist={onPlaylistRename}
          onDeletePlaylist={onPlaylistDelete}
          onSelect={onPlaylistSelect}
          onRemove={onPlaylistRemove}
          onMove={onPlaylistMove}
          onOpen={onPlaylistOpen}
          onClear={onPlaylistClear}
          onClose={closeDrawers}
          {repeatMode}
          {shuffleEnabled}
          onRepeatToggle={onRepeatToggle}
          onShuffleToggle={onShuffleToggle}
          {recentFiles}
          {recentAvailability}
          {onRecentSelect}
          {onRecentAdd}
        />
        {#if isDropActive && dropZone === "playlist"}
          <div class="playlist-drop-overlay">DROP TO ADD TRACKS</div>
        {/if}
      </aside>
    {:else if equalizerVisible}
      <aside class="side-drawer equalizer-drawer" aria-label="Equalizer drawer" transition:fly={{ x: 320, duration: 240, easing: cubicOut }} onoutroend={handleDrawerOutro}>
        <Equalizer
          enabled={equalizerEnabled}
          normalize={equalizerNormalize}
          preset={equalizerPreset}
          gains={equalizerGains}
          onToggle={onToggleEqualizerEnabled}
          onToggleNormalize={onToggleEqualizerNormalize}
          onPreset={onEqualizerPreset}
          onGain={onEqualizerGain}
          onClose={closeDrawers}
          {isPlaying}
          {windowActive}
        />
      </aside>
    {/if}
  </div>
    </div>
  </div>
</section>

<style>
  .retro-tv-stage {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 0;
    min-height: 0;
    height: 100%;
    overflow: auto;
    padding: var(--tv-stage-padding);
    color: var(--tv-text);
    background: transparent;
    isolation: isolate;
  }

  .retro-tv-scale-frame {
    position: relative;
    flex: 0 0 auto;
    max-width: none;
    max-height: none;
  }

  .retro-tv-scale-wrapper {
    position: absolute;
    top: 50%;
    left: 50%;
    transform-origin: center center;
    will-change: transform;
  }

  .retro-tv-layout {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    max-width: none;
    height: max-content;
    min-width: 0;
    min-height: 0;
    margin-inline: auto;
  }

  .retro-tv-layout.drawer-open {
    grid-template-columns: minmax(0, var(--tv-casing-max-width)) var(--tv-drawer-width);
    column-gap: var(--tv-drawer-gap);
    max-width: calc(var(--tv-casing-max-width) + var(--tv-drawer-gap) + var(--tv-drawer-width));
  }

  .retro-tv-casing {
    position: relative;
    display: grid;
    grid-template-rows: auto var(--tv-foot-height);
    width: 100%;
    max-width: none;
    height: max-content;
    min-height: 0;
    padding: var(--tv-casing-padding);
    padding-bottom: calc(var(--tv-casing-padding) + var(--tv-foot-height));
    background:
      linear-gradient(100deg, var(--tv-wood-dark) 0%, transparent 10%, transparent 86%, var(--tv-wood-dark) 100%),
      repeating-linear-gradient(169deg, transparent 0 7px, var(--tv-wood-grain) 7px 9px, transparent 9px 18px),
      repeating-linear-gradient(11deg, var(--tv-wood-color) 0 13px, var(--tv-wood-light) 13px 15px, var(--tv-wood-color) 15px 25px);
    border: var(--tv-casing-border) solid var(--tv-wood-dark);
    border-radius: var(--tv-casing-radius);
    box-shadow: 0 var(--tv-casing-shadow-y) var(--tv-casing-shadow-blur) var(--tv-shadow), inset 0 1px 0 var(--tv-wood-highlight), inset 0 -7px 0 var(--tv-wood-deep);
  }

  .tv-body {
    display: grid;
    grid-template-columns: minmax(0, 1fr) var(--tv-control-width);
    gap: var(--tv-panel-gap);
    height: max-content;
    min-height: 0;
  }

  .screen-column {
    display: grid;
    grid-template-rows: auto auto auto;
    align-content: start;
    gap: 0;
    height: max-content;
    min-width: 0;
    min-height: max-content;
  }

  .screen-bezel {
    width: 100%;
    aspect-ratio: var(--tv-screen-aspect-ratio);
    box-sizing: border-box;
    min-width: 0;
    min-height: 0;
    padding: var(--tv-bezel-padding);
    background: linear-gradient(145deg, var(--tv-bezel-highlight), var(--tv-bezel-cream) 35%, var(--tv-bezel-shadow));
    border: var(--tv-bezel-border) solid var(--tv-bezel-shadow);
    border-radius: var(--tv-bezel-radius) var(--tv-bezel-radius) 0 0;
    box-shadow: inset 0 2px 0 var(--tv-bezel-highlight), inset 0 -4px 0 var(--tv-bezel-deep), 0 2px 3px var(--tv-shadow);
  }

  .screen-inner {
    position: relative;
    display: grid;
    min-height: 0;
    height: 100%;
    overflow: hidden;
    background: var(--tv-screen-color);
    border: var(--tv-screen-border) solid var(--tv-screen-black);
    border-radius: var(--tv-screen-radius);
    box-shadow: inset 0 0 0 2px var(--tv-screen-glass), inset 0 0 30px var(--tv-screen-shadow);
    isolation: isolate;
  }

  .browser-video {
    display: block;
    width: 100%;
    height: 100%;
    min-height: 0;
    background: var(--tv-screen-color);
    object-fit: contain;
  }

  .native-video-viewport {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  .browser-error-overlay {
    align-items: flex-end;
    flex-direction: column;
    text-align: right;
  }

  .screen-empty {
    z-index: 1;
    display: grid;
    place-items: center;
    align-content: center;
    gap: var(--tv-screen-empty-gap);
    padding: 1rem;
    color: var(--tv-screen-text);
    text-align: center;
    text-shadow: 0 0 10px var(--tv-screen-glow);
  }

  .screen-empty-mark {
    color: var(--tv-screen-glow);
    font: var(--tv-screen-mark-size)/0.8 Georgia, serif;
  }

  .screen-empty-label {
    font: 700 var(--tv-screen-label-size)/1 monospace;
    letter-spacing: var(--tv-screen-label-spacing);
  }

  .screen-open-button {
    padding: var(--tv-open-button-padding);
    color: var(--tv-bezel-deep);
    background: var(--tv-screen-text);
    border: 1px solid var(--tv-screen-glow);
    font: 700 var(--tv-open-button-size)/1 monospace;
    letter-spacing: 0.08em;
    cursor: pointer;
  }

  .screen-open-button:hover,
  .screen-open-button:focus-visible {
    color: var(--tv-bezel-cream);
    background: var(--tv-wood-dark);
    outline: 2px solid var(--tv-screen-glow);
    outline-offset: 2px;
  }

  .screen-scanlines,
  .crt-vignette {
    position: absolute;
    z-index: 2;
    inset: 0;
    pointer-events: none;
  }

  .screen-scanlines {
    background: repeating-linear-gradient(0deg, transparent 0 3px, var(--tv-scanline) 3px 4px);
    opacity: var(--tv-scanline-opacity);
  }

  .crt-vignette {
    background: radial-gradient(ellipse at center, transparent 42%, var(--tv-vignette) 100%);
    box-shadow: inset 0 0 20px var(--tv-vignette-inner);
  }

  .media-overlay {
    position: absolute;
    z-index: 3;
    right: var(--tv-screen-overlay-inset);
    bottom: var(--tv-screen-overlay-inset);
    left: var(--tv-screen-overlay-inset);
    display: block;
    color: var(--tv-screen-muted);
    font: var(--tv-screen-overlay-size)/1 monospace;
    opacity: 0.9;
    pointer-events: none;
  }

  .media-status {
    display: block;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--tv-screen-glow);
    font-weight: 700;
  }

  .bezel-info {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr) max-content;
    align-items: center;
    gap: var(--tv-bezel-info-gap);
    width: 100%;
    box-sizing: border-box;
    min-width: 0;
    min-height: var(--tv-bezel-info-height);
    overflow: hidden;
    padding: var(--tv-bezel-info-padding);
    color: var(--tv-bezel-info-text);
    background: linear-gradient(180deg, var(--tv-bezel-highlight), var(--tv-bezel-info-background));
    border: 1px solid var(--tv-bezel-shadow);
    box-shadow: inset 0 1px 0 var(--tv-bezel-highlight), inset 0 -1px 0 var(--tv-bezel-deep);
    font: 700 var(--tv-bezel-info-size)/1 monospace;
    letter-spacing: 0.04em;
    white-space: nowrap;
  }

  .bezel-info > * {
    min-width: 0;
  }

  .bezel-info span {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .bezel-info .bezel-status {
    min-width: max-content;
    white-space: nowrap;
  }

  .bezel-info .bezel-status,
  .bezel-info .crt-status-toggle.active {
    color: var(--tv-bezel-info-accent);
  }

  .crt-status-toggle {
    min-width: 3.2rem;
    max-width: none;
    box-sizing: border-box;
    overflow: visible;
    padding: 0;
    color: var(--tv-bezel-info-text);
    background: transparent;
    border: 0;
    font: inherit;
    letter-spacing: inherit;
    cursor: pointer;
    white-space: nowrap;
  }

  .crt-status-toggle:hover,
  .crt-status-toggle:focus-visible {
    color: var(--tv-bezel-info-accent);
    opacity: 0.78;
    outline: 1px solid var(--tv-bezel-info-accent);
    outline-offset: 2px;
  }

  .bezel-info .bezel-track {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: center;
    white-space: nowrap;
  }

  .tv-lower-strip {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--tv-lower-gap);
    min-width: 0;
    min-height: var(--tv-lower-strip-height);
    padding: var(--tv-lower-padding);
    background: linear-gradient(180deg, var(--tv-wood-color), var(--tv-wood-dark));
    border: var(--tv-lower-border) solid var(--tv-wood-dark);
    box-shadow: inset 0 1px 0 var(--tv-wood-highlight), inset 0 -2px 0 var(--tv-wood-deep);
  }

  .physical-controls {
    display: flex;
    align-items: center;
    gap: var(--tv-button-gap);
    min-width: 0;
  }

  .physical-button {
    position: relative;
    display: grid;
    place-items: center;
    width: var(--tv-button-size);
    height: var(--tv-button-size);
    color: var(--tv-button-text);
    background:
      radial-gradient(ellipse at 31% 18%, var(--tv-button-top-highlight) 0%, rgb(255 247 222 / 12%) 22%, transparent 52%),
      radial-gradient(circle at 31% 23%, var(--tv-button-highlight) 0%, var(--tv-button-color) 46%, var(--tv-button-dark) 100%);
    border: var(--tv-button-border) solid var(--tv-button-dark);
    border-radius: 50%;
    box-shadow: inset 1px 1px 0 var(--tv-button-shine), inset -2px -3px 3px var(--tv-button-shadow), 0 2px 3px var(--tv-shadow);
    cursor: pointer;
  }

  .physical-button {
    padding: 0.28rem;
  }

  .physical-button :global(.playback-icon) {
    width: 56%;
    height: 56%;
    pointer-events: none;
  }

  .physical-button::before {
    position: absolute;
    top: 0.18rem;
    right: 24%;
    left: 24%;
    height: 0.3rem;
    content: "";
    background: radial-gradient(ellipse at center, rgb(255 247 222 / 24%) 0%, rgb(255 247 222 / 10%) 34%, transparent 76%);
    border-radius: 50%;
    filter: blur(0.035rem);
    opacity: 0.8;
    pointer-events: none;
  }

  .physical-button:hover,
  .physical-button:focus-visible {
    color: var(--tv-button-active-text);
    border-color: var(--tv-button-active-border);
    outline: 2px solid var(--tv-button-active-border);
    outline-offset: 2px;
  }

  .physical-button.active {
    color: var(--tv-button-text);
    border-color: var(--tv-button-active-border);
    outline: 2px solid var(--tv-button-active-border);
    outline-offset: 2px;
    box-shadow: inset 1px 1px 0 var(--tv-button-shine), inset -2px -3px 3px var(--tv-button-shadow), 0 0 0.45rem rgb(243 189 93 / 60%), 0 2px 3px var(--tv-shadow);
  }

  .physical-button.active:hover,
  .physical-button.active:focus-visible {
    color: var(--tv-button-active-text);
  }

  .physical-button:disabled {
    color: var(--tv-button-text);
    cursor: not-allowed;
    filter: none;
    outline: 0;
    transform: none;
  }

  .physical-button:disabled :global(.playback-icon) {
    opacity: 0.42;
  }

  .physical-button.active:disabled {
    color: var(--tv-button-text);
    border-color: var(--tv-button-active-border);
    outline: 2px solid var(--tv-button-active-border);
    outline-offset: 2px;
  }

  .physical-button.active:disabled :global(.playback-icon) {
    opacity: 0.62;
  }

  .physical-button:active {
    box-shadow: inset 2px 2px 0 var(--tv-button-shadow), inset -1px -1px 0 var(--tv-button-shine), 0 1px 1px var(--tv-shadow);
    transform: translateY(1px);
  }

  .physical-button:disabled:active {
    box-shadow: inset 1px 1px 0 var(--tv-button-shine), inset -2px -3px 3px var(--tv-button-shadow), 0 2px 3px var(--tv-shadow);
    transform: none;
  }

  .seek-module {
    position: relative;
    min-width: 0;
  }

  .seek-preview {
    position: absolute;
    z-index: 4;
    bottom: calc(100% + 0.5rem);
    display: grid;
    justify-items: center;
    width: 120px;
    transform: translateX(-50%);
    pointer-events: none;
    color: var(--tv-lower-text);
    font: 700 var(--tv-seek-label-size)/1 monospace;
  }

  .seek-preview-frame {
    display: grid;
    place-items: center;
    width: 120px;
    height: 90px;
    overflow: hidden;
    background: #24251f;
    border: 3px solid var(--tv-bezel-highlight);
    box-shadow: inset 0 0 0 1px var(--tv-shadow), 2px 2px 0 var(--tv-shadow);
  }

  .seek-preview-frame img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .seek-preview-placeholder {
    color: var(--tv-lower-text);
    font-size: 0.48rem;
    letter-spacing: 0.06em;
  }

  .seek-preview-placeholder.loading {
    animation: seek-preview-blink 850ms steps(2, end) infinite;
  }

  @keyframes seek-preview-blink {
    50% { opacity: 0.35; }
  }

  .seek-labels {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto auto;
    gap: var(--tv-seek-label-gap);
    margin-bottom: var(--tv-seek-label-margin);
    color: var(--tv-lower-text);
    font: var(--tv-seek-label-size)/1 monospace;
  }

  .seek-labels span:nth-child(2) {
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: center;
    white-space: nowrap;
  }

  .speed-button {
    min-width: 2.5rem;
    padding: 0.12rem 0.28rem;
    color: var(--tv-button-text);
    background: var(--tv-button-color);
    border: 1px solid var(--tv-button-shadow);
    border-color: var(--tv-button-shine) var(--tv-button-shadow) var(--tv-button-shadow) var(--tv-button-shine);
    border-radius: 999px;
    box-shadow: inset 1px 1px 0 var(--tv-button-shine), inset -1px -1px 0 var(--tv-button-shadow);
    font: 700 var(--tv-seek-label-size)/1 monospace;
    cursor: pointer;
  }

  .speed-button:hover,
  .speed-button:focus-visible {
    color: var(--tv-button-active-text);
    background: var(--tv-button-color);
    border-color: var(--tv-button-active-border);
    outline: none;
  }

  .speed-button:active {
    border-color: var(--tv-button-shadow) var(--tv-button-shine) var(--tv-button-shine) var(--tv-button-shadow);
    box-shadow: inset 1px 1px 0 var(--tv-button-shadow), inset -1px -1px 0 var(--tv-button-shine);
  }

  .speed-button:disabled {
    cursor: default;
    opacity: 0.58;
  }

  .seek-module input {
    width: 100%;
    height: var(--tv-seek-height);
    margin: 0;
    appearance: none;
    background: linear-gradient(to right, var(--tv-seek-progress-color) 0 var(--tv-seek-progress), var(--tv-seek-track) var(--tv-seek-progress) 100%);
    border: 1px solid var(--tv-seek-border);
    box-shadow: inset 1px 1px 0 var(--tv-seek-shadow), inset -1px -1px 0 var(--tv-seek-highlight);
    cursor: pointer;
  }

  .seek-module input::-webkit-slider-runnable-track {
    height: var(--tv-seek-track-height);
    background: transparent;
  }

  .seek-module input::-webkit-slider-thumb {
    width: var(--tv-seek-thumb-width);
    height: var(--tv-seek-thumb-height);
    margin-top: var(--tv-seek-thumb-offset);
    appearance: none;
    background: var(--tv-seek-thumb);
    border: 1px solid var(--tv-seek-thumb-border);
    box-shadow: inset 1px 1px 0 var(--tv-seek-highlight), inset -1px -1px 0 var(--tv-seek-shadow);
  }

  .seek-module input::-moz-range-track {
    height: var(--tv-seek-track-height);
    background: transparent;
  }

  .seek-module input::-moz-range-progress {
    height: var(--tv-seek-track-height);
    background: var(--tv-seek-progress-color);
  }

  .seek-module input::-moz-range-thumb {
    width: var(--tv-seek-thumb-width);
    height: var(--tv-seek-thumb-height);
    background: var(--tv-seek-thumb);
    border: 1px solid var(--tv-seek-thumb-border);
    border-radius: 0;
    box-shadow: inset 1px 1px 0 var(--tv-seek-highlight), inset -1px -1px 0 var(--tv-seek-shadow);
  }

  .tv-control-panel {
    position: relative;
    display: grid;
    grid-template-rows: auto auto minmax(70px, var(--tv-speaker-height)) auto;
    gap: var(--tv-dial-gap);
    min-width: 0;
    min-height: 0;
    padding: var(--tv-panel-padding);
    background:
      linear-gradient(90deg, var(--tv-panel-dark), transparent 12%, transparent 88%, var(--tv-panel-dark)),
      repeating-linear-gradient(12deg, var(--tv-panel-color) 0 11px, var(--tv-panel-grain) 11px 13px);
    border: var(--tv-panel-border) solid var(--tv-panel-dark);
    box-shadow: inset 1px 1px 0 var(--tv-panel-highlight), inset -2px -2px 0 var(--tv-panel-shadow);
  }

  .dial-unit {
    display: grid;
    justify-items: center;
    gap: var(--tv-dial-label-gap);
    min-width: 0;
  }

  .channel-unit {
    row-gap: 0;
  }

  .channel-unit .dial-wrap {
    margin-top: -0.125rem;
  }

  .channel-unit .dial-help {
    margin-top: -0.6375rem;
  }

  /* Tighten only the two requested panel transitions. */
  .volume-unit {
    margin-top: calc(var(--tv-dial-gap) * -0.85);
  }

  .dial-heading {
    display: flex;
    justify-content: space-between;
    width: 100%;
    color: var(--tv-panel-text);
    font: 700 var(--tv-dial-label-size)/1 monospace;
    letter-spacing: var(--tv-dial-label-spacing);
  }

  .dial-heading output {
    color: var(--tv-panel-accent);
  }

  .dial-wrap {
    position: relative;
    width: min(100%, var(--tv-channel-dial-size));
    aspect-ratio: 1;
  }

  .volume-wrap {
    width: min(100%, var(--tv-volume-dial-size));
  }

  .dial {
    position: absolute;
    inset: var(--tv-dial-inset);
    display: grid;
    place-items: center;
    color: var(--tv-dial-center-text);
    background: radial-gradient(circle at 34% 28%, var(--tv-dial-shine), transparent 21%), linear-gradient(145deg, var(--tv-dial-metal), var(--tv-dial-dark));
    border: var(--tv-dial-border) solid var(--tv-dial-dark);
    border-radius: 50%;
    box-shadow: inset 2px 2px 2px var(--tv-dial-highlight), inset -4px -4px 5px var(--tv-dial-shadow), 0 2px 3px var(--tv-shadow);
    cursor: ns-resize;
    touch-action: none;
    user-select: none;
  }

  .dial:focus-visible {
    outline: 2px solid var(--tv-panel-accent);
    outline-offset: 3px;
  }

  .dial-indicator {
    position: absolute;
    top: var(--tv-dial-indicator-inset);
    left: 50%;
    width: var(--tv-dial-indicator-width);
    height: var(--tv-dial-indicator-height);
    background: var(--tv-dial-indicator);
    border-radius: var(--tv-dial-indicator-radius);
    box-shadow: 0 0 2px var(--tv-dial-indicator-shadow);
    transform: translateX(-50%) rotate(var(--dial-angle));
    transform-origin: 50% calc(100% + var(--tv-dial-indicator-origin));
  }

  .dial-center {
    z-index: 1;
    display: grid;
    place-items: center;
    width: var(--tv-dial-center-size);
    height: var(--tv-dial-center-size);
    color: var(--tv-dial-center-text);
    background: var(--tv-dial-center);
    border: 1px solid var(--tv-dial-center-border);
    border-radius: 50%;
    font: 700 var(--tv-dial-center-label-size)/1 monospace;
  }

  .dial-help {
    color: var(--tv-panel-muted);
    font: var(--tv-dial-help-size)/1 monospace;
    letter-spacing: var(--tv-dial-help-spacing);
  }

  .speaker-grille {
    position: relative;
    display: grid;
    place-items: center;
    min-height: 0;
    margin-top: calc(var(--tv-dial-gap) * -0.95);
    padding: var(--tv-speaker-padding) var(--tv-speaker-padding) 1.75rem;
    overflow: hidden;
    background:
      linear-gradient(145deg, rgb(255 255 255 / 10%), transparent 34%, transparent 68%, rgb(0 0 0 / 30%)),
      linear-gradient(180deg, var(--tv-speaker-metal-highlight), var(--tv-speaker-texture) 48%, var(--tv-speaker-dark));
    border: var(--tv-speaker-border) solid var(--tv-speaker-frame);
    box-shadow: inset 1px 1px 0 var(--tv-speaker-light), inset -2px -2px 0 var(--tv-speaker-shadow), 0 2px 3px var(--tv-shadow);
  }

  .speaker-holes {
    display: grid;
    grid-template-columns: repeat(8, minmax(0, 1fr));
    align-content: center;
    justify-items: center;
    gap: 0.22rem;
    width: 100%;
    min-height: 0;
  }

  .speaker-hole {
    width: min(100%, 0.72rem);
    aspect-ratio: 1;
    background:
      radial-gradient(circle at 34% 25%, rgb(255 255 255 / 25%) 0 6%, transparent 23%),
      radial-gradient(circle, var(--tv-speaker-hole) 0 45%, #1b1f1d 56%, var(--tv-speaker-hole-edge) 66%, #111412 78%, #3b403b 100%);
    border-radius: 50%;
    box-shadow: inset 0 0.08rem 0.14rem rgb(0 0 0 / 88%), 0 -0.035rem 0.06rem rgb(255 255 255 / 22%);
    transition: background 160ms ease, box-shadow 160ms ease, transform 160ms ease;
  }

  .speaker-hole.lit {
    background:
      radial-gradient(circle at 35% 28%, #ffe5a7 0 7%, #e9aa4c 23%, #824a16 45%, #1b140c 68%, #5b5140 100%);
    box-shadow: 0 0 0.45rem var(--tv-speaker-hole-glow), inset 0 0.08rem 0.14rem rgb(0 0 0 / 78%), 0 -0.035rem 0.06rem rgb(255 226 155 / 42%);
    transform: scale(1.08);
  }

  .speaker-label {
    position: absolute;
    bottom: 0.3rem;
    left: 50%;
    z-index: 1;
    padding: var(--tv-speaker-label-padding);
    color: var(--tv-speaker-label-text);
    background: var(--tv-speaker-label-background);
    border: 1px solid rgb(255 219 165 / 20%);
    font: 700 var(--tv-speaker-label-size)/1 monospace;
    letter-spacing: var(--tv-speaker-label-spacing);
    text-shadow: 0 1px 1px rgb(0 0 0 / 80%);
    transform: translateX(-50%);
    white-space: nowrap;
  }

  .utility-controls {
    position: static;
    display: block;
  }

  .utility-buttons {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--tv-utility-gap);
  }

  .utility-button {
    width: auto;
    min-width: 0;
    height: var(--tv-utility-height);
    padding: var(--tv-utility-padding);
    overflow: hidden;
    border-radius: 0.28rem;
    background:
      linear-gradient(180deg, rgb(255 247 222 / 20%), transparent 25%),
      linear-gradient(180deg, #71695e 0%, #4b4741 48%, #25221e 100%);
    border-color: #918575 #171410 #171410 #918575;
    box-shadow: inset 1px 1px 0 rgb(255 247 222 / 24%), inset -2px -2px 0 rgb(15 12 9 / 80%), 0 2px 2px var(--tv-shadow);
    font: 700 var(--tv-dial-label-size)/1 monospace;
    letter-spacing: 0.08em;
    text-shadow: 0 1px 1px rgb(0 0 0 / 80%);
    transition: transform 80ms ease, box-shadow 80ms ease;
  }

  .utility-button::before {
    top: 0.14rem;
    right: 14%;
    left: 14%;
    height: 0.18rem;
    background: linear-gradient(90deg, transparent, rgb(255 247 222 / 28%), transparent);
    filter: blur(0.035rem);
  }

  .utility-button:active {
    box-shadow: inset 2px 2px 0 rgb(15 12 9 / 72%), inset -1px -1px 0 rgb(255 247 222 / 18%), 0 1px 1px var(--tv-shadow);
    transform: translateY(2px);
  }

  .audio-jacks {
    position: absolute;
    right: var(--tv-panel-padding);
    bottom: calc(var(--tv-panel-padding) + 3rem + 0.4rem);
    left: var(--tv-panel-padding);
    display: grid;
    place-items: center;
    align-content: start;
    min-width: 0;
    padding: 0.12rem 0;
  }

  .audio-jack-row {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--tv-audio-jack-gap);
    width: min(100%, 8rem);
  }

  .audio-jack-port {
    display: grid;
    justify-items: center;
    gap: 0.12rem;
    color: var(--tv-panel-muted);
    font: var(--tv-audio-jack-label-size)/1 monospace;
    letter-spacing: var(--tv-audio-jack-label-spacing);
  }

  .audio-jack-hole {
    display: block;
    width: var(--tv-audio-jack-size);
    aspect-ratio: 1;
    background:
      radial-gradient(circle at 35% 25%, rgb(255 255 255 / 22%) 0 7%, transparent 23%),
      radial-gradient(circle, #070908 0 42%, #151915 54%, #050706 68%, #6a7068 78%, #101310 100%);
    border-radius: 50%;
    box-shadow: inset 0 0.08rem 0.14rem rgb(0 0 0 / 90%), 0 -0.035rem 0.06rem rgb(255 255 255 / 20%);
  }

  .panel-branding {
    position: absolute;
    right: var(--tv-panel-padding);
    bottom: var(--tv-panel-padding);
    left: var(--tv-panel-padding);
    z-index: 1;
    display: grid;
    grid-template-columns: 2.1rem minmax(0, 1fr);
    align-items: center;
    gap: 0.32rem;
    min-width: 0;
    pointer-events: none;
  }

  .panel-brand-emblem {
    display: grid;
    place-items: center;
    width: 2.1rem;
    height: 1.28rem;
    color: var(--tv-panel-accent);
    background:
      linear-gradient(180deg, rgb(255 247 222 / 24%), transparent 30%),
      linear-gradient(180deg, #665f54, #302c27 56%, #1a1713);
    border: 1px solid #a08f76;
    border-radius: 50%;
    box-shadow: inset 1px 1px 0 rgb(255 247 222 / 30%), inset -1px -1px 0 rgb(15 12 9 / 80%), 0 1px 2px var(--tv-shadow);
    font: 800 0.56rem/1 monospace;
    letter-spacing: -0.12em;
    text-shadow: 0 1px 1px rgb(0 0 0 / 85%);
  }

  .panel-brand-copy {
    display: grid;
    gap: 0.14rem;
    min-width: 0;
    justify-items: center;
    padding: 0.2rem 0.24rem;
    overflow: hidden;
    background: rgb(38 12 4 / 22%);
    border: 1px solid rgb(255 219 165 / 12%);
    box-shadow: inset 1px 1px 0 rgb(255 247 222 / 7%), inset -1px -1px 0 rgb(13 3 0 / 30%);
  }

  .panel-brand-name {
    width: 100%;
    overflow: hidden;
    color: var(--tv-panel-accent);
    font: 700 0.64rem/1 monospace;
    letter-spacing: 0.13em;
    text-align: center;
    white-space: nowrap;
  }

  .panel-brand-subtitle {
    width: 100%;
    overflow: hidden;
    color: var(--tv-panel-text);
    font: 0.44rem/1 monospace;
    letter-spacing: 0.1em;
    text-align: center;
    white-space: nowrap;
  }

  .side-drawer {
    position: relative;
    align-self: stretch;
    display: flex;
    width: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background:
      linear-gradient(100deg, var(--tv-wood-dark) 0%, transparent 10%, transparent 90%, var(--tv-wood-dark) 100%),
      repeating-linear-gradient(169deg, transparent 0 7px, var(--tv-wood-grain) 7px 9px, transparent 9px 18px),
      linear-gradient(180deg, var(--tv-panel-color), var(--tv-panel-dark));
    border: var(--tv-casing-border) solid var(--tv-wood-dark);
    border-radius: var(--tv-drawer-radius);
    box-shadow: 0 var(--tv-casing-shadow-y) var(--tv-casing-shadow-blur) var(--tv-shadow), inset 0 1px 0 var(--tv-wood-highlight), inset 0 -7px 0 var(--tv-wood-deep);
  }

  .playlist-drawer :global(.playlist-panel) {
    width: 100%;
    min-width: 0;
    max-width: none;
    height: 100%;
    min-height: 0;
    border: 0;
    background: linear-gradient(180deg, var(--tv-panel-color), var(--tv-panel-dark));
  }

  .playlist-drawer :global(.playlist-viewport) {
    height: auto;
  }

  .equalizer-drawer :global(.equalizer-panel) {
    width: 100%;
    min-width: 0;
    min-height: 100%;
    overflow: auto;
    border: 0;
    background: linear-gradient(180deg, var(--tv-panel-color), var(--tv-panel-dark));
  }

  .equalizer-drawer :global(.bands) {
    min-height: 8rem;
  }

  .tv-feet {
    position: absolute;
    right: var(--tv-foot-inset);
    bottom: calc(var(--tv-foot-height) / -2);
    left: var(--tv-foot-inset);
    display: flex;
    justify-content: space-between;
    pointer-events: none;
  }

  .tv-feet span {
    display: block;
    width: var(--tv-foot-width);
    height: var(--tv-foot-height);
    background: linear-gradient(90deg, var(--tv-foot-dark), var(--tv-foot-color), var(--tv-foot-dark));
    border-radius: 0 0 var(--tv-foot-radius) var(--tv-foot-radius);
    box-shadow: 0 4px 3px var(--tv-shadow);
    transform: skewX(var(--tv-foot-skew));
  }

  .drop-overlay {
    position: absolute;
    z-index: 6;
    inset: var(--tv-drop-inset);
    display: grid;
    place-items: center;
    color: var(--tv-drop-text);
    background: var(--tv-drop-background);
    border: var(--tv-drop-border) dashed var(--tv-drop-border-color);
    font: 700 var(--tv-drop-size)/1 monospace;
    letter-spacing: var(--tv-drop-spacing);
    pointer-events: none;
  }

  .drop-active .retro-tv-casing {
    outline: var(--tv-drop-outline) solid var(--tv-drop-border-color);
    outline-offset: var(--tv-drop-outline-offset);
  }

  .playlist-drawer.drop-target {
    outline: var(--tv-drop-outline) solid var(--tv-drop-border-color);
    outline-offset: var(--tv-drop-outline-offset);
  }

  .playlist-drop-overlay {
    position: absolute;
    z-index: 9;
    inset: 0.7rem;
    display: grid;
    place-items: center;
    color: var(--tv-drop-text);
    background: var(--tv-drop-background);
    border: var(--tv-drop-border) dashed var(--tv-drop-border-color);
    font: 700 var(--tv-drop-size)/1 monospace;
    letter-spacing: var(--tv-drop-spacing);
    pointer-events: none;
  }

  @media (max-width: 1240px) {
    /* Fallback: keep the TV usable when the viewport cannot fit two columns. */
    .retro-tv-layout.drawer-open {
      display: block;
      max-width: var(--tv-casing-max-width);
    }

    .side-drawer {
      position: absolute;
      z-index: 8;
      top: var(--tv-drawer-top);
      right: calc(var(--tv-control-width) + var(--tv-panel-gap));
      bottom: var(--tv-drawer-bottom);
      width: var(--tv-drawer-width);
      height: auto;
    }
  }

  @media (max-width: 760px) {
    .tv-body {
      grid-template-columns: minmax(0, 1fr) var(--tv-control-width-compact);
    }

    .side-drawer {
      right: calc(var(--tv-control-width-compact) + var(--tv-panel-gap));
    }

    .tv-lower-strip {
      grid-template-columns: 1fr;
      gap: var(--tv-compact-lower-gap);
    }

    .physical-controls {
      justify-content: center;
    }

    .physical-controls-left {
      grid-row: 2;
    }

    .seek-module {
      grid-row: 1;
    }

    .physical-controls-right {
      grid-row: 3;
    }

    .seek-labels span:nth-child(2) {
      display: none;
    }

  }

  @media (max-width: 520px) {
    .retro-tv-stage {
      padding: var(--tv-stage-padding-compact);
    }

    .retro-tv-casing {
      min-height: 520px;
    }

    .tv-body {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto auto;
    }

    .tv-control-panel {
      grid-template-columns: 1fr 1fr;
      grid-template-rows: auto var(--tv-speaker-height) auto;
    }

    .speaker-grille {
      grid-column: 1 / -1;
    }

    .utility-controls {
      grid-column: 1 / -1;
    }

    .audio-jacks {
      grid-column: 1 / -1;
    }

    .side-drawer {
      right: var(--tv-casing-padding);
    }

  }
</style>
