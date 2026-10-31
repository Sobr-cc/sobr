# Architecture Decision Records — Sobr.ia

Les ADR documentent les décisions structurantes du projet. Chaque ADR :
- Est **immuable** une fois `Accepted`. Pour le changer, on crée un nouvel ADR `Supersedes`.
- Suit le format MADR (Markdown Architecture Decision Records).
- Est numéroté `ADR-NNNN-titre-court.md`.

## Index

| ID | Titre | Statut |
|----|-------|--------|
| [ADR-0001](./ADR-0001-rust-tauri.md) | Choix de Rust + Tauri 2 comme stack native | Accepted |
| [ADR-0002](./ADR-0002-sveltekit.md) | SvelteKit + TypeScript pour le frontend | Accepted |
| [ADR-0003](./ADR-0003-sqlite-duckdb.md) | Dual-store SQLite (ACID) + DuckDB (OLAP) | Accepted |
| [ADR-0004](./ADR-0004-monte-carlo.md) | Propagation d'incertitude par Monte-Carlo N=10⁴ | Accepted |
| [ADR-0005](./ADR-0005-webextension-mv3.md) | Extension WebExtension Manifest V3 | Accepted |
| [ADR-0006](./ADR-0006-licences.md) | Licences MIT + Etalab 2.0 + CC-BY 4.0 | Accepted |
| [ADR-0007](./ADR-0007-dvc.md) | DVC pour le versionnage du référentiel | Accepted |
| [ADR-0008](./ADR-0008-observable-plot.md) | Observable Plot + D3 pour la dataviz | Accepted |
| [ADR-0009](./ADR-0009-medallion-architecture.md) | Architecture médaillon Copper/Silver/Gold automatique | Accepted |
| [ADR-0010](./ADR-0010-personas-and-module-gating.md) | Personas et gating modulaire par préférences utilisateur | Accepted |
| [ADR-0011](./ADR-0011-reduction-perimetre-v1-0.md) | Réduction du périmètre v1.0 à 13 modules essentiels | Accepted |
| [ADR-0012](./ADR-0012-multi-methodology-engine.md) | Catalogue multi-méthodologie d'estimation d'empreinte LLM | Accepted |
| [ADR-0013](./ADR-0013-extension-pairing-team-mode.md) | Extension navigateur + pairing local + mode Équipe self-hosted | Phases 1 and 2 Implemented, phase 3 Backlog |
| [ADR-0014](./ADR-0014-dual-track-local-cloud.md) | Stratégie dual-track : local-first + cloud opt-in | Accepted |
| [ADR-0015](./ADR-0015-privacy-mode-equipe.md) | Périmètre privacy du Mode Équipe (k-anonymat + partage opt-in) | Accepted |
| [ADR-0016](./ADR-0016-politique-visibilite-deploiement.md) | Politique de visibilité par déploiement (étend ADR-0015) | Accepted |
| [ADR-0017](./ADR-0017-contrat-demo-web.md) | Contrat démo web (fixtures du moteur réel hors Tauri) | Accepted |
| [ADR-0018](./ADR-0018-pilotage-linear.md) | Piloter l'avancement dans Linear | Accepted |
| [ADR-0019](./ADR-0019-split-code-into-repos.md) | Split the code into several repos of the Sobr-cc organization | Accepted (2026-10-31) |
| [ADR-0020](./ADR-0020-renommage-sobr-cc.md) | Renommer Sobr.ia en Sobr.cc | Proposed |
| [ADR-0021](./ADR-0021-single-app-desktop-and-web.md) | One Svelte app, shipped on desktop and on the web (amends rule 2 of ADR-0017) | Proposed |
| [ADR-0024](./ADR-0024-kubernetes-platform-on-proxmox.md) | Kubernetes platform on Proxmox | Proposed |

## Format à respecter

```markdown
# ADR-NNNN — Titre court à l'impératif

- **Statut** : Proposed | Accepted | Deprecated | Superseded by ADR-MMMM
- **Date** : YYYY-MM-DD
- **Décideurs** : Thibault, Cowork
- **Contexte technique** : Sprint Sx

## Contexte et énoncé du problème
[Décrit la situation et la question à trancher]

## Options envisagées
- Option A — …
- Option B — …
- Option C — …

## Décision
[Option retenue + une phrase explicative]

## Conséquences
**Positives** : …
**Négatives** : …
**Neutres** : …

## Liens
- CDC §X.Y
- Autres ADR
```
