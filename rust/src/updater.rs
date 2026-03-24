//! Auto-update checker for CodexBar
//! Checks GitHub releases for new versions and handles background downloads

use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::watch;
use crate::settings::UpdateChannel;

const GITHUB_REPO: &str = "Finesssee/Win-CodexBar";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// State of the update download process
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateState {
    /// No update available or not checked
    Idle,
    /// Update available but not downloaded
    Available,
    /// Currently downloading with progress (0.0 to 1.0)
    Downloading(f32),
    /// Download complete, ready to install
    Ready(PathBuf),
    /// Download or install failed
    Failed(String),
}

impl Default for UpdateState {
    fn default() -> Self {
        UpdateState::Idle
    }
}

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    #[allow(dead_code)]
    pub release_url: String,
    #[allow(dead_code)]
    pub release_notes: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    assets: Vec<GitHubAsset>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    #[allow(dead_code)]
    prerelease: bool,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Check for updates from GitHub releases
///
/// When `channel` is `UpdateChannel::Beta`, includes pre-release versions.
/// When `channel` is `UpdateChannel::Stable`, only considers stable releases.
#[allow(dead_code)]
pub async fn check_for_updates() -> Option<UpdateInfo> {
    check_for_updates_with_channel(UpdateChannel::Stable).await
}

/// Check for updates from GitHub releases with a specific channel
///
/// When `channel` is `UpdateChannel::Beta`, includes pre-release versions.
/// When `channel` is `UpdateChannel::Stable`, only considers stable releases.
pub async fn check_for_updates_with_channel(channel: UpdateChannel) -> Option<UpdateInfo> {
    let url = match channel {
        UpdateChannel::Beta => {
            // For beta, we need to check all releases and find the latest (including pre-releases)
            format!(
                "https://api.github.com/repos/{}/releases",
                GITHUB_REPO
            )
        }
        UpdateChannel::Stable => {
            // For stable, use the /latest endpoint which excludes pre-releases
            format!(
                "https://api.github.com/repos/{}/releases/latest",
                GITHUB_REPO
            )
        }
    };

    let client = reqwest::Client::builder()
        .user_agent("CodexBar")
        .build()
        .ok()?;

    let response = client.get(&url).send().await.ok()?;

    if !response.status().is_success() {
        tracing::debug!("GitHub API returned status: {}", response.status());
        return None;
    }

    // Parse response based on channel
    let release: GitHubRelease = match channel {
        UpdateChannel::Beta => {
            // For beta, we get an array of releases - take the first non-draft one
            let releases: Vec<GitHubRelease> = response.json().await.ok()?;
            releases.into_iter().find(|r| !r.draft)?
        }
        UpdateChannel::Stable => {
            // For stable, we get a single release object
            response.json().await.ok()?
        }
    };

    // Parse version from tag (remove 'v' prefix and '-windows' suffix if present)
    let remote_version = release
        .tag_name
        .trim_start_matches('v')
        .split('-')
        .next()
        .unwrap_or(&release.tag_name);

    // Compare versions
    if is_newer_version(remote_version, CURRENT_VERSION) {
        // Find the installer or exe asset
        let download_url = release
            .assets
            .iter()
            .find(|a| a.name.ends_with("-Setup.exe"))
            .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".exe")))
            .map(|a| a.browser_download_url.clone())
            .unwrap_or_else(|| release.html_url.clone());

        Some(UpdateInfo {
            version: release.tag_name,
            download_url,
            release_url: release.html_url,
            release_notes: release.body.unwrap_or_default(),
        })
    } else {
        None
    }
}

/// Compare semantic versions, returns true if remote is newer
fn is_newer_version(remote: &str, current: &str) -> bool {
    let parse_version = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<u32> = v
            .split('.')
            .filter_map(|p| p.parse().ok())
            .collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    };

    let remote_v = parse_version(remote);
    let current_v = parse_version(current);

    remote_v > current_v
}

/// Get the current version
#[allow(dead_code)]
pub fn current_version() -> &'static str {
    CURRENT_VERSION
}

/// Get the download directory for updates
fn get_download_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|p| p.join("CodexBar").join("updates"))
}

/// Download an update with progress reporting
///
/// Returns a receiver that will receive progress updates (0.0 to 1.0)
/// and the final downloaded file path on completion.
pub async fn download_update(
    update_info: &UpdateInfo,
    progress_tx: watch::Sender<UpdateState>,
) -> Result<PathBuf, String> {
    let download_dir = get_download_dir()
        .ok_or_else(|| "Could not determine download directory".to_string())?;

    // Create download directory if it doesn't exist
    std::fs::create_dir_all(&download_dir)
        .map_err(|e| format!("Failed to create download directory: {}", e))?;

    // Extract filename from URL or use default
    let filename = update_info.download_url
        .split('/')
        .last()
        .unwrap_or("CodexBar-Setup.exe")
        .to_string();

    let file_path = download_dir.join(&filename);

    // Start download
    let client = reqwest::Client::builder()
        .user_agent("CodexBar")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&update_info.download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to start download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);

    // Create file for writing
    let mut file = tokio::fs::File::create(&file_path)
        .await
        .map_err(|e| format!("Failed to create file: {}", e))?;

    use tokio::io::AsyncWriteExt;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    use futures::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Error downloading chunk: {}", e))?;

        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write chunk: {}", e))?;

        downloaded += chunk.len() as u64;

        // Calculate and send progress
        let progress = if total_size > 0 {
            (downloaded as f32 / total_size as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let _ = progress_tx.send(UpdateState::Downloading(progress));
    }

    file.flush()
        .await
        .map_err(|e| format!("Failed to flush file: {}", e))?;

    // Verify download integrity using SHA256 checksum from release
    verify_download_hash(&client, &update_info.download_url, &file_path).await?;

    // Signal download complete
    let _ = progress_tx.send(UpdateState::Ready(file_path.clone()));

    Ok(file_path)
}

/// Verify the SHA256 hash of a downloaded file against a .sha256 sidecar file
async fn verify_download_hash(
    client: &reqwest::Client,
    download_url: &str,
    file_path: &PathBuf,
) -> Result<(), String> {
    use sha2::{Sha256, Digest};

    let hash_url = format!("{}.sha256", download_url);

    let hash_resp = client
        .get(&hash_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch checksum: {}", e))?;

    if !hash_resp.status().is_success() {
        tracing::warn!(
            "No SHA256 checksum file found at {}. Skipping verification — consider publishing a .sha256 sidecar.",
            hash_url
        );
        return Ok(());
    }

    let expected_hash = hash_resp
        .text()
        .await
        .map_err(|e| format!("Failed to read checksum: {}", e))?;

    // Parse hash (format: "hash  filename" or just "hash")
    let expected = expected_hash
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase();

    if expected.len() != 64 {
        return Err(format!("Invalid SHA256 hash length: {} chars", expected.len()));
    }

    let file_bytes = tokio::fs::read(file_path)
        .await
        .map_err(|e| format!("Failed to read downloaded file for hashing: {}", e))?;

    let mut hasher = Sha256::new();
    hasher.update(&file_bytes);
    let actual = format!("{:x}", hasher.finalize());

    if actual != expected {
        // Delete the corrupted file
        let _ = tokio::fs::remove_file(file_path).await;
        return Err(format!(
            "SHA256 mismatch: expected {}, got {}. Download may be corrupted or tampered.",
            expected, actual
        ));
    }

    tracing::info!("SHA256 verification passed for {:?}", file_path);
    Ok(())
}

/// Start background download of an update
///
/// Returns a receiver that can be polled for progress updates.
#[allow(dead_code)]
pub fn start_background_download(
    update_info: UpdateInfo,
) -> (Arc<watch::Receiver<UpdateState>>, std::thread::JoinHandle<()>) {
    let (tx, rx) = watch::channel(UpdateState::Available);
    let rx = Arc::new(rx);

    let handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            match download_update(&update_info, tx.clone()).await {
                Ok(_path) => {
                    // UpdateState::Ready is already sent by download_update
                }
                Err(e) => {
                    let _ = tx.send(UpdateState::Failed(e));
                }
            }
        });
    });

    (rx, handle)
}

/// Apply a downloaded update by spawning the installer and exiting
///
/// This function will:
/// 1. Spawn the installer executable
/// 2. Exit the current application
///
/// The installer should handle upgrading the application while it's closed.
pub fn apply_update(installer_path: &PathBuf) -> Result<(), String> {
    use std::process::Command;

    // Verify the file exists
    if !installer_path.exists() {
        return Err(format!("Installer not found: {:?}", installer_path));
    }

    // Spawn the installer process
    // Using /SILENT for NSIS-style installers, or /quiet for MSI
    // The installer should detect the running app and wait or prompt
    #[cfg(target_os = "windows")]
    {
        Command::new(installer_path)
            .args(["/SILENT", "/CLOSEAPPLICATIONS"])
            .spawn()
            .map_err(|e| format!("Failed to launch installer: {}", e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new(installer_path)
            .spawn()
            .map_err(|e| format!("Failed to launch installer: {}", e))?;
    }

    // Exit the application to allow the installer to proceed
    std::process::exit(0);
}

/// Check if there's a pending update ready to install
#[allow(dead_code)]
pub fn get_pending_update() -> Option<PathBuf> {
    let download_dir = get_download_dir()?;

    if !download_dir.exists() {
        return None;
    }

    // Look for any .exe files in the updates directory
    std::fs::read_dir(&download_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .map(|ext| ext.eq_ignore_ascii_case("exe"))
                .unwrap_or(false)
        })
}

/// Clean up downloaded updates
#[allow(dead_code)]
pub fn cleanup_downloads() {
    if let Some(download_dir) = get_download_dir() {
        let _ = std::fs::remove_dir_all(&download_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("1.0.1", "1.0.0"));
        assert!(is_newer_version("1.1.0", "1.0.0"));
        assert!(is_newer_version("2.0.0", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("0.9.0", "1.0.0"));
        assert!(is_newer_version("1.0.0", "0.1.0"));
    }
}
