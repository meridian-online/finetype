# Survey: Annotated Datasets for Column Type Annotation

**Date:** 2026-03-19
**Author:** Nightingale
**Purpose:** Identify datasets beyond GitTables and SOTAB that could strengthen FineType's evaluation and training data. Conducted as part of distillation v3 planning after incident recovery required re-downloading all datasets.

---

## What We Already Use

```
| Dataset       | Tables  | Annotated Cols | Types     | Schema          | Size on Disk | Source             |
|---------------|---------|----------------|-----------|-----------------|--------------|--------------------|
| GitTables 1M  | ~1M     | ~738K (S.o)    | 2000+     | Schema.org      | 15.1 GB      | Zenodo #6517052    |
|               |         | ~723K (DBp)    |           | + DBpedia       |              |                    |
| SOTAB V2      | ~46K    | ~130K train    | 82-91     | Schema.org      | 4.0 GB       | Zenodo #8422037    |
|               |         | ~15K test      |           |                 |              |                    |
```

**GitTables** provides scale — 1M tables from GitHub CSVs with automated annotations. Phase 2 distillation processed 507 of these (0.13%). The corpus is dominated by HN discussion dumps and software metrics, which triggers the same FineType weaknesses repeatedly.

**SOTAB** provides quality — web tables annotated with Schema.org types from the Web Data Commons corpus. We have the CTA validation split (5,732 JSON table files) but haven't run it through distillation yet.

Both are on Zenodo with stable download URLs. The `scripts/download_datasets.py` script handles both.

---

## Tier 1 — High Value, Should Acquire

### Sherlock / VizNet

The foundational dataset for semantic type detection. Every CTA paper benchmarks against it.

```
| Attribute        | Value                                                    |
|------------------|----------------------------------------------------------|
| Columns          | 686,765                                                  |
| Types            | 78 (DBpedia-derived)                                     |
| Source corpus     | VizNet — 31M tables from open data repositories          |
| Annotation method | Automated header matching to DBpedia types               |
| Download         | github.com/mitmedialab/sherlock-project                  |
| Size             | ~500 MB (features + labels)                              |
| Format           | Parquet feature files + CSV labels                       |
| License          | MIT                                                      |
| Papers           | Sherlock (KDD 2019), Sato (VLDB 2020)                    |
```

**78 types include:** address, age, album, area, artist, birth_date, birth_place, city, classification, club, code, command, company, component, continent, country, county, creator, credit, currency, day, depth, description, director, duration, education, election, elevation, email, family_name, file_size, format, gender, genre, grades, industry, isbn, jockey, language, location, manufacturer, name, nationality, notes, operator, order, organisation, origin, owner, person, phone_number, plays, position, product, publisher, range, rank, ranking, region, religion, requirement, result, sales, service, sex, species, state, status, symbol, team, team_name, type, weight, year.

**Relevance to FineType:** Strong overlap. Many of these 78 types map directly to FineType taxonomy entries (email, city, country, currency, url, phone_number, etc.). The automated annotation means some noise, but 687K columns at this scale compensates. Essential for comparability claims in any future paper.

**Sato** extends Sherlock with table-context features (topic model over neighbouring columns). Same underlying data, different model architecture. The data files from either repo work.

**Integration effort:** Low. Download feature files, map 78 Sherlock types to FineType's 250-type taxonomy (many are 1:1). Run FineType profile on the raw VizNet columns and compare.

---

### SemTab 2024 WikidataTables

The most recent annual Semantic Web challenge dataset for table annotation.

```
| Attribute        | Value                                                    |
|------------------|----------------------------------------------------------|
| Tables           | ~109K (Round 1: 30K, Round 2: 79K)                      |
| Annotated cols   | Tens of thousands (CTA task)                             |
| Types            | Wikidata classes (hierarchical)                          |
| Download         | Zenodo #14207232                                         |
| Size             | ~few GB                                                  |
| Format           | CSV tables + ground truth CSVs                           |
| License          | CC BY 4.0                                                |
| Papers           | SemTab 2024 proceedings                                  |
```

**CTA task:** Annotate columns with Wikidata class IDs (e.g., Q515 for "city"). Uses hierarchical scoring (`cscore`) that rewards near-misses in the type hierarchy — a city labelled as "populated place" scores higher than one labelled as "vehicle".

**Relevance to FineType:** Wikidata's type hierarchy is well-maintained and deep. Building a Wikidata→FineType mapping would let us evaluate on all SemTab rounds (2019-2024). The hierarchical scoring model is also interesting — FineType's domain/category/type hierarchy could support similar partial-credit evaluation.

**Integration effort:** Medium. Need to build a Wikidata QID → FineType key mapping. Many Wikidata classes won't map (too domain-specific), but the common ones (person, place, date, organization, quantity) should map cleanly.

---

### TURL WikiTables

The largest human-curated table corpus with fine-grained type labels.

```
| Attribute        | Value                                                    |
|------------------|----------------------------------------------------------|
| Tables           | 570K+ from Wikipedia                                     |
| Annotated cols   | ~580K                                                    |
| Types            | 255 (Freebase types)                                     |
| Download         | github.com/sunlab-osu/TURL                               |
| Size             | ~2-5 GB                                                  |
| Format           | JSON                                                     |
| License          | Research use                                              |
| Papers           | TURL (VLDB 2021)                                         |
```

**255 Freebase types** — the closest match to FineType's 250-type taxonomy in any public dataset. Types include fine-grained distinctions like `/location/country`, `/people/person`, `/music/album`, `/time/event`.

**Warning: download broken.** As of late 2025, the original OneDrive download link is dead — the author reportedly lost access. Potential recovery options:
- Fork: `github.com/sefeoglu/TURL-dataset-re` — may have mirrored data
- HuggingFace: search for community re-uploads
- Contact author: Xiang Deng (Ohio State)

**Relevance to FineType:** Would be the single most valuable benchmark if obtainable. 255 types at 580K columns, Wikipedia-sourced (diverse domains), KG-linked annotations.

**Integration effort:** Medium-high if obtainable. Freebase types need mapping to FineType taxonomy. The JSON format needs extraction to columnar data.

---

### SportsTables

Uniquely valuable for numerical column disambiguation.

```
| Attribute        | Value                                                    |
|------------------|----------------------------------------------------------|
| Tables           | Thousands (baseball, basketball, football, hockey, soccer)|
| Annotated cols   | Thousands                                                |
| Types            | Custom domain ontology                                   |
| Download         | github.com/DHBWMosbachWI/pythagoras                      |
| Size             | Small                                                    |
| Format           | CSV + annotations                                        |
| License          | Open source                                              |
| Papers           | Hulsebos et al. (Datenbank-Spektrum 2023)                |
```

**86% of columns are numerical.** Most CTA benchmarks are text-heavy — entity names, descriptions, categories. SportsTables inverts this, with columns like "points scored", "games played", "batting average", "salary", "age", "weight", "height", "year".

**Relevance to FineType:** Directly targets our weakest area. The Phase 2 distillation top disagreements are dominated by numeric confusion:
- `integer_number` vs `boolean.binary` (175 disagreements)
- `basis_points` header hint misfires (114)
- `amount_minor_int` false positives (77 + 27 + 27)
- `decimal_number` vs `yield` (66)
- `increment` vs float-stored integers (83)

A dataset where numeric disambiguation is the primary challenge would be the best evaluation corpus for PR-2's fixes.

**Integration effort:** Low. CSV format, custom ontology needs mapping to FineType types, but the mapping is straightforward for numeric types (score→integer_number, percentage→percentage, salary→amount, year→year, etc.).

---

## Tier 2 — Useful but Narrower

### T2Dv2

```
| Attribute        | Value                                                    |
|------------------|----------------------------------------------------------|
| Tables           | 779                                                      |
| Annotated cols   | ~237 (CTA)                                               |
| Types            | DBpedia types (varies)                                   |
| Download         | webdatacommons.org/webtables/goldstandardV2.html         |
| Size             | Small                                                    |
| License          | Open                                                     |
| Papers           | Ritze et al. (2015)                                      |
```

The classic gold standard from Web Data Commons. Too small for training (237 CTA annotations) but widely cited in papers. Useful as a held-out eval set for comparability claims. Every CTA paper reports T2Dv2 numbers.

### tFood / tFoodL

```
| Attribute        | Value                                                    |
|------------------|----------------------------------------------------------|
| Tables           | ~43K (tFoodL)                                            |
| Annotated cols   | Thousands                                                |
| Types            | Wikidata classes                                         |
| Download         | Zenodo #7828163 (tFood) / #10277790 (tFoodL)            |
| Size             | ~few GB                                                  |
| License          | CC BY 4.0                                                |
| Papers           | SemTab 2023 challenge                                    |
```

Domain-specific food tables. Part of SemTab 2023. Useful for testing FineType on specialized vocabularies (ingredient names, nutritional values, cooking measurements). Wikidata-annotated.

### tBiomedL

```
| Attribute        | Value                                                    |
|------------------|----------------------------------------------------------|
| Tables           | Unknown (likely thousands)                               |
| Annotated cols   | Thousands                                                |
| Types            | Wikidata classes                                         |
| Download         | Zenodo #10283119                                         |
| Size             | Unknown                                                  |
| License          | CC BY 4.0                                                |
| Papers           | SemTab 2023 challenge                                    |
```

Biomedical domain tables. Interesting for testing FineType on scientific data (dosages, lab values, gene identifiers, clinical codes). Wikidata-annotated.

### WikiDBs

```
| Attribute        | Value                                                    |
|------------------|----------------------------------------------------------|
| Databases        | 100K relational databases                                |
| Structure        | Multi-table with foreign keys                            |
| Types            | Wikidata properties (implicit)                           |
| Download         | Zenodo #11559814                                         |
| Size             | 165 GB                                                   |
| License          | Open                                                     |
```

Unique in providing relational context (multi-table with foreign keys). At 165 GB it's very large and requires significant preprocessing to extract column-level CTA annotations. Column types are implicit from Wikidata properties rather than explicit annotations.

### Limaye

```
| Attribute        | Value                                                    |
|------------------|----------------------------------------------------------|
| Tables           | 400                                                      |
| Annotated cols   | 84-428                                                   |
| Types            | YAGO types                                               |
| Download         | Zenodo #3087000                                          |
| Size             | Small                                                    |
| License          | Open                                                     |
```

Early web table annotation dataset. Very small, YAGO-typed. Historical interest only — superseded by T2Dv2 and SOTAB.

---

## Tier 3 — Interesting but Limited

### SALT-KG (SAP)

```
| Attribute        | Value                                                    |
|------------------|----------------------------------------------------------|
| Fields           | ~990                                                     |
| Types            | 1,954 (custom business knowledge graph)                  |
| Download         | github.com/SAP-samples/salt-kg                           |
| Size             | Small                                                    |
| License          | Apache 2.0                                               |
```

Enterprise-focused. 1,954 semantic object types from SAP business data. The custom KG schema won't map well to FineType's taxonomy, and the field count is too small. Interesting only for enterprise use-case positioning.

### GitTables Benchmark (small curated subset)

```
| Attribute        | Value                                                    |
|------------------|----------------------------------------------------------|
| Download         | Zenodo #5706316                                          |
| Size             | Small                                                    |
```

A curated subset of GitTables used in the original paper's evaluation. Redundant if we already have the full GitTables 1M.

---

## Comparison Matrix

```
| Dimension              | GitTables  | SOTAB V2  | Sherlock   | TURL       | SemTab '24 | SportsTbls |
|------------------------|------------|-----------|------------|------------|------------|------------|
| Scale (columns)        | ~730K      | ~145K     | ~687K      | ~580K      | ~100K+     | thousands  |
| Type granularity       | 2000+      | 82-91     | 78         | 255        | varies     | custom     |
| Source domain           | GitHub CSV | Web tables| Open data  | Wikipedia  | Wikidata   | Sports     |
| Annotation method      | Automated  | Auto+man  | Automated  | KG-linked  | KG-linked  | Manual     |
| Annotation quality     | Noisy      | High      | Moderate   | High       | High       | High       |
| Numeric col coverage   | Mixed      | Low       | Low        | Low        | Low        | Very high  |
| Download reliability   | Zenodo ✅   | Zenodo ✅  | GitHub ✅   | Broken ⚠️   | Zenodo ✅   | GitHub ✅   |
| Integration effort     | Done       | Medium    | Low        | Med-High   | Medium     | Low        |
| FineType type overlap  | Partial    | Moderate  | High       | High       | Requires   | Numeric    |
|                        |            |           | (78→250)   | (255→250)  | mapping    | subset     |
```

---

## Recommendations

### Add to download script now

1. **Sherlock/VizNet** (~500 MB, GitHub) — the reference benchmark, essential for comparability
2. **SportsTables** (small, GitHub) — directly targets our numeric disambiguation weakness

### Add for Phase 3

3. **SemTab 2024** (~few GB, Zenodo) — freshest competition data, requires Wikidata mapping
4. **T2Dv2** (small, WDC website) — classic gold standard, held-out eval
5. **tFood/tFoodL** (~few GB, Zenodo) — domain diversity

### Investigate but don't block on

6. **TURL WikiTables** — the dream dataset (255 types, 580K columns) but download is broken. Check mirrors.

### Skip

- WikiDBs (165 GB, too large, needs heavy preprocessing)
- SALT-KG (too small, enterprise-specific schema)
- Limaye (superseded by T2Dv2)

---

## Key Papers

For reference when writing up evaluation methodology:

- **Sherlock:** Hulsebos et al., "Sherlock: A Deep Learning Approach to Semantic Data Type Detection" (KDD 2019)
- **Sato:** Zhang et al., "Sato: Contextual Semantic Type Detection in Tables" (VLDB 2020)
- **Doduo:** Suhara et al., "Annotating Columns with Pre-trained Language Models" (SIGMOD 2022)
- **TURL:** Deng et al., "TURL: Table Understanding through Representation Learning" (VLDB 2021)
- **ArcheType:** Freire et al., "ArcheType: A Novel Framework for Open-Source Column Type Annotation using Large Language Models" (VLDB 2024)
- **SportsTables:** Hulsebos et al., "SportsTables: A New Corpus for Semantic Type Detection" (Datenbank-Spektrum 2023)
- **CTA Evaluation Critique:** "Evaluating CTA Models and Benchmarks" (WWW 2025) — critical analysis of dataset quality issues across benchmarks

---

## Disk Budget

```
| Dataset              | Size     | Cumulative |
|----------------------|----------|------------|
| GitTables 1M         | 15.1 GB  | 15.1 GB    |
| SOTAB V2             | 4.0 GB   | 19.1 GB    |
| Sherlock/VizNet      | ~0.5 GB  | 19.6 GB    |
| SportsTables         | ~0.1 GB  | 19.7 GB    |
| SemTab 2024          | ~3 GB    | 22.7 GB    |
| T2Dv2                | ~0.1 GB  | 22.8 GB    |
| tFood/tFoodL         | ~3 GB    | 25.8 GB    |
| Available disk       | 850 GB   |            |
```

All Tier 1 + Tier 2 datasets fit comfortably within available disk.
