//! Golden-file comparison for the characterization tests.
//!
//! Numbers are compared with a small relative tolerance so that the files
//! stay valid on every OS the CI runs on. To rewrite the files, only when the
//! change is intended:
//!
//! ```text
//! SOBRIA_UPDATE_GOLDEN=1 cargo test -p sobria-team-aggregator --no-fail-fast
//! $env:SOBRIA_UPDATE_GOLDEN = "1"; cargo test -p sobria-team-aggregator --no-fail-fast   # PowerShell
//! ```
//!
//! A test that rewrites a file fails on purpose, so that a variable left
//! exported cannot hide a regression. Run again without it to check.

use std::{fmt::Write as _, fs, path::PathBuf};

use serde_json::Value;

const UPDATE_ENV: &str = "SOBRIA_UPDATE_GOLDEN";
const REL_TOL: f64 = 1e-9;
const MAX_REPORTED_DIFFS: usize = 15;

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(format!("{name}.json"))
}

/// Compares `actual` with `tests/golden/<name>.json`.
pub fn assert_golden(name: &str, actual: &Value) {
    let path = golden_path(name);

    if std::env::var(UPDATE_ENV).is_ok_and(|v| v == "1") {
        assert!(
            std::env::var_os("CI").is_none(),
            "{UPDATE_ENV}=1 is refused in CI: it would rewrite the references instead of comparing"
        );
        let text = format!(
            "{}\n",
            serde_json::to_string_pretty(actual).expect("render")
        );
        if fs::read_to_string(&path).is_ok_and(|current| current == text) {
            return;
        }
        fs::write(&path, text).expect("write golden file");
        panic!(
            "golden file rewritten: {}\nRun again WITHOUT {UPDATE_ENV}, then review `git diff`.",
            path.display()
        );
    }

    let raw = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "golden file not found: {} ({e}).\nCreate it with {UPDATE_ENV}=1 cargo test -p sobria-team-aggregator",
            path.display()
        )
    });
    let expected: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("unreadable golden file {} ({e})", path.display()));

    let mut diffs = Vec::new();
    compare("$", &expected, actual, &mut diffs);
    if diffs.is_empty() {
        return;
    }
    let mut message = format!(
        "behavior changed: {} difference(s) with {}\n",
        diffs.len(),
        path.display()
    );
    for diff in diffs.iter().take(MAX_REPORTED_DIFFS) {
        let _ = writeln!(message, "  {diff}");
    }
    if diffs.len() > MAX_REPORTED_DIFFS {
        let _ = writeln!(
            message,
            "  ... and {} more",
            diffs.len() - MAX_REPORTED_DIFFS
        );
    }
    let _ = write!(
        message,
        "If the change is intended, regenerate with {UPDATE_ENV}=1 and say so in the PR; otherwise it is a regression."
    );
    panic!("{message}");
}

fn compare(path: &str, expected: &Value, actual: &Value, diffs: &mut Vec<String>) {
    match (expected, actual) {
        (Value::Number(e), Value::Number(a)) => {
            // Floats: tolerance. Anything else (integers, integer vs float): exact.
            let same = match (e.as_f64(), a.as_f64()) {
                (Some(e_f), Some(a_f)) if e.is_f64() && a.is_f64() => {
                    (e_f - a_f).abs() <= REL_TOL * e_f.abs().max(a_f.abs())
                },
                _ => e == a,
            };
            if !same {
                diffs.push(format!("{path}: expected {e}, got {a}"));
            }
        },
        (Value::Object(e), Value::Object(a)) => {
            for (key, e_child) in e {
                let child_path = format!("{path}.{key}");
                match a.get(key) {
                    Some(a_child) => compare(&child_path, e_child, a_child, diffs),
                    None => diffs.push(format!("{child_path}: field disappeared")),
                }
            }
            for key in a.keys().filter(|k| !e.contains_key(*k)) {
                diffs.push(format!("{path}.{key}: field appeared"));
            }
        },
        (Value::Array(e), Value::Array(a)) => {
            if e.len() != a.len() {
                diffs.push(format!(
                    "{path}: expected length {}, got {}",
                    e.len(),
                    a.len()
                ));
            }
            for (i, (e_child, a_child)) in e.iter().zip(a).enumerate() {
                compare(&format!("{path}[{i}]"), e_child, a_child, diffs);
            }
        },
        _ if expected == actual => {},
        _ => diffs.push(format!("{path}: expected {expected}, got {actual}")),
    }
}
