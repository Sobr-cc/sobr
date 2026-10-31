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
    let mut ok = minimal_request();
    ok.tokens_in = MAX_TOKENS;
    ok.tokens_out_estimated = MAX_TOKENS;
    assert!(ok.validate().is_ok());

    let mut too_many_in = minimal_request();
    too_many_in.tokens_in = MAX_TOKENS + 1;
    assert_eq!(
        error(&too_many_in),
        format!("{PREFIX}tokens absurdes : in=10000001, out=720")
    );

    let mut too_many_out = minimal_request();
    too_many_out.tokens_out_estimated = MAX_TOKENS + 1;
    assert_eq!(
        error(&too_many_out),
        format!("{PREFIX}tokens absurdes : in=23, out=10000001")
    );

    let mut max = minimal_request();
    max.tokens_in = u32::MAX;
    assert_eq!(
        error(&max),
        format!("{PREFIX}tokens absurdes : in=4294967295, out=720")
    );
}

#[test]
fn a_blank_model_id_is_refused_and_a_padded_one_is_accepted() {
    for blank in ["", " ", "\t\n"] {
        let mut request = minimal_request();
        request.model_id = blank.into();

        assert_eq!(
            error(&request),
            format!("{PREFIX}model_id vide"),
            "{blank:?}"
        );
    }

    let mut padded = minimal_request();
    padded.model_id = " gpt-4o-mini ".into();
    assert!(padded.validate().is_ok());
}

#[test]
fn thirty_two_modalities_are_accepted_and_thirty_three_are_refused() {
    let mut ok = minimal_request();
    ok.modalities = vec![InputModality::Text; 32];
    assert!(ok.validate().is_ok());

    let mut too_many = minimal_request();
    too_many.modalities = vec![InputModality::Text; 33];
    assert_eq!(
        error(&too_many),
        format!("{PREFIX}trop de modalities : 33 > 32")
    );
}

#[test]
fn each_overhead_field_accepts_ten_million_and_refuses_one_more() {
    let fields: [fn(u32) -> ContextOverhead; 4] = [
        |n| ContextOverhead::new(n, 0, 0, 0),
        |n| ContextOverhead::new(0, n, 0, 0),
        |n| ContextOverhead::new(0, 0, n, 0),
        |n| ContextOverhead::new(0, 0, 0, n),
    ];

    for (index, overhead) in fields.into_iter().enumerate() {
        let mut ok = minimal_request();
        ok.overhead = overhead(MAX_TOKENS);
        assert!(ok.validate().is_ok(), "field {index}");

        let mut too_many = minimal_request();
        too_many.overhead = overhead(MAX_TOKENS + 1);
        assert_eq!(
            error(&too_many),
            format!("{PREFIX}overhead tokens absurdes"),
            "field {index}"
        );
    }
}

#[test]
fn the_overhead_fields_are_limited_one_by_one_not_by_their_sum() {
    let mut request = minimal_request();
    request.overhead = ContextOverhead::new(MAX_TOKENS, MAX_TOKENS, MAX_TOKENS, MAX_TOKENS);

    assert!(request.validate().is_ok());
}

#[test]
fn the_checks_run_in_order_tokens_then_model_then_modalities_then_overhead() {
    let mut request = minimal_request();
    request.tokens_in = MAX_TOKENS + 1;
    request.model_id = String::new();
    request.modalities = vec![InputModality::Text; 33];
    request.overhead = ContextOverhead::new(MAX_TOKENS + 1, 0, 0, 0);
    assert!(error(&request).contains("tokens absurdes"));

    request.tokens_in = 23;
    assert!(error(&request).contains("model_id vide"));

    request.model_id = "gpt-4o-mini".into();
    assert!(error(&request).contains("trop de modalities"));

    request.modalities = Vec::new();
    assert!(error(&request).contains("overhead tokens absurdes"));
}
