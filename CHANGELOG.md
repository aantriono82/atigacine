# Changelog

All notable changes to Atiga Cine are documented here. Release sections use the
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.

## [Unreleased]

### Changed

- The equalizer visualizer and the speaker grille lights now follow the real
  audio level. An `astats` meter after the equalizer measures per-channel RMS
  levels ten times a second; the bars show a stereo level history (left channel
  on the left, right on the right, newest at the center) instead of a
  procedural animation.

## [0.1.10] - 2026-10-24

### Fixed

- Fullscreen and the mini player now work. The window capability was missing
  the permissions to toggle fullscreen, open the mini player window, and hide
  and restore the main window, so both buttons did nothing.
- The mini player's video no longer covers its title bar and controls, so the
  mini player can be closed to return to the main window.

### Changed

- Release builds use GitHub Actions versions that run on Node.js 24.

## [0.1.9] - 2026-10-24

### Fixed

- Native video on Linux/X11 is no longer cropped with black bands on the left
  and bottom of the TV screen. libmpv now embeds into a dedicated native window
  that follows the TV viewport, so it renders at the viewport size instead of
  the full application window size.

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

[Unreleased]: https://github.com/aantriono82/atigacine/compare/v0.1.10...HEAD
[0.1.10]: https://github.com/aantriono82/atigacine/releases/tag/v0.1.10
[0.1.9]: https://github.com/aantriono82/atigacine/releases/tag/v0.1.9
[0.1.8]: https://github.com/aantriono82/atigacine/releases/tag/v0.1.8
[0.1.7]: https://github.com/aantriono82/atigacine/releases/tag/v0.1.7
[0.1.6]: https://github.com/aantriono82/atigacine/releases/tag/v0.1.6
