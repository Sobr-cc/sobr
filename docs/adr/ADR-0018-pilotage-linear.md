# ADR-0018 — Piloter l'avancement dans Linear

- **Statut** : Accepted
- **Amendment 1** (Accepted 2026-10-30, SOB-36): thematic projects, two-week cycles, label groups, Claude agents only. See the section below.
- **Date** : 2026-10-30
- **Décideurs** : Thibault
- **Contexte technique** : post-C46, préparation v1.0 (C35, C36) et v1.1 (C31)

## Amendment 1 — Match the Linear organization (2026-10-30)

**Status.** Accepted (2026-10-30), SOB-36.
- The amendment is written in place, as SOB-87 decided: an Accepted ADR is
  amended by a PR that Thibault approves, and git keeps the history.
- The original text below stays, and a note marks the point this amendment
  replaces.
- It is in English under the language rule of
  [How we work](https://linear.app/sobrcc/document/how-we-work-cb9e7156febb)
  (section "Language", decision of October 30).

**What changes and why.** This ADR was accepted on October 30, before the
Linear organization was set up the same day. That organization is described
in the document "Linear organization and agents" (a private claude.ai link,
<https://claude.ai/artifact/Cxbf5v5XMs8F5unZfEsbSg>; its French title is
"Organisation Linear et agents").
- **Original point 1 is replaced.** It said one project per chantier
  (`Cxx`), one issue per sub-chantier and one-week cycles, which is not what
  was set up.
- **Original points 2, 3, 5 and 6 stand.**
- **Original point 4 (branches) is confirmed** by SOB-12.

The amendment's own items are numbered A1 to A7, so they are not confused
with the original points.

### Decision

- **A1 — Thematic projects, not one per chantier.**
  - "Linear organization and agents" plans twelve projects on team `SOB`:
    eleven dated projects that lead to the public V1 (2027-03-31), plus Model
    catalog, which runs continuously.
  - Infrastructure was added on October 30 ("How we work"), which makes 13
    today: Foundations, Design V7, Data model and sessions, Claude Code and
    MCP, Browser extension, VS Code extension, Linear and GitHub
    integrations, Organization and identity, Cloud and self-hosting, Website
    and research, V1 launch, Model catalog, Infrastructure.
  - Issues are written in plain English with the `linear-issue-writing`
    skill and are 8 points at most ("How we work"). They are created when
    their project starts ("Linear organization and agents").
- **A2 — Briefs stay as specs where they exist.**
  - `briefs/chantiers/` still holds C31, C35 and C36. Closed chantiers are
    archived in `docs/archive/chantiers/`.
  - An issue or a project links its brief when it has one.
  - New work is specified in the issue itself: Why, What, Acceptance
    criteria, How to verify. Original point 1's rule "each Linear project
    points to its brief" no longer applies.
- **A3 — Two-week cycles starting on Monday.** The first cycle starts on
  Monday 2 November 2026 (Linear team settings).
- **A4 — Five label groups** on team `SOB`, as set in Linear:

  | Group | Labels |
  |--|--|
  | Type | Feature, Bug, Chore, Design, Docs, Research, Decision |
  | Area | App, Browser extension, VS Code, Claude Code, MCP, Server, Site, Data |
  | Agent | `agent:claude-code`, `agent:claude-design`, `human-only` |
  | Role | `role:orchestrator`, `role:po-individuals`, `role:po-organizations`, `role:po-research`, `role:designer`, `role:ux-writer`, `role:cto`, `role:rust`, `role:frontend`, `role:extensions`, `role:integrations`, `role:devops`, `role:qa`, `role:security`, `role:data`, `role:docs`, `role:infra`, `role:sre` |
  | Model | `model:haiku`, `model:sonnet`, `model:opus` |
- **A5 — Claude agents only.**
  - The work is done by Claude Code, with its roles in `.claude/agents/`,
    and by Claude Design.
  - Linear's own AI features (Linear Agent, automatic triage, coding
    sessions) stay off ("How we work").
  - Nothing is set in Settings › AI & Agents ("Linear organization and
    agents", decision of October 30).
- **A6 — Branches, as decided in SOB-12.** Original point 4 stands:
  `feature/sob-123-<short-desc>`, `fix/sob-123-<short-desc>`,
  `docs/sob-123-<short-desc>`. Claude Code builds the name from the issue
  ID and its type, rather than taking the name Linear suggests.
- **A7 — Working rules live in "How we work".** Who does what, the Ready and
  Done definitions, the estimation scale and the In Review limit are in
  [How we work](https://linear.app/sobrcc/document/how-we-work-cb9e7156febb)
  and in CLAUDE.md §11. This ADR does not repeat them.

### Effect on other documents

- **Already match A6:** CLAUDE.md §4.4 and CONTRIBUTING.md (line 19, branch
  names).
- **Follow in SOB-138:**
  - CLAUDE.md §2 (row `briefs/chantiers/Cxx-*.md`) and §11 (step 2 reads the
    brief; step 4f links it in the PR), which still tie every issue to a
    brief, against A2;
  - CONTRIBUTING.md line 11 ("the active sprint and its brief");
  - the Role list in `.claude/skills/linear-issue-writing/SKILL.md`, which
    lacks `role:infra` and `role:sre`.
- **Handled by other issues:**
  - CLAUDE.md §13 (French and English for every doc): SOB-97;
  - `docs/adr/README.md` (Accepted ADRs said to be immutable): SOB-127.

## Contexte et énoncé du problème

L'état d'avancement du projet est dispersé et en partie périmé :

- `docs/ROADMAP.md` planifie S0 → S16 de mai à septembre 2026 avec des
  statuts en emoji ; le dépôt en est à C46 fin octobre et ces statuts ne
  sont plus tenus (S0 y est encore 🟡 alors qu'il est terminé).
- Les briefs (`briefs/chantiers/`) décrivent le périmètre mais pas qui
  fait quoi ni où en est chaque sous-chantier.
- La convention `TODO(sobria-XXX)` (CLAUDE.md §14) devait pointer vers des
  issues GitHub qui n'ont jamais été créées : un seul TODO l'utilise
  (`deny.toml`), avec un identifiant libre.

Il faut une source de vérité unique pour les statuts, reliée aux branches
et aux PR, sans dupliquer le contenu des briefs.

## Options envisagées

- **Option A — Statu quo** : statuts dans `ROADMAP.md`, issues GitHub au
  besoin. Gratuit et versionné, mais rien n'oblige la mise à jour : c'est
  ce qui a produit la dérive actuelle.
- **Option B — GitHub Projects** : proche du code, mais cycles,
  initiatives et liens branche → ticket moins outillés.
- **Option C — Linear** (workspace `sobrcc`, équipe `SOB`) : projets,
  cycles et initiatives natifs ; l'intégration GitHub rattache
  automatiquement branches, commits et PR aux tickets via leur identifiant.

## Décision

**Option C — Linear devient la source de vérité de l'avancement.**

> Point 1 is replaced by Amendment 1 (2026-10-30): thematic projects,
> briefs only where they exist, two-week cycles.

1. **Découpage** : un projet Linear par chantier (`Cxx`), un ticket par
   sous-chantier (`Cxx.y`), cycles d'une semaine. Les briefs restent dans
   le dépôt comme spécification détaillée ; chaque projet Linear pointe
   vers son brief.
2. **`docs/ROADMAP.md`** devient le plan initial, conservé pour
   l'historique : ses statuts ne sont plus mis à jour.
3. **TODO** : `// TODO(SOB-123): …`, où `SOB-123` est un ticket Linear
   existant. Pas de TODO sans ticket.
4. **Branches** : l'identifiant du ticket suit le préfixe de type —
   `feature/sob-123-<short-desc>`, `fix/sob-123-<short-desc>`,
   `docs/sob-123-<short-desc>`. Linear détecte l'identifiant et lie la
   branche puis la PR au ticket.
5. **Commits et PR** : Conventional Commits inchangé (CLAUDE.md §4.3) ;
   footer `Refs: SOB-123`, ou `Fixes SOB-123` dans la description de PR
   pour clore le ticket au merge.
6. **Contributions externes** : le workspace Linear est privé ; les
   contributeurs externes continuent d'ouvrir des issues GitHub
   (CONTRIBUTING.md), que les mainteneurs reportent dans Linear.

## Conséquences

**Positives** :
- Un seul endroit pour savoir qui fait quoi et où en est chaque chantier.
- Lien automatique ticket ↔ branche ↔ PR, sans saisie manuelle.
- Les identifiants `SOB-123` sont stables : utilisables dans les TODO,
  les commits et les briefs.

**Négatives** :
- Dépendance à un service SaaS pour le pilotage. Elle ne touche pas le
  produit (aucune donnée utilisateur, aucun appel depuis l'application) ;
  le périmètre et les décisions restent versionnés dans le dépôt (briefs,
  ADR, CHANGELOG).
- Le backlog n'est pas visible publiquement.

**Neutres** :
- Prérequis : intégration GitHub de Linear connectée à l'organisation
  `Sobr-cc`.
- Le TODO existant de `deny.toml` sera renommé quand son ticket existera.

## Liens

- CLAUDE.md §2, §4.3, §4.4, §11, §14 (mis à jour par cet ADR)
- CONTRIBUTING.md
- `docs/ROADMAP.md`
- [How we work](https://linear.app/sobrcc/document/how-we-work-cb9e7156febb) and "Linear organization and agents" (private claude.ai link, https://claude.ai/artifact/Cxbf5v5XMs8F5unZfEsbSg) (Amendment 1)
- SOB-12 (branch names), SOB-87 (amend in place), SOB-36 (Amendment 1), SOB-138 (follow-up in CLAUDE.md, CONTRIBUTING.md and the skill)
