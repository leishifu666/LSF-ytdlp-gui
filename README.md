# yt-dlp GUI

A clean, fast desktop GUI for [yt-dlp](https://github.com/yt-dlp/yt-dlp), built with **Tauri 2 + Vue 3 + TypeScript**.

> ⚠️ Early work-in-progress. Core downloading works; more features landing soon.

## Features

- 📥 Paste a URL → pick quality → download, with live progress (speed / ETA / percentage)
- 🎚️ Quality presets: best / 1080p / 720p / 480p / audio-only MP3
- 🧰 **Raw args passthrough** — every yt-dlp CLI flag is reachable from the Advanced panel (shell-quoted, appended verbatim)
- 🍪 Cookies support for members-only videos (paste `cookies.txt` content)
- 👁️ URL analysis: title, uploader, duration, thumbnail before downloading
- 📃 Job queue: concurrent downloads, cancel, clear finished
- 🔄 **In-app yt-dlp updater** — one click pulls the latest `yt-dlp.exe` from GitHub releases (yt-dlp moves fast; this matters)
- 🌐 i18n: 简体中文 / English, persisted per launch
- 📦 **Zero-config install**: ships with `yt-dlp.exe` and `ffmpeg` bundled — install and it just works

## Tech stack

| Layer    | Choice                                        |
| -------- | --------------------------------------------- |
| Shell    | Tauri 2 (Rust) — thin backend, WebView2 UI    |
| Frontend | Vue 3 + TypeScript + Vite                     |
| Engine   | yt-dlp subprocess, machine-readable `--print` / `--progress-template` output (formats verified against real runs) |
| Media    | ffmpeg (bundled) for muxing / audio extraction |

The Rust side does only what Rust is great at here: spawning processes, streaming/parsing stdout, managing the job table, and self-updating. All UI lives in Vue single-file components.

## Development

Prerequisites: [Node.js](https://nodejs.org) ≥ 20, [pnpm](https://pnpm.io), [Rust](https://rustup.rs) (stable, MSVC toolchain on Windows).

```bash
pnpm install
# put yt-dlp.exe and ffmpeg in src-tauri/binaries (see structure below)
pnpm tauri dev
```

### Binary layout

The app resolves `yt-dlp.exe` in this order: app-data override (written by the in-app updater) → bundled copy → `PATH`.

```
src-tauri/binaries/
├── yt-dlp.exe              # from https://github.com/yt-dlp/yt-dlp/releases
└── ffmpeg/bin/
    ├── ffmpeg.exe          # from https://www.gyan.dev/ffmpeg/builds/
    └── ffprobe.exe
```

These are `.gitignore`d; fetch them before first build. In dev, if the bundled copy is missing, a system-installed `yt-dlp` on `PATH` also works.

### Build installer

```bash
pnpm tauri build
```

Produces an NSIS installer under `src-tauri/target/release/bundle/nsis/`.

## Roadmap

- [ ] Playlist picker (select which entries to download)
- [ ] Subtitle selection UI
- [ ] Download history (SQLite)
- [ ] Clipboard watch / batch paste
- [ ] App self-update via Tauri updater
- [ ] macOS / Linux builds

## License

MIT
