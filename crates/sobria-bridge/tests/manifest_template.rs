//! Caractérisation (SOB-11) du manifest Native Messaging fourni par le crate.
//!
//! Le crate ne contient aucun code de vérification de manifest : ce qu'il
//! livre, c'est `manifest.template.json` et les scripts `install-dev.*` qui
//! le remplissent. Ces tests figent la forme du modèle et les noms des jetons
//! que les scripts remplacent ; ils n'exécutent pas les scripts. L'installation
//! réelle dans les navigateurs (chemins par OS, registre Windows) est dans
//! `sobria-app` (`bridge_install.rs`).
//!
//! `manifest/com.sobria.bridge.json.tmpl` n'est référencé par aucun script ni
//! aucun code Rust : il n'est donc pas figé ici.

use std::collections::BTreeSet;

use serde_json::{json, Value};
use sobria_bridge::NATIVE_HOST_NAME;

const TEMPLATE: &str = include_str!("../manifest.template.json");
const INSTALL_SH: &str = include_str!("../scripts/install-dev.sh");
const INSTALL_PS1: &str = include_str!("../scripts/install-dev.ps1");

const EXTENSION_ID: &str = "abcdefghijklmnopabcdefghijklmnop";
const BRIDGE_PATH: &str = "/opt/sobria/sobria-bridge";

/// Noms des jetons `{{NOM}}` présents dans `text`, triés (l'ordre des clés du
/// modèle n'a pas d'importance).
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

/// Lignes de `script` hors commentaires : les en-têtes des scripts citent déjà
/// les jetons, ce qui ne prouve pas qu'ils les remplissent.
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
    serde_json::from_str(&text).expect("le modèle rempli est un JSON valide")
}

#[test]
fn template_has_exactly_the_two_placeholders_the_install_scripts_fill() {
    assert_eq!(placeholders(TEMPLATE), ["BRIDGE_PATH", "EXTENSION_ID"]);
}

#[test]
fn both_install_scripts_fill_the_template_by_those_placeholders() {
    // Chaque script lit le modèle puis substitue les deux jetons dans son code
    // (`sed` côté shell, `-replace` côté PowerShell), commentaires exclus.
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
        .expect("objet JSON")
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
    // Particularité actuelle : les deux scripts écrivent ce même modèle dans le
    // dossier Chrome et dans le dossier Firefox, qui ne reçoit donc que
    // `allowed_origins` (clé Chrome).
    assert!(filled_manifest().get("allowed_extensions").is_none());
}

#[test]
fn native_host_name_follows_the_chrome_naming_rules() {
    // https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging
    // « lowercase alphanumeric, underscores and dots; no leading or trailing
    // dot; no two consecutive dots ».
    assert!(!NATIVE_HOST_NAME.is_empty());
    assert!(NATIVE_HOST_NAME
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'.'));
    assert!(!NATIVE_HOST_NAME.starts_with('.') && !NATIVE_HOST_NAME.ends_with('.'));
    assert!(!NATIVE_HOST_NAME.contains(".."));
}
