use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter, State};

/// Decode subprocess output to a Rust String. Normal case is UTF-8; if that
/// fails (yt-dlp fell back to the console codepage), try GBK before falling
/// back to lossy replacement — a mojibake path would point at a file that
/// doesn't exist, breaking "show in folder".
fn decode_output(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            #[cfg(windows)]
            if let Some(s) = codepage_to_utf8(bytes, 936) {
                return s;
            }
            String::from_utf8_lossy(bytes).into_owned()
        }
    }
}

/// Convert bytes in a Windows codepage (936 = GBK) to a UTF-8 String.
#[cfg(windows)]
fn codepage_to_utf8(bytes: &[u8], codepage: u32) -> Option<String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn MultiByteToWideChar(
            codepage: u32,
            flags: u32,
            src: *const u8,
            src_len: i32,
            dst: *mut u16,
            dst_len: i32,
        ) -> i32;
    }
    unsafe {
        let len = MultiByteToWideChar(
            codepage,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            std::ptr::null_mut(),
            0,
        );
        if len <= 0 {
            return None;
        }
        let mut wide = vec![0u16; len as usize];
        let written = MultiByteToWideChar(
            codepage,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            wide.as_mut_ptr(),
            len,
        );
        if written != len {
            return None;
        }
        // UTF-16 → UTF-8 via String::from_utf16 (lossless for valid wide chars)
        String::from_utf16(&wide).ok()
    }
}

// ---------------------------------------------------------------------------
// Data types shared with the frontend (serde-serializable)
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Resolving,
    Downloading,
    Finished,
    Error,
    Cancelled,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub id: u32,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed: Option<f64>,
    pub eta: Option<u64>,
    pub status: JobStatus,
    pub message: Option<String>,
    /// Set when the backend learns the video title / final file path, so the
    /// UI can update the job card without a full list_jobs round-trip.
    pub title: Option<String>,
    pub filepath: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JobInfo {
    pub id: u32,
    pub url: String,
    pub title: Option<String>,
    pub status: JobStatus,
    pub filepath: Option<String>,
    pub error: Option<String>,
}

pub struct JobState {
    pub info: JobInfo,
    pub child: Option<Child>,
}

/// Shared download-queue state. Cloned into worker threads.
#[derive(Clone, Default)]
pub struct JobManager {
    jobs: Arc<Mutex<HashMap<u32, Arc<Mutex<JobState>>>>>,
    next_id: Arc<Mutex<u32>>,
}

impl JobManager {
    fn alloc_id(&self) -> u32 {
        let mut n = self.next_id.lock().unwrap();
        *n += 1;
        *n
    }

    fn insert(&self, id: u32, info: JobInfo) {
        self.jobs
            .lock()
            .unwrap()
            .insert(id, Arc::new(Mutex::new(JobState { info, child: None })));
    }

    fn update_info(&self, id: u32, f: impl FnOnce(&mut JobInfo)) {
        let jobs = self.jobs.lock().unwrap();
        if let Some(js) = jobs.get(&id) {
            let mut state = js.lock().unwrap();
            f(&mut state.info);
        }
    }

    fn set_child(&self, id: u32, child: Child) {
        let jobs = self.jobs.lock().unwrap();
        if let Some(js) = jobs.get(&id) {
            js.lock().unwrap().child = Some(child);
        }
    }

    fn take_child(&self, id: u32) -> Option<Child> {
        let jobs = self.jobs.lock().unwrap();
        if let Some(js) = jobs.get(&id) {
            js.lock().unwrap().child.take()
        } else {
            None
        }
    }

    fn is_cancelled(&self, id: u32) -> bool {
        let jobs = self.jobs.lock().unwrap();
        jobs.get(&id)
            .map(|js| js.lock().unwrap().info.status == JobStatus::Cancelled)
            .unwrap_or(false)
    }
}

// ---------------------------------------------------------------------------
// Options coming from the UI
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct DownloadOptions {
    /// e.g. "bv*+ba/b" — the yt-dlp format selector string
    pub format: Option<String>,
    /// output directory; falls back to yt-dlp's default
    pub output_dir: Option<String>,
    /// extra raw yt-dlp CLI arguments appended verbatim (power-user escape hatch)
    pub raw_args: Option<String>,
    /// optional cookies.txt content, written to a temp file
    pub cookies: Option<String>,
}

impl DownloadOptions {
    fn to_args(&self) -> Result<Vec<String>, String> {
        let mut args: Vec<String> = vec![
            // machine-readable plumbing (line formats verified against 2026.07.04)
            "--no-simulate".into(),
            "--progress".into(),
            "--newline".into(),
            // The PyInstaller-packed exe ignores PYTHONUTF8 and prints the
            // console codepage (GBK on zh-CN) when piped; only this flag
            // reliably forces UTF-8 on stdout.
            "--encoding".into(),
            "utf-8".into(),
            "--print".into(),
            "before_dl:__ENTRY__|%(title)s".into(),
            "--print".into(),
            "after_move:__FILE__|%(filepath)s".into(),
            "--progress-template".into(),
            "download:__PROG__|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s".into(),
            "--no-mtime".into(),
        ];
        if let Some(f) = &self.format {
            if !f.trim().is_empty() {
                args.push("--format".into());
                args.push(f.trim().into());
            }
        }
        if let Some(dir) = &self.output_dir {
            if !dir.trim().is_empty() {
                args.push("--paths".into());
                args.push(dir.trim().into());
            }
        } else if let Some(home) = std::env::var("USERPROFILE").ok() {
            // No explicit location chosen → save to the user's Downloads folder.
            let downloads = PathBuf::from(home).join("Downloads");
            if downloads.is_dir() {
                args.push("--paths".into());
                args.push(downloads.to_string_lossy().into_owned());
            }
        }
        if let Some(cookies) = &self.cookies {
            if !cookies.trim().is_empty() {
                // Per-job file: jobs run concurrently and share nothing.
                static JOB_SEQ: std::sync::atomic::AtomicU32 =
                    std::sync::atomic::AtomicU32::new(0);
                let n = JOB_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let path = std::env::temp_dir()
                    .join(format!("ytdlp-gui-cookies-{n}.txt"));
                // BOM makes yt-dlp read the file as UTF-8 regardless of locale.
                let mut bytes = b"\xef\xbb\xbf".to_vec();
                bytes.extend_from_slice(cookies.trim().as_bytes());
                std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
                args.push("--cookies".into());
                args.push(path.to_string_lossy().into_owned());
            }
        }
        if let Some(raw) = &self.raw_args {
            let extra = shell_words::split(raw)
                .map_err(|e| format!("raw args parse error: {e}"))?;
            args.extend(extra);
        }
        Ok(args)
    }
}

// ---------------------------------------------------------------------------
// Binary resolution
// ---------------------------------------------------------------------------

const APP_DATA_DIR: &str = "com.ytdlpgui.app";

/// Where a user-updated yt-dlp.exe lives (written by update_ytdlp_now).
fn app_data_dir() -> Option<PathBuf> {
    std::env::var("APPDATA").ok().map(|h| PathBuf::from(h).join(APP_DATA_DIR))
}

/// Find yt-dlp.exe: app-data override → bundled copy → dev tree → PATH.
pub fn resolve_binary() -> PathBuf {
    if let Some(dir) = app_data_dir() {
        let p = dir.join("yt-dlp.exe");
        if p.exists() {
            return p;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let bundled = exe.parent().unwrap().join("binaries").join("yt-dlp.exe");
        if bundled.exists() {
            return bundled;
        }
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries").join("yt-dlp.exe");
    if dev.exists() {
        return dev;
    }
    PathBuf::from("yt-dlp.exe")
}

/// Find the bundled ffmpeg directory, if present.
pub fn resolve_ffmpeg_dir() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        let bundled = exe.parent().unwrap().join("binaries").join("ffmpeg").join("bin");
        if bundled.exists() {
            return Some(bundled);
        }
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries").join("ffmpeg").join("bin");
    if dev.exists() {
        return Some(dev);
    }
    None
}

// ---------------------------------------------------------------------------
// Progress line parsing
// ---------------------------------------------------------------------------

fn parse_optional(s: &str) -> Option<f64> {
    match s {
        "NA" | "None" | "" => None,
        _ => s.parse().ok(),
    }
}

fn parse_progress_line(id: u32, body: &str) -> Option<DownloadProgress> {
    // "__PROG__|downloaded|total|total_estimate|speed|eta"
    let p: Vec<&str> = body.split('|').collect();
    if p.len() < 6 || p[0] != "__PROG__" {
        return None;
    }
    let total = parse_optional(p[2])
        .or_else(|| parse_optional(p[3]))
        .map(|v| v as u64);
    Some(DownloadProgress {
        id,
        downloaded: parse_optional(p[1]).unwrap_or(0.0) as u64,
        total,
        speed: parse_optional(p[4]),
        eta: parse_optional(p[5]).map(|v| v as u64),
        status: JobStatus::Downloading,
        message: None,
        title: None,
        filepath: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn emit_status(
    app: &AppHandle,
    id: u32,
    status: JobStatus,
    message: Option<String>,
    title: Option<String>,
    filepath: Option<String>,
) {
    let _ = app.emit(
        "download-status",
        DownloadProgress {
            id,
            downloaded: 0,
            total: None,
            speed: None,
            eta: None,
            status,
            message,
            title,
            filepath,
        },
    );
}

// ---------------------------------------------------------------------------
// Worker: spawn yt-dlp, read stdout line by line, emit Tauri events
// ---------------------------------------------------------------------------

fn run_job(
    app: AppHandle,
    manager: JobManager,
    id: u32,
    url: String,
    options: DownloadOptions,
) {
    let args = match options.to_args() {
        Ok(a) => a,
        Err(e) => {
            manager.update_info(id, |j| {
                j.status = JobStatus::Error;
                j.error = Some(e.clone());
            });
            emit_status(&app, id, JobStatus::Error, Some(e), None, None);
            return;
        }
    };

    let mut cmd = Command::new(resolve_binary());
    cmd.args(&args)
        .arg(&url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // yt-dlp writes non-ASCII paths (e.g. D:\下载\…) in the console codepage
    // when piped, which breaks UTF-8 line parsing; force UTF-8 output.
    cmd.env("PYTHONUTF8", "1").env("PYTHONIOENCODING", "utf-8");
    if let Some(ffmpeg_dir) = resolve_ffmpeg_dir() {
        cmd.arg("--ffmpeg-location").arg(&ffmpeg_dir);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    emit_status(&app, id, JobStatus::Resolving, None, None, None);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("failed to spawn yt-dlp: {e}");
            manager.update_info(id, |j| {
                j.status = JobStatus::Error;
                j.error = Some(msg.clone());
            });
            emit_status(&app, id, JobStatus::Error, Some(msg), None, None);
            return;
        }
    };
    // Detach the pipes first so `child` itself can be parked in the shared
    // job state (for cancellation) without borrow conflicts.
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    manager.set_child(id, child);

    let mut title: Option<String> = None;
    let mut filepath: Option<String> = None;

    if let Some(stdout) = stdout {
        // Read raw bytes and decode lossily: a hard error from lines() on a
        // stray non-UTF-8 byte would silently truncate the stream.
        use std::io::Read;
        let mut reader = BufReader::new(stdout);
        let mut buf = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                Err(_) => break,
            }
        }
        // Decode per line: yt-dlp prints titles in UTF-8 (--encoding) but file
        // paths through os.fsdecode in the console codepage (GBK on zh-CN), so
        // the two lines in one download can have different encodings.
        let mut lines_text = String::new();
        let mut line_start = 0usize;
        for (i, &b) in buf.iter().enumerate() {
            if b == b'\n' {
                let raw = &buf[line_start..i];
                lines_text.push_str(&decode_output(raw));
                lines_text.push('\n');
                line_start = i + 1;
            }
        }
        if line_start < buf.len() {
            lines_text.push_str(&decode_output(&buf[line_start..]));
        }
        for line in lines_text.lines() {
            if let Some(body) = line.strip_prefix("__ENTRY__|") {
                title = Some(body.to_string());
                manager.update_info(id, |j| j.title = Some(body.to_string()));
            } else if let Some(body) = line.strip_prefix("__FILE__|") {
                filepath = Some(body.to_string());
                manager.update_info(id, |j| j.filepath = Some(body.to_string()));
            } else if let Some(p) = parse_progress_line(id, line) {
                let _ = app.emit("download-progress", p);
            }
        }
    }

    // stderr is only consumed after stdout closes; yt-dlp writes errors there.
    let mut error_msg: Option<String> = None;
    if let Some(mut stderr) = stderr {
        use std::io::Read;
        let mut buf = String::new();
        let _ = stderr.read_to_string(&mut buf);
        let last = buf.lines().last().unwrap_or("").trim().to_string();
        if !last.is_empty() {
            error_msg = Some(last);
        }
    }

    // Parked child comes back for wait(); gone if the user cancelled.
    let exit_ok = match manager.take_child(id) {
        Some(mut c) => c.wait().map(|s| s.success()).unwrap_or(false),
        None => false,
    };
    let cancelled = manager.is_cancelled(id);
    let status = if cancelled {
        JobStatus::Cancelled
    } else if exit_ok {
        JobStatus::Finished
    } else {
        JobStatus::Error
    };

    manager.update_info(id, |j| {
        j.status = status;
        if let Some(t) = &title {
            j.title = Some(t.clone());
        }
        if let Some(f) = &filepath {
            j.filepath = Some(f.clone());
        }
        if status == JobStatus::Error && j.error.is_none() {
            j.error = error_msg.clone();
        }
    });
    emit_status(
        &app,
        id,
        status,
        if exit_ok || cancelled { None } else { error_msg },
        title,
        filepath,
    );
}

// ---------------------------------------------------------------------------
// Tauri commands (called from the frontend)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn start_download(
    app: AppHandle,
    manager: State<JobManager>,
    url: String,
    options: Option<DownloadOptions>,
) -> Result<u32, String> {
    let url = url.trim().to_string();
    if url.is_empty() {
        return Err("URL is empty".into());
    }
    let id = manager.alloc_id();
    let info = JobInfo {
        id,
        url: url.clone(),
        title: None,
        status: JobStatus::Queued,
        filepath: None,
        error: None,
    };
    manager.insert(id, info);
    let options = options.unwrap_or_default();
    let mgr = manager.inner().clone();

    thread::spawn(move || run_job(app, mgr, id, url, options));
    Ok(id)
}

#[tauri::command]
pub fn cancel_download(manager: State<JobManager>, id: u32) -> Result<(), String> {
    let jobs = manager.jobs.lock().unwrap();
    if let Some(js) = jobs.get(&id) {
        let mut state = js.lock().unwrap();
        if let Some(child) = state.child.as_mut() {
            let _ = child.kill();
        }
        state.info.status = JobStatus::Cancelled;
    }
    Ok(())
}

#[tauri::command]
pub fn list_jobs(manager: State<JobManager>) -> Vec<JobInfo> {
    let jobs = manager.jobs.lock().unwrap();
    let mut out: Vec<JobInfo> = jobs
        .values()
        .map(|js| js.lock().unwrap().info.clone())
        .collect();
    out.sort_by_key(|j| j.id);
    out
}

#[tauri::command]
pub fn clear_finished(manager: State<JobManager>) {
    let mut jobs = manager.jobs.lock().unwrap();
    jobs.retain(|_, js| {
        let s = js.lock().unwrap().info.status;
        !matches!(s, JobStatus::Finished | JobStatus::Error | JobStatus::Cancelled)
    });
}

/// Probe metadata (title, duration, formats, etc.) for a URL without downloading.
#[tauri::command]
pub fn fetch_info(url: String) -> Result<serde_json::Value, String> {
    let mut cmd = Command::new(resolve_binary());
    cmd.arg("-J")
        .arg("--no-warnings")
        .arg("--encoding")
        .arg("utf-8")
        .arg(url.trim());
    if let Some(ffmpeg_dir) = resolve_ffmpeg_dir() {
        cmd.arg("--ffmpeg-location").arg(&ffmpeg_dir);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let output = cmd.output().map_err(|e| format!("failed to run yt-dlp: {e}"))?;
    if !output.status.success() {
        let err = decode_output(&output.stderr);
        let last = err.lines().last().unwrap_or("failed to fetch info").to_string();
        return Err(last);
    }
    let stdout = decode_output(&output.stdout);
    serde_json::from_str(&stdout).map_err(|e| format!("failed to parse metadata: {e}"))
}

/// Current yt-dlp version + whether a newer one exists upstream.
#[tauri::command]
pub fn ytdlp_version() -> Result<serde_json::Value, String> {
    let out = Command::new(resolve_binary())
        .arg("--version")
        .output()
        .map_err(|e| format!("failed to run yt-dlp: {e}"))?;
    let current = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let latest = crate::updater::latest_ytdlp_version().ok();
    let update_available = match &latest {
        Some(l) => !l.is_empty() && l.as_str() != current.as_str(),
        None => false,
    };
    Ok(serde_json::json!({
        "current": current,
        "latest": latest,
        "updateAvailable": update_available,
    }))
}

/// Download the newest yt-dlp.exe into the app-data override dir.
#[tauri::command]
pub fn update_ytdlp_now() -> Result<String, String> {
    let dir = app_data_dir().ok_or("cannot resolve app-data dir")?;
    crate::updater::update_ytdlp(&dir)
}

/// Reveal a file or folder in Windows Explorer (selects the file).
#[tauri::command]
pub fn reveal_path(app: tauri::AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| format!("failed to reveal path: {e}"))
}

/// The user's Downloads folder — used as the default save location.
#[tauri::command]
pub fn default_download_dir() -> Result<String, String> {
    // No winrt dependency; standard layout via known-folder env is unreliable,
    // so query the shell for the real Downloads path.
    let out = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(New-Object -ComObject Shell.Application).Namespace('shell:Downloads').Self.Path",
        ])
        .output()
        .map_err(|e| format!("failed to query Downloads folder: {e}"))?;
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if path.is_empty() {
        // fallback: %USERPROFILE%\Downloads
        let profile = std::env::var("USERPROFILE").map_err(|e| e.to_string())?;
        return Ok(PathBuf::from(profile)
            .join("Downloads")
            .to_string_lossy()
            .into_owned());
    }
    Ok(path)
}
