//! Entrées fixes partagées par les tests de caractérisation.

use std::fmt::Debug;

use chrono::{TimeZone, Utc};
use serde::Serialize;
use serde_json::{json, Map, Value};
use sobria_core::{ContextOverhead, EstimationRequest, InputModality};
use sobria_estimator::{EmpreinteEngine, EstimationParams, EstimatorError};

/// Overhead nul (l'état par défaut d'une requête).
pub const NO_OVERHEAD: ContextOverhead = ContextOverhead {
    system_prompt_tokens: 0,
    tools_definition_tokens: 0,
    memory_tokens: 0,
    thinking_tokens_p50: 0,
};

/// Overhead typique d'une interface chat avec outils et mémoire.
pub const FULL_OVERHEAD: ContextOverhead = ContextOverhead {
    system_prompt_tokens: 2000,
    tools_definition_tokens: 800,
    memory_tokens: 1500,
    thinking_tokens_p50: 0,
};

/// Marqueur qui, dans l'erreur « modèle inconnu », précède la liste de tout
/// le registre.
const KNOWN_MODELS_MARKER: &str = ". Modèles connus";

/// Sérialise en JSON (le masquage de `computed_at` est fait par `assert_golden`).
pub fn to_json<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("la valeur doit être sérialisable")
}

/// Requête texte simple, sans overhead ni modalité. L'horodatage est figé :
/// c'est le seul champ de date non masqué.
pub fn request(model_id: &str, tokens_in: u32, tokens_out: u32) -> EstimationRequest {
    EstimationRequest {
        model_id: model_id.into(),
        tokens_in,
        tokens_out_estimated: tokens_out,
        datacenter_id: None,
        timestamp: Utc
            .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .single()
            .expect("date valide"),
        modalities: Vec::new(),
        overhead: NO_OVERHEAD,
    }
}

/// Paramètres calibrés pour un modèle du registre.
pub fn params_for(model_id: &str) -> EstimationParams {
    EstimationParams::for_model(model_id).expect("modèle du registre")
}

/// Jeu de modalités couvrant toutes les variantes de `InputModality`.
pub fn all_modalities() -> Vec<InputModality> {
    vec![
        InputModality::Text,
        InputModality::VisionLow { image_count: 2 },
        InputModality::VisionHigh {
            image_count: 1,
            avg_width: 1024,
            avg_height: 768,
        },
        InputModality::Document { page_count: 3 },
        InputModality::AudioInput {
            duration_seconds: 30,
        },
    ]
}

/// Jeu de requêtes de référence, nommées. Le nom sert de clé dans les golden
/// files. Couvre : petit/grand modèle, dense/MoE, reasoning (thinking auto
/// et explicite), modalités, overhead, extrêmes de taille.
pub fn reference_requests() -> Vec<(&'static str, EstimationRequest)> {
    vec![
        ("small_chat", request("gpt-4o-mini", 100, 500)),
        ("large_chat", request("claude-opus-4-8", 1000, 1000)),
        ("dense_70b", request("llama-3-1-70b", 300, 700)),
        ("moe", request("llama-4-maverick", 300, 700)),
        ("reasoning_auto_thinking", request("o3", 200, 800)),
        (
            "reasoning_explicit_thinking",
            EstimationRequest {
                overhead: ContextOverhead {
                    thinking_tokens_p50: 3000,
                    ..NO_OVERHEAD
                },
                ..request("o3", 200, 800)
            },
        ),
        (
            "multimodal_anthropic",
            EstimationRequest {
                modalities: all_modalities(),
                ..request("claude-sonnet-4-6", 400, 600)
            },
        ),
        (
            "multimodal_openai",
            EstimationRequest {
                modalities: all_modalities(),
                ..request("gpt-4o", 400, 600)
            },
        ),
        (
            "full_overhead",
            EstimationRequest {
                overhead: FULL_OVERHEAD,
                ..request("claude-sonnet-4-6", 400, 600)
            },
        ),
        (
            "with_datacenter",
            EstimationRequest {
                datacenter_id: Some("dc-fr-paris".into()),
                ..request("gpt-4o-mini", 100, 500)
            },
        ),
        ("minimal_tokens", request("gpt-4o-mini", 1, 1)),
        ("long_context", request("gemini-3-1-pro", 100_000, 20_000)),
    ]
}

/// Résultat complet du moteur pour chaque requête de référence, avec les
/// paramètres calibrés de son modèle.
pub fn reference_results(engine: &dyn EmpreinteEngine) -> Value {
    let mut out = Map::new();
    for (name, req) in reference_requests() {
        let result = engine
            .estimate(&req, &params_for(&req.model_id))
            .expect("estimation");
        out.insert(name.into(), to_json(&result));
    }
    Value::Object(out)
}

/// Représentation stable d'une erreur : variante + message affiché.
///
/// Le message « modèle inconnu » est coupé avant la liste du registre, déjà
/// figée dans `model_registry.json` : elle ne doit pas dupliquer ce golden file.
pub fn error_json(error: &EstimatorError) -> Value {
    let variant = match error {
        EstimatorError::Schema(_) => "Schema",
        EstimatorError::Validation(_) => "Validation",
        EstimatorError::Other(_) => "Other",
    };
    let message = error.to_string();
    let shown = message
        .split_once(KNOWN_MODELS_MARKER)
        .map_or(message.as_str(), |(head, _)| head);
    json!({ "variant": variant, "message": shown })
}

/// Enregistre l'erreur d'un cas qui doit échouer, sous `name`.
pub fn insert_error<T: Debug>(
    out: &mut Map<String, Value>,
    name: &str,
    result: Result<T, EstimatorError>,
) {
    let error = result.expect_err("le cas doit échouer");
    out.insert(name.into(), error_json(&error));
}

/// `{"ok": …}` ou `{"err": …}` selon l'issue, pour figer aussi les refus.
pub fn outcome_json<T: Serialize>(result: Result<T, EstimatorError>) -> Value {
    match result {
        Ok(value) => json!({ "ok": value }),
        Err(error) => json!({ "err": error_json(&error) }),
    }
}
