import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import type {
  DownloadOptions,
  DownloadProgress,
  JobInfo,
  VideoInfo,
} from "./types";

export function startDownload(
  url: string,
  options?: DownloadOptions,
): Promise<number> {
  return invoke("start_download", { url, options: options ?? null });
}

export function cancelDownload(id: number): Promise<void> {
  return invoke("cancel_download", { id });
}

export function listJobs(): Promise<JobInfo[]> {
  return invoke("list_jobs");
}

export function clearFinished(): Promise<void> {
  return invoke("clear_finished");
}

export function fetchInfo(url: string): Promise<VideoInfo> {
  return invoke("fetch_info", { url });
}

export interface YtdlpVersionInfo {
  current: string;
  latest: string | null;
  updateAvailable: boolean;
}

export function ytdlpVersion(): Promise<YtdlpVersionInfo> {
  return invoke("ytdlp_version");
}

export function updateYtdlpNow(): Promise<string> {
  return invoke("update_ytdlp_now");
}

/** Reveal a file/folder in Explorer (selects the item). */
export function revealPath(path: string): Promise<void> {
  return invoke("reveal_path", { path });
}

/** The user's Downloads folder path. */
export function defaultDownloadDir(): Promise<string> {
  return invoke("default_download_dir");
}

export function onProgress(
  cb: (p: DownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen<DownloadProgress>("download-progress", (e) => cb(e.payload));
}

// ---------------------------------------------------------------------------
// App self-update (Tauri updater plugin)
// ---------------------------------------------------------------------------

export type AppUpdate = Update;

/** The running app's version from tauri.conf.json. */
export function appVersion(): Promise<string> {
  return getVersion();
}

/** Ask the update feed (GitHub releases) whether a newer app version exists. */
export function checkAppUpdate(): Promise<AppUpdate | null> {
  return check();
}

/** Restart the app (used after an update has been installed). */
export function relaunchApp(): Promise<void> {
  return relaunch();
}

export function onStatus(
  cb: (p: DownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen<DownloadProgress>("download-status", (e) => cb(e.payload));
}
