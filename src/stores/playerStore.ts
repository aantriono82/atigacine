import { writable } from "svelte/store";
import type { PlayerState } from "../types";

const initialState: PlayerState = {
  initialized: false,
  status: "idle",
  buffering: false,
  bufferDuration: 0,
  currentFile: null,
  format: null,
  videoCodec: null,
  audioCodec: null,
  position: 0,
  duration: 0,
  volume: 80,
  muted: false,
  width: null,
  height: null,
  message: "Ready",
};

export const playerStore = writable<PlayerState>(initialState);

export function updatePlayer(patch: Partial<PlayerState>): void {
  playerStore.update((state) => ({ ...state, ...patch }));
}

export function resetPlayer(): void {
  playerStore.set(initialState);
}
