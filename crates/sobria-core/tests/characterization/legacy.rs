//! Payloads written by older versions must stay readable: the audit ledger stores each result
//! as JSON. Unknown fields are accepted and dropped; wrong or missing required ones are refused.

use serde_json::json;
use sobria_core::{
    EmpreinteMethod, EstimationRequest, EstimationResult, Indicator, IndicatorValue, InputModality,
    UncertaintyInterval,
};

use crate::shapes::{full_result, minimal_request, utc};

const MINIMAL_REQUEST: &str = r#"{"model_id":"gpt-4o-mini","tokens_in":23,"tokens_out_estimated":720,"timestamp":"2026-05-12T14:32:08Z"}"#;

fn error_of<T: serde::de::DeserializeOwned>(text: &str) -> String {
    match serde_json::from_str::<T>(text) {
        Ok(_) => panic!("expected an error for {text}"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn a_request_with_every_optional_field_absent_is_read_and_written_back_unchanged() {
    let request: EstimationRequest = serde_json::from_str(MINIMAL_REQUEST).unwrap();

    assert_eq!(request, minimal_request());
    assert_eq!(serde_json::to_string(&request).unwrap(), MINIMAL_REQUEST);
}

#[test]
fn a_request_with_explicit_empty_optional_fields_is_read_the_same() {
    let text = r#"{"model_id":"gpt-4o-mini","tokens_in":23,"tokens_out_estimated":720,"datacenter_id":null,"timestamp":"2026-05-12T14:32:08Z","modalities":[],"overhead":{}}"#;

    let request: EstimationRequest = serde_json::from_str(text).unwrap();

    assert_eq!(request, minimal_request());
}

const LEGACY_RESULT: &str = r#"{
    "request": {"model_id": "gpt-4o-mini", "tokens_in": 10, "tokens_out_estimated": 50,
                "timestamp": "2026-04-01T10:00:00Z"},
    "indicators": [{"indicator": "co2_eq", "interval": {"p5": 1.0, "p50": 2.0, "p95": 3.0},
                    "unit": "gCO2eq"}],
    "equivalents": [],
    "hypotheses": [],
    "computed_at": "2026-04-01T10:00:00Z",
    "seed": 42
}"#;

fn legacy_result() -> EstimationResult {
    EstimationResult {
        method: EmpreinteMethod::AfnorSobria,
        request: EstimationRequest {
            tokens_in: 10,
            tokens_out_estimated: 50,
            timestamp: utc("2026-04-01T10:00:00Z"),
            ..minimal_request()
        },
        indicators: vec![IndicatorValue {
            indicator: Indicator::Co2Eq,
            interval: UncertaintyInterval::new(1.0, 2.0, 3.0).unwrap(),
            unit: "gCO2eq".into(),
            bins: None,
        }],
        equivalents: vec![],
        hypotheses: vec![],
        computed_at: utc("2026-04-01T10:00:00Z"),
        seed: 42,
    }
}

#[test]
fn a_result_without_method_modalities_overhead_or_bins_is_read_with_the_defaults() {
    let result: EstimationResult = serde_json::from_str(LEGACY_RESULT).unwrap();

    assert_eq!(result, legacy_result());
}

#[test]
fn a_legacy_result_is_written_back_with_the_method_added() {
    let result: EstimationResult = serde_json::from_str(LEGACY_RESULT).unwrap();

    assert_eq!(
        serde_json::to_string(&result).unwrap(),
        r#"{"method":"afnor_sobria","request":{"model_id":"gpt-4o-mini","tokens_in":10,"tokens_out_estimated":50,"timestamp":"2026-04-01T10:00:00Z"},"indicators":[{"indicator":"co2_eq","interval":{"p5":1.0,"p50":2.0,"p95":3.0},"unit":"gCO2eq"}],"equivalents":[],"hypotheses":[],"computed_at":"2026-04-01T10:00:00Z","seed":42}"#
    );
}

#[test]
fn unknown_fields_are_accepted_and_dropped_at_every_depth() {
    let text = r#"{"model_id":"gpt-4o-mini","tokens_in":23,"tokens_out_estimated":720,"timestamp":"2026-05-12T14:32:08Z","extra":1,"modalities":[{"kind":"document","page_count":3,"ocr":true}],"overhead":{"memory_tokens":5,"cache_tokens":9}}"#;

    let request: EstimationRequest = serde_json::from_str(text).unwrap();

    assert_eq!(
        request.modalities,
        [InputModality::Document { page_count: 3 }]
    );
    assert_eq!(request.overhead.memory_tokens, 5);
    let written = serde_json::to_value(&request).unwrap();
    assert_eq!(
        written,
        json!({
            "model_id": "gpt-4o-mini",
            "tokens_in": 23,
            "tokens_out_estimated": 720,
            "timestamp": "2026-05-12T14:32:08Z",
            "modalities": [{"kind": "document", "page_count": 3}],
            "overhead": {
                "system_prompt_tokens": 0,
                "tools_definition_tokens": 0,
                "memory_tokens": 5,
                "thinking_tokens_p50": 0
            }
        })
    );

    let mut with_extra: serde_json::Value = serde_json::from_str(LEGACY_RESULT).unwrap();
    with_extra["schema"] = json!(2);
    with_extra["indicators"][0]["interval"]["p99"] = json!(9.0);
    let result: EstimationResult = serde_json::from_value(with_extra).unwrap();
    assert_eq!(result, legacy_result());
}

#[test]
fn unknown_fields_are_accepted_in_every_nested_type_of_a_result() {
    let mut value = serde_json::to_value(full_result()).unwrap();
    for pointer in [
        "/request",
        "/indicators/0",
        "/indicators/0/interval",
        "/indicators/0/bins",
        "/equivalents/0",
        "/hypotheses/0",
    ] {
        value.pointer_mut(pointer).unwrap()["extra"] = json!(1);
    }

    let result: EstimationResult = serde_json::from_value(value).unwrap();

    assert_eq!(result, full_result());
}

#[test]
fn a_request_missing_a_required_field_is_refused() {
    let text = r#"{"model_id":"gpt-4o-mini","tokens_out_estimated":720,"timestamp":"2026-05-12T14:32:08Z"}"#;

    assert!(error_of::<EstimationRequest>(text).starts_with("missing field `tokens_in`"));
}

#[test]
fn a_result_missing_its_seed_is_refused() {
    let text = LEGACY_RESULT.replace(r#""seed": 42"#, r#""sead": 42"#);

    assert!(error_of::<EstimationResult>(&text).starts_with("missing field `seed`"));
}

#[test]
fn unknown_modality_kinds_and_methods_are_refused() {
    let modality = MINIMAL_REQUEST.replace('}', r#","modalities":[{"kind":"video"}]}"#);
    let method = LEGACY_RESULT.replace(r#""request""#, r#""method": "quantum", "request""#);

    assert!(error_of::<EstimationRequest>(&modality).starts_with("unknown variant `video`"));
    assert!(error_of::<EstimationResult>(&method).starts_with("unknown variant `quantum`"));
}

#[test]
fn token_counts_that_are_negative_fractional_or_too_large_are_refused() {
    let with = |tokens: &str| MINIMAL_REQUEST.replace("23", tokens);

    assert!(error_of::<EstimationRequest>(&with("-1")).starts_with("invalid value: integer `-1`"));
    assert!(error_of::<EstimationRequest>(&with("1.5"))
        .starts_with("invalid type: floating point `1.5`"));
    assert!(error_of::<EstimationRequest>(&with("4294967296"))
        .starts_with("invalid value: integer `4294967296`"));
}

#[test]
fn an_interval_read_from_json_is_not_checked_until_validate_is_called() {
    let interval: UncertaintyInterval =
        serde_json::from_str(r#"{"p5":3.0,"p50":2.0,"p95":1.0}"#).unwrap();

    assert_eq!(interval.p5, 3.0);
    assert!(interval.validate().is_err());
}
