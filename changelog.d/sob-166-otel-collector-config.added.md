Metrics-only OpenTelemetry collector configuration for Claude Code (2026-10-31, SOB-166)

- **`deploy/otel-collector/`**: a collector configuration that keeps only the token and cost metrics of Claude Code, with `model`, `type` and `session.id`, cuts times to the day and refuses logs and traces; a stand-in for the Sobr server; and a README with the steps and the results of real runs (request paths, headers, delta temporality, receiver down).
