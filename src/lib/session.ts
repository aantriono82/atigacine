import { invoke } from "@tauri-apps/api/core";
import { DEFAULT_PLAYLIST_ID, DEFAULT_PLAYLIST_NAME } from "../stores/playlistStore";
import type { PlaylistItem, PlaylistSession } from "../types";

const STORAGE_KEY = "atiga-cine-session";
const LEGACY_STORAGE_KEY = "atiga-cine-playlist";

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function migrateSession(value: unknown): PlaylistSession {
  if (Array.isArray(value)) {
    return {
      playlists: [{ id: DEFAULT_PLAYLIST_ID, name: DEFAULT_PLAYLIST_NAME, items: value as PlaylistItem[] }],
      activePlaylistId: DEFAULT_PLAYLIST_ID,
    };
  }

  if (value && typeof value === "object") {
    const candidate = value as Partial<PlaylistSession>;
    if (Array.isArray(candidate.playlists)) {
      return {
        playlists: candidate.playlists,
        activePlaylistId: typeof candidate.activePlaylistId === "string" ? candidate.activePlaylistId : null,
      };
    }
  }

  return { playlists: [], activePlaylistId: DEFAULT_PLAYLIST_ID };
}

export async function loadSession(): Promise<PlaylistSession> {
  if (isTauri()) {
    return invoke<PlaylistSession>("load_playlist_session");
  }

  try {
    const stored = localStorage.getItem(STORAGE_KEY) ?? localStorage.getItem(LEGACY_STORAGE_KEY);
    return stored ? migrateSession(JSON.parse(stored)) : migrateSession(null);
  } catch {
    return migrateSession(null);
  }
}

export async function saveSession(session: PlaylistSession): Promise<void> {
  if (isTauri()) {
    await invoke("save_playlist_session", { session });
    return;
  }

  localStorage.setItem(STORAGE_KEY, JSON.stringify(session));
}
