# Claude Code metrics collector

Optional setup for organizations (SOB-166; see `docs/sources/CLAUDE-CODE-USAGE-CAPTURE.md`, section 3). An OpenTelemetry collector sits between Claude Code and the Sobr server. Claude Code sends it everything, including the user's e-mail, so run it in your own infrastructure. It forwards only what the ADR-0022 upload contract allows.

| File | Role |
| --- | --- |
| `collector.yaml` | The collector. Reads the Sobr server address from `SOBR_OTLP_ENDPOINT`. |
| `stub-sink.yaml` | A second collector that stands in for the Sobr server and writes what it receives to a JSON lines file. For tests only. |

## What the collector keeps

* Metrics: `claude_code.token.usage` and `claude_code.cost.usage`. Every other metric is dropped.
* Data point attributes: `model`, `type` (token usage only) and `session.id`. Everything else is dropped, among them `user.email`, `user.id`, `user.account_uuid`, `user.account_id`, `organization.id`, `terminal.type`, `query_source` and `effort`.
* Resource attributes: none.
* Times: `time` and `start_time` are cut to 00:00:00 UTC of the day.
* Logs and traces: refused (the collector answers 404, there is no pipeline for them).
* Passed on unchanged: the scope name and version (`com.anthropic.claude_code`, the Claude Code version) and the metric unit and description. Claude Code 2.1.314 sends no exemplars and no flags; check again when it changes.

The collector does not add up data points. Claude Code splits one call by `query_source` and `effort`, so after those attributes are dropped several data points have the same attributes, and since the times are cut to the day they also have the same times. They are increments (see Temporality): the receiver must add them, not keep one per attributes and time.

Two choices are open and decided by Thibault: the day is the UTC day, not the space's time zone of ADR-0022 (SOB-258), and `session.id` is forwarded although Amendment 1 of ADR-0022 keeps source session IDs on the device (SOB-259).

## Try it

You need the OpenTelemetry Collector **Contrib** distribution (tested with 0.163.2). In three terminals:

```sh
# 1. Stand-in for the Sobr server; writes stub-log.jsonl in the current folder
otelcol-contrib --config stub-sink.yaml

# 2. The collector, forwarding to the stand-in
SOBR_OTLP_ENDPOINT=http://127.0.0.1:4399 otelcol-contrib --config collector.yaml

# 3. Claude Code with the telemetry variables (the `env` block of the managed settings sets the same ones)
CLAUDE_CODE_ENABLE_TELEMETRY=1 OTEL_METRICS_EXPORTER=otlp OTEL_LOGS_EXPORTER=none \
OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4318 \
claude -p "Reply with the single word ok."
```

Then read `stub-log.jsonl`. It must hold only the two metric names, the attribute keys `model`, `type` and `session.id`, no resource attributes, and times at midnight UTC. Do not commit the file.

## What was tested

Claude Code 2.1.314, collector Contrib 0.163.2, Windows, October 31, 2026. Short `claude -p` runs on a real account.

| Question | Result |
| --- | --- |
| Path of a base endpoint | `http://host:4318` sends `POST /v1/metrics`. `http://host:4318/prefix` sends `POST /prefix/v1/metrics`. `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT` is used as given (`/custom/metrics` stayed `/custom/metrics`). gRPC to port 4317 works through the same collector. |
| Content type | `http/json`: `application/json`. `http/protobuf`: `application/x-protobuf`. No `Content-Encoding`. User agent `OTel-OTLP-Exporter-JavaScript/0.208.0`. |
| Headers | `OTEL_EXPORTER_OTLP_HEADERS="Authorization=Bearer …,x-sobr-test=1"` arrived on every request. `otelHeadersHelper` (a command that prints a JSON object), set in a `--settings` file, also worked: its headers arrived. |
| Temporality | Delta: `aggregationTemporality` is 1 and the sums are monotonic. An export holds only the increase since the previous one. |
| When it sends | Once per export interval (`OTEL_METRIC_EXPORT_INTERVAL`, 60 s by default) and once at exit: a `claude -p` run of a few seconds with the default interval sent one request with all four metrics. |
| Totals | In 6 runs with every hop up (JSON, protobuf, gRPC, logs enabled in the client, final configuration, Sobr server started late), the sums at the stand-in equal the totals of the `--output-format json` result of Claude Code: input, output, cache read, cache creation and cost. |
| Nothing listens on the collector port | Claude Code exits 0, prints nothing on stderr, and takes the same time. The export is not retried: with the collector started 3 s after Claude Code, the tokens of the first call (1,174 input and 14 output) never arrived, while the second call did. Delta increments lost this way cannot be recovered. |
| Collector up, Sobr server down | The collector logs "Exporting failed. Will retry the request after interval." at info level (6.7 s, then 11.6 s) and delivers when the server is back: with the stand-in back about 9 s after the first failed export, the totals were complete. A longer outage was not tested. |
| Logs and traces sent to the collector | `/v1/logs` and `/v1/traces` answer 404. A client with `OTEL_LOGS_EXPORTER=otlp` still exits 0 and the stand-in receives only metrics. |

## Not covered

* TLS, authentication and the bind address. The receivers listen on `127.0.0.1`, so a collector on a server cannot be reached from developers' machines until `endpoint` is changed, and Claude Code then fails without any error (see the table). Add TLS and an authentication extension before the receivers are reachable from other machines.
* Retry limits. The exporter uses the collector's default retry and in-memory queue, so a long outage or a collector restart can lose data. Persistent queues (`file_storage`) were not set up.
* The managed settings file was not installed on the test machine (it applies to every Claude Code session on it). The same variables were set in the environment of the test process, which is what the `env` block does according to the documentation, and `otelHeadersHelper` was read from a `--settings` file.
* A collector restart with data waiting to be sent, interactive sessions, subagents, and an exit by Ctrl+C.
