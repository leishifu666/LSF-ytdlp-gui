use serde::Deserialize;

/// GitHub API response for the latest yt-dlp release (only fields we need).
#[derive(Deserialize, Debug)]
pub struct Release {
    #[serde(default)]
    pub tag_name: String,
}

pub const YT_DLP_RELEASES_API: &str =
    "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";
pub const YT_DLP_EXE_URL: &str =
    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent("ytdlp-gui")
        .build()
        .map_err(|e| e.to_string())
}

/// Query the latest yt-dlp version tag, e.g. "2026.07.04".
pub fn latest_ytdlp_version() -> Result<String, String> {
    let release: Release = client()?
        .get(YT_DLP_RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;
    if release.tag_name.is_empty() {
        return Err("empty tag_name from GitHub API".into());
    }
    Ok(release.tag_name)
}

/// Stream a URL to a file on disk.
pub fn download_to_file(url: &str, dest: &std::path::Path) -> Result<(), String> {
    let mut resp = client()?
        .get(url)
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let mut file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    std::io::copy(&mut resp, &mut file).map_err(|e| e.to_string())?;
    Ok(())
}

/// Download the newest yt-dlp.exe into `dir` (the app-data override location).
pub fn update_ytdlp(dir: &std::path::Path) -> Result<String, String> {
    let new_version = latest_ytdlp_version()?;
    let tmp = dir.join("yt-dlp.exe.new");
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    download_to_file(YT_DLP_EXE_URL, &tmp)?;
    let final_path = dir.join("yt-dlp.exe");
    // On Windows a running/recently-run exe may be briefly locked; retry once.
    let swap = || {
        std::fs::rename(&tmp, &final_path)
            .or_else(|_| {
                std::fs::remove_file(&final_path)?;
                std::fs::rename(&tmp, &final_path)
            })
            .map_err(|e: std::io::Error| e.to_string())
    };
    match swap() {
        Ok(()) => Ok(new_version),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(format!("failed to replace yt-dlp.exe: {e}"))
        }
    }
}
