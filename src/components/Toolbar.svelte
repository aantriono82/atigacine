<script lang="ts">
  import PlaybackIcon from "./PlaybackIcon.svelte";
  import SeekBar from "./SeekBar.svelte";
  import type { ThumbnailPreview } from "../lib/thumbnailPreview";
  import type { PlaybackStatus } from "../types";

  export let status: PlaybackStatus;
  export let isPlaying: boolean;
  export let muted: boolean;
  export let position: number;
  export let duration: number;
  export let trackName: string;
  export let speed: number;
  export let speedDisabled = false;
  export let thumbnailPreview: ThumbnailPreview | null = null;
  export let onPlay: () => void;
  export let onPause: () => void;
  export let onStop: () => void;
  export let onPrevious: () => void;
  export let onNext: () => void;
  export let onMute: () => void;
  export let onSeek: (position: number) => void;
  export let onSeekHover: (event: MouseEvent) => void;
  export let onSeekLeave: () => void;
  export let onSpeedCycle: () => void;
</script>

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

  <SeekBar {position} {duration} {trackName} {speed} {speedDisabled} {thumbnailPreview} {onSeek} {onSeekHover} {onSeekLeave} {onSpeedCycle} />

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

<style>
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

  @media (max-width: 760px) {
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

    .physical-controls-right {
      grid-row: 3;
    }
  }
</style>
