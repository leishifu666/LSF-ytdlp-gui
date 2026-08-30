import { ref } from "vue";

export type Locale = "zh" | "en";

const saved = (localStorage.getItem("lang") as Locale) || "zh";
export const locale = ref<Locale>(saved === "en" ? "en" : "zh");

export function setLocale(l: Locale) {
  locale.value = l;
  localStorage.setItem("lang", l);
}

export const messages = {
  zh: {
    appName: "yt-dlp GUI",
    tabDownload: "下载",
    tabHistory: "任务",
    tabSettings: "设置",
    urlPlaceholder: "粘贴视频链接，例如 https://www.youtube.com/watch?v=...",
    addUrl: "添加到队列",
    analyzing: "解析中…",
    fetchInfoFailed: "解析失败",
    quality: "画质",
    qualityBest: "最佳画质",
    quality1080: "1080p",
    quality720: "720p",
    quality480: "480p",
    qualityAudio: "仅音频 (MP3)",
    outputDir: "保存位置",
    chooseDir: "浏览…",
    advanced: "高级选项",
    rawArgs: "原始 yt-dlp 参数（空格分隔，追加在命令行末尾）",
    rawArgsPlaceholder: "例如: --write-thumbnail --embed-metadata",
    cookies: "Cookies（可选，粘贴 cookies.txt 内容以下载会员视频）",
    cookiesPlaceholder: "# Netscape HTTP Cookie File …",
    download: "开始下载",
    queue: "下载队列",
    noJobs: "暂无任务",
    clearFinished: "清除已完成",
    status: {
      queued: "排队中",
      resolving: "解析中",
      downloading: "下载中",
      finished: "已完成",
      error: "失败",
      cancelled: "已取消",
    },
    settings: "设置",
    appVersion: "应用版本",
    restartNow: "重启应用",
    ytdlpVersion: "yt-dlp 内核版本",
    checkUpdate: "检查更新",
    checking: "检查中…",
    upToDate: "已是最新版本",
    updateAvailable: "发现新版本",
    updateNow: "立即更新",
    updating: "下载更新中…",
    updateDone: "已更新到",
    updateFailed: "更新失败",
    language: "界面语言",
    openFolder: "打开文件夹",
    openLocation: "打开位置",
    cancel: "取消",
    retry: "重试",
    dismiss: "关闭",
    errorTitle: "错误",
    urlRequired: "请先输入视频链接",
    eta: "剩余",
    speed: "速度",
  },
  en: {
    appName: "yt-dlp GUI",
    tabDownload: "Download",
    tabHistory: "Jobs",
    tabSettings: "Settings",
    urlPlaceholder: "Paste a video URL, e.g. https://www.youtube.com/watch?v=...",
    addUrl: "Add to queue",
    analyzing: "Analyzing…",
    fetchInfoFailed: "Failed to analyze",
    quality: "Quality",
    qualityBest: "Best quality",
    quality1080: "1080p",
    quality720: "720p",
    quality480: "480p",
    qualityAudio: "Audio only (MP3)",
    outputDir: "Save location",
    chooseDir: "Browse…",
    advanced: "Advanced options",
    rawArgs: "Raw yt-dlp arguments (space-separated, appended to the command)",
    rawArgsPlaceholder: "e.g. --write-thumbnail --embed-metadata",
    cookies: "Cookies (optional, paste cookies.txt content for members-only videos)",
    cookiesPlaceholder: "# Netscape HTTP Cookie File …",
    download: "Download",
    queue: "Download queue",
    noJobs: "No jobs yet",
    clearFinished: "Clear finished",
    status: {
      queued: "Queued",
      resolving: "Resolving",
      downloading: "Downloading",
      finished: "Done",
      error: "Failed",
      cancelled: "Cancelled",
    },
    settings: "Settings",
    appVersion: "App version",
    restartNow: "Restart app",
    ytdlpVersion: "yt-dlp engine version",
    checkUpdate: "Check for updates",
    checking: "Checking…",
    upToDate: "Up to date",
    updateAvailable: "Update available",
    updateNow: "Update now",
    updating: "Downloading update…",
    updateDone: "Updated to",
    updateFailed: "Update failed",
    language: "Language",
    openFolder: "Open folder",
    openLocation: "Show in folder",
    cancel: "Cancel",
    retry: "Retry",
    dismiss: "Dismiss",
    errorTitle: "Error",
    urlRequired: "Enter a video URL first",
    eta: "ETA",
    speed: "Speed",
  },
} as const;

export type Messages = typeof messages.zh;
export type MessageKey = keyof Messages;

export function useI18n() {
  const tr = (key: MessageKey): string => {
    const table = messages[locale.value] as Record<string, unknown>;
    const val = table[key];
    return typeof val === "string" ? val : key;
  };
  const trStatus = (s: string): string => {
    const table = messages[locale.value] as Record<string, unknown>;
    const statuses = table["status"] as Record<string, string> | undefined;
    return statuses?.[s] ?? s;
  };
  return { locale, tr, trStatus, setLocale };
}
