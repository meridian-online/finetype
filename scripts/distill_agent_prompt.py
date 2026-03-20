#!/usr/bin/env python3
"""Generate agent prompts for distillation v3 blind-first adjudication.

Usage:
    python3 scripts/distill_agent_prompt.py --batch-id sherlock_batch_0000 \
        --jsonl output/distillation-v3/sherlock_test.jsonl \
        --offset 0 --limit 100

Prints the full agent prompt to stdout. The agent:
1. Classifies each column blind (without seeing FineType's prediction)
2. Runs finetype infer --mode column --batch to get FineType's prediction
3. Adjudicates disagreements
4. Writes batch CSV + .done marker
"""

import json
import os
import sys

TAXONOMY_COMPACT = """container.array: comma_separated, pipe_separated, semicolon_separated, whitespace_separated
container.key_value: query_string, form_data
container.object: json, json_array, xml, html, yaml, csv
datetime.component: year, day_of_week, month_name, periodicity
datetime.date: iso, mdy_slash, dmy_slash, dmy_dot, short_dmy, short_mdy, short_ymd, compact_ymd, compact_dmy, compact_mdy, abbreviated_month, long_full_month, weekday_abbreviated_month, weekday_full_month, ordinal, julian, iso_week, ymd_slash, ymd_dot, dmy_dash, mdy_dash, dmy_space_abbrev, dmy_space_full, abbrev_month_no_comma, full_month_no_comma, dmy_dash_abbrev, dmy_dash_abbrev_short, mdy_short_slash, dmy_short_slash, dmy_short_dot, year_month, compact_ym, month_year_full, month_year_abbrev, month_year_slash, weekday_dmy_full, chinese_ymd, korean_ymd, jp_era_short, jp_era_long
datetime.duration: iso_8601
datetime.epoch: unix_seconds, unix_milliseconds, unix_microseconds
datetime.offset: iana, utc
datetime.period: quarter, fiscal_year
datetime.time: iso, hms_24h, hm_24h, hms_12h, hm_12h
datetime.timestamp: iso_8601, iso_8601_compact, iso_8601_microseconds, iso_8601_offset, rfc_2822, rfc_3339, sql_standard, mdy_12h, mdy_24h, dmy_hm, iso_microseconds, rfc_2822_ordinal, sql_microseconds, sql_milliseconds, iso_8601_milliseconds, iso_8601_millis_offset, iso_8601_micros_offset, clf, syslog_bsd, sql_microseconds_offset, pg_short_offset, dot_dmy_24h, slash_ymd_24h, ctime, epoch_nanoseconds, iso_space_zulu, dot_ymd_24h
finance.banking: swift_bic, iban, aba_routing, bsb
finance.crypto: bitcoin_address, ethereum_address
finance.currency: amount, amount_comma, currency_code, currency_symbol, amount_accounting, amount_comma_suffix, amount_space, amount_lakh, amount_apostrophe, amount_nodecimal, amount_code_prefix, amount_minor_int, amount_crypto, amount_multisym, amount_neg_trailing
finance.payment: credit_card_number, credit_card_expiration_date, paypal_email
finance.rate: basis_points, yield
finance.securities: cusip, isin, sedol, figi, lei
geography.address: full_address, street_name, street_suffix, postal_code
geography.contact: calling_code
geography.coordinate: latitude, longitude, coordinates, geohash, plus_code, dms, mgrs
geography.format: wkt, geojson
geography.index: h3
geography.location: country, country_code, continent, region, city
geography.transportation: iata_code, icao_code, iso6346, hs_code, unlocode
identity.academic: orcid
identity.commerce: ean, isbn, issn, upc, isrc
identity.government: vin, eu_vat, ssn, ein, pan_india, abn
identity.medical: npi, dea_number, ndc, icd10, loinc, cpt, hcpcs
identity.person: full_name, first_name, last_name, email, phone_number, email_display, phone_e164, username, password, gender, gender_code, gender_symbol, blood_type, height, weight
representation.boolean: binary, initials, terms
representation.discrete: categorical, ordinal
representation.file: extension, mime_type, file_size, excel_format
representation.format: color_hex, color_rgb, color_hsl
representation.identifier: uuid, alphanumeric_id, increment, numeric_code
representation.numeric: integer_number, decimal_number, decimal_number_comma, scientific_notation, percentage, si_number
representation.scientific: dna_sequence, rna_sequence, protein_sequence, measurement_unit, cas_number, inchi, smiles, metric_prefix
representation.text: plain_text, sentence, word, emoji, entity_name, paragraph
technology.cloud: aws_arn, s3_uri
technology.code: imei, doi, locale_code
technology.cryptographic: hash, token_hex, token_urlsafe, jwt
technology.development: version, calver, docker_ref, git_sha
technology.identifier: ulid, tsid, snowflake_id
technology.internet: ip_v4, ip_v4_with_port, ip_v6, mac_address, url, hostname, top_level_domain, user_agent, http_method, cidr, urn, data_uri"""


def build_prompt(batch_id, jsonl_path, offset, limit, dest_dir="output/distillation-v3"):
    """Build the full agent prompt for a distillation batch."""

    # Read the batch records
    records = []
    with open(jsonl_path) as f:
        for line_num, line in enumerate(f):
            if line_num < offset:
                continue
            if len(records) >= limit:
                break
            try:
                records.append(json.loads(line))
            except json.JSONDecodeError:
                continue

    # Build column descriptions for blind classification
    columns_text = ""
    for i, rec in enumerate(records):
        header = rec.get("column_name") or "(no header)"
        values = rec.get("values", [])
        values_preview = ", ".join(f'"{v}"' for v in values[:10])
        if len(values) > 10:
            values_preview += f" ... ({len(values)} total)"
        columns_text += f"\nColumn {i}: header={header}\n  values: [{values_preview}]\n"

    # Build finetype JSONL input
    finetype_input = ""
    for rec in records:
        header = rec.get("column_name") or ""
        values = rec.get("values", [])
        finetype_input += json.dumps({"header": header, "values": values}) + "\n"

    # Build metadata JSON (outside f-string to avoid brace escaping issues)
    metadata_list = []
    for i, r in enumerate(records):
        metadata_list.append({
            "index": i,
            "source": r.get("source", ""),
            "source_file": r.get("source_file", ""),
            "column_name": r.get("column_name", ""),
            "ground_truth_label": r.get("ground_truth_label", "") or "",
            "ground_truth_source": r.get("ground_truth_source", "") or "",
        })
    metadata_json = json.dumps(metadata_list, indent=None)

    output_csv = os.path.join(dest_dir, f"{batch_id}.csv")
    done_marker = os.path.join(dest_dir, f"{batch_id}.done")

    prompt = f"""You are a data type classification agent for FineType distillation v3.
Your task: classify {len(records)} columns using blind-first adjudication protocol.

## Protocol

For each column:
1. **PASS 1 (Blind):** Classify the column from header + sample values ONLY. Pick the best label from the taxonomy below. Rate your confidence (high/medium/low).
2. **PASS 2 (Adjudicate):** After ALL blind classifications, run FineType to get its predictions. Compare. For disagreements, write brief reasoning and pick a final_label.

## Taxonomy (250 types)

Labels are formatted as domain.category.type. You MUST pick from this list:

{TAXONOMY_COMPACT}

Full label format: domain.category.type (e.g., geography.location.country, datetime.date.iso)

## Columns to classify

{columns_text}

## Instructions

### Step 1: Blind classification

For each column, output your blind classification. Think about:
- What data type do the values represent?
- Does the header (if present) help disambiguate?
- Pick the MOST SPECIFIC type that fits the majority of values
- If values are clearly text with no structured format, use representation.text.* types
- If values look like names, consider identity.person.* or representation.text.entity_name

### Step 2: Run FineType

Run this command from the repo root (/home/hugh/github/meridian-online/finetype):

```bash
cat <<'FINETYPE_INPUT' | finetype infer --mode column --batch -o json
{finetype_input}FINETYPE_INPUT
```

Parse each line of output to get finetype's label and confidence.

### Step 3: Write output CSV

Write the output CSV to: {output_csv}

The CSV must have these columns (in this order):
source, source_file, column_name, sample_values, blind_label, blind_confidence, finetype_label, finetype_confidence, agreement, final_label, reasoning, ground_truth_label, ground_truth_source

For each column:
- source, source_file, column_name: from the input data below
- sample_values: JSON array of the sample values
- blind_label: your Pass 1 classification
- blind_confidence: high | medium | low
- finetype_label: FineType's prediction from Step 2
- finetype_confidence: FineType's confidence score
- agreement: "yes" if blind_label == finetype_label, "no" otherwise
- final_label: if agree → blind_label; if disagree → your reasoned choice
- reasoning: if disagree → brief explanation of which is correct and why (1-2 sentences); if agree → empty
- ground_truth_label: from input data (may be empty)
- ground_truth_source: from input data (may be empty)

### Step 4: Write .done marker

After validating the CSV has the correct number of rows ({len(records)}), write the done marker:

```bash
echo "rows={len(records)}" > {done_marker}
```

## Input record metadata

Use this to fill source, source_file, column_name, ground_truth_label, ground_truth_source:

```json
{metadata_json}
```

## Critical rules

- You MUST classify ALL {len(records)} columns — do not skip any
- Labels MUST be valid taxonomy keys (domain.category.type format)
- Do NOT look at ground_truth_label during blind classification — it's for offline comparison only
- Write the CSV using Python's csv module to handle quoting correctly
- The output CSV should have exactly {len(records)} data rows + 1 header row
"""
    return prompt


def main():
    args = sys.argv[1:]
    batch_id = None
    jsonl_path = None
    offset = 0
    limit = 100
    dest_dir = "output/distillation-v3"

    i = 0
    while i < len(args):
        if args[i] == "--batch-id":
            batch_id = args[i + 1]
            i += 2
        elif args[i] == "--jsonl":
            jsonl_path = args[i + 1]
            i += 2
        elif args[i] == "--offset":
            offset = int(args[i + 1])
            i += 2
        elif args[i] == "--limit":
            limit = int(args[i + 1])
            i += 2
        elif args[i] == "--dest":
            dest_dir = args[i + 1]
            i += 2
        elif args[i] in ("-h", "--help"):
            print(__doc__)
            sys.exit(0)
        else:
            print(f"Unknown argument: {args[i]}", file=sys.stderr)
            sys.exit(1)

    if not batch_id or not jsonl_path:
        print("ERROR: --batch-id and --jsonl required", file=sys.stderr)
        sys.exit(1)

    print(build_prompt(batch_id, jsonl_path, offset, limit, dest_dir))


if __name__ == "__main__":
    main()
