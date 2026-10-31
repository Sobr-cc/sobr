# Data source catalog — Sobr.ia

> **Version**: 2.2 — translated to English (2026-10-31, SOB-149); 2.1 — model catalog sources section added (2026-10-31, SOB-6); 2.0 — ComparIA pivot (12 May 2026).
> Each source goes through the medallion pipeline (ADR-0009).
> Any change = update of the matching Silver schema + version bump.

---

## Prioritization strategy

| Tier | Criterion | Sources |
|------|-----------|---------|
| **Tier 1** | Official datasets referenced by the data.gouv.fr challenge | ComparIA, RTE-NaTran-Teréga IRIS |
| **Tier 2** | Public, free sources, no authentication | ADEME, EcoLogits, ML.Energy, CodeCarbon, HF AI Energy Score, Papers |
| **Tier 3** | Strategic national source with a free account | RTE eco2mix (live FR electricity mix) |
| **Removed** | Sources that need a paid account or a restrictive key | Electricity Maps, MaxMind GeoLite2 |

**v1.0 commitment**: zero blocking source behind a paywall. All Tier 1 and Tier 2 sources can be downloaded anonymously.

---

# 🥇 Tier 1 — Official datasets of the challenge

## S01 — ComparIA (Ministry of Culture / Beta.gouv)

| Field | Value |
|-------|-------|
| **data.gouv.fr URL** | https://www.data.gouv.fr/datasets/compar-ia |
| **Platform** | https://comparia.beta.gouv.fr/ |
| **Source code** | https://github.com/betagouv/ComparIA |
| **Producer** | Ministry of Culture (DINUM / Beta.gouv) |
| **Licence** | **Licence Ouverte / Open Licence v2.0 (Etalab 2.0)** |
| **Sobr.ia compatibility** | ✅ Perfect (Etalab ↔ MIT/CC-BY) |
| **Update frequency** | Quarterly (snapshot of the last update: 5 March 2026) |
| **Fetch policy** | Weekly (metadata poll) |
| **Authentication** | ❌ None |
| **Challenge status** | 🎯 Central dataset |

### Files to ingest

| File | Format | Size | Download URL |
|------|--------|------|--------------|
| Conversations | Parquet | 682 MB | https://www.data.gouv.fr/api/1/datasets/r/7651fd0b-f222-43b3-8db8-ed6ae660d313 |
| Votes | Parquet | 733 MB | https://www.data.gouv.fr/api/1/datasets/r/4ffc86e1-84a4-4fdc-9726-66408e596fef |
| Reactions | Parquet | 3.4 GB | https://www.data.gouv.fr/api/1/datasets/r/9dd3d51f-4299-4193-ab46-81ae039fe1be |
| Documentation | CSV | 8.9 KB | https://www.data.gouv.fr/api/1/datasets/r/278fb12a-f75a-4891-add8-fff1d82f1622 |

**Total volume: ~5 GB** — large, but columnar Parquet allows smooth ingestion into DuckDB / polars.

### Silver entities produced

- `comparia_conversations.parquet` — questions + answers + model metadata + environmental impact per message
- `comparia_votes.parquet` — user preferences per model pair
- `comparia_reactions.parquet` — message-by-message reactions (fine granularity)

### Critical fields for Sobr.ia

- Model used (provider + name + version)
- Actual input / output tokens (measured, not estimated)
- Environmental impact computed by EcoLogits (CO₂eq, energy, water if present)
- Server location (datacenter)
- Question type (thematic category)
- User preferences (quality signal)

### Why it is crucial

- **Real usage data** in French (~250,000 questions, ~150,000 votes).
- **Official environmental methodology** (EcoLogits + ISO 14044) already applied.
- **Cross-referenceable** with ComparIA is French cultural mention → territorial consistency.
- **Etalab 2.0 licence** → republication possible.

### Risks

- Large volume → DVC mandatory, no Git check-in.
- User data potentially sensitive (questions written in plain text) → check the pseudonymization in the ComparIA documentation.
- Uneven coverage of closed/open models.

### Tests / validation

- Consistency: sum(impact per message) = impact per conversation.
- Mandatory fields present on ≥ 99% of the rows.
- Expected distribution of models (≥ 30 models covered).

---

## S02 — Annual IRIS consumption of industrial sites (RTE / NaTran / Teréga)

| Field | Value |
|-------|-------|
| **data.gouv.fr URL** | https://www.data.gouv.fr/datasets/consommation-annuelle-definitive-delectricite-et-de-gaz-par-iris-des-sites-industriels-raccordes-aux-reseaux-de-transport |
| **Original source** | https://odre.opendatasoft.com/explore/dataset/consommation-annuelle-par-iris/ |
| **Producers** | NaTran, RTE, Teréga (electricity + gas transmission network operators) |
| **Distributor** | Open Data Réseaux Énergies (ODRÉ) |
| **Licence** | **Licence Ouverte / Open Licence v2.0 (Etalab 2.0)** |
| **Sobr.ia compatibility** | ✅ Perfect |
| **Update frequency** | Annual (last: 6 December 2024, IRIS 2023 referential) |
| **Fetch policy** | Annual (triggered manually at each ODRÉ update) |
| **Authentication** | ❌ None |
| **Challenge status** | 🎯 Official dataset — territorial angle |

### Files to ingest

| File | Format | Size | Download URL |
|------|--------|------|--------------|
| Tabular data | CSV | 90.2 MB | https://www.data.gouv.fr/api/1/datasets/r/631d6ec4-74c5-4f0f-9187-442cf9d1f0bc |
| Data + coordinates (JSON) | JSON | 97.8 MB | https://www.data.gouv.fr/api/1/datasets/r/67a173e0-30c6-461c-aed5-3f6a8ab15a09 |
| GeoJSON | JSON | 91.0 MB | https://www.data.gouv.fr/api/1/datasets/r/2b584d52-f4e0-4232-87df-c456e715f334 |
| Shapefile (ZIP) | ZIP | 38.9 MB | https://www.data.gouv.fr/api/1/datasets/r/95fb8941-47a9-4d15-8c7a-3320fa0e0319 |

### Silver entities produced

- `iris_consommation_annuelle.parquet` — electricity + gas consumption (MWh) per IRIS + number of sites
- `iris_geometries.parquet` (from the GeoJSON / shapefile) — IRIS polygons for map rendering

### Critical fields

- IRIS code (INSEE 2023 referential)
- Annual electricity consumption (MWh)
- Annual gas consumption (MWh)
- Number of industrial sites connected to the transmission network
- Reference year
- Geometry of the IRIS polygon

### Usage in Sobr.ia

- **Module M12 — French territory**: choropleth visualization of industrial concentrations.
- **Datacenter heuristic**: an IRIS with > N MWh electricity + low gas is a candidate to host a datacenter (to be validated).
- **Regional scenarios**: project the impact of an LLM deployment by French region.
- **Storytelling**: "here are the 10 IRIS that concentrate X% of industrial electricity consumption — how much AI usage corresponds to an average IRIS?"

### Risks

- Annual data → coarse temporal granularity (offset by eco2mix for live data).
- Anonymization by thresholds: some IRIS may be under "statistical secrecy" (secret statistique) → special handling.
- Yearly evolution of the IRIS polygons (INSEE) → version the referential.

### Tests / validation

- Sum of the consumption per region consistent with the national RTE/GRDF balances (±5%).
- Valid geometries (GeoJSON RFC 7946).
- Number of IRIS covered (~50,000 IRIS in France) → check the coverage ratio.

---

# 🥈 Tier 2 — Complementary sources (no authentication)

## S03 — ADEME Base Empreinte

| Field | Value |
|-------|-------|
| **URL** | https://base-empreinte.ademe.fr/ |
| **API** | https://data.ademe.fr/datasets/base-empreinte |
| **Copper format** | CSV + JSON |
| **Licence** | Etalab 2.0 |
| **Update frequency** | Quarterly |
| **Fetch policy** | Weekly |
| **Authentication** | ❌ None |
| **Silver entities** | `electricity_factors`, `hardware_factors` |

**Usage**: reference emission factors (gCO₂eq/kWh by country, by source, by year). Essential to normalize ComparIA → CO₂eq when the field is not pre-filled.

---

## S04 — GenAI Impact / EcoLogits

| Field | Value |
|-------|-------|
| **URL** | https://github.com/genai-impact/ecologits |
| **Copper format** | JSON + Python (model catalog) |
| **Licence** | MIT |
| **Update frequency** | Continuous (GitHub releases) |
| **Fetch policy** | Weekly (releases poll) |
| **Authentication** | ❌ None |
| **Silver entities** | `ecologits_models`, `ecologits_assumptions` |

**Usage**: **the official methodology** already adopted by ComparIA. Sobr.ia must apply it faithfully (and validate it on the 3 reference studies). Importing the model catalog + uncertainty parameters also makes it possible to estimate models not covered by ComparIA.

---

## S05 — Hugging Face AI Energy Score

| Field | Value |
|-------|-------|
| **URL** | https://huggingface.co/AIEnergyScore |
| **Copper format** | JSON / Parquet via HF Hub |
| **Licence** | Apache 2.0 (models) / CC-BY (scores) |
| **Update frequency** | Ad hoc (new models) |
| **Fetch policy** | Weekly |
| **Authentication** | ❌ None (public read on HF Hub) |
| **Silver entities** | `model_energy_score` |

**Usage**: complements ComparIA for the models it does not cover. A standardized score allows comparisons.

---

## S06 — CodeCarbon Hub

| Field | Value |
|-------|-------|
| **URL** | https://github.com/mlco2/codecarbon |
| **Copper format** | CSV / JSON via GitHub releases |
| **Licence** | MIT |
| **Update frequency** | Continuous |
| **Fetch policy** | Monthly |
| **Authentication** | ❌ None |
| **Silver entities** | `training_runs`, `inference_runs` |

**Usage**: real training and inference measurements submitted by the community. Cross-validation element.

---

## S07 — ML.Energy Leaderboard

| Field | Value |
|-------|-------|
| **URL** | https://ml.energy/leaderboard |
| **Copper format** | Published CSV (+ scraping fallback) |
| **Licence** | CC-BY 4.0 |
| **Update frequency** | Monthly |
| **Fetch policy** | Monthly |
| **Authentication** | ❌ None |
| **Silver entities** | `inference_benchmarks` |

**Usage**: controlled benchmarks on standardized hardware (NVIDIA A100/H100). Reference to validate the Sobr.ia distributions.

---

## S08 — Academic papers

| Field | Value |
|-------|-------|
| **Sources** | arXiv, ACL Anthology, NeurIPS, Nature, etc. |
| **Copper format** | PDF + manual extracts |
| **Licence** | Variable (arXiv open, journals to check) |
| **Update frequency** | Ad hoc |
| **Fetch policy** | Manual, curated |
| **Authentication** | Depends on the source (most are open) |
| **Silver entities** | `extracted_measures` |

**Usage**: methodological cross-validation (Luccioni 2023, Patterson 2021, Faiz 2024, etc.).

---

# 🥉 Tier 3 — Optional source (free account)

## S09 — RTE eco2mix (live French electricity mix)

| Field | Value |
|-------|-------|
| **URL** | https://www.rte-france.com/eco2mix |
| **API** | https://digital.iservices.rte-france.com/ (OAuth2) |
| **Copper format** | JSON |
| **Licence** | Etalab 2.0 |
| **Upstream update frequency** | Real time (15 min) |
| **Fetch policy** | Hourly |
| **Authentication** | OAuth2 client credentials — **free key after registration** |
| **Silver entities** | `mix_hourly_fr`, `mix_daily_fr` |
| **Optionality** | ✅ Optional for v1.0 — recommended for fine storytelling |

**Usage**: for real-time estimates and the M4 simulator module with the RTE 2030 trajectory. If the key is not obtained in time, fall back on the ADEME annual average value (≈ 56 gCO₂eq/kWh for France).

**Fallback without a key**: RTE also publishes **historical CSVs downloadable without authentication** on its open data portal (https://opendata.reseaux-energies.fr/). These CSVs are enough for retrospective analyses.

---

# ❌ Sources removed from the v1.0 scope

| Source | Reason for removal | Replacement |
|--------|--------------------|-------------|
| Electricity Maps | Limited free plan, viral CC-BY-SA licence | RTE eco2mix for FR, ADEME for other countries |
| MaxMind GeoLite2 | Free account required, viral CC-BY-SA | For M9: provider → zone mapping (heuristic); for M12: not needed (IRIS = native geocoding) |

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
| **Mistral AI** | `GET /v1/models` ([doc](https://docs.mistral.ai/api/endpoint/models), [OpenAPI](https://docs.mistral.ai/openapi.yaml)); API key; returns `id`, `created`, `name`, `description`, `max_context_length`, `capabilities`, `aliases`, `deprecation`, `deprecation_replacement_model`. The 5 open-weight presets are also listed by the Hugging Face org [mistralai](https://huggingface.co/mistralai), no key | [Pricing](https://docs.mistral.ai/inference/pricing): HTML, Markdown copy at `/pricing.md`; no price field in the API | [Commercial Terms](https://legal.mistral.ai/terms/commercial-terms-of-service) s.2.2(g) limit content extraction (effective 2026-09-25); [Usage Policy](https://legal.mistral.ai/terms/usage-policy): no sentence found | In the API (`max_context_length`, `deprecation`); [models page](https://docs.mistral.ai/models) lists deprecated and retired models with dates; [lifecycle policy](https://docs.mistral.ai/inference/model-lifecycle) | Only for prices. s.2.2(g) restricts extraction "other than as permitted"; silent on reading public docs; [llms.txt](https://docs.mistral.ai/llms.txt) serves Markdown copies of the docs pages for LLM use |
| **DeepSeek** | `GET https://api.deepseek.com/models` ([doc](https://api-docs.deepseek.com/api/list-models)); API key (401 without); returns `id`, `owned_by`, `name`, `context_window`, `max_output_tokens`, modalities, `api_capabilities`; no created date, no deprecation field. The 2 open-weight presets (`deepseek-v4-pro`, `deepseek-r1`) are also listed by the Hugging Face org [deepseek-ai](https://huggingface.co/deepseek-ai), no key | [Pricing](https://api-docs.deepseek.com/quick_start/pricing): HTML table (peak and off-peak) | [Terms of Use](https://cdn.deepseek.com/policies/en-US/deepseek-terms-of-use.html) s.3.5(3) bar robots and copying (updated 2026-03-27); [Open Platform Terms](https://cdn.deepseek.com/policies/en-US/deepseek-open-platform-terms-of-service.html): no such sentence found | Context windows in the API; deprecations only in the HTML [changelog](https://api-docs.deepseek.com/updates) and the pricing footnote | For prices and deprecations. s.3.5(3) bars copying content of the "Services" with robots or spiders; docs site not addressed |
| **xAI** | `GET /v1/models` and `/v1/language-models` ([doc](https://docs.x.ai/developers/rest-api-reference/inference/models)); API key; returns `id`, `aliases`, `created`, `context_length`, token prices; no deprecation field | [Pricing](https://docs.x.ai/developers/pricing): HTML; prices also in the API | The pages are branded SpaceXSI. [Acceptable Use Policy](https://x.ai/legal/acceptable-use-policy) (effective 2026-08-14): bars using bots to access the Service to reverse engineer it, and scraping or harvesting Input or Output; [Terms of Service](https://x.ai/legal/terms-of-service) (consumer, updated 2026-10-27): no scraping sentence found by keyword search; the API (enterprise) terms: not checked. Our fetch tool got HTTP 403 on both pages; a plain request got HTTP 200. [robots.txt](https://docs.x.ai/robots.txt): `Allow: /` with `Content-Signal: ai-train=no, search=yes, ai-input=yes` | Context windows in the API and on the [models page](https://docs.x.ai/developers/models); deprecations only in HTML ([example](https://docs.x.ai/developers/migration/may-15-retirement)) | Not for prices or context windows (API, with a key). Deprecations are HTML only. The two pages read do not address reading the docs; the API terms are not checked |
| **Alibaba** (Qwen) | Model Studio `GET https://dashscope-intl.aliyuncs.com/api/v1/models` ([doc](https://www.alibabacloud.com/help/en/model-studio/list-models)); API key; returns `model`, `name`, `context_window`, `max_input_tokens`, `max_output_tokens`, `prices[]`, `published_time`, `offline_time`. The Hugging Face org Qwen (`GET https://huggingface.co/api/models?author=Qwen`, no key) lists open Qwen models only; the referential's `qwen-3-6-plus` is closed-weight, so Model Studio, with a key, is its source | [Pricing](https://www.alibabacloud.com/help/en/model-studio/model-pricing): HTML; prices also in the API | [Website terms](https://www.alibabacloud.com/help/en/legal/latest/alibaba-cloud-international-website-terms-of-use) (updated 2026-03-20) s.3.2(b) and 3.4(a) limit robots and crawling; API terms and service terms: no sentence found | Context windows and an estimated `offline_time` in the API; [deprecations](https://www.alibabacloud.com/help/en/model-studio/model-depreciation): HTML, dated sections often without model names | Not for prices, context windows or offline dates (API, with a key). Deprecation history is HTML only. Website terms restrict robots and crawling of the site; API terms silent |
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

The referential marks 14 of its 34 presets as open-weight: five Llama (Meta), five Mistral AI, two DeepSeek and two Phi (Microsoft). The listings of their official orgs, `meta-llama`, `mistralai`, `deepseek-ai` and `microsoft`, are the only machine-readable source found for these models that needs no key. The 20 other presets are closed-weight (Anthropic, OpenAI, Google, xAI and the single Alibaba preset, `qwen-3-6-plus`): their weights are not published, so each provider's own API, with a key, is the source. The `Qwen` org also lists without a key, but only open Qwen models, none of which is in the referential today. Checked on 2026-10-31:

- **API**: `GET https://huggingface.co/api/models?author=<org>` ([doc](https://huggingface.co/docs/hub/api)) returns `id`, `createdAt`, `downloads` and `tags`, without a key, for the four orgs (and for `Qwen`).
- **Terms**: the [Terms of Service](https://huggingface.co/terms-of-service) (effective 2022-09-15) have no sentence on scraping, crawling or automated access, and describe running models through an API as an intended use. [robots.txt](https://huggingface.co/robots.txt): `Allow: /`.
- **Rate limits** ([doc](https://huggingface.co/docs/hub/rate-limits), values for September 2025): anonymous clients get 500 Hub API calls per 5 minutes and per IP, free accounts 1,000; file downloads (`/resolve/`) get 3,000 and 5,000. Hugging Face advises always sending a token. A weekly job needs a few dozen calls, far below the anonymous limit.
- **Gating**: `config.json` of the repos checked returned HTTP 200 without a key for `microsoft` (Phi-4-reasoning), `mistralai` (Mistral-Large-4, Mistral-Small-4, Mistral-Medium-3.5) and `deepseek-ai` (DeepSeek-R1, DeepSeek-V4 repos), and none is gated. For the `meta-llama` repos checked (Llama-4 Scout and Maverick, Llama-3.3-70B) it returned HTTP 401 (`gated: manual`): the licence must be accepted with an account, which is a human step. The repo names in the registry and on Hugging Face do not always match one to one (for example Mistral Large 3 against `Mistral-Large-3-675B-Base-2512`), so a job must map them.

## Providers without an official machine-readable source

| Provider | What is missing | Recommendation |
|---|---|---|
| **Meta** (Llama) | No hosted Llama API found and no Llama prices from Meta. Meta's own API lists Muse models, not Llama. The Llama `config.json` files are gated | Take names and dates from the listing of the Hugging Face org `meta-llama` (JSON, no key). Take context windows from the HTML model cards, or from `config.json` with a token once Thibault has accepted the licence. Record the price as "not published by Meta"; do not scrape Meta pages |
| **Microsoft** | For the two Phi models of the referential, names, dates and context windows are on Hugging Face (org `microsoft`); Microsoft publishes no Phi price outside Azure. For other Azure-sold models (MAI, Model Router) there is no public catalog-wide model list API and context windows are HTML only | Use the Hugging Face listing for Phi. For prices, use the Azure Retail Prices API once its coverage of Foundry models is checked. For an Azure-only model, read the Microsoft Learn pages by hand until Thibault confirms that a scheduled read is within the terms |

For every other provider the model list is machine-readable, but **prices are HTML only** at Anthropic, OpenAI, Google, Mistral and DeepSeek (Anthropic, OpenAI and Mistral offer Markdown copies of their docs pages). Only xAI and Alibaba return prices in their list API.

## Open points for Thibault

1. Every official list API found needs an API key (or Azure credentials for Microsoft). Only the Hugging Face listings need none, and they cover only the 14 open-weight presets (Meta Llama, Mistral AI, DeepSeek, Microsoft Phi); the 20 closed-weight presets (Anthropic, OpenAI, Google, xAI, Alibaba) need the provider's key. Creating accounts and keys, and accepting the Llama licence, are human-only matters. CLAUDE.md requires v1.0 to work without any key, so a key would belong to the scheduled catalog job, not to the app.
2. Read the OpenAI terms (HTTP 403 for every client we tried), the xAI API (enterprise) terms, Microsoft's Azure Product Terms, Meta's developer terms and the Llama licence and acceptable use policy: none of them was read.
3. Confirm, provider by provider, whether reading the public pricing and deprecation pages on a schedule is acceptable where there is no API for them (prices: Anthropic, OpenAI, Google, Mistral, DeepSeek; deprecations: Google, DeepSeek, xAI, Alibaba).

---

# Overview — quick matrix

| ID | Source | Tier | Blocking? | Target sprint |
|----|--------|------|-----------|---------------|
| S01 | **ComparIA** | 1 🎯 | **Yes** | **S2** |
| S02 | **RTE IRIS** | 1 🎯 | **Yes** | **S2** |
| S03 | ADEME Base Empreinte | 2 | Yes | S2 |
| S04 | EcoLogits | 2 | Yes (methodology) | S0 + S3 |
| S05 | HF AI Energy Score | 2 | Recommended | S3 |
| S06 | CodeCarbon | 2 | Recommended | S3 |
| S07 | ML.Energy | 2 | Recommended | S3 |
| S08 | Papers | 2 | Yes (validation) | S0 + S3 |
| S09 | RTE eco2mix | 3 | No (optional) | S3 |

**8 active sources instead of 10**, **0 blocking key** for v1.0, **2 official challenge datasets** at the core.

---

# Silver schemas (overview)

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
└── mix_hourly_fr-v1.json            (S09, optional)
```

---

# Prerequisites before ingestion (S2)

- [x] Source strategy frozen (this document)
- [ ] DVC remote storage configured (R2 or S3 — ComparIA weighs 5 GB)
- [ ] Silver v1 schemas frozen and committed
- [ ] `proptest` template tests ready
- [ ] (Optional) RTE eco2mix API key obtained (Thibault)

---

# Retention policy per source

| Source | Copper policy |
|--------|---------------|
| ComparIA | 1 snapshot per quarterly update (4/year), kept indefinitely (scientific lineage) |
| RTE IRIS | 1 snapshot per annual update, kept indefinitely |
| ADEME | Last 30 snapshots, then monthly for 2 years, then yearly |
| EcoLogits | All GitHub release snapshots (strict versioning) |
| Others | Last 30 snapshots, then monthly for 1 year |

---

*This catalog is living. Any new source = new sheet + new `DataLayer` implementation. If the challenge adds an official dataset, it goes straight to Tier 1.*
