ADR-0025 amended: a gap inside a report makes it partial (2026-10-31, SOB-267)

- **ADR-0025, Amendment 2** (Proposed): section 5 tested a report by its two
  ends only, so an app closed for an hour in the middle of a session left the
  OpenTelemetry report "complete" and it beat the full transcript, with the
  hour missing (SOB-166: Claude Code does not retry an export nobody received).
  The receiver now logs its silences on the device. A gap is a witness record
  (a transcript line with usage that the session owns) inside a silence,
  counted from 60 seconds before it begins because of the export interval. A
  report with a gap is partial, and the flag is stored with the report and
  survives a new report, a cleaned transcript and a merge. The 5-minute edge
  tolerance is unchanged.
- Eleven test cases (a gap with a full transcript in both orders, no
  transcript, silences with and without a witness, a transcript read later,
  a crash and its boundaries, errors, several silences, a fork, and the three
  ways the flag could be lost) and the changes to build issues 1 and 4.
