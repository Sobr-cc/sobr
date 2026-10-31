printpdf 0.12 for the PDF reports (2026-10-31, SOB-197)

- **`sobria-export`**: `printpdf` goes from 0.7 to 0.12 (`default-features = false`,
  no HTML layout engine), which brings `lopdf` 0.44. This removes the `lopdf`
  stack-overflow advisory (RUSTSEC-2026-0187) and the unmaintained `ttf-parser`
  (RUSTSEC-2026-0192) from `cargo deny check advisories`. Same report: two A4
  pages, the same built-in fonts, sizes and positions for all 32 text blocks.
  The one visible change: the arrow in "Période : a → b" and "id a → b" was
  silently dropped by the old version (the fonts have no arrow) and would print
  as `?` with the new one, so both lines use an en dash instead. The new
  version dates a document 1970-01-01 by default: the creation, modification
  and metadata dates are set to the current time, as before. A save that
  reports an error now returns `ExportError::Pdf` instead of a damaged report.
- **Dependencies**: the `sobria-export` tree goes from 80 to 134 crates, mostly
  the PDF encryption stack that `lopdf` 0.44 always includes (`aes`, `cbc`, `sha2`
  0.11, `rand` 0.10) and the font crates of `printpdf`. `cargo deny` bans and
  licenses stay ok.
