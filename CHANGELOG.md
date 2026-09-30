# Changelog

All notable changes to Atiga Cine are documented here. Release sections use the
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.

## [Unreleased]

## [0.1.8] - 2026-09-30

### Fixed

- Native libmpv playback no longer reparents its X11 child window after
  initialization, preventing Linux playback from remaining at `LOADING` with
  no audio or video.
- AppImage builds no longer stall during linuxdeploy's recursive GTK dependency
  scan.

## [0.1.7] - 2026-09-30

### Fixed

- Native libmpv video rendering follows the responsive TV viewport during
  resize and scaling.

### Changed

- Playlist, recent-file, and playback-position data are isolated by application
  version so a new release starts without media references from an older release.
- Release notes are now read from this changelog when a version tag is published.

## [0.1.6] - 2026-09-27

### Changed

- Improved native video scaling for responsive TV layouts.
- Configured libmpv to preserve the video aspect ratio while fitting the native
  viewport with centered letterboxing.

[Unreleased]: https://github.com/aantriono82/atigacine/compare/v0.1.8...HEAD
[0.1.8]: https://github.com/aantriono82/atigacine/releases/tag/v0.1.8
[0.1.7]: https://github.com/aantriono82/atigacine/releases/tag/v0.1.7
[0.1.6]: https://github.com/aantriono82/atigacine/releases/tag/v0.1.6
