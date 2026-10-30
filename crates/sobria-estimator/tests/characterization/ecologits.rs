//! Caractérisation du moteur EcoLogits : `EcoLogitsEngine` et ses formules
//! publiques (`n_gpu`, énergie/latence par token, énergie et embodied par
//! requête).

use serde_json::{json, Map, Value};
use sobria_core::DEFAULT_SEED;
use sobria_estimator::{
    engines::ecologits::{
        f_energy_per_token_wh, f_latency_per_token_sec, n_gpu, request_embodied_co2eq_g,
        request_energy_kwh,
    },
    Distribution, EcoLogitsEngine,
};
use Distribution::{LogNormal, Normal, Point, Uniform};

use crate::{
    fixtures::{insert_error, params_for, reference_results, request, to_json},
    golden::assert_golden,
};

/// Moteur au seed explicite : les tests ne dépendent pas de l'environnement.
fn engine() -> EcoLogitsEngine {
    EcoLogitsEngine::new(DEFAULT_SEED)
}

#[test]
fn reference_requests_with_registry_params() {
    assert_golden(
        "ecologits_reference_requests",
        &reference_results(&engine()),
    );
}

#[test]
fn parameter_overrides() {
    let engine = engine();
    let req = request("llama-3-1-70b", 300, 700);
    let base = params_for("llama-3-1-70b");
    let cases = [
        ("registry_params", base),
        ("pue_point", base.with_pue(Point { value: 1.5 })),
        // Une distribution est réduite à sa valeur centrale.
        (
            "pue_uniform_midpoint",
            base.with_pue(Uniform {
                low: 1.0,
                high: 2.0,
            }),
        ),
        (
            "if_electrical_normal_mean",
            base.with_if_electrical(Normal {
                mean: 300.0,
                std: 50.0,
            }),
        ),
        (
            "wue_lognormal_median",
            base.with_wue(LogNormal {
                mu: 0.5,
                sigma: 0.2,
            }),
        ),
        (
            "if_electrical_point",
            base.with_if_electrical(Point { value: 20.0 }),
        ),
        ("wue_point", base.with_wue(Point { value: 3.0 })),
        // Seule la valeur exacte Point(0) désactive l'embodied.
        (
            "embodied_point_zero",
            base.with_embodied(Point { value: 0.0 }),
        ),
        (
            "embodied_point_nonzero",
            base.with_embodied(Point { value: 5.0 }),
        ),
        // Les ε du preset sont ignorés par EcoLogits.
        (
            "epsilon_decode_ignored",
            base.with_epsilon_decode(Point { value: 1000.0 }),
        ),
    ];
    let mut out = Map::new();
    for (name, params) in cases {
        let result = engine.estimate(&req, &params).expect("estimation");
        out.insert(name.into(), to_json(&result));
    }
    assert_golden("ecologits_parameter_overrides", &out);
}

#[test]
fn seed_is_traced_but_does_not_change_values() {
    let req = request("gpt-4o-mini", 100, 500);
    let params = params_for("gpt-4o-mini");
    let mut out = Map::new();
    for seed in [DEFAULT_SEED, 7] {
        let engine = EcoLogitsEngine::new(seed);
        let result = engine.estimate(&req, &params).expect("estimation");
        out.insert(
            format!("seed_{seed}"),
            json!({ "engine_seed": engine.seed(), "result": result }),
        );
    }
    assert_golden("ecologits_seed", &out);
}

#[test]
fn formulas_over_a_size_grid() {
    const PARAMS_BILLIONS: [f64; 12] = [
        1.0, 3.8, 7.0, 8.0, 13.0, 34.0, 70.0, 120.0, 175.0, 405.0, 671.0, 1800.0,
    ];
    let rows: Vec<Value> = PARAMS_BILLIONS
        .iter()
        .map(|&p| {
            json!({
                "p_billions": p,
                "n_gpu": n_gpu(p),
                "f_energy_per_token_wh": f_energy_per_token_wh(p),
                "f_latency_per_token_sec": f_latency_per_token_sec(p),
                "request_energy_kwh_500_tokens_pue_1_2": request_energy_kwh(p, 500, 1.2),
                "request_energy_kwh_0_tokens_pue_1_2": request_energy_kwh(p, 0, 1.2),
                "request_embodied_co2eq_g_500_tokens": request_embodied_co2eq_g(p, 500),
                "request_embodied_co2eq_g_0_tokens": request_embodied_co2eq_g(p, 0),
            })
        })
        .collect();
    assert_golden("ecologits_formulas", &rows);
}

#[test]
fn errors() {
    let engine = engine();
    let base = params_for("llama-3-1-70b");
    let mut out = Map::new();
    insert_error(
        &mut out,
        "unknown_model",
        engine.estimate(&request("not-in-registry", 100, 500), &base),
    );
    insert_error(
        &mut out,
        "empty_model_id",
        engine.estimate(&request("", 1, 1), &base),
    );
    insert_error(
        &mut out,
        "tokens_above_limit",
        engine.estimate(&request("llama-3-1-70b", 10_000_001, 1), &base),
    );
    insert_error(
        &mut out,
        "invalid_pue",
        engine.estimate(
            &request("llama-3-1-70b", 100, 500),
            &base.with_pue(Uniform {
                low: 2.0,
                high: 1.0,
            }),
        ),
    );
    assert_golden("ecologits_errors", &out);
}
