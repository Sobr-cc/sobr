# ADR-0019 — Découper le code en plusieurs dépôts de l'organisation Sobr-cc

- **Statut** : Proposed (passe à Accepted quand Thibault approuve la PR)
- **Date** : 2026-10-30
- **Décideurs** : Thibault (décision) ; recommandation du CTO
- **Contexte technique** : Foundations, jalon Structure (SOB-65)

> **Summary (EN).** As decided on 2026-10-30 ("no monolith"), the code leaves
> the single repository, but along release units, not along folders. `sobr`
> keeps what is compiled and shipped together: the Rust engine, the desktop
> app, the organization server, the Svelte app, the ADR register and the
> docs. Five pieces get their own repo in three waves: `.github`, `site` and
> `design-system`, then `extension`, then `data`. `vscode` and `claude-code`
> are created when their projects start. Shared Rust code and front-end
> assets are pinned by git tag; every cross-repo interface becomes a
> versioned contract. Each wave is a safe stopping point. Five questions for
> Thibault come first.

## Points à trancher

Cinq questions pour Thibault ; les recommandations s'appuient sur la section
Décision.

1. **Le cœur reste dans un seul dépôt (`sobr`).** Recommandé : oui. Moteur,
   app de bureau, serveur et app Svelte sont compilés et livrés ensemble
   (ADR-0014 : même binaire partout) ; les séparer multiplie les PR par
   changement, relues une à une par Thibault. C'est le point où cette
   recommandation s'écarte le plus de « contre les monolithes » : `sobr`
   reste le plus gros dépôt. Alternative : sortir `web/` dans son propre
   dépôt, au prix d'un artefact versionné à récupérer par l'app et par le
   serveur, et de DTO à synchroniser entre deux dépôts.
2. **Le code Rust partagé passe par une dépendance `git` épinglée sur un
   tag**, sans crates.io pour l'instant. Recommandé : oui. Alternatives :
   crates.io (compte humain, API à stabiliser) ou un dépôt `core` (deux PR
   dès que le modèle de sessions change).
3. **Tout `docs/` reste dans `sobr`**, et le site le récupère en une fois.
   Recommandé : oui. Alternative : docs par dépôt, un fetch par dépôt côté
   site.
4. **Placement des composants Claude Code.** Recommandé : l'import des
   journaux est une crate de `sobr` (il s'exécute dans l'app et écrit dans
   son magasin de sessions) ; le plugin va dans `claude-code` ; le serveur
   MCP va dans `claude-code` s'il est un processus séparé qui parle à l'API
   HTTP, dans `sobr` s'il est un module du serveur. L'ADR du projet « Claude
   Code and MCP » confirme.
5. **Information, pas de réponse attendue ici : livraison des données à
   l'app** (catalogue de modèles, fichiers de territoire). Aujourd'hui rien
   ne relie le Gold d'`ingest` à l'app (voir Contexte). Une issue d'ADR dédiée
   est prévue en vague 3 ; elle conditionne le projet « Model catalog », pas
   les déplacements.

## Contexte et énoncé du problème

Le 30 octobre 2026, Thibault a décidé que les projets vivent dans des dépôts
séparés de l'organisation GitHub `Sobr-cc`, sans monolithe (commentaire sur
SOB-33). Tout est aujourd'hui dans `github.com/Sobr-cc/sobr` : 11 crates,
`web/`, `web-team/`, `extension/`, `site/`, le design system, le pipeline de
données (DVC) et la documentation. La décision « Dépôts » du document de
routes (« dépôt principal sobr (workspaces Cargo et pnpm), site, .github »)
ne place ni l'extension, ni les données, ni le design system, et ne dit pas
comment ils partagent du code. Elle parle aussi de workspaces pnpm, alors que
chaque dossier JavaScript utilise npm avec son propre `package-lock.json`.

Constats sur `main` au 2026-10-30 :

1. **Un bloc compilé et livré ensemble.** `sobria-app` embarque `web/build`
   (`crates/sobria-app/tauri.conf.json`, `frontendDist`).
   `sobria-team-aggregator` embarque `web-team/build` avec `rust-embed`
   (`src/server/embedded_web.rs`) ; l'ADR-0021 « app unique » (SOB-14) remplace
   `web-team/` par `web/`. L'app et le serveur partagent `core`, `audit` et
   `export`. Les types TypeScript de `web/` recopient à la main les DTO Rust
   (`web/src/lib/api.ts`).
2. **Des liens faibles ailleurs.** `extension/` n'a aucune dépendance de
   code : le protocole du bridge et le client d'API y sont recopiés, les
   logos copiés par script. `site/` ne dépend que de `docs/`
   (`scripts/sync-docs.mjs`). Le design system n'est consommé que par le
   script de logos de l'extension. `sobria-ingest` n'utilise de
   `sobria-core` que `DEFAULT_SEED` et de `sobria-geoloc` que les
   enregistrements de datacenters ; ses schémas sont lus à la compilation
   (`include_str!`).
3. **Une dérive déjà présente.** Les jetons CSS de `web/` sont recopiés dans
   trois autres dossiers, les 8 polices existent en 3 exemplaires, l'ancienne
   URL `BkOff-fr/defis-lia-generatif` est en dur dans 17 fichiers, un tag
   `vX.Y.Z` déclenche 3 workflows de release, deux workflows nocturnes DVC
   rejouent le même pipeline (`ingest-nightly` référence un remote `origin`
   absent de `.dvc/config`), et `ci.yml` n'a aucun filtre de chemin.
4. **Un lien données → app qui n'existe pas encore.** L'app crée
   `referentiel.sqlite` dans le répertoire de données de l'utilisateur et y
   écrit ses propres tables (préférences, objectifs, projets). Aucun workflow
   de release ne fournit le Gold produit par `sobria-ingest`. Le contrat
   `data` → `sobr` est donc à définir, pas à préserver.
5. **Une contrainte de gouvernance.** Chaque merge demande l'approbation de
   Thibault (« How we work »). Un changement qui touche N dépôts coûte N
   relectures.
6. **Des décisions déjà prises.** Le site va dans `Sobr-cc/site` (SOB-69), les
   fichiers communs dans `Sobr-cc/.github` (SOB-70), le dépôt infra est nommé
   par l'ADR SOB-73, `web-team` est supprimé après fusion (SOB-14, SOB-64),
   `sobria-import` est supprimé (SOB-67), et aucune issue d'un autre projet
   ne touche un dossier avant son Deep cleanup (projet Foundations).

Question : quels dépôts, avec quel contenu, quel partage de code, dans quel
ordre ?

## Critère de découpe

Deux règles, qui servent aussi à placer les futurs composants :

1. Ce qui est compilé dans le même binaire, ou embarqué dedans, et change en
   même temps reste dans le même dépôt.
2. Ce qui a son propre canal de livraison (magasin d'extensions, hébergeur
   statique, pipeline de données), ou est une ressource partagée par
   plusieurs dépôts, et ne parle au reste que par un contrat versionné
   (protocole du bridge, API HTTP, fichier SQLite, jetons de design) a son
   propre dépôt.

## Options envisagées

| Option | Principe | Coût | Risque | Réversibilité |
|--------|----------|------|--------|---------------|
| A | Statu quo : un seul dépôt | nul | contredit la décision du 30 octobre | — (écartée d'office) |
| B | Un dépôt par dossier ou par crate (une quinzaine) | élevé | élevé | difficile |
| C | Un dépôt par unité de livraison, en 3 vagues | ≈ 50 points (§6) | moyen, concentré sur `data` | bonne : chaque vague est un point d'arrêt |
| D | Seulement `site` et `.github` (texte actuel du document de routes) | ≈ 8 points | faible | excellente |

- **B** : l'app de bureau et le serveur embarquent chacun le build de
  l'interface, et les DTO sont recopiés à la main entre Rust et `web/`.
  Séparer `web/` des crates transforme chaque changement de DTO en deux PR
  plus un changement de version, et les versions croisées entre `core`,
  `estimator`, `audit` et `export` en matrice de compatibilité. Fusionner
  des dépôts après coup est possible mais long.
- **D** : peu coûteuse et réversible, mais laisse dans le dépôt principal des
  composants aux rythmes de livraison différents (extension, données, design
  system). C'est une partie de la vague 1 de l'option C.
- **C** : coupe là où le couplage est déjà faible ou recopié à la main, et
  garde ensemble ce qui est livré ensemble. Le gain est réel pour
  l'extension (revue des magasins indépendante), les données (rythme
  nocturne ou hebdomadaire) et le site (déploiement statique).

## Décision

**Option C est proposée.** Les issues Linear dérivent des sections 1 à 5
(section 6).

### 1. Dépôts cibles

Les noms suivent ceux déjà posés (`sobr`, `site`, `.github`). Pas de dépôt
créé avant d'avoir du contenu à y mettre.

| Dépôt | Contenu | Rôle propriétaire | Création |
|-------|---------|-------------------|----------|
| `sobr` (existant) | crates du moteur, de l'app de bureau et du serveur ; app Svelte (`web/`) ; registre des ADR et `docs/` ; outillage de test et de recherche (détail §2) | Lead Rust (`web/` : Svelte front end) | existe |
| `.github` | profil de l'organisation, fichiers communautaires, modèles d'issue et de PR, workflows CI réutilisables | DevOps | vague 1 |
| `site` | site Astro (S1 à S4, recherche) | Svelte front end | vague 1 |
| `design-system` | design system, jetons CSS, polices, logos, versions taguées | Product designer | vague 1 |
| `extension` | extension navigateur (Chrome, Firefox, Edge ; ADR-0005) | Dev extensions | vague 2 |
| `data` | `sobria-ingest`, schémas, pipeline DVC, workflow nocturne, catalogue de modèles, jeu de données de recherche | Data and method | vague 3 |
| `infra` | réservé : nom et contenu fixés par SOB-73 (création : SOB-74) | Infra | projet Infrastructure |
| `vscode` | extension VS Code | Dev extensions | à l'ouverture du projet « VS Code extension » |
| `claude-code` | plugin Claude Code (hooks, ligne d'état) et, selon le point 4, serveur MCP | Dev integrations | à l'ouverture du projet « Claude Code and MCP » |

### 2. Destination de chaque dossier actuel

| Aujourd'hui | Destination | Remarque |
|-------------|-------------|----------|
| `crates/sobria-core`, `sobria-referentiel`, `sobria-estimator`, `sobria-geoloc`, `sobria-audit`, `sobria-export`, `sobria-bridge`, `sobria-app`, `sobria-team-aggregator` | `sobr` | |
| `crates/sobria-ingest` | `data` | |
| `crates/sobria-import` | aucune | supprimée par SOB-67 ; l'import des journaux Claude Code suit le point 4 |
| `web/` | `sobr` | |
| `web-team/` | aucune | fusionné dans `web/` puis supprimé (SOB-14, SOB-64) ; ne déménage jamais |
| `extension/` | `extension` | |
| `site/` | `site` | |
| `sobr-ia-design-system/` | `design-system` | avec ses polices et leurs licences ; le dossier prend le nom du dépôt |
| `schemas/` | `data` | lus à la compilation par `sobria-ingest` |
| `dvc.yaml`, `.dvc/`, `.dvcignore` | `data` | |
| `tools/` (`fixturegen`) | `sobr` | génère les fixtures de `web/` depuis `core` et `estimator` (ADR-0017) |
| `deploy/` | `sobr` | livré avec les binaires ; l'hébergement propre à Sobr relève de SOB-73 |
| `scripts/bootstrap.sh`, `scripts/build-all.sh` | `sobr` | les étapes `fetch` d'ingest passent dans `data` |
| `scripts/sync-docs.mjs` | `site` | remplacé par un fetch de `docs/` (§3) |
| `docs/` (dont `docs/adr/`) | `sobr` | arbre unique que le site récupère ; registre unique des ADR, une décision concerne l'organisation et non un dépôt |
| `briefs/`, `research/`, `notebook/` | `sobr` | le notebook valide l'estimateur |
| `.claude/` (agents, skills `orchestrate` et `linear-issue-writing`) | `sobr` | source des règles de travail ; les autres dépôts en reçoivent la part qui les concerne (§4) |
| `.github/workflows/ci.yml`, `app-release.yml`, `team-aggregator-release.yml`, `team-docker.yml`, `zenodo.yml` | `sobr` | les jobs de `ci.yml` pour les dossiers partis les suivent |
| `.github/workflows/extension-release.yml` | `extension` | |
| `.github/workflows/site-deploy.yml` | `site` | |
| `.github/workflows/dvc-nightly.yml`, `ingest-nightly.yml` | `data` | fusionnés en un seul |
| `CODE_OF_CONDUCT.md`, `SECURITY.md`, `CONTRIBUTING.md` | `.github` | SOB-70 ; un dépôt garde le sien s'il en a besoin d'un différent |
| `LICENSE-DATA` | `data` | |
| `LICENSE`, `LICENSE-DOCS`, `.zenodo.json`, `Cargo.toml`, `Cargo.lock`, `deny.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `CLAUDE.md`, `CHANGELOG.md`, `README.md`, `.env.example`, `.gitattributes`, `.gitignore` | `sobr` | chaque autre dépôt a ses copies ou les siens ; `LICENSE` est copiée partout (ADR-0006) |

### 3. Partage de code et contrats

Pas de registre de paquets pour l'instant : un compte crates.io ou npm est
une action humaine et engage la stabilité d'une API publique. Le passage à un
registre plus tard ne change que la ligne de dépendance.

| Surface partagée | Producteur → consommateurs | Mécanisme | Versionnement |
|------------------|----------------------------|-----------|---------------|
| Crates `sobria-core` et `sobria-geoloc` | `sobr` → `data` | dépendance Cargo `git` + `tag` | tag de release de `sobr` |
| Jetons CSS, polices, logos | `design-system` → `web/`, `extension`, `site`, `vscode` | dépendance npm `github:Sobr-cc/design-system#vX.Y.Z`, fichiers prêts à l'emploi ; remplace les copies | SemVer ; majeur = jeton retiré ou renommé |
| Protocole du bridge (ADR-0013) | `sobr` → `extension` | JSON Schema généré depuis les types Rust, publié avec chaque release ; test de contrat côté extension | version de protocole dans les messages |
| API HTTP du serveur | `sobr` → `extension`, `vscode`, `claude-code`, app | OpenAPI publiée avec chaque release | préfixe `/api/v1` ; un changement cassant ouvre `/api/v2` en parallèle |
| Référentiel et catalogue (ADR-0007, ADR-0009) | `data` → `sobr` | artefact publié en release de `data` (CalVer, `MANIFEST.sha256`) ; réception côté app : point 5 | version de schéma + CalVer |
| `docs/` | `sobr` → `site` | clone partiel à la construction, ref épinglée, reconstruction à chaque changement | ref de `sobr` |
| Jobs CI communs | `.github` → tous | workflows réutilisables | tag |

### 4. Historique, PR ouvertes, CI, DVC, releases

- **Historique** : chaque dossier part avec son historique (`git filter-repo`
  sur un clone neuf). Auteurs, dates et `Refs: SOB-n` sont conservés ; les
  numéros de PR et d'issues GitHub restent dans `sobr`, leurs liens restent
  valides.
- **PR ouvertes** : un dossier ne part pas tant qu'une PR le touche ; elle est
  mergée, fermée ou rejouée dans le nouveau dépôt.
- **CI** : chaque dépôt reçoit son `ci.yml`, extrait de l'actuel. Le
  `ci.yml` de `sobr` perd les jobs partis dans la même PR que la suppression
  du dossier : CI verte des deux côtés.
- **Releases** : `sobr` garde les tags `vX.Y.Z` pour l'app et le serveur, qui
  sortent ensemble (ADR-0014). `extension` a ses propres tags dans son
  dépôt. La compatibilité se lit dans la version de protocole et `/api/v1`,
  avec une matrice dans le README de chaque dépôt.
- **DVC** : `dvc.yaml`, `.dvc/`, les schémas et `sobria-ingest` partent
  ensemble, car les stages hachent `crates/sobria-ingest/src` et `schemas/` :
  les séparer casserait `dvc repro`. Un seul workflow nocturne. La commande
  unique de CLAUDE.md §3bis (`pipeline run`) s'exécute alors dans `data`.
  Critère de réussite : `dvc repro` dans `data` redonne le même
  `MANIFEST.sha256` qu'avant le déplacement.
- **Actions humaines à chaque dépôt** : création, protection de `main`
  (SOB-17), secrets de déploiement ou de remote DVC, couverture de
  l'intégration GitHub de Linear (SOB-46).
- **Règles de travail des agents** : `CLAUDE.md` §11, les skills et les
  fichiers d'agents sont dans `sobr` (`.claude/`, SOB-43, SOB-44). Chaque nouveau dépôt
  reçoit un `CLAUDE.md` court (son périmètre, ses commandes, renvoi vers les
  règles communes). Conventions de branches, de commits et de TODO de
  l'ADR-0018 inchangées.

### 5. Ordre des déplacements

**Conditions communes à toute vague** : cet ADR approuvé ; CI verte et `main`
protégée (SOB-8, SOB-17) ; liens GitHub corrigés (SOB-40, pour que chaque
dossier parte avec les bonnes URL) ; Deep cleanup du dossier terminé s'il y
figure (règle Foundations).

1. **Vague 1 : `.github`, `site`, `design-system`.** Hors Deep cleanup, aucun
   couplage de code ; la seule dépendance est `docs/` pour le site. Part dès
   les conditions communes et SOB-66. Valide la procédure au plus bas risque.
2. **Vague 2 : `extension`.** Après son Deep cleanup (SOB-31) et après la
   publication des contrats (§3), sinon le protocole recopié à la main
   dérive entre deux dépôts.
3. **Vague 3 : `data`.** Après le Deep cleanup d'ingest (SOB-21) et le
   renommage des crates (SOB-37), pour naître avec les noms définitifs. C'est
   la plus risquée (DVC, secrets, workflow nocturne). Les vagues 2 et 3 ne
   dépendent pas l'une de l'autre : leur ordre est de prudence.

L'adoption des jetons suit le déplacement de chaque consommateur ; `web/`
attend la fin de son Deep cleanup (SOB-27 à SOB-29). Le travail interne à
`sobr` (app unique ADR-0021, renommage ADR-0020, suppression d'`import` SOB-67)
ne dépend pas des vagues. Chaque vague est un point d'arrêt : à sa fin,
`sobr` compile et sa CI est verte.

**Renommage (ADR-0020).** Les noms de dépôts sont fixés ici ; les noms de
crates, de dossiers et de paquets relèvent de l'ADR-0020 (préfixe `sobr-`
proposé), appliqué dépôt par dépôt (SOB-38 se scinde par dépôt ; SOB-39
devient le renommage du contenu du design system). Zenodo et DOI sont par
dépôt : `sobr` (logiciel) et `data` (jeu de données) ont chacun leurs
métadonnées de citation. Les noms `sobria-*` de cet ADR sont ceux du
2026-10-30.

**App unique (ADR-0021).** Tout front embarqué par l'app de bureau ou par le
serveur reste dans `sobr`, qu'il y ait une ou deux apps. L'ADR-0021 s'appuie
sur cette règle.

### 6. Issues à créer après approbation

Titres en anglais, à créer dans Linear par l'orchestrateur, chacune de 8
points au plus ; les points sont indicatifs, l'estimation officielle se fait
à la création. Total : environ 41 points pour les issues nouvelles, plus
SOB-66, SOB-69 et SOB-70 déjà créées.

| Quand | Issue | Rôle | Points |
|-------|-------|------|--------|
| Avant les vagues | Share the agent working rules across the Sobr-cc repos (après SOB-43, SOB-44) | CTO | 3 |
| Vague 1 | SOB-66 Create the site and .github repos in Sobr-cc (existante, ajouter `design-system`) | DevOps | — |
| Vague 1 | SOB-70 Set up the Sobr-cc/.github repo (existante) | DevOps | — |
| Vague 1 | SOB-69 Move the site to the Sobr-cc/site repo (existante, ajouter le fetch de `docs/`, ré-estimer) | DevOps | — |
| Vague 1 | Move the design system to Sobr-cc/design-system and tag its tokens | DevOps | 3 |
| Vague 1 | Add reusable CI workflows to Sobr-cc/.github (avant la vague 2) | DevOps | 3 |
| Vague 1 | Use the design tokens by tag in the site | Svelte front end | 2 |
| Vague 2 | Create the extension repo in Sobr-cc (human-only) | DevOps | 1 |
| Vague 2 | Publish the bridge JSON Schema and the server OpenAPI with each release | Lead Rust | 5 |
| Vague 2 | Move the extension to Sobr-cc/extension | DevOps | 5 |
| Vague 2 | Add contract tests against the bridge schema and the API in the extension | Dev extensions | 3 |
| Vague 2 | Use the design tokens by tag in the extension | Dev extensions | 2 |
| Vague 3 | Create the data repo in Sobr-cc (human-only) | DevOps | 1 |
| Vague 3 | Move sobria-ingest, the schemas and the DVC pipeline to Sobr-cc/data | DevOps | 8 |
| Vague 3 | Write the ADR for delivering data updates (model catalog, territory files) to the app | CTO | 3 |
| Après le Deep cleanup de `web/` | Use the design tokens by tag in web | Svelte front end | 2 |
| Plus tard | Create the vscode repo, Create the claude-code repo (human-only, à l'ouverture de leur projet) | DevOps | 1 + 1 |

Autres issues existantes à ajuster : SOB-38 et SOB-39 (voir §5), SOB-6 et
SOB-71 (le catalogue arrive dans `data`).

### 7. Texte qui remplace la décision « Dépôts » du document de routes

> **Dépôts** — Organisation GitHub Sobr-cc, un dépôt par unité de livraison :
> `sobr` (moteur, app de bureau, serveur d'organisation, app Svelte, docs et
> ADR), `extension`, `site`, `design-system`, `data`, `.github` et `infra` ;
> `vscode` et `claude-code` à l'ouverture de leurs projets. Voir ADR-0019.

## Conséquences

**Positives** :
- Livraison indépendante : l'extension sort sans reconstruire l'app, le site
  se déploie sans toucher au Rust, le pipeline de données tourne à son
  rythme.
- CI par dépôt, avec des filtres naturels : fin du `ci.yml` unique qui
  reconstruit tout à chaque push (CLAUDE.md §8, frugalité).
- Propriétaire clair par dépôt, par rôle.
- Les contrats (schéma du bridge, OpenAPI, jetons versionnés) remplacent des
  copies manuelles aujourd'hui sans test.
- Réversible par vagues (§5).

**Négatives** :
- Le travail listé au §6, sans fonctionnalité visible.
- Des contrats à maintenir et une matrice de compatibilité à tenir entre
  `sobr`, `extension` et `data`.
- Configuration dupliquée (`deny.toml`, `rustfmt.toml`, lint) : atténuée par
  les workflows réutilisables de `.github`.
- Actions humaines à chaque dépôt (§4).
- `sobr` reste volumineux (point 1).

**Neutres** :
- Registre unique des ADR dans `sobr` ; numérotation continue.
- La dépendance `git` entre dépôts est justifiée par cet ADR (CLAUDE.md §13).

## Liens

- Linear : SOB-65, SOB-13 (renommage), SOB-14 (app unique), SOB-33
  (décision « pas de monolithe »), SOB-66, SOB-69, SOB-70, SOB-73, SOB-74
- ADR-0005, ADR-0006, ADR-0007, ADR-0009, ADR-0013, ADR-0014, ADR-0017,
  ADR-0018, ADR-0020 (renommage), ADR-0021 (app unique)
- CLAUDE.md §3bis, §8, §13
- Document de routes (décision « Dépôts », remplacée par le §7)
- Linear, « How we work »
