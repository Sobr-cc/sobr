Characterization tests of the report, PROV-O document and datasheet of sobria-export (2026-10-31, SOB-221)

- **`sobria-export`**: 12 tests in `tests/` record what the documents say
  today, without changing the code. The text of the PDF report is read back
  with the parser of `printpdf` and pinned in golden files, for a ledger that
  only used EcoLogits and for a ledger with two methods. The PROV-O graph of a
  report and the Gebru datasheet are pinned in golden files too, with the
  value that comes from the clock masked. Other tests pin the label of each
  method in the PDF, the datasheet and PROV-O, the fixed AFNOR header and
  methodology block, the ignored `locale`, "Issued on" and the PROV-O times
  taken from the end of the period (even in the future), the default datasheet
  options (seed 42, N = 10,000, the crate version), `dateModified` inside the
  hashed JSON-LD, a character outside WinAnsi printed as `?` (SOB-241), and a
  malformed payload (the report fails, the datasheet skips the entry).
- Pinned quirks are named `quirk`. The report states "Rapport conforme AFNOR
  SPEC 2314" and a fixed methodology block whatever the ledger used (SOB-124);
  a character outside WinAnsi prints as `?` (SOB-241). The other audit
  findings are left to SOB-23.
- Not covered: the fake `team-shim-{id}` entries fed by the team server
  (SOB-25), and the look of the PDF (no rendering check).
