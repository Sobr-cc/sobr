What the app's local OpenTelemetry receiver must accept from Claude Code (2026-10-31, SOB-166)

- **`docs/sources/CLAUDE-CODE-USAGE-CAPTURE.md`, section 3**: results of real Claude Code 2.1.314 runs: request paths, protocols, headers, delta temporality, attributes, send timing, and what happens when the receiver is down or answers an error (no retry on a refused connection, retries on 503); the managed settings that point Claude Code at the local app; the recommendation now follows ADR-0025 (the app receives the export, no collector forwards to the server).
- **`tools/otel-receiver-stub/`**: a collector configuration that stands in for the app's receiver in tests and forwards nothing.
