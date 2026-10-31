//! Caractérisation du registre de modèles et de ce qui en dérive : presets,
//! paramètres par modèle, tokens effectifs, équivalents, méthodologies.

use serde_json::{json, Map, Value};
use sobria_core::{EmpreinteMethod, EstimationResult, InputModality, DEFAULT_SEED};
use sobria_estimator::{
    auto_thinking_tokens, available_models, available_models_filtered, effective_tokens,
    engine_for,
    equivalents::{co2eq_to_car_km, energy_wh_to_screen_hours, energy_wh_to_shower_seconds},
    find_preset, info_for, modalities_total, modality_tokens, overhead_from_preset,
    EcoLogitsEngine, EstimationParams, ModelPreset, MonteCarloEngine, AVAILABLE_METHODS,
    MODEL_REGISTRY,
};

use crate::{
    fixtures::{params_for, reference_requests, request, to_json},
    golden::{assert_golden, Goldens},
};

#[test]
fn model_registry_content() {
    assert_golden("model_registry", &MODEL_REGISTRY);
}

#[test]
fn model_lists_and_lookup() {
    let ids = |models: Vec<&ModelPreset>| -> Vec<&'static str> {
        models.into_iter().map(|p| p.id).collect()
    };
    assert_golden(
        "model_lists",
        &json!({
            "available_models": ids(available_models()),
            "available_models_with_deprecated": ids(available_models_filtered(true)),
            "available_models_without_deprecated": ids(available_models_filtered(false)),
            "find_known": find_preset("gpt-4o-mini").map(|p| p.id),
            "find_unknown": find_preset("not-in-registry").map(|p| p.id),
            "find_is_case_sensitive": find_preset("GPT-4o-mini").map(|p| p.id),
        }),
    );
}

#[test]
fn params_derived_from_each_preset() {
    let mut out = Map::new();
    out.insert(
        "conservative_default".into(),
        to_json(&EstimationParams::conservative_default()),
    );
    for preset in MODEL_REGISTRY {
        let params = params_for(preset.id);
        let from_preset = EstimationParams::from_preset(preset).expect("preset valide");
        assert_eq!(params, from_preset, "for_model et from_preset divergent");
        out.insert(preset.id.into(), to_json(&params));
    }
    assert_golden("estimation_params_by_model", &out);
}

#[test]
fn standard_request_on_every_model_with_both_engines() {
    let afnor = MonteCarloEngine::new(DEFAULT_SEED);
    let ecologits = EcoLogitsEngine::new(DEFAULT_SEED);
    let intervals = |result: &EstimationResult| -> Value {
        let mut map = Map::new();
        for value in &result.indicators {
            map.insert(format!("{:?}", value.indicator), to_json(&value.interval));
        }
        Value::Object(map)
    };

    let mut out = Map::new();
    for preset in MODEL_REGISTRY {
        // 500 tokens en entrée, 500 en sortie : une requête de chat courante.
        let req = request(preset.id, 500, 500);
        let params = params_for(preset.id);
        let afnor_result = afnor.estimate(&req, &params).expect("estimation AFNOR");
        let ecologits_result = ecologits
            .estimate(&req, &params)
            .expect("estimation EcoLogits");
        out.insert(
            preset.id.into(),
            json!({
                "afnor": intervals(&afnor_result),
                "ecologits": intervals(&ecologits_result),
            }),
        );
    }
    assert_golden("standard_request_by_model", &out);
}

#[test]
fn effective_tokens_for_reference_requests() {
    let mut out = Map::new();
    for (name, req) in reference_requests() {
        let (tokens_in, tokens_out) = effective_tokens(&req, find_preset(&req.model_id));
        out.insert(name.into(), json!({ "in": tokens_in, "out": tokens_out }));
    }
    // Modèle absent du registre : tokens bruts + overhead utilisateur.
    let (tokens_in, tokens_out) = effective_tokens(&request("not-in-registry", 100, 500), None);
    out.insert(
        "unknown_model".into(),
        json!({ "in": tokens_in, "out": tokens_out }),
    );
    // Saturation plutôt que débordement.
    let (tokens_in, tokens_out) =
        effective_tokens(&request("o3", u32::MAX, u32::MAX), find_preset("o3"));
    out.insert(
        "saturating_u32_max".into(),
        json!({ "in": tokens_in, "out": tokens_out }),
    );
    assert_golden("effective_tokens", &out);
}

/// Tokens de chaque modalité pour **chaque** preset (les grilles de tarification
/// vision diffèrent selon le vendor), plus le cas sans preset.
///
/// La grille d'images encadre les seuils des tarifications par palier
/// (≤ 384 px, 512, 1024×768, 4096) et les cas dégénérés (0 pixel, 0 image).
#[test]
fn modality_tokens_per_model() {
    const DIMENSIONS: [(u32, u32); 6] = [
        (0, 0),
        (384, 384),
        (385, 385),
        (512, 512),
        (1024, 768),
        (4096, 4096),
    ];
    let mut modalities = vec![
        InputModality::Text,
        InputModality::Document { page_count: 3 },
        InputModality::AudioInput {
            duration_seconds: 30,
        },
    ];
    for image_count in [0, 1, 2, 4] {
        modalities.push(InputModality::VisionLow { image_count });
        for (avg_width, avg_height) in DIMENSIONS {
            modalities.push(InputModality::VisionHigh {
                image_count,
                avg_width,
                avg_height,
            });
        }
    }

    // Une ligne par modèle : un entier par modalité, puis leur total.
    let tokens_for = |preset: Option<&ModelPreset>| -> Vec<u32> {
        let mut tokens: Vec<u32> = modalities
            .iter()
            .map(|m| modality_tokens(m, preset))
            .collect();
        tokens.push(modalities_total(&modalities, preset));
        tokens
    };
    let mut tokens_by_model = Map::new();
    tokens_by_model.insert("no_preset".into(), json!(tokens_for(None)));
    for preset in MODEL_REGISTRY {
        tokens_by_model.insert(preset.id.into(), json!(tokens_for(Some(preset))));
    }
    assert_golden(
        "modality_tokens",
        &json!({
            "modalities": modalities.iter().map(|m| format!("{m:?}")).collect::<Vec<_>>(),
            "tokens_by_model": tokens_by_model,
        }),
    );
}

#[test]
fn thinking_tokens_and_overhead_per_preset() {
    const OUTPUT_TOKENS: [u32; 5] = [0, 1, 500, 10_000, u32::MAX];
    let thinking_for = |preset: Option<&ModelPreset>| -> Vec<u32> {
        OUTPUT_TOKENS
            .iter()
            .map(|&out| auto_thinking_tokens(out, preset))
            .collect()
    };

    let mut thinking = Map::new();
    let mut overhead = Map::new();
    thinking.insert("no_preset".into(), json!(thinking_for(None)));
    for preset in MODEL_REGISTRY {
        thinking.insert(preset.id.into(), json!(thinking_for(Some(preset))));
        overhead.insert(
            preset.id.into(),
            to_json(&overhead_from_preset(preset, 1_000)),
        );
    }
    let mut goldens = Goldens::new();
    goldens.assert("auto_thinking_tokens", &thinking);
    goldens.assert("overhead_from_preset", &overhead);
}

#[test]
fn equivalents_conversions() {
    const INPUTS: [f64; 8] = [0.0, 1e-6, 0.001, 1.0, 25.0, 192.0, 1_750.0, 1e6];
    let rows: Vec<Value> = INPUTS
        .iter()
        .map(|&x| {
            json!({
                "input": x,
                "co2eq_to_car_km": co2eq_to_car_km(x),
                "energy_wh_to_shower_seconds": energy_wh_to_shower_seconds(x),
                "energy_wh_to_screen_hours": energy_wh_to_screen_hours(x),
            })
        })
        .collect();
    assert_golden("equivalents", &rows);
}

#[test]
fn methodologies_catalogue() {
    let mut engines = Map::new();
    for &method in EmpreinteMethod::all() {
        let engine = engine_for(method);
        engines.insert(
            format!("{method:?}"),
            json!({
                "engine_method": format!("{:?}", engine.method()),
                "info": info_for(method),
                "engine_info_is_catalogue_entry": std::ptr::eq(
                    engine.methodology_info(),
                    info_for(method),
                ),
            }),
        );
    }
    assert_golden(
        "methodologies",
        &json!({
            "all": EmpreinteMethod::all().iter().map(|m| format!("{m:?}")).collect::<Vec<_>>(),
            "default": format!("{:?}", EmpreinteMethod::default()),
            "available_methods": AVAILABLE_METHODS,
            "engines": engines,
        }),
    );
}
