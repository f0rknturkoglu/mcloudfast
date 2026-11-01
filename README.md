# mcloudfast

<p align="center">
  <img src="docs/assets/images/logo.svg" alt="mcloudfast logo" width="88" height="88">
</p>

<p align="center"><strong>Apple Music interface on a fast, native Spotify client.</strong><br>A lightweight desktop music app for Linux, macOS, and Windows.</p>

<p align="center">
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/Built_with-Rust-1ed760" alt="Built with Rust"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-1ed760" alt="MIT license"></a>
</p>

**mcloudfast** brings the full Apple Music experience from the [mcloud](https://github.com/bulutakbulutt/mcloud) theme to a high-performance native desktop client built in Rust with [egui](https://www.egui.rs) and [fastframe](https://fastframe.dev). With zero browser engine overhead, instant response times, and native Liquid Glass styling, mcloudfast combines the refined aesthetics of Apple Music with the speed of Spotifast.

Choose the style anytime under **Settings → Appearance → Interface style → mcloud**.

---

## Highlights

- **Liquid Glass Navigation & Player:** Floating, hardware-accelerated frosted glass player, topbar composer, and sidebar ambient glow.
- **Apple Music Pages:** Dedicated layouts for Albums (with copyright, track counts, release details, and "More by [Artist]" shelves), Playlists, Liked Songs (with genre chips), and Artists.
- **Top Header Experience:** A unified liquid glass header with Home button, 36px navigation controls, and search composer with browse drawer. The header seamlessly blurs on scroll and becomes clear when scrolled to the top.
- **Dual Playback Engines:** Built-in librespot player or Spotify's official Soloist player with equalizer and high-fidelity output.
- **Fast and Private:** Written entirely in native Rust. No Electron, no telemetry, no bundled Chromium.
- **Complete Feature Set:** Full library management, Spotify Connect, lyrics, queue management, Winamp mini player, and MilkDrop visualizers.

---

## Features

| Feature | Description |
|---|---|
| **Apple Music Design** | Red-accented sidebar, ambient edge glow, floating liquid glass player bar, and clean flat headers. |
| **Library and Browse** | Category filter chips (All, Music, Podcasts), Recently Added, All Playlists, and category tiles. |
| **Search Composer** | Integrated search pill with browse drawer shortcut, instant autocomplete, and keyboard navigation. |
| **Soloist & librespot** | Listen via Spotify's official player engine or the integrated librespot player. |
| **Lyrics & Visuals** | Synced lyrics support, plus Winamp classic skin player and MilkDrop 3D visualizers. |
| **Local Files** | Play local audio tracks directly alongside your Spotify playlists. |

---

## Building from source

Prerequisites:
- [Rust](https://rustup.rs/) (1.80 or newer recommended)
- Standard system build tools and graphics drivers with OpenGL 2.0+ support

Clone the repository and build:

```sh
git clone https://github.com/f0rknturkoglu/mcloudfast.git
cd mcloudfast
cargo run --release
```

To run without a Spotify account in demo mode:

```sh
cargo run --features demo -- --demo
```

---

## Getting Started

1. Create a free Spotify developer application in the [Spotify Developer Dashboard](https://developer.spotify.com/dashboard).
   - Set the Redirect URI to `http://127.0.0.1:8989/login`.
   - Ensure the **Web API** option is enabled.
2. Enter your Client ID on the sign-in screen and approve authorization in your default browser.
3. Open the device menu on the player bar or navigate to **Settings** to choose your playback device (**This computer** or another Spotify Connect speaker).
4. Enjoy your music with the full Apple Music interface.

---

## Credits & License

mcloudfast is based on [Spotifast](https://github.com/crmne/spotifast) by Carmine Paolino and contributors, and inspired by the [mcloud](https://github.com/bulutakbulutt/mcloud) Spicetify theme by Bulut Akbulut.

Licensed under the [MIT License](LICENSE). Spotifast and mcloudfast are independent projects not affiliated with Spotify AB.
