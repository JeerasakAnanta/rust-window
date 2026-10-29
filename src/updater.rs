use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use semver::Version;
use sha2::{Digest, Sha256};

pub const DEFAULT_API_URL: &str =
    "https://api.github.com/repos/JeerasakAnanta/rust-window/releases/latest";
const MAX_DOWNLOAD: u64 = 200 * 1024 * 1024;

#[derive(Debug, PartialEq)]
pub struct Release {
    pub version: Version,
    pub installer_url: String,
    pub sha256_url: String,
}

pub fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("valid CARGO_PKG_VERSION")
}

pub fn api_url() -> String {
    std::env::var("RUST_WINDOW_UPDATE_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string())
}

/// Parse a GitHub "latest release" JSON document.
pub fn parse_release(json: &str) -> Result<Release, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let tag = v["tag_name"].as_str().ok_or("missing tag_name")?;
    let version = Version::parse(tag.trim_start_matches('v')).map_err(|e| e.to_string())?;
    let assets = v["assets"].as_array().ok_or("missing assets")?;
    let url_of = |suffix: &str| {
        assets.iter().find_map(|a| {
            let name = a["name"].as_str()?;
            name.ends_with(suffix)
                .then(|| a["browser_download_url"].as_str().map(str::to_string))
                .flatten()
        })
    };
    let installer_url = url_of("-setup.exe").ok_or("no installer asset")?;
    let sha256_url = url_of("-setup.exe.sha256").ok_or("no checksum asset")?;
    Ok(Release { version, installer_url, sha256_url })
}

/// Returns the release only if it is newer than `current`.
pub fn newer_than(release: Release, current: &Version) -> Option<Release> {
    (release.version > *current).then_some(release)
}

pub fn check(api_url: &str, current: &Version) -> Result<Option<Release>, String> {
    let body = ureq::get(api_url)
        .header("User-Agent", "rust-window-updater")
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    Ok(newer_than(parse_release(&body)?, current))
}

fn get_bytes(url: &str) -> Result<Vec<u8>, String> {
    ureq::get(url)
        .header("User-Agent", "rust-window-updater")
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD)
        .read_to_vec()
        .map_err(|e| e.to_string())
}

/// `expected` is the content of a `.sha256` file: "<hex>  <filename>" or just "<hex>".
pub fn verify_sha256(data: &[u8], expected: &str) -> Result<(), String> {
    let want = expected
        .split_whitespace()
        .next()
        .ok_or("empty checksum file")?
        .to_ascii_lowercase();
    let got = hex::encode(Sha256::digest(data));
    if got == want {
        Ok(())
    } else {
        Err(format!("checksum mismatch: expected {want}, got {got}"))
    }
}

/// Downloads the installer into `dir` and verifies its SHA-256 before it touches disk.
pub fn download_verified(release: &Release, dir: &Path) -> Result<PathBuf, String> {
    let installer = get_bytes(&release.installer_url)?;
    let checksum = String::from_utf8(get_bytes(&release.sha256_url)?).map_err(|e| e.to_string())?;
    verify_sha256(&installer, &checksum)?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("rust-window-{}-setup.exe", release.version));
    fs::write(&path, installer).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Starts the installer silently; it closes and restarts the app itself.
pub fn launch_installer(path: &Path) -> Result<(), String> {
    Command::new(path)
        .args(["/SILENT", "/CLOSEAPPLICATIONS", "/RESTARTAPPLICATIONS"])
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r#"{"tag_name":"v0.2.0","assets":[
        {"name":"rust-window-setup.exe","browser_download_url":"http://x/a-setup.exe"},
        {"name":"rust-window-setup.exe.sha256","browser_download_url":"http://x/a.sha256"}]}"#;

    #[test]
    fn parses_release() {
        let r = parse_release(JSON).unwrap();
        assert_eq!(r.version, Version::new(0, 2, 0));
        assert_eq!(r.installer_url, "http://x/a-setup.exe");
        assert_eq!(r.sha256_url, "http://x/a.sha256");
    }

    #[test]
    fn rejects_missing_checksum_asset() {
        let json = r#"{"tag_name":"v0.2.0","assets":[
            {"name":"rust-window-setup.exe","browser_download_url":"http://x"}]}"#;
        assert!(parse_release(json).is_err());
    }

    #[test]
    fn only_newer_versions_are_offered() {
        let r = || parse_release(JSON).unwrap();
        assert!(newer_than(r(), &Version::new(0, 1, 0)).is_some());
        assert!(newer_than(r(), &Version::new(0, 2, 0)).is_none());
        assert!(newer_than(r(), &Version::new(0, 10, 0)).is_none());
    }

    #[test]
    fn checksum_accepts_match_and_rejects_tamper() {
        let data = b"installer bytes";
        let good = hex::encode(Sha256::digest(data));
        assert!(verify_sha256(data, &format!("{good}  rust-window-setup.exe\n")).is_ok());
        assert!(verify_sha256(data, &good.to_uppercase()).is_ok());
        assert!(verify_sha256(b"tampered", &good).is_err());
        assert!(verify_sha256(data, "").is_err());
    }
}
