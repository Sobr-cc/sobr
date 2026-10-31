# Catalogue des sources de données — Sobr.ia

> **Version** : 2.1 — model catalog sources section added (2026-10-31, SOB-6) ; 2.0 — pivot ComparIA (12 mai 2026).
> Chaque source traverse le pipeline médaillon (ADR-0009).
> Toute modification = mise à jour du schéma Silver correspondant + bump de version.

---

## Stratégie de priorisation

| Tier | Critère | Sources |
|------|---------|---------|
| **Tier 1** | Datasets officiels référencés par le défi data.gouv.fr | ComparIA, RTE-NaTran-Teréga IRIS |
| **Tier 2** | Sources publiques, gratuites, sans authentification | ADEME, EcoLogits, ML.Energy, CodeCarbon, HF AI Energy Score, Papers |
| **Tier 3** | Source nationale stratégique avec compte gratuit | RTE eco2mix (mix élec FR live) |
| **Supprimées** | Sources nécessitant compte payant ou clé restrictive | Electricity Maps, MaxMind GeoLite2 |

**Engagement v1.0** : zéro source bloquante derrière un paywall. Toutes les sources Tier 1 et Tier 2 sont téléchargeables anonymement.

---

# 🥇 Tier 1 — Datasets officiels du défi

## S01 — ComparIA (Ministère de la Culture / Beta.gouv)

| Champ | Valeur |
|-------|--------|
| **URL data.gouv.fr** | https://www.data.gouv.fr/datasets/compar-ia |
| **Plateforme** | https://comparia.beta.gouv.fr/ |
| **Code source** | https://github.com/betagouv/ComparIA |
| **Producteur** | Ministère de la Culture (DINUM / Beta.gouv) |
| **Licence** | **Licence Ouverte / Open Licence v2.0 (Etalab 2.0)** |
| **Compatibilité Sobr.ia** | ✅ Parfaite (Etalab ↔ MIT/CC-BY) |
| **Fréquence MAJ** | Trimestrielle (snapshot dernière MAJ : 5 mars 2026) |
| **Politique fetch** | Hebdomadaire (poll metadata) |
| **Authentification** | ❌ Aucune |
| **Status défi** | 🎯 Dataset central |

### Fichiers à ingérer

| Fichier | Format | Taille | URL téléchargement |
|---------|--------|--------|---------------------|
| Conversations | Parquet | 682 MB | https://www.data.gouv.fr/api/1/datasets/r/7651fd0b-f222-43b3-8db8-ed6ae660d313 |
| Votes | Parquet | 733 MB | https://www.data.gouv.fr/api/1/datasets/r/4ffc86e1-84a4-4fdc-9726-66408e596fef |
| Réactions | Parquet | 3,4 GB | https://www.data.gouv.fr/api/1/datasets/r/9dd3d51f-4299-4193-ab46-81ae039fe1be |
| Documentation | CSV | 8,9 KB | https://www.data.gouv.fr/api/1/datasets/r/278fb12a-f75a-4891-add8-fff1d82f1622 |

**Volume total : ~5 GB** — gros, mais Parquet colonnaire permet une ingestion fluide en DuckDB / polars.

### Entités Silver produites

- `comparia_conversations.parquet` — questions + réponses + métadonnées modèle + impact env. par message
- `comparia_votes.parquet` — préférences utilisateur par paire de modèles
- `comparia_reactions.parquet` — réactions message par message (granularité fine)

### Champs critiques pour Sobr.ia

- Modèle utilisé (provider + nom + version)
- Tokens entrée / sortie réels (mesurés, pas estimés)
- Impact environnemental calculé par EcoLogits (CO₂eq, énergie, eau si présent)
- Localisation serveur (datacenter)
- Type de question (catégorie thématique)
- Préférences utilisateurs (signal qualité)

### Pourquoi c'est crucial

- **Données réelles d'usage** en français (~250 000 questions, ~150 000 votes).
- **Méthodologie environnementale officielle** (EcoLogits + ISO 14044) déjà appliquée.
- **Croisable** avec ComparIA est mention culturelle française → cohérence territoriale.
- **Licence Etalab 2.0** → republication possible.

### Risques

- Volume important → DVC obligatoire, pas de check-in Git.
- Données utilisateur potentiellement sensibles (questions formulées en clair) → pseudonymisation à vérifier dans la doc ComparIA.
- Couverture modèles fermés/ouverts inégale.

### Tests / validation

- Cohérence : somme(impact par message) = impact par conversation.
- Présence des champs obligatoires sur ≥ 99 % des lignes.
- Distribution des modèles attendue (≥ 30 modèles couverts).

---

## S02 — Consommation annuelle IRIS sites industriels (RTE / NaTran / Teréga)

| Champ | Valeur |
|-------|--------|
| **URL data.gouv.fr** | https://www.data.gouv.fr/datasets/consommation-annuelle-definitive-delectricite-et-de-gaz-par-iris-des-sites-industriels-raccordes-aux-reseaux-de-transport |
| **Source originale** | https://odre.opendatasoft.com/explore/dataset/consommation-annuelle-par-iris/ |
| **Producteurs** | NaTran, RTE, Teréga (gestionnaires des réseaux de transport élec + gaz) |
| **Diffuseur** | Open Data Réseaux Énergies (ODRÉ) |
| **Licence** | **Licence Ouverte / Open Licence v2.0 (Etalab 2.0)** |
| **Compatibilité Sobr.ia** | ✅ Parfaite |
| **Fréquence MAJ** | Annuelle (dernière : 6 décembre 2024, référentiel IRIS 2023) |
| **Politique fetch** | Annuelle (déclenchée manuellement à chaque MAJ ODRÉ) |
| **Authentification** | ❌ Aucune |
| **Status défi** | 🎯 Dataset officiel — angle territorial |

### Fichiers à ingérer

| Fichier | Format | Taille | URL téléchargement |
|---------|--------|--------|---------------------|
| Données tabulaires | CSV | 90,2 MB | https://www.data.gouv.fr/api/1/datasets/r/631d6ec4-74c5-4f0f-9187-442cf9d1f0bc |
| Données + coordonnées (JSON) | JSON | 97,8 MB | https://www.data.gouv.fr/api/1/datasets/r/67a173e0-30c6-461c-aed5-3f6a8ab15a09 |
| GeoJSON | JSON | 91,0 MB | https://www.data.gouv.fr/api/1/datasets/r/2b584d52-f4e0-4232-87df-c456e715f334 |
| Shapefile (ZIP) | ZIP | 38,9 MB | https://www.data.gouv.fr/api/1/datasets/r/95fb8941-47a9-4d15-8c7a-3320fa0e0319 |

### Entités Silver produites

- `iris_consommation_annuelle.parquet` — consommation MWh élec + gaz par IRIS + nombre de sites
- `iris_geometries.parquet` (depuis le GeoJSON / shapefile) — polygones IRIS pour rendu carto

### Champs critiques

- Code IRIS (référentiel INSEE 2023)
- Consommation électrique annuelle (MWh)
- Consommation gaz annuelle (MWh)
- Nombre de sites industriels raccordés transport
- Année de référence
- Géométrie du polygone IRIS

### Usage dans Sobr.ia

- **Module M12 — Territoire français** : visualisation choroplèthe des concentrations industrielles.
- **Heuristique datacenter** : un IRIS avec > N MWh élec + faible gaz est candidat à héberger un datacenter (à valider).
- **Scénarios régionaux** : projeter l'impact d'un déploiement LLM par région française.
- **Storytelling** : "voici les 10 IRIS qui concentrent X % de la consommation industrielle élec — combien d'usage IA correspond à un IRIS moyen ?"

### Risques

- Données annuelles → granularité temporelle grossière (compensée par eco2mix pour live).
- Anonymisation par seuils : certains IRIS peuvent être en "secret statistique" → traitement spécial.
- Évolution annuelle des polygones IRIS (INSEE) → versionner le référentiel.

### Tests / validation

- Somme des consommations par région cohérente avec les bilans RTE/GRDF nationaux (±5 %).
- Géométries valides (GeoJSON RFC 7946).
- Nombre d'IRIS couverts (~50 000 IRIS en France) → check ratio de couverture.

---

# 🥈 Tier 2 — Sources complémentaires (sans authentification)

## S03 — ADEME Base Empreinte

| Champ | Valeur |
|-------|--------|
| **URL** | https://base-empreinte.ademe.fr/ |
| **API** | https://data.ademe.fr/datasets/base-empreinte |
| **Format Copper** | CSV + JSON |
| **Licence** | Etalab 2.0 |
| **Fréquence MAJ** | Trimestrielle |
| **Politique fetch** | Hebdomadaire |
| **Authentification** | ❌ Aucune |
| **Entités Silver** | `electricity_factors`, `hardware_factors` |

**Usage** : facteurs d'émission de référence (gCO₂eq/kWh par pays, par source, par année). Indispensable pour normaliser ComparIA → CO₂eq quand le champ n'est pas pré-rempli.

---

## S04 — GenAI Impact / EcoLogits

| Champ | Valeur |
|-------|--------|
| **URL** | https://github.com/genai-impact/ecologits |
| **Format Copper** | JSON + Python (catalogue modèles) |
| **Licence** | MIT |
| **Fréquence MAJ** | Continue (releases GitHub) |
| **Politique fetch** | Hebdomadaire (poll releases) |
| **Authentification** | ❌ Aucune |
| **Entités Silver** | `ecologits_models`, `ecologits_assumptions` |

**Usage** : **la méthodologie officielle** déjà adoptée par ComparIA. Sobr.ia doit l'appliquer fidèlement (et la valider sur les 3 études de référence). Importer le catalogue de modèles + paramètres d'incertitude permet d'estimer aussi des modèles non couverts par ComparIA.

---

## S05 — Hugging Face AI Energy Score

| Champ | Valeur |
|-------|--------|
| **URL** | https://huggingface.co/AIEnergyScore |
| **Format Copper** | JSON / Parquet via HF Hub |
| **Licence** | Apache 2.0 (modèles) / CC-BY (scores) |
| **Fréquence MAJ** | Ad hoc (nouveaux modèles) |
| **Politique fetch** | Hebdomadaire |
| **Authentification** | ❌ Aucune (lecture publique HF Hub) |
| **Entités Silver** | `model_energy_score` |

**Usage** : complète ComparIA pour les modèles non couverts. Score standardisé permettant des comparaisons.

---

## S06 — CodeCarbon Hub

| Champ | Valeur |
|-------|--------|
| **URL** | https://github.com/mlco2/codecarbon |
| **Format Copper** | CSV / JSON via GitHub releases |
| **Licence** | MIT |
| **Fréquence MAJ** | Continue |
| **Politique fetch** | Mensuelle |
| **Authentification** | ❌ Aucune |
| **Entités Silver** | `training_runs`, `inference_runs` |

**Usage** : mesures réelles d'entraînement et d'inférence soumises par la communauté. Élément de validation croisée.

---

## S07 — ML.Energy Leaderboard

| Champ | Valeur |
|-------|--------|
| **URL** | https://ml.energy/leaderboard |
| **Format Copper** | CSV publié (+ scraping fallback) |
| **Licence** | CC-BY 4.0 |
| **Fréquence MAJ** | Mensuelle |
| **Politique fetch** | Mensuelle |
| **Authentification** | ❌ Aucune |
| **Entités Silver** | `inference_benchmarks` |

**Usage** : benchmarks contrôlés sur hardware standardisé (NVIDIA A100/H100). Référence pour validation des distributions Sobr.ia.

---

## S08 — Papers académiques

| Champ | Valeur |
|-------|--------|
| **Sources** | arXiv, ACL Anthology, NeurIPS, Nature, etc. |
| **Format Copper** | PDF + extracts manuels |
| **Licence** | Variable (arXiv libre, journaux à vérifier) |
| **Fréquence MAJ** | Ad hoc |
| **Politique fetch** | Manuelle curatée |
| **Authentification** | Selon source (la plupart libre) |
| **Entités Silver** | `extracted_measures` |

**Usage** : validation croisée méthodologique (Luccioni 2023, Patterson 2021, Faiz 2024, etc.).

---

# 🥉 Tier 3 — Source optionnelle (compte gratuit)

## S09 — RTE eco2mix (mix électrique français live)

| Champ | Valeur |
|-------|--------|
| **URL** | https://www.rte-france.com/eco2mix |
| **API** | https://digital.iservices.rte-france.com/ (OAuth2) |
| **Format Copper** | JSON |
| **Licence** | Etalab 2.0 |
| **Fréquence MAJ amont** | Temps réel (15 min) |
| **Politique fetch** | Horaire |
| **Authentification** | OAuth2 client credentials — **clé gratuite après inscription** |
| **Entités Silver** | `mix_hourly_fr`, `mix_daily_fr` |
| **Optionnalité** | ✅ Optionnel pour v1.0 — recommandé pour storytelling fin |

**Usage** : pour les estimations temps réel et le module M4 simulateur avec trajectoire RTE 2030. Si la clé n'est pas obtenue en temps voulu, fallback sur valeur annuelle moyenne ADEME (≈ 56 gCO₂eq/kWh pour la France).

**Fallback sans clé** : RTE publie aussi des **CSV historiques téléchargeables sans authentification** sur leur portail open data (https://opendata.reseaux-energies.fr/). Ces CSV suffisent pour les analyses retrospectives.

---

# ❌ Sources supprimées du périmètre v1.0

| Source | Raison de suppression | Replacement |
|--------|----------------------|-------------|
| Electricity Maps | Plan gratuit limité, licence CC-BY-SA virale | RTE eco2mix pour FR, ADEME pour autres pays |
| MaxMind GeoLite2 | Compte gratuit requis, CC-BY-SA virale | Pour M9 : mapping provider → zone (heuristique) ; pour M12 : pas besoin (IRIS = géocodage natif) |

---

# Model catalog — official sources per provider

> **Checked on 2026-10-31** (SOB-6). Scope: the nine providers of the current model referential, `MODEL_REGISTRY` in `crates/sobria-estimator/src/model_presets.rs` (34 presets on that date): Anthropic, OpenAI, Google, Meta, Mistral AI, DeepSeek, xAI, Alibaba, Microsoft.
> This is a factual record of what the official pages say, not legal advice. Whether an automated access is allowed is for Thibault to confirm (legal matters are human-only). "Not verified" means the page could not be opened, or the wording was not checked word for word. "No sentence found" and "silent" mean only that nothing appeared in what we read; they do not prove that no restriction exists.
> These are candidate sources for the model catalog, not ADR-0009 sources yet: each one will need its own Copper, Silver and Gold implementation when ingest is written (out of scope for SOB-6).

## One row per provider

| Provider | 1. Official model list (API) | 2. Pricing page | 3. Terms on automated access | 4. Context windows and retired models | 5. Scraping needed? Allowed? |
|---|---|---|---|---|---|
| **Anthropic** | `GET /v1/models` ([doc](https://platform.claude.com/docs/en/api/models-list)); API key; returns `id`, `display_name`, `created_at`, `max_input_tokens`, `max_tokens`, `lifecycle`, `deprecated_at`, `retires_at`, `capabilities`, `line` | [Pricing](https://platform.claude.com/docs/en/about-claude/pricing): HTML tables, no JSON; Markdown copies listed in [llms.txt](https://platform.claude.com/llms.txt) | [Consumer Terms](https://www.anthropic.com/legal/consumer-terms) s.3 limit crawling and automated access to "our Services"; [Commercial Terms](https://www.anthropic.com/legal/commercial-terms): no sentence found | In the API (`max_input_tokens`, `lifecycle` including `retired`, `deprecated_at`, `retires_at`); also [models overview](https://platform.claude.com/docs/en/about-claude/models/overview) and [deprecations](https://platform.claude.com/docs/en/about-claude/model-deprecations) (HTML) | Only for prices. Commercial Terms silent; Consumer Terms cover "our Services": not confirmed whether the docs pages count |
| **OpenAI** | `GET /v1/models` ([doc](https://developers.openai.com/api/reference/resources/models/methods/list)); API key; returns `id`, `created`, `owned_by`, optional `shutdown_date`; no display name, context window or price | [Pricing](https://developers.openai.com/api/docs/pricing): HTML; Markdown copies of docs pages and an index in [llms.txt](https://developers.openai.com/llms.txt) | [Terms of use](https://openai.com/policies/terms-of-use/), [Business Terms](https://openai.com/policies/business-terms/), [Usage Policies](https://openai.com/policies/usage-policies/) and [Service Terms](https://openai.com/policies/service-terms/) returned HTTP 403 to our tools: **not verified**. [robots.txt](https://developers.openai.com/robots.txt): `Allow: /` | Context windows on the [models page](https://developers.openai.com/api/docs/models) (HTML); [deprecations](https://developers.openai.com/api/docs/deprecations) (HTML); shutdown dates also in the API | For prices and context windows. Terms not verified: cannot say |
| **Google** (Gemini API) | `GET /v1beta/models` ([doc](https://ai.google.dev/api/models)); API key; returns `name`, `displayName`, `version`, `inputTokenLimit`, `outputTokenLimit`, `supportedGenerationMethods`; no price or date. Vertex AI list endpoint: not verified | [Pricing](https://ai.google.dev/gemini-api/docs/pricing): HTML | [Gemini API Terms](https://ai.google.dev/gemini-api/terms), [Google ToS](https://policies.google.com/terms), [Google APIs Terms](https://developers.google.com/terms) s.5(e)(1): see details | Token limits in the API and on per-model pages; [deprecations](https://ai.google.dev/gemini-api/docs/deprecations): HTML table with shutdown dates, not said to be in the API | For prices and shutdown dates. The terms read have no clause on reading the docs pages; the Google ToS ties automated access to robots.txt, which allows them ([robots.txt](https://ai.google.dev/robots.txt)) |
| **Meta** (Llama) | No official hosted Llama API found. Meta's own API serves Muse models: `GET https://api.meta.ai/v1/models` ([doc](https://dev.meta.ai/docs/api-reference/models/list-models)), key needed, Llama not seen. Llama list: official Hugging Face org [meta-llama](https://huggingface.co/meta-llama), `GET https://huggingface.co/api/models?author=meta-llama` ([doc](https://huggingface.co/docs/hub/api)), no key | [Meta pricing](https://dev.meta.ai/docs/pricing-rate-limits): Muse, image, voice and SAM only; **no Llama prices from Meta** | [Facebook terms](https://www.facebook.com/terms) s.3.2(3) (French text read): limit automated data collection from the Products; see details. Meta's developer terms and the Llama licence and acceptable use policy: not checked | Llama context windows only in HTML model cards ([dev.meta.ai/llama](https://dev.meta.ai/llama/)); `config.json` of a `meta-llama` repo on Hugging Face returned HTTP 401 (gated, manual approval); no deprecations page found | Prices do not exist officially. Context windows from the HTML model cards, or from `config.json` with a Hugging Face token after the licence is accepted (a human step). The Facebook terms limit automated collection from the Products; docs pages not addressed |
| **Mistral AI** | `GET /v1/models` ([doc](https://docs.mistral.ai/api/endpoint/models), [OpenAPI](https://docs.mistral.ai/openapi.yaml)); API key; returns `id`, `created`, `name`, `description`, `max_context_length`, `capabilities`, `aliases`, `deprecation`, `deprecation_replacement_model` | [Pricing](https://docs.mistral.ai/inference/pricing): HTML, Markdown copy at `/pricing.md`; no price field in the API | [Commercial Terms](https://legal.mistral.ai/terms/commercial-terms-of-service) s.2.2(g) limit content extraction (effective 2026-09-25); [Usage Policy](https://legal.mistral.ai/terms/usage-policy): no sentence found | In the API (`max_context_length`, `deprecation`); [models page](https://docs.mistral.ai/models) lists deprecated and retired models with dates; [lifecycle policy](https://docs.mistral.ai/inference/model-lifecycle) | Only for prices. s.2.2(g) restricts extraction "other than as permitted"; silent on reading public docs; [llms.txt](https://docs.mistral.ai/llms.txt) serves Markdown copies of the docs pages for LLM use |
| **DeepSeek** | `GET https://api.deepseek.com/models` ([doc](https://api-docs.deepseek.com/api/list-models)); API key (401 without); returns `id`, `owned_by`, `name`, `context_window`, `max_output_tokens`, modalities, `api_capabilities`; no created date, no deprecation field | [Pricing](https://api-docs.deepseek.com/quick_start/pricing): HTML table (peak and off-peak) | [Terms of Use](https://cdn.deepseek.com/policies/en-US/deepseek-terms-of-use.html) s.3.5(3) bar robots and copying (updated 2026-03-27); [Open Platform Terms](https://cdn.deepseek.com/policies/en-US/deepseek-open-platform-terms-of-service.html): no such sentence found | Context windows in the API; deprecations only in the HTML [changelog](https://api-docs.deepseek.com/updates) and the pricing footnote | For prices and deprecations. s.3.5(3) bars copying content of the "Services" with robots or spiders; docs site not addressed |
| **xAI** | `GET /v1/models` and `/v1/language-models` ([doc](https://docs.x.ai/developers/rest-api-reference/inference/models)); API key; returns `id`, `aliases`, `created`, `context_length`, token prices; no deprecation field | [Pricing](https://docs.x.ai/developers/pricing): HTML; prices also in the API | The pages are branded SpaceXSI. [Acceptable Use Policy](https://x.ai/legal/acceptable-use-policy) (effective 2026-08-14): bars using bots to access the Service to reverse engineer it, and scraping or harvesting Input or Output; [Terms of Service](https://x.ai/legal/terms-of-service) (consumer, updated 2026-10-27): no scraping sentence found by keyword search; the API (enterprise) terms: not checked. Our fetch tool got HTTP 403 on both pages; a plain request got HTTP 200. [robots.txt](https://docs.x.ai/robots.txt): `Allow: /` with `Content-Signal: ai-train=no, search=yes, ai-input=yes` | Context windows in the API and on the [models page](https://docs.x.ai/developers/models); deprecations only in HTML ([example](https://docs.x.ai/developers/migration/may-15-retirement)) | Not for prices or context windows (API, with a key). Deprecations are HTML only. The two pages read do not address reading the docs; the API terms are not checked |
| **Alibaba** (Qwen) | Model Studio `GET https://dashscope-intl.aliyuncs.com/api/v1/models` ([doc](https://www.alibabacloud.com/help/en/model-studio/list-models)); API key; returns `model`, `name`, `context_window`, `max_input_tokens`, `max_output_tokens`, `prices[]`, `published_time`, `offline_time`. Hugging Face org Qwen: `GET https://huggingface.co/api/models?author=Qwen`, no key | [Pricing](https://www.alibabacloud.com/help/en/model-studio/model-pricing): HTML; prices also in the API | [Website terms](https://www.alibabacloud.com/help/en/legal/latest/alibaba-cloud-international-website-terms-of-use) (updated 2026-03-20) s.3.2(b) and 3.4(a) limit robots and crawling; API terms and service terms: no sentence found | Context windows and an estimated `offline_time` in the API; [deprecations](https://www.alibabacloud.com/help/en/model-studio/model-depreciation): HTML, dated sections often without model names | Not for prices, context windows or offline dates (API, with a key). Deprecation history is HTML only. Website terms restrict robots and crawling of the site; API terms silent |
| **Microsoft** (referential: `phi-4-reasoning`, `phi-4-reasoning-vision`, both open-weight) | Phi: official Hugging Face org [microsoft](https://huggingface.co/microsoft), `GET https://huggingface.co/api/models?author=microsoft`, no key; the Phi-4-reasoning repos checked are not gated. Other Azure-sold models (MAI, Model Router): no public catalog-wide list API found; per subscription `GET .../Microsoft.CognitiveServices/locations/{location}/models` ([doc](https://learn.microsoft.com/en-us/rest/api/aiservices/accountmanagement/models/list)), no context window; per Azure OpenAI resource `GET {endpoint}/openai/models` ([doc](https://learn.microsoft.com/en-us/rest/api/azureopenai/models/list)), `api-key` | [Azure OpenAI pricing](https://azure.microsoft.com/en-us/pricing/details/azure-openai/): HTML. Public JSON: [Retail Prices API](https://learn.microsoft.com/en-us/rest/api/cost-management/retail-prices/azure-retail-prices); coverage of Foundry model prices not verified | [Microsoft Services Agreement](https://www.microsoft.com/en-us/servicesagreement) s.14.s.iii (effective 2026-09-30) limits scraping of the AI services; whether it governs Azure: **not verified**; [Azure Product Terms](https://www.microsoft.com/licensing/terms/productoffering/MicrosoftAzure/AllEngagements): not checked. For Phi on Hugging Face, see the Hugging Face section | Phi: model cards and `config.json` on Hugging Face (HTTP 200 without a key for the Phi-4-reasoning repo checked). Other Azure models: context windows in HTML tables ([sold by Azure](https://learn.microsoft.com/en-us/azure/foundry/foundry-models/concepts/models-sold-directly-by-azure), [partner models](https://learn.microsoft.com/en-us/azure/foundry/foundry-models/concepts/models-from-partners)); [retirement schedule](https://learn.microsoft.com/en-us/azure/foundry/openai/concepts/model-retirement-schedule): HTML; deprecation dates also in the APIs above (Azure credentials) | Not for Phi names, dates or context windows. For Azure prices and other Azure models: Retail API or HTML. The Services Agreement sentence concerns the AI services and is silent on docs and pricing pages; robots.txt does not block them |

## Terms on automated access, in detail

All pages were read on 2026-10-31 through a fetch tool that summarises pages, so every sentence below must be confirmed on the page before anyone relies on it. The quoted sentences of Anthropic, Mistral AI and DeepSeek came back with the same wording from two separate fetches; the others are close paraphrases.

- **Anthropic**: [Consumer Terms](https://www.anthropic.com/legal/consumer-terms) s.3, quoted: "To crawl, scrape, or otherwise harvest data or information from our Services other than as permitted under these Terms." and "to access the Services through automated or non-human means, whether through a bot, script, or otherwise", except when accessing through an Anthropic API key or where Anthropic explicitly permits it. [platform.claude.com/robots.txt](https://platform.claude.com/robots.txt) disallows only `/api/`.
- **Mistral AI**: [Commercial Terms](https://legal.mistral.ai/terms/commercial-terms-of-service) s.2.2(g), start of the sentence, quoted: "use any method to extract any content from the Mistral AI Products other than as permitted". The EU and rest-of-world consumer terms under [legal.mistral.ai/terms](https://legal.mistral.ai/terms/) use the same wording.
- **DeepSeek**: [Terms of Use](https://cdn.deepseek.com/policies/en-US/deepseek-terms-of-use.html) s.3.5(3), quoted: "capturing, copying any content of the Services, including but not limited to using any robots, spiders, or other automatic setups, setting mirrors".
- **Google**: the Gemini API Terms bar reverse engineering, extracting or replicating components of the Services, including the underlying data or models, and set rules on programmatic collection of grounding links and Google Maps data. The Google Terms of Service bar automated access that violates the machine-readable instructions of a page, for example robots.txt. The Google APIs Terms (s.5(e)(1)) bar scraping or building permanent copies of API content.
- **Meta**: the consumer Facebook terms (French text; English wording not verified) bar collecting data from the Products or accessing it by automated means. They are not the terms that govern the developer docs or Llama: Meta's developer terms and the Llama licence and acceptable use policy were **not checked**. A comment in [developer.meta.com/robots.txt](https://developer.meta.com/robots.txt) says the same about Facebook data without written permission. [dev.meta.ai/robots.txt](https://dev.meta.ai/robots.txt) does not disallow the docs.
- **Alibaba**: the website terms bar access through robots, spiders, automatic devices or manual processes (s.3.2(b)) and monitoring or crawling of any System (s.3.4(a)). [alibabacloud.com/robots.txt](https://www.alibabacloud.com/robots.txt) disallows `/api/*` and `/*.json`, with no rule for `/help/`.
- **Microsoft**: the Services Agreement s.14.s.iii bars web scraping, web harvesting or web data extraction methods to extract data from the AI services. The [Azure Product Terms](https://www.microsoft.com/licensing/terms/productoffering/MicrosoftAzure/AllEngagements) were not opened.
- **xAI**: the pages are branded SpaceXSI and answered HTTP 403 to our fetch tool, but a plain request got HTTP 200, so they were read once with a script and searched by keyword. The [Acceptable Use Policy](https://x.ai/legal/acceptable-use-policy) (effective 2026-08-14) lists, among the ways of detrimentally impacting the Service, using bots to access it in order to reverse engineer it, and scraping, harvesting or reselling any Input or Output. The consumer [Terms of Service](https://x.ai/legal/terms-of-service) (updated 2026-10-27) have no scraping sentence. The API (enterprise) terms were not checked.
- **OpenAI**: [terms of use](https://openai.com/policies/terms-of-use/), [Business Terms](https://openai.com/policies/business-terms/), [Usage Policies](https://openai.com/policies/usage-policies/) and [Service Terms](https://openai.com/policies/service-terms/) answered HTTP 403 to every client we tried, so nothing was read. Thibault should read them in a browser before any automated access is designed.

## Hugging Face, the source recommended for open-weight models

The listings of the official orgs `meta-llama` (Llama), `Qwen` (Alibaba) and `microsoft` (Phi) are the only machine-readable source found for these open-weight models that needs no key. Checked on 2026-10-31:

- **API**: `GET https://huggingface.co/api/models?author=<org>` ([doc](https://huggingface.co/docs/hub/api)) returns `id`, `createdAt`, `downloads` and `tags`, without a key, for the three orgs.
- **Terms**: the [Terms of Service](https://huggingface.co/terms-of-service) (effective 2022-09-15) have no sentence on scraping, crawling or automated access, and describe running models through an API as an intended use. [robots.txt](https://huggingface.co/robots.txt): `Allow: /`.
- **Rate limits** ([doc](https://huggingface.co/docs/hub/rate-limits), values for September 2025): anonymous clients get 500 Hub API calls per 5 minutes and per IP, free accounts 1,000; file downloads (`/resolve/`) get 3,000 and 5,000. Hugging Face advises always sending a token. A weekly job needs a few dozen calls, far below the anonymous limit.
- **Gating**: `config.json` of the `Qwen` and `microsoft` Phi-4-reasoning repos checked returned HTTP 200 without a key. For a `meta-llama` repo it returned HTTP 401 (`gated: manual`): the licence must be accepted with an account, which is a human step.

## Providers without an official machine-readable source

| Provider | What is missing | Recommendation |
|---|---|---|
| **Meta** (Llama) | No hosted Llama API found and no Llama prices from Meta. Meta's own API lists Muse models, not Llama. The Llama `config.json` files are gated | Take names and dates from the listing of the Hugging Face org `meta-llama` (JSON, no key). Take context windows from the HTML model cards, or from `config.json` with a token once Thibault has accepted the licence. Record the price as "not published by Meta"; do not scrape Meta pages |
| **Microsoft** | For the two Phi models of the referential, names, dates and context windows are on Hugging Face (org `microsoft`); Microsoft publishes no Phi price outside Azure. For other Azure-sold models (MAI, Model Router) there is no public catalog-wide model list API and context windows are HTML only | Use the Hugging Face listing for Phi. For prices, use the Azure Retail Prices API once its coverage of Foundry models is checked. For an Azure-only model, read the Microsoft Learn pages by hand until Thibault confirms that a scheduled read is within the terms |

For every other provider the model list is machine-readable, but **prices are HTML only** at Anthropic, OpenAI, Google, Mistral and DeepSeek (Anthropic, OpenAI and Mistral offer Markdown copies of their docs pages). Only xAI and Alibaba return prices in their list API.

## Open points for Thibault

1. Every official list API found needs an API key (or Azure credentials for Microsoft). Only the Hugging Face listings (Meta Llama, Alibaba Qwen, Microsoft Phi) need none. Creating accounts and keys, and accepting the Llama licence, are human-only matters. CLAUDE.md requires v1.0 to work without any key, so a key would belong to the scheduled catalog job, not to the app.
2. Read the OpenAI terms (HTTP 403 for every client we tried), the xAI API (enterprise) terms, Microsoft's Azure Product Terms, Meta's developer terms and the Llama licence and acceptable use policy: none of them was read.
3. Confirm, provider by provider, whether reading the public pricing and deprecation pages on a schedule is acceptable where there is no API for them (prices: Anthropic, OpenAI, Google, Mistral, DeepSeek; deprecations: Google, DeepSeek, xAI, Alibaba).

---

# Vue d'ensemble — matrice rapide

| ID | Source | Tier | Bloquant ? | Sprint cible |
|----|--------|------|------------|--------------|
| S01 | **ComparIA** | 1 🎯 | **Oui** | **S2** |
| S02 | **RTE IRIS** | 1 🎯 | **Oui** | **S2** |
| S03 | ADEME Base Empreinte | 2 | Oui | S2 |
| S04 | EcoLogits | 2 | Oui (méthodo) | S0 + S3 |
| S05 | HF AI Energy Score | 2 | Recommandé | S3 |
| S06 | CodeCarbon | 2 | Recommandé | S3 |
| S07 | ML.Energy | 2 | Recommandé | S3 |
| S08 | Papers | 2 | Oui (validation) | S0 + S3 |
| S09 | RTE eco2mix | 3 | Non (optionnel) | S3 |

**8 sources actives au lieu de 10**, **0 clé bloquante** pour v1.0, **2 datasets officiels du défi** au cœur.

---

# Schémas Silver (vue d'ensemble)

```
schemas/silver/
├── comparia_conversations-v1.json   (S01)
├── comparia_votes-v1.json           (S01)
├── comparia_reactions-v1.json       (S01)
├── iris_consommation-v1.json        (S02)
├── iris_geometries-v1.json          (S02)
├── electricity_factors-v1.json      (S03)
├── hardware_factors-v1.json         (S03)
├── ecologits_models-v1.json         (S04)
├── ecologits_assumptions-v1.json    (S04)
├── model_energy_score-v1.json       (S05)
├── training_runs-v1.json            (S06)
├── inference_runs-v1.json           (S06)
├── inference_benchmarks-v1.json     (S07)
├── extracted_measures-v1.json       (S08)
└── mix_hourly_fr-v1.json            (S09, optionnel)
```

---

# Pré-requis avant ingestion (S2)

- [x] Stratégie de sources figée (ce document)
- [ ] Stockage DVC remote configuré (R2 ou S3 — ComparIA pèse 5 GB)
- [ ] Schémas Silver v1 figés et committés
- [ ] Tests `proptest` template prêts
- [ ] (Optionnel) Clé API RTE eco2mix obtenue (Thibault)

---

# Politique de rétention par source

| Source | Politique Copper |
|--------|-------------------|
| ComparIA | 1 snapshot par MAJ trimestrielle (4/an), conservé indéfiniment (lineage scientifique) |
| RTE IRIS | 1 snapshot par MAJ annuelle, conservé indéfiniment |
| ADEME | 30 derniers snapshots, puis mensuels 2 ans, puis annuels |
| EcoLogits | Tous les snapshots de release GitHub (versionnement strict) |
| Autres | 30 derniers snapshots, puis mensuels 1 an |

---

*Ce catalogue est vivant. Toute nouvelle source = nouvelle fiche + nouvelle implémentation `DataLayer`. Si le défi ajoute un dataset officiel, il passe directement en Tier 1.*
