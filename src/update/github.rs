//! GitHub Releases client for the updater.
//!
//! Only two endpoints are used: the release metadata for the latest tag and
//! the assets it points to (the tarball and `SHA256SUMS`). An optional token
//! (`GH_TOKEN` or `GITHUB_TOKEN`) raises the rate limit of the public API.

use semver::Version;
use serde::Deserialize;
use std::io::Read;
use std::time::Duration;

/// Metadata for one downloadable release asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    #[allow(dead_code)]
    pub size: u64,
}

/// A release with its parsed version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub tag: String,
    pub version: Version,
    pub assets: Vec<Asset>,
    #[allow(dead_code)]
    pub html_url: String,
}

#[derive(Debug, Deserialize)]
struct WireRelease {
    tag_name: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    assets: Vec<WireAsset>,
}

#[derive(Debug, Deserialize)]
struct WireAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

struct HttpResponse {
    content_length: Option<u64>,
    body: ureq::Body,
}

/// Small HTTP client with xfetch's user agent and optional auth.
pub struct Client {
    agent: ureq::Agent,
    token: Option<String>,
}

impl Client {
    pub fn new() -> Result<Self, String> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .user_agent(concat!("xfetch/", env!("CARGO_PKG_VERSION")))
            .max_redirects(5)
            .http_status_as_error(false)
            .build()
            .into();
        let token = std::env::var("GH_TOKEN")
            .or_else(|_| std::env::var("GITHUB_TOKEN"))
            .ok()
            .filter(|value| !value.trim().is_empty());
        Ok(Self { agent, token })
    }

    /// Fetches a small JSON/text document with a bounded size.
    pub fn get_text(&self, url: &str, cap: usize, timeout: Duration) -> Result<String, String> {
        let mut response = self.request(url, timeout)?;
        let mut body = Vec::new();
        response
            .body
            .with_config()
            .limit(cap as u64 + 1)
            .reader()
            .take(cap as u64 + 1)
            .read_to_end(&mut body)
            .map_err(|err| format!("Failed to read '{}': {}", url, err))?;
        if body.len() > cap {
            return Err(format!("Response from '{}' is too large", url));
        }
        String::from_utf8(body).map_err(|_| format!("Response from '{}' is not UTF-8", url))
    }

    /// Downloads a binary asset with a bounded size.
    pub fn get_bytes(&self, url: &str, cap: usize) -> Result<Vec<u8>, String> {
        let mut response = self.request(url, Duration::from_secs(300))?;
        let declared = response.content_length.unwrap_or(0);
        if declared > cap as u64 {
            return Err(format!(
                "Asset from '{}' is larger than the {} MiB limit",
                url,
                cap / (1024 * 1024)
            ));
        }

        let mut body = Vec::new();
        response
            .body
            .with_config()
            .limit(cap as u64 + 1)
            .reader()
            .take(cap as u64 + 1)
            .read_to_end(&mut body)
            .map_err(|err| format!("Failed to download '{}': {}", url, err))?;
        if body.len() > cap {
            return Err(format!("Asset from '{}' exceeds the size limit", url));
        }
        Ok(body)
    }

    /// Sends a GET with the standard headers.
    fn request(&self, url: &str, timeout: Duration) -> Result<HttpResponse, String> {
        let mut request = self
            .agent
            .get(url)
            .config()
            .timeout_global(Some(timeout))
            .build()
            .header("Accept", "application/vnd.github+json");
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let mut response = request
            .call()
            .map_err(|err| format!("Request to '{}' failed: {}", url, err))?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            if status == 403 {
                return Err(format!(
                    "'{}' returned HTTP 403 (rate limit?). Set GH_TOKEN or try again later.",
                    url
                ));
            }
            let body = response
                .body_mut()
                .with_config()
                .limit(160)
                .read_to_string()
                .unwrap_or_default();
            return Err(format!("'{}' returned HTTP {}: {}", url, status, body));
        }

        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        Ok(HttpResponse {
            content_length,
            body: response.into_body(),
        })
    }
}

/// Fetches and parses the latest release.
pub fn fetch_latest(client: &Client, api_url: &str) -> Result<Release, String> {
    let body = client.get_text(api_url, 1024 * 1024, Duration::from_secs(20))?;
    parse_release(&body)
}

/// Parses the release JSON returned by the GitHub API.
pub fn parse_release(body: &str) -> Result<Release, String> {
    let wire: WireRelease =
        serde_json::from_str(body).map_err(|err| format!("Invalid release JSON: {}", err))?;

    let version_text = wire.tag_name.trim_start_matches('v');
    let version = Version::parse(version_text)
        .map_err(|err| format!("Invalid release tag '{}': {}", wire.tag_name, err))?;

    let assets = wire
        .assets
        .into_iter()
        .map(|asset| Asset {
            name: asset.name,
            url: asset.browser_download_url,
            size: asset.size,
        })
        .collect();

    Ok(Release {
        tag: wire.tag_name,
        version,
        assets,
        html_url: wire.html_url,
    })
}

/// Release asset name for one target, as produced by the release workflow.
///
/// Unix targets are published as `.tar.gz`; the Windows target is a `.zip`.
pub fn asset_name(version: &Version, target: &str) -> String {
    let extension = if target.contains("windows") {
        "zip"
    } else {
        "tar.gz"
    };
    format!("xfetch-{}-{}.{}", version, target, extension)
}

/// Finds the prebuilt asset for `target`.
pub fn choose_asset<'a>(release: &'a Release, target: &str) -> Option<&'a Asset> {
    let name = asset_name(&release.version, target);
    release.assets.iter().find(|asset| asset.name == name)
}

/// Finds the `SHA256SUMS` asset.
pub fn checksums_asset(release: &Release) -> Option<&Asset> {
    release
        .assets
        .iter()
        .find(|asset| asset.name == "SHA256SUMS")
}

/// Parses one hash out of a `sha256sum`-style file.
pub fn parse_checksum(sums: &str, file: &str) -> Option<String> {
    for line in sums.lines() {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        if name == file && hash.len() == 64 {
            return Some(hash.to_ascii_lowercase());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
        "tag_name": "v1.2.3",
        "html_url": "https://github.com/xfetch-cli/xfetch/releases/tag/v1.2.3",
        "assets": [
            {
                "name": "xfetch-1.2.3-x86_64-unknown-linux-gnu.tar.gz",
                "browser_download_url": "https://example.com/xfetch.tar.gz",
                "size": 1234
            },
            {
                "name": "xfetch-1.2.3-x86_64-pc-windows-msvc.zip",
                "browser_download_url": "https://example.com/xfetch.zip",
                "size": 2345
            },
            {
                "name": "SHA256SUMS",
                "browser_download_url": "https://example.com/SHA256SUMS",
                "size": 42
            }
        ]
    }"#;

    #[test]
    fn parses_release_and_assets() {
        let release = parse_release(FIXTURE).expect("parse");
        assert_eq!(release.version, Version::parse("1.2.3").expect("version"));
        assert_eq!(release.tag, "v1.2.3");
        assert_eq!(release.assets.len(), 3);

        let asset = choose_asset(&release, "x86_64-unknown-linux-gnu").expect("asset");
        assert_eq!(asset.url, "https://example.com/xfetch.tar.gz");

        let windows = choose_asset(&release, "x86_64-pc-windows-msvc").expect("windows asset");
        assert_eq!(windows.url, "https://example.com/xfetch.zip");

        assert!(
            choose_asset(&release, "aarch64-apple-darwin").is_none(),
            "missing targets must not match"
        );
        assert!(checksums_asset(&release).is_some());
    }

    #[test]
    fn asset_names_use_zip_for_windows() {
        let version = Version::parse("1.2.3").expect("version");
        assert_eq!(
            asset_name(&version, "x86_64-unknown-linux-gnu"),
            "xfetch-1.2.3-x86_64-unknown-linux-gnu.tar.gz"
        );
        assert_eq!(
            asset_name(&version, "x86_64-pc-windows-msvc"),
            "xfetch-1.2.3-x86_64-pc-windows-msvc.zip"
        );
    }

    #[test]
    fn rejects_bad_tags() {
        let body = r#"{"tag_name": "not-a-version", "assets": []}"#;
        assert!(parse_release(body).is_err());
    }

    #[test]
    fn parses_checksums_with_star_suffix() {
        let hash_a = "a".repeat(64);
        let hash_b = "b".repeat(64);
        let sums = format!(
            "{hash_a}  xfetch-1.2.3-x86_64-unknown-linux-gnu.tar.gz\n\
             {hash_b} *xfetch-1.2.3-aarch64-apple-darwin.tar.gz\n"
        );
        assert_eq!(
            parse_checksum(&sums, "xfetch-1.2.3-x86_64-unknown-linux-gnu.tar.gz"),
            Some(hash_a)
        );
        assert_eq!(
            parse_checksum(&sums, "xfetch-1.2.3-aarch64-apple-darwin.tar.gz"),
            Some(hash_b)
        );
        assert_eq!(parse_checksum(&sums, "other.tar.gz"), None);
    }
}
