# CLAUDE.md — Contexte projet Sobr.ia

> **Ce fichier est lu en priorité par Claude Code à chaque session.**
> Il contient tout le contexte indispensable pour produire du code aligné sur le projet, ses contraintes, et ses conventions.

---

## 1. Projet en une phrase

**Sobr.ia** est une application native multi-plateforme (Rust + Tauri 2 + SvelteKit) qui mesure et visualise l'impact environnemental de l'usage des LLMs, exploite les datasets officiels du défi data.gouv.fr (**ComparIA** + **RTE IRIS** sites industriels), apporte un angle territorial français au niveau IRIS, et est accompagnée d'une extension navigateur, d'un dataset consolidé, et d'une méthodologie scientifique validée — candidature au défi data.gouv.fr « L'impact environnemental de l'IA générative ».

---

## 2. Documents de référence (à lire avant d'agir)

| Document | Quand le lire |
|----------|---------------|
| `docs/CAHIER-DES-CHARGES-v1.0.md` | Toujours en premier, donne le périmètre complet |
| Linear (`linear.app/sobrcc`, équipe `SOB`) | Pour savoir ce qui est en cours — source de vérité des statuts (ADR-0018) |
| `docs/ROADMAP.md` | Plan initial, conservé pour l'historique (statuts plus tenus) |
| `docs/adr/*.md` | Pour respecter les décisions architecturales |
| `docs/adr/ADR-0009-medallion-architecture.md` | **Avant tout travail d'ingestion** — pattern Copper/Silver/Gold imposé |
| `briefs/chantiers/Cxx-*.md` | Le brief du chantier lié au ticket Linear en cours |
| `docs/sources/CATALOGUE-SOURCES.md` | Avant tout travail sur l'ingestion |
| `docs/methodology/AFNOR-SPEC-2314-synthese.md` | Avant tout travail sur l'estimateur |

---

## 3. Stack technique imposée

| Couche | Technologie | Version cible |
|--------|-------------|---------------|
| Wrapper natif | Tauri | 2.x |
| Backend | Rust | stable (≥ 1.79) |
| Frontend | SvelteKit + TypeScript | SvelteKit 2.x, TS 5.x |
| Dataviz | Observable Plot + D3 | Plot 0.6+, D3 7.x |
| DB transactionnelle | SQLite (WAL) via `rusqlite` | rusqlite ≥ 0.31 |
| DB analytique | DuckDB via `duckdb-rs` | duckdb-rs ≥ 1.0 |
| ETL Rust | reqwest + serde + polars-rs | dernière stable |
| Notebook | Quarto | 1.4+ |
| Extension navigateur | WebExtension MV3 (TypeScript) | Chrome 120+, Firefox 120+ |
| Versionnage données | DVC | 3.x |
| CI/CD | GitHub Actions | - |

**Anti-patterns à proscrire** :
- ❌ Pas d'Electron (le projet est anti-Electron par principe).
- ❌ Pas de Node.js comme runtime (sauf build / extension).
- ❌ Pas de framework UI lourd côté Svelte (Skeleton CSS custom léger uniquement).
- ❌ Pas de tracking / télémétrie sans opt-in explicite.
- ❌ Pas d'envoi de prompts utilisateurs vers un serveur externe.
- ❌ Pas de dépendances optionnelles non-justifiées dans un ADR.
- ❌ Pas de clé API bloquante (v1.0 doit être installable et fonctionnelle sans aucune clé).
- ❌ Pas de méthodologie environnementale parallèle à EcoLogits sans justification : **on aligne sur la méthodologie officielle ComparIA / Data for Good**.

**Datasets prioritaires (Tier 1, défi)** :
- 🎯 **ComparIA** (Beta.gouv / Ministère de la Culture) — 5 GB Parquet, Etalab 2.0, méthodologie EcoLogits intégrée
- 🎯 **RTE/NaTran/Teréga IRIS** (ODRÉ) — consommation industrielle élec + gaz par maille IRIS, Etalab 2.0
- Catalogue complet : `docs/sources/CATALOGUE-SOURCES.md`

---

## 3bis. Architecture médaillon — RÈGLE STRICTE

Toute donnée externe traverse OBLIGATOIREMENT le pipeline à 3 couches défini dans `ADR-0009` :

```
🟫 Copper  → données brutes immutables, datées, hashées (format d'origine)
🥈 Silver  → Parquet validé, normalisé, schéma versionné (lineage vers Copper)
🥇 Gold    → referentiel.sqlite + analytics.parquet (consommé par l'app)
```

**Règles non-négociables** :
1. ❌ Jamais d'accès direct du code applicatif aux sources externes : seul `sobria-ingest` parle aux APIs.
2. ❌ Jamais de transformation *ad hoc* en dehors du trait `DataLayer`.
3. ❌ Jamais de Gold sans Silver, jamais de Silver sans Copper validé.
4. ✅ Toute nouvelle source = un seul trait à implémenter (`ingest_copper`, `promote_silver`, `contribute_gold`).
5. ✅ Schémas Silver versionnés (`schemas/silver/<entity>-v<n>.json`).
6. ✅ Tests `proptest` + golden files sur chaque transformation.
7. ✅ Commande unique : `cargo run -p sobria-ingest -- pipeline run`.
8. ✅ Orchestration via `dvc.yaml` — `dvc repro` doit reproduire à l'identique.

Voir `ADR-0009-medallion-architecture.md` pour le détail (interface du trait, exemples, stages DVC).

---

## 4. Conventions de code

### 4.1 Rust

- **Édition** : 2021
- **Linter** : `cargo clippy -- -D warnings`
- **Formatteur** : `cargo fmt` (rustfmt config dans `rustfmt.toml`)
- **Tests** : `cargo test` + couverture mesurée par `cargo-tarpaulin`
- **Nommage crates** : `sobria-*` (snake_case mais préfixe `sobria`)
- **Organisation** : workspace Cargo, une crate par responsabilité (voir CDC §7.1)
- **Erreurs** : `thiserror` pour les erreurs publiques, `anyhow` pour les binaires
- **Async** : `tokio` (multi-thread runtime), `async-trait` si nécessaire
- **Sérialisation** : `serde` partout, formats stables (JSON, MessagePack, Parquet)
- **Documentation** : rustdoc (`///` et `//!`) sur tout item public, une ligne courte par item, un exemple seulement quand il est utile ; les autres commentaires suivent la règle du §13

### 4.2 TypeScript / SvelteKit

- **Strict mode** : `"strict": true` dans tsconfig
- **Linter** : ESLint + `@typescript-eslint`, règles strictes
- **Formatteur** : Prettier (config partagée racine)
- **Composants Svelte** : un composant = un dossier si plus de 100 lignes
- **Stores** : préférer les stores typés Svelte 5 (runes) si on adopte la rune syntax
- **Tests** : Vitest + Playwright pour e2e
- **i18n** : `svelte-i18n` ou `@inlang/paraglide`, clés en kebab-case

### 4.3 Commit messages (Conventional Commits)

```
<type>(<scope>): <description>

[corps optionnel]

[footer optionnel]
```

Types : `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`.
Scopes : `core`, `estimator`, `referentiel`, `ingest`, `geoloc`, `import`, `export`, `audit`, `app`, `ui`, `ext`, `docs`, `ci`.

Exemple : `feat(estimator): add Monte-Carlo uncertainty propagation`. Les messages de commit, les titres et les descriptions de PR sont en anglais (§13).

Footer : `Refs: SOB-123` pour lier le commit au ticket Linear ; `Fixes SOB-123` dans la description de PR pour clore le ticket au merge (ADR-0018).

### 4.4 Branching

- `main` : protégée, releases uniquement
- `develop` : intégration
- `feature/sob-123-<short-desc>` : features
- `fix/sob-123-<short-desc>` : corrections
- `docs/sob-123-<short-desc>` : docs only

`sob-123` est l'identifiant du ticket Linear : Linear rattache la branche et la PR au ticket (ADR-0018).

---

## 5. Definition of Done (DoD)

Une tâche est **terminée** quand TOUT est vrai :

- [ ] Code écrit + relu (self-review)
- [ ] Tests unitaires écrits, couverture locale ≥ 80 %
- [ ] `cargo clippy -- -D warnings` passe
- [ ] `cargo fmt --check` passe
- [ ] `npm run lint && npm run check` passent côté front
- [ ] Documentation à jour (`cargo doc`, doc utilisateur si fonctionnalité visible)
- [ ] Commit message respecte Conventional Commits
- [ ] Si la tâche affecte une décision archi → ADR mis à jour
- [ ] Si la tâche touche au référentiel → schéma versionné
- [ ] Si la tâche touche à l'UI → screenshot dans la PR + check a11y (axe)

---

## 6. Méthodologie scientifique (à respecter)

Voir `docs/methodology/` pour les détails.

- Toute formule de calcul est **sourcée** dans le code (commentaire avec DOI/URL).
- Toute hypothèse a une **distribution d'incertitude** documentée.
- Tout résultat est accompagné d'un **intervalle P5-P95**.
- Les calculs sont **reproductibles** à partir d'un seed (`SOBRIA_SEED` env var, défaut 42).
- Le moteur passe les **3 tests de validation croisée** (Luccioni 2023, Patterson 2021, EcoLogits 2024) à ±15 %.

---

## 7. Sécurité et privacy

- **Privacy by design** : tout traitement par défaut local, pas d'envoi externe.
- **Opt-in obligatoire** pour toute télémétrie ou partage.
- **Chiffrement** : `argon2` pour les hash, `rustls` pour TLS, pas d'OpenSSL.
- **Secrets** : aucun secret en clair, `.env` jamais commité, secrets CI via GitHub Encrypted Secrets.
- **Dépendances** : `cargo deny` + `cargo audit` à chaque CI, blocage si vulnérabilité critique.
- **Extension navigateur** : permissions minimales, pas de remote code, audit avant soumission stores.

---

## 8. Frugalité par défaut

Le projet incarne son sujet. Chaque décision technique passe le filtre :

> *« Est-ce que ce choix est le plus frugal acceptable ? »*

Concrètement :
- Préférer compilation à build vs runtime (Svelte vs React, Rust vs Node).
- Éviter dépendances ≥ 1 Mo sauf absolument nécessaire.
- Mesurer et publier l'empreinte CO₂eq de Sobr.ia lui-même (méta-cohérence).
- Builds optimisés en taille (`opt-level = "z"`, `lto = true`, `strip = true`, `panic = "abort"`).

---

## 9. Multi-plateforme

| Plateforme | Statut | Priorité |
|------------|--------|----------|
| Windows | 1ʳᵉ classe | Bloquante v1.0 |
| macOS | 1ʳᵉ classe | Bloquante v1.0 |
| Linux | 1ʳᵉ classe | Bloquante v1.0 |
| Web (Wasm) | 2ᵉ classe | Bloquante v1.0 (démo) |
| Android | 3ᵉ classe | Bonus (Tauri 2 mobile) |
| iOS | 3ᵉ classe | Bonus (Tauri 2 mobile) |

---

## 10. Commandes utiles (à connaître)

```bash
# Bootstrap (à partir du repo cloné)
./scripts/bootstrap.sh             # installe toutes les deps

# Dev
cargo run -p sobria-app             # lance Tauri en dev
cd web && npm run dev               # frontend SvelteKit en hot reload
cd extension && npm run dev         # extension en mode dev

# Tests
cargo test --workspace              # tests Rust
cargo tarpaulin --workspace         # couverture
cd web && npm run test              # tests Vitest
cd web && npm run e2e               # tests Playwright

# Lint
cargo clippy -- -D warnings
cargo fmt --check
cd web && npm run lint && npm run check

# Pipeline médaillon (ADR-0009)
cargo run -p sobria-ingest -- pipeline run                 # tout le pipeline Copper→Silver→Gold
cargo run -p sobria-ingest -- pipeline run --incremental   # ne ré-ingère que ce qui a changé
cargo run -p sobria-ingest -- pipeline run --source comparia    # source unique : ComparIA
cargo run -p sobria-ingest -- pipeline run --source rte-iris    # source unique : RTE IRIS
cargo run -p sobria-ingest -- copper --all                 # juste la couche brute
cargo run -p sobria-ingest -- silver --all                 # promotion Copper→Silver
cargo run -p sobria-ingest -- gold                         # construction du Gold final
cargo run -p sobria-ingest -- validate                     # vérifie l'intégrité + lineage
dvc repro                                                  # rejoue les stages modifiés
dvc push                                                   # publie le nouveau référentiel sur le remote

# Build production
./scripts/build-all.sh              # produit tous les binaires + web + ext

# Méthodologie
quarto render notebook/validation.qmd
```

---

## 11. Workflow et règles de travail

Source : le document Linear [« How we work »](https://linear.app/sobrcc/document/how-we-work-cb9e7156febb). Linear (workspace `sobrcc`, équipe `SOB`) pilote l'avancement (ADR-0018). Tous les agents sont des agents Claude ; les fonctions IA de Linear restent désactivées.

### Qui fait quoi

| Acteur | Fait | Ne fait jamais |
|--------|------|----------------|
| Thibault | Priorise, valide le design, relit et merge les PR, gère comptes, paiements, stores et décisions juridiques | — |
| Orchestrateur (session principale de Claude Code, skill `/orchestrate`) | Trie Triage, écrit les tâches avec le skill `linear-issue-writing`, découpe ce qui dépasse 8 points, rédige la mise à jour hebdomadaire de chaque projet | Coder, merger, fermer une tâche |
| Claude Design | Produit les écrans d'une tâche Design (pas d'accès à Linear : Thibault ou Claude joint le lien à la tâche) | Inventer un écran absent des routes |
| Claude Code (dans le dépôt, avec le MCP Linear) | Prend une tâche Todo `agent:claude-code`, la passe In Progress, branche au nom de l'identifiant, code, vérifie, ouvre une PR liée, passe In Review avec un résumé | Merger, élargir le périmètre (il crée une tâche en Triage), toucher une tâche `human-only` |

Les rôles de Claude Code sont décrits dans `.claude/agents/` (un fichier par rôle). Pour décider, Thibault répond en commentaire d'une tâche Decision. Pour piloter, il change la priorité ou le label Role, ou renvoie une tâche en Backlog.

### Règles de travail

- Une tâche = une PR, 8 points au plus. Au-delà, découpage en sous-tâches avant Todo.
- Chaque tâche cite les écrans concernés par leur identifiant de route (E1, Z2, V3…) et lie les deux documents de référence (routes et architecture).
- Branches, commits et PR : §4.3 et §4.4 (ADR-0018) ; commits, titres et descriptions de PR en anglais (§13). L'intégration GitHub relie la PR à la tâche et fait avancer l'état.
- En passant une tâche In Review, un commentaire en trois parties, signé du rôle :

  ```
  Done: what changed, in one to three sentences.
  Verified: the command or steps run, and the result.
  Left: anything not done, follow-up issue links, or questions for Thibault.
  -- role:rust (model:sonnet)
  ```

- Tout écart de périmètre devient une nouvelle tâche en Triage, jamais un ajout silencieux.
- Au plus trois tâches In Review à la fois ; quand elles sont trois, les agents ne prennent rien de nouveau et attendent.
- Le label `human-only` couvre comptes, paiements, stores, secrets et juridique : aucun agent n'y travaille.
- Toute décision qui change un ADR passe par une tâche Docs et une PR sur l'ADR, validée par Thibault.
- Chaque vendredi, mise à jour de chaque projet actif : santé (On track, At risk, Off track) et trois lignes (Progress, Risks, Next), moins de 120 mots, des faits seulement.
- Décision du 30 octobre : pas de monolithe, les projets vivent dans des dépôts séparés de l'organisation GitHub Sobr-cc ; le découpage est fixé par SOB-65.

### Prêt (avant qu'une tâche passe en Todo)

- Titre à verbe, résultat décrit, moins de 70 caractères.
- Why, What, Acceptance criteria et How to verify remplis.
- Estimation posée, 8 au plus.
- Labels Type, Area, Agent, Role, Model, projet et priorité posés.
- Aucune question ouverte. S'il en reste une, la tâche reste en Backlog et une tâche Decision ou Research lui est liée.

### Terminé (tâches de code)

En plus de la définition du §5 (clippy, tests, lint, CHANGELOG) :

- `tsc --noEmit` passe.
- `/simplify` et `/code-review` lancés sur le diff, remarques traitées.
- Une vraie vérification : `/verify`, un test Playwright, ou un appel `curl` sur le point d'accès modifié.

### Estimation et modèle

Les points mesurent la taille de la tâche, relecture comprise : 1 (moins d'une heure), 2 (une demi-journée), 3 (environ un jour), 5 (2 à 3 jours), 8 (environ une semaine, maximum). Le label `Model` suit la taille et le risque, pas le prestige du rôle : `model:haiku` pour 1 point mécanique sans décision, `model:sonnet` pour 2 à 5 points sur un chemin connu (défaut), `model:opus` pour 8 points ou un risque élevé. L'orchestrateur lance l'agent avec ce modèle ; le fichier de l'agent ne porte que le modèle par défaut du rôle. Fable n'est jamais le défaut : seulement si Opus a échoué deux fois, avec l'accord de Thibault. On ne monte d'un cran qu'après deux tentatives bloquées : l'agent le note en commentaire et l'orchestrateur change le label.

### Déroulé d'une session

À chaque nouvelle session :

1. Lire `CLAUDE.md` (ce fichier).
2. Identifier le ticket Linear (`SOB-123`) de la tâche et lire le brief du chantier lié (`briefs/chantiers/Cxx-*.md`).
3. Lire les ADR pertinents pour la tâche.
4. Pour chaque tâche :
   a. Comprendre — relire CDC §pertinent + ADR + research/biblio si scientifique.
   b. Plan rapide — 3-7 sous-étapes.
   c. Implémenter — petit commit par sous-étape si possible.
   d. Tester — ajouter tests + faire passer linters.
   e. Documenter — code docs + doc utilisateur si visible.
   f. PR description : référence CDC + ADR + lien brief + `Fixes SOB-123`.

---

## 12. Personnes & rôles

| Rôle | Personne | Outils |
|------|----------|--------|
| Chef de projet, architecte, méthodologie | Claude Cowork (assistant) | Docs, planning, validation |
| Réalisation code | Claude Code | Toutes crates, frontend, extension |
| Décisions produit, validation | Thibault | Validation finale, choix UX |
| Mentor scientifique | (Ecolab/ADEME) | Relecture méthodologie |
| Testeurs utilisateurs | (5 personnes) | Entretiens S11 |

---

## 13. Ce que tu ne dois jamais faire

- ❌ Ne JAMAIS implémenter un calcul scientifique sans source documentée.
- ❌ Ne JAMAIS introduire une dépendance non listée dans le CDC ou un ADR.
- ❌ Ne JAMAIS commiter un secret, une clé API, un dataset complet (DVC pour ça).
- ❌ Ne JAMAIS faire un `force push` sur `main` ou `develop`.
- ❌ Ne JAMAIS skipper les tests pour "gagner du temps".
- ❌ Ne JAMAIS introduire de tracking utilisateur silencieux.
- ❌ Ne JAMAIS écrire en français (Linear, commentaires de code, rustdoc, commits, titres et descriptions de PR, ADR, CHANGELOG, README) : toute la documentation est en anglais uniquement. Seuls les textes du produit visibles par l'utilisateur (interface, i18n) sont en français et en anglais (décision du 30 octobre). La règle vaut pour ce qui s'écrit à partir de maintenant : un fichier existant en français est traduit la prochaine fois qu'un agent le modifie (les ADR acceptés ne sont pas modifiés ; les nouveaux ADR sont en anglais). Les livrables du CDC (NF-16, L5) restent en français et en anglais tant que Thibault n'a pas listé les documents destinés aux utilisateurs.
- ❌ Ne JAMAIS commenter ce que le code dit déjà : commentaires au strict minimum, une ligne courte pour une fonction complexe ou un choix non évident. Exceptions : le rustdoc des items publics (§4.1), les sources exigées par le §6 et le §14 (formules, hypothèses, valeurs numériques), `TODO(SOB-123)` (§14).
- ❌ Ne JAMAIS résoudre une ambiguïté du CDC sans demander confirmation.

---

## 14. Ce que tu dois faire systématiquement

- ✅ Citer la source de toute formule, hypothèse, valeur numérique.
- ✅ Ajouter `#[cfg(test)]` mod tests à chaque module Rust non trivial.
- ✅ Préfixer les TODOs : `// TODO(SOB-123):` avec un ticket Linear existant (ADR-0018).
- ✅ Mesurer la taille des binaires à chaque build de release.
- ✅ Mettre à jour CHANGELOG.md à chaque feature/fix.
- ✅ Demander si une ambiguïté apparaît dans le CDC ou le brief.

---

*Ce fichier évolue avec le projet. Toute modification structurante implique un ADR.*
