//! Golden-file checks for the estimator tests.
//!
//! The comparator is shared with `sobria-team-aggregator` (SOB-150) and lives in
//! `test-support/golden.rs`. This module re-exports it and keeps its unit tests.

#[path = "../../../../test-support/golden.rs"]
mod shared;

pub use shared::{assert_golden, Goldens};

#[cfg(test)]
mod tests {
    use std::{fs, panic, path::PathBuf};

    use serde_json::{json, Value};

    use super::shared::{
        compare, golden_text, redact_volatile, render, summary, Goldens, REDACTED,
    };

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
        assert!(
            diffs[0].contains("2 different integer value(s)"),
            "{diffs:?}"
        );
        assert!(diffs[0].contains("[1]"), "{diffs:?}");
    }

    #[test]
    fn render_round_trips_and_keeps_scalar_arrays_on_one_line() {
        let value = json!({"b": [1, 2, 3], "a": {"x": [{"y": 1.5}], "empty": [], "none": {}}, "s": "q\"uote"});
        let mut text = String::new();
        render(&value, 0, &mut text);
        assert_eq!(
            serde_json::from_str::<Value>(&text).expect("valid JSON"),
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
            fs::write(self.file(name), golden_text(value)).expect("seed golden file");
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
        assert_eq!(dir.read("a"), golden_text(&json!({"x": 1})));
        assert_eq!(dir.read("b"), golden_text(&json!({"x": 2})));
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
        assert_eq!(dir.read("new"), golden_text(&json!({"x": 1})));
        assert!(
            dir.read("new").ends_with("}\n"),
            "a golden file ends with a newline"
        );
    }

    #[test]
    fn summary_names_every_rewritten_file() {
        let text = summary(&["a.json", "b.json"].map(PathBuf::from));

        assert!(
            text.contains("2 golden file(s) rewritten:\n  a.json\n  b.json\n"),
            "{text}"
        );
    }

    #[test]
    fn update_run_is_refused_in_ci_and_writes_nothing() {
        let dir = Scratch::new("ci");
        dir.seed("a", &json!({"x": 0}));

        let message = panic_message(|| dir.goldens(true, true).assert("a", &json!({"x": 1})))
            .expect("must be refused");

        assert!(message.contains("refused in CI"), "{message}");
        assert_eq!(dir.read("a"), golden_text(&json!({"x": 0})));
    }

    #[test]
    fn missing_golden_file_is_an_error_outside_an_update_run() {
        let dir = Scratch::new("missing");

        let message =
            panic_message(|| dir.goldens(false, false).assert("absent", &json!({"x": 1})))
                .expect("must fail");

        assert!(message.contains("not found"), "{message}");
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
        assert_eq!(dir.read("a"), golden_text(&json!({"x": 1})));
    }
}
