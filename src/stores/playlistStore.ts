import { derived, get, writable } from "svelte/store";
import { fileName } from "../lib/media";
import type { Playlist, PlaylistItem, PlaylistSession } from "../types";

export const MAX_PLAYLISTS = 100;
export const MAX_PLAYLIST_ITEMS = 2_000;
export const DEFAULT_PLAYLIST_ID = "default-playlist";
export const DEFAULT_PLAYLIST_NAME = "My Playlist";

export const playlistStore = writable<Playlist[]>([createPlaylistRecord(DEFAULT_PLAYLIST_NAME, DEFAULT_PLAYLIST_ID)]);
export const activePlaylistId = writable<string>(DEFAULT_PLAYLIST_ID);
export const currentItemId = writable<string | null>(null);
export const activePlaylistStore = derived([playlistStore, activePlaylistId], ([$playlists, $activeId]) =>
  $playlists.find((playlist) => playlist.id === $activeId) ?? $playlists[0] ?? null,
);

function createId(seed: string): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }

  return `${seed}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function createPlaylistRecord(name: string, id = createId(name)): Playlist {
  return { id, name: name.trim() || DEFAULT_PLAYLIST_NAME, items: [] };
}

function normalizeItems(items: PlaylistItem[]): PlaylistItem[] {
  const paths = new Set<string>();
  const ids = new Set<string>();
  const normalized: PlaylistItem[] = [];

  for (const item of items) {
    const path = typeof item?.path === "string" ? item.path.trim() : "";
    if (!path || paths.has(path) || normalized.length >= MAX_PLAYLIST_ITEMS) continue;

    let id = typeof item?.id === "string" ? item.id.trim() : "";
    if (!id || ids.has(id)) id = createId(path);

    paths.add(path);
    ids.add(id);
    normalized.push({
      id,
      path,
      name: typeof item?.name === "string" && item.name.trim() ? item.name.trim() : fileName(path),
    });
  }

  return normalized;
}

function normalizePlaylists(playlists: Playlist[]): Playlist[] {
  const ids = new Set<string>();
  const normalized: Playlist[] = [];

  for (const playlist of playlists) {
    if (!playlist || normalized.length >= MAX_PLAYLISTS) continue;

    let id = typeof playlist.id === "string" ? playlist.id.trim() : "";
    if (!id || ids.has(id)) id = createId(playlist.name || "playlist");

    ids.add(id);
    normalized.push({
      id,
      name: typeof playlist.name === "string" && playlist.name.trim() ? playlist.name.trim() : `Playlist ${normalized.length + 1}`,
      items: normalizeItems(Array.isArray(playlist.items) ? playlist.items : []),
    });
  }

  return normalized.length > 0 ? normalized : [createPlaylistRecord(DEFAULT_PLAYLIST_NAME, DEFAULT_PLAYLIST_ID)];
}

export function getActivePlaylist(): Playlist {
  return get(activePlaylistStore) ?? get(playlistStore)[0];
}

export function getActiveItems(): PlaylistItem[] {
  return getActivePlaylist()?.items ?? [];
}

export function createPlaylist(name: string): Playlist {
  const trimmedName = name.trim();
  const playlist = createPlaylistRecord(trimmedName || `Playlist ${get(playlistStore).length + 1}`);

  playlistStore.update((playlists) => (playlists.length >= MAX_PLAYLISTS ? playlists : [...playlists, playlist]));
  activePlaylistId.set(playlist.id);
  currentItemId.set(null);
  return playlist;
}

export function renamePlaylist(id: string, name: string): void {
  const trimmedName = name.trim();
  if (!trimmedName) return;

  playlistStore.update((playlists) => playlists.map((playlist) => playlist.id === id ? { ...playlist, name: trimmedName } : playlist));
}

export function deletePlaylist(id: string): Playlist | undefined {
  const playlists = get(playlistStore);
  const deleted = playlists.find((playlist) => playlist.id === id);
  if (!deleted || playlists.length <= 1) return undefined;

  const remaining = playlists.filter((playlist) => playlist.id !== id);
  playlistStore.set(remaining);

  if (get(activePlaylistId) === id) {
    const fallback = remaining[0];
    activePlaylistId.set(fallback.id);
    currentItemId.set(fallback.items[0]?.id ?? null);
  }

  return deleted;
}

export function setActivePlaylist(id: string): void {
  const playlist = get(playlistStore).find((entry) => entry.id === id);
  if (!playlist) return;

  activePlaylistId.set(id);
  const current = get(currentItemId);
  currentItemId.set(playlist.items.some((item) => item.id === current) ? current : playlist.items[0]?.id ?? null);
}

export function replaceSession(session: PlaylistSession): void {
  const playlists = normalizePlaylists(Array.isArray(session?.playlists) ? session.playlists : []);
  playlistStore.set(playlists);

  const savedActiveId = typeof session?.activePlaylistId === "string" ? session.activePlaylistId : "";
  const active = playlists.find((playlist) => playlist.id === savedActiveId) ?? playlists[0];
  activePlaylistId.set(active.id);
  currentItemId.set(active.items[0]?.id ?? null);
}

export function getSession(): PlaylistSession {
  return {
    playlists: get(playlistStore),
    activePlaylistId: get(activePlaylistId),
  };
}

export function addPaths(paths: string[]): PlaylistItem[] {
  const added: PlaylistItem[] = [];
  const activeId = get(activePlaylistId);

  playlistStore.update((playlists) => playlists.map((playlist) => {
    if (playlist.id !== activeId || playlist.items.length >= MAX_PLAYLIST_ITEMS) return playlist;

    const existing = new Set(playlist.items.map((item) => item.path));
    const next = [...playlist.items];

    for (const rawPath of paths) {
      const path = rawPath.trim();
      if (!path || existing.has(path) || next.length >= MAX_PLAYLIST_ITEMS) continue;

      const item = { id: createId(path), path, name: fileName(path) };
      existing.add(path);
      added.push(item);
      next.push(item);
    }

    return { ...playlist, items: next };
  }));

  return added;
}

export function removeItem(id: string): PlaylistItem | undefined {
  let removed: PlaylistItem | undefined;
  const activeId = get(activePlaylistId);

  playlistStore.update((playlists) => playlists.map((playlist) => {
    if (playlist.id !== activeId) return playlist;
    removed = playlist.items.find((item) => item.id === id);
    return { ...playlist, items: playlist.items.filter((item) => item.id !== id) };
  }));

  currentItemId.update((current) => (current === id ? null : current));
  return removed;
}

export function clearPlaylist(): void {
  const activeId = get(activePlaylistId);
  playlistStore.update((playlists) => playlists.map((playlist) => playlist.id === activeId ? { ...playlist, items: [] } : playlist));
  currentItemId.set(null);
}

export function moveItem(id: string, direction: -1 | 1): void {
  const activeId = get(activePlaylistId);
  playlistStore.update((playlists) => playlists.map((playlist) => {
    if (playlist.id !== activeId) return playlist;
    const index = playlist.items.findIndex((item) => item.id === id);
    const target = index + direction;
    if (index < 0 || target < 0 || target >= playlist.items.length) return playlist;

    const next = [...playlist.items];
    [next[index], next[target]] = [next[target], next[index]];
    return { ...playlist, items: next };
  }));
}

export function setCurrentItem(id: string | null): void {
  currentItemId.set(id);
}
