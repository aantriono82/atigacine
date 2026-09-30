# Changelog

All notable changes to Atiga Cine are documented here. Release sections use the
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.

## [Unreleased]

No changes yet.

## [0.1.7] - 2026-09-30

### Fixed

- Native libmpv video rendering now follows the responsive TV viewport through a
  dedicated X11 container, keeping the video centered during resize and scaling.

### Changed

- Playlist, recent-file, and playback-position data are isolated by application
  version so a new release starts without media references from an older release.
- Release notes are now read from this changelog when a version tag is published.

## [0.1.6] - 2026-09-27

### Changed

- Improved native video scaling for responsive TV layouts.
- Configured libmpv to preserve the video aspect ratio while fitting the native
  viewport with centered letterboxing.

[Unreleased]: https://github.com/aantriono82/atigacine/compare/v0.1.7...HEAD
[0.1.7]: https://github.com/aantriono82/atigacine/releases/tag/v0.1.7
[0.1.6]: https://github.com/aantriono82/atigacine/releases/tag/v0.1.6
