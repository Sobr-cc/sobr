//! JSON shape of the shared types, pinned by golden files and read back.

use std::{fmt::Debug, path::Path};

use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use sobria_core::{
    ContextOverhead, DistributionBins, EmpreinteMethod, Equivalent, EstimationRequest,
    EstimationResult, Hypothesis, Indicator, IndicatorValue, InputModality, ModuleId, Persona,
    UncertaintyInterval,
};

use crate::golden::Goldens;

pub fn utc(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .unwrap()
        .with_timezone(&Utc)
}

pub fn modalities() -> Vec<InputModality> {
    vec![
        InputModality::Text,
        InputModality::VisionLow { image_count: 2 },
        InputModality::VisionHigh {
            image_count: 1,
            avg_width: 1024,
            avg_height: 768,
        },
        InputModality::Document { page_count: 12 },
        InputModality::AudioInput {
            duration_seconds: 30,
        },
    ]
}

pub fn minimal_request() -> EstimationRequest {
    EstimationRequest {
        model_id: "gpt-4o-mini".into(),
        tokens_in: 23,
        tokens_out_estimated: 720,
        datacenter_id: None,
        timestamp: utc("2026-05-12T14:32:08Z"),
        modalities: Vec::new(),
        overhead: ContextOverhead::default(),
    }
}

pub fn full_request() -> EstimationRequest {
    EstimationRequest {
        datacenter_id: Some("openai-us-east-virginia".into()),
        modalities: modalities(),
        overhead: ContextOverhead::new(2000, 800, 150, 3000),
        ..minimal_request()
    }
}

fn interval(p5: f64, p50: f64, p95: f64) -> UncertaintyInterval {
    UncertaintyInterval::new(p5, p50, p95).unwrap()
}

fn full_result() -> EstimationResult {
    EstimationResult {
        method: EmpreinteMethod::EcoLogits,
        request: full_request(),
        indicators: vec![
            IndicatorValue {
                indicator: Indicator::Co2Eq,
                interval: interval(1.68, 2.14, 2.74),
                unit: "gCO2eq".into(),
                bins: Some(DistributionBins {
                    min: 1.5,
                    max: 3.0,
                    counts: vec![4, 10, 25, 10, 1],
                }),
            },
            IndicatorValue {
                indicator: Indicator::Energy,
                interval: interval(5.5, 7.25, 9.5),
                unit: "Wh".into(),
                bins: None,
            },
            IndicatorValue {
                indicator: Indicator::Water,
                interval: interval(0.012, 0.02, 0.031),
                unit: "L".into(),
                bins: None,
            },
            IndicatorValue {
                indicator: Indicator::CriticalMetals,
                interval: interval(0.5, 0.75, 1.25),
                unit: "mg".into(),
                bins: None,
            },
            IndicatorValue {
                indicator: Indicator::Cost,
                interval: interval(0.0001, 0.0002, 0.0004),
                unit: "EUR".into(),
                bins: None,
            },
        ],
        equivalents: vec![
            Equivalent {
                label: "km en voiture thermique".into(),
                value: 0.017,
                source: "ADEME 2025".into(),
            },
            Equivalent {
                label: "heures d'ampoule LED".into(),
                value: 0.5,
                source: "ADEME 2025".into(),
            },
        ],
        hypotheses: vec![
            Hypothesis {
                key: "epsilon_decode_mJ_per_token".into(),
                value: json!(1.8),
                source: "HF AI Energy Score 2026".into(),
            },
            Hypothesis {
                key: "pue".into(),
                value: json!({"p5": 1.1, "p50": 1.2, "p95": 1.4}),
                source: "Uptime Institute 2024".into(),
            },
            Hypothesis {
                key: "mode".into(),
                value: json!("expert"),
                source: "test".into(),
            },
        ],
        computed_at: utc("2026-05-12T14:32:09.250Z"),
        seed: 42,
    }
}

fn golden_text(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{name}.json"));
    std::fs::read_to_string(path).unwrap()
}

/// Compares the serialized value with `tests/golden/<name>.json`, then reads the file back.
fn assert_shape<T: Serialize + DeserializeOwned + PartialEq + Debug>(name: &str, value: &T) {
    Goldens::new().assert(name, value);

    let back: T = serde_json::from_str(&golden_text(name)).unwrap();
    assert_eq!(&back, value);
}

#[test]
fn an_estimation_request_with_every_field_has_the_golden_shape() {
    assert_shape("estimation_request_full", &full_request());
}

#[test]
fn an_estimation_request_leaves_out_its_empty_optional_fields() {
    assert_shape("estimation_request_minimal", &minimal_request());
}

#[test]
fn every_input_modality_variant_has_the_golden_shape() {
    assert_shape("input_modalities", &modalities());
}

#[test]
fn a_context_overhead_has_the_golden_shape() {
    let overheads = vec![
        ContextOverhead::new(2000, 800, 150, 3000),
        ContextOverhead::default(),
    ];

    assert_shape("context_overheads", &overheads);
}

#[test]
fn an_uncertainty_interval_has_the_golden_shape() {
    assert_shape("uncertainty_interval", &interval(1.68, 2.14, 2.74));
}

#[test]
fn an_estimation_result_has_the_golden_shape() {
    let result = full_result();
    // The comparator masks `computed_at`, so its text is pinned here.
    Goldens::new().assert("estimation_result", &result);
    let written = serde_json::to_value(&result).unwrap();
    assert_eq!(written["computed_at"], "2026-05-12T14:32:09.250Z");

    let mut golden: Value = serde_json::from_str(&golden_text("estimation_result")).unwrap();
    golden["computed_at"] = written["computed_at"].clone();
    let back: EstimationResult = serde_json::from_value(golden).unwrap();
    assert_eq!(back, result);
}

#[test]
fn the_identifiers_of_personas_modules_methods_and_indicators_have_the_golden_spelling() {
    let indicators = [
        Indicator::Co2Eq,
        Indicator::Energy,
        Indicator::Water,
        Indicator::CriticalMetals,
        Indicator::Cost,
    ];
    let identifiers = json!({
        "personas": Persona::all(),
        "modules": ModuleId::all(),
        "methods": EmpreinteMethod::all(),
        "indicators": indicators,
    });

    Goldens::new().assert("identifiers", &identifiers);

    let golden: Value = serde_json::from_str(&golden_text("identifiers")).unwrap();
    let back = |key: &str| golden[key].clone();
    assert_eq!(
        serde_json::from_value::<Vec<Persona>>(back("personas")).unwrap(),
        Persona::all()
    );
    assert_eq!(
        serde_json::from_value::<Vec<ModuleId>>(back("modules")).unwrap(),
        ModuleId::all()
    );
    assert_eq!(
        serde_json::from_value::<Vec<EmpreinteMethod>>(back("methods")).unwrap(),
        EmpreinteMethod::all()
    );
    assert_eq!(
        serde_json::from_value::<Vec<Indicator>>(back("indicators")).unwrap(),
        indicators
    );
}

/// The comparator sorts keys, so the golden files do not pin the order of the written text,
/// which the audit ledger stores and hashes.
fn assert_keys_in_order(text: &str, keys: &[&str]) {
    let mut from = 0;
    for key in keys {
        let found = text[from..]
            .find(&format!("\"{key}\":"))
            .unwrap_or_else(|| panic!("{key} not found in order in {text}"));
        from += found + key.len() + 3;
    }
}

#[test]
fn the_written_text_keeps_the_field_order_of_the_structs() {
    let request = serde_json::to_string(&full_request()).unwrap();
    assert_keys_in_order(
        &request,
        &[
            "model_id",
            "tokens_in",
            "tokens_out_estimated",
            "datacenter_id",
            "timestamp",
            "modalities",
            "overhead",
            "system_prompt_tokens",
            "tools_definition_tokens",
            "memory_tokens",
            "thinking_tokens_p50",
        ],
    );

    let result = serde_json::to_string(&full_result()).unwrap();
    assert_keys_in_order(
        &result,
        &[
            "method",
            "request",
            "indicators",
            "indicator",
            "interval",
            "p5",
            "p50",
            "p95",
            "unit",
            "bins",
            "min",
            "max",
            "counts",
            "equivalents",
            "label",
            "value",
            "source",
            "hypotheses",
            "key",
            "computed_at",
            "seed",
        ],
    );
}

#[test]
fn each_input_modality_is_written_with_its_kind_first() {
    let written: Vec<String> = modalities()
        .iter()
        .map(|m| serde_json::to_string(m).unwrap())
        .collect();

    assert_eq!(
        written,
        [
            r#"{"kind":"text"}"#,
            r#"{"kind":"vision_low","image_count":2}"#,
            r#"{"kind":"vision_high","image_count":1,"avg_width":1024,"avg_height":768}"#,
            r#"{"kind":"document","page_count":12}"#,
            r#"{"kind":"audio_input","duration_seconds":30}"#,
        ]
    );
}

#[test]
fn the_default_units_of_the_metals_and_cost_indicators_are_pinned() {
    assert_eq!(Indicator::CriticalMetals.default_unit(), "mg");
    assert_eq!(Indicator::Cost.default_unit(), "EUR");
}
