<script lang="ts">
  type PlaybackIconName = "previous" | "play" | "stop" | "next" | "pause" | "volume" | "mute";

  export let name: PlaybackIconName;
</script>

<svg class="playback-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
  {#if name === "previous"}
    <path d="M11.7 4.6 3.7 11.1a1.15 1.15 0 0 0 0 1.8l8 6.5a1.1 1.1 0 0 0 1.8-.9v-2.2l5.9 3.1a1.1 1.1 0 0 0 1.6-1V5.6a1.1 1.1 0 0 0-1.6-1l-5.9 3.1V5.5a1.1 1.1 0 0 0-1.8-.9Z" />
  {:else if name === "play"}
    <path d="M6.3 4.3a1.15 1.15 0 0 1 1.8-.9l11.2 7.7a1.1 1.1 0 0 1 0 1.8L8.1 20.6a1.15 1.15 0 0 1-1.8-.9V4.3Z" />
  {:else if name === "stop"}
    <rect x="5" y="5" width="14" height="14" rx="1.4" />
  {:else if name === "next"}
    <path d="M12.3 4.6a1.1 1.1 0 0 0-1.8.9v2.2L4.6 4.6a1.1 1.1 0 0 0-1.6 1v12.8a1.1 1.1 0 0 0 1.6 1l5.9-3.1v2.2a1.1 1.1 0 0 0 1.8.9l8-6.5a1.15 1.15 0 0 0 0-1.8l-8-6.5Z" />
  {:else if name === "pause"}
    <path d="M5.4 5.1A1.1 1.1 0 0 1 6.5 4h2.2a1.1 1.1 0 0 1 1.1 1.1v13.8A1.1 1.1 0 0 1 8.7 20H6.5a1.1 1.1 0 0 1-1.1-1.1V5.1Zm8.2 0A1.1 1.1 0 0 1 14.7 4h2.2A1.1 1.1 0 0 1 18 5.1v13.8a1.1 1.1 0 0 1-1.1 1.1h-2.2a1.1 1.1 0 0 1-1.1-1.1V5.1Z" />
  {:else if name === "mute"}
    <path d="M3.2 9.1v5.8h4.1l6 5V4.1l-6 5H3.2Z" />
    <path class="playback-icon-line" d="m16.4 9 4.5 6m0-6-4.5 6" />
  {:else}
    <path d="M3.2 9.1v5.8h4.1l6 5V4.1l-6 5H3.2Z" />
    <path class="playback-icon-line" d="M16.2 9.3a4.1 4.1 0 0 1 0 5.4m2.4-7.7a7.5 7.5 0 0 1 0 10" />
  {/if}
</svg>

<style>
  .playback-icon {
    display: block;
    width: 100%;
    height: 100%;
    overflow: visible;
    fill: currentColor;
  }

  .playback-icon-line {
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2.35;
  }
</style>
