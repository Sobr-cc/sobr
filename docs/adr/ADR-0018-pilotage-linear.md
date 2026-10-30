# ADR-0018 — Piloter l'avancement dans Linear

- **Statut** : Accepted
- **Date** : 2026-10-30
- **Décideurs** : Thibault
- **Contexte technique** : post-C46, préparation v1.0 (C35, C36) et v1.1 (C31)

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
