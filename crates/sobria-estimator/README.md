# sobria-estimator

Computes the footprint of one LLM request (CO₂eq, energy, water), with a
P5-P95 interval, and builds the simulations and forecasts on top of it.

- **Methodologies** (`engine_trait.rs`, ADR-0012): AFNOR SPEC 2314
  (`MonteCarloEngine`, Monte-Carlo over the distributions of `EstimationParams`)
  and EcoLogits (`engines/ecologits.rs`, a deterministic port).
  `engines::factory::engine_for` picks the engine for a method.
- **Model registry** (`model_presets.rs`): one preset per model, with its
  energy and embodied ranges and its source URL.
- **Tokens** (`effective_tokens.rs`): image, audio and context overhead
  tokens, and the hidden reasoning tokens of reasoning models.
- **What-if and forecasts**: `scenarios.rs` (M13, scenarios and a P50 series)
  and `yearly_forecast.rs` (M16, P5/P50/P95 series).
- **Validation** (`validation/`): plausibility ranges and reproduction cases.

Every formula and default value cites its source in the code (CLAUDE.md §6);
the ones still missing are listed in SOB-209.

## Tests

```bash
cargo test -p sobria-estimator
```

The engines use the fixed seed `sobria_core::DEFAULT_SEED` (42); nothing reads
`SOBRIA_SEED` yet (SOB-84). `tests/characterization/` pins today's outputs in
`tests/golden/` (SOB-9); see `tests/golden/README.md` before changing a golden
file.
