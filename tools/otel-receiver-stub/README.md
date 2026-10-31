# OpenTelemetry receiver stub

A test aid for SOB-166 and for build issue 4 of ADR-0025. `receiver-stub.yaml` is an OpenTelemetry Collector (Contrib, tested with 0.163.2) that stands in for the desktop app's local receiver: it accepts OTLP metrics from Claude Code on `127.0.0.1:4318` (HTTP) and `127.0.0.1:4317` (gRPC), and appends each request to `stub-log.jsonl` in the folder where you start it. It forwards nothing. If ports 4317 or 4318 are taken, change them in `receiver-stub.yaml`.

**The log is raw.** It holds the account e-mail, the account and organization IDs and the session ID of the run. Start the stub from a scratch folder outside the repository, keep the log on your machine and delete it after use. Git ignores any file named `stub-log.jsonl`, and the name cannot be changed.

The log holds the metric payloads only. It does not show the request paths, the content type or the headers; for those, see the table in section 3 of the note, or point Claude Code at a small HTTP server that prints the request line and the header names.

```sh
# 1. The stand-in receiver (use the path to this folder)
otelcol-contrib --config /path/to/sobr/tools/otel-receiver-stub/receiver-stub.yaml

# 2. Claude Code with the variables of the managed settings (docs/sources/CLAUDE-CODE-USAGE-CAPTURE.md, section 3)
CLAUDE_CODE_ENABLE_TELEMETRY=1 OTEL_METRICS_EXPORTER=otlp OTEL_LOGS_EXPORTER=none \
OTEL_EXPORTER_OTLP_PROTOCOL=http/json OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4318 \
claude -p "Reply with the single word ok."
```

Then read `stub-log.jsonl`: the metric names, attributes, values and times Claude Code sends, as listed in the section 3 table.
