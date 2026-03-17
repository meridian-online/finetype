#!/usr/bin/env python3
"""Evaluate synonym expansion impact on Model2Vec similarity scores.

Adds 5-10 additional synonyms per type for the top-20 most affected types,
re-computes type embeddings, and measures the impact on matching.

Usage:
    python3 specs/model2vec-specialisation/evaluate_synonym_expansion.py
"""

import csv
import json
import sys
from pathlib import Path

import numpy as np
from model2vec import StaticModel
from safetensors.numpy import load_file
from tokenizers import Tokenizer

WORKSPACE = Path(__file__).parent.parent.parent
MODELS_DIR = WORKSPACE / "models" / "model2vec"
EVAL_DIR = WORKSPACE / "eval" / "eval_output"
LABELS_DIR = WORKSPACE / "labels"

THRESHOLD = 0.70


# ═══════════════════════════════════════════════════════════════════════════════
# Expanded synonym lists for the top-20 most affected types
# ═══════════════════════════════════════════════════════════════════════════════

# These come from real-world column naming conventions observed in:
# - Kaggle datasets, GitTables, SOTAB
# - Common database naming patterns (snake_case, camelCase, abbreviations)
# - Domain-specific terminology

EXPANDED_SYNONYMS = {
    "representation.numeric.decimal_number": [
        # Current: title + label components + header hints (price, cost, amount, salary, fare, fee, toll, charge)
        # Additional synonyms from real-world column names:
        "wages", "compensation", "remuneration", "income", "revenue",
        "profit", "margin", "balance", "total", "subtotal",
        "tax", "discount", "rate", "value", "measurement",
        "reading", "score", "metric", "weight", "length",
        "width", "depth", "size", "area", "volume",
        "avg", "mean", "sum", "min", "max",
        "amt", "val", "num", "pct", "decimal",
        "float", "double", "real", "numeric",
    ],
    "representation.numeric.integer_number": [
        "count", "total count", "row count", "record count",
        "qty", "quantity", "num", "number of", "n",
        "frequency", "occurrences", "hits", "visits",
        "likes", "shares", "views", "downloads", "followers",
        "votes", "points", "attempts", "retries",
        "age", "year", "month", "day", "hour",
        "integer", "int", "bigint", "smallint",
    ],
    "representation.numeric.percentage": [
        "percent", "pct", "ratio", "rate",
        "yield", "return", "growth", "change",
        "accuracy", "precision", "recall", "f1",
        "completion", "progress", "coverage",
        "utilization", "efficiency", "margin",
        "share", "proportion", "fraction",
    ],
    "representation.discrete.categorical": [
        "category", "type", "class", "group",
        "label", "tag", "kind", "sort",
        "status", "state", "phase", "stage",
        "level", "tier", "grade",
        "sector", "segment", "division", "department",
        "sport", "genre", "method", "source",
        "embarked", "boarded", "terminal", "gate",
    ],
    "representation.discrete.ordinal": [
        "rank", "ranking", "position", "order",
        "priority", "severity", "importance",
        "rating", "score", "grade", "level",
        "class", "tier", "quality",
        "satisfaction", "stars",
    ],
    "representation.boolean.binary": [
        "boolean", "bool", "flag", "indicator",
        "is active", "is enabled", "is deleted", "is verified",
        "survived", "alive", "deceased", "confirmed",
        "approved", "rejected", "accepted",
        "true false", "yes no", "on off",
        "binary", "bit",
    ],
    "representation.boolean.terms": [
        "is gift", "is broadcast", "is admitted",
        "has discount", "has warranty", "has coupon",
        "enabled", "disabled", "active", "inactive",
        "available", "unavailable",
    ],
    "datetime.timestamp.iso_8601": [
        "timestamp", "datetime", "date time", "created at",
        "updated at", "modified at", "deleted at",
        "created date", "update date", "modified date",
        "event date", "order date", "purchase date",
        "start date", "end date", "due date",
        "log timestamp", "record timestamp",
        "ts", "dt", "dttm",
    ],
    "datetime.date.iso": [
        "date", "date of birth", "birth date", "dob",
        "hire date", "join date", "release date",
        "visit date", "appointment date",
        "effective date", "expiry date", "expiration date",
    ],
    "datetime.offset.iana": [
        "timezone", "time zone", "tz", "iana timezone",
        "tz name", "timezone name", "time zone name",
        "olson timezone", "zone",
    ],
    "datetime.epoch.unix_seconds": [
        "unix timestamp", "epoch", "unix epoch",
        "unix time", "epoch time", "posix time",
        "seconds since epoch", "unix seconds",
        "created unix", "updated unix",
    ],
    "geography.location.country_code": [
        "country code", "iso country", "iso alpha 2",
        "iso alpha 3", "alpha 2", "alpha 3",
        "country iso", "cc", "country abbreviation",
        "nation code", "iso 3166",
    ],
    "geography.address.postal_code": [
        "postal code", "zip code", "zip", "postcode",
        "zipcode", "plz", "postal",
        "shipping postal code", "billing postal code",
        "mailing zip", "delivery postcode",
    ],
    "technology.internet.url": [
        "url", "uri", "link", "href",
        "website", "homepage", "web address",
        "request url", "tracking url", "callback url",
        "redirect url", "source url", "target url",
        "endpoint", "api url",
    ],
    "technology.internet.http_status_code": [
        "status code", "http status", "http code",
        "response code", "response status",
        "http status code", "status number",
    ],
    "representation.file.mime_type": [
        "mime type", "content type", "media type",
        "file type", "file format", "format",
        "content format", "encoding",
    ],
    "representation.code.alphanumeric_id": [
        "code", "id", "identifier", "key",
        "ticket", "cabin", "seat", "room",
        "reference", "ref", "order number",
        "tracking number", "serial number",
        "sku", "ean", "upc",
    ],
    "technology.code.locale_code": [
        "locale", "language code", "lang code",
        "language tag", "bcp47", "ietf tag",
        "locale code", "locale id",
    ],
    "identity.person.full_name": [
        "name", "full name", "fullname",
        "person name", "author", "writer",
        "creator", "contributor", "editor",
        "contact name", "display name",
        "player name", "user name",
    ],
}


def load_taxonomy():
    """Load all taxonomy definitions from YAML files."""
    import yaml
    taxonomy = {}
    for yaml_file in sorted(LABELS_DIR.glob("definitions_*.yaml")):
        with open(yaml_file) as f:
            data = yaml.safe_load(f)
            if data:
                taxonomy.update(data)
    return taxonomy


def load_header_hints():
    """Load header hints from prepare_model2vec.py."""
    # Import from the script
    sys.path.insert(0, str(WORKSPACE / "scripts"))
    from prepare_model2vec import build_header_hint_entries, build_synonym_texts
    return build_header_hint_entries(), build_synonym_texts


def embed_column_name(name, tokenizer, embeddings):
    """Embed a column name (same logic as semantic.rs)."""
    normalized = name.lower().replace("_", " ").replace("-", " ").replace(".", " ").strip()
    if not normalized:
        return None
    encoding = tokenizer.encode(normalized, add_special_tokens=False)
    ids = [i for i in encoding.ids if i != 0]
    if not ids:
        return None
    token_embeds = embeddings[ids]
    mean_embed = np.mean(token_embeds, axis=0)
    norm = np.linalg.norm(mean_embed)
    if norm < 1e-8:
        return None
    return mean_embed / norm


def compute_type_embeddings(model, synonyms):
    """Compute type embeddings from synonym texts."""
    label_index = sorted(synonyms.keys())
    embeddings = []
    for label in label_index:
        texts = synonyms[label]
        if not texts:
            texts = [label.replace(".", " ")]
        vecs = model.encode(texts)
        mean_vec = np.mean(vecs, axis=0)
        norm = np.linalg.norm(mean_vec)
        if norm > 0:
            mean_vec = mean_vec / norm
        embeddings.append(mean_vec)
    return np.stack(embeddings).astype(np.float32), label_index


def score_column(name, gt_label, type_embeddings, label_index, tokenizer, token_embeddings, gt_to_finetype):
    """Score a single column name against type embeddings."""
    vec = embed_column_name(name, tokenizer, token_embeddings)
    if vec is None:
        return None, 0.0, False

    sims = type_embeddings @ vec
    best_idx = np.argmax(sims)
    best_label = label_index[best_idx]
    best_sim = float(sims[best_idx])

    acceptable = gt_to_finetype.get(gt_label, [])
    correct = best_label in acceptable
    if not correct and acceptable:
        pred_domain = best_label.split(".")[0]
        for acc in acceptable:
            if acc.split(".")[0] == pred_domain:
                correct = True
                break

    return best_label, best_sim, correct


# Reuse GT mapping from analyse_similarity.py
GT_TO_FINETYPE = {
    "id": ["representation.numeric.increment", "representation.code.alphanumeric_id"],
    "boolean": ["representation.boolean.binary", "representation.boolean.terms", "representation.boolean.initials"],
    "class": ["representation.discrete.ordinal"],
    "name": ["identity.person.full_name"],
    "gender": ["identity.person.gender"],
    "age": ["identity.person.age"],
    "number": ["representation.numeric.integer_number", "representation.numeric.decimal_number"],
    "code": ["representation.code.alphanumeric_id"],
    "price": ["representation.numeric.decimal_number"],
    "category": ["representation.discrete.categorical"],
    "city": ["geography.location.city"],
    "country": ["geography.location.country"],
    "iata": ["geography.transportation.iata_code"],
    "icao": ["geography.transportation.icao_code"],
    "latitude": ["geography.coordinate.latitude"],
    "longitude": ["geography.coordinate.longitude"],
    "utc offset": ["datetime.offset.utc"],
    "time zone": ["datetime.offset.iana"],
    "country code": ["geography.location.country_code"],
    "region": ["geography.location.region"],
    "date": ["datetime.date.iso", "datetime.date.us_slash", "datetime.date.eu_slash", "datetime.timestamp.iso_8601"],
    "state": ["geography.location.state", "geography.location.region"],
    "decimal number": ["representation.numeric.decimal_number"],
    "email": ["identity.person.email"],
    "currency": ["identity.payment.currency_code"],
    "postal code": ["geography.address.postal_code"],
    "status": ["representation.discrete.categorical"],
    "url": ["technology.internet.url"],
    "telephone": ["identity.person.phone_number"],
    "ip_v4": ["technology.internet.ip_v4"],
    "mac address": ["technology.internet.mac_address"],
    "operating system": ["technology.development.os"],
    "version": ["technology.development.version"],
    "language": ["technology.code.locale_code", "identity.person.nationality", "technology.development.programming_language"],
    "port": ["technology.internet.port"],
    "uuid": ["technology.cryptographic.uuid"],
    "user agent": ["technology.internet.user_agent"],
    "timestamp": ["datetime.timestamp.iso_8601", "datetime.timestamp.iso_8601_microseconds", "datetime.timestamp.sql_standard", "datetime.epoch.unix_seconds"],
    "first name": ["identity.person.first_name"],
    "last name": ["identity.person.last_name"],
    "occupation": ["identity.person.occupation"],
    "height": ["identity.person.height"],
    "weight": ["identity.person.weight"],
    "isbn": ["technology.code.isbn"],
    "issn": ["technology.code.issn"],
    "ean": ["technology.code.ean"],
    "credit card number": ["identity.payment.credit_card_number"],
    "cvv": ["identity.payment.cvv"],
    "swift code": ["identity.payment.swift_bic"],
    "color": ["representation.text.color_hex"],
    "file format": ["representation.file.mime_type"],
    "language code": ["technology.code.locale_code"],
    "hash": ["technology.cryptographic.hash"],
    "npi": ["identity.medical.npi"],
    "coordinates": ["geography.coordinate.coordinates"],
    "address": ["geography.address.full_address"],
    "street number": ["geography.address.street_number"],
    "percentage": ["representation.numeric.percentage"],
    "measurement unit": ["representation.scientific.measurement_unit"],
    "year": ["datetime.component.year"],
    "month name": ["datetime.component.month_name"],
    "day of week": ["datetime.component.day_of_week"],
    "time 24h": ["datetime.time.hms_24h", "datetime.time.hm_24h"],
    "duration": ["datetime.duration.iso_8601"],
    "sql timestamp": ["datetime.timestamp.sql_standard"],
    "time": ["datetime.epoch.unix_seconds", "datetime.epoch.unix_microseconds"],
    "http status code": ["technology.internet.http_status_code"],
    "rating": ["representation.discrete.ordinal"],
    "author": ["identity.person.full_name"],
    "title": ["representation.text.sentence"],
    "description": ["representation.text.sentence", "representation.text.plain_text"],
    "value": ["representation.numeric.decimal_number", "representation.numeric.integer_number"],
    "hostname": ["technology.internet.hostname"],
}


def main():
    import yaml

    print("Loading Model2Vec model for re-embedding...")
    model = StaticModel.from_pretrained("minishlab/potion-base-4M")
    token_embeddings = np.asarray(model.embedding, dtype=np.float32)

    # Load tokenizer for column name embedding
    tokenizer = Tokenizer.from_file(str(MODELS_DIR / "tokenizer.json"))

    # Load taxonomy
    taxonomy = load_taxonomy()

    # Build BASELINE synonym texts (same as prepare_model2vec.py)
    sys.path.insert(0, str(WORKSPACE / "scripts"))
    from prepare_model2vec import build_header_hint_entries, build_synonym_texts
    header_hints = build_header_hint_entries()
    baseline_synonyms = build_synonym_texts(taxonomy, header_hints)
    total_baseline = sum(len(v) for v in baseline_synonyms.values())

    # Build EXPANDED synonym texts
    expanded_synonyms = {}
    for label, texts in baseline_synonyms.items():
        expanded_synonyms[label] = list(texts)  # copy
    for label, extra in EXPANDED_SYNONYMS.items():
        if label in expanded_synonyms:
            existing = set(expanded_synonyms[label])
            for s in extra:
                if s.lower() not in existing:
                    expanded_synonyms[label].append(s.lower())
                    existing.add(s.lower())
        else:
            print(f"  Warning: {label} not in taxonomy, skipping")
    total_expanded = sum(len(v) for v in expanded_synonyms.values())

    print(f"\nBaseline: {len(baseline_synonyms)} types, {total_baseline} total synonym texts")
    print(f"Expanded: {len(expanded_synonyms)} types, {total_expanded} total synonym texts")
    print(f"Added: {total_expanded - total_baseline} new synonyms across {len(EXPANDED_SYNONYMS)} types")

    # Compute type embeddings for both
    print("\nComputing type embeddings...")
    baseline_embs, baseline_labels = compute_type_embeddings(model, baseline_synonyms)
    expanded_embs, expanded_labels = compute_type_embeddings(model, expanded_synonyms)
    assert baseline_labels == expanded_labels

    # Load ground truth
    columns = []
    with open(EVAL_DIR / "ground_truth.csv") as f:
        for row in csv.DictReader(f):
            columns.append(row)

    # Score all columns with both embeddings
    print(f"\nScoring {len(columns)} columns...")
    print(f"\n{'=' * 100}")
    print("COMPARISON: BASELINE vs EXPANDED SYNONYMS")
    print(f"{'=' * 100}")

    changes = []  # (column_name, gt, baseline_sim, expanded_sim, baseline_correct, expanded_correct)

    for col in columns:
        name = col["column_name"]
        gt = col["gt_label"]

        b_label, b_sim, b_correct = score_column(
            name, gt, baseline_embs, baseline_labels, tokenizer, token_embeddings, GT_TO_FINETYPE
        )
        e_label, e_sim, e_correct = score_column(
            name, gt, expanded_embs, expanded_labels, tokenizer, token_embeddings, GT_TO_FINETYPE
        )

        changes.append({
            "dataset": col["dataset"],
            "column_name": name,
            "gt_label": gt,
            "baseline_label": b_label,
            "baseline_sim": b_sim,
            "baseline_correct": b_correct,
            "expanded_label": e_label,
            "expanded_sim": e_sim,
            "expanded_correct": e_correct,
            "sim_delta": e_sim - b_sim,
        })

    # Summary
    b_tp = sum(1 for c in changes if c["baseline_correct"] and c["baseline_sim"] >= THRESHOLD)
    b_fp = sum(1 for c in changes if not c["baseline_correct"] and c["baseline_sim"] >= THRESHOLD)
    e_tp = sum(1 for c in changes if c["expanded_correct"] and c["expanded_sim"] >= THRESHOLD)
    e_fp = sum(1 for c in changes if not c["expanded_correct"] and c["expanded_sim"] >= THRESHOLD)

    print(f"\n  Baseline @ {THRESHOLD}: {b_tp} TP, {b_fp} FP")
    print(f"  Expanded @ {THRESHOLD}: {e_tp} TP, {e_fp} FP")
    print(f"  Net change: {e_tp - b_tp:+d} TP, {e_fp - b_fp:+d} FP")

    # Also show at lower threshold
    for thresh in [0.65, 0.60]:
        b_tp_t = sum(1 for c in changes if c["baseline_correct"] and c["baseline_sim"] >= thresh)
        b_fp_t = sum(1 for c in changes if not c["baseline_correct"] and c["baseline_sim"] >= thresh)
        e_tp_t = sum(1 for c in changes if c["expanded_correct"] and c["expanded_sim"] >= thresh)
        e_fp_t = sum(1 for c in changes if not c["expanded_correct"] and c["expanded_sim"] >= thresh)
        print(f"\n  Baseline @ {thresh}: {b_tp_t} TP, {b_fp_t} FP")
        print(f"  Expanded @ {thresh}: {e_tp_t} TP, {e_fp_t} FP")
        print(f"  Net change: {e_tp_t - b_tp_t:+d} TP, {e_fp_t - b_fp_t:+d} FP")

    # Show biggest improvements
    improvements = [c for c in changes if c["sim_delta"] > 0.01]
    improvements.sort(key=lambda x: -x["sim_delta"])

    print(f"\n{'=' * 100}")
    print(f"TOP IMPROVEMENTS (sim delta > 0.01): {len(improvements)} columns")
    print(f"{'=' * 100}")
    print(f"{'Column Name':30s} {'GT':20s} {'Base Label':40s} {'B.Sim':>6s} {'Exp Label':40s} {'E.Sim':>6s} {'Delta':>7s}")
    print("-" * 150)
    for c in improvements[:40]:
        b_icon = "✅" if c["baseline_correct"] else "❌"
        e_icon = "✅" if c["expanded_correct"] else "❌"
        print(
            f"  {c['column_name']:30s} {c['gt_label']:20s} "
            f"{b_icon} {c['baseline_label'] or '':38s} {c['baseline_sim']:6.3f} "
            f"{e_icon} {c['expanded_label'] or '':38s} {c['expanded_sim']:6.3f} {c['sim_delta']:+7.3f}"
        )

    # Show regressions (if any)
    regressions = [c for c in changes if c["sim_delta"] < -0.01]
    regressions.sort(key=lambda x: x["sim_delta"])
    if regressions:
        print(f"\n{'=' * 100}")
        print(f"REGRESSIONS (sim delta < -0.01): {len(regressions)} columns")
        print(f"{'=' * 100}")
        for c in regressions[:20]:
            b_icon = "✅" if c["baseline_correct"] else "❌"
            e_icon = "✅" if c["expanded_correct"] else "❌"
            print(
                f"  {c['column_name']:30s} {c['gt_label']:20s} "
                f"{b_icon} {c['baseline_label'] or '':38s} {c['baseline_sim']:6.3f} "
                f"{e_icon} {c['expanded_label'] or '':38s} {c['expanded_sim']:6.3f} {c['sim_delta']:+7.3f}"
            )

    # Columns that flip from below threshold to above
    flipped_up = [c for c in changes
                  if c["expanded_correct"] and c["expanded_sim"] >= THRESHOLD
                  and c["baseline_sim"] < THRESHOLD]
    if flipped_up:
        print(f"\n{'=' * 100}")
        print(f"NEWLY RECOVERED (below→above threshold): {len(flipped_up)} columns")
        print(f"{'=' * 100}")
        for c in sorted(flipped_up, key=lambda x: -x["expanded_sim"]):
            print(f"  {c['column_name']:30s} {c['gt_label']:20s} "
                  f"{c['baseline_sim']:.3f} → {c['expanded_sim']:.3f} "
                  f"({c['expanded_label']})")

    # Columns that flip from correct to wrong
    flipped_bad = [c for c in changes
                   if c["baseline_correct"] and not c["expanded_correct"]
                   and c["expanded_sim"] >= THRESHOLD]
    if flipped_bad:
        print(f"\n{'=' * 100}")
        print(f"NEW FALSE POSITIVES (correct→wrong above threshold): {len(flipped_bad)} columns")
        print(f"{'=' * 100}")
        for c in flipped_bad:
            print(f"  {c['column_name']:30s} {c['gt_label']:20s} "
                  f"was {c['baseline_label']} → now {c['expanded_label']} "
                  f"(sim {c['baseline_sim']:.3f} → {c['expanded_sim']:.3f})")

    # Full threshold sweep comparison
    print(f"\n{'=' * 100}")
    print("THRESHOLD SWEEP: BASELINE vs EXPANDED")
    print(f"{'=' * 100}")
    print(f"{'Threshold':>10s} {'B.TP':>6s} {'B.FP':>6s} {'B.Prec':>8s} {'B.Rec':>7s} │ {'E.TP':>6s} {'E.FP':>6s} {'E.Prec':>8s} {'E.Rec':>7s} │ {'ΔTP':>5s} {'ΔFP':>5s}")

    total_correct = sum(1 for c in changes if c["expanded_correct"])
    for thresh in [0.50, 0.55, 0.60, 0.65, 0.68, 0.70, 0.72, 0.75, 0.80]:
        btp = sum(1 for c in changes if c["baseline_correct"] and c["baseline_sim"] >= thresh)
        bfp = sum(1 for c in changes if not c["baseline_correct"] and c["baseline_sim"] >= thresh)
        etp = sum(1 for c in changes if c["expanded_correct"] and c["expanded_sim"] >= thresh)
        efp = sum(1 for c in changes if not c["expanded_correct"] and c["expanded_sim"] >= thresh)
        bp = btp / (btp + bfp) if (btp + bfp) else 0
        br = btp / total_correct if total_correct else 0
        ep = etp / (etp + efp) if (etp + efp) else 0
        er = etp / total_correct if total_correct else 0
        print(
            f"  {thresh:>8.2f}   {btp:>4d}   {bfp:>4d}   {bp:>7.3f}   {br:>6.3f} │ "
            f"  {etp:>4d}   {efp:>4d}   {ep:>7.3f}   {er:>6.3f} │ {etp-btp:>+4d} {efp-bfp:>+4d}"
        )


if __name__ == "__main__":
    main()
