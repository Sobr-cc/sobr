# ADR-0020 — Renommer Sobr.ia en Sobr.cc

- **Statut** : Proposed (passe à Accepted quand Thibault approuve la PR)
- **Date** : 2026-10-30
- **Décideurs** : Thibault (décision) ; recommandation du CTO
- **Contexte technique** : Foundations, jalon Structure (SOB-13)

> **Summary (EN).** The product is renamed Sobr.cc everywhere (decision of
> 2026-10-30). Nothing was installed under the old name by outside users:
> release v0.8.0 holds only team-server binaries, with 0 downloads, and no
> DOI exists. Public identifiers are therefore cheapest to change now, before
> the first signed release (V1, 2027-03-31). Every name is sorted by nature
> (rules R1 to R4): public names are renamed now, names set by operators are
> renamed with a read-both period until V1, persisted or signed names are
> kept. New crate prefix: `sobr-`. Three questions for Thibault come first,
> then five facts only he can confirm.

## Points à trancher

1. **Préfixe des crates : `sobr-`** (`sobr-core`, lib `sobr_core`). Recommandé :
   oui. Les alternatives sont dans la section Options.
2. **On ne réécrit jamais une donnée persistée ou signée** (règles R1 à R4).
   Recommandé : oui. Concrètement : l'identifiant de méthode `afnor_sobria`
   et le dossier de données `sobria/` sont conservés. Alternative : renommer
   le dossier en `sobr/` avec une lecture de l'ancien dossier à vie, soit un
   chemin de plus pour changer un nom que seul un utilisateur technique voit.
3. **Identifiants publics changés maintenant**, avant la première release
   signée : bundle id, hôte natif du bridge et ID Firefox (lignes « Tauri » et
   « Extension »). Recommandé : oui, sous réserve des faits 1 et 2 ci-dessous.
   Plus tard, chacun coûterait une réinstallation ou une perte de données aux
   utilisateurs.

## À confirmer par Thibault

Cinq faits que le dépôt ne contient pas ; l'ADR dit ce qui change selon la
réponse.

1. **`sobr.cc` est-il enregistré et sous votre contrôle ?** Le dépôt n'en
   contient aucune trace, et le chantier C33
   (`docs/archive/chantiers/C33-site-internet.md`) note `sobr.ia` « non
   disponible à l'enregistrement ». Sans `sobr.cc`, le bundle id, l'ID Firefox
   et les IRI d'export ne peuvent pas être basculés.
2. **Une fiche existe-t-elle sur le Chrome Web Store ou sur AMO ?** Si oui,
   l'ID Firefox `sobria@sobr.ia` est conservé (R3).
3. **Le dossier de candidature data.gouv.fr a-t-il été déposé ?** Si oui, il
   n'est pas modifié (ligne « DOI, Zenodo, data.gouv.fr »).
4. **Zenodo est-il déjà activé sur un dépôt ?** Aucun DOI n'existe, mais
   l'intégration a pu être activée.
5. **Des opérateurs extérieurs font-ils déjà tourner le serveur d'équipe**
   (variables `SOBRIA_TEAM_*`) ? Sinon R2 devient « renommer sans
   compatibilité ».

## Contexte et énoncé du problème

Le 30 octobre 2026, Thibault a décidé que le produit s'appelle Sobr.cc
partout, et demandé un ADR avant tout renommage : le changement touche
beaucoup d'endroits à la fois et certains cassent des choses pour les
utilisateurs existants.

Sur `main`, « Sobr.ia » apparaît sur 1206 lignes de 302 fichiers, et `sobr.cc`
nulle part. Mais presque rien n'est installé chez des tiers sous l'ancien nom,
au 2026-10-30 :

- la seule release publique, v0.8.0 (2026-05-17), contient des fichiers du
  serveur d'équipe (binaire Windows, sommes SHA-256), avec 0 téléchargement ;
  aucun installateur desktop ni extension n'a été publié ;
- aucun DOI n'est minté (le README porte `PENDING-FIRST-RELEASE`) ;
- l'application n'a pas de mise à jour automatique ;
- les crates ne sont pas publiées (`publish = false`).

Le coût d'un identifiant public est donc au plus bas aujourd'hui et monte avec
la V1 du 31 mars 2027 : installateurs signés, fiches de magasins, DOI.

## Critère : quatre règles par nature de nom

- **R1 — Visible ou public** (noms affichés, binaires, bundle id, hôte natif,
  ID d'add-on, liens, domaines) : renommé maintenant. Un ancien nom public est
  redirigé, jamais réattribué.
- **R2 — Réglé par un utilisateur ou un opérateur** (variables
  d'environnement) : nouveau nom lu en premier, ancien nom accepté avec un
  avertissement jusqu'à la V1, puis retiré. Une variable que le code livré ne
  lit pas est supprimée, non migrée.
- **R3 — Persisté, haché dans une donnée signée, ou déjà déposé chez un
  tiers** (identifiant de méthode, dossier de données, noms de fichiers
  SQLite, clés de stockage, `$id` des schémas v1, magasin, data.gouv.fr,
  Zenodo) : conservé, jamais réécrit. Seul l'affichage change. L'historique
  (`docs/archive/`, dossier déposé) reste tel quel. Cet invariant est déjà
  celui de l'ADR-0012 : la chaîne SHA-256 du ledger reste valide parce que les
  JSON signés ne sont pas modifiés.
- **R4 — Interne et non persisté** (variables de CI, de test, de build et de
  déploiement, attributs DOM, préfixe de console, filtre de log) : renommé
  sans compatibilité.

## Options envisagées

Stratégie :

| Option | Principe | Risque | Réversibilité |
|--------|----------|--------|---------------|
| A | Tout renommer, y compris les identifiants persistés, avec migration des données | élevé : réécrire des valeurs signées invalide la chaîne du ledger | difficile |
| B | Ne renommer que ce que voit l'utilisateur, garder tous les identifiants `sobria` | faible, mais contraire à « partout » : fige `sobria` dans binaires, logs, bundle id, hôte natif, dont le coût de changement monte après la V1 | bonne |
| C | Règles R1 à R4 selon la nature de chaque nom | faible : un chemin de compatibilité temporaire, un permanent, aucune migration | bonne |

Préfixe des crates :

| Préfixe | Pour | Contre |
|---------|------|--------|
| `sobr-` | aligné sur le dépôt `Sobr-cc/sobr`, court, mécanique | — |
| `sobrcc-` | reflète le nom du produit | se lit mal (`sobrcc_core`) |
| `sobria-` (inchangé) | aucun coût | l'ancien nom reste dans binaires, logs et user-agents |

## Décision

**Option C est proposée**, avec le préfixe de crates `sobr-`.

### Table de renommage

| Élément | Ancien → nouveau | Ce qui casse pour les utilisateurs existants | Traitement |
|---------|------------------|----------------------------------------------|------------|
| Crates Rust | `sobria-*` (paquets, libs `sobria_*`, binaires) → `sobr-*`, `sobr_*` | Rien à l'exécution. Un `RUST_LOG=sobria…` personnalisé ne filtre plus ; les user-agents `sobria-app/…` et `sobria-ingest/…` changent ; le certificat TLS du serveur d'équipe garde l'organisation « Sobr.ia Team Aggregator » jusqu'à sa régénération | R1. Renommage mécanique ; le filtre de log par défaut change dans le même commit ; le CHANGELOG signale `RUST_LOG`. Le user-agent d'ingest porte l'URL du dépôt GitHub, pas un domaine |
| Variables d'environnement | `SOBRIA_*` → `SOBR_*` | Celles qu'un utilisateur ou un opérateur règle (`SOBRIA_SEED`, `SOBRIA_DATA_ROOT`, `SOBRIA_REFERENTIEL_PATH`, `SOBRIA_TEAM_*`) seraient ignorées en silence : le `.env` d'un opérateur perdrait mot de passe admin et port | R2 pour celles-ci (sans compatibilité si le fait 5 est négatif) ; R4 pour les variables de CI, de test, de build et de déploiement. `SOBRIA_SEED` n'est lue que par le CLI d'ingest, alors que CLAUDE.md §6 la dit lue par le moteur : à corriger avec SOB-37 |
| Tauri | identifiant de bundle `ia.sobr.app` → `cc.sobr.app` ; productName `Sobr.ia` → `Sobr.cc` | Sur macOS et Windows, c'est une autre application : installation côte à côte, stockage du webview remis à zéro (d'après le fonctionnement de Tauri). Les données n'en dépendent pas (ligne suivante). Personne n'a l'app installée et il n'y a pas de mise à jour automatique | R1, avant la première release signée. Dépend du fait 1 (identifiant dérivé du domaine) |
| Dossier de données de l'app | `<data_dir>/sobria` (`audit.sqlite`, `referentiel.sqlite`) et `~/.sobria/spool` : inchangés | Renommer sans repli donnerait une app vide : ledger, préférences et appairages perdus | R3. Aucun code de repli. L'ancien nom reste dans un chemin que seul un utilisateur technique voit (alternative : point 2) |
| Domaine `sobr.ia` | IRI d'export `https://sobr.ia/vocab#` et `…/vocab/datasheet#`, `$id` des schémas JSON, homepage Cargo et extension, contact sécurité → `sobr.cc` | Les exports déjà produits gardent l'ancienne IRI : lus avec un seul préfixe, anciens et nouveaux exports ne sont plus comparables. `sobr.ia` n'est pas enregistré (C33) | R1 pour les nouveaux fichiers (`https://sobr.cc/vocab#`) ; lecture des deux préfixes, en permanence. Schémas v1 et leur `$id` gelés (R3 : snapshots, lignage, ADR-0009) ; la base d'`$id` change à partir de la version suivante. Le contact `security@sobr.ia` demande une vraie boîte (humain). Dépend du fait 1 |
| Site | hôte `sobria.brilliantstudio.co` → `sobr.cc` | Liens déjà publiés vers l'ancien hôte, dont `contact@sobria.brilliantstudio.co` | R1 : redirection 301 de l'ancien hôte tant qu'il existe (humain ; déploiement : `docs/operations/site-deploy.md`). SOB-69 choisit le dépôt du site, cet ADR fixe l'hôte |
| Dossier du design system | `sobr-ia-design-system/` → `design-system` si l'ADR-0019 est accepté, sinon `sobr-design-system` | Références cassées : script de logos de l'extension, commentaire de `web/src/app.css`, README | R1. `name: sobria-design` → `sobr-design`. Le logo dessine le mot « Sobr.ia » : le redessiner est un travail de design (Design V7), pas un renommage |
| Extension navigateur | nom « Sobr.ia » → « Sobr.cc » ; ID Firefox `sobria@sobr.ia` → `sobr@sobr.cc` ; hôte natif `com.sobria.bridge` → `cc.sobr.bridge` | ID Firefox changé : c'est un autre add-on, stockage et appairage perdus. Hôte natif changé : manifestes et clés de registre à réinstaller dans chaque navigateur (ADR-0013) ; un binaire renommé laisse d'anciens manifestes pointer dans le vide | R1, maintenant (faits 1 et 2) ; l'installateur supprime les anciens manifestes. Clés `chrome.storage` : R3 |
| DOI, Zenodo, data.gouv.fr | `.zenodo.json` (titre, affiliation et créateurs « Sobr.ia », lien vers `BkOff-fr`) → « Sobr.cc » et `Sobr-cc/sobr` ; dossier de candidature « Sobr.ia » | Aucun DOI à casser. Un DOI créé sous l'ancien nom garderait à vie titre et créateurs de la première version | `.zenodo.json` mis à jour avant le premier dépôt ; Zenodo activé sur chaque dépôt qui mintera un DOI (humain). Dossier déjà déposé (fait 3) : R3, non modifié, avec une mention « devenu Sobr.cc » dans le dépôt (`docs/CANDIDATURE-DATA-GOUV.md`) |
| Liens vers l'ancien dépôt | `BkOff-fr/defis-lia-generatif` et `BkOff-fr/sobria` → `Sobr-cc/sobr` | Rien tant que ces noms redirigent vers `Sobr-cc/sobr` (vérifié par l'API GitHub) ; la redirection tomberait si un dépôt était recréé sous l'un d'eux.<br>Déjà morts (404) : `anthropics/defis-lia-generatif`, `sobria/sobria`, et des liens `<TBD>` | R1 : tous les liens sont corrigés (SOB-40) |
| Identifiant de méthode | `afnor_sobria` : inchangé | Le réécrire invalide les signatures du ledger, les exports et les événements de l'extension : la chaîne SHA-256 couvre le JSON qui contient l'identifiant (ADR-0012) | R3. Le libellé affiché (`methodology.afnor-sobria`, i18n) devient « Sobr.cc » |
| Déploiement du serveur d'équipe | projet compose `sobria-team`, volume `sobria-team_data`, image, assets de release `sobria-team-aggregator-*` | Renommer le projet compose ou le volume orpheline la base `team.sqlite` (`docs/operations/deploiement-equipe.md`) | R3 pour le projet compose et le volume : nom conservé dans l'exemple fourni. Image et assets suivent R1, nommés par le projet « Cloud and self-hosting » |
| Clés de stockage et attributs internes | `localStorage` `sobria_*`, `sessionStorage`, `data-sobria-*`, préfixe de console `[sobria]` | Renommer les clés imposerait du code de migration pour aucun gain visible | R3 pour les clés de stockage ; R4 pour les attributs DOM et le préfixe de console |
| Textes affichés | « Sobr.ia » dans l'app, l'extension, le site, la documentation vivante → « Sobr.cc » | Rien de persisté | R1 (SOB-38 pour l'app et l'extension). `docs/archive/` n'est pas réécrite (R3) |

### Ordre

- Le renommage des crates et des variables (SOB-37) suit le Deep cleanup de
  chaque crate (règle Foundations) et précède tout déplacement de crate vers
  un autre dépôt (ADR-0019).
- Nom du binaire du bridge, hôte natif, ID Firefox et bundle id changent dans
  une seule livraison, avec l'appairage automatique (SOB-34), pour que les
  manifestes natifs ne soient réécrits qu'une fois.
- SOB-38 (app et extension), SOB-39 (design system) et SOB-40 (liens)
  couvrent les lignes du même nom. SOB-39 est absorbée par le déplacement si
  l'ADR-0019 est accepté ; SOB-38 se scinde par dépôt si l'extension a déjà
  déménagé.
- Les étapes humaines (enregistrement de `sobr.cc`, redirection de l'ancien
  hôte, activation de Zenodo, boîte de sécurité) ne dépendent pas du code :
  elles démarrent dès les réponses « À confirmer ».

## Conséquences

**Positives** :
- Un seul nom avant la V1, et les identifiants publics changés quand personne
  n'en dépend.
- Aucune donnée signée n'est réécrite : le ledger reste vérifiable.
- Chaque nom a une règle, donc chaque issue de renommage sait quoi faire.

**Négatives** :
- Deux noms coexistent durablement dans les données : dossier `sobria/`,
  `afnor_sobria`, clés de stockage, volume compose. À documenter comme
  identifiants hérités.
- Deux chemins de compatibilité : un temporaire (variables `SOBRIA_*`, retiré
  à la V1), un permanent (lecture des deux IRI).
- Dépend de l'enregistrement de `sobr.cc`, une action humaine.

**Neutres** :
- CLAUDE.md et la documentation sont mis à jour avec SOB-37 et SOB-38.
- `docs/archive/` garde l'ancien nom.

## Liens

- Linear : SOB-13, SOB-34 (appairage automatique), SOB-37, SOB-38, SOB-39,
  SOB-40, SOB-69
- ADR-0009 (schémas versionnés), ADR-0012 (ledger signé), ADR-0013 (bridge),
  ADR-0019 (découpage en dépôts, Proposed, PR #2)
- CLAUDE.md §6
- `docs/operations/deploiement-equipe.md`, `docs/operations/site-deploy.md`,
  `docs/CANDIDATURE-DATA-GOUV.md`, `docs/archive/chantiers/C33-site-internet.md`
- Document de routes (décision « Nom ») ; Linear, « How we work »
