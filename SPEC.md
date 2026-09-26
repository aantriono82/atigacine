# Atiga Cine — Specification

## Peran dan tujuan

Bertindak sebagai senior software engineer yang andal, dengan spesialisasi pengembangan aplikasi desktop Linux menggunakan Tauri + Rust, frontend modern dengan Svelte, serta integrasi library native seperti libmpv. Aplikasi harus dibangun dengan constraint performa ketat untuk perangkat low-spec.

Tugasnya adalah membuat scaffold aplikasi video player desktop Linux bernama **Atiga Cine** dengan tampilan klasik, terinspirasi Media Player Classic / Winamp era.

## Stack teknologi

- Shell: Tauri v2 + Rust
- Engine playback: libmpv melalui crate `tauri-plugin-libmpv` (https://github.com/nini22P/tauri-plugin-libmpv)
- Jangan menggunakan tag `<video>` HTML5 bawaan WebView; playback harus menggunakan native libmpv.
- Frontend: Svelte + Vite + TypeScript
- Package manager: pnpm

## Target hardware dan performa

Aplikasi harus tetap responsif pada perangkat kelas Celeron N3050 / RAM 2GB / Linux, termasuk Artix/Xorg/Fluxbox atau distro ringan sejenis.

- Playlist harus divirtualisasi dan hanya merender item yang terlihat di viewport.
- Import puluhan atau ratusan file harus asinkron, tidak memblokir main thread, dan menampilkan progress indicator.
- Pergantian skin/tema harus instan, target kurang dari 100ms, melalui CSS custom properties pada `:root`; jangan melakukan remount komponen atau rerender seluruh tree.
- Equalizer/audio filter harus memakai property `af` bawaan libmpv, misalnya `lavfi=[equalizer=...]`.
- Jangan mengimplementasikan DSP custom di frontend/JavaScript.

## Desain UI dan skin

- Estetika desktop klasik: custom title bar, menu bar bergaya native, toolbar dengan tombol play/pause/stop/previous/next, seekbar dengan timestamp, dan volume slider bergaya lama.
- Sediakan minimal dua skin berbasis CSS variables:
  1. `charcoal-amber`, konsisten dengan skema warna Atiga Amp.
  2. Skin alternatif klasik abu-abu/silver ala Windows 98/XP.
- Skin baru harus dapat ditambahkan cukup dengan file CSS variables baru tanpa mengubah komponen Svelte.
- Panel equalizer bergaya klasik dengan slider vertikal per band, dapat di-toggle show/hide, dan saat disembunyikan tidak mengganggu layout utama.

## Fitur inti MVP

1. Buka file video melalui dialog atau drag-and-drop.
2. Kontrol playback: play/pause, stop, seek klik/drag, volume, dan mute.
3. Playlist: tambah, hapus, reorder, dan simpan sesi terakhir.
4. Subtitle eksternal `.srt`/`.ass`, melalui load manual atau auto-detect nama file yang sama.
5. Fullscreen toggle.
6. Keyboard shortcuts: spasi play/pause, panah kiri/kanan seek, panah atas/bawah volume, `F` fullscreen.
7. Informasi format: nama file, durasi, dan resolusi di title bar/status bar.
8. Equalizer audio minimal 5–10 band: bass, low-mid, mid, high-mid, treble; preset flat, bass boost, vocal boost; dan custom preset per sesi. Semua dikontrol melalui property `af` libmpv dari UI Svelte melalui Tauri command.

## Arsitektur

- Komunikasi Svelte dan libmpv melalui Tauri commands dan event listener.
- Event posisi playback dan status buffering harus di-throttle, jangan emit setiap frame.
- State management menggunakan store sederhana Svelte writable stores; jangan memakai library state management berat.
- Struktur folder Rust harus memisahkan modul player, wrapper libmpv termasuk kontrol audio filter, playlist, dan commands.
- Struktur folder Svelte:

  - `components/`: `Player`, `Playlist`, `Toolbar`, `SeekBar`, `VolumeControl`, `Equalizer`
  - `stores/`: `playerStore`, `playlistStore`, `themeStore`, `equalizerStore`
  - `styles/skins/`: satu file CSS variables per skin

## Non-goals

- Tidak perlu streaming online atau YouTube.
- Tidak perlu plugin atau scripting system.
- Tidak perlu multi-window.

## Urutan pengerjaan

1. Setup project Tauri + Svelte + Vite.
2. Integrasi `tauri-plugin-libmpv` dengan test play video sederhana tanpa UI custom.
3. Bangun UI klasik dasar untuk playback dan playlist secara bertahap.
4. Tambahkan equalizer setelah playback inti stabil.

## Quality gate

Setiap perubahan harus divalidasi dengan lint dan unit test sebelum melanjutkan ke tahap berikutnya. Gunakan `pnpm verify` sebagai gate project.
