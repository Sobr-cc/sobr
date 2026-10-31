//! The limits of `EstimationRequest::validate`, at their edges, with the error texts.

use sobria_core::{ContextOverhead, EstimationRequest, InputModality};

use crate::shapes::minimal_request;

const MAX_TOKENS: u32 = 10_000_000;
const PREFIX: &str = "validation échouée : ";

fn error(request: &EstimationRequest) -> String {
    request.validate().unwrap_err().to_string()
}

#[test]
fn a_minimal_request_is_valid() {
    assert!(minimal_request().validate().is_ok());
}

#[test]
fn ten_million_tokens_are_accepted_and_one_more_is_refused() {
    let ok = EstimationRequest {
        tokens_in: MAX_TOKENS,
        tokens_out_estimated: MAX_TOKENS,
        ..minimal_request()
    };
    assert!(ok.validate().is_ok());

    let too_many_in = EstimationRequest {
        tokens_in: MAX_TOKENS + 1,
        ..minimal_request()
    };
    assert_eq!(
        error(&too_many_in),
        format!("{PREFIX}tokens absurdes : in=10000001, out=720")
    );

    let too_many_out = EstimationRequest {
        tokens_out_estimated: MAX_TOKENS + 1,
        ..minimal_request()
    };
    assert_eq!(
        error(&too_many_out),
        format!("{PREFIX}tokens absurdes : in=23, out=10000001")
    );

    let max = EstimationRequest {
        tokens_in: u32::MAX,
        ..minimal_request()
    };
    assert_eq!(
        error(&max),
        format!("{PREFIX}tokens absurdes : in=4294967295, out=720")
    );
}

#[test]
fn a_blank_model_id_is_refused_and_a_padded_one_is_accepted() {
    for blank in ["", " ", "\t\n"] {
        let request = EstimationRequest {
            model_id: blank.into(),
            ..minimal_request()
        };

        assert_eq!(
            error(&request),
            format!("{PREFIX}model_id vide"),
            "{blank:?}"
        );
    }

    let padded = EstimationRequest {
        model_id: " gpt-4o-mini ".into(),
        ..minimal_request()
    };
    assert!(padded.validate().is_ok());
}

#[test]
fn thirty_two_modalities_are_accepted_and_thirty_three_are_refused() {
    let ok = EstimationRequest {
        modalities: vec![InputModality::Text; 32],
        ..minimal_request()
    };
    assert!(ok.validate().is_ok());

    let too_many = EstimationRequest {
        modalities: vec![InputModality::Text; 33],
        ..minimal_request()
    };
    assert_eq!(
        error(&too_many),
        format!("{PREFIX}trop de modalities : 33 > 32")
    );
}

#[test]
fn each_overhead_field_accepts_ten_million_and_refuses_one_more() {
    type Overhead = fn(u32) -> ContextOverhead;
    let fields: [(&str, Overhead); 4] = [
        ("system_prompt_tokens", |n| ContextOverhead::new(n, 0, 0, 0)),
        ("tools_definition_tokens", |n| {
            ContextOverhead::new(0, n, 0, 0)
        }),
        ("memory_tokens", |n| ContextOverhead::new(0, 0, n, 0)),
        ("thinking_tokens_p50", |n| ContextOverhead::new(0, 0, 0, n)),
    ];

    for (name, overhead) in fields {
        let ok = EstimationRequest {
            overhead: overhead(MAX_TOKENS),
            ..minimal_request()
        };
        assert!(ok.validate().is_ok(), "{name}");

        let too_many = EstimationRequest {
            overhead: overhead(MAX_TOKENS + 1),
            ..minimal_request()
        };
        assert_eq!(
            error(&too_many),
            format!("{PREFIX}overhead tokens absurdes"),
            "{name}"
        );
    }
}

#[test]
fn the_overhead_fields_are_limited_one_by_one_not_by_their_sum() {
    let request = EstimationRequest {
        overhead: ContextOverhead::new(MAX_TOKENS, MAX_TOKENS, MAX_TOKENS, MAX_TOKENS),
        ..minimal_request()
    };

    assert!(request.validate().is_ok());
}

#[test]
fn the_checks_run_in_order_tokens_then_model_then_modalities_then_overhead() {
    let mut request = EstimationRequest {
        tokens_in: MAX_TOKENS + 1,
        model_id: String::new(),
        modalities: vec![InputModality::Text; 33],
        overhead: ContextOverhead::new(MAX_TOKENS + 1, 0, 0, 0),
        ..minimal_request()
    };
    assert!(error(&request).contains("tokens absurdes"));

    request.tokens_in = 23;
    assert!(error(&request).contains("model_id vide"));

    request.model_id = "gpt-4o-mini".into();
    assert!(error(&request).contains("trop de modalities"));

    request.modalities = Vec::new();
    assert!(error(&request).contains("overhead tokens absurdes"));
}
