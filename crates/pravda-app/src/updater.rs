//! Authenticated, opt-in checks for cryptographically signed stable releases.
//! Not compiled into unsigned Windows packages, diagnostic tools or Linux.
//! The official Tauri updater owns signature verification; this module
//! NEVER downloads arbitrary URLs or installs an unverified artifact.
use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

const GITHUB_RELEASE_ASSETS: &str =
    "https://github.com/suvorovpravda/Pravda-Bot/releases/download/v";
const MAX_CHECK_SECONDS: u64 = 25;

pub(crate) fn configured() -> bool {
    option_env!("PRAVDA_UPDATER_PUBKEY")
        .is_some_and(|public_key| public_key.trim().len() > 40)
}

pub(crate) fn register(app: &mut tauri::App) -> tauri::Result<()> {
    // No private signing key is ever embedded or read from disk. If the
    // public key is absent, do not register the updater at all.
    if let Some(public_key) =
        option_env!("PRAVDA_UPDATER_PUBKEY").filter(|value| value.trim().len() > 40)
    {
        app.handle()
            .plugin(tauri_plugin_updater::Builder::new().pubkey(public_key).build())?;
    }
    Ok(())
}

fn stable_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && (part == &"0" || !part.starts_with('0'))
                && part.bytes().all(|c| c.is_ascii_digit())
        })
}

fn trusted_release_asset(version: &str, url: &str) -> bool {
    if !stable_version(version) {
        return false;
    }
    let base = format!("{GITHUB_RELEASE_ASSETS}{version}/");
    let Some(file) = url.strip_prefix(&base) else {
        return false;
    };
    // Restrict to an actual Windows x64 MSI/NSIS, not an arbitrary executable,
    // query-bearing redirect, local URL or path traversal.
    !file.is_empty()
        && file.len() <= 160
        && file.bytes().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-')
        })
        && (file.ends_with(".msi") || file.ends_with("-setup.exe"))
        && !file.contains("..")
}

pub(crate) async fn check(app: AppHandle) -> Result<String, String> {
    if !configured() {
        return Ok("disabled".to_string());
    }
    let updater = app
        .updater_builder()
        .timeout(Duration::from_secs(MAX_CHECK_SECONDS))
        .build()
        .map_err(|_| "UPDATE_NOT_AVAILABLE".to_string())?;
    let remote = updater
        .check()
        .await
        .map_err(|_| "UPDATE_CHECK_FAILED".to_string())?;
    match remote {
        Some(update) => {
            if !trusted_release_asset(&update.version, update.download_url.as_str()) {
                return Err("UPDATE_SOURCE_REJECTED".to_string());
            }
            // Never return a URL, signature, arbitrary release notes or
            // authentication material to WebView2.
            Ok(format!("available|{}", update.version))
        }
        None => Ok("current".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{stable_version, trusted_release_asset};
    #[test]
    fn updater_accepts_only_stable_semver() {
        assert!(stable_version("0.3.5"));
        assert!(stable_version("10.30.2"));
        for value in ["0.3.5-rc.1", "01.3.5", "../1", "2.3", "1.2.3.4", ""] {
            assert!(!stable_version(value));
        }
    }
    #[test]
    fn updater_rejects_untrusted_download_urls() {
        assert!(trusted_release_asset(
            "0.3.5",
            "https://github.com/suvorovpravda/Pravda-Bot/releases/download/v0.3.5/Pravda-Bot_0.3.5_x64_en-US.msi"
        ));
        assert!(trusted_release_asset(
            "0.3.5",
            "https://github.com/suvorovpravda/Pravda-Bot/releases/download/v0.3.5/Pravda-Bot_0.3.5_x64-setup.exe"
        ));
        for url in [
            "https://evil.example/releases/download/v0.3.5/app.msi",
            "http://github.com/suvorovpravda/Pravda-Bot/releases/download/v0.3.5/app.msi",
            "https://github.com/suvorovpravda/Pravda-Bot/releases/download/v0.3.4/app.msi",
            "https://github.com/suvorovpravda/Pravda-Bot/releases/download/v0.3.5/../app.msi",
            "https://github.com/suvorovpravda/Pravda-Bot/releases/download/v0.3.5/app.msi?token=test",
            "https://github.com/suvorovpravda/Pravda-Bot/releases/download/v0.3.5/app.exe",
        ] {
            assert!(!trusted_release_asset("0.3.5", url));
        }
    }
}
