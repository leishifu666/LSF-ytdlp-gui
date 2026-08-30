# Downloads the bundled yt-dlp.exe and ffmpeg/ffprobe into src-tauri/binaries/.
# Run this once before `pnpm tauri build` (binaries are gitignored).
$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$bin = Join-Path $root "src-tauri/binaries"
New-Item -ItemType Directory -Force -Path (Join-Path $bin "ffmpeg/bin") | Out-Null

Write-Host "Downloading yt-dlp.exe ..."
Invoke-WebRequest `
  -Uri "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe" `
  -OutFile (Join-Path $bin "yt-dlp.exe")

Write-Host "Downloading ffmpeg (essentials build) ..."
$zip = Join-Path $env:TEMP "ffmpeg-essentials.zip"
Invoke-WebRequest `
  -Uri "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip" `
  -OutFile $zip

$tmp = Join-Path $env:TEMP "ffmpeg-essentials"
Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
Expand-Archive -Path $zip -DestinationPath $tmp -Force
Copy-Item (Get-ChildItem $tmp -Recurse -Filter "ffmpeg.exe" | Select-Object -First 1).FullName `
  (Join-Path $bin "ffmpeg/bin/ffmpeg.exe")
Copy-Item (Get-ChildItem $tmp -Recurse -Filter "ffprobe.exe" | Select-Object -First 1).FullName `
  (Join-Path $bin "ffmpeg/bin/ffprobe.exe")
Remove-Item -Recurse -Force $tmp
Remove-Item $zip

Write-Host "Done. Bundled binaries are in $bin"
