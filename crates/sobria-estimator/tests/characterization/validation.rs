//! Caractérisation du module `validation` : les rapports de plausibilité
//! (ordres de grandeur) et de reproduction (valeurs publiées ± tolérance).

use serde_json::Map;
use sobria_core::InputModality;
use sobria_estimator::{
    run_all_plausibility, run_all_reproduction, run_plausibility, run_reproduction,
    EmpreinteMethod, PlausibilityCase, ReproductionCase,
};

use crate::{
    fixtures::{insert_error, outcome_json, FULL_OVERHEAD, NO_OVERHEAD},
    golden::assert_golden,
};

const GPT_4O_MINI_FR: PlausibilityCase = PlausibilityCase {
    id: "characterization-plausibility",
    description: "GPT-4o mini, mix France, prompt court",
    model_id: "gpt-4o-mini",
    tokens_in: 100,
    tokens_out: 500,
    if_electrical_g_per_kwh: 56.0,
    expected_range_g_co2eq: (1e-5, 1e-2),
    reference: "caractérisation SOB-9",
};

const LLAMA_70B_REPRODUCTION: ReproductionCase = ReproductionCase {
    id: "characterization-reproduction",
    method: EmpreinteMethod::AfnorSobria,
    source_doi_or_url: "caractérisation SOB-9",
    model_id: "llama-3-1-70b",
    tokens_in: 200,
    tokens_out: 1000,
    if_electrical_g_per_kwh: 56.0,
    pue: 1.2,
    expected_p50_g_co2eq: 0.05,
    tolerance: 0.5,
    disable_embodied: false,
    notes: "",
    modalities: &[],
    overhead: NO_OVERHEAD,
};

#[test]
fn every_built_in_plausibility_case() {
    assert_golden("validation_plausibility_cases", &run_all_plausibility());
}

#[test]
fn every_built_in_reproduction_case() {
    assert_golden("validation_reproduction_cases", &run_all_reproduction());
}

#[test]
fn plausibility_runner() {
    let cases = [
        ("passes", GPT_4O_MINI_FR),
        (
            "fails_outside_range",
            PlausibilityCase {
                expected_range_g_co2eq: (100.0, 200.0),
                ..GPT_4O_MINI_FR
            },
        ),
    ];
    let mut out = Map::new();
    for (name, case) in cases {
        out.insert(name.into(), outcome_json(run_plausibility(&case)));
    }
    let unknown = PlausibilityCase {
        model_id: "not-in-registry",
        ..GPT_4O_MINI_FR
    };
    insert_error(&mut out, "unknown_model", run_plausibility(&unknown));
    assert_golden("validation_plausibility_runner", &out);
}

#[test]
fn reproduction_runner() {
    let cases = [
        ("afnor_within_tolerance", LLAMA_70B_REPRODUCTION),
        (
            "afnor_outside_tolerance",
            ReproductionCase {
                tolerance: 0.0001,
                ..LLAMA_70B_REPRODUCTION
            },
        ),
        (
            "afnor_without_embodied",
            ReproductionCase {
                disable_embodied: true,
                ..LLAMA_70B_REPRODUCTION
            },
        ),
        (
            "ecologits",
            ReproductionCase {
                method: EmpreinteMethod::EcoLogits,
                ..LLAMA_70B_REPRODUCTION
            },
        ),
        (
            "ecologits_without_embodied",
            ReproductionCase {
                method: EmpreinteMethod::EcoLogits,
                disable_embodied: true,
                ..LLAMA_70B_REPRODUCTION
            },
        ),
        (
            "afnor_with_modalities_and_overhead",
            ReproductionCase {
                model_id: "claude-sonnet-4-6",
                modalities: &[
                    InputModality::VisionLow { image_count: 2 },
                    InputModality::Document { page_count: 3 },
                ],
                overhead: FULL_OVERHEAD,
                ..LLAMA_70B_REPRODUCTION
            },
        ),
    ];
    let mut out = Map::new();
    for (name, case) in cases {
        out.insert(name.into(), outcome_json(run_reproduction(&case)));
    }
    let unknown = ReproductionCase {
        model_id: "not-in-registry",
        ..LLAMA_70B_REPRODUCTION
    };
    insert_error(&mut out, "unknown_model", run_reproduction(&unknown));
    assert_golden("validation_reproduction_runner", &out);
}
