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
| [ADR-0019](./ADR-0019-decoupage-en-depots.md) | Découper le code en plusieurs dépôts de l'organisation Sobr-cc | Proposed |
| [ADR-0020](./ADR-0020-renommage-sobr-cc.md) | Renommer Sobr.ia en Sobr.cc | Proposed |
| [ADR-0021](./ADR-0021-single-app-desktop-and-web.md) | One Svelte app, shipped on desktop and on the web (amends rule 2 of ADR-0017) | Proposed |

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
