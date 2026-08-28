// Shared types matching the Rust backend's serde output (camelCase).

export type JobStatus =
  | "queued"
  | "resolving"
  | "downloading"
  | "finished"
  | "error"
  | "cancelled";

export interface DownloadProgress {
  id: number;
  downloaded: number;
  total: number | null;
  speed: number | null;
  eta: number | null;
  status: JobStatus;
  message: string | null;
}

export interface JobInfo {
  id: number;
  url: string;
  title: string | null;
  status: JobStatus;
  filepath: string | null;
  error: string | null;
}

/** JobInfo plus live progress fields tracked in the UI layer. */
export interface JobView extends JobInfo {
  downloaded?: number;
  total?: number | null;
  speed?: number | null;
  eta?: number | null;
}

export interface DownloadOptions {
  format?: string;
  outputDir?: string;
  rawArgs?: string;
  cookies?: string;
}

export interface VideoFormat {
  format_id: string;
  ext: string;
  resolution?: string | null;
  height?: number | null;
  fps?: number | null;
  vcodec: string;
  acodec: string;
  filesize?: number | null;
  filesize_approx?: number | null;
  tbr?: number | null;
  format_note?: string | null;
  protocol?: string | null;
}

export interface VideoInfo {
  id: string;
  title: string;
  _type?: string;
  entries?: VideoInfo[];
  duration?: number | null;
  uploader?: string | null;
  extractor_key?: string | null;
  webpage_url?: string | null;
  thumbnail?: string | null;
  formats?: VideoFormat[];
}
