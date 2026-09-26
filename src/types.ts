export interface PlaylistItem {
  id: string;
  path: string;
  name: string;
}

export interface Playlist {
  id: string;
  name: string;
  items: PlaylistItem[];
}

export interface PlaylistSession {
  playlists: Playlist[];
  activePlaylistId: string | null;
}

export type PlaybackStatus = "idle" | "loading" | "buffering" | "playing" | "paused" | "stopped" | "error";

export type RepeatMode = "off" | "one" | "all";

export interface PlayerState {
  initialized: boolean;
  status: PlaybackStatus;
  buffering: boolean;
  bufferDuration: number;
  currentFile: string | null;
  format: string | null;
  videoCodec: string | null;
  audioCodec: string | null;
  position: number;
  duration: number;
  volume: number;
  muted: boolean;
  width: number | null;
  height: number | null;
  message: string;
}

export type ThemeName = "charcoal-amber" | "silver-classic" | "winamp-classic" | "retro-tv";
