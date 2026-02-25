#!/usr/bin/env python3
"""Evaluate custom distillation with analytics-domain vocabulary.

Distills from sentence-transformers/all-MiniLM-L6-v2 with a vocabulary
curated for column name classification, then compares against
the stock potion-base-4M model.

Also tests false positive risk with an expanded generic name set (AC #4).

Usage:
    python3 discovery/model2vec-specialisation/evaluate_distillation.py
"""

import csv
import json
import sys
from pathlib import Path

import numpy as np
from model2vec import StaticModel
try:
    from model2vec.distill import distill
    HAS_DISTILL = True
except ImportError:
    HAS_DISTILL = False
from safetensors.numpy import load_file
from tokenizers import Tokenizer

WORKSPACE = Path(__file__).parent.parent.parent
MODELS_DIR = WORKSPACE / "models" / "model2vec"
EVAL_DIR = WORKSPACE / "eval" / "eval_output"

THRESHOLD = 0.70

# Analytics/database column name vocabulary for custom distillation
ANALYTICS_VOCAB = [
    # Type taxonomy labels (all 169 leaf names)
    "email", "url", "ip", "uuid", "gender", "age", "latitude", "longitude",
    "country", "city", "state", "region", "postal code", "zip code",
    "phone", "telephone", "mobile", "fax",
    "name", "first name", "last name", "full name", "surname",
    "date", "time", "timestamp", "datetime", "year", "month", "day",
    "boolean", "binary", "flag", "indicator",
    "category", "categorical", "ordinal", "class", "rank", "rating",
    "number", "integer", "decimal", "float", "percentage", "percent",
    "code", "identifier", "id", "key", "reference",
    "address", "street", "coordinate", "elevation", "altitude",
    "currency", "price", "cost", "amount", "salary", "wage",
    "password", "hash", "token", "secret",
    "version", "port", "hostname", "mac address", "user agent",
    "isbn", "issn", "ean", "npi", "cvv", "iban", "swift",
    "credit card", "debit card",
    "occupation", "job title", "profession",
    "weight", "height", "temperature", "pressure",
    "duration", "interval", "period",
    "description", "title", "comment", "note", "text", "sentence",
    "status", "state", "phase", "stage",
    "formula", "equation", "expression",
    "locale", "language", "timezone",
    "color", "colour", "hex color",
    "mime type", "content type", "file format",
    "measurement", "unit", "metric",
    "continent", "country code", "calling code",
    "iata", "icao",

    # Common database column naming patterns
    "created at", "updated at", "deleted at", "modified at",
    "created date", "update date", "modified date",
    "start date", "end date", "due date", "effective date",
    "birth date", "date of birth", "dob", "hire date",
    "order id", "customer id", "user id", "product id", "session id",
    "order date", "purchase date", "ship date", "delivery date",
    "total price", "unit price", "list price", "sale price",
    "quantity", "qty", "count", "total", "subtotal",
    "tax", "discount", "shipping", "handling",
    "first name", "last name", "middle name", "display name",
    "email address", "phone number", "fax number",
    "street address", "city name", "state code", "zip",
    "country name", "country code", "region code",

    # Common abbreviations
    "amt", "qty", "pct", "dt", "ts", "desc", "cat", "num", "val",
    "yr", "mo", "hr", "min", "sec", "ms",
    "src", "dst", "req", "res", "msg", "err",
    "avg", "sum", "cnt", "max", "min", "std", "var",
    "lat", "lng", "lon", "alt", "elev",

    # Generic column names (should NOT match)
    "data", "value", "field", "column", "col", "var",
    "x", "y", "z", "a", "b", "c",
    "col1", "col2", "col3", "field1", "field2",
    "foo", "bar", "baz", "test", "tmp", "temp",
    "V1", "V2", "V3", "result", "output", "input",

    # Domain-specific terms
    "revenue", "profit", "margin", "balance", "volume",
    "attendance", "capacity", "frequency", "rate",
    "score", "grade", "level", "tier", "priority",
    "sport", "genre", "brand", "model", "series",
    "diagnosis", "procedure", "patient", "specimen",
    "ticker", "exchange", "dividend", "yield", "ratio",
    "species", "genus", "family", "order", "kingdom",

    # Type system keywords
    "string", "int", "bigint", "smallint", "tinyint",
    "float", "double", "real", "numeric", "decimal",
    "varchar", "char", "text", "blob", "clob",
    "date", "time", "timestamp", "interval",
    "boolean", "bool", "bit",
    "json", "xml", "yaml", "csv",
    "array", "list", "set", "map", "dict",
]


# ═══════════════════════════════════════════════════════════════════════════════
# Expanded set of generic/ambiguous column names for false positive testing
# ═══════════════════════════════════════════════════════════════════════════════

GENERIC_NAMES = [
    # Pure generic
    "data", "value", "field", "column", "col", "var", "variable",
    "x", "y", "z", "a", "b", "c", "n", "m", "k",
    "col1", "col2", "col3", "col4", "col5",
    "field1", "field2", "field3",
    "V1", "V2", "V3", "V4", "V5",
    "var1", "var2", "var3",
    "foo", "bar", "baz", "qux", "quux",
    "test", "tmp", "temp", "sample", "example",
    "result", "output", "input", "param", "arg",
    "item", "element", "entry", "record", "row",
    "key", "val", "pair", "tuple",
    "info", "detail", "meta", "extra", "misc",
    "other", "unknown", "undefined", "null", "none",
    "custom", "user defined", "raw", "original",
    "new", "old", "prev", "next", "current",
    "primary", "secondary", "tertiary",
    "left", "right", "top", "bottom", "center",
    "main", "sub", "alt", "backup",
    "feature", "attribute", "property",
    "label", "tag", "marker", "flag",
    "index", "idx", "pos", "offset",
    "count", "total", "sum", "avg", "mean",
    "min", "max", "range", "span",
    "note", "comment", "remark", "annotation",
    "group", "cluster", "bucket", "bin",
    "source", "target", "origin", "destination",
    "parent", "child", "sibling",
    "ref", "reference", "link", "pointer",
    "text", "string", "blob", "content",
    "json", "xml", "html", "csv",
    "config", "setting", "option", "preference",
    "error", "warning", "message", "log",
    "action", "event", "trigger", "callback",
    "query", "filter", "sort", "order",
    "page", "section", "block", "chunk",
    # Short codes
    "id", "pk", "fk", "sk", "rk",
    "dt", "ts", "seq", "ver",
]


# Same GT mapping as analyse_similarity.py
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


def load_taxonomy():
    import yaml
    taxonomy = {}
    for f in sorted(WORKSPACE.glob("labels/definitions_*.yaml")):
        with open(f) as fh:
            data = yaml.safe_load(fh)
            if data:
                taxonomy.update(data)
    return taxonomy


def embed_name(name, model):
    """Embed column name using Model2Vec model directly."""
    normalized = name.lower().replace("_", " ").replace("-", " ").replace(".", " ").strip()
    if not normalized:
        return None
    vec = model.encode([normalized])[0]
    norm = np.linalg.norm(vec)
    if norm < 1e-8:
        return None
    return vec / norm


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


def test_model(model, model_name, synonyms, columns, generic_names):
    """Test a model against columns and generic names."""
    type_embs, label_index = compute_type_embeddings(model, synonyms)

    # Score eval columns
    results = []
    for col in columns:
        name = col["column_name"]
        gt = col["gt_label"]
        vec = embed_name(name, model)
        if vec is None:
            results.append({"name": name, "gt": gt, "label": None, "sim": 0.0, "correct": False})
            continue
        sims = type_embs @ vec
        best_idx = np.argmax(sims)
        best_label = label_index[best_idx]
        best_sim = float(sims[best_idx])
        acceptable = GT_TO_FINETYPE.get(gt, [])
        correct = best_label in acceptable
        if not correct and acceptable:
            pred_domain = best_label.split(".")[0]
            for acc in acceptable:
                if acc.split(".")[0] == pred_domain:
                    correct = True
                    break
        results.append({"name": name, "gt": gt, "label": best_label, "sim": best_sim, "correct": correct})

    # Score generic names (should all be below threshold)
    generic_results = []
    for name in generic_names:
        vec = embed_name(name, model)
        if vec is None:
            generic_results.append({"name": name, "label": None, "sim": 0.0})
            continue
        sims = type_embs @ vec
        best_idx = np.argmax(sims)
        generic_results.append({
            "name": name,
            "label": label_index[best_idx],
            "sim": float(sims[best_idx]),
        })

    # Print summary
    print(f"\n{'=' * 80}")
    print(f"MODEL: {model_name}")
    print(f"{'=' * 80}")

    for thresh in [0.60, 0.65, 0.68, 0.70, 0.75]:
        tp = sum(1 for r in results if r["correct"] and r["sim"] >= thresh)
        fp = sum(1 for r in results if not r["correct"] and r["sim"] >= thresh)
        prec = tp / (tp + fp) if (tp + fp) else 0
        total_correct = sum(1 for r in results if r["correct"])
        rec = tp / total_correct if total_correct else 0
        generic_fp = sum(1 for r in generic_results if r["sim"] >= thresh)
        print(f"  @{thresh:.2f}: {tp:>3d} TP, {fp:>3d} FP, prec={prec:.3f}, rec={rec:.3f}, generic_fp={generic_fp}/{len(generic_results)}")

    # Generic name false positive details
    generic_above = [(r["name"], r["label"], r["sim"]) for r in generic_results if r["sim"] >= THRESHOLD]
    if generic_above:
        print(f"\n  Generic names above {THRESHOLD} (FALSE POSITIVES):")
        for name, label, sim in sorted(generic_above, key=lambda x: -x[2]):
            print(f"    '{name}' → {label} @ {sim:.3f}")
    else:
        print(f"\n  ✅ Zero generic names above {THRESHOLD}")

    # Show generic name distribution
    generic_sims = [r["sim"] for r in generic_results]
    print(f"\n  Generic name similarity stats: min={min(generic_sims):.3f}, max={max(generic_sims):.3f}, mean={np.mean(generic_sims):.3f}, median={np.median(generic_sims):.3f}")
    for thresh_test in [0.50, 0.55, 0.60, 0.65, 0.70]:
        n_above = sum(1 for s in generic_sims if s >= thresh_test)
        print(f"    >= {thresh_test}: {n_above}/{len(generic_sims)} ({n_above/len(generic_sims)*100:.1f}%)")

    return results, generic_results


def main():
    # Load evaluation data
    columns = []
    with open(EVAL_DIR / "ground_truth.csv") as f:
        for row in csv.DictReader(f):
            columns.append(row)

    # Load taxonomy + build synonyms
    taxonomy = load_taxonomy()
    sys.path.insert(0, str(WORKSPACE / "scripts"))
    from prepare_model2vec import build_header_hint_entries, build_synonym_texts
    header_hints = build_header_hint_entries()
    synonyms = build_synonym_texts(taxonomy, header_hints)

    # ═══════════════════════════════════════════════════════════════════════════
    # Test 1: Stock potion-base-4M (baseline)
    # ═══════════════════════════════════════════════════════════════════════════
    print("Loading baseline model: potion-base-4M...")
    baseline_model = StaticModel.from_pretrained("minishlab/potion-base-4M")
    baseline_results, baseline_generic = test_model(
        baseline_model, "potion-base-4M (baseline)", synonyms, columns, GENERIC_NAMES
    )

    # ═══════════════════════════════════════════════════════════════════════════
    # Test 2: Custom distillation from all-MiniLM-L6-v2 (requires torch)
    # ═══════════════════════════════════════════════════════════════════════════
    if HAS_DISTILL:
        print("\n\nDistilling custom model from all-MiniLM-L6-v2...")
        print(f"  Vocabulary size: {len(ANALYTICS_VOCAB)} terms")
        try:
            custom_model = distill(
                model_name_or_path="sentence-transformers/all-MiniLM-L6-v2",
                vocabulary=ANALYTICS_VOCAB,
                pca_dims=256,
            )
            print(f"  Distilled model embedding shape: {custom_model.embedding.shape}")
            test_model(custom_model, "MiniLM-L6-v2 distilled (custom vocab)", synonyms, columns, GENERIC_NAMES)
        except Exception as e:
            print(f"  Distillation failed: {e}")

        print("\n\nDistilling custom model (PCA 128)...")
        try:
            custom128_model = distill(
                model_name_or_path="sentence-transformers/all-MiniLM-L6-v2",
                vocabulary=ANALYTICS_VOCAB,
                pca_dims=128,
            )
            print(f"  Distilled model embedding shape: {custom128_model.embedding.shape}")
            test_model(custom128_model, "MiniLM-L6-v2 distilled (custom vocab, PCA 128)", synonyms, columns, GENERIC_NAMES)
        except Exception as e:
            print(f"  Distillation (128) failed: {e}")
    else:
        print("\n\n⚠️  Skipping custom distillation (torch not installed)")
        print("  To test: pip install model2vec[distill]")

    # ═══════════════════════════════════════════════════════════════════════════
    # AC #4: FALSE POSITIVE ASSESSMENT
    # ═══════════════════════════════════════════════════════════════════════════
    print(f"\n\n{'=' * 80}")
    print("FALSE POSITIVE ASSESSMENT (AC #4)")
    print(f"{'=' * 80}")
    print(f"\nTested {len(GENERIC_NAMES)} generic/ambiguous column names against baseline model")
    print(f"These names should NOT trigger semantic hints.\n")

    # Detailed false positive report for baseline at various thresholds
    for thresh in [0.65, 0.68, 0.70]:
        fps = [(r["name"], r["label"], r["sim"]) for r in baseline_generic if r["sim"] >= thresh]
        print(f"\n  At threshold {thresh}: {len(fps)} false positives out of {len(GENERIC_NAMES)}")
        for name, label, sim in sorted(fps, key=lambda x: -x[2]):
            print(f"    '{name}' → {label} @ {sim:.3f}")


if __name__ == "__main__":
    main()
