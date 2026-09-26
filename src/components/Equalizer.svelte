<script lang="ts">
  import { onMount } from "svelte";
  import { EQUALIZER_BANDS, type EqualizerPreset } from "../stores/equalizerStore";

  export let enabled: boolean;
  export let normalize: boolean;
  export let preset: EqualizerPreset;
  export let gains: number[];
  export let onToggle: () => void;
  export let onToggleNormalize: () => void;
  export let onPreset: (preset: Exclude<EqualizerPreset, "custom">) => void;
  export let onGain: (index: number, gain: number) => void;
  export let onClose: () => void;
  export let isPlaying: boolean;
  export let windowActive: boolean;

  const VISUALIZER_BAR_COUNT = 16;
  let visualizerTick = 0;
  let visualizerLevels = Array.from({ length: VISUALIZER_BAR_COUNT }, () => 0.08);
  let visualizerTimer: number | undefined;
  let mounted = false;

  function updateVisualizer(): void {
    visualizerTick += 1;

    if (!isPlaying) {
      visualizerLevels = visualizerLevels.map((level) => Math.max(0.06, level * 0.76));
      return;
    }

    // TODO(stage 2): replace these procedural levels with real libmpv astats metadata.
    visualizerLevels = visualizerLevels.map((level, index) => {
      const phase = visualizerTick * 0.34 + index * 0.71;
      const wave = (Math.sin(phase) + 1) / 2;
      const ripple = (Math.sin(visualizerTick * 0.17 + index * 1.83) + 1) / 2;
      const target = Math.min(0.96, 0.16 + wave * 0.48 + ripple * 0.28);
      const decayed = level * 0.78;
      return Math.max(target, decayed);
    });
  }

  function startVisualizer(): void {
    if (!mounted || !isPlaying || !windowActive || visualizerTimer !== undefined) return;
    visualizerTimer = window.setInterval(updateVisualizer, 90);
  }

  function stopVisualizer(): void {
    if (visualizerTimer === undefined) return;
    window.clearInterval(visualizerTimer);
    visualizerTimer = undefined;
  }

  $: if (isPlaying && windowActive) {
    startVisualizer();
  } else {
    stopVisualizer();
  }

  onMount(() => {
    mounted = true;
    startVisualizer();
    return () => {
      mounted = false;
      stopVisualizer();
    };
  });
</script>

<section class="equalizer-panel" aria-label="Audio equalizer">
  <div class="equalizer-heading">
    <div>
      <p class="panel-kicker">AUDIO FILTER</p>
      <h2>Equalizer <span>5 BAND</span></h2>
    </div>
    <div class="heading-actions">
      <label class="enable-toggle">
        <input type="checkbox" checked={enabled} onchange={onToggle} />
        <span>{enabled ? "ON" : "OFF"}</span>
      </label>
      <label class="enable-toggle normalize-toggle" title="Normalize loudness after equalizer">
        <input type="checkbox" checked={normalize} onchange={onToggleNormalize} />
        <span>{normalize ? "AUTO VOL ON" : "AUTO VOL"}</span>
      </label>
      <button class="panel-close" type="button" aria-label="Close equalizer drawer" title="Close equalizer drawer" onclick={onClose}>×</button>
    </div>
  </div>

  <div class="visualizer" aria-label="Procedural audio visualizer" aria-hidden="true">
    {#each visualizerLevels as level, index (index)}
      <span class="visualizer-bar" style={`height: ${Math.round(level * 100)}%`}></span>
    {/each}
  </div>

  <div class="equalizer-controls">
    <label class="preset-control">
      <span>PRESET</span>
      <select
        value={preset}
        onchange={(event) => {
          const value = event.currentTarget.value as EqualizerPreset;
          if (value !== "custom") onPreset(value);
        }}
      >
        <option value="flat">FLAT</option>
        <option value="bass-boost">BASS BOOST</option>
        <option value="vocal-boost">VOCAL BOOST</option>
        <option value="custom">CUSTOM</option>
      </select>
    </label>
    <span class="filter-note">LIBMPV / AF</span>
  </div>

  <div class="bands">
    {#each EQUALIZER_BANDS as band, index (band.frequency)}
      <label class="band" title={`${band.label} ${band.frequency} Hz`}>
        <span class="gain">{gains[index] > 0 ? "+" : ""}{Math.round(gains[index] ?? 0)} dB</span>
        <span class="slider-well">
          <input
            aria-label={`${band.label} gain`}
            type="range"
            min="-12"
            max="12"
            step="1"
            value={gains[index] ?? 0}
            oninput={(event) => onGain(index, Number(event.currentTarget.value))}
          />
        </span>
        <span class="band-label">{band.label}</span>
        <span class="frequency">{band.frequency >= 1000 ? `${band.frequency / 1000}K` : band.frequency}Hz</span>
      </label>
    {/each}
  </div>
</section>

<style>
  .equalizer-panel {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    min-height: 100%;
    padding: 0.45rem 0.7rem 0.6rem;
    color: var(--skin-text);
    background: var(--skin-surface-deep);
    border: 1px solid var(--skin-border);
  }

  .equalizer-heading,
  .equalizer-controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .heading-actions {
    display: flex;
    align-items: center;
    gap: 0.45rem;
  }

  .panel-close {
    display: grid;
    place-items: center;
    width: 23px;
    height: 23px;
    flex: 0 0 auto;
    color: var(--skin-accent-strong);
    background: var(--skin-surface-raised);
    border: 1px solid var(--button-shadow);
    border-color: var(--button-highlight) var(--button-shadow) var(--button-shadow) var(--button-highlight);
    box-shadow: inset 1px 1px 0 var(--button-highlight), inset -1px -1px 0 var(--button-shadow);
    font: 700 0.9rem/1 monospace;
    cursor: pointer;
    transition: color 100ms ease, transform 100ms ease, border-color 100ms ease;
  }

  .panel-close:hover,
  .panel-close:focus-visible {
    color: var(--skin-text);
    border-color: var(--skin-accent);
    outline: none;
    transform: scale(1.08);
  }

  .panel-close:active {
    border-color: var(--button-shadow) var(--button-highlight) var(--button-highlight) var(--button-shadow);
    box-shadow: inset 1px 1px 0 var(--button-shadow), inset -1px -1px 0 var(--button-highlight);
    transform: scale(0.98);
  }

  .visualizer {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 0.28rem;
    height: 8.4rem;
    padding: 0.55rem 0.65rem;
    background:
      linear-gradient(180deg, rgb(0 0 0 / 12%), transparent 55%),
      repeating-linear-gradient(0deg, transparent 0 0.8rem, rgb(255 187 73 / 8%) 0.8rem 0.84rem),
      var(--skin-surface-deep);
    border: 1px solid var(--skin-border);
    box-shadow: inset 0 0 0 1px rgb(0 0 0 / 15%), inset 0 -3px 0 rgb(0 0 0 / 22%);
  }

  .visualizer-bar {
    display: block;
    width: 100%;
    max-width: 0.75rem;
    min-height: 6%;
    background: linear-gradient(to top, #36b86b 0%, #c7d34d 48%, #e99a35 76%, #d84432 100%);
    border: 1px solid rgb(255 222 139 / 22%);
    box-shadow: 0 0 0.25rem rgb(221 91 39 / 45%);
    transition: height 140ms ease-out;
  }

  .panel-kicker {
    margin: 0 0 0.2rem;
    color: var(--skin-accent);
    font: 700 0.55rem/1 monospace;
    letter-spacing: 0.2em;
  }

  h2 {
    margin: 0;
    font: 700 0.95rem/1 Georgia, serif;
  }

  h2 span,
  .filter-note,
  .preset-control span,
  .frequency {
    color: var(--skin-text-muted);
    font: 0.54rem/1 monospace;
    letter-spacing: 0.06em;
  }

  .enable-toggle,
  .preset-control {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    color: var(--skin-accent-strong);
    font: 700 0.6rem/1 monospace;
  }

  .normalize-toggle span {
    color: var(--skin-accent);
  }

  .enable-toggle input {
    accent-color: var(--skin-accent);
  }

  .preset-control select {
    padding: 0.2rem 0.25rem;
    color: var(--skin-text);
    background: var(--skin-surface-raised);
    border: 1px solid var(--skin-border);
    font: 0.6rem/1 monospace;
  }

  .bands {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    align-items: start;
    gap: 0.4rem;
    min-height: 11.8rem;
  }

  .band {
    display: grid;
    grid-template-rows: 1rem 8.2rem auto auto;
    justify-items: center;
    align-content: start;
    gap: 0.28rem;
    min-width: 0;
    color: var(--skin-text-muted);
    font: 0.52rem/1 monospace;
  }

  .gain {
    display: grid;
    align-items: center;
    width: 100%;
    min-width: 0;
    height: 1rem;
    color: var(--skin-accent-strong);
    font: 0.58rem/1 monospace;
    text-align: center;
  }

  .slider-well {
    display: grid;
    place-items: center;
    width: 100%;
    min-width: 0;
    min-height: 8.2rem;
    padding-block: 0.7rem;
  }

  .band input[type="range"] {
    width: 6.7rem;
    height: 1rem;
    margin: 0;
    accent-color: var(--skin-accent);
    transform: rotate(-90deg);
    cursor: pointer;
  }

  .band-label {
    color: var(--skin-text-muted);
    font-size: 0.53rem;
    text-align: center;
  }
</style>
