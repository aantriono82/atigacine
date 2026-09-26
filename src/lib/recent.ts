import { fileName } from "./media";

export interface RecentFile {
  path: string;
  name: string;
  lastOpened: number;
}

export const MAX_RECENT_FILES = 20;
const STORAGE_KEY = "atiga-cine-recent-files";

export function loadRecentFiles(): RecentFile[] {
  if (typeof localStorage === "undefined") return [];

  try {
    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]") as unknown;
    if (!Array.isArray(stored)) return [];

    return stored
      .filter((entry): entry is Partial<RecentFile> => Boolean(entry && typeof entry === "object"))
      .map((entry) => ({
        path: typeof entry.path === "string" ? entry.path.trim() : "",
        name: typeof entry.name === "string" ? entry.name.trim() : "",
        lastOpened: typeof entry.lastOpened === "number" ? entry.lastOpened : 0,
      }))
      .filter((entry) => entry.path && entry.lastOpened > 0)
      .sort((a, b) => b.lastOpened - a.lastOpened)
      .slice(0, MAX_RECENT_FILES);
  } catch {
    return [];
  }
}

function saveRecentFiles(files: RecentFile[]): void {
  if (typeof localStorage === "undefined") return;

  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(files.slice(0, MAX_RECENT_FILES)));
  } catch {
    // Recent-file history is optional and must never interrupt importing.
  }
}

export function recordRecentFiles(paths: readonly string[]): RecentFile[] {
  const now = Date.now();
  const existing = loadRecentFiles();
  const byPath = new Map(existing.map((entry) => [entry.path, entry]));

  paths.forEach((rawPath, index) => {
    const path = rawPath.trim();
    if (!path) return;
    byPath.set(path, {
      path,
      name: fileName(path),
      lastOpened: now + index,
    });
  });

  const next = [...byPath.values()].sort((a, b) => b.lastOpened - a.lastOpened).slice(0, MAX_RECENT_FILES);
  saveRecentFiles(next);
  return next;
}

export function createRecentItemId(path: string): string {
  return `recent:${path}`;
}
