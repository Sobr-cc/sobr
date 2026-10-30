//! Caractérisation du moteur Monte-Carlo AFNOR : `MonteCarloEngine`,
//! `Distribution`, `bin_samples`.

use rand_xoshiro::{rand_core::SeedableRng, Xoshiro256PlusPlus};
use serde_json::{json, Map};
use sobria_core::{EstimationRequest, InputModality, DEFAULT_SEED};
use sobria_estimator::{
    bin_samples, Distribution, EstimationParams, MonteCarloEngine, DEFAULT_BIN_COUNT, DEFAULT_N,
    MIN_SAMPLES_FOR_BINS,
};
use Distribution::{LogNormal, Normal, Point, Uniform};

use crate::{
    fixtures::{insert_error, params_for, reference_results, request, to_json},
    golden::assert_golden,
};

/// Moteur au seed explicite : les tests ne dépendent pas de l'environnement.
fn engine() -> MonteCarloEngine {
    MonteCarloEngine::new(DEFAULT_SEED)
}

#[test]
fn engine_constants_are_stable() {
    assert_golden(
        "monte_carlo_constants",
        &json!({
            "default_seed": DEFAULT_SEED,
            "default_n": DEFAULT_N,
            "default_bin_count": DEFAULT_BIN_COUNT,
            "min_samples_for_bins": MIN_SAMPLES_FOR_BINS,
        }),
    );
}

#[test]
fn reference_requests_with_registry_params() {
    assert_golden(
        "monte_carlo_reference_requests",
        &reference_results(&engine()),
    );
}

#[test]
fn engine_configuration_changes_the_draws() {
    let req = request("gpt-4o-mini", 100, 500);
    let params = params_for("gpt-4o-mini");
    let configs = [
        ("seed_42_n_default", MonteCarloEngine::new(42)),
        ("seed_7_n_default", MonteCarloEngine::new(7)),
        ("seed_42_n_1000", MonteCarloEngine::new(42).with_n(1_000)),
        ("seed_42_n_10", MonteCarloEngine::new(42).with_n(10)),
        ("seed_42_n_1", MonteCarloEngine::new(42).with_n(1)),
    ];
    let mut out = Map::new();
    for (name, engine) in configs {
        let result = engine.estimate(&req, &params).expect("estimation");
        out.insert(
            name.into(),
            json!({ "seed": engine.seed(), "n": engine.n(), "result": result }),
        );
    }
    assert_golden("monte_carlo_engine_configuration", &out);
}

#[test]
fn parameter_overrides() {
    let engine = engine();
    let req = request("gpt-4o-mini", 100, 500);
    let base = params_for("gpt-4o-mini");
    let all_points = EstimationParams {
        epsilon_prefill_mj_per_token: Point { value: 1.0 },
        epsilon_decode_mj_per_token: Point { value: 2.0 },
        pue: Point { value: 1.2 },
        if_electrical_g_per_kwh: Point { value: 56.0 },
        embodied_g_per_request: Point { value: 0.02 },
        wue_l_per_kwh: Point { value: 1.5 },
    };
    let cases = [
        ("registry_params", base),
        (
            "conservative_default",
            EstimationParams::conservative_default(),
        ),
        ("pue_point", base.with_pue(Point { value: 1.2 })),
        (
            "pue_uniform_wide",
            base.with_pue(Uniform {
                low: 1.0,
                high: 2.0,
            }),
        ),
        (
            "if_electrical_low_carbon",
            base.with_if_electrical(Point { value: 20.0 }),
        ),
        (
            "if_electrical_high_carbon",
            base.with_if_electrical(Point { value: 600.0 }),
        ),
        ("embodied_zero", base.with_embodied(Point { value: 0.0 })),
        ("wue_point", base.with_wue(Point { value: 1.0 })),
        (
            "epsilon_decode_normal",
            base.with_epsilon_decode(Normal {
                mean: 2.0,
                std: 0.5,
            }),
        ),
        (
            "epsilon_decode_lognormal",
            base.with_epsilon_decode(LogNormal {
                mu: 0.5,
                sigma: 0.3,
            }),
        ),
        // Aucune incertitude : P5 = P50 = P95 et pas d'histogramme.
        ("all_points_degenerate", all_points),
    ];
    let mut out = Map::new();
    for (name, params) in cases {
        let result = engine.estimate(&req, &params).expect("estimation");
        out.insert(name.into(), to_json(&result));
    }
    // Modèle absent du registre : pas d'overhead ni de modalités.
    let unknown = engine
        .estimate(
            &request("not-in-registry", 100, 500),
            &EstimationParams::conservative_default(),
        )
        .expect("estimation");
    out.insert(
        "unknown_model_conservative_default".into(),
        to_json(&unknown),
    );
    assert_golden("monte_carlo_parameter_overrides", &out);
}

#[test]
fn errors() {
    let engine = engine();
    let req = request("gpt-4o-mini", 100, 500);
    let base = params_for("gpt-4o-mini");
    let too_many_modalities = EstimationRequest {
        modalities: vec![InputModality::Text; 33],
        ..req.clone()
    };

    let mut out = Map::new();
    insert_error(&mut out, "n_zero", engine.with_n(0).estimate(&req, &base));
    insert_error(
        &mut out,
        "empty_model_id",
        engine.estimate(&request("", 1, 1), &base),
    );
    insert_error(
        &mut out,
        "tokens_in_above_limit",
        engine.estimate(&request("gpt-4o-mini", 10_000_001, 1), &base),
    );
    insert_error(
        &mut out,
        "tokens_out_above_limit",
        engine.estimate(&request("gpt-4o-mini", 1, 10_000_001), &base),
    );
    insert_error(
        &mut out,
        "too_many_modalities",
        engine.estimate(&too_many_modalities, &base),
    );
    insert_error(
        &mut out,
        "invalid_pue_bounds",
        engine.estimate(
            &req,
            &base.with_pue(Uniform {
                low: 2.0,
                high: 1.0,
            }),
        ),
    );
    insert_error(
        &mut out,
        "invalid_sigma",
        engine.estimate(
            &req,
            &base.with_epsilon_decode(LogNormal {
                mu: 0.0,
                sigma: -1.0,
            }),
        ),
    );
    insert_error(
        &mut out,
        "non_finite_wue",
        engine.estimate(
            &req,
            &base.with_wue(Point {
                value: f64::INFINITY,
            }),
        ),
    );
    insert_error(
        &mut out,
        "for_model_unknown",
        EstimationParams::for_model("not-in-registry"),
    );
    assert_golden("monte_carlo_errors", &out);
}

#[test]
fn same_seed_reproduces_the_same_result() {
    let req = request("o3", 200, 800);
    let params = params_for("o3");
    let engine = engine();
    // Tout sauf `computed_at` : les indicateurs portent les quantiles et les bins.
    let first = engine.estimate(&req, &params).expect("estimation");
    let second = engine.estimate(&req, &params).expect("estimation");
    assert_eq!(to_json(&first.indicators), to_json(&second.indicators));
    assert_eq!(to_json(&first.hypotheses), to_json(&second.hypotheses));
}

#[test]
fn distribution_sampling_stream() {
    let distributions = [
        ("point", Point { value: 3.5 }),
        (
            "uniform",
            Uniform {
                low: 1.1,
                high: 1.4,
            },
        ),
        (
            "normal",
            Normal {
                mean: 2.0,
                std: 0.5,
            },
        ),
        // Tronquée à 0 quand le tirage normal est négatif.
        (
            "normal_truncated_at_zero",
            Normal {
                mean: 0.1,
                std: 1.0,
            },
        ),
        (
            "log_normal",
            LogNormal {
                mu: 0.0,
                sigma: 0.3,
            },
        ),
    ];
    let mut out = Map::new();
    for (name, distribution) in distributions {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(DEFAULT_SEED);
        let draws: Vec<f64> = (0..8).map(|_| distribution.sample(&mut rng)).collect();
        out.insert(name.into(), json!(draws));
    }
    assert_golden("distribution_sampling_stream", &out);
}

#[test]
fn distribution_calibration_and_validation() {
    let mut out = Map::new();

    let intervals = [
        ("ratio_4", (1.0, 2.0, 4.0)),
        ("narrow", (0.9, 1.0, 1.1)),
        ("tiny_values", (1e-6, 5e-6, 2e-5)),
    ];
    for (name, (p5, p50, p95)) in intervals {
        let dist = Distribution::log_normal_from_interval(p5, p50, p95).expect("intervalle valide");
        out.insert(format!("log_normal_from_interval.{name}"), to_json(&dist));
    }

    let bad_intervals = [
        ("non_positive_p5", (0.0, 1.0, 2.0)),
        ("unordered", (2.0, 1.0, 3.0)),
        ("flat", (1.0, 1.0, 1.0)),
    ];
    for (name, (p5, p50, p95)) in bad_intervals {
        insert_error(
            &mut out,
            &format!("log_normal_from_interval.{name}"),
            Distribution::log_normal_from_interval(p5, p50, p95),
        );
    }

    let invalid = [
        ("point_nan", Point { value: f64::NAN }),
        (
            "uniform_inverted",
            Uniform {
                low: 2.0,
                high: 1.0,
            },
        ),
        (
            "uniform_infinite",
            Uniform {
                low: 0.0,
                high: f64::INFINITY,
            },
        ),
        (
            "normal_zero_std",
            Normal {
                mean: 1.0,
                std: 0.0,
            },
        ),
        (
            "log_normal_negative_sigma",
            LogNormal {
                mu: 0.0,
                sigma: -0.1,
            },
        ),
        (
            "log_normal_nan_mu",
            LogNormal {
                mu: f64::NAN,
                sigma: 0.1,
            },
        ),
    ];
    for (name, dist) in invalid {
        insert_error(&mut out, &format!("validate.{name}"), dist.validate());
    }

    assert_golden("distribution_calibration_and_validation", &out);
}

#[test]
fn bin_samples_edge_cases() {
    let ramp: Vec<f64> = (0..100).map(f64::from).collect();
    let skewed: Vec<f64> = std::iter::repeat_n(0.0, 50)
        .chain((1..=50).map(|i| f64::from(i) * 10.0))
        .collect();
    // Une rampe valide avec une seule valeur non finie : sans le garde-fou
    // de `bin_samples`, elle produirait un histogramme (min < max).
    let mut with_nan = ramp.clone();
    with_nan[3] = f64::NAN;
    let mut with_inf = ramp.clone();
    with_inf[3] = f64::INFINITY;

    let cases: Vec<(&str, Vec<f64>, usize)> = vec![
        ("below_min_samples", vec![1.0; MIN_SAMPLES_FOR_BINS - 1], 5),
        (
            "at_min_samples_degenerate",
            vec![1.0; MIN_SAMPLES_FOR_BINS],
            5,
        ),
        ("zero_bins", ramp.clone(), 0),
        ("contains_nan", with_nan, 5),
        ("contains_infinity", with_inf, 5),
        ("ramp_10_bins", ramp.clone(), 10),
        ("ramp_default_bins", ramp.clone(), DEFAULT_BIN_COUNT),
        ("ramp_single_bin", ramp, 1),
        ("skewed_right_tail", skewed, 10),
    ];
    let mut out = Map::new();
    for (name, samples, n_bins) in cases {
        out.insert(name.into(), to_json(&bin_samples(&samples, n_bins)));
    }
    assert_golden("bin_samples_edge_cases", &out);
}
