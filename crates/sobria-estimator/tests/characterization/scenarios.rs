//! Caractérisation du simulateur « Et si… ? » (`simulate`) et de la
//! projection annuelle (`forecast_yearly`), avec les deux moteurs.

use serde_json::{json, Map};
use sobria_estimator::{
    engine_for, forecast_yearly, simulate, EmpreinteMethod, EstimationParams, ForecastConfig,
    ParamOverrides, Scenario, SimulationRequest, YearlyForecastRequest, YearlyScenario,
    MAX_FORECAST_HORIZON_MONTHS, MAX_FORECAST_MONTHS, MAX_FORECAST_SCENARIOS, MAX_SCENARIOS,
};

use crate::{
    fixtures::{insert_error, outcome_json, params_for, request, to_json},
    golden::{assert_golden, Goldens},
};

const METHODS: [(&str, EmpreinteMethod); 2] = [
    ("afnor", EmpreinteMethod::AfnorSobria),
    ("ecologits", EmpreinteMethod::EcoLogits),
];

fn scenario(label: &str, overrides: ParamOverrides) -> Scenario {
    Scenario {
        label: label.into(),
        overrides,
    }
}

/// `ParamOverrides` dont seuls les champs touchés par `set` sont renseignés.
fn overrides(set: impl FnOnce(&mut ParamOverrides)) -> ParamOverrides {
    let mut overrides = ParamOverrides::default();
    set(&mut overrides);
    overrides
}

fn low_carbon_grid() -> Scenario {
    scenario(
        "Low carbon grid",
        overrides(|o| o.if_electrical_g_per_kwh = Some(20.0)),
    )
}

fn forecast(months: u32, monthly_growth_pct: f64, base_volume_per_day: f64) -> ForecastConfig {
    ForecastConfig {
        months,
        monthly_growth_pct,
        base_volume_per_day,
    }
}

/// Un scénario par type d'override, plus une combinaison et un scénario vide.
fn all_override_scenarios() -> Vec<Scenario> {
    vec![
        scenario(
            "Model swap",
            overrides(|o| o.model_id = Some("claude-haiku-4-5".into())),
        ),
        scenario("Short answers", overrides(|o| o.tokens_out = Some(100))),
        scenario("Efficient datacenter", overrides(|o| o.pue = Some(1.1))),
        low_carbon_grid(),
        scenario(
            "No embodied",
            overrides(|o| o.embodied_g_per_request = Some(0.0)),
        ),
        scenario("Dry cooling", overrides(|o| o.wue_l_per_kwh = Some(0.5))),
        scenario(
            "Everything",
            ParamOverrides {
                model_id: Some("llama-3-1-8b".into()),
                tokens_out: Some(200),
                pue: Some(1.1),
                if_electrical_g_per_kwh: Some(20.0),
                embodied_g_per_request: Some(0.001),
                wue_l_per_kwh: Some(0.5),
            },
        ),
        scenario("Same as baseline", ParamOverrides::default()),
    ]
}

fn simulation(scenarios: Vec<Scenario>, forecast: Option<ForecastConfig>) -> SimulationRequest {
    SimulationRequest {
        baseline: request("gpt-4o-mini", 100, 500),
        scenarios,
        forecast,
    }
}

#[test]
fn limits_are_stable() {
    assert_golden(
        "scenarios_constants",
        &json!({
            "max_scenarios": MAX_SCENARIOS,
            "max_forecast_months": MAX_FORECAST_MONTHS,
            "max_forecast_scenarios": MAX_FORECAST_SCENARIOS,
            "max_forecast_horizon_months": MAX_FORECAST_HORIZON_MONTHS,
        }),
    );
}

#[test]
fn param_overrides_any() {
    let cases = [
        ("empty", ParamOverrides::default()),
        ("model_id", overrides(|o| o.model_id = Some("o3".into()))),
        ("tokens_out", overrides(|o| o.tokens_out = Some(0))),
        ("pue", overrides(|o| o.pue = Some(1.0))),
        (
            "if_electrical",
            overrides(|o| o.if_electrical_g_per_kwh = Some(0.0)),
        ),
        (
            "embodied",
            overrides(|o| o.embodied_g_per_request = Some(0.0)),
        ),
        ("wue", overrides(|o| o.wue_l_per_kwh = Some(0.0))),
    ];
    let mut out = Map::new();
    for (name, overrides) in cases {
        out.insert(name.into(), json!(overrides.any()));
    }
    assert_golden("param_overrides_any", &out);
}

#[test]
fn simulate_every_override_with_forecast() {
    let req = simulation(all_override_scenarios(), Some(forecast(12, 5.0, 1_000.0)));
    let mut goldens = Goldens::new();
    for (name, method) in METHODS {
        let engine = engine_for(method);
        let result =
            simulate(engine.as_ref(), &req, &params_for("gpt-4o-mini")).expect("simulation");
        goldens.assert(&format!("simulate_{name}"), &result);
    }
}

/// Seule la projection est figée ici : le baseline et les scénarios le sont
/// déjà par `simulate_every_override_with_forecast`, et l'arithmétique de la
/// projection ne lit que le P50 (EcoLogits, déterministe, suffit).
#[test]
fn simulate_forecast_variants() {
    let cases = [
        ("no_forecast", None),
        ("decay_12_months", Some(forecast(12, -10.0, 500.0))),
        ("no_growth", Some(forecast(6, 0.0, 250.0))),
        (
            "max_horizon_max_growth",
            Some(forecast(60, 50.0, 1_000_000.0)),
        ),
        ("min_growth_zero_volume", Some(forecast(1, -50.0, 0.0))),
    ];
    let engine = engine_for(EmpreinteMethod::EcoLogits);
    let mut out = Map::new();
    for (name, config) in cases {
        let result = simulate(
            engine.as_ref(),
            &simulation(vec![low_carbon_grid()], config),
            &params_for("gpt-4o-mini"),
        )
        .expect("simulation");
        out.insert(name.into(), to_json(&result.forecast));
    }
    assert_golden("simulate_forecast_variants", &out);
}

#[test]
fn simulate_edge_cases() {
    let mut out = Map::new();
    for (name, method) in METHODS {
        let engine = engine_for(method);
        // Aucun scénario : seul le baseline est calculé.
        out.insert(
            format!("{name}.no_scenarios"),
            outcome_json(simulate(
                engine.as_ref(),
                &simulation(Vec::new(), None),
                &params_for("gpt-4o-mini"),
            )),
        );
        // Baseline hors registre : AFNOR retombe sur les tokens bruts,
        // EcoLogits refuse.
        let unknown = SimulationRequest {
            baseline: request("not-in-registry", 100, 500),
            scenarios: Vec::new(),
            forecast: None,
        };
        out.insert(
            format!("{name}.unknown_baseline_model"),
            outcome_json(simulate(
                engine.as_ref(),
                &unknown,
                &EstimationParams::conservative_default(),
            )),
        );
    }
    assert_golden("simulate_edge_cases", &out);
}

#[test]
fn simulate_errors() {
    let engine = engine_for(EmpreinteMethod::AfnorSobria);
    let params = params_for("gpt-4o-mini");
    let run = |req: &SimulationRequest| simulate(engine.as_ref(), req, &params);
    let named = |label: &str| scenario(label, ParamOverrides::default());
    let too_many: Vec<Scenario> = (0..=MAX_SCENARIOS)
        .map(|i| named(&format!("s{i}")))
        .collect();

    let mut out = Map::new();
    insert_error(
        &mut out,
        "too_many_scenarios",
        run(&simulation(too_many, None)),
    );
    insert_error(
        &mut out,
        "duplicate_label",
        run(&simulation(vec![named("same"), named("same")], None)),
    );
    for (name, config) in [
        ("forecast_zero_months", forecast(0, 5.0, 100.0)),
        (
            "forecast_too_many_months",
            forecast(MAX_FORECAST_MONTHS + 1, 5.0, 100.0),
        ),
        ("forecast_growth_too_high", forecast(12, 50.5, 100.0)),
        ("forecast_growth_too_low", forecast(12, -50.5, 100.0)),
        ("forecast_negative_volume", forecast(12, 5.0, -1.0)),
        (
            "forecast_volume_above_limit",
            forecast(12, 5.0, 1_000_001.0),
        ),
        ("forecast_volume_nan", forecast(12, 5.0, f64::NAN)),
    ] {
        insert_error(&mut out, name, run(&simulation(Vec::new(), Some(config))));
    }
    let bad_model_override = scenario(
        "Unknown model",
        overrides(|o| o.model_id = Some("not-in-registry".into())),
    );
    insert_error(
        &mut out,
        "override_unknown_model",
        run(&simulation(vec![bad_model_override], None)),
    );
    assert_golden("simulate_errors", &out);
}

fn yearly_scenario(label: &str, monthly_growth_pct: f64) -> YearlyScenario {
    YearlyScenario {
        label: label.into(),
        monthly_growth_pct,
    }
}

fn yearly(months: u32, base_volume_per_day: f64) -> YearlyForecastRequest {
    YearlyForecastRequest {
        baseline: request("gpt-4o-mini", 100, 500),
        scenarios: vec![
            yearly_scenario("Status quo", 0.0),
            yearly_scenario("+10% / month", 10.0),
            yearly_scenario("-5% / month", -5.0),
        ],
        months,
        base_volume_per_day,
    }
}

#[test]
fn forecast_yearly_with_both_engines() {
    let mut goldens = Goldens::new();
    for (name, method) in METHODS {
        let engine = engine_for(method);
        let result = forecast_yearly(engine.as_ref(), &yearly(12, 1_000.0)).expect("forecast");
        goldens.assert(&format!("forecast_yearly_{name}"), &result);
    }
}

#[test]
fn forecast_yearly_horizons() {
    let engine = engine_for(EmpreinteMethod::AfnorSobria);
    let mut out = Map::new();
    for (name, months, volume) in [
        ("one_month", 1, 1_000.0),
        ("three_years", 36, 1_000.0),
        ("max_horizon", MAX_FORECAST_HORIZON_MONTHS, 1_000_000.0),
        ("zero_volume", 12, 0.0),
    ] {
        let result = forecast_yearly(engine.as_ref(), &yearly(months, volume)).expect("forecast");
        out.insert(name.into(), to_json(&result));
    }
    assert_golden("forecast_yearly_horizons", &out);
}

#[test]
fn forecast_yearly_errors() {
    let engine = engine_for(EmpreinteMethod::AfnorSobria);
    let with_scenarios = |scenarios: Vec<YearlyScenario>| YearlyForecastRequest {
        scenarios,
        ..yearly(12, 1_000.0)
    };
    let too_many: Vec<YearlyScenario> = (0..=MAX_FORECAST_SCENARIOS)
        .map(|i| yearly_scenario(&format!("s{i}"), 1.0))
        .collect();

    let cases: Vec<(&str, YearlyForecastRequest)> = vec![
        ("no_scenarios", with_scenarios(Vec::new())),
        ("too_many_scenarios", with_scenarios(too_many)),
        ("zero_months", yearly(0, 1_000.0)),
        (
            "too_many_months",
            yearly(MAX_FORECAST_HORIZON_MONTHS + 1, 1_000.0),
        ),
        ("negative_volume", yearly(12, -1.0)),
        ("volume_above_limit", yearly(12, 1_000_001.0)),
        ("volume_infinite", yearly(12, f64::INFINITY)),
        (
            "growth_too_high",
            with_scenarios(vec![yearly_scenario("up", 50.1)]),
        ),
        (
            "growth_too_low",
            with_scenarios(vec![yearly_scenario("down", -50.1)]),
        ),
        (
            "duplicate_label",
            with_scenarios(vec![
                yearly_scenario("same", 1.0),
                yearly_scenario("same", 2.0),
            ]),
        ),
        (
            "unknown_baseline_model",
            YearlyForecastRequest {
                baseline: request("not-in-registry", 100, 500),
                ..yearly(12, 1_000.0)
            },
        ),
    ];
    let mut out = Map::new();
    for (name, req) in cases {
        insert_error(&mut out, name, forecast_yearly(engine.as_ref(), &req));
    }
    assert_golden("forecast_yearly_errors", &out);
}
