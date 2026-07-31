#!/usr/bin/env python3
"""ac-02 — Generate synthetic `identity.person.full_name` training
columns from Wikidata Q5 name primitives.

Reads `$HOME/datasets/wikidata/<date>/{given_names,family_names,persons}.tsv`
(produced by scripts/fetch_wikidata_persons.sh) and emits synthetic
person-name columns in the sherlock_distilled schema. These are the
synthetic hard negatives against `geography.location.city` — values
that share city-like shape (capitalised word(s), 1-3 tokens, no digits)
but are labelled `identity.person.full_name`.

The generator combines `given × family` for the synthetic core and
mixes in real Q5 labels from `persons.tsv` for distributional realism
(some labels carry honorifics, multilingual transliterations, or
mononyms that pure combination can't synthesise).

Each output column applies one noise recipe drawn from a
per-recipe-weight distribution, mirroring the v21 GeoNames generator's
design (scripts/generate_geonames_geography.py). Recipes:

  canonical                "John Smith"                          weight 0.40
  last_first               "Smith, John"                         weight 0.12
  first_initial_last       "J. Smith"                            weight 0.08
  with_middle              "John A. Smith"                       weight 0.07
  with_suffix              "John Smith Jr." / "III"              weight 0.04
  with_title               "Dr. John Smith"                      weight 0.04
  upper                    "JOHN SMITH"                          weight 0.03
  lower                    "john smith"                          weight 0.03
  given_only               "John"                                weight 0.05
  family_only              "Smith"                               weight 0.05
  whitespace_noise         "  John  Smith  "                     weight 0.03
  real_q5_sample           draw from persons.tsv labels directly weight 0.06

Output: output/distillation-v22/wikidata_persons.csv.gz
"""
from __future__ import annotations

import argparse
import csv
import gzip
import json
import random
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_SOURCE = Path("/Users/hugh/datasets/wikidata")
DEFAULT_OUT = REPO / "output/distillation-v22/wikidata_persons.csv.gz"
DEFAULT_MANIFEST = REPO / "output/distillation-v22/wikidata_persons_manifest.json"

RECIPE_WEIGHTS = {
    "canonical": 0.40,
    "last_first": 0.12,
    "first_initial_last": 0.08,
    "with_middle": 0.07,
    "with_suffix": 0.04,
    "with_title": 0.04,
    "upper": 0.03,
    "lower": 0.03,
    "given_only": 0.05,
    "family_only": 0.05,
    "whitespace_noise": 0.03,
    "real_q5_sample": 0.06,
}

TITLES_EN = ["Dr.", "Mr.", "Mrs.", "Ms.", "Prof.", "Sir", "Lady"]
SUFFIXES_EN = ["Jr.", "Sr.", "II", "III", "IV", "PhD", "MD", "Esq."]
MIDDLE_INITIALS = list("ABCDEFGHIJKLMNOPQRSTUVWXYZ")


def load_names(path: Path) -> list[str]:
    if not path.exists():
        return []
    names: list[str] = []
    with path.open(encoding="utf-8") as f:
        reader = csv.DictReader(f, delimiter="\t")
        for row in reader:
            lab = (row.get("label") or "").strip()
            if not lab:
                continue
            # Drop names that are obviously codes or numeric: keep names
            # that contain at least one alpha character.
            if not any(c.isalpha() for c in lab):
                continue
            names.append(lab)
    return names


def recipe_canonical(given: str, family: str, rng: random.Random) -> str:
    return f"{given} {family}"


def recipe_last_first(given: str, family: str, rng: random.Random) -> str:
    return f"{family}, {given}"


def recipe_first_initial_last(given: str, family: str, rng: random.Random) -> str:
    init = given[0] if given else ""
    return f"{init}. {family}"


def recipe_with_middle(given: str, family: str, rng: random.Random) -> str:
    mi = rng.choice(MIDDLE_INITIALS)
    return f"{given} {mi}. {family}"


def recipe_with_suffix(given: str, family: str, rng: random.Random) -> str:
    suf = rng.choice(SUFFIXES_EN)
    return f"{given} {family} {suf}"


def recipe_with_title(given: str, family: str, rng: random.Random) -> str:
    title = rng.choice(TITLES_EN)
    return f"{title} {given} {family}"


def recipe_upper(given: str, family: str, rng: random.Random) -> str:
    return f"{given} {family}".upper()


def recipe_lower(given: str, family: str, rng: random.Random) -> str:
    return f"{given} {family}".lower()


def recipe_given_only(given: str, family: str, rng: random.Random) -> str:
    return given


def recipe_family_only(given: str, family: str, rng: random.Random) -> str:
    return family


def recipe_whitespace_noise(given: str, family: str, rng: random.Random) -> str:
    return f"  {given}   {family}  "


def recipe_real_q5_sample(given: str, family: str, rng: random.Random) -> str:
    # Placeholder — the real-q5 recipe is handled directly in the
    # column-build loop using the persons.tsv pool.
    return f"{given} {family}"


RECIPE_FNS = {
    "canonical": recipe_canonical,
    "last_first": recipe_last_first,
    "first_initial_last": recipe_first_initial_last,
    "with_middle": recipe_with_middle,
    "with_suffix": recipe_with_suffix,
    "with_title": recipe_with_title,
    "upper": recipe_upper,
    "lower": recipe_lower,
    "given_only": recipe_given_only,
    "family_only": recipe_family_only,
    "whitespace_noise": recipe_whitespace_noise,
    "real_q5_sample": recipe_real_q5_sample,
}


HEADER_POOL = [
    "name", "Name", "full_name", "Full Name", "person", "Person",
    "Author", "author", "owner", "Owner", "customer", "Customer",
    "contact", "Contact", "recipient", "user", "User", "username",
    "first_last", "Name (Last, First)", "Display Name", "Cardholder",
    "Employee", "employee_name", "Manager", "applicant", "Patient",
    "Reporter", "Witness", "submitted_by", "approved_by", "reviewer",
    "Defendant", "Plaintiff", "Speaker", "presenter", "moderator",
]


def make_column(
    givens: list[str],
    families: list[str],
    real_q5: list[str],
    n_values: int,
    rng: random.Random,
) -> tuple[list[str], str, str]:
    """Build one synthetic person-name column.

    Returns (values, header, recipe_label).
    Header is drawn from HEADER_POOL. Recipe is the dominant recipe for
    that column (helps the trainer learn invariance to header form).
    """
    recipes = list(RECIPE_WEIGHTS.keys())
    weights = [RECIPE_WEIGHTS[r] for r in recipes]
    recipe = rng.choices(recipes, weights=weights, k=1)[0]
    fn = RECIPE_FNS[recipe]
    values: list[str] = []
    for _ in range(n_values):
        if recipe == "real_q5_sample" and real_q5:
            values.append(rng.choice(real_q5))
        else:
            g = rng.choice(givens)
            f = rng.choice(families)
            values.append(fn(g, f, rng))
    header = rng.choice(HEADER_POOL)
    return values, header, recipe


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--source", type=Path, default=DEFAULT_SOURCE,
                   help="Root of the wikidata dataset tree; the most-recent dated subdir is used.")
    p.add_argument("--date", type=str, default=None,
                   help="Explicit dated subdir under --source (default: most recent).")
    p.add_argument("--out", type=Path, default=DEFAULT_OUT)
    p.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    p.add_argument("--n-columns", type=int, default=25000,
                   help="Number of synthetic columns to emit. The v22 blend audit gate "
                        "needs ≥5,000 full_name rows AND a ≥170,000 total — wikidata "
                        "is the main lever for the 170k floor since v19+v21+ac-02+ac-03 "
                        "sum to ~147k.")
    p.add_argument("--values-per-column", type=int, default=40)
    p.add_argument("--seed", type=int, default=42)
    args = p.parse_args()

    if args.date:
        src_dir = args.source / args.date
    else:
        # Pick most-recent dated subdir.
        candidates = sorted(
            (d for d in args.source.iterdir() if d.is_dir()),
            reverse=True,
        )
        if not candidates:
            print(f"error: no dated subdir under {args.source}",
                  file=sys.stderr)
            return 2
        src_dir = candidates[0]

    print(f"reading primitives from {src_dir}...", file=sys.stderr)
    givens = load_names(src_dir / "given_names.tsv")
    families = load_names(src_dir / "family_names.tsv")
    real_q5 = load_names(src_dir / "persons.tsv")
    print(f"  given_names:  {len(givens):,}", file=sys.stderr)
    print(f"  family_names: {len(families):,}", file=sys.stderr)
    print(f"  persons:      {len(real_q5):,}", file=sys.stderr)

    if len(givens) < 100 or len(families) < 100:
        print("error: too few name primitives — fetch incomplete?",
              file=sys.stderr)
        return 2

    rng = random.Random(args.seed)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    per_recipe_count: dict[str, int] = defaultdict(int)
    per_header_count: dict[str, int] = defaultdict(int)
    n_written = 0
    total_values = 0
    LABEL = "identity.person.full_name"

    with gzip.open(args.out, "wt", newline="", encoding="utf-8") as fout:
        writer = csv.writer(fout)
        writer.writerow(["final_label", "sample_values", "column_name"])
        for _ in range(args.n_columns):
            n_vals = args.values_per_column + rng.randint(-10, 30)
            n_vals = max(8, n_vals)
            values, header, recipe = make_column(
                givens, families, real_q5, n_vals, rng,
            )
            writer.writerow([
                LABEL,
                json.dumps(values, ensure_ascii=False),
                header,
            ])
            per_recipe_count[recipe] += 1
            per_header_count[header] += 1
            n_written += 1
            total_values += len(values)

    manifest = {
        "source_dir": str(src_dir),
        "n_columns_written": n_written,
        "total_values": total_values,
        "per_recipe_count": dict(sorted(per_recipe_count.items())),
        "per_header_count": dict(sorted(per_header_count.items())),
        "config": {
            "values_per_column_target": args.values_per_column,
            "seed": args.seed,
            "n_given_names": len(givens),
            "n_family_names": len(families),
            "n_persons_sample": len(real_q5),
        },
    }
    args.manifest.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"wrote {n_written:,} columns ({total_values:,} values) to {args.out}",
          file=sys.stderr)
    print(f"manifest: {args.manifest}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
