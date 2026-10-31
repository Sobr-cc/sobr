//! Golden-file comparison for the characterization tests of `sobria-estimator`
//! and `sobria-team-aggregator`. One copy, included by both crates with
//! `#[path]` (SOB-150): `CARGO_MANIFEST_DIR` and `CARGO_PKG_NAME` resolve in the
//! including crate, so each crate reads its own `tests/golden/`.
//!
//! Integers and text are compared exactly, floats with a relative tolerance of
//! 1e-9: `ln` and `exp` are not guaranteed identical to the last bit on every OS
//! the CI runs on. `insta`, used by `sobria-ingest`, compares floats exactly.
//!
//! To rewrite the files, only when the change is intended:
//!
//! ```text
//! SOBRIA_UPDATE_GOLDEN=1 cargo test -p <crate>
//! $env:SOBRIA_UPDATE_GOLDEN = "1"; cargo test -p <crate>   # PowerShell
//! ```
//!
//! An update run rewrites **every** changed file of a test, then fails that
//! test **once**, listing them, so an exported variable cannot swallow a
//! regression silently. Rerun without the variable to check, and review
//! `git diff`. A test that checks several files must share one [`Goldens`];
//! [`assert_golden`] is for a test with a single file. An update run is
//! refused when `CI` is set.

use std::{fmt::Write as _, fs, path::PathBuf};

use serde::Serialize;
use serde_json::Value;

/// Environment variable that rewrites the golden files instead of comparing.
/// Only the value `1` is recognised.
const UPDATE_ENV: &str = "SOBRIA_UPDATE_GOLDEN";

/// Relative tolerance on floats.
const REL_TOL: f64 = 1e-9;

/// Maximum number of detailed differences in a failure message.
const MAX_REPORTED_DIFFS: usize = 15;

/// Replaces the non-deterministic fields (computation timestamp).
pub(super) const REDACTED: &str = "<redacted>";

/// Masks the volatile fields (`computed_at`, at any depth).
pub(super) fn redact_volatile(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if key == "computed_at" {
                    *child = Value::String(REDACTED.into());
                } else {
                    redact_volatile(child);
                }
            }
        },
        Value::Array(items) => items.iter_mut().for_each(redact_volatile),
        _ => {},
    }
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
}

/// The golden-file checks of one test. In an update run, [`Goldens::assert`]
/// rewrites each changed file and dropping the value fails the test once,
/// listing them (or only prints them if the test is already failing);
/// otherwise it compares and panics at the first difference.
pub struct Goldens {
    dir: PathBuf,
    update: bool,
    rewritten: Vec<PathBuf>,
}

impl Goldens {
    /// Reads `SOBRIA_UPDATE_GOLDEN` and `CI`; an update run is refused in CI.
    pub fn new() -> Self {
        let update = std::env::var(UPDATE_ENV).is_ok_and(|v| v == "1");
        Self::with(golden_dir(), update, std::env::var_os("CI").is_some())
    }

    pub(super) fn with(dir: PathBuf, update: bool, ci: bool) -> Self {
        assert!(
            !(update && ci),
            "{UPDATE_ENV}=1 is refused in CI: it would rewrite the references instead of comparing"
        );
        Self {
            dir,
            update,
            rewritten: Vec::new(),
        }
    }

    /// Checks `actual` against `<name>.json`. Outside an update run, a missing
    /// golden file is an error.
    pub fn assert<T: Serialize + ?Sized>(&mut self, name: &str, actual: &T) {
        let mut actual = serde_json::to_value(actual).expect("the value must be serializable");
        redact_volatile(&mut actual);
        let path = self.dir.join(format!("{name}.json"));

        if self.update {
            let text = golden_text(&actual);
            if !fs::read_to_string(&path).is_ok_and(|current| current == text) {
                fs::write(&path, text).expect("write golden file");
                self.rewritten.push(path);
            }
            return;
        }

        let raw = fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "golden file not found: {} ({e}).\nCreate it with {UPDATE_ENV}=1 cargo test -p {}",
                path.display(),
                env!("CARGO_PKG_NAME")
            )
        });
        let expected: Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("unreadable golden file {} ({e})", path.display()));

        let mut diffs = Vec::new();
        compare("$", &expected, &actual, &mut diffs);
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
}

pub(super) fn summary(rewritten: &[PathBuf]) -> String {
    let files: String = rewritten
        .iter()
        .map(|path| format!("  {}\n", path.display()))
        .collect();
    format!(
        "{} golden file(s) rewritten:\n{files}\
         Rerun WITHOUT {UPDATE_ENV} to check, and review `git diff`.",
        rewritten.len()
    )
}

impl Drop for Goldens {
    fn drop(&mut self) {
        if self.rewritten.is_empty() {
            return;
        }
        let report = summary(&self.rewritten);
        if std::thread::panicking() {
            // A second panic would abort and hide the real failure: only report.
            eprintln!("The test failed after rewriting golden files:\n{report}");
        } else {
            panic!("{report}");
        }
    }
}

/// Compares `actual` to `tests/golden/<name>.json`, for a test with a single
/// golden file. A test with several files uses one [`Goldens`] instead.
pub fn assert_golden<T: Serialize + ?Sized>(name: &str, actual: &T) {
    Goldens::new().assert(name, actual);
}

/// The text of a golden file: the rendered JSON and a final newline.
pub(super) fn golden_text(value: &Value) -> String {
    let mut text = String::new();
    render(value, 0, &mut text);
    text.push('\n');
    text
}

/// Renders indented JSON, but keeps arrays of scalars (histograms, monthly
/// series) on one line: in `git diff`, a changed histogram takes one line
/// instead of fifty.
pub(super) fn render(value: &Value, depth: usize, out: &mut String) {
    let indent = |level: usize| "  ".repeat(level);
    match value {
        Value::Object(map) if !map.is_empty() => {
            out.push_str("{\n");
            for (i, (key, child)) in map.iter().enumerate() {
                let _ = write!(out, "{}{}: ", indent(depth + 1), Value::String(key.clone()));
                render(child, depth + 1, out);
                out.push_str(if i + 1 < map.len() { ",\n" } else { "\n" });
            }
            let _ = write!(out, "{}}}", indent(depth));
        },
        Value::Array(items) if items.iter().any(|v| v.is_object() || v.is_array()) => {
            out.push_str("[\n");
            for (i, child) in items.iter().enumerate() {
                out.push_str(&indent(depth + 1));
                render(child, depth + 1, out);
                out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
            }
            let _ = write!(out, "{}]", indent(depth));
        },
        // Scalar, empty object or array, array of scalars: one line.
        flat => {
            let _ = write!(out, "{flat}");
        },
    }
}

fn is_integer_array(items: &[Value]) -> bool {
    items.iter().all(|v| v.is_i64() || v.is_u64())
}

pub(super) fn compare(path: &str, expected: &Value, actual: &Value, diffs: &mut Vec<String>) {
    match (expected, actual) {
        (Value::Number(e), Value::Number(a)) => {
            // Floats: tolerance. Anything else (integers, integer vs float):
            // exact, which reports a change of type.
            let same = match (e.as_f64(), a.as_f64()) {
                (Some(e_f), Some(a_f)) if e.is_f64() && a.is_f64() => close(e_f, a_f),
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
            if is_integer_array(e) && is_integer_array(a) {
                // Histogram or integer series: one summary line, so the
                // quantiles are not buried under 50 lines of bins.
                let mut differing = e.iter().zip(a).enumerate().filter(|(_, (x, y))| x != y);
                if let Some((first, (x, y))) = differing.next() {
                    let count = 1 + differing.count();
                    diffs.push(format!(
                        "{path}: {count} different integer value(s), \
                         the first at [{first}]: expected {x}, got {y}"
                    ));
                }
            } else {
                for (i, (e_child, a_child)) in e.iter().zip(a).enumerate() {
                    compare(&format!("{path}[{i}]"), e_child, a_child, diffs);
                }
            }
        },
        _ if expected == actual => {},
        _ => diffs.push(format!("{path}: expected {expected}, got {actual}")),
    }
}

fn close(expected: f64, actual: f64) -> bool {
    (expected - actual).abs() <= REL_TOL * expected.abs().max(actual.abs())
}
