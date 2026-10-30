# ADR-0020 — Une seule app Svelte, livrée en bureau et en web

- **Statut** : Proposed (passe à Accepted quand Thibault approuve la PR)
- **Date** : 2026-10-30
- **Décideurs** : Thibault (décision) ; recommandation du CTO
- **Contexte technique** : Foundations, jalon Structure (SOB-14)
- **Amende** : ADR-0017, règle 2 (activation de la démo), à l'acceptation

> **Summary (EN).** `web/` becomes the only interface. One build of it is
> embedded both by the desktop app (Tauri) and by the organization server, in
> place of `web-team/`. At start-up the app detects one of three contexts:
> desktop, web served by the server, or the static demo of ADR-0017. On the
> web, the merge covers the organization space only: sign-in, the member home,
> the Organisation section and alerts. The Understand pages stay on desktop
> until a later decision on how they can run on a server without the prompt
> leaving the machine. Organization data goes through one client, `$lib/org`.
> It fetches the server directly in a browser and goes through a Rust proxy on
> desktop, so tokens never reach the webview. The seven web-team pages land on
> screens M2, Z6, Z7, Z9, B2 and A6. The Organisation section is admin-only
> until roles arrive. Six points for Thibault come first.

## Points à trancher

Six questions pour Thibault ; les recommandations s'appuient sur la section
Décision.

1. **Un seul build de `web/` pour le bureau, le serveur et la démo**, le
   contexte étant détecté à l'exécution. Recommandé : oui. Un seul artefact
   à tester, comme le binaire serveur unique de l'ADR-0014. Alternative :
   un build par contexte, au prix de trois artefacts qui divergent.
2. **Sur le bureau, l'espace organisation passe par le Rust.** Recommandé :
   oui. Une commande Tauri relaie les appels vers le serveur, et les jetons
   restent côté Rust. Le webview ne voit jamais un jeton, et sa CSP n'a pas
   à s'ouvrir à l'URL du serveur. Alternative : le webview appelle le
   serveur directement. Il faudrait alors ouvrir `connect-src` à toute URL
   d'organisation et garder les jetons en JavaScript, comme le fait
   aujourd'hui `team_enroll`, que l'audit SOB-7 signale.
3. **Sur le web, la fusion se limite à l'espace organisation** : connexion,
   Accueil du membre, section Organisation, alertes. Recommandé : oui. La
   section Comprendre ne passe sur le web qu'après une décision à part
   (§6, issue de recherche), pour quatre raisons :
   - le prompt voyage aujourd'hui dans l'URL de `/comparer`, donc il
     arriverait au serveur par l'en-tête `Referer` et les journaux ;
   - les pages Estimer et Comparer écrivent dans le ledger local ;
   - il faudrait une crate de logique partagée ;
   - les fichiers de données du territoire ne sont pas livrés au serveur.

   Alternative : tout porter tout de suite. La fusion attendrait alors ces
   quatre chantiers.
4. **Section Organisation réservée aux admins, d'après le rôle du jeton,
   jusqu'à l'arrivée des rôles.** Recommandé : oui. Le serveur ne connaît
   que `user` et `admin` (`server/auth/jwt.rs`). Le rôle de manager, ce
   qu'il voit et le périmètre de k-anonymat qui s'y applique (ADR-0015,
   ADR-0016) relèvent du projet Organization and identity.
5. **Session web.** Recommandé : garder pour la fusion le schéma actuel de
   web-team (jeton d'accès en mémoire, jeton de rafraîchissement en
   `sessionStorage`), isolé dans `$lib/org`, et y ajouter deux choses :
   - une déconnexion qui révoque le jeton côté serveur, par la seule
     nouvelle route de la fusion, `POST /api/v1/logout` ;
   - un rafraîchissement coordonné entre onglets.

   On passe à un cookie `HttpOnly` avec le SSO. Alternative : le cookie dès la fusion, qui demande
   d'émettre le cookie et de contrôler l'en-tête `Origin` côté serveur.
6. **Textes de la fusion en français seulement.** Recommandé : oui. `web/`
   n'a pas d'i18n (`svelte-i18n` est installé mais n'est pas utilisé).
   SOB-47, SOB-62 et SOB-63 demandent du français et de l'anglais : chacune
   devrait monter l'i18n seule. On retire ce critère de ces trois issues, et
   une issue dédiée installe l'i18n une fois pour toute l'app (§6).
   Alternative : faire de l'i18n un prérequis de la fusion.

## Contexte et énoncé du problème

Le document « Architecture de l'application et des écrans » (30 octobre) fixe
une seule interface pour tous. Elle est livrée en application de bureau
(Tauri) ou en application web servie par le serveur d'organisation, en cloud
comme en self-hosted. L'administration en fait partie : il n'y a plus de
web-team séparé.

Aujourd'hui, le dépôt contient deux apps SvelteKit (`adapter-static`, mode
SPA) :

| | `web/` | `web-team/` |
|--|--|--|
| Taille | ~33 000 lignes, 20 pages | ~2 500 lignes, 7 pages et une redirection |
| Embarquée par | `sobria-app` (Tauri, `frontendDist: ../../web/build`) | `sobria-team-aggregator` (`rust-embed`, dossier `web-team/build`) |
| Accès aux données | `call(cmd, args)` dans `src/lib/api.ts` : `invoke` Tauri (55 commandes), sinon fixtures de la démo (ADR-0017) | `apiGet` et `apiPost` sur `/api/v1/*` : jeton Bearer, rafraîchissement automatique |
| Session | aucune (local) | jeton d'accès en mémoire, jeton de rafraîchissement en `sessionStorage` |

Quatre faits pèsent sur la décision :

- **Le bureau parle déjà au serveur depuis le Rust.**
  `crates/sobria-app/src/team_client.rs` fait le ping, l'enrôlement et
  l'envoi des estimations, mais avec des faiblesses (audit SOB-7) :
  - `team_enroll` renvoie les deux jetons au webview ;
  - une seule paire de jetons est gardée, en clair dans `referentiel.sqlite` ;
  - changer l'URL du serveur ne vide pas la session.
- **Le serveur est un binaire unique** en cloud et en self-hosted
  (ADR-0014). Il sert déjà un tableau de bord (ADR-0013, étage 2) et
  applique les règles de confidentialité côté serveur (ADR-0015, ADR-0016).
  Ses erreurs ont la forme `{error, code}` et ses rôles sont `user` et
  `admin`.
- **L'app et le serveur restent ensemble.** Le bureau, le serveur et
  `web/` restent dans le dépôt `sobr`, et `web-team/` disparaît après la
  fusion (ADR-0019).
- **La démo web reste en place** (ADR-0017) : fixtures du moteur réel,
  bannière permanente, jamais dans l'app de bureau.

## Options envisagées

- **A — Garder deux apps.** Écartée par la décision d'architecture du
  30 octobre ; elle double l'entretien (jetons de couleur, formats et client
  d'API déjà recopiés, audit SOB-7).
- **B — Une base de code, un build par contexte.** Une variable de build
  choisit bureau, serveur ou démo. Le build est simple, mais trois artefacts
  doivent être testés.
- **C — Une base de code, un seul build, trois contextes détectés à
  l'exécution, une couche d'accès aux données par contexte.** Le même
  `web/build` part dans Tauri, dans le serveur et sur l'hébergement statique
  de la démo.

## Décision

**Option C.**

### 1. Un seul build, trois contextes

| Contexte | Détection au démarrage | Commandes de l'app (`$lib/api`) | Données d'organisation (`$lib/org`) | Pages proposées |
|--|--|--|--|--|
| Bureau | `window.__TAURI_INTERNALS__` présent (inchangé) | `invoke` Tauri | commande Tauri `org_request`, relais Rust (§5) | toutes |
| Web | balise `<meta name="sobr-runtime" content="server">` dans le HTML servi par le serveur | aucune : refus avec le code `not_available_here` | `fetch` sur la même origine, `/api/v1/...` | espace organisation (§4) |
| Démo | ni l'un ni l'autre (hébergement statique) | fixtures (ADR-0017) | aucune | celles de l'ADR-0017 |

- **La détection est synchrone** et lue une seule fois au démarrage. Il n'y
  a ni sonde réseau ni course possible.
- **Le serveur met la balise dans toutes les réponses HTML** : `/`,
  `/index.html` et la redirection SPA. Une page servie par le serveur ne
  bascule donc jamais en démo, même si l'API ne répond pas : elle affiche une
  erreur.
- **En développement**, `VITE_SOBR_RUNTIME=server` force le contexte web, et
  le proxy `/api` de Vite pointe vers un serveur local, comme pour web-team
  aujourd'hui. Vite fige cette variable dans le bundle au build, donc l'app
  ne la lit que si `import.meta.env.DEV` est vrai. L'artefact de production
  se teste avec la balise injectée par le serveur.
- **Amendement à l'ADR-0017, règle 2.** La démo s'active quand l'app n'est
  ni dans Tauri ni servie par le serveur ; ce n'est plus seulement « hors de
  Tauri ». Les autres règles de l'ADR-0017 ne changent pas.

### 2. Couche d'accès aux données

- **Les pages n'appellent jamais `invoke` ni `fetch` elles-mêmes.**
  - `$lib/api` couvre les commandes de l'app, comme aujourd'hui.
  - `$lib/org` couvre l'organisation. Il remplace `web-team/src/lib/api.ts` ;
    ses types d'API viennent de web-team.
  - Les DTO restent écrits à la main (ADR-0019). Les générer depuis le Rust
    est une décision à part.
- **Chaque route déclare les contextes où elle apparaît** (bureau, web,
  démo). Le menu et la garde de route suivent cette déclaration.
  - Hors de son contexte, une route est masquée : pas de bouton désactivé,
    conformément à la règle du document d'architecture.
  - La démo garde sa bannière et ses actions désactivées avec explication
    (ADR-0017, règles 3 et 4).
- **`$lib/org`** :
  - traduit les erreurs du serveur (`{error, code}` et le statut HTTP) en
    `SobriaIpcError`, pour que l'UI garde un seul type d'erreur ;
  - ne fait qu'un rafraîchissement de jeton à la fois, dans un onglet et
    entre onglets (Web Locks), et transmet la nouvelle paire aux autres
    onglets (`BroadcastChannel`). Un onglet dupliqué garde sinon une copie
    de l'ancien jeton, déjà révoqué. Cela corrige la déconnexion relevée
    par l'audit dans web-team ; l'extension a son propre client et sa
    propre issue.

### 3. Le serveur sert l'app

- `sobria-team-aggregator` embarque `web/build` à la place de
  `web-team/build`. C'est le même artefact que celui du bureau.
- **Routage**, dans cet ordre :
  1. `/api/*` va à l'API. Un chemin d'API inconnu renvoie une 404 JSON ;
     aujourd'hui il renvoie `index.html` avec un 200 (audit SOB-7).
  2. Un fichier présent dans le bundle est servi. S'il s'agit de HTML, le
     serveur y ajoute la balise `sobr-runtime`.
  3. Un chemin qui ressemble à un fichier absent renvoie une 404.
  4. Sinon, le serveur renvoie `index.html` avec la balise.
- **En-têtes de sécurité sur les réponses HTML** :
  - une CSP :
    - `default-src 'self'` ;
    - `script-src 'self'` plus l'empreinte du script de démarrage en ligne
      de SvelteKit. Le serveur la calcule depuis l'`index.html` embarqué
      (`build.rs` ou démarrage), jamais à la main ;
    - `style-src 'self' 'unsafe-inline'` et `font-src 'self' data:`, comme
      la CSP du bureau (`web/` utilise des styles en ligne) ;
    - `connect-src 'self'`, `img-src 'self' data:` ;
    - `frame-ancestors 'none'` ;
  - `Referrer-Policy: no-referrer` et `X-Content-Type-Options: nosniff` ;
  - `Strict-Transport-Security` quand le serveur sert en TLS.

  Le contexte web n'affiche aucune carte, donc aucune tuile tierce n'est
  chargée. Les tests Playwright en contexte web échouent à toute violation
  de la CSP.
- **Ordre de build.** La CI construit `web/` avant de construire et de
  tester le serveur, sinon le bundle est vide et le test de l'interface
  embarquée échoue (audit SOB-7).
- **Une seule nouvelle route d'API pour la fusion : `POST /api/v1/logout`**,
  qui révoque le jeton de rafraîchissement reçu. Pour le reste, les pages web
  utilisent l'API `/api/v1` existante (`/login`, `/enroll`, `/refresh`,
  `/me/*`, `/admin/*`). Le CORS réservé à l'extension reste tel quel ; l'app web est
  sur la même origine que l'API et n'en a pas besoin.

### 4. Destination de chaque page de web-team

| Page web-team | Écran cible | Route proposée dans l'app | Visible pour | Issue |
|--|--|--|--|--|
| `/login`, onglet admin | M2 Connexion (avant le SSO) | `/connexion` | personne non connectée (web) | SOB-47 |
| `/login`, onglet employé | M2 Connexion (avant le SSO) | `/connexion` | personne non connectée (web) | SOB-47 |
| `/login`, enrôlement par code à 12 chiffres | M1 puis M2 (le code tient lieu d'invitation tant que Z5 et Z6 n'envoient pas de lien) | `/connexion#code=…` : le fragment n'est jamais envoyé au serveur, et l'app l'efface de l'URL après lecture | personne non connectée (web) | SOB-47 |
| `/admin/dashboard` | Z7 Dashboard de suivi | `/organisation` | admin | SOB-62 |
| `/admin/codes` | Z6 Invitations | `/organisation/invitations` | admin | SOB-62 |
| `/admin/users` | Z9 Gestion des utilisateurs | `/organisation/utilisateurs` | admin | SOB-62 |
| `/admin/users/[id]` | Z9 Gestion des utilisateurs (détail) | `/organisation/utilisateurs/[id]` | admin | SOB-62 |
| `/admin/alerts` | B2 Budgets et alertes, portée organisation | `/organisation/alertes` jusqu'au design V7, puis dans B2 avec les budgets | admin | SOB-62 |
| `/user/dashboard` (usage et partage) | A6 Accueil, espace organisation | `/suivi` (l'Accueil), alimenté par `/api/v1/me/usage` et `/api/v1/me/sharing` | membre (rôle `user`) | SOB-63 |
| `/` (redirection) | aucun | dans le contexte web, `/` mène au premier écran du rôle : admin → `/organisation`, membre → `/suivi`, sinon `/connexion`. Sur le bureau, `/` reste la page Estimer. | tous | SOB-47 |

Règles :

- **La section Organisation n'apparaît que si le jeton porte le rôle
  `admin`.** Elle n'est jamais affichée désactivée. Un membre qui ouvre une
  route `/organisation/*` revient sur `/suivi`.
- **Un admin n'a pas d'usage propre sur le serveur** (table à part,
  `/me/usage` lui renvoie une 403). Son premier écran est donc Z7.
- **Sur le bureau, l'Accueil d'un membre enrôlé** montre son résumé local,
  plus un bloc organisation lu par `org_request`. Cela relève de l'issue
  d'accès organisation au bureau (§6).
- **Les routes sont des propositions pour la fusion** ; le design V7 peut
  les renommer sans rouvrir cet ADR.

### 5. Connexion selon le contexte

| Contexte | Espace perso | Espace organisation |
|--|--|--|
| Bureau | aucune connexion : invité local, comme aujourd'hui (ADR-0013, ADR-0014). Le compte perso relève de SOB-35 | enrôlement (code à 12 chiffres) ou connexion faits par le Rust ; les pages passent par `org_request` (règles ci-dessous) |
| Web | pas d'espace perso sur un serveur d'organisation (il viendra avec le compte perso, SOB-35) | page `/connexion` (M2) : admin (identifiant et mot de passe), employé (identifiant et mot de passe), enrôlement (code, mot de passe, nom affiché), avec les routes actuelles `/api/v1/login`, `/enroll` et `/refresh` ; déconnexion par `POST /api/v1/logout` (point 5) |
| Démo | aucune connexion | n'existe pas |

**Règles du relais Rust sur le bureau :**

- `org_request` n'est jamais un proxy ouvert :
  - il n'accepte qu'un chemin relatif ;
  - il le joint à l'URL configurée, qui peut contenir un sous-chemin, en
    segments relatifs ;
  - il vérifie sur le chemin obtenu, une fois normalisé, que l'hôte n'a
    pas changé et que le chemin reste sous `<base>/api/v1/me/` ou
    `<base>/api/v1/admin/`. Ainsi `me/../admin/x` est jugé comme
    `admin/x`.
- Chaque préfixe a son jeton : `/me/*` prend celui du membre, `/admin/*`
  celui de la session d'admin. Sans le bon jeton, l'appel est refusé sans
  partir.
- Changer l'URL du serveur ou le réglage des certificats efface la session.
- Une session d'admin ouverte sur le bureau reste en mémoire et ne remplace
  pas la paire de jetons du membre, qui sert à l'envoi des estimations.
- Un seul rafraîchissement de jeton à la fois.
- Aucune commande ne renvoie de jeton au webview, `team_enroll` compris.
- Les jetons du membre restent stockés en clair dans `referentiel.sqlite`
  pour la fusion, contre CLAUDE.md §7. Le constat est posté sur SOB-26
  (audit SOB-7) et son traitement suit SOB-83 : un trousseau de l'OS
  demande une dépendance à justifier.

**Limites connues jusqu'au SSO :**

- L'identifiant d'un employé est l'empreinte de son navigateur, que l'audit
  SOB-7 classe High (collisions entre postes identiques). On le garde pour
  la fusion. Le corriger avant le SSO relève de SOB-83.
- Un membre enrôlé depuis le bureau a un identifiant de la forme
  `app-desktop-<uuid>`. Pour se connecter sur le web, il lit cet
  identifiant dans les Paramètres de l'app.

Le SSO et le SCIM (écran M2 définitif, projet Organization and identity)
remplacent la connexion par mot de passe et ces deux limites.

### 6. Ordre et issues

1. **SOB-42 (ajusté)**, servir l'app depuis le serveur :
   - embarquer `web/build`, avec la balise `sobr-runtime` et les en-têtes
     de sécurité ;
   - détection des trois contextes, client `$lib/org` et contextes par
     route ;
   - 404 JSON sous `/api`, ordre de build de la CI.

   Les pages Comprendre ne sont plus « aussi accessibles dans un
   navigateur ». Le critère « données réelles du serveur » passe à SOB-47
   et SOB-63. À la place : l'app démarre en contexte web, la balise et les
   en-têtes sont présents, et `/` mène à `/connexion`.
2. **SOB-47 (ajusté)**, `/connexion`, `POST /api/v1/logout` côté serveur,
   session web partagée entre onglets.
3. **SOB-62 (ajusté)**, section Organisation réservée aux admins (Z7, Z6,
   Z9, alertes).
4. **SOB-63 (ajusté)**, Accueil web du membre.
5. **SOB-64**, supprimer `web-team/`, inchangée.
6. **Nouvelle issue : accès à l'espace organisation depuis le bureau** :
   `org_request` et ses règles (§5), connexion par le Rust, Accueil avec un
   bloc organisation. Bloquée par SOB-42 et SOB-47.
7. **Nouvelle issue : installer le français et l'anglais dans l'app**
   (i18n). Elle porte le critère retiré de SOB-47, SOB-62 et SOB-63
   (point 6).
8. **Nouvelle issue de recherche : la section Comprendre sur le web sans
   envoyer le prompt.** Elle couvre :
   - pas de prompt dans les URL ;
   - une route serveur qui ne stocke ni ne journalise rien, avec un calcul
     borné ;
   - une crate de logique partagée ;
   - la livraison des fichiers de territoire (ADR-0019, vague 3) ;
   - le statut d'API interne ou publique (ADR-0019 §3).

## Conséquences

**Positives**

- **Un seul artefact pour trois contextes.** L'administration est
  utilisable au bureau et sur le web, comme le demande le document
  d'architecture, avec le même binaire serveur en cloud et en self-hosted
  (ADR-0014).
- **Les jetons d'organisation sortent du webview** sur le bureau. Le relais
  Rust ferme trois failles relevées par l'audit : URL changée, paire
  unique, rafraîchissements concurrents. Le stockage en clair des jetons
  reste, lui, à traiter (§5).
- **La fusion n'ajoute qu'une route d'API (la déconnexion)** et ne fait
  sortir aucune donnée de plus vers le serveur.
- **Les pages servies sur le web ont des en-têtes de sécurité**, ce qui
  n'est pas le cas aujourd'hui.

**Négatives**

- **Sur le web, l'app ne propose d'abord que l'espace organisation.** La
  section Comprendre attend sa propre décision.
- **Trois issues de plus** à écrire (accès organisation au bureau, i18n,
  recherche sur Comprendre).
- **Le serveur réécrit le HTML** pour la balise de contexte, et la CSP doit
  suivre l'empreinte du script de démarrage à chaque build.
- La connexion par empreinte et le jeton de rafraîchissement en
  `sessionStorage` restent jusqu'au SSO.

**Neutres**

- La démo garde ses règles, sauf la règle 2 de l'ADR-0017, précisée.
- Les routes de l'app restent en français, comme aujourd'hui.
- Les DTO restent écrits à la main.

## Liens

- [Architecture de l'application et des écrans](https://claude.ai/artifact/5UaiKHXrKPVyRPEd6gfeyr) (arborescence, accès par rôle, quatre règles)
- [Route utilisateur, écran par écran](https://claude.ai/artifact/Cua7SK84VzMCwYFZRQfUHD) (M1, M2, Z6, Z7, Z9, B2, A6)
- ADR-0013 (appairage et mode Équipe), ADR-0014 (binaire unique), ADR-0015
  et ADR-0016 (confidentialité), ADR-0017 (démo web, règle 2 amendée),
  ADR-0019 (découpage en dépôts, PR #2)
- [Codebase audit (Nov 2026)](https://linear.app/sobrcc/document/codebase-audit-nov-2026-6f4f34265a5e)
- Issues : SOB-14 (cet ADR), SOB-42, SOB-47, SOB-62, SOB-63, SOB-64, SOB-35, SOB-83
