import { writable } from "svelte/store";

export const EQUALIZER_BANDS = [
  { label: "BASS", frequency: 60 },
  { label: "LOW-MID", frequency: 250 },
  { label: "MID", frequency: 1_000 },
  { label: "HIGH-MID", frequency: 4_000 },
  { label: "TREBLE", frequency: 12_000 },
] as const;

export type EqualizerPreset = "flat" | "bass-boost" | "vocal-boost" | "custom";

export interface EqualizerState {
  visible: boolean;
  enabled: boolean;
  normalize: boolean;
  preset: EqualizerPreset;
  gains: number[];
}

const PRESETS: Record<Exclude<EqualizerPreset, "custom">, number[]> = {
  flat: [0, 0, 0, 0, 0],
  "bass-boost": [7, 5, 2, 0, 0],
  "vocal-boost": [0, -2, 3, 5, 3],
};

const initialState: EqualizerState = {
  visible: false,
  enabled: false,
  normalize: false,
  preset: "flat",
  gains: [...PRESETS.flat],
};

const STORAGE_KEY = "atiga-cine-equalizer";

function clampGain(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? Math.min(12, Math.max(-12, value)) : 0;
}

function readState(): EqualizerState {
  if (typeof localStorage === "undefined") return { ...initialState, gains: [...initialState.gains] };

  try {
    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null") as Partial<EqualizerState> | null;
    const gains = Array.isArray(stored?.gains) && stored.gains.length === EQUALIZER_BANDS.length
      ? stored.gains.map(clampGain)
      : [...initialState.gains];
    const preset = stored?.preset === "flat" || stored?.preset === "bass-boost" || stored?.preset === "vocal-boost" || stored?.preset === "custom"
      ? stored.preset
      : initialState.preset;

    return {
      visible: stored?.visible === true,
      enabled: stored?.enabled === true,
      normalize: stored?.normalize === true,
      preset,
      gains,
    };
  } catch {
    return { ...initialState, gains: [...initialState.gains] };
  }
}

export const equalizerStore = writable<EqualizerState>(readState());

if (typeof localStorage !== "undefined") {
  equalizerStore.subscribe((state) => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
    } catch {
      // A full or unavailable WebView storage should never interrupt playback.
    }
  });
}

export function setEqualizerVisible(visible: boolean): void {
  equalizerStore.update((state) => ({ ...state, visible }));
}

export function setEqualizerEnabled(enabled: boolean): void {
  equalizerStore.update((state) => ({ ...state, enabled }));
}

export function setNormalizationEnabled(normalize: boolean): void {
  equalizerStore.update((state) => ({ ...state, normalize }));
}

export function setEqualizerPreset(preset: Exclude<EqualizerPreset, "custom">): void {
  equalizerStore.update((state) => ({
    ...state,
    preset,
    gains: [...PRESETS[preset]],
  }));
}

export function setEqualizerGain(index: number, gain: number): void {
  equalizerStore.update((state) => {
    if (index < 0 || index >= state.gains.length) return state;

    const gains = [...state.gains];
    gains[index] = clampGain(gain);
    return { ...state, preset: "custom", gains };
  });
}
