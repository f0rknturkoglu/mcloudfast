# mcloudfast

<p align="center">
  <img src="docs/assets/images/logo.svg" alt="mcloudfast logo" width="96" height="96">
</p>

<p align="center">
  <strong>Native Apple Music interface on an ultra-fast Spotify client.</strong><br>
  Built with native Rust, egui, and fastframe for Linux, macOS, and Windows.
</p>

<p align="center">
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/Built_with-Rust-fa2d48" alt="Built with Rust"></a>
  <a href="https://github.com/f0rknturkoglu/mcloudfast"><img src="https://img.shields.io/badge/Interface-Apple_Music-fa2d48" alt="Apple Music Interface"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue" alt="MIT license"></a>
</p>

---

## Overview

**mcloudfast** is a native Spotify desktop client that ports the complete visual and interactive design of Apple Music from the popular [mcloud](https://github.com/bulutakbulutt/mcloud) theme to high-performance native Rust.

Unlike web-based Spicetify extensions or Chromium wrappers, mcloudfast runs directly on the GPU using [egui](https://github.com/emilk/egui) and [fastframe](https://fastframe.dev). It delivers instantaneous 120 FPS navigation, native Liquid Glass post-processing shaders, and minimal resource usage (under 230 MB RAM and sub-millisecond GPU draw calls).

---

## Key Features

- **Liquid Glass Architecture:**
  - Hardware-accelerated Gaussian blur halo passes with tone mapping and refraction rim curvature.
  - Floating player capsule with animated marquee, volume scrubber, and playback controls.
  - Top header band with responsive navigation, Home button, and search composer that smoothly blurs on page scroll.
  - Interactive two-state browse drawer icon with outline idle state and filled active state.

- **Apple Music Pages & Navigation:**
  - **Sidebar:** Ambient edge glow, red accented navigation, collapsible Library and Playlists folders.
  - **Albums:** Full Apple Music layout with high-res cover art, release date, track count, copyright metadata, and "More by [Artist]" shelves.
  - **Home & Browse:** Instant category chips (All, Music, Podcasts) with sticky headers.
  - **Artists & Playlists:** Hero play buttons, disc separators, and inline curation.

- **Dual Playback Engines:**
  - **librespot:** Built-in native playback without external dependencies.
  - **Soloist:** Optional Spotify desktop engine integration for lossless streams and official Connect controls.

- **Built-in Extras:**
  - Synced lyrics with automatic vocal sync.
  - 10-band graphic equalizer with custom presets and pre-amp control.
  - Classic Winamp mini-player mode with skin support (`.wsz`).
  - MilkDrop 3D music visualizer powered by projectM.

---

## Performance

mcloudfast is designed to be lightweight, instant, and battery-friendly:

| Metric | mcloudfast | Web / Electron Clients |
| :--- | :---: | :---: |
| **Startup Time** | **< 300 ms** | ~3000 ms - 5000 ms |
| **Memory Footprint (RSS)** | **~220 MB** | 800 MB - 1.5 GB |
| **GPU Shader Time** | **~0.9 ms** (120 FPS smooth) | Variable browser compositor |
| **Engine Overhead** | **0% (Pure Native Rust)** | Chromium / V8 Runtime |

---

## Building from Source

### Prerequisites
- [Rust](https://rustup.rs/) (1.80 or newer)
- C/C++ compiler toolchain (clang / gcc / MSVC)
- CMake (for MilkDrop visualizer dependencies)

### Build and Run

```sh
# Clone the repository
git clone https://github.com/f0rknturkoglu/mcloudfast.git
cd mcloudfast

# Run in release mode
cargo run --release
```

To run demo mode without a Spotify login:

```sh
cargo run --features demo -- --demo --demo-show mcloud
```

---

## Getting Started

1. Create a free developer application in the [Spotify Developer Dashboard](https://developer.spotify.com/dashboard).
2. Add `http://127.0.0.1:8989/login` as an authorized **Redirect URI**.
3. Launch mcloudfast, enter your Client ID, and authorize in your default browser.
4. Select your playback output under the player bar device menu or in Settings.

---

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Space` | Play / Pause |
| `⌘ + →` / `Ctrl + →` | Next track |
| `⌘ + ←` / `Ctrl + ←` | Previous track |
| `⌘ + L` / `Ctrl + L` | Focus Search Composer |
| `⌘ + [` / `⌘ + ]` | Navigate Back / Forward |
| `⌘ + S` / `Ctrl + S` | Toggle Shuffle |
| `⌘ + R` / `Ctrl + R` | Cycle Repeat Mode |

---

## Credits & Acknowledgements

- Based on [Spotifast](https://github.com/crmne/spotifast) by Carmine Paolino and contributors.
- Inspired by [mcloud](https://github.com/bulutakbulutt/mcloud) Spicetify theme by Bulut Akbulut.
- Powered by [egui](https://github.com/emilk/egui), [librespot](https://github.com/librespot-org/librespot), and [fastframe](https://fastframe.dev).

---

## License

Licensed under the [MIT License](LICENSE). mcloudfast is an independent open-source project not affiliated with Spotify AB or Apple Inc.
