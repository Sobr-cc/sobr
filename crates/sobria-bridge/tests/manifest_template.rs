//! Characterization of the Native Messaging manifest shipped by the crate (SOB-11).
//!
//! The crate ships `manifest.template.json` and the `install-dev.*` scripts
//! that fill it. These tests pin the template shape and the placeholder names
//! without running the scripts. Real browser installation (per-OS paths,
//! Windows registry) lives in `sobria-app` (`bridge_install.rs`).

use std::collections::BTreeSet;

use serde_json::{json, Value};
use sobria_bridge::NATIVE_HOST_NAME;

const TEMPLATE: &str = include_str!("../manifest.template.json");
const INSTALL_SH: &str = include_str!("../scripts/install-dev.sh");
const INSTALL_PS1: &str = include_str!("../scripts/install-dev.ps1");

const EXTENSION_ID: &str = "abcdefghijklmnopabcdefghijklmnop";
const BRIDGE_PATH: &str = "/opt/sobria/sobria-bridge";

/// Sorted `{{NAME}}` placeholder names: template key order does not matter.
fn placeholders(text: &str) -> Vec<&str> {
    let mut found: Vec<&str> = text
        .split("{{")
        .skip(1)
        .filter_map(|token| token.split_once("}}"))
        .map(|(name, _)| name)
        .collect();
    found.sort_unstable();
    found
}

/// Drops comment lines: script headers mention the placeholders, which does
/// not prove the code fills them.
fn code_of(script: &str) -> String {
    script
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

fn filled_manifest() -> Value {
    let text = TEMPLATE
        .replace("{{BRIDGE_PATH}}", BRIDGE_PATH)
        .replace("{{EXTENSION_ID}}", EXTENSION_ID);
    serde_json::from_str(&text).expect("filled template is valid JSON")
}

#[test]
fn template_has_exactly_the_two_placeholders_the_install_scripts_fill() {
    assert_eq!(placeholders(TEMPLATE), ["BRIDGE_PATH", "EXTENSION_ID"]);
}

#[test]
fn both_install_scripts_fill_the_template_by_those_placeholders() {
    let sh = code_of(INSTALL_SH);
    assert!(sh.contains("manifest.template.json"), "install-dev.sh");
    assert!(sh.contains("s|{{BRIDGE_PATH}}|"), "install-dev.sh");
    assert!(sh.contains("s|{{EXTENSION_ID}}|"), "install-dev.sh");

    let ps1 = code_of(INSTALL_PS1);
    assert!(ps1.contains("manifest.template.json"), "install-dev.ps1");
    assert!(
        ps1.contains("-replace '{{BRIDGE_PATH}}'"),
        "install-dev.ps1"
    );
    assert!(
        ps1.contains("-replace '{{EXTENSION_ID}}'"),
        "install-dev.ps1"
    );
}

#[test]
fn filled_template_is_a_chrome_style_stdio_manifest() {
    let manifest = filled_manifest();
    let keys: BTreeSet<&str> = manifest
        .as_object()
        .expect("JSON object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        BTreeSet::from(["allowed_origins", "description", "name", "path", "type"])
    );
    assert_eq!(manifest["name"], NATIVE_HOST_NAME);
    assert_eq!(manifest["type"], "stdio");
    assert_eq!(manifest["path"], BRIDGE_PATH);
    assert!(!manifest["description"].as_str().unwrap().is_empty());
}

#[test]
fn allowed_origins_lists_the_single_extension_with_a_trailing_slash() {
    assert_eq!(
        filled_manifest()["allowed_origins"],
        json!([format!("chrome-extension://{EXTENSION_ID}/")])
    );
}

#[test]
fn template_has_no_firefox_allowed_extensions_key() {
    // Both scripts write this same template to the Chrome and Firefox folders,
    // so Firefox only gets the Chrome key `allowed_origins`.
    // Known defect: Firefox requires `allowed_extensions`. The fix (SOB-82)
    // will change this test.
    assert!(filled_manifest().get("allowed_extensions").is_none());
}

#[test]
fn native_host_name_follows_the_chrome_naming_rules() {
    // https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging
    // "lowercase alphanumeric, underscores and dots; no leading or trailing
    // dot; no two consecutive dots".
    assert!(!NATIVE_HOST_NAME.is_empty());
    assert!(NATIVE_HOST_NAME
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'.'));
    assert!(!NATIVE_HOST_NAME.starts_with('.') && !NATIVE_HOST_NAME.ends_with('.'));
    assert!(!NATIVE_HOST_NAME.contains(".."));
}
