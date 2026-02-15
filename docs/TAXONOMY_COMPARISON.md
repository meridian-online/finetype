# Taxonomy Comparison: FineType vs External Type Systems

FineType's taxonomy of 159 types classifies data by **format** — the character-level structure of values. This document compares it to three external type systems that classify data by **semantics** — what the data represents.

Understanding these differences helps users know what FineType can and can't detect, and where it fits in the broader data ecosystem.

## Philosophical Differences

| Dimension | FineType | schema.org | Wikidata Properties | GitTables Annotations |
|---|---|---|---|---|
| **Classifies by** | Format (character patterns) | Semantic meaning | Entity relationships | Semantic column role |
| **Question answered** | "How is this value structured?" | "What does this value represent in web content?" | "What property of an entity is this?" | "What semantic type best describes this column?" |
| **Scope** | Individual values or columns | Structured web data (JSON-LD, Microdata) | Knowledge graph triples | CSV table columns |
| **Granularity** | 159 leaf types in 6 domains | ~800 types + ~1,400 properties | ~10,000+ properties | 59 schema.org + 122 DBpedia labels |
| **Overlap with FineType** | N/A | ~15 direct format matches | ~10 direct format matches | ~12 format-detectable types |

### The Key Distinction

FineType answers: *"This string `192.168.1.1` has the format of an IPv4 address."*

schema.org answers: *"This field represents a contact point's IP address within a web service."*

Wikidata answers: *"This is property P2699 (URL) of entity Q42 (Douglas Adams)."*

GitTables answers: *"This column in a CSV table semantically represents IP addresses."*

**FineType is format-first.** A column of person names and a column of author names have the same format — FineType correctly identifies both as `identity.person.full_name`. The distinction between "author" and "name" is semantic context that requires understanding the table structure, not the value format.

## Comparison Tables

### FineType to schema.org

Types where FineType has a direct or close equivalent in schema.org:

| FineType Type | schema.org Equivalent | Match Quality | Notes |
|---|---|---|---|
| `identity.person.email` | `schema:email` | Direct | Both identify email format |
| `identity.person.full_name` | `schema:name` | Close | schema.org `name` is broader (includes org names) |
| `identity.person.gender` | `schema:gender` | Direct | FineType also has `gender_code` and `gender_symbol` |
| `geography.address.postal_code` | `schema:postalCode` | Direct | FineType detects format; schema.org describes semantics |
| `geography.location.country` | `schema:Country` | Close | FineType detects country name strings |
| `geography.location.state` | `schema:State` | Close | FineType detects US state / region strings |
| `geography.location.city` | `schema:City` | Close | FineType detects city name strings |
| `technology.internet.url` | `schema:URL` | Direct | Both identify URL format |
| `datetime.date.*` | `schema:dateIssued`, `schema:purchaseDate` | Partial | FineType distinguishes 12+ date formats; schema.org has one `Date` type |
| `datetime.timestamp.*` | `schema:startTime`, `schema:endTime`, `schema:Time` | Partial | FineType distinguishes ISO 8601, RFC 3339, Unix, etc. |
| `identity.payment.credit_card_number` | `schema:identifier` (broad) | Loose | schema.org has no specific credit card type |
| `representation.numeric.percentage` | `schema:value` (broad) | Loose | schema.org doesn't distinguish numeric formats |
| `representation.numeric.decimal_number` | `schema:price`, `schema:depth`, `schema:height`, `schema:width` | Semantic gap | FineType sees format (decimal); schema.org sees meaning (price, height) |
| `representation.text.plain_text` | `schema:Text`, `schema:description` | Loose | FineType detects text format; schema.org describes role |
| `technology.code.doi` | — | No match | schema.org has no DOI type |
| `technology.code.isbn` | — | No match | schema.org has no ISBN type (uses `identifier`) |

**schema.org types with no FineType equivalent:**

| schema.org Type | Why FineType Can't Detect It |
|---|---|
| `schema:author` | Semantic role — same format as any person name |
| `schema:category` | Semantic role — format varies (any string) |
| `schema:Product` | Entity type — no distinctive format |
| `schema:Event` | Entity type — no distinctive format |
| `schema:Rating` | Semantic role — looks like a number to FineType |
| `schema:Role` | Semantic role — format varies |
| `schema:status` | Semantic role — any string value |
| `schema:description` | Semantic role — FineType sees it as `plain_text` or `sentence` |
| `schema:comment` | Semantic role — FineType sees it as `plain_text` or `sentence` |
| `schema:Action` | Entity type — no distinctive format |
| `schema:manufacturer` | Semantic role — same format as any organization name |
| `schema:serialNumber` | Could be detectable — format varies by manufacturer |
| `schema:orderNumber` | Could be detectable — format varies by system |

### FineType to Wikidata Properties

Wikidata properties describe relationships between entities. Some have format constraints that align with FineType types:

| FineType Type | Wikidata Property | Property ID | Match Quality |
|---|---|---|---|
| `identity.person.email` | email address | P968 | Direct |
| `technology.internet.url` | official website | P856 | Direct |
| `geography.coordinate.latitude` | coordinate location (lat) | P625 | Direct |
| `geography.coordinate.longitude` | coordinate location (lon) | P625 | Direct |
| `technology.code.doi` | DOI | P356 | Direct |
| `technology.code.isbn` | ISBN-13 / ISBN-10 | P212 / P957 | Direct |
| `technology.code.issn` | ISSN | P236 | Direct |
| `identity.payment.isin` | ISIN | P946 | Direct |
| `identity.person.phone_number` | phone number | P1329 | Direct |
| `technology.internet.ip_v4` | IPv4 address | — | No Wikidata property |
| `technology.internet.ip_v6` | IPv6 address | — | No Wikidata property |
| `technology.internet.mac_address` | — | — | No Wikidata property |
| `technology.cryptographic.uuid` | — | — | No Wikidata property |
| `identity.person.gender` | sex or gender | P21 | Semantic gap — Wikidata uses entity links, not format |
| `geography.location.country` | country | P17 | Semantic gap — Wikidata uses entity IDs (Q30), not names |
| `datetime.component.year` | inception / date of birth | P571 / P569 | Partial — Wikidata uses full dates, not bare years |

**Key insight**: Wikidata's strength is in **identifiers with standardized formats** (DOI, ISBN, ISSN, ISIN) — these are exactly the types where FineType's format detection is most reliable. For semantic properties (gender, country), Wikidata uses entity links rather than string values, so the approaches are complementary.

### FineType to GitTables Column Annotations

GitTables provides column-level semantic annotations from both schema.org (59 types) and DBpedia (122 types). Since FineType is evaluated against GitTables, this mapping directly affects benchmark accuracy.

#### Format-Detectable Types (FineType performs well)

These GitTables types have distinctive formats that FineType can detect:

| GitTables Label | Source | FineType Mapping | Eval Accuracy |
|---|---|---|---|
| `url` | schema.org | `technology.internet.url` | **89.7%** |
| `created` | DBpedia | `datetime.timestamp.*` | **100%** |
| `date` | DBpedia | `datetime.date.*` | **88.2%** |
| `country` | schema.org | `geography.location.country` | **100%** |
| `state` | schema.org | `geography.location.*` | **90.0%** |
| `name` / `author` | Both | `identity.person.*` | **80-85%** |
| `gender` | Both | `identity.person.gender` | **100%** |
| `postal code` / `zip code` | Both | `geography.address.postal_code` | Variable |
| `email` | schema.org | `identity.person.email` | Limited data |
| `issn` | DBpedia | `technology.code.issn` | Limited data |
| `year` | DBpedia | `datetime.component.year` | **28.4%** (column-mode) |
| `percentage` | DBpedia | `representation.numeric.percentage` | Variable |

#### Semantic-Only Types (No format signal)

These GitTables types describe meaning, not format — FineType cannot detect them:

| GitTables Label | Source | Why Not Detectable | FineType Sees Instead |
|---|---|---|---|
| `description` / `abstract` | Both | Free text — no format pattern | `representation.text.sentence` |
| `comment` / `note` | Both | Free text | `representation.text.sentence` |
| `category` / `class` / `type` | Both | Arbitrary strings | `representation.text.word` |
| `rank` / `order` | Both | Numbers with semantic meaning | `representation.numeric.integer_number` |
| `species` / `genus` | DBpedia | Domain vocabulary | `representation.text.word` |
| `rating` / `score` | Both | Numbers with semantic meaning | `representation.numeric.decimal_number` |
| `price` / `cost` | Both | Numbers with currency context | `representation.numeric.decimal_number` |
| `height` / `weight` / `depth` / `width` | Both | Numbers with unit context | `representation.numeric.decimal_number` |
| `duration` | Both | Various formats (seconds, HH:MM:SS, text) | Mixed |
| `status` | Both | Arbitrary strings | `representation.text.word` |
| `role` / `title` | Both | Person/job names | `identity.person.*` or `representation.text.*` |
| `product` / `manufacturer` | Both | Entity names | `representation.text.*` |
| `project` / `series` | Both | Arbitrary labels | `representation.text.*` |
| `language` | Both | Language names/codes | `representation.text.word` |

#### Boundary Types (Partially detectable)

Types where format provides some signal but isn't definitive:

| GitTables Label | Source | FineType Potential | Challenge |
|---|---|---|---|
| `id` / `identifier` | Both | `representation.numeric.increment` (if sequential) | IDs can be any format |
| `serial number` | schema.org | Could detect with patterns | Varies by manufacturer |
| `file format` / `content type` | Both | `representation.file.mime_type` (if MIME) | Sometimes free text |
| `version` | Both | Could detect semver pattern | Many formats exist |
| `color` | schema.org | `representation.text.color_hex` (if hex) | Also RGB, named colors |
| `start date` / `end date` | Both | `datetime.date.*` or `datetime.timestamp.*` | Dates detectable, semantics not |
| `address` | DBpedia | Partial via components | Full addresses are complex |

## Types Unique to FineType

FineType covers many format-specific types that external systems don't distinguish:

| Domain | FineType Types | Why External Systems Don't Need These |
|---|---|---|
| **Network** | `ip_v4`, `ip_v6`, `mac_address`, `hostname`, `port`, `slug`, `user_agent` | schema.org/Wikidata model network concepts differently |
| **Cryptographic** | `uuid`, `hash`, `token_hex`, `jwt` | These are infrastructure formats, not semantic types |
| **DateTime formats** | 46 types: `iso_8601`, `rfc_3339`, `rfc_2822`, `us_slash`, `eu_slash`, `short_dmy`, etc. | External systems have one "Date" or "DateTime" type |
| **Container formats** | `json_object`, `json_array`, `csv_row`, `key_value_pair`, `query_string` | External systems describe content, not container format |
| **Scientific** | `dna_sequence`, `rna_sequence`, `protein_sequence` | Specialized formats not in general ontologies |
| **Financial identifiers** | `isin`, `cusip`, `sedol`, `swift_bic`, `lei` | Wikidata has some (ISIN, ISSN); schema.org doesn't |

**This is FineType's niche**: the 46 datetime formats, 35 technology formats, and 11 container formats have no equivalent in semantic type systems. They exist because FineType answers a different question — not "what does this mean?" but "how should this be parsed?"

## Gaps and Opportunities

### Types FineType Should Add (informed by external systems)

Based on real-world GitTables data and the Titanic profiling analysis:

| Proposed Type | Informed By | Priority | Backlog |
|---|---|---|---|
| `representation.categorical` | GitTables: category, class, type, status | High | NNFT-063 |
| `representation.ordinal` | GitTables: rank, rating, score | High | NNFT-063 |
| `representation.code.alphanumeric_id` | GitTables: id, serial number, code | High | NNFT-063 |
| `identity.person.age` (disambiguation) | DBpedia: age; Titanic analysis | Medium | NNFT-065 |
| Semver / version string | schema.org: version; DBpedia: version | Low | — |
| Language code (ISO 639) | schema.org: language; DBpedia: language | Low | — |

### Types That Will Always Be Semantic Gaps

These require understanding meaning, not format — they're out of scope for format detection:

- **Entity types**: product, event, school, company, manufacturer
- **Relational roles**: author, parent, producer, members, speaker
- **Domain concepts**: species, genus, formula, technique, training
- **Subjective labels**: comment, note, description, abstract

These gaps are a feature, not a bug. FineType's value proposition is reliable format detection that enables safe type casting — semantic understanding is a complementary capability served by LLMs, knowledge graphs, or column-level embedding models.

## Practical Implications

### When to use FineType

- **Data ingestion**: Detect column formats for safe `CAST()` / `TRY_CAST()` operations
- **Schema inference**: Assign DuckDB broad types to untyped CSV/JSON data
- **Data quality**: Validate that values match expected formats (email, URL, date)
- **ETL pipelines**: Route data to format-specific parsers

### When to use schema.org / Wikidata / knowledge graphs

- **Semantic search**: Find all "author" columns regardless of format
- **Data integration**: Match columns across datasets by meaning
- **Knowledge enrichment**: Link values to entity databases
- **Content understanding**: Classify what data *represents*, not how it's *formatted*

### When to combine both

The most powerful approach uses FineType for format detection and semantic systems for meaning. For example:

1. FineType detects a column as `identity.person.full_name` (format: person name strings)
2. Column name "author" maps to `schema:author` (semantic: this column represents authors)
3. Combined: "This column contains person names that represent authors"

This layered approach is the direction FineType's column-name heuristic (NNFT-067) begins to explore — using column metadata as a semantic signal alongside format detection.

## References

- [FineType Taxonomy](../labels/) — Full YAML definitions for all 159 types
- [GitTables Benchmark](https://zenodo.org/record/5706316) — 1M+ tables with schema.org and DBpedia annotations
- [schema.org Full Hierarchy](https://schema.org/docs/full.html) — Complete schema.org type system
- [Wikidata Property Constraints](https://www.wikidata.org/wiki/Help:Property_constraints_portal) — Property format expectations
- [GitTables Evaluation Report](../eval/gittables/REPORT.md) — FineType's benchmark results
