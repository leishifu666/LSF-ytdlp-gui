# LSF-ytdlp-gui

[English](README.en.md) | 简体中文

一个简洁、快速的 [yt-dlp](https://github.com/yt-dlp/yt-dlp) 桌面图形界面，基于 **Tauri 2 + Vue 3 + TypeScript** 构建。

> ⚠️ 早期开发中。核心下载功能可用，更多功能持续添加。

## 功能

- 📥 粘贴链接 → 选画质 → 下载，实时进度（速度 / 剩余时间 / 百分比）
- 🎚️ 画质预设：最佳 / 1080p / 720p / 480p / 仅音频 MP3
- 🧰 **原始参数透传** — yt-dlp 的所有 CLI 参数都可以在高级面板中使用（shell 引号解析，原样追加）
- 🍪 Cookies 支持，可下载会员/登录专属视频（粘贴 `cookies.txt` 内容）
- 👁️ 链接解析：下载前展示标题、作者、时长、封面
- 📃 任务队列：并发下载、取消、清除已完成
- 🔄 **应用内 yt-dlp 更新器** — 一键从 GitHub Releases 拉取最新 `yt-dlp.exe`（yt-dlp 迭代很快，这很重要）
- 🌐 界面语言：简体中文 / English，按次启动记忆
- 📦 **零配置安装**：自带 `yt-dlp.exe` 和 `ffmpeg` — 装完即用

## 支持的站点与平台

**站点：** 所有 [yt-dlp 支持的站点](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md) — 数千个，包括 YouTube、哔哩哔哩、TikTok、Twitter/X、Instagram、抖音等。其中抖音需要额外配置，见下文专门说明。

**桌面平台：**

| 平台 | 状态 |
| ---- | ---- |
| Windows 10 / 11 | ✅ 支持（NSIS 安装包） |
| macOS / Linux | 🚧 计划中（开发版可用，打包未配置） |

## 下载抖音视频

抖音的 Web API 要求请求携带**浏览器 cookies** — 没有 cookies 时 yt-dlp 会报
`Fresh cookies (not necessarily logged in) are needed`。**不需要登录**抖音账号，只需要从浏览器导出一套新鲜的 cookies。

最简单的导出工具是浏览器扩展 [Cookie-Editor](https://chromewebstore.google.com/detail/cookie-editor/hlkenndednhfkekhgcdicdfddnkalmdm)（支持 Chrome / Edge / Firefox）：

1. 在浏览器打开 [douyin.com](https://www.douyin.com)，随便播放一个视频，让网站写入新鲜 cookies。登录与否均可。
2. 在 douyin.com 标签页上，点击工具栏里的 Cookie-Editor 图标。
3. 点击 **Export As（导出）→ Netscape** — 会把 `cookies.txt` 格式的内容复制到剪贴板。
4. 在 LSF-ytdlp-gui 中，把内容粘贴进 **Cookies** 输入框。
5. 粘贴视频链接（`https://www.douyin.com/video/<id>`），开始下载。

注意事项：

- cookies 会过期。如果突然下载失败，重新访问 douyin.com 播放一个视频，再导出一次即可。
- 请使用**单视频链接**（`douyin.com/video/<id>`）。yt-dlp 不支持 `douyin.com/user/self?...&modal_id=...` 这类主页链接 — 如果只拿到这种链接，从中复制 `modal_id` 的数字，改用 `douyin.com/video/<modal_id>`。
- 导出的 cookies 可能包含你的登录凭证，不要分享给他人。

## 下载安装

从 [Releases](https://github.com/leishifu666/LSF-ytdlp-gui/releases) 下载最新的 `*_x64-setup.exe`，双击安装即可。安装包自带 `yt-dlp.exe` 和 `ffmpeg`，无需任何额外配置。

## 开发

环境要求：[Node.js](https://nodejs.org) ≥ 20、[pnpm](https://pnpm.io)、[Rust](https://rustup.rs)（stable，Windows 上需 MSVC 工具链）。

```bash
pnpm install
# 把 yt-dlp.exe 和 ffmpeg 放入 src-tauri/binaries（结构见下）
pnpm tauri dev
```

### 二进制文件布局

应用按以下顺序解析 `yt-dlp.exe`：app-data 覆盖（应用内更新器写入）→ 捆绑副本 → `PATH`。

```
src-tauri/binaries/
├── yt-dlp.exe              # 来自 https://github.com/yt-dlp/yt-dlp/releases
└── ffmpeg/bin/
    ├── ffmpeg.exe          # 来自 https://www.gyan.dev/ffmpeg/builds/
    └── ffprobe.exe
```

这些文件已被 `.gitignore` 忽略；首次构建前请自行下载。开发时如果捆绑副本缺失，`PATH` 上系统安装的 `yt-dlp` 也可以。

### 构建安装包

```bash
pnpm tauri build
```

产物为 NSIS 安装包，位于 `src-tauri/target/release/bundle/nsis/`。

## 技术栈

| 层       | 选择                                          |
| -------- | --------------------------------------------- |
| 外壳     | Tauri 2 (Rust) — 轻后端，WebView2 UI          |
| 前端     | Vue 3 + TypeScript + Vite                     |
| 下载引擎 | yt-dlp 子进程，机器可读的 `--print` / `--progress-template` 输出（格式经真实运行验证） |
| 媒体处理 | ffmpeg（捆绑）负责混流 / 音频提取             |

Rust 侧只做 Rust 擅长的事：拉起进程、流式解析 stdout、管理任务表、自更新。所有 UI 都是 Vue 单文件组件。

## 路线图

- [ ] 播放列表选择器（勾选要下载的条目）
- [ ] 字幕选择 UI
- [ ] 下载历史（SQLite）
- [ ] 剪贴板监听 / 批量粘贴
- [ ] 应用自更新（Tauri updater）
- [ ] macOS / Linux 构建

## 许可证

[MIT](LICENSE)
