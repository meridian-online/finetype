#!/usr/bin/env python3
"""Prepare Model2Vec semantic embeddings for FineType column name classification.

Downloads a pre-distilled Model2Vec model (potion-base-4M), reads the FineType
taxonomy, and pre-computes type label embeddings from title + aliases + header
hint entries. Outputs 4 files into models/model2vec/:

  tokenizer.json          — wordpiece tokenizer (copied from potion model)
  model.safetensors       — token embedding matrix [vocab_size, embed_dim]
  type_embeddings.safetensors — pre-computed label embeddings [N_types * K, embed_dim]
  label_index.json        — ordered list mapping row index to label string

Max-sim matching (default): stores K representative embeddings per type using
Farthest Point Sampling. Layout is interleaved: rows 0..K are type 0's K
embeddings, rows K..2K are type 1's, etc. K is inferred at load time from
type_embeddings.shape[0] / len(label_index).

Usage:
    python scripts/prepare_model2vec.py [--model minishlab/potion-base-4M] [--max-k 3]
    python scripts/prepare_model2vec.py --legacy  # Force K=1 mean-pool (old behavior)
"""

import argparse
import json
import os
import re
import sys
from pathlib import Path

import numpy as np
import yaml
from model2vec import StaticModel
from safetensors.numpy import save_file


def load_taxonomy(labels_dir: Path) -> dict:
    """Load all taxonomy definitions from YAML files."""
    taxonomy = {}
    for yaml_file in sorted(labels_dir.glob("definitions_*.yaml")):
        with open(yaml_file) as f:
            data = yaml.safe_load(f)
            if data:
                taxonomy.update(data)
    return taxonomy


def build_header_hint_entries() -> dict[str, str]:
    """Extract all exact-match entries from the current header_hint() function.

    This ensures backward compatibility — every column name that the hardcoded
    function recognises will appear as a synonym for the corresponding type.
    """
    # Manually curated from column.rs header_hint() exact matches.
    # If header_hint() changes, re-extract these.
    return {
        # Email
        "email": "identity.person.email",
        "e mail": "identity.person.email",
        "email address": "identity.person.email",
        "emailaddress": "identity.person.email",
        # URL
        "url": "technology.internet.url",
        "uri": "technology.internet.url",
        "link": "technology.internet.url",
        "href": "technology.internet.url",
        "website": "technology.internet.url",
        "homepage": "technology.internet.url",
        "tracking url": "technology.internet.url",
        "callback url": "technology.internet.url",
        "redirect url": "technology.internet.url",
        "api url": "technology.internet.url",
        # IP
        "ip": "technology.internet.ip_v4",
        "ip address": "technology.internet.ip_v4",
        "ipaddress": "technology.internet.ip_v4",
        "ip addr": "technology.internet.ip_v4",
        "source ip": "technology.internet.ip_v4",
        "destination ip": "technology.internet.ip_v4",
        "src ip": "technology.internet.ip_v4",
        "dst ip": "technology.internet.ip_v4",
        "server ip": "technology.internet.ip_v4",
        "client ip": "technology.internet.ip_v4",
        "remote ip": "technology.internet.ip_v4",
        "local ip": "technology.internet.ip_v4",
        # UUID
        "uuid": "technology.cryptographic.uuid",
        "guid": "technology.cryptographic.uuid",
        # Person
        "gender": "identity.person.gender",
        "sex": "identity.person.gender",
        "age": "identity.person.age",
        # Geo coordinates
        "latitude": "geography.coordinate.latitude",
        "lat": "geography.coordinate.latitude",
        "longitude": "geography.coordinate.longitude",
        "lng": "geography.coordinate.longitude",
        "lon": "geography.coordinate.longitude",
        "long": "geography.coordinate.longitude",
        # Geo locations
        "country": "geography.location.country",
        "country name": "geography.location.country",
        "country code": "geography.location.country_code",
        "alpha 2": "geography.location.country_code",
        "alpha 3": "geography.location.country_code",
        "iso country": "geography.location.country_code",
        "iso alpha 2": "geography.location.country_code",
        "iso alpha 3": "geography.location.country_code",
        "country iso": "geography.location.country_code",
        "city": "geography.location.city",
        "city name": "geography.location.city",
        "state": "geography.location.region",
        "province": "geography.location.region",
        "region": "geography.location.region",
        "subcountry": "geography.location.region",
        "subregion": "geography.location.region",
        "sub region": "geography.location.region",
        "sub country": "geography.location.region",
        # Currency
        "currency": "identity.payment.currency_code",
        "currency code": "identity.payment.currency_code",
        # Port
        "port": "technology.internet.port",
        # Phone
        "phone": "identity.person.phone_number",
        "phone number": "identity.person.phone_number",
        "telephone": "identity.person.phone_number",
        "mobile": "identity.person.phone_number",
        "fax": "identity.person.phone_number",
        # Postal
        "zip": "geography.address.postal_code",
        "zip code": "geography.address.postal_code",
        "zipcode": "geography.address.postal_code",
        "postal code": "geography.address.postal_code",
        "postalcode": "geography.address.postal_code",
        "postcode": "geography.address.postal_code",
        "shipping postal code": "geography.address.postal_code",
        "billing postal code": "geography.address.postal_code",
        "mailing zip": "geography.address.postal_code",
        # Names
        "name": "identity.person.full_name",
        "full name": "identity.person.full_name",
        "fullname": "identity.person.full_name",
        "first name": "identity.person.first_name",
        "firstname": "identity.person.first_name",
        "given name": "identity.person.first_name",
        "last name": "identity.person.last_name",
        "lastname": "identity.person.last_name",
        "surname": "identity.person.last_name",
        "family name": "identity.person.last_name",
        # Date/time
        "date": "datetime.timestamp.iso_8601",
        "created date": "datetime.timestamp.iso_8601",
        "timestamp": "datetime.timestamp.iso_8601",
        "datetime": "datetime.timestamp.iso_8601",
        "year": "datetime.component.year",
        "birth date": "datetime.date.iso",
        "birthdate": "datetime.date.iso",
        "dob": "datetime.date.iso",
        "date of birth": "datetime.date.iso",
        # Password
        "password": "identity.person.password",
        "passwd": "identity.person.password",
        # Numeric
        "price": "representation.numeric.decimal_number",
        "cost": "representation.numeric.decimal_number",
        "amount": "representation.numeric.decimal_number",
        "salary": "representation.numeric.decimal_number",
        "fare": "representation.numeric.decimal_number",
        "fee": "representation.numeric.decimal_number",
        "toll": "representation.numeric.decimal_number",
        "charge": "representation.numeric.decimal_number",
        "count": "representation.numeric.integer_number",
        "quantity": "representation.numeric.integer_number",
        "qty": "representation.numeric.integer_number",
        "sibsp": "representation.numeric.integer_number",
        "parch": "representation.numeric.integer_number",
        "siblings": "representation.numeric.integer_number",
        "parents": "representation.numeric.integer_number",
        "children": "representation.numeric.integer_number",
        "dependents": "representation.numeric.integer_number",
        "id": "representation.numeric.increment",
        "identifier": "representation.numeric.increment",
        # Ordinal
        "class": "representation.discrete.ordinal",
        "pclass": "representation.discrete.ordinal",
        "grade": "representation.discrete.ordinal",
        "rank": "representation.discrete.ordinal",
        "level": "representation.discrete.ordinal",
        "tier": "representation.discrete.ordinal",
        "rating": "representation.discrete.ordinal",
        "priority": "representation.discrete.ordinal",
        "score": "representation.discrete.ordinal",
        # Boolean
        "survived": "representation.boolean.binary",
        "alive": "representation.boolean.binary",
        "deceased": "representation.boolean.binary",
        "dead": "representation.boolean.binary",
        "active": "representation.boolean.binary",
        "enabled": "representation.boolean.binary",
        "disabled": "representation.boolean.binary",
        "deleted": "representation.boolean.binary",
        "verified": "representation.boolean.binary",
        "approved": "representation.boolean.binary",
        "flagged": "representation.boolean.binary",
        # UTC offset
        "utc offset": "datetime.offset.utc",
        "gmt offset": "datetime.offset.utc",
        "timezone offset": "datetime.offset.utc",
        "tz offset": "datetime.offset.utc",
        "utcoffset": "datetime.offset.utc",
        "gmtoffset": "datetime.offset.utc",
        # IANA timezone (NNFT-123: targeted synonym for column name matching)
        "timezone": "datetime.offset.iana",
        "tz": "datetime.offset.iana",
        "time zone": "datetime.offset.iana",
        "iana timezone": "datetime.offset.iana",
        # Financial codes
        "cvv": "identity.payment.cvv",
        "cvc": "identity.payment.cvv",
        "security code": "identity.payment.cvv",
        "card security": "identity.payment.cvv",
        "swift": "identity.payment.swift_bic",
        "swift code": "identity.payment.swift_bic",
        "bic": "identity.payment.swift_bic",
        "bic code": "identity.payment.swift_bic",
        "swiftcode": "identity.payment.swift_bic",
        "biccode": "identity.payment.swift_bic",
        "issn": "technology.code.issn",
        # Medical
        "npi": "identity.medical.npi",
        "npi number": "identity.medical.npi",
        # Barcode
        "ean": "technology.code.ean",
        "barcode": "technology.code.ean",
        "gtin": "technology.code.ean",
        "upc": "technology.code.ean",
        # OS
        "os": "technology.development.os",
        "operating system": "technology.development.os",
        "platform": "technology.development.os",
        # Occupation
        "occupation": "identity.person.occupation",
        "job title": "identity.person.occupation",
        "jobtitle": "identity.person.occupation",
        "job": "identity.person.occupation",
        "profession": "identity.person.occupation",
        "role": "identity.person.occupation",
        "position": "identity.person.occupation",
        # Categorical
        "embarked": "representation.discrete.categorical",
        "boarded": "representation.discrete.categorical",
        "departed": "representation.discrete.categorical",
        "terminal": "representation.discrete.categorical",
        "gate": "representation.discrete.categorical",
        # Alphanumeric ID
        "ticket": "representation.code.alphanumeric_id",
        "ticket number": "representation.code.alphanumeric_id",
        "ticketno": "representation.code.alphanumeric_id",
        "cabin": "representation.code.alphanumeric_id",
        "room": "representation.code.alphanumeric_id",
        "compartment": "representation.code.alphanumeric_id",
        "berth": "representation.code.alphanumeric_id",
        "seat": "representation.code.alphanumeric_id",
        # Address
        "address": "geography.address.full_address",
        "street": "geography.address.full_address",
        "street address": "geography.address.full_address",
        # Weight/Height
        "weight": "identity.person.weight",
        "height": "identity.person.height",
        # HTTP status code (NNFT-123: targeted synonym for column name matching)
        "status code": "technology.internet.http_status_code",
        "response code": "technology.internet.http_status_code",
        "http status": "technology.internet.http_status_code",
        # MIME type (NNFT-123: targeted synonym for column name matching)
        "content type": "representation.file.mime_type",
        "media type": "representation.file.mime_type",
        "mime": "representation.file.mime_type",
    }


def build_synonym_texts(
    taxonomy: dict, header_hints: dict[str, str]
) -> dict[str, list[str]]:
    """Build synonym text lists for each type label.

    For each type, collects:
    - title field
    - aliases field
    - label components split on dots (e.g., "identity person email")
    - all header_hint entries that map to this type
    """
    synonyms: dict[str, list[str]] = {}

    for label, defn in taxonomy.items():
        if not isinstance(defn, dict):
            continue

        texts = set()

        # Title (e.g., "Email Address")
        title = defn.get("title", "")
        if title:
            texts.add(title.lower())

        # Aliases (e.g., ["email", "emailaddress"])
        aliases = defn.get("aliases", [])
        if aliases:
            for alias in aliases:
                texts.add(str(alias).lower())

        # Label components (e.g., "identity person email")
        parts = label.replace(".", " ")
        texts.add(parts)
        # Also add the leaf name alone (e.g., "email")
        leaf = label.split(".")[-1].replace("_", " ")
        texts.add(leaf)

        synonyms[label] = list(texts)

    # Add header hint entries
    for hint_text, hint_label in header_hints.items():
        if hint_label in synonyms:
            if hint_text not in synonyms[hint_label]:
                synonyms[hint_label].append(hint_text)
        else:
            # Type exists in hints but not in taxonomy — skip
            print(f"  Warning: header hint label '{hint_label}' not in taxonomy", file=sys.stderr)

    return synonyms


def farthest_point_sampling(
    vectors: np.ndarray, k: int
) -> np.ndarray:
    """Select K representative vectors via Farthest Point Sampling.

    Seed: the vector closest to the mean (most "central" representative).
    Then greedily add the vector farthest from all already-selected points
    (maximize min-distance). O(N*K), trivial for K=3 and N≈20.

    Args:
        vectors: L2-normalised embeddings [N, dim]
        k: number of representatives to select

    Returns:
        Selected representative vectors [K, dim]
    """
    n = vectors.shape[0]
    assert n >= k, f"Need at least {k} vectors, got {n}"

    # Seed: closest to mean
    mean_vec = np.mean(vectors, axis=0)
    mean_norm = np.linalg.norm(mean_vec)
    if mean_norm > 0:
        mean_vec = mean_vec / mean_norm
    dists_to_mean = 1.0 - vectors @ mean_vec  # cosine distance
    seed_idx = int(np.argmin(dists_to_mean))

    selected = [seed_idx]
    # min_dists[i] = min distance from point i to any selected point
    # Using cosine distance: 1 - cos_sim
    min_dists = 1.0 - vectors @ vectors[seed_idx]  # [N]

    for _ in range(k - 1):
        # Pick the point with the largest min-distance to selected set
        # Mask already-selected points
        min_dists_masked = min_dists.copy()
        for idx in selected:
            min_dists_masked[idx] = -1.0
        next_idx = int(np.argmax(min_dists_masked))
        selected.append(next_idx)

        # Update min distances
        new_dists = 1.0 - vectors @ vectors[next_idx]
        min_dists = np.minimum(min_dists, new_dists)

    return vectors[selected]  # [K, dim]


def embed_type_labels(
    model: StaticModel, synonyms: dict[str, list[str]], max_k: int = 1
) -> tuple[np.ndarray, list[str]]:
    """Compute type label embeddings.

    When max_k=1 (legacy mode): mean-pools all synonym embeddings into one
    centroid per type. Returns [N_types, dim].

    When max_k>1 (max-sim mode): selects K representative embeddings per type
    via Farthest Point Sampling. Returns [N_types * K, dim] with interleaved
    layout: rows 0..K are type 0's representatives, K..2K are type 1's, etc.
    Types with fewer than K synonyms are zero-padded.

    Returns (embeddings [N_types * K, dim], label_index [N_types]).
    """
    label_index = sorted(synonyms.keys())
    dim = None
    all_type_embeddings = []

    for label in label_index:
        texts = synonyms[label]
        if not texts:
            # Fallback: use the label itself
            texts = [label.replace(".", " ")]

        # Embed all synonym texts and L2-normalise each
        vecs = model.encode(texts)  # [n_texts, dim]
        norms = np.linalg.norm(vecs, axis=1, keepdims=True)
        norms = np.maximum(norms, 1e-8)
        vecs = vecs / norms  # L2-normalised [n_texts, dim]

        if dim is None:
            dim = vecs.shape[1]

        if max_k == 1:
            # Legacy: mean pool and L2 normalize
            mean_vec = np.mean(vecs, axis=0)
            norm = np.linalg.norm(mean_vec)
            if norm > 0:
                mean_vec = mean_vec / norm
            all_type_embeddings.append(mean_vec.reshape(1, dim))
        else:
            # Max-sim: select K representatives
            n_synonyms = vecs.shape[0]
            if n_synonyms <= max_k:
                # Use all available, zero-pad the rest
                reps = np.zeros((max_k, dim), dtype=np.float32)
                reps[:n_synonyms] = vecs
            else:
                # Farthest Point Sampling
                reps = farthest_point_sampling(vecs, max_k)
            all_type_embeddings.append(reps)

    return np.concatenate(all_type_embeddings, axis=0).astype(np.float32), label_index


def main():
    parser = argparse.ArgumentParser(description="Prepare Model2Vec for FineType")
    parser.add_argument(
        "--model",
        default="minishlab/potion-base-4M",
        help="HuggingFace model ID for pre-distilled Model2Vec",
    )
    parser.add_argument(
        "--output",
        default=None,
        help="Output directory (default: models/model2vec)",
    )
    parser.add_argument(
        "--max-k",
        type=int,
        default=3,
        help="Number of representative embeddings per type (default: 3). "
        "K=1 is equivalent to mean-pool (legacy behavior).",
    )
    parser.add_argument(
        "--legacy",
        action="store_true",
        help="Force K=1 mean-pool (reproduces old single-centroid behavior)",
    )
    args = parser.parse_args()

    max_k = 1 if args.legacy else args.max_k
    if max_k < 1:
        parser.error("--max-k must be >= 1")

    # Resolve paths relative to workspace root
    script_dir = Path(__file__).parent
    workspace_root = script_dir.parent
    labels_dir = workspace_root / "labels"
    output_dir = Path(args.output) if args.output else workspace_root / "models" / "model2vec"

    print(f"Loading Model2Vec: {args.model}")
    model = StaticModel.from_pretrained(args.model)

    print(f"Reading taxonomy from {labels_dir}")
    taxonomy = load_taxonomy(labels_dir)
    print(f"  Found {len(taxonomy)} type definitions")

    print("Building synonym texts from taxonomy + header hints")
    header_hints = build_header_hint_entries()
    synonyms = build_synonym_texts(taxonomy, header_hints)
    total_synonyms = sum(len(v) for v in synonyms.values())
    print(f"  {len(synonyms)} types, {total_synonyms} total synonym texts")

    print(f"Computing type label embeddings (K={max_k})")
    type_embeddings, label_index = embed_type_labels(model, synonyms, max_k=max_k)
    print(f"  Shape: {type_embeddings.shape} ({'max-sim' if max_k > 1 else 'legacy mean-pool'})")
    if max_k > 1:
        print(f"  {len(label_index)} types × {max_k} representatives = {len(label_index) * max_k} rows")

    # Save outputs
    output_dir.mkdir(parents=True, exist_ok=True)

    # 1. Copy tokenizer from the model
    tokenizer_path = output_dir / "tokenizer.json"
    tokenizer_json = model.tokenizer.to_str()
    tokenizer_path.write_text(tokenizer_json)
    print(f"  Saved tokenizer.json ({tokenizer_path.stat().st_size / 1024:.1f} KB)")

    # 2. Save token embedding matrix as float16 to reduce binary size
    # (~7.5MB vs ~15MB for float32). Cosine similarity is robust to fp16.
    # model.embedding is already a numpy array in model2vec >= 0.7
    emb = model.embedding
    token_embeddings = np.asarray(emb, dtype=np.float16)
    model_path = output_dir / "model.safetensors"
    save_file({"embeddings": token_embeddings}, str(model_path))
    print(
        f"  Saved model.safetensors ({model_path.stat().st_size / 1024:.1f} KB, "
        f"shape {token_embeddings.shape}, dtype=float16)"
    )

    # 3. Save type embeddings
    type_emb_path = output_dir / "type_embeddings.safetensors"
    save_file({"type_embeddings": type_embeddings}, str(type_emb_path))
    print(
        f"  Saved type_embeddings.safetensors ({type_emb_path.stat().st_size / 1024:.1f} KB, "
        f"shape {type_embeddings.shape})"
    )

    # 4. Save label index
    label_path = output_dir / "label_index.json"
    label_path.write_text(json.dumps(label_index, indent=2) + "\n")
    print(f"  Saved label_index.json ({len(label_index)} labels)")

    # Quick verification — uses max-sim matching when K > 1
    print("\nVerification — test embeddings:")
    test_names = ["email", "zip_code", "latitude", "phone_number", "first_name", "created_at"]
    n_types = len(label_index)
    for name in test_names:
        # Normalize like our Rust code will
        normalized = name.lower().replace("_", " ").replace("-", " ").strip()
        query_vec = model.encode([normalized])  # [1, dim]
        query_vec = query_vec / np.linalg.norm(query_vec, axis=1, keepdims=True)
        all_sims = type_embeddings @ query_vec.T  # [N_types * K, 1]
        all_sims = all_sims.reshape(n_types, max_k)  # [N_types, K]
        max_sims = np.max(all_sims, axis=1)  # [N_types] — best rep per type
        best_type = np.argmax(max_sims)
        best_sim = max_sims[best_type]
        print(f"  '{name}' -> {label_index[best_type]} (sim={best_sim:.3f})")

    print(f"\nDone! Output in {output_dir}")


if __name__ == "__main__":
    main()
