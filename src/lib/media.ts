export const SUPPORTED_VIDEO_EXTENSIONS = [
  "3gp",
  "avi",
  "flac",
  "m4v",
  "mkv",
  "mov",
  "mp4",
  "mpeg",
  "mpg",
  "ogg",
  "ogv",
  "ts",
  "webm",
  "wmv",
] as const;

export const SUPPORTED_SUBTITLE_EXTENSIONS = ["ass", "srt", "ssa", "sub", "sup", "vtt"] as const;

function extensionOf(path: string): string {
  const fileName = path.split(/[\\/]/).pop() ?? path;
  const extensionStart = fileName.lastIndexOf(".");
  return extensionStart < 0 ? "" : fileName.slice(extensionStart + 1).toLowerCase();
}

export function isSupportedVideoPath(path: string): boolean {
  return SUPPORTED_VIDEO_EXTENSIONS.includes(extensionOf(path) as (typeof SUPPORTED_VIDEO_EXTENSIONS)[number]);
}

export function isSupportedSubtitlePath(path: string): boolean {
  return SUPPORTED_SUBTITLE_EXTENSIONS.includes(extensionOf(path) as (typeof SUPPORTED_SUBTITLE_EXTENSIONS)[number]);
}

export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export function filterSupportedVideoPaths(paths: readonly string[]): string[] {
  return paths.filter((path) => path.trim().length > 0 && isSupportedVideoPath(path));
}
