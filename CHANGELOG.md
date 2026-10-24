# Changelog

All notable changes to Atiga Cine are documented here. Release sections use the
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.

## [Unreleased]

### Fixed

- The equalizer preset name is readable again. WebKitGTK drew the preset field
  as a white GTK-themed box with the skin's light text; the field and its arrow
  are now drawn by the skin.
- Seek-bar thumbnails show the hovered position instead of the current frame,
  and hovering no longer pauses or seeks the video. Thumbnails come from a
  hidden decoder that seeks to the nearest keyframe, so a preview can be
  slightly before the hovered position. The decoder stops 10 seconds after the
  pointer leaves the seek bar.

### Changed

- Playback uses far less CPU: WebKit's share dropped from about 147% to 58% on
  an i7-7600U. The clock, seek bar, speaker grille and equalizer bars now
  update together five times a second, without fades or height animations.

## [0.1.11] - 2026-10-24

### Changed

- The equalizer visualizer and the speaker grille lights now follow the real
  audio level. An `astats` meter after the equalizer measures per-channel RMS
  levels ten times a second; the bars show a stereo level history (left channel
  on the left, right on the right, newest at the center) instead of a
  procedural animation.
- During playback the TV screen keeps its rounded black rim and glass edge.
  The native video window is clipped to the screen's curved corners with the
  X11 Shape extension and inset inside the glass ring, instead of covering the
  whole screen as a sharp rectangle.

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

[Unreleased]: https://github.com/aantriono82/atigacine/compare/v0.1.11...HEAD
[0.1.11]: https://github.com/aantriono82/atigacine/releases/tag/v0.1.11
[0.1.10]: https://github.com/aantriono82/atigacine/releases/tag/v0.1.10
[0.1.9]: https://github.com/aantriono82/atigacine/releases/tag/v0.1.9
[0.1.8]: https://github.com/aantriono82/atigacine/releases/tag/v0.1.8
[0.1.7]: https://github.com/aantriono82/atigacine/releases/tag/v0.1.7
[0.1.6]: https://github.com/aantriono82/atigacine/releases/tag/v0.1.6
