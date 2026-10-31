# OpenTelemetry receiver stub

A test aid for SOB-166 and for build issue 4 of ADR-0025. `receiver-stub.yaml` is an OpenTelemetry Collector (Contrib, tested with 0.163.2) that stands in for the desktop app's local receiver: it accepts OTLP metrics from Claude Code on `127.0.0.1:4318` (HTTP) and `127.0.0.1:4317` (gRPC), and appends every request to `stub-log.jsonl` in the current folder (`SOBR_STUB_LOG` sets another path). It forwards nothing.

**The log is raw.** It holds the account e-mail, the account and organization IDs and the session ID of the run. Run the stub from a scratch folder, keep the log on your machine and delete it after use. Git ignores any file named `stub-log.jsonl`. If ports 4317 or 4318 are taken, change them in `receiver-stub.yaml`.

```sh
# 1. The stand-in receiver
otelcol-contrib --config receiver-stub.yaml

# 2. Claude Code with the variables of the managed settings (docs/sources/CLAUDE-CODE-USAGE-CAPTURE.md, section 3)
CLAUDE_CODE_ENABLE_TELEMETRY=1 OTEL_METRICS_EXPORTER=otlp OTEL_LOGS_EXPORTER=none \
OTEL_EXPORTER_OTLP_PROTOCOL=http/json OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4318 \
claude -p "Reply with the single word ok."
```

Then read `stub-log.jsonl`: the metrics, attributes and times Claude Code sends, as listed in the section 3 table.
