Agent rules match "How we work" (2026-10-31, SOB-138)

- **`CLAUDE.md`, `CONTRIBUTING.md`**: a `briefs/chantiers/` brief is read
  only when the Linear issue or its project links one (ADR-0018,
  Amendment 1); the `import` commit scope is gone with its crate.
- **Merging and review:** no limit on issues In Review; a reviewer agent
  merges a PR on its own "ready to merge" verdict, without waiting for
  Thibault; an ADR PR may be merged as Proposed. Same wording in
  `CLAUDE.md` §11, `.claude/agents/` and the orchestrate skill.
- **Issue skill:** lists the `Repo` area and the `role:infra` and
  `role:sre` roles, as in Linear.
- **Merge rule, aligned everywhere**: a newer "changes requested" from another reviewer on
  the same head blocks the merge; an in-place amendment of an Accepted ADR is merged as
  Proposed and takes effect when Thibault knowingly accepts it, unless it only records
  decisions he already took in writing (`CLAUDE.md` §11, `docs/adr/README.md`);
  `CONTRIBUTING.md` and the `ci.yml` header allow a merge when the failing jobs also fail
  on main for the same cause.
