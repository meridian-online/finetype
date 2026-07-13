# url override tier — recover URLs from confident mislabels (2026-07-14)

**Headline:** ~290 columns of real web URLs that FineType was confidently calling the wrong thing —
an IPv6 address, an XML blob, an AWS ARN, a Bitcoin wallet, a JWT — now type as `url`. Zero genuine
non-URLs were touched, because the recovery is gated by a validator that a bare host, ARN, or wallet
cannot satisfy.

## What changed

`structured_string_refinement`'s existing url reader re-asserts `technology.internet.url` when ≥90%
of a column's values pass the url taxonomy validator (`scheme://dotted-host`). It only fired on the
RESIDUAL labels (plain_text/word/unknown), so it could not reach a URL column the sibling-aware model
labels with a CONFIDENT wrong type. The reservoir-mining sweep (2026-07-14, `output/reservoir-mining/`)
measured exactly where real URLs strand. This widens the url reader's fire-on to the OVERRIDE tier:

    ip_v6 · container.object.xml · aws_arn · bitcoin_address · jwt · entity_name ·
    full_address · alphanumeric_id · eu_vat   (+ the original residual labels)

A genuine member of any of those lacks a `scheme://dotted-host`, so the url validator (the real guard)
rejects it — the widened fire-on only lets the validator *adjudicate*. Deliberately NOT "any label":
url-subtype leaves where url would be a demotion (`doi`, `urn`) are excluded. RHH-toggle
`url_override_tier`; one-line semantic change (`mod.rs`, structured_string_refinement).

## Gates (all pass)

| Instrument | Result |
|---|---|
| Corpus-honest fast gate (blocking) | **GO** — zero triggers, zero bands |
| Gold (reframe) | **882/1037 flat**, 0 changed rows |
| Representative (advisory) | **195/260 flat**, 0 changed rows |
| Actual promotions (cand vs base, 33k sample) | **146** — xml 42, bitcoin_address 27, entity_name 19, ip_v6 17, alphanumeric_id 14, aws_arn 11, unknown 11, jwt 5; **0 url lost** |
| Mandatory spot-check | **0 promotions with <90% URL values** — every promoted column is genuinely `http(s)://`/`ftp://` URLs |

The 53 promoted columns that are DOI/ontology-IRI URLs (`https://doi.org/…`, `http://purl.obolibrary.org/obo/…`)
were sensed as xml/bitcoin_address — definitively wrong — so `url` is strictly better, and `doi` isn't
in the fire-on set so no correct DOI was demoted. A future `doi`-URL recovery (`https://doi.org/10.x` →
doi) is a separate narrower enhancement, not a blocker.

Est. corpus reach ~290 override-tier columns. NO retrain (0096). Gold-invisible (gold has ~0 URL-as-
override rows) → gated on corpus-honest + spot-check, per the residual-slice scope.

Substrate: this file; `output/url-override/{gate,eval}/`; roadmap `output/reservoir-mining/roadmap.md`.
