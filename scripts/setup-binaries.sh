#!/usr/bin/env bash
# Downloads the bundled yt-dlp.exe and ffmpeg/ffprobe into src-tauri/binaries/.
# Run this once before `pnpm tauri build` (binaries are gitignored).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
bin="$root/src-tauri/binaries"
mkdir -p "$bin/ffmpeg/bin"

echo "Downloading yt-dlp.exe ..."
curl -fL -o "$bin/yt-dlp.exe" \
  "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"

echo "Downloading ffmpeg (essentials build) ..."
zip="$(mktemp -u /tmp/ffmpeg-XXXX.zip)"
tmp="$(mktemp -d /tmp/ffmpeg-XXXX)"
curl -fL -o "$zip" "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"
unzip -q "$zip" -d "$tmp"
find "$tmp" -name "ffmpeg.exe" -exec cp {} "$bin/ffmpeg/bin/ffmpeg.exe" \;
find "$tmp" -name "ffprobe.exe" -exec cp {} "$bin/ffmpeg/bin/ffprobe.exe" \;
rm -rf "$zip" "$tmp"

echo "Done. Bundled binaries are in $bin"
