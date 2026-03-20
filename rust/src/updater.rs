//! Auto-update checker for CodexBar
//! Checks GitHub releases for new versions and handles background downloads

use crate::settings::UpdateChannel;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::watch;

const GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_REPO: &str = "Finesssee/Win-CodexBar";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const SHA256_PREFIX: &str = "sha256:";

/// State of the update download process
#[derive(Debug, Clone, PartialEq, Default)]
pub enum UpdateState {
    /// No update available or not checked
    #[default]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateAssetKind {
    Installer,
    ManualDownload,
}

impl UpdateAssetKind {
    pub fn is_installable(self) -> bool {
        matches!(self, Self::Installer)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub asset_name: String,
    pub download_url: String,
    #[allow(dead_code)]
    pub release_url: String,
    #[allow(dead_code)]
    pub release_notes: String,
    asset_kind: UpdateAssetKind,
    sha256: Option<String>,
}

impl UpdateInfo {
    pub fn is_installable(&self) -> bool {
        self.asset_kind.is_installable()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingUpdate {
    pub update: UpdateInfo,
    pub file_path: PathBuf,
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
    #[serde(default)]
    digest: Option<String>,
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
    let release_repo = update_repo();
    let url = match channel {
        UpdateChannel::Beta => {
            // For beta, we need to check all releases and find the latest (including pre-releases)
            format!("{}/repos/{}/releases", update_api_base(), release_repo)
        }
        UpdateChannel::Stable => {
            // For stable, use the /latest endpoint which excludes pre-releases
            format!(
                "{}/repos/{}/releases/latest",
                update_api_base(),
                release_repo
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
        let selected_asset = select_release_asset(&release.assets);
        let (asset_name, download_url, asset_kind, sha256) = match selected_asset {
            Some(asset) => (
                asset.name.clone(),
                asset.browser_download_url.clone(),
                asset_kind_for_name(&asset.name),
                parse_sha256_value(asset.digest.as_deref().unwrap_or("")),
            ),
            None => (
                release.tag_name.clone(),
                release.html_url.clone(),
                UpdateAssetKind::ManualDownload,
                None,
            ),
        };

        Some(UpdateInfo {
            version: release.tag_name,
            asset_name,
            download_url,
            release_url: release.html_url,
            release_notes: release.body.unwrap_or_default(),
            asset_kind,
            sha256,
        })
    } else {
        None
    }
}

/// Compare semantic versions, returns true if remote is newer
fn is_newer_version(remote: &str, current: &str) -> bool {
    let parse_version = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<u32> = v.split('.').filter_map(|p| p.parse().ok()).collect();
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

fn update_api_base() -> String {
    std::env::var("CODEXBAR_UPDATE_API_BASE").unwrap_or_else(|_| GITHUB_API_BASE.to_string())
}

fn update_repo() -> String {
    std::env::var("CODEXBAR_UPDATE_REPO").unwrap_or_else(|_| GITHUB_REPO.to_string())
}

fn pending_update_metadata_path(download_dir: &Path) -> PathBuf {
    download_dir.join("pending-update.json")
}

fn prepare_download_dir() -> Result<PathBuf, String> {
    let download_dir =
        get_download_dir().ok_or_else(|| "Could not determine download directory".to_string())?;

    if download_dir.exists() {
        std::fs::remove_dir_all(&download_dir)
            .map_err(|e| format!("Failed to clear old updates: {}", e))?;
    }

    std::fs::create_dir_all(&download_dir)
        .map_err(|e| format!("Failed to create download directory: {}", e))?;

    Ok(download_dir)
}

/// Download an update with progress reporting
///
/// Returns a receiver that will receive progress updates (0.0 to 1.0)
/// and the final downloaded file path on completion.
pub async fn download_update(
    update_info: &UpdateInfo,
    progress_tx: watch::Sender<UpdateState>,
) -> Result<PathBuf, String> {
    if !update_info.is_installable() {
        return Err("This release must be installed manually from the release page.".to_string());
    }

    let download_dir = prepare_download_dir()?;
    let file_path = download_dir.join(&update_info.asset_name);

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
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
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

    // Verify download integrity before saving pending-update metadata.
    verify_download_hash(&client, update_info, &file_path).await?;
    save_pending_update(update_info, &file_path)?;

    // Signal download complete
    let _ = progress_tx.send(UpdateState::Ready(file_path.clone()));

    Ok(file_path)
}

/// Verify the SHA256 hash of a downloaded file against a .sha256 sidecar file
async fn verify_download_hash(
    client: &reqwest::Client,
    update_info: &UpdateInfo,
    file_path: &Path,
) -> Result<(), String> {
    let expected = match update_info.sha256.clone() {
        Some(digest) => digest,
        None => fetch_sidecar_hash(client, &update_info.download_url, file_path).await?,
    };
    let actual = calculate_sha256(file_path).await?;

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

async fn calculate_sha256(file_path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};

    let file_bytes = tokio::fs::read(file_path)
        .await
        .map_err(|e| format!("Failed to read downloaded file for hashing: {}", e))?;

    let mut hasher = Sha256::new();
    hasher.update(&file_bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

async fn fetch_sidecar_hash(
    client: &reqwest::Client,
    download_url: &str,
    file_path: &Path,
) -> Result<String, String> {
    let hash_url = format!("{}.sha256", download_url);
    let hash_resp = client
        .get(&hash_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch checksum: {}", e))?;

    if !hash_resp.status().is_success() {
        let _ = tokio::fs::remove_file(file_path).await;
        return Err(format!(
            "No checksum is available for this installer (looked for {}). Open the release page and install manually.",
            hash_url
        ));
    }

    let expected_hash = hash_resp
        .text()
        .await
        .map_err(|e| format!("Failed to read checksum: {}", e))?;

    let expected = match parse_sha256_value(&expected_hash) {
        Some(expected) => expected,
        None => {
            let _ = tokio::fs::remove_file(file_path).await;
            return Err("The published checksum is invalid.".to_string());
        }
    };

    tracing::info!("Using sidecar SHA256 checksum from {}", hash_url);
    Ok(expected)
}

fn select_release_asset(assets: &[GitHubAsset]) -> Option<&GitHubAsset> {
    assets
        .iter()
        .find(|asset| asset_kind_for_name(&asset.name).is_installable())
        .or_else(|| {
            assets.iter().find(|asset| {
                matches!(
                    asset_kind_for_name(&asset.name),
                    UpdateAssetKind::ManualDownload
                )
            })
        })
}

fn asset_kind_for_name(name: &str) -> UpdateAssetKind {
    if name.ends_with("-Setup.exe") {
        UpdateAssetKind::Installer
    } else {
        UpdateAssetKind::ManualDownload
    }
}

fn parse_sha256_value(raw: &str) -> Option<String> {
    let digest = raw.split_whitespace().next().unwrap_or("").trim();
    let digest = digest.strip_prefix(SHA256_PREFIX).unwrap_or(digest).trim();

    if digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(digest.to_ascii_lowercase())
    } else {
        None
    }
}

fn save_pending_update(update_info: &UpdateInfo, file_path: &Path) -> Result<(), String> {
    let download_dir =
        get_download_dir().ok_or_else(|| "Could not determine download directory".to_string())?;
    save_pending_update_in(&download_dir, update_info, file_path)
}

fn save_pending_update_in(
    download_dir: &Path,
    update_info: &UpdateInfo,
    file_path: &Path,
) -> Result<(), String> {
    std::fs::create_dir_all(download_dir)
        .map_err(|e| format!("Failed to prepare update metadata directory: {}", e))?;

    let pending = PendingUpdate {
        update: update_info.clone(),
        file_path: file_path.to_path_buf(),
    };
    let json = serde_json::to_string_pretty(&pending)
        .map_err(|e| format!("Failed to serialize pending update metadata: {}", e))?;

    std::fs::write(pending_update_metadata_path(download_dir), json)
        .map_err(|e| format!("Failed to save pending update metadata: {}", e))
}

/// Start background download of an update
///
/// Returns a receiver that can be polled for progress updates.
#[allow(dead_code)]
pub fn start_background_download(
    update_info: UpdateInfo,
) -> (
    Arc<watch::Receiver<UpdateState>>,
    std::thread::JoinHandle<()>,
) {
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
pub fn apply_update(installer_path: &Path) -> Result<(), String> {
    use std::process::Command;

    // Verify the file exists
    if !installer_path.exists() {
        return Err(format!("Installer not found: {:?}", installer_path));
    }

    // Spawn the installer process with silent-upgrade flags.
    #[cfg(target_os = "windows")]
    {
        Command::new(installer_path)
            .args([
                "/SP-",
                "/VERYSILENT",
                "/SUPPRESSMSGBOXES",
                "/NORESTART",
                "/CLOSEAPPLICATIONS",
            ])
            .spawn()
            .map_err(|e| format!("Failed to launch installer: {}", e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new(installer_path)
            .spawn()
            .map_err(|e| format!("Failed to launch installer: {}", e))?;
    }

    clear_pending_update_metadata();

    // Exit the application to allow the installer to proceed
    std::process::exit(0);
}

/// Check if there's a pending update ready to install
#[allow(dead_code)]
pub fn get_pending_update() -> Option<PendingUpdate> {
    let download_dir = get_download_dir()?;
    load_pending_update_from_dir(&download_dir)
}

fn load_pending_update_from_dir(download_dir: &Path) -> Option<PendingUpdate> {
    let metadata_path = pending_update_metadata_path(download_dir);
    let content = std::fs::read_to_string(&metadata_path).ok()?;
    let pending = serde_json::from_str::<PendingUpdate>(&content).ok()?;

    if pending.file_path.exists() {
        Some(pending)
    } else {
        let _ = std::fs::remove_file(metadata_path);
        None
    }
}

pub fn clear_pending_update_metadata() {
    if let Some(download_dir) = get_download_dir() {
        let _ = std::fs::remove_file(pending_update_metadata_path(&download_dir));
    }
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

    #[test]
    fn test_parse_sha256_value() {
        let digest = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert_eq!(parse_sha256_value(digest), Some(digest.to_string()));
        assert_eq!(
            parse_sha256_value(&format!("sha256:{}", digest)),
            Some(digest.to_string())
        );
        assert_eq!(
            parse_sha256_value(&format!("{}  CodexBar-1.2.3-Setup.exe", digest)),
            Some(digest.to_string())
        );
        assert_eq!(parse_sha256_value("not-a-digest"), None);
    }

    #[test]
    fn test_select_release_asset_prefers_installer() {
        let assets = vec![
            GitHubAsset {
                name: "codexbar.exe".to_string(),
                browser_download_url: "https://example.com/codexbar.exe".to_string(),
                digest: None,
            },
            GitHubAsset {
                name: "CodexBar-1.2.2-Setup.exe".to_string(),
                browser_download_url: "https://example.com/CodexBar-1.2.2-Setup.exe".to_string(),
                digest: Some(format!(
                    "{}{}",
                    SHA256_PREFIX,
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                )),
            },
        ];

        let selected = select_release_asset(&assets).unwrap();
        assert_eq!(selected.name, "CodexBar-1.2.2-Setup.exe");
        assert!(asset_kind_for_name(&selected.name).is_installable());
    }

    #[test]
    fn test_pending_update_round_trip() {
        let temp = tempfile::tempdir().unwrap();
        let installer_path = temp.path().join("CodexBar-1.2.3-Setup.exe");
        std::fs::write(&installer_path, b"installer").unwrap();

        let pending = UpdateInfo {
            version: "v1.2.3".to_string(),
            asset_name: "CodexBar-1.2.3-Setup.exe".to_string(),
            download_url: "https://example.com/CodexBar-1.2.3-Setup.exe".to_string(),
            release_url: "https://example.com/release".to_string(),
            release_notes: "notes".to_string(),
            asset_kind: UpdateAssetKind::Installer,
            sha256: Some(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            ),
        };

        save_pending_update_in(temp.path(), &pending, &installer_path).unwrap();
        let loaded = load_pending_update_from_dir(temp.path()).unwrap();

        assert_eq!(loaded.update.version, "v1.2.3");
        assert_eq!(loaded.file_path, installer_path);
        assert!(loaded.update.is_installable());
    }
}
