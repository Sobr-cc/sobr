ADR-0025, session identity and duplicates across sources (2026-10-31, SOB-146)

- **ADR-0025** (Accepted): a session has one random Sobr ID and, on the
  device only, the references that each source gives it. Two reports are one
  session only when a reference proves it, whether or not Claude Code uses one
  session ID in all its sources (to be confirmed by SOB-163).
  - A source's report for a session replaces its own previous report. The
    numbers come whole from the most precise source whose report is complete
    (OpenTelemetry, transcript, extension); a partial report never beats a
    complete one. A better report arriving later replaces the numbers and
    the older one stays in the history.
  - Billing names no session, so it never changes a session or a total: its
    figure is shown beside the total Sobr saw, with the gap.
  - A correction by the person belongs to the session and survives every
    change of winner. 16 shared test cases, 3 worked examples, 9 build issues
    (38 points), marking those that wait for a Deep cleanup.
  - Thibault took the recommendation on both open points (SOB-195): the
    billed figure is shown beside the total, and the desktop app receives
    OpenTelemetry on the same machine. He also decided that the device keeps
    the folder, repo and branch of a session for 24 months, on the device only
    (SOB-198).
  - The billing credential of an organization can do more than read usage, so
    the ADR lists protections for it. It lives on the Sobr server (SOB-206,
    option A); its rotation, and whether a read-only credential exists, are
    design points of build issue 8.
- **ADR-0022, Amendment 1** (Accepted): closes the references that left these points to
  SOB-146, and adds one item to its upload contract (a session merge).
- Corrects one sentence of the ADR-0023 entry (SOB-144).
