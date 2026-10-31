//! Sobr.ia — auto-install of the Native Messaging manifests.
//!
//! Replaces the shell/PowerShell scripts `crates/sobria-bridge/scripts/install-dev.*`
//! for the end user: the Tauri app itself writes the
//! `com.sobria.bridge.json` files to the OS locations expected by each
//! WebExtension (Chrome / Firefox / Edge / Brave / Chromium).
//!
//! ## Supported locations
//!
//! ### macOS — `~/Library/Application Support`
//! | Browser   | Subfolder |
//! |-----------|-----------|
//! | Chrome    | `Google/Chrome/NativeMessagingHosts` |
//! | Firefox   | `Mozilla/NativeMessagingHosts` |
//! | Edge      | `Microsoft Edge/NativeMessagingHosts` |
//! | Brave     | `BraveSoftware/Brave-Browser/NativeMessagingHosts` |
//! | Chromium  | `Chromium/NativeMessagingHosts` |
//!
//! ### Linux
//! | Browser   | Path |
//! |-----------|------|
//! | Chrome    | `~/.config/google-chrome/NativeMessagingHosts` |
//! | Firefox   | `~/.mozilla/native-messaging-hosts` *(outside `~/.config`)* |
//! | Edge      | `~/.config/microsoft-edge/NativeMessagingHosts` |
//! | Brave     | `~/.config/BraveSoftware/Brave-Browser/NativeMessagingHosts` |
//! | Chromium  | `~/.config/chromium/NativeMessagingHosts` |
//!
//! ### Windows
//! The manifest file is written to `%APPDATA%\Sobria\bridge\com.sobria.bridge.json`
//! (a common path, not browser-specific) and a key
//! `HKCU\Software\<Vendor>\<Browser>\NativeMessagingHosts\com.sobria.bridge`
//! points to it (default value = absolute path of the JSON). Implemented
//! with `reg.exe ADD` / `reg.exe DELETE` (standard Windows binary, no
//! `winreg` crate added; see the patch's out-of-scope list).
//!
//! ## Security
//!
//! - The manifests are written with explicit user consent
//!   (dedicated dialog in `/parametres`).
//! - `allowed_origins` (Chrome/Edge/Brave/Chromium) restricts the bridge
//!   to a single `extension_id`.
//! - `allowed_extensions` (Firefox) restricts it to `sobria@sobr.ia` (see
//!   `extension/manifest.firefox.json`).

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

pub use sobria_bridge::NATIVE_HOST_NAME;

/// Description embedded in the JSON manifest.
pub const NATIVE_HOST_DESCRIPTION: &str = "Sobr.ia native messaging bridge";

/// Firefox add-on ID (see `extension/manifest.firefox.json` →
/// `browser_specific_settings.gecko.id`).
pub const FIREFOX_EXTENSION_ID: &str = "sobria@sobr.ia";

/// Browsers supported by the auto-install.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserKind {
    Chrome,
    Firefox,
    Edge,
    Brave,
    Chromium,
}

impl BrowserKind {
    /// Full list of the supported browsers.
    pub const ALL: [BrowserKind; 5] = [
        Self::Chrome,
        Self::Firefox,
        Self::Edge,
        Self::Brave,
        Self::Chromium,
    ];

    /// Stable snake_case identifier, the same as the serde name (logs).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Firefox => "firefox",
            Self::Edge => "edge",
            Self::Brave => "brave",
            Self::Chromium => "chromium",
        }
    }

    /// Human-readable UI name.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Chrome => "Google Chrome",
            Self::Firefox => "Mozilla Firefox",
            Self::Edge => "Microsoft Edge",
            Self::Brave => "Brave",
            Self::Chromium => "Chromium",
        }
    }

    /// `true` if this browser uses the `allowed_extensions` format (Firefox)
    /// rather than `allowed_origins` (Chrome family).
    #[must_use]
    pub const fn is_firefox(self) -> bool {
        matches!(self, Self::Firefox)
    }
}

/// Host OS. Used internally to make the functions testable on any
/// platform (the test passes the wanted OS without depending on the
/// runtime).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostOs {
    Macos,
    Linux,
    Windows,
}

impl HostOs {
    /// Current OS (read at compile time via `target_os`).
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

/// Current bridge status, exposed to the frontend by the `bridge_status` IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeStatus {
    /// Absolute path of the `sobria-bridge` binary found next to the app,
    /// or `None` if not found.
    pub bridge_path: Option<PathBuf>,
    /// Browsers whose native manifest is already installed.
    pub installed: Vec<BrowserKind>,
    /// Browsers detected on the machine (config dir present).
    pub detected: Vec<BrowserKind>,
}

/// Detects the installed browsers by checking that their user profile
/// folder exists. Indicative only: a browser without a config dir can still
/// support pairing if the user starts it later.
#[must_use]
pub fn detect_installed_browsers() -> Vec<BrowserKind> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let appdata = dirs::config_dir();
    let local = dirs::data_local_dir();
    detect_browsers_in(
        HostOs::current(),
        &home,
        appdata.as_deref(),
        local.as_deref(),
    )
}

/// Testable variant: takes the target OS and `home`/`appdata`/`localappdata`
/// explicitly. Useful to test the macOS resolution from Linux/Windows
/// with a mocked `tempdir`.
#[must_use]
pub fn detect_browsers_in(
    os: HostOs,
    home: &Path,
    appdata: Option<&Path>,
    local: Option<&Path>,
) -> Vec<BrowserKind> {
    BrowserKind::ALL
        .into_iter()
        .filter(|b| user_data_path(os, *b, home, appdata, local).is_some_and(|p| p.exists()))
        .collect()
}

/// Expected path of the browser's user-data folder. `None` if the
/// (OS, browser) pair is not supported or if `appdata`/`local` is missing.
fn user_data_path(
    os: HostOs,
    browser: BrowserKind,
    home: &Path,
    appdata: Option<&Path>,
    local: Option<&Path>,
) -> Option<PathBuf> {
    let (base, dir) = match (os, browser) {
        (HostOs::Macos, BrowserKind::Chrome) => (home, "Library/Application Support/Google/Chrome"),
        (HostOs::Macos, BrowserKind::Firefox) => (home, "Library/Application Support/Firefox"),
        (HostOs::Macos, BrowserKind::Edge) => (home, "Library/Application Support/Microsoft Edge"),
        (HostOs::Macos, BrowserKind::Brave) => (
            home,
            "Library/Application Support/BraveSoftware/Brave-Browser",
        ),
        (HostOs::Macos, BrowserKind::Chromium) => (home, "Library/Application Support/Chromium"),

        (HostOs::Linux, BrowserKind::Chrome) => (home, ".config/google-chrome"),
        (HostOs::Linux, BrowserKind::Firefox) => (home, ".mozilla/firefox"),
        (HostOs::Linux, BrowserKind::Edge) => (home, ".config/microsoft-edge"),
        (HostOs::Linux, BrowserKind::Brave) => (home, ".config/BraveSoftware/Brave-Browser"),
        (HostOs::Linux, BrowserKind::Chromium) => (home, ".config/chromium"),

        (HostOs::Windows, BrowserKind::Chrome) => (local?, "Google/Chrome/User Data"),
        (HostOs::Windows, BrowserKind::Firefox) => (appdata?, "Mozilla/Firefox/Profiles"),
        (HostOs::Windows, BrowserKind::Edge) => (local?, "Microsoft/Edge/User Data"),
        (HostOs::Windows, BrowserKind::Brave) => (local?, "BraveSoftware/Brave-Browser/User Data"),
        (HostOs::Windows, BrowserKind::Chromium) => (local?, "Chromium/User Data"),
    };
    Some(base.join(dir))
}

/// Install path of the manifest for this browser on the current OS.
pub fn manifest_install_path(browser: BrowserKind) -> Result<PathBuf> {
    let home = dirs::home_dir().context("home_dir introuvable")?;
    let appdata = dirs::config_dir();
    manifest_install_path_for(HostOs::current(), browser, &home, appdata.as_deref())
}

/// Testable variant: explicit OS. On Windows, `appdata` is required and
/// the given `appdata` is the base of `Sobria/bridge/com.sobria.bridge.json`.
pub fn manifest_install_path_for(
    os: HostOs,
    browser: BrowserKind,
    home: &Path,
    appdata: Option<&Path>,
) -> Result<PathBuf> {
    let (base, dir) = match (os, browser) {
        // ── macOS ────────────────────────────────────────────────────────────
        (HostOs::Macos, BrowserKind::Chrome) => (
            home,
            "Library/Application Support/Google/Chrome/NativeMessagingHosts",
        ),
        (HostOs::Macos, BrowserKind::Firefox) => (
            home,
            "Library/Application Support/Mozilla/NativeMessagingHosts",
        ),
        (HostOs::Macos, BrowserKind::Edge) => (
            home,
            "Library/Application Support/Microsoft Edge/NativeMessagingHosts",
        ),
        (HostOs::Macos, BrowserKind::Brave) => (
            home,
            "Library/Application Support/BraveSoftware/Brave-Browser/NativeMessagingHosts",
        ),
        (HostOs::Macos, BrowserKind::Chromium) => (
            home,
            "Library/Application Support/Chromium/NativeMessagingHosts",
        ),

        // ── Linux ────────────────────────────────────────────────────────────
        (HostOs::Linux, BrowserKind::Chrome) => {
            (home, ".config/google-chrome/NativeMessagingHosts")
        },
        (HostOs::Linux, BrowserKind::Firefox) => (home, ".mozilla/native-messaging-hosts"),
        (HostOs::Linux, BrowserKind::Edge) => (home, ".config/microsoft-edge/NativeMessagingHosts"),
        (HostOs::Linux, BrowserKind::Brave) => (
            home,
            ".config/BraveSoftware/Brave-Browser/NativeMessagingHosts",
        ),
        (HostOs::Linux, BrowserKind::Chromium) => (home, ".config/chromium/NativeMessagingHosts"),

        // ── Windows ──────────────────────────────────────────────────────────
        // One manifest for all browsers; the per-browser HKCU registry key
        // points to it.
        (HostOs::Windows, _) => (appdata.context("APPDATA introuvable")?, "Sobria/bridge"),
    };
    Ok(base.join(format!("{dir}/{NATIVE_HOST_NAME}.json")))
}

/// On Windows: name of the HKCU\Software\... key to point to the manifest.
#[must_use]
pub fn windows_registry_key(browser: BrowserKind) -> String {
    let base = match browser {
        BrowserKind::Chrome => r"HKCU\Software\Google\Chrome\NativeMessagingHosts",
        BrowserKind::Firefox => r"HKCU\Software\Mozilla\NativeMessagingHosts",
        BrowserKind::Edge => r"HKCU\Software\Microsoft\Edge\NativeMessagingHosts",
        BrowserKind::Brave => r"HKCU\Software\BraveSoftware\Brave-Browser\NativeMessagingHosts",
        BrowserKind::Chromium => r"HKCU\Software\Chromium\NativeMessagingHosts",
    };
    format!(r"{base}\{NATIVE_HOST_NAME}")
}

/// Builds the JSON content of the manifest for this browser.
///
/// - Chrome family: `allowed_origins: ["chrome-extension://<extension_id>/"]`.
/// - Firefox: `allowed_extensions: ["<FIREFOX_EXTENSION_ID>"]` (`extension_id`
///   is ignored, the `gecko.id` ID is used instead).
#[must_use]
pub fn build_manifest_json(
    browser: BrowserKind,
    bridge_path: &Path,
    extension_id: &str,
) -> serde_json::Value {
    let mut manifest = serde_json::json!({
        "name": NATIVE_HOST_NAME,
        "description": NATIVE_HOST_DESCRIPTION,
        "path": bridge_path.to_string_lossy(),
        "type": "stdio",
    });
    if browser.is_firefox() {
        manifest["allowed_extensions"] = serde_json::json!([FIREFOX_EXTENSION_ID]);
    } else {
        manifest["allowed_origins"] =
            serde_json::json!([format!("chrome-extension://{extension_id}/")]);
    }
    manifest
}

/// Writes the native manifest to the standard OS location. On Windows, also
/// writes the HKCU registry key pointing to the file (`reg.exe ADD`).
///
/// Returns the path of the written JSON file.
pub fn install_native_manifest(
    browser: BrowserKind,
    bridge_path: &Path,
    extension_id: &str,
) -> Result<PathBuf> {
    let path = manifest_install_path(browser)?;
    install_native_manifest_at(browser, bridge_path, extension_id, &path)?;
    #[cfg(target_os = "windows")]
    {
        register_windows_native_host(browser, &path)?;
    }
    Ok(path)
}

/// Testable variant: writes the manifest to `path` without touching the
/// Windows registry. Used by the tests to check the JSON content without
/// system side effects.
pub fn install_native_manifest_at(
    browser: BrowserKind,
    bridge_path: &Path,
    extension_id: &str,
    path: &Path,
) -> Result<()> {
    if !browser.is_firefox() && extension_id.trim().is_empty() {
        bail!(
            "extension_id requis pour {} (non-Firefox)",
            browser.display_name()
        );
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let manifest = build_manifest_json(browser, bridge_path, extension_id);
    let pretty = serde_json::to_string_pretty(&manifest)?;
    fs::write(path, pretty.as_bytes())
        .with_context(|| format!("write manifest {}", path.display()))?;
    tracing::info!(
        browser = browser.id(),
        path = %path.display(),
        "bridge: manifest natif installé"
    );
    Ok(())
}

/// Uninstalls the native manifest. On Windows, also deletes the HKCU key
/// (best-effort: no error if the key is missing).
pub fn uninstall_native_manifest(browser: BrowserKind) -> Result<()> {
    let path = manifest_install_path(browser)?;
    uninstall_native_manifest_at(&path)?;
    #[cfg(target_os = "windows")]
    {
        unregister_windows_native_host(browser);
    }
    Ok(())
}

/// Testable variant: only deletes the file (idempotent: no error if it is
/// missing).
pub fn uninstall_native_manifest_at(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path).with_context(|| format!("rm {}", path.display()))?;
        tracing::info!(path = %path.display(), "bridge: manifest natif supprimé");
    }
    Ok(())
}

/// Overall status: bridge binary, installed manifests, detected browsers.
#[must_use]
pub fn bridge_status(bridge_path: Option<PathBuf>) -> BridgeStatus {
    let detected = detect_installed_browsers();
    let installed = BrowserKind::ALL
        .into_iter()
        .filter(|b| manifest_install_path(*b).is_ok_and(|p| p.exists()))
        .collect();
    BridgeStatus {
        bridge_path,
        installed,
        detected,
    }
}

// ─── Windows helpers (reg.exe) ───────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn register_windows_native_host(browser: BrowserKind, manifest_path: &Path) -> Result<()> {
    let key = windows_registry_key(browser);
    let path_str = manifest_path
        .to_str()
        .context("manifest path non-UTF-8 — incompatible reg.exe")?;
    let output = std::process::Command::new("reg")
        .args(["ADD", &key, "/ve", "/d", path_str, "/f"])
        .output()
        .context("invoke reg.exe ADD")?;
    if !output.status.success() {
        bail!(
            "reg.exe ADD a échoué pour {} : {}",
            browser.display_name(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    tracing::info!(browser = browser.id(), key = %key, "bridge: clé registre HKCU écrite");
    Ok(())
}

#[cfg(target_os = "windows")]
fn unregister_windows_native_host(browser: BrowserKind) {
    let key = windows_registry_key(browser);
    let _ = std::process::Command::new("reg")
        .args(["DELETE", &key, "/f"])
        .output();
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn browser_kind_id_and_display_name_are_stable() {
        for b in BrowserKind::ALL {
            assert_ne!(b.id().len(), 0);
            assert_ne!(b.display_name().len(), 0);
        }
        assert_eq!(BrowserKind::Chrome.id(), "chrome");
        assert!(BrowserKind::Firefox.is_firefox());
        assert!(!BrowserKind::Chrome.is_firefox());
    }

    /// Normalizes Windows separators (`\`) to `/` for cross-platform
    /// assertions.
    fn norm(p: &Path) -> String {
        p.to_string_lossy().replace('\\', "/")
    }

    #[test]
    fn manifest_path_macos_chrome_matches_spec() {
        let home = Path::new("/Users/test");
        let p = manifest_install_path_for(HostOs::Macos, BrowserKind::Chrome, home, None).unwrap();
        assert_eq!(
            norm(&p),
            "/Users/test/Library/Application Support/Google/Chrome/NativeMessagingHosts/com.sobria.bridge.json"
        );
    }

    #[test]
    fn manifest_path_macos_firefox_matches_spec() {
        let home = Path::new("/Users/test");
        let p = manifest_install_path_for(HostOs::Macos, BrowserKind::Firefox, home, None).unwrap();
        assert_eq!(
            norm(&p),
            "/Users/test/Library/Application Support/Mozilla/NativeMessagingHosts/com.sobria.bridge.json"
        );
    }

    #[test]
    fn manifest_path_linux_firefox_uses_dot_mozilla() {
        // Firefox on Linux: `~/.mozilla/native-messaging-hosts/...`
        // (NOT `~/.config/mozilla/...`).
        let home = Path::new("/home/test");
        let p = manifest_install_path_for(HostOs::Linux, BrowserKind::Firefox, home, None).unwrap();
        assert_eq!(
            norm(&p),
            "/home/test/.mozilla/native-messaging-hosts/com.sobria.bridge.json"
        );
    }

    #[test]
    fn manifest_path_linux_chrome_uses_dot_config() {
        let home = Path::new("/home/test");
        let p = manifest_install_path_for(HostOs::Linux, BrowserKind::Chrome, home, None).unwrap();
        assert_eq!(
            norm(&p),
            "/home/test/.config/google-chrome/NativeMessagingHosts/com.sobria.bridge.json"
        );
    }

    #[test]
    fn manifest_path_windows_uses_common_appdata_path_for_all_browsers() {
        let home = Path::new("C:/Users/test");
        let appdata = Path::new("C:/Users/test/AppData/Roaming");
        // All Windows browsers point to the same file; the registry key
        // is what tells them apart.
        for b in BrowserKind::ALL {
            let p = manifest_install_path_for(HostOs::Windows, b, home, Some(appdata)).unwrap();
            assert!(
                p.ends_with("Sobria/bridge/com.sobria.bridge.json")
                    || p.ends_with("Sobria\\bridge\\com.sobria.bridge.json"),
                "unexpected path: {}",
                p.display()
            );
        }
    }

    #[test]
    fn manifest_path_windows_requires_appdata() {
        let home = Path::new("C:/Users/test");
        let err = manifest_install_path_for(HostOs::Windows, BrowserKind::Chrome, home, None)
            .unwrap_err();
        assert!(err.to_string().contains("APPDATA"));
    }

    #[test]
    fn windows_registry_key_format() {
        assert_eq!(
            windows_registry_key(BrowserKind::Chrome),
            r"HKCU\Software\Google\Chrome\NativeMessagingHosts\com.sobria.bridge"
        );
        assert_eq!(
            windows_registry_key(BrowserKind::Firefox),
            r"HKCU\Software\Mozilla\NativeMessagingHosts\com.sobria.bridge"
        );
        assert_eq!(
            windows_registry_key(BrowserKind::Brave),
            r"HKCU\Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\com.sobria.bridge"
        );
    }

    #[test]
    fn build_manifest_json_chrome_has_allowed_origins() {
        let bridge = Path::new("/usr/local/bin/sobria-bridge");
        let manifest = build_manifest_json(BrowserKind::Chrome, bridge, "abcdef1234567890");
        assert_eq!(manifest["name"], "com.sobria.bridge");
        assert_eq!(manifest["type"], "stdio");
        assert_eq!(manifest["path"], "/usr/local/bin/sobria-bridge");
        assert_eq!(
            manifest["allowed_origins"][0],
            "chrome-extension://abcdef1234567890/"
        );
        // Firefox-only key absent.
        assert!(manifest.get("allowed_extensions").is_none());
    }

    #[test]
    fn build_manifest_json_firefox_has_allowed_extensions_fixed_id() {
        let bridge = Path::new("/usr/local/bin/sobria-bridge");
        // The given extension_id is ignored for Firefox: the stable gecko.id is used.
        let manifest = build_manifest_json(BrowserKind::Firefox, bridge, "ignored");
        assert_eq!(manifest["allowed_extensions"][0], FIREFOX_EXTENSION_ID);
        assert!(manifest.get("allowed_origins").is_none());
    }

    #[test]
    fn install_native_manifest_at_writes_valid_json() {
        let dir = TempDir::new().unwrap();
        let path = dir
            .path()
            .join("NativeMessagingHosts/com.sobria.bridge.json");
        let bridge = Path::new("/usr/local/bin/sobria-bridge");
        install_native_manifest_at(BrowserKind::Chrome, bridge, "deadbeefdeadbeef", &path).unwrap();
        assert!(path.exists());
        let written = fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(parsed["name"], NATIVE_HOST_NAME);
        assert_eq!(
            parsed["allowed_origins"][0],
            "chrome-extension://deadbeefdeadbeef/"
        );
    }

    #[test]
    fn install_native_manifest_at_rejects_empty_extension_id_for_chrome() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.json");
        let bridge = Path::new("/bin/sobria-bridge");
        let err = install_native_manifest_at(BrowserKind::Chrome, bridge, "", &path).unwrap_err();
        assert!(err.to_string().contains("extension_id requis"));
        assert!(
            !path.exists(),
            "no file should be written on validation error"
        );
    }

    #[test]
    fn install_native_manifest_at_allows_empty_extension_id_for_firefox() {
        // Firefox ignores extension_id (uses the stable gecko.id).
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.json");
        install_native_manifest_at(BrowserKind::Firefox, Path::new("/b"), "", &path).unwrap();
        assert!(path.exists());
        let parsed: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(parsed["allowed_extensions"][0], FIREFOX_EXTENSION_ID);
    }

    #[test]
    fn uninstall_native_manifest_at_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ghost.json");
        // Missing file: no error.
        uninstall_native_manifest_at(&path).unwrap();
        // Present file: deleted.
        fs::write(&path, "{}").unwrap();
        uninstall_native_manifest_at(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn detect_browsers_in_returns_only_those_with_user_data_dir() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        // Simulates a Chrome installed on Linux.
        fs::create_dir_all(home.join(".config/google-chrome")).unwrap();
        // And a Firefox.
        fs::create_dir_all(home.join(".mozilla/firefox")).unwrap();
        let detected = detect_browsers_in(HostOs::Linux, home, None, None);
        assert!(detected.contains(&BrowserKind::Chrome));
        assert!(detected.contains(&BrowserKind::Firefox));
        assert!(!detected.contains(&BrowserKind::Edge));
        assert!(!detected.contains(&BrowserKind::Brave));
    }

    #[test]
    fn detect_browsers_in_macos_paths() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        fs::create_dir_all(home.join("Library/Application Support/Google/Chrome")).unwrap();
        fs::create_dir_all(home.join("Library/Application Support/BraveSoftware/Brave-Browser"))
            .unwrap();
        let detected = detect_browsers_in(HostOs::Macos, home, None, None);
        assert!(detected.contains(&BrowserKind::Chrome));
        assert!(detected.contains(&BrowserKind::Brave));
        assert!(!detected.contains(&BrowserKind::Firefox));
    }

    #[test]
    fn detect_browsers_in_windows_uses_local_appdata() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let appdata = tmp.path().join("Roaming");
        let local = tmp.path().join("Local");
        fs::create_dir_all(appdata.join("Mozilla/Firefox/Profiles")).unwrap();
        fs::create_dir_all(local.join("Google/Chrome/User Data")).unwrap();
        let detected = detect_browsers_in(HostOs::Windows, home, Some(&appdata), Some(&local));
        assert!(detected.contains(&BrowserKind::Chrome));
        assert!(detected.contains(&BrowserKind::Firefox));
        assert!(!detected.contains(&BrowserKind::Edge));
    }

    #[test]
    fn detect_browsers_in_returns_empty_when_no_browser_dir() {
        let tmp = TempDir::new().unwrap();
        let detected = detect_browsers_in(HostOs::Linux, tmp.path(), None, None);
        assert_eq!(detected.len(), 0);
    }

    #[test]
    fn round_trip_install_then_uninstall() {
        let dir = TempDir::new().unwrap();
        let path = dir
            .path()
            .join("Library/Application Support/Google/Chrome/NativeMessagingHosts")
            .join("com.sobria.bridge.json");
        install_native_manifest_at(
            BrowserKind::Chrome,
            Path::new("/usr/local/bin/sobria-bridge"),
            "abc123",
            &path,
        )
        .unwrap();
        assert!(path.exists());
        uninstall_native_manifest_at(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn host_os_current_returns_expected_value_for_target() {
        let os = HostOs::current();
        if cfg!(target_os = "macos") {
            assert_eq!(os, HostOs::Macos);
        } else if cfg!(target_os = "windows") {
            assert_eq!(os, HostOs::Windows);
        } else {
            assert_eq!(os, HostOs::Linux);
        }
    }
}
