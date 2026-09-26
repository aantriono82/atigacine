<script lang="ts">
  export let minValue = 0;
  export let maxValue = 100;
  export let labelStep = 20;
  export let showLabels = true;
  export let activeValue: number | null = null;
  export let startAngle = -135;
  export let endAngle = 135;

  type ScaleLabel = {
    text: string;
    angle: number;
    active: boolean;
  };

  function createLabels(minimum: number, maximum: number, step: number, highlightedValue: number | null): ScaleLabel[] {
    if (!Number.isFinite(minimum) || !Number.isFinite(maximum) || maximum < minimum) return [];
    const safeStep = Number.isFinite(step) && step > 0 ? step : maximum - minimum || 1;
    const values: number[] = [];

    for (let value = minimum; value < maximum; value += safeStep) values.push(value);
    values.push(maximum);

    if (highlightedValue !== null && highlightedValue >= minimum && highlightedValue <= maximum) values.push(highlightedValue);

    return values
      .sort((left, right) => left - right)
      .filter((value, index, allValues) => index === 0 || value !== allValues[index - 1])
      .map((value) => ({
        text: String(Math.round(value)),
        angle: startAngle + ((value - minimum) / (maximum - minimum || 1)) * (endAngle - startAngle),
        active: highlightedValue !== null && Math.round(value) === Math.round(highlightedValue),
      }));
  }

  $: labels = showLabels ? createLabels(minValue, maxValue, labelStep, activeValue) : [];
</script>

<div class="dial-scale" aria-hidden="true">
  <div class="dial-ticks"></div>
  {#each labels as label (label.text)}
    <span class:dial-scale-active={label.active} class="dial-scale-marker" style={`--dial-scale-angle: ${label.angle}deg; --dial-scale-counter-angle: ${-label.angle}deg`}>
      <span class="dial-scale-label">{label.text}</span>
    </span>
  {/each}
</div>

<style>
  .dial-scale {
    position: absolute;
    z-index: 0;
    inset: 0;
    pointer-events: none;
  }

  .dial-ticks {
    position: absolute;
    inset: 0;
    background: repeating-conic-gradient(from -135deg, var(--tv-dial-tick) 0 1.5deg, transparent 1.5deg 15deg);
    border-radius: 50%;
    mask: radial-gradient(circle, transparent 0 68%, var(--tv-mask) 69% 100%);
  }

  .dial-scale-marker {
    position: absolute;
    inset: 0;
    transform: rotate(var(--dial-scale-angle));
  }

  .dial-scale-label {
    position: absolute;
    top: var(--tv-dial-scale-label-inset);
    left: 50%;
    display: block;
    min-width: 1.1rem;
    color: var(--tv-dial-scale-text);
    font: 700 var(--tv-dial-scale-label-size)/1 monospace;
    text-align: center;
    text-shadow: 0 1px 2px var(--tv-shadow);
    transform: translateX(-50%) rotate(var(--dial-scale-counter-angle));
  }

  .dial-scale-active .dial-scale-label {
    color: var(--tv-dial-scale-active-text);
    font-weight: 800;
    text-shadow: 0 0 0.22rem var(--tv-dial-scale-active-glow), 0 1px 2px var(--tv-shadow);
  }
</style>
