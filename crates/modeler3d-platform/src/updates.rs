//! GitHub release discovery and verified installer handoff. No network or process
//! runs without an explicit call. Desktop callers run blocking work on a worker.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use thiserror::Error;

const MAX_METADATA: u64 = 2 * 1024 * 1024;
const MAX_DOWNLOAD: u64 = 512 * 1024 * 1024;

/// Failures preserve the installed app and are suitable for an update dialog.
#[derive(Debug, Error)]
#[error("{0}")]
pub struct UpdateError(String);

fn failure(message: impl ToString) -> UpdateError {
    UpdateError(message.to_string())
}

/// Immutable release identity, selected installer and expected SHA-256.
#[derive(Clone, Debug)]
pub struct Update {
    /// Stable version offered to the user.
    pub version: String,
    /// GitHub's release notes page, validated against the configured repository.
    pub notes_url: String,
    asset: Asset,
}

#[derive(Clone, Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    html_url: String,
    assets: Vec<Asset>,
}

/// Checks an `owner/repository` value without allowing URL or path injection.
pub fn validate_repository(repo: &str) -> Result<(), UpdateError> {
    let parts: Vec<_> = repo.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|p| {
            p.is_empty()
                || *p == "."
                || *p == ".."
                || !p
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
        })
    {
        return Err(failure("Update repository must be owner/repository"));
    }
    Ok(())
}

fn agent(seconds: u64) -> ureq::Agent {
    ureq::Agent::config_builder()
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(seconds)))
        .build()
        .into()
}

/// Checks the latest stable release. A repository without a release is normal.
/// Anonymous HTTPS is used; no developer GitHub credentials are read or shipped.
pub fn check(repo: &str, running: &str) -> Result<Option<Update>, UpdateError> {
    validate_repository(repo)?;
    let mut response = match agent(15)
        .get(&format!(
            "https://api.github.com/repos/{repo}/releases/latest"
        ))
        .header("User-Agent", "Modeler3D-Updater")
        .header("Accept", "application/vnd.github+json")
        .call()
    {
        Ok(response) => response,
        Err(ureq::Error::StatusCode(404)) => return Ok(None),
        Err(error) => return Err(failure(error)),
    };
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(MAX_METADATA + 1)
        .read_to_end(&mut bytes)
        .map_err(failure)?;
    if bytes.len() as u64 > MAX_METADATA {
        return Err(failure("Release information is too large"));
    }
    select(
        &bytes,
        repo,
        running,
        std::env::consts::OS,
        std::env::consts::ARCH,
    )
}

fn select(
    bytes: &[u8],
    repo: &str,
    running: &str,
    os: &str,
    arch: &str,
) -> Result<Option<Update>, UpdateError> {
    let release: Release = serde_json::from_slice(bytes).map_err(failure)?;
    let version = semver::Version::parse(
        release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name),
    )
    .map_err(failure)?;
    let current = semver::Version::parse(running).map_err(failure)?;
    if release.draft || release.prerelease || !version.pre.is_empty() || version <= current {
        return Ok(None);
    }
    let page = format!(
        "https://github.com/{repo}/releases/tag/{}",
        release.tag_name
    );
    if release.html_url != page {
        return Err(failure("Release page is outside the update repository"));
    }
    let filename = asset_name(&version.to_string(), os, arch)?;
    let mut candidates = release.assets.into_iter().filter(|a| a.name == filename);
    let Some(asset) = candidates.next() else {
        return Ok(None);
    };
    if candidates.next().is_some() {
        return Err(failure("Release has duplicate installers"));
    }
    let expected_url = format!(
        "https://github.com/{repo}/releases/download/{}/{filename}",
        release.tag_name
    );
    if asset.browser_download_url != expected_url
        || asset.size == 0
        || asset.size > MAX_DOWNLOAD
        || expected_digest(&asset).is_err()
    {
        return Err(failure(
            "Installer has an invalid URL, size or SHA-256 digest",
        ));
    }
    Ok(Some(Update {
        version: version.to_string(),
        notes_url: release.html_url,
        asset,
    }))
}

fn asset_name(version: &str, os: &str, arch: &str) -> Result<String, UpdateError> {
    let suffix = match (os, arch) {
        ("windows", "x86_64") => "windows-x86_64.msi",
        ("macos", "aarch64" | "x86_64") => "macos-universal.dmg",
        ("linux", "x86_64") => "linux-x86_64.tar.gz",
        _ => {
            return Err(failure(
                "No update installer is available for this platform",
            ));
        }
    };
    Ok(format!("3D-Modeler-{version}-{suffix}"))
}

fn expected_digest(asset: &Asset) -> Result<&str, UpdateError> {
    asset
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .filter(|d| d.len() == 64 && d.bytes().all(|c| c.is_ascii_hexdigit()))
        .ok_or_else(|| failure("Installer is missing its SHA-256 digest"))
}

/// A verified installer. Dropping it never installs anything.
#[derive(Debug)]
pub struct PreparedUpdate {
    directory: PathBuf,
    file: PathBuf,
}

/// Downloads to a fresh cache directory and verifies both length and SHA-256.
/// `progress` receives downloaded/total bytes; it never touches UI state directly.
pub fn download(
    update: &Update,
    mut progress: impl FnMut(u64, u64),
) -> Result<PreparedUpdate, UpdateError> {
    let paths = crate::current_runtime_paths();
    let directory = paths
        .cache_dir
        .join("updates")
        .join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&directory).map_err(failure)?;
    let result = download_into(update, &directory, &mut progress);
    if result.is_err() {
        let _ = fs::remove_dir_all(&directory);
    }
    result
}

fn download_into(
    update: &Update,
    directory: &Path,
    progress: &mut impl FnMut(u64, u64),
) -> Result<PreparedUpdate, UpdateError> {
    let file = directory.join(&update.asset.name);
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file)
        .map_err(failure)?;
    let mut response = agent(600)
        .get(&update.asset.browser_download_url)
        .header("User-Agent", "Modeler3D-Updater")
        .call()
        .map_err(failure)?;
    let mut input = response.body_mut().as_reader();
    let mut hash = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let n = input.read(&mut buffer).map_err(failure)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > update.asset.size {
            return Err(failure("Installer download exceeded its advertised size"));
        }
        output.write_all(&buffer[..n]).map_err(failure)?;
        hash.update(&buffer[..n]);
        progress(total, update.asset.size);
    }
    output.sync_all().map_err(failure)?;
    verify_download(total, &hash.finalize(), &update.asset)?;
    Ok(PreparedUpdate {
        directory: directory.to_owned(),
        file,
    })
}

fn verify_download(size: u64, digest: &[u8], asset: &Asset) -> Result<(), UpdateError> {
    let actual: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    if size != asset.size || !actual.eq_ignore_ascii_case(expected_digest(asset)?) {
        return Err(failure(
            "Installer verification failed. Nothing was installed; try again.",
        ));
    }
    Ok(())
}

impl PreparedUpdate {
    /// Starts an installer helper only after the desktop has resolved unsaved
    /// work and committed to closing. The helper waits for this process to exit.
    /// Windows uses the existing MSI upgrade identity; Mac/Linux keep a rollback
    /// copy and restore it if replacement fails. Settings live outside the install.
    pub fn install_after_exit(&self) -> Result<(), UpdateError> {
        if !can_install() {
            return Err(failure(
                "This copy must be updated using its downloaded installer",
            ));
        }
        let exe = std::env::current_exe().map_err(failure)?;
        let pid = std::process::id().to_string();
        let script = self.directory.join(if cfg!(windows) {
            "install.ps1"
        } else {
            "install.sh"
        });
        fs::write(
            &script,
            if cfg!(windows) {
                include_str!("update-windows.ps1")
            } else {
                include_str!("update-unix.sh")
            },
        )
        .map_err(failure)?;
        let mut command;
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command = Command::new("powershell.exe");
            command
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                ])
                .arg(&script)
                .creation_flags(0x08000000);
        }
        #[cfg(not(windows))]
        {
            command = Command::new("/bin/sh");
            command.arg(&script);
        }
        command
            .arg(pid)
            .arg(&self.file)
            .arg(exe)
            .arg(&self.directory)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(failure)?;
        Ok(())
    }
}

/// Whether this is a supported installed copy, rather than a Cargo build or
/// an app running from a read-only Mac disk image. Development builds still
/// offer the release download but never replace their own source/build tree.
pub fn can_install() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let Some(parent) = exe.parent() else {
        return false;
    };
    if cfg!(windows) {
        let Some(base) = std::env::var_os("LOCALAPPDATA") else {
            return false;
        };
        match (
            fs::canonicalize(&exe),
            fs::canonicalize(PathBuf::from(base).join("Programs/Modeler3D/Modeler3D.exe")),
        ) {
            (Ok(actual), Ok(installed)) => actual == installed,
            _ => false,
        }
    } else if cfg!(target_os = "macos") {
        parent.file_name().is_some_and(|n| n == "MacOS")
            && parent.parent().and_then(Path::parent).is_some_and(|p| {
                p.extension().is_some_and(|e| e == "app")
                    && !p.starts_with("/Volumes")
                    && !p
                        .components()
                        .any(|part| part.as_os_str() == "AppTranslocation")
            })
    } else {
        exe.file_name().is_some_and(|n| n == "Modeler3D") && parent.join("assets/original").is_dir()
    }
}

/// Reads the previous helper's bounded outcome once. This never changes an
/// installed application and may be called during desktop startup.
pub fn take_outcome() -> Option<String> {
    let file = crate::current_runtime_paths()
        .cache_dir
        .join("updates/last-result.txt");
    let mut bytes = Vec::new();
    fs::File::open(&file)
        .ok()?
        .take(4096)
        .read_to_end(&mut bytes)
        .ok()?;
    let _ = fs::remove_file(file);
    Some(String::from_utf8_lossy(&bytes).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn release(version: &str, os: &str, arch: &str) -> serde_json::Value {
        let name = asset_name(version, os, arch).unwrap();
        serde_json::json!({"tag_name":format!("v{version}"),"draft":false,"prerelease":false,
            "html_url":format!("https://github.com/owner/repo/releases/tag/v{version}"),
            "assets":[{"name":name,"browser_download_url":format!("https://github.com/owner/repo/releases/download/v{version}/{name}"),"size":3,"digest":format!("sha256:{:x}",Sha256::digest(b"abc"))}]})
    }
    fn choose(
        v: &serde_json::Value,
        current: &str,
        os: &str,
        arch: &str,
    ) -> Result<Option<Update>, UpdateError> {
        select(
            &serde_json::to_vec(v).unwrap(),
            "owner/repo",
            current,
            os,
            arch,
        )
    }
    #[test]
    fn stable_versions_compare_numerically_and_never_downgrade() {
        for (current, offered) in [("0.9.0", true), ("0.10.0", false), ("1.0.0", false)] {
            assert_eq!(
                choose(
                    &release("0.10.0", "windows", "x86_64"),
                    current,
                    "windows",
                    "x86_64"
                )
                .unwrap()
                .is_some(),
                offered
            );
        }
        let mut r = release("1.0.0", "windows", "x86_64");
        r["prerelease"] = true.into();
        assert!(choose(&r, "0.1.0", "windows", "x86_64").unwrap().is_none());
        r["prerelease"] = false.into();
        r["draft"] = true.into();
        assert!(choose(&r, "0.1.0", "windows", "x86_64").unwrap().is_none());
    }
    #[test]
    fn picks_only_the_exact_platform_installer() {
        for (os, arch) in [
            ("windows", "x86_64"),
            ("macos", "aarch64"),
            ("macos", "x86_64"),
            ("linux", "x86_64"),
        ] {
            assert!(
                choose(&release("1.0.0", os, arch), "0.1.0", os, arch)
                    .unwrap()
                    .is_some()
            );
        }
        assert!(
            choose(
                &release("1.0.0", "windows", "x86_64"),
                "0.1.0",
                "macos",
                "aarch64"
            )
            .unwrap()
            .is_none()
        );
    }
    #[test]
    fn refuses_untrusted_urls_missing_digests_and_oversized_installers() {
        for (field, value) in [
            (
                "browser_download_url",
                serde_json::json!("https://evil.test/setup.msi"),
            ),
            ("digest", serde_json::Value::Null),
            ("size", serde_json::json!(MAX_DOWNLOAD + 1)),
            ("size", serde_json::json!(0)),
        ] {
            let mut r = release("1.0.0", "windows", "x86_64");
            r["assets"][0][field] = value;
            assert!(choose(&r, "0.1.0", "windows", "x86_64").is_err());
        }
        for repo in [
            "owner/../repo",
            "../repo",
            "https://github.com/a/b",
            "a/b?x",
            "a/",
        ] {
            assert!(validate_repository(repo).is_err());
        }
    }
    #[test]
    fn rejects_corruption_and_truncation_before_installation() {
        let u = choose(
            &release("1.0.0", "windows", "x86_64"),
            "0.1.0",
            "windows",
            "x86_64",
        )
        .unwrap()
        .unwrap();
        assert!(verify_download(3, &Sha256::digest(b"abc"), &u.asset).is_ok());
        assert!(verify_download(2, &Sha256::digest(b"ab"), &u.asset).is_err());
        assert!(verify_download(3, &Sha256::digest(b"abd"), &u.asset).is_err());
        assert!(select(b"{", "owner/repo", "0.1.0", "windows", "x86_64").is_err());
    }
}
