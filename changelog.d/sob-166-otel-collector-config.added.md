Metrics-only OpenTelemetry collector configuration for Claude Code (2026-10-31, SOB-166)

- **`deploy/otel-collector/`**: a collector configuration that keeps only `claude_code.token.usage` and `claude_code.cost.usage`, with `model`, `type` and `session.id`, no resource attributes and times cut to the day, and refuses logs and traces; a stand-in for the Sobr server; and a README with the steps and the results of real Claude Code 2.1.314 runs (request paths, headers, delta temporality, and what happens when a receiver is down).
