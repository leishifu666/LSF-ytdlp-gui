import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
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

export function onProgress(
  cb: (p: DownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen<DownloadProgress>("download-progress", (e) => cb(e.payload));
}

export function onStatus(
  cb: (p: DownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen<DownloadProgress>("download-status", (e) => cb(e.payload));
}
