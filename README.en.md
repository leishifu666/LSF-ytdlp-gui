# LSF-ytdlp-gui

English | [简体中文](README.md)

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

## Supported sites & platforms

**Sites:** anything [yt-dlp supports](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md) — thousands of them, including YouTube, Bilibili, TikTok, Twitter/X, Instagram, and Douyin. See the Douyin section below for the one site that needs extra setup.

**Desktop platforms:**

| Platform | Status |
| -------- | ------ |
| Windows 10 / 11 | ✅ Supported (NSIS installer) |
| macOS / Linux | 🚧 On the roadmap (dev builds work, packaging not yet set up) |

## Downloading Douyin (抖音) videos

Douyin requires **browser cookies** for its web API — without them yt-dlp fails with
`Fresh cookies (not necessarily logged in) are needed`. No login is required; you just
need a fresh set of cookies exported from your browser.

The easiest way to export them is the [Cookie-Editor](https://chromewebstore.google.com/detail/cookie-editor/hlkenndednhfkekhgcdicdfddnkalmdm)
extension (available for Chrome / Edge / Firefox):

1. Open [douyin.com](https://www.douyin.com) in your browser and play any video once, so the site sets fresh cookies. Logging in is optional.
2. Click the Cookie-Editor icon in the toolbar while on the douyin.com tab.
3. Click **Export As → Netscape** — this copies a `cookies.txt`-format blob to your clipboard.
4. In LSF-ytdlp-gui, paste it into the **Cookies** box.
5. Paste the video URL (`https://www.douyin.com/video/<id>`) and download.

Notes:

- Cookies go stale after a while. If downloads suddenly fail, revisit douyin.com, play a video, and re-export.
- Use **per-video links** (`douyin.com/video/<id>`). Profile-page URLs like `douyin.com/user/self?...&modal_id=...` are not supported by yt-dlp — if you only have one of those, copy the `modal_id` number from it and use `douyin.com/video/<modal_id>`.
- Exported cookies may contain your login session. Don't share them.

## Install

Grab the latest `*_x64-setup.exe` from [Releases](https://github.com/leishifu666/LSF-ytdlp-gui/releases) and run it. The installer bundles `yt-dlp.exe` and `ffmpeg` — zero extra setup.

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

## Tech stack

| Layer    | Choice                                        |
| -------- | --------------------------------------------- |
| Shell    | Tauri 2 (Rust) — thin backend, WebView2 UI    |
| Frontend | Vue 3 + TypeScript + Vite                     |
| Engine   | yt-dlp subprocess, machine-readable `--print` / `--progress-template` output (formats verified against real runs) |
| Media    | ffmpeg (bundled) for muxing / audio extraction |

The Rust side does only what Rust is great at here: spawning processes, streaming/parsing stdout, managing the job table, and self-updating. All UI lives in Vue single-file components.

## Roadmap

- [ ] Playlist picker (select which entries to download)
- [ ] Subtitle selection UI
- [ ] Download history (SQLite)
- [ ] Clipboard watch / batch paste
- [ ] App self-update via Tauri updater
- [ ] macOS / Linux builds

## License

[MIT](LICENSE)
