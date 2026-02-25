#!/usr/bin/env python3
"""Analyse Model2Vec semantic similarity for all profile eval column names.

Measures the current similarity distribution and identifies:
- Correct matches above threshold (true positives in current system)
- Correct matches below threshold (lost opportunities)
- Wrong matches above threshold (false positives if threshold were lowered)
- No useful match (generic/ambiguous names)

Usage:
    python discovery/model2vec-specialisation/analyse_similarity.py
"""

import csv
import json
import sys
from pathlib import Path

import numpy as np
from safetensors.numpy import load_file
from tokenizers import Tokenizer

# Resolve paths
WORKSPACE = Path(__file__).parent.parent.parent
MODELS_DIR = WORKSPACE / "models" / "model2vec"
EVAL_DIR = WORKSPACE / "eval" / "eval_output"

THRESHOLD = 0.70


def load_model():
    """Load Model2Vec artifacts."""
    tokenizer = Tokenizer.from_file(str(MODELS_DIR / "tokenizer.json"))
    model_tensors = load_file(str(MODELS_DIR / "model.safetensors"))
    embeddings = model_tensors["embeddings"].astype(np.float32)
    type_tensors = load_file(str(MODELS_DIR / "type_embeddings.safetensors"))
    type_embeddings = type_tensors["type_embeddings"].astype(np.float32)
    with open(MODELS_DIR / "label_index.json") as f:
        label_index = json.load(f)
    return tokenizer, embeddings, type_embeddings, label_index


def embed_column_name(name: str, tokenizer, embeddings) -> np.ndarray | None:
    """Embed a column name using the same logic as semantic.rs."""
    # Normalize: lowercase, replace separators with spaces
    normalized = name.lower().replace("_", " ").replace("-", " ").replace(".", " ").strip()
    if not normalized:
        return None

    encoding = tokenizer.encode(normalized, add_special_tokens=False)
    ids = [i for i in encoding.ids if i != 0]  # filter PAD

    if not ids:
        return None

    # Index into embedding matrix
    token_embeds = embeddings[ids]  # [n_tokens, dim]

    # Mean pool
    mean_embed = np.mean(token_embeds, axis=0)  # [dim]

    # L2 normalize
    norm = np.linalg.norm(mean_embed)
    if norm < 1e-8:
        return None
    return mean_embed / norm


def classify_name(query_vec, type_embeddings, label_index):
    """Get top-K matches for a query vector."""
    similarities = type_embeddings @ query_vec  # [n_types]
    sorted_indices = np.argsort(similarities)[::-1]

    results = []
    for idx in sorted_indices[:5]:  # Top 5
        results.append((label_index[idx], float(similarities[idx])))
    return results


# Ground truth label → acceptable FineType label mappings
# This is a simplified mapping for scoring. A match is "correct" if the
# Model2Vec prediction matches any of the acceptable labels.
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
    "date": ["datetime.date.iso", "datetime.date.us_slash", "datetime.date.eu_slash"],
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
    "timestamp": ["datetime.timestamp.iso_8601", "datetime.timestamp.iso_8601_microseconds", "datetime.timestamp.sql_standard"],
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
    "description": ["representation.text.sentence"],
    "value": ["representation.numeric.decimal_number", "representation.numeric.integer_number"],
    "hostname": ["technology.internet.hostname"],
}


def is_correct_match(predicted_label: str, gt_label: str) -> bool:
    """Check if a Model2Vec prediction is correct for the ground truth."""
    acceptable = GT_TO_FINETYPE.get(gt_label, [])
    if predicted_label in acceptable:
        return True
    # Also check domain match (less strict)
    if acceptable:
        pred_domain = predicted_label.split(".")[0]
        for acc in acceptable:
            if acc.split(".")[0] == pred_domain:
                return True  # Domain-level match
    return False


def main():
    print("Loading Model2Vec artifacts...")
    tokenizer, embeddings, type_embeddings, label_index = load_model()
    print(f"  Token embeddings: {embeddings.shape}")
    print(f"  Type embeddings: {type_embeddings.shape} ({len(label_index)} types)")

    # Load ground truth
    gt_path = EVAL_DIR / "ground_truth.csv"
    columns = []
    with open(gt_path) as f:
        reader = csv.DictReader(f)
        for row in reader:
            columns.append({
                "dataset": row["dataset"],
                "column_name": row["column_name"],
                "gt_label": row["gt_label"],
            })
    print(f"  Ground truth: {len(columns)} columns across {len(set(r['dataset'] for r in columns))} datasets")

    # Compute similarities for all column names
    results = []
    for col in columns:
        name = col["column_name"]
        gt = col["gt_label"]

        vec = embed_column_name(name, tokenizer, embeddings)
        if vec is None:
            results.append({
                **col,
                "top1_label": None,
                "top1_sim": 0.0,
                "top2_label": None,
                "top2_sim": 0.0,
                "correct": False,
                "above_threshold": False,
                "category": "no_embedding",
            })
            continue

        matches = classify_name(vec, type_embeddings, label_index)
        top1_label, top1_sim = matches[0]
        top2_label, top2_sim = matches[1]

        correct = is_correct_match(top1_label, gt)
        above = top1_sim >= THRESHOLD

        if correct and above:
            cat = "correct_above"
        elif correct and not above:
            cat = "correct_below"
        elif not correct and above:
            cat = "wrong_above"
        else:
            cat = "wrong_below"

        results.append({
            **col,
            "top1_label": top1_label,
            "top1_sim": top1_sim,
            "top2_label": top2_label,
            "top2_sim": top2_sim,
            "top1_top2_gap": top1_sim - top2_sim,
            "correct": correct,
            "above_threshold": above,
            "category": cat,
        })

    # Summary statistics
    print("\n" + "=" * 80)
    print("SIMILARITY DISTRIBUTION SUMMARY")
    print("=" * 80)

    cats = {}
    for r in results:
        cat = r["category"]
        cats.setdefault(cat, []).append(r)

    total = len(results)
    for cat_name, cat_label in [
        ("correct_above", "✅ Correct match ABOVE threshold (active TPs)"),
        ("correct_below", "🟡 Correct match BELOW threshold (lost opportunities)"),
        ("wrong_above", "🔴 Wrong match ABOVE threshold (false positives)"),
        ("wrong_below", "⚪ Wrong/no match below threshold (correctly rejected)"),
        ("no_embedding", "⚫ Could not embed"),
    ]:
        items = cats.get(cat_name, [])
        pct = len(items) / total * 100 if total else 0
        print(f"\n{cat_label}: {len(items)}/{total} ({pct:.1f}%)")
        if items and cat_name in ("correct_below", "wrong_above"):
            for r in sorted(items, key=lambda x: -x["top1_sim"]):
                print(f"  {r['dataset']}.{r['column_name']:30s} → {r['top1_label']:45s} sim={r['top1_sim']:.3f}  (gt: {r['gt_label']})")

    # Detailed: correct matches below threshold (the recoverable ones)
    correct_below = cats.get("correct_below", [])
    if correct_below:
        print(f"\n{'=' * 80}")
        print(f"RECOVERABLE: {len(correct_below)} correct matches below {THRESHOLD} threshold")
        print(f"{'=' * 80}")
        sims = [r["top1_sim"] for r in correct_below]
        print(f"  Similarity range: {min(sims):.3f} - {max(sims):.3f}")
        print(f"  Mean: {np.mean(sims):.3f}, Median: {np.median(sims):.3f}")
        for bucket_lo, bucket_hi in [(0.65, 0.70), (0.60, 0.65), (0.55, 0.60), (0.50, 0.55), (0.0, 0.50)]:
            in_bucket = [r for r in correct_below if bucket_lo <= r["top1_sim"] < bucket_hi]
            if in_bucket:
                print(f"\n  Bucket [{bucket_lo:.2f}, {bucket_hi:.2f}): {len(in_bucket)} columns")
                for r in sorted(in_bucket, key=lambda x: -x["top1_sim"]):
                    print(f"    {r['column_name']:30s} → {r['top1_label']:40s} sim={r['top1_sim']:.3f}")

    # Wrong matches above threshold (the dangerous ones)
    wrong_above = cats.get("wrong_above", [])
    if wrong_above:
        print(f"\n{'=' * 80}")
        print(f"FALSE POSITIVES: {len(wrong_above)} wrong matches above {THRESHOLD} threshold")
        print(f"{'=' * 80}")
        for r in sorted(wrong_above, key=lambda x: -x["top1_sim"]):
            print(f"  {r['column_name']:30s} → {r['top1_label']:40s} sim={r['top1_sim']:.3f}  (gt: {r['gt_label']}, top2: {r['top2_label']} @ {r['top2_sim']:.3f})")

    # Full table sorted by similarity
    print(f"\n{'=' * 80}")
    print(f"FULL SIMILARITY TABLE (sorted by similarity, descending)")
    print(f"{'=' * 80}")
    print(f"{'Column Name':30s} {'GT Label':20s} {'Top-1 Prediction':45s} {'Sim':>6s} {'Cat':>6s}")
    print("-" * 120)
    for r in sorted(results, key=lambda x: -x["top1_sim"]):
        cat_icon = {
            "correct_above": "✅",
            "correct_below": "🟡",
            "wrong_above": "🔴",
            "wrong_below": "⚪",
            "no_embedding": "⚫",
        }.get(r["category"], "?")
        label_str = r["top1_label"] or "(none)"
        print(f"  {r['column_name']:30s} {r['gt_label']:20s} {label_str:45s} {r['top1_sim']:6.3f} {cat_icon}")

    # Similarity histogram
    all_sims = [r["top1_sim"] for r in results if r["top1_sim"] > 0]
    correct_sims = [r["top1_sim"] for r in results if r["correct"]]
    wrong_sims = [r["top1_sim"] for r in results if not r["correct"] and r["top1_sim"] > 0]

    print(f"\n{'=' * 80}")
    print("THRESHOLD SWEEP")
    print(f"{'=' * 80}")
    print(f"{'Threshold':>10s} {'Correct TP':>12s} {'Wrong FP':>10s} {'Precision':>10s} {'Recall':>10s}")
    for thresh in [0.50, 0.55, 0.60, 0.65, 0.68, 0.70, 0.72, 0.75, 0.80]:
        tp = sum(1 for r in results if r["correct"] and r["top1_sim"] >= thresh)
        fp = sum(1 for r in results if not r["correct"] and r["top1_sim"] >= thresh)
        total_correct = sum(1 for r in results if r["correct"])
        precision = tp / (tp + fp) if (tp + fp) > 0 else 0
        recall = tp / total_correct if total_correct > 0 else 0
        print(f"  {thresh:>8.2f}   {tp:>10d}   {fp:>8d}   {precision:>9.3f}   {recall:>9.3f}")


if __name__ == "__main__":
    main()
