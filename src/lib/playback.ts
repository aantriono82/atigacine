import type { RepeatMode } from "../types";

const STORAGE_KEY = `atiga-cine-v${__APP_VERSION__}-playback`;

export interface TrackPlaybackPosition {
  playlistId: string;
  itemId: string;
  path: string;
  position: number;
  updatedAt: number;
}

export interface PlaybackPersistence {
  lastPlaylistId: string | null;
  lastItemId: string | null;
  positions: Record<string, TrackPlaybackPosition>;
  repeatModes: Record<string, RepeatMode>;
  shuffleModes: Record<string, boolean>;
}

export function createPlaybackPersistence(): PlaybackPersistence {
  return {
    lastPlaylistId: null,
    lastItemId: null,
    positions: {},
    repeatModes: {},
    shuffleModes: {},
  };
}

export function playbackPositionKey(playlistId: string, itemId: string): string {
  return `${playlistId}:${itemId}`;
}

function isRepeatMode(value: unknown): value is RepeatMode {
  return value === "off" || value === "one" || value === "all";
}

function finiteNonNegative(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 ? value : null;
}

function normalizePersistence(value: unknown): PlaybackPersistence {
  const result = createPlaybackPersistence();
  if (!value || typeof value !== "object") return result;

  const candidate = value as Partial<PlaybackPersistence>;
  result.lastPlaylistId = typeof candidate.lastPlaylistId === "string" ? candidate.lastPlaylistId : null;
  result.lastItemId = typeof candidate.lastItemId === "string" ? candidate.lastItemId : null;

  if (candidate.positions && typeof candidate.positions === "object") {
    for (const [key, rawPosition] of Object.entries(candidate.positions)) {
      if (!rawPosition || typeof rawPosition !== "object") continue;
      const position = rawPosition as Partial<TrackPlaybackPosition>;
      const seconds = finiteNonNegative(position.position);
      if (
        seconds === null ||
        typeof position.playlistId !== "string" ||
        typeof position.itemId !== "string" ||
        typeof position.path !== "string"
      ) {
        continue;
      }

      result.positions[key] = {
        playlistId: position.playlistId,
        itemId: position.itemId,
        path: position.path,
        position: seconds,
        updatedAt: typeof position.updatedAt === "number" && Number.isFinite(position.updatedAt) ? position.updatedAt : 0,
      };
    }
  }

  if (candidate.repeatModes && typeof candidate.repeatModes === "object") {
    for (const [playlistId, mode] of Object.entries(candidate.repeatModes)) {
      if (isRepeatMode(mode)) result.repeatModes[playlistId] = mode;
    }
  }

  if (candidate.shuffleModes && typeof candidate.shuffleModes === "object") {
    for (const [playlistId, enabled] of Object.entries(candidate.shuffleModes)) {
      if (typeof enabled === "boolean") result.shuffleModes[playlistId] = enabled;
    }
  }

  return result;
}

export function loadPlaybackPersistence(): PlaybackPersistence {
  if (typeof localStorage === "undefined") return createPlaybackPersistence();

  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored ? normalizePersistence(JSON.parse(stored)) : createPlaybackPersistence();
  } catch {
    return createPlaybackPersistence();
  }
}

export function savePlaybackPersistence(state: PlaybackPersistence): void {
  if (typeof localStorage === "undefined") return;

  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // A full or unavailable WebView storage should never interrupt playback.
  }
}
