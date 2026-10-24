<script lang="ts">
  import DialScale from "./DialScale.svelte";

  // "step" dials emit discrete steps (channel); "continuous" dials emit a value (volume).
  export let variant: "channel" | "volume";
  export let mode: "step" | "continuous";
  export let label: string;
  export let readout: string;
  export let ariaLabel: string;
  export let help: string;
  export let min: number;
  export let max: number;
  export let value: number;
  export let ariaValueNow: number;
  export let angle: number;
  export let centerText: string | number;
  export let labelStep: number;
  export let activeValue: number | null = null;
  export let showLabels = true;
  export let onStep: (direction: -1 | 1) => void = () => undefined;
  export let onChange: (value: number) => void = () => undefined;
  /** Click, Enter, or Space on the dial. */
  export let onActivate: () => void;

  type DialDrag = {
    pointerId: number;
    startY: number;
    startValue: number;
    startedAt: number;
    lastStep: number;
    lastValue: number;
    interactionScale: number;
  };

  let dialDrag: DialDrag | null = null;

  function clamp(next: number): number {
    return Math.min(Math.max(next, min), max);
  }

  function startDrag(event: PointerEvent): void {
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
      pointerId: event.pointerId,
      startY: event.clientY,
      startValue: value,
      startedAt: Date.now(),
      lastStep: 0,
      lastValue: value,
      interactionScale,
    };
  }

  function moveDrag(event: PointerEvent): void {
    if (!dialDrag || event.pointerId !== dialDrag.pointerId) return;

    const delta = (dialDrag.startY - event.clientY) / dialDrag.interactionScale;
    // TODO: upgrade this vertical drag interaction to full mouse-angle rotation.
    if (mode === "step") {
      const step = Math.trunc(delta / 28);
      if (step > dialDrag.lastStep) {
        for (let index = dialDrag.lastStep; index < step; index += 1) onStep(1);
      } else if (step < dialDrag.lastStep) {
        for (let index = dialDrag.lastStep; index > step; index -= 1) onStep(-1);
      }
      dialDrag.lastStep = step;
      return;
    }

    const nextValue = clamp(dialDrag.startValue + delta / 2);
    if (Math.round(nextValue) !== Math.round(dialDrag.lastValue)) {
      dialDrag.lastValue = nextValue;
      onChange(nextValue);
    }
  }

  function finishDrag(event: PointerEvent, shouldActivate = true): void {
    if (!dialDrag || event.pointerId !== dialDrag.pointerId) return;
    const activeDrag = dialDrag;
    const target = event.currentTarget as HTMLElement;
    if (target.hasPointerCapture(event.pointerId)) target.releasePointerCapture(event.pointerId);
    dialDrag = null;

    const wasClick = Math.abs(event.clientY - activeDrag.startY) < 8 * activeDrag.interactionScale && Date.now() - activeDrag.startedAt < 450;
    if (shouldActivate && wasClick) onActivate();
  }

  function handleKey(event: KeyboardEvent): void {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onActivate();
      return;
    }
    if (event.key !== "ArrowUp" && event.key !== "ArrowDown" && event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();

    const increase = event.key === "ArrowUp" || event.key === "ArrowRight";
    if (mode === "step") onStep(increase ? 1 : -1);
    else onChange(clamp(value + (increase ? 5 : -5)));
  }
</script>

<div class={`dial-unit ${variant}-unit`}>
  <div class="dial-heading">
    <span>{label}</span>
    <output>{readout}</output>
  </div>
  <div class:volume-wrap={variant === "volume"} class="dial-wrap">
    <DialScale minValue={min} maxValue={max} {labelStep} {activeValue} {showLabels} />
    <div
      class={`dial ${variant}-dial`}
      role="slider"
      tabindex="0"
      aria-label={ariaLabel}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={ariaValueNow}
      style={`--dial-angle: ${angle}deg`}
      onpointerdown={startDrag}
      onpointermove={moveDrag}
      onpointerup={finishDrag}
      onpointercancel={(event) => finishDrag(event, false)}
      onkeydown={handleKey}
    >
      <span class="dial-indicator" aria-hidden="true"></span>
      <span class="dial-center" aria-hidden="true">{centerText}</span>
    </div>
  </div>
  <span class="dial-help">{help}</span>
</div>

<style>
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
</style>
