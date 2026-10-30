//! Comparaison à des golden files JSON, avec tolérance sur les flottants.
//!
//! Le projet utilise déjà `insta` pour ses snapshots (`sobria-ingest`), mais
//! `insta` compare les flottants à l'identique. Or la CI tourne sur Linux,
//! macOS et Windows, et `ln` / `exp` (utilisés par les tirages log-normaux)
//! ne sont pas garantis identiques au dernier bit d'une plateforme à l'autre.
//! D'où ce comparateur : égalité exacte sur les entiers et les textes,
//! tolérance relative de 1e-9 sur les flottants — des ordres de grandeur
//! sous le moindre changement réel d'un coefficient.
//!
//! Régénérer les golden files, **uniquement si le changement est voulu**
//! (bash, puis PowerShell) :
//!
//! ```text
//! SOBRIA_UPDATE_GOLDEN=1 cargo test -p sobria-estimator --test characterization
//!
//! $env:SOBRIA_UPDATE_GOLDEN = "1"; cargo test -p sobria-estimator --test characterization
//! Remove-Item Env:SOBRIA_UPDATE_GOLDEN
//! ```
//!
//! An update run rewrites **every** changed file of a test, then fails that
//! test **once**, listing them, so an exported variable cannot swallow a
//! regression silently. Rerun without the variable to check, and review
//! `git diff`. A test that checks several files must share one [`Goldens`];
//! [`assert_golden`] is for a test with a single file.

use std::{fmt::Write as _, fs, path::PathBuf};

use serde::Serialize;
use serde_json::Value;

/// Variable d'environnement qui réécrit les golden files au lieu de comparer.
/// Seule la valeur `1` est reconnue.
const UPDATE_ENV: &str = "SOBRIA_UPDATE_GOLDEN";

/// Tolérance relative sur les flottants.
const REL_TOL: f64 = 1e-9;

/// Nombre maximal d'écarts détaillés dans le message d'échec.
const MAX_REPORTED_DIFFS: usize = 15;

/// Valeur qui remplace les champs non déterministes (horodatage du calcul).
const REDACTED: &str = "<redacted>";

/// Masque les champs volatils (`computed_at`, à toute profondeur).
fn redact_volatile(value: &mut Value) {
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
/// listing them; otherwise it compares and panics at the first difference.
pub struct Goldens {
    dir: PathBuf,
    update: bool,
    rewritten: Vec<PathBuf>,
}

impl Goldens {
    pub fn new() -> Self {
        let update = std::env::var(UPDATE_ENV).is_ok_and(|v| v == "1");
        Self::with(golden_dir(), update, std::env::var_os("CI").is_some())
    }

    fn with(dir: PathBuf, update: bool, ci: bool) -> Self {
        assert!(
            !(update && ci),
            "{UPDATE_ENV}=1 est interdit en CI : il réécrirait les références au lieu de les comparer"
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
        let mut actual = serde_json::to_value(actual).expect("la valeur doit être sérialisable");
        redact_volatile(&mut actual);
        let path = self.dir.join(format!("{name}.json"));

        if self.update {
            let mut text = String::new();
            render(&actual, 0, &mut text);
            text.push('\n');
            if !fs::read_to_string(&path).is_ok_and(|current| current == text) {
                fs::write(&path, text).expect("écriture du golden file");
                self.rewritten.push(path);
            }
            return;
        }

        let raw = fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "golden file introuvable : {} ({e}).\n\
             Pour le créer : {UPDATE_ENV}=1 cargo test -p sobria-estimator --test characterization",
                path.display()
            )
        });
        let expected: Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("golden file illisible : {} ({e})", path.display()));

        let mut diffs = Vec::new();
        compare("$", &expected, &actual, &mut diffs);
        if diffs.is_empty() {
            return;
        }

        let mut message = format!(
            "le comportement de l'estimateur a changé — {} écart(s) avec {} :\n",
            diffs.len(),
            path.display()
        );
        for diff in diffs.iter().take(MAX_REPORTED_DIFFS) {
            let _ = writeln!(message, "  {diff}");
        }
        if diffs.len() > MAX_REPORTED_DIFFS {
            let _ = writeln!(
                message,
                "  … et {} autre(s)",
                diffs.len() - MAX_REPORTED_DIFFS
            );
        }
        let _ = write!(
            message,
            "Si le changement est voulu, régénérer avec {UPDATE_ENV}=1 \
         et le justifier dans la PR ; sinon, c'est une régression."
        );
        panic!("{message}");
    }
}

impl Drop for Goldens {
    fn drop(&mut self) {
        // Never panic while unwinding: that would abort and hide the real failure.
        if self.rewritten.is_empty() || std::thread::panicking() {
            return;
        }
        let files: String = self
            .rewritten
            .iter()
            .map(|path| format!("  {}\n", path.display()))
            .collect();
        panic!(
            "{} golden file(s) rewritten:\n{files}\
             Rerun WITHOUT {UPDATE_ENV} to check, and review `git diff`.",
            self.rewritten.len()
        );
    }
}

/// Compares `actual` to `tests/golden/<name>.json`, for a test with a single
/// golden file. A test with several files uses one [`Goldens`] instead.
pub fn assert_golden<T: Serialize + ?Sized>(name: &str, actual: &T) {
    Goldens::new().assert(name, actual);
}

/// Rend le JSON indenté, mais garde les tableaux de scalaires (histogrammes,
/// séries mensuelles) sur une seule ligne : dans `git diff`, un histogramme
/// modifié tient en une ligne au lieu de cinquante.
fn render(value: &Value, depth: usize, out: &mut String) {
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
        // Scalaire, objet ou tableau vide, tableau de scalaires : une ligne.
        flat => {
            let _ = write!(out, "{flat}");
        },
    }
}

fn is_integer_array(items: &[Value]) -> bool {
    items.iter().all(|v| v.is_i64() || v.is_u64())
}

fn compare(path: &str, expected: &Value, actual: &Value, diffs: &mut Vec<String>) {
    match (expected, actual) {
        (Value::Number(e), Value::Number(a)) => {
            // Flottants : tolérance. Sinon (entiers, ou entier contre
            // flottant) : égalité stricte, qui signale un changement de type.
            let same = match (e.as_f64(), a.as_f64()) {
                (Some(e_f), Some(a_f)) if e.is_f64() && a.is_f64() => close(e_f, a_f),
                _ => e == a,
            };
            if !same {
                diffs.push(format!("{path} : attendu {e}, obtenu {a}"));
            }
        },
        (Value::Object(e), Value::Object(a)) => {
            for (key, e_child) in e {
                let child_path = format!("{path}.{key}");
                match a.get(key) {
                    Some(a_child) => compare(&child_path, e_child, a_child, diffs),
                    None => diffs.push(format!("{child_path} : champ disparu")),
                }
            }
            for key in a.keys().filter(|k| !e.contains_key(*k)) {
                diffs.push(format!("{path}.{key} : champ apparu"));
            }
        },
        (Value::Array(e), Value::Array(a)) => {
            if e.len() != a.len() {
                diffs.push(format!(
                    "{path} : longueur attendue {}, obtenue {}",
                    e.len(),
                    a.len()
                ));
            }
            if is_integer_array(e) && is_integer_array(a) {
                // Histogramme ou série d'entiers : un seul écart récapitulatif,
                // pour ne pas noyer les quantiles sous 50 lignes de classes.
                let mut differing = e.iter().zip(a).enumerate().filter(|(_, (x, y))| x != y);
                if let Some((first, (x, y))) = differing.next() {
                    let count = 1 + differing.count();
                    diffs.push(format!(
                        "{path} : {count} valeur(s) entière(s) différente(s), \
                         la première en [{first}] : attendu {x}, obtenu {y}"
                    ));
                }
            } else {
                for (i, (e_child, a_child)) in e.iter().zip(a).enumerate() {
                    compare(&format!("{path}[{i}]"), e_child, a_child, diffs);
                }
            }
        },
        _ if expected == actual => {},
        _ => diffs.push(format!("{path} : attendu {expected}, obtenu {actual}")),
    }
}

fn close(expected: f64, actual: f64) -> bool {
    (expected - actual).abs() <= REL_TOL * expected.abs().max(actual.abs())
}

#[cfg(test)]
mod tests {
    use std::panic;

    use serde_json::json;

    use super::*;

    fn diffs_between(expected: &Value, actual: &Value) -> Vec<String> {
        let mut diffs = Vec::new();
        compare("$", expected, actual, &mut diffs);
        diffs
    }

    #[test]
    fn floats_within_tolerance_are_equal() {
        assert!(diffs_between(&json!({"x": 1.0}), &json!({"x": 1.0 + 1e-12})).is_empty());
    }

    #[test]
    fn floats_beyond_tolerance_differ() {
        assert_eq!(
            diffs_between(&json!({"x": 1.0}), &json!({"x": 1.0001})).len(),
            1
        );
    }

    #[test]
    fn integers_are_compared_exactly() {
        assert_eq!(
            diffs_between(&json!({"n": 100}), &json!({"n": 101})).len(),
            1
        );
    }

    #[test]
    fn integer_turning_into_float_is_reported() {
        assert_eq!(
            diffs_between(&json!({"n": 100}), &json!({"n": 100.0})).len(),
            1
        );
    }

    #[test]
    fn structural_changes_are_reported() {
        let diffs = diffs_between(
            &json!({"a": 1, "b": [1, 2]}),
            &json!({"a": 1, "b": [1], "c": 3}),
        );
        assert_eq!(diffs.len(), 2, "{diffs:?}");
    }

    #[test]
    fn integer_arrays_are_summarised_in_one_line() {
        let expected = json!({"counts": [1, 2, 3, 4, 5]});
        let actual = json!({"counts": [1, 9, 3, 8, 5]});
        let diffs = diffs_between(&expected, &actual);
        assert_eq!(diffs.len(), 1, "{diffs:?}");
        assert!(diffs[0].contains("2 valeur(s)"), "{diffs:?}");
        assert!(diffs[0].contains("[1]"), "{diffs:?}");
    }

    #[test]
    fn render_round_trips_and_keeps_scalar_arrays_on_one_line() {
        let value = json!({"b": [1, 2, 3], "a": {"x": [{"y": 1.5}], "empty": [], "none": {}}, "s": "q\"uote"});
        let mut text = String::new();
        render(&value, 0, &mut text);
        assert_eq!(
            serde_json::from_str::<Value>(&text).expect("JSON valide"),
            value
        );
        assert!(text.contains("\"b\": [1,2,3]"), "{text}");
        assert!(text.contains("\"empty\": []"), "{text}");
    }

    #[test]
    fn computed_at_is_redacted_at_any_depth() {
        let mut value =
            json!({"computed_at": "2026-01-01T00:00:00Z", "inner": [{"computed_at": 1}]});
        redact_volatile(&mut value);
        assert_eq!(value["computed_at"], REDACTED);
        assert_eq!(value["inner"][0]["computed_at"], REDACTED);
    }

    /// A scratch directory removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("sobria-golden-{}-{tag}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("scratch directory");
            Self(dir)
        }

        fn file(&self, name: &str) -> PathBuf {
            self.0.join(format!("{name}.json"))
        }

        fn seed(&self, name: &str, value: &Value) {
            fs::write(self.file(name), rendered(value)).expect("seed golden file");
        }

        fn read(&self, name: &str) -> String {
            fs::read_to_string(self.file(name)).expect("golden file")
        }

        fn goldens(&self, update: bool, ci: bool) -> Goldens {
            Goldens::with(self.0.clone(), update, ci)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn rendered(value: &Value) -> String {
        let mut text = String::new();
        render(value, 0, &mut text);
        text.push('\n');
        text
    }

    /// The panic message of `f`, or `None` if it returns normally.
    fn panic_message(f: impl FnOnce()) -> Option<String> {
        panic::catch_unwind(panic::AssertUnwindSafe(f))
            .err()
            .map(|payload| {
                payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                    .unwrap_or_default()
            })
    }

    #[test]
    fn update_run_rewrites_every_changed_file_then_fails_once() {
        let dir = Scratch::new("rewrite-all");
        dir.seed("a", &json!({"x": 0}));
        dir.seed("b", &json!({"x": 0}));
        dir.seed("c", &json!({"x": 3}));

        let message = panic_message(|| {
            let mut goldens = dir.goldens(true, false);
            goldens.assert("a", &json!({"x": 1}));
            goldens.assert("b", &json!({"x": 2}));
            goldens.assert("c", &json!({"x": 3}));
        })
        .expect("an update run must fail");

        assert!(message.contains("2 golden file(s) rewritten"), "{message}");
        assert!(
            message.contains("a.json") && message.contains("b.json"),
            "{message}"
        );
        assert!(!message.contains("c.json"), "{message}");
        assert_eq!(dir.read("a"), rendered(&json!({"x": 1})));
        assert_eq!(dir.read("b"), rendered(&json!({"x": 2})));
    }

    #[test]
    fn rerun_without_the_variable_passes_after_an_update_run() {
        let dir = Scratch::new("rerun");
        dir.seed("a", &json!({"x": 0}));
        dir.seed("b", &json!({"x": 0}));
        let _ = panic_message(|| {
            let mut goldens = dir.goldens(true, false);
            goldens.assert("a", &json!({"x": 1}));
            goldens.assert("b", &json!({"x": 2}));
        });

        let rerun = panic_message(|| {
            let mut goldens = dir.goldens(false, false);
            goldens.assert("a", &json!({"x": 1}));
            goldens.assert("b", &json!({"x": 2}));
        });

        assert_eq!(rerun, None);
    }

    #[test]
    fn update_run_with_nothing_to_rewrite_passes() {
        let dir = Scratch::new("nothing-to-rewrite");
        dir.seed("a", &json!({"x": 1}));

        let outcome = panic_message(|| dir.goldens(true, false).assert("a", &json!({"x": 1})));

        assert_eq!(outcome, None);
    }

    #[test]
    fn update_run_creates_a_missing_file_then_fails() {
        let dir = Scratch::new("create");

        let message = panic_message(|| dir.goldens(true, false).assert("new", &json!({"x": 1})))
            .expect("an update run must fail");

        assert!(message.contains("1 golden file(s) rewritten"), "{message}");
        assert_eq!(dir.read("new"), rendered(&json!({"x": 1})));
    }

    #[test]
    fn update_run_is_refused_in_ci_and_writes_nothing() {
        let dir = Scratch::new("ci");
        dir.seed("a", &json!({"x": 0}));

        let message = panic_message(|| dir.goldens(true, true).assert("a", &json!({"x": 1})))
            .expect("must be refused");

        assert!(message.contains("interdit en CI"), "{message}");
        assert_eq!(dir.read("a"), rendered(&json!({"x": 0})));
    }

    #[test]
    fn missing_golden_file_is_an_error_outside_an_update_run() {
        let dir = Scratch::new("missing");

        let message =
            panic_message(|| dir.goldens(false, false).assert("absent", &json!({"x": 1})))
                .expect("must fail");

        assert!(message.contains("introuvable"), "{message}");
        assert!(!dir.file("absent").exists());
    }

    #[test]
    fn update_failure_does_not_mask_a_panic_of_the_test() {
        let dir = Scratch::new("masking");

        let message = panic_message(|| {
            let mut goldens = dir.goldens(true, false);
            goldens.assert("a", &json!({"x": 1}));
            panic!("the test itself failed");
        })
        .expect("must fail");

        assert_eq!(message, "the test itself failed");
    }
}
