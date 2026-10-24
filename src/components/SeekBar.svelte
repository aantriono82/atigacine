<script lang="ts">
  import { formatTime } from "../lib/time";
  import type { ThumbnailPreview } from "../lib/thumbnailPreview";

  export let position: number;
  export let duration: number;
  export let trackName: string;
  export let speed: number;
  export let speedDisabled = false;
  export let thumbnailPreview: ThumbnailPreview | null = null;
  export let onSeek: (position: number) => void;
  export let onSeekHover: (event: MouseEvent) => void;
  export let onSeekLeave: () => void;
  export let onSpeedCycle: () => void;

  $: safeDuration = Number.isFinite(duration) && duration > 0 ? duration : 0;
  $: safePosition = Math.min(Math.max(position, 0), safeDuration || 1);

  function formatSpeed(value: number): string {
    return `${value}x`;
  }

  function seekFromInput(event: Event): void {
    onSeek(Number((event.currentTarget as HTMLInputElement).value));
  }
</script>

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
    <span title={trackName}>{trackName || "TUNER READY"}</span>
    <button class="speed-button" type="button" title="Cycle playback speed" aria-label={`Playback speed ${formatSpeed(speed)}`} onclick={onSpeedCycle} disabled={speedDisabled}>{formatSpeed(speed)}</button>
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

<style>
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

  @media (max-width: 760px) {
    .seek-module {
      grid-row: 1;
    }

    .seek-labels span:nth-child(2) {
      display: none;
    }
  }
</style>
