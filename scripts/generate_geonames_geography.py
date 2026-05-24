#!/usr/bin/env python3
"""Generate per-type, per-locale geography training columns from a
registered GeoNames snapshot.

Per spec 2026-05-24-v21-geonames-geography ac-02 + ac-04.

Reads:
  - eval/datasets/sources.yaml (entry: geonames) → local_path + snapshot
  - eval/geonames/{type_mapping,locale_targets,header_pools,noise_recipes}.yaml

Writes:
  - output/distillation-v21-geonames/geonames_geography.csv.gz
      sherlock_distilled schema — `final_label`, `sample_values`
      (JSON-encoded list), `column_name`. Drop-in for
      `prepare_multibranch_data.py --distilled`.
  - output/distillation-v21-geonames/manifest.json
      per (type, locale) column counts, snapshot reference, seed.

Usage
-----
  source eval/gittables/.venv/bin/activate    # (only for pyyaml; no other deps)
  python3 scripts/generate_geonames_geography.py
  python3 scripts/generate_geonames_geography.py --seed 42
  python3 scripts/generate_geonames_geography.py --columns-per-type-per-locale 200
  python3 scripts/generate_geonames_geography.py --values-per-column 100

Determinism: same --seed produces byte-identical output. Re-run is
idempotent over output/distillation-v21-geonames/.
"""
from __future__ import annotations

import argparse
import csv
import gzip
import json
import random
import re
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SOURCES_YAML = REPO / "eval" / "datasets" / "sources.yaml"
CFG_DIR = REPO / "eval" / "geonames"
OUT_DIR = REPO / "output" / "distillation-v21-geonames"

SAMPLE_DELIM = "│"  # match the corpus pass column writer (gittables_corpus_pass.py)

# ────────────────────────────────────────────────────────────────────
#  Config loading
# ────────────────────────────────────────────────────────────────────

def load_yaml(path: Path) -> dict:
    import yaml
    with path.open() as f:
        return yaml.safe_load(f) or {}


def resolve_geonames_root() -> Path:
    """Look up the registered geonames local_path from sources.yaml."""
    data = load_yaml(SOURCES_YAML)
    for entry in data.get("sources") or []:
        if "geonames" in (entry.get("datasets") or []):
            local = entry.get("local_path")
            if not local:
                sys.exit("error: geonames entry in sources.yaml has no local_path")
            return Path(local)
    sys.exit("error: no geonames entry in sources.yaml (run scripts/fetch_geonames.sh first)")


# ────────────────────────────────────────────────────────────────────
#  GeoNames source loaders
# ────────────────────────────────────────────────────────────────────

@dataclass
class CityRecord:
    geonameid: int
    name: str
    asciiname: str
    country_code: str
    admin1_code: str
    population: int
    latitude: float
    longitude: float


@dataclass
class Admin1Record:
    code: str           # e.g. "US.CA"
    country_code: str   # e.g. "US"
    name: str
    geonameid: int


@dataclass
class CountryRecord:
    iso2: str
    iso3: str
    country_name: str
    continent_code: str
    geonameid: int


def load_cities(path: Path) -> list[CityRecord]:
    """Load cities500.txt entirely (~250k records, ~38MB)."""
    out: list[CityRecord] = []
    with path.open("r", encoding="utf-8") as f:
        for line in f:
            parts = line.rstrip("\n").split("\t")
            if len(parts) < 15:
                continue
            if parts[6] != "P":  # feature_class P = populated place
                continue
            try:
                pop = int(parts[14] or 0)
            except ValueError:
                pop = 0
            try:
                lat = float(parts[4])
                lon = float(parts[5])
            except ValueError:
                continue
            out.append(CityRecord(
                geonameid=int(parts[0]),
                name=parts[1],
                asciiname=parts[2],
                country_code=parts[8],
                admin1_code=parts[10],
                population=pop,
                latitude=lat,
                longitude=lon,
            ))
    return out


def load_admin1(path: Path) -> list[Admin1Record]:
    """admin1CodesASCII.txt: code, name, asciiname, geonameid."""
    out: list[Admin1Record] = []
    with path.open("r", encoding="utf-8") as f:
        for line in f:
            parts = line.rstrip("\n").split("\t")
            if len(parts) < 4:
                continue
            code = parts[0]
            country_code = code.split(".")[0] if "." in code else ""
            try:
                gid = int(parts[3])
            except ValueError:
                continue
            out.append(Admin1Record(code=code, country_code=country_code, name=parts[1], geonameid=gid))
    return out


def load_countries(path: Path) -> list[CountryRecord]:
    """countryInfo.txt: skip '#' comments. 19 tab-separated fields."""
    out: list[CountryRecord] = []
    with path.open("r", encoding="utf-8") as f:
        for line in f:
            if line.startswith("#") or not line.strip():
                continue
            parts = line.rstrip("\n").split("\t")
            if len(parts) < 17:
                continue
            try:
                gid = int(parts[16]) if parts[16] else 0
            except ValueError:
                gid = 0
            out.append(CountryRecord(
                iso2=parts[0],
                iso3=parts[1],
                country_name=parts[4],
                continent_code=parts[8],
                geonameid=gid,
            ))
    return out


def load_postal_by_country(path: Path, country_codes: set[str], per_country_cap: int = 5000
                           ) -> dict[str, list[str]]:
    """Stream postal/allCountries.txt. Schema:
    country_code, postal_code, place_name, admin1_name, admin1_code,
    admin2_name, admin2_code, admin3_name, admin3_code, lat, lon, accuracy"""
    out: dict[str, list[str]] = defaultdict(list)
    with path.open("r", encoding="utf-8") as f:
        for line in f:
            parts = line.rstrip("\n").split("\t")
            if len(parts) < 2:
                continue
            cc = parts[0]
            if cc not in country_codes:
                continue
            if len(out[cc]) >= per_country_cap:
                continue
            pcode = parts[1].strip()
            if pcode:
                out[cc].append(pcode)
    return dict(out)


def load_alternate_names(
    path: Path,
    geonameids: set[int],
    isolanguages: set[str],
    per_geonameid_cap: int = 8,
) -> dict[int, dict[str, list[str]]]:
    """Stream alternateNamesV2.txt:
    alternateNameId, geonameid, isolanguage, alternate_name, isPreferredName,
    isShortName, isColloquial, isHistoric, from, to

    Returns geonameid -> isolanguage -> list of names.
    """
    out: dict[int, dict[str, list[str]]] = defaultdict(lambda: defaultdict(list))
    with path.open("r", encoding="utf-8") as f:
        for line in f:
            parts = line.rstrip("\n").split("\t")
            if len(parts) < 4:
                continue
            try:
                gid = int(parts[1])
            except ValueError:
                continue
            if gid not in geonameids:
                continue
            lang = parts[2]
            if lang not in isolanguages:
                continue
            name = parts[3].strip()
            if not name:
                continue
            bucket = out[gid][lang]
            if len(bucket) >= per_geonameid_cap:
                continue
            bucket.append(name)
    # Convert defaultdicts to plain dicts for safer downstream use
    return {gid: dict(langs) for gid, langs in out.items()}


# ────────────────────────────────────────────────────────────────────
#  Noise recipes
# ────────────────────────────────────────────────────────────────────

US_STATE_BY_ADMIN1 = {
    # ISO/USPS code for each admin1 in the US — for the city_state recipe.
    "AL": "AL", "AK": "AK", "AZ": "AZ", "AR": "AR", "CA": "CA", "CO": "CO",
    "CT": "CT", "DE": "DE", "FL": "FL", "GA": "GA", "HI": "HI", "ID": "ID",
    "IL": "IL", "IN": "IN", "IA": "IA", "KS": "KS", "KY": "KY", "LA": "LA",
    "ME": "ME", "MD": "MD", "MA": "MA", "MI": "MI", "MN": "MN", "MS": "MS",
    "MO": "MO", "MT": "MT", "NE": "NE", "NV": "NV", "NH": "NH", "NJ": "NJ",
    "NM": "NM", "NY": "NY", "NC": "NC", "ND": "ND", "OH": "OH", "OK": "OK",
    "OR": "OR", "PA": "PA", "RI": "RI", "SC": "SC", "SD": "SD", "TN": "TN",
    "TX": "TX", "UT": "UT", "VT": "VT", "VA": "VA", "WA": "WA", "WV": "WV",
    "WI": "WI", "WY": "WY",
}

ABBREVIATIONS = {
    "New York": "NYC", "Los Angeles": "L.A.", "San Francisco": "S.F.",
    "Washington": "D.C.", "Chicago": "Chi", "Philadelphia": "Philly",
    "London": "Ldn", "Tokyo": "TYO", "Paris": "Prs",
}


def noise_canonical(values: list[str], rng: random.Random) -> list[str]:
    return list(values)


def noise_mixed_case(values: list[str], rng: random.Random) -> list[str]:
    out = []
    for v in values:
        choice = rng.choice(("lower", "upper", "title", "as_is"))
        if choice == "lower":
            out.append(v.lower())
        elif choice == "upper":
            out.append(v.upper())
        elif choice == "title":
            out.append(v.title())
        else:
            out.append(v)
    return out


def noise_abbreviated(values: list[str], rng: random.Random) -> list[str]:
    out = []
    for v in values:
        out.append(ABBREVIATIONS.get(v, v))
    return out


def noise_city_state(values: list[str], rng: random.Random, *,
                     city_recs: list[CityRecord] | None = None) -> list[str]:
    if not city_recs:
        return list(values)
    out = []
    for i, v in enumerate(values):
        rec = city_recs[i] if i < len(city_recs) else None
        if rec and rec.country_code == "US" and rec.admin1_code in US_STATE_BY_ADMIN1:
            out.append(f"{v}, {US_STATE_BY_ADMIN1[rec.admin1_code]}")
        else:
            out.append(v)
    return out


def noise_city_country(values: list[str], rng: random.Random, *,
                       city_recs: list[CityRecord] | None = None,
                       country_name_by_cc: dict[str, str] | None = None) -> list[str]:
    if not city_recs or not country_name_by_cc:
        return list(values)
    out = []
    for i, v in enumerate(values):
        rec = city_recs[i] if i < len(city_recs) else None
        cname = country_name_by_cc.get(rec.country_code) if rec else None
        if cname:
            out.append(f"{v} ({cname})")
        else:
            out.append(v)
    return out


def noise_partial_value(values: list[str], rng: random.Random) -> list[str]:
    out = []
    for v in values:
        cut = max(1, int(len(v) * 0.6))
        out.append(v[:cut])
    return out


def noise_whitespace_noise(values: list[str], rng: random.Random) -> list[str]:
    out = []
    for v in values:
        ws = rng.choice(("  ", "\t", " ", "  ", " "))
        side = rng.choice(("leading", "trailing", "internal_double"))
        if side == "leading":
            out.append(ws + v)
        elif side == "trailing":
            out.append(v + ws)
        else:
            out.append(re.sub(r" ", "  ", v, count=1))
    return out


def noise_empty_interspersed(values: list[str], rng: random.Random) -> list[str]:
    out = []
    for v in values:
        roll = rng.random()
        if roll < 0.05:
            out.append("")
        elif roll < 0.10:
            out.append("NULL")
        else:
            out.append(v)
    return out


def noise_mixed_language(values: list[str], rng: random.Random, *,
                         alt_names_for: dict[int, list[str]] | None = None,
                         city_recs: list[CityRecord] | None = None) -> list[str]:
    if not alt_names_for or not city_recs:
        return list(values)
    out = []
    for i, v in enumerate(values):
        if rng.random() < 0.30 and i < len(city_recs):
            rec = city_recs[i]
            alts = alt_names_for.get(rec.geonameid) or []
            if alts:
                out.append(rng.choice(alts))
                continue
        out.append(v)
    return out


# Postal-code-specific recipes
def noise_postal_with_space(values: list[str], rng: random.Random, *, country_code: str | None = None) -> list[str]:
    out = []
    for v in values:
        if " " not in v and len(v) > 4:
            mid = len(v) // 2
            out.append(v[:mid] + " " + v[mid:])
        else:
            out.append(v)
    return out


def noise_postal_with_hyphen(values: list[str], rng: random.Random, *, country_code: str | None = None) -> list[str]:
    out = []
    for v in values:
        digits = re.sub(r"\D", "", v)
        if len(digits) >= 5 and country_code == "US":
            extra = "".join(rng.choice("0123456789") for _ in range(4))
            out.append(f"{digits[:5]}-{extra}")
        else:
            out.append(v)
    return out


def noise_postal_country_prefixed(values: list[str], rng: random.Random, *, country_code: str | None = None) -> list[str]:
    if not country_code:
        return list(values)
    return [f"{country_code}-{v}" for v in values]


# Coordinate recipes
def noise_decimal_degrees(values: list[str], rng: random.Random) -> list[str]:
    return [f"{float(v):.4f}" for v in values]


def noise_decimal_degrees_short(values: list[str], rng: random.Random) -> list[str]:
    return [f"{float(v):.2f}" for v in values]


def noise_signed_decimal(values: list[str], rng: random.Random) -> list[str]:
    return [f"{float(v):+.4f}" for v in values]


def noise_dms_notation(values: list[str], rng: random.Random, *, is_latitude: bool = True) -> list[str]:
    out = []
    for v in values:
        f = float(v)
        hemi = ("N" if f >= 0 else "S") if is_latitude else ("E" if f >= 0 else "W")
        f = abs(f)
        d = int(f)
        m = int((f - d) * 60)
        s = ((f - d) * 60 - m) * 60
        out.append(f"{d}°{m:02d}'{s:04.1f}\" {hemi}")
    return out


def noise_csv_pair(values: list[str], rng: random.Random) -> list[str]:
    """Pair adjacent values as 'lat, lon'-style strings.
    Less semantically meaningful but mimics real-world packed coordinates."""
    out = []
    for v in values:
        other = rng.uniform(-180, 180)
        out.append(f"{float(v):.4f}, {other:.4f}")
    return out


def noise_with_full_name(values: list[str], rng: random.Random) -> list[str]:
    """Continent code → 'EU (Europe)' style."""
    expansion = {
        "AF": "Africa", "AN": "Antarctica", "AS": "Asia", "EU": "Europe",
        "NA": "North America", "OC": "Oceania", "SA": "South America",
    }
    return [f"{v} ({expansion[v]})" if v in expansion else v for v in values]


NOISE_FNS = {
    "canonical":             noise_canonical,
    "mixed_case":            noise_mixed_case,
    "abbreviated":           noise_abbreviated,
    "city_state":            noise_city_state,
    "city_country":          noise_city_country,
    "partial_value":         noise_partial_value,
    "whitespace_noise":      noise_whitespace_noise,
    "empty_interspersed":    noise_empty_interspersed,
    "mixed_language":        noise_mixed_language,
    "with_space":            noise_postal_with_space,
    "with_hyphen":           noise_postal_with_hyphen,
    "country_prefixed":     noise_postal_country_prefixed,
    "decimal_degrees":       noise_decimal_degrees,
    "decimal_degrees_short": noise_decimal_degrees_short,
    "signed_decimal":        noise_signed_decimal,
    "dms_notation":          noise_dms_notation,
    "csv_pair":              noise_csv_pair,
    "with_full_name":        noise_with_full_name,
}


def pick_recipe(recipes: dict[str, float], rng: random.Random) -> str:
    """Sample a recipe name weighted by its config rate."""
    items = list(recipes.items())
    total = sum(w for _, w in items)
    target = rng.random() * total
    cum = 0.0
    for name, w in items:
        cum += w
        if target <= cum:
            return name
    return items[-1][0]


# ────────────────────────────────────────────────────────────────────
#  Column generation
# ────────────────────────────────────────────────────────────────────

@dataclass
class ColumnRow:
    final_label: str
    sample_values: list[str]
    column_name: str
    locale: str = ""        # for the manifest, not the CSV
    recipe: str = ""        # for the manifest


@dataclass
class GenContext:
    """Shared per-locale + per-type generation state."""
    cities_by_country: dict[str, list[CityRecord]]   # cities500 grouped by cc
    admin1_by_country: dict[str, list[Admin1Record]]
    countries: list[CountryRecord]
    country_name_by_cc: dict[str, str]
    postal_by_country: dict[str, list[str]]
    alt_names: dict[int, dict[str, list[str]]]
    header_pools: dict
    noise_recipes: dict
    rng: random.Random


def sample_values_for_type(
    type_key: str,
    type_cfg: dict,
    locale_cfg: dict,
    ctx: GenContext,
    n_values: int,
) -> tuple[list[str], list[CityRecord] | None]:
    """Pick the canonical-value list (and optional CityRecord parallel list
    for recipe helpers that need lat/state lookup)."""
    rng = ctx.rng
    countries = locale_cfg["country_codes"]
    source = type_cfg["source"]

    if source == "cities500.txt":
        pool: list[CityRecord] = []
        for cc in countries:
            pool.extend(ctx.cities_by_country.get(cc, []))
        if not pool:
            return [], None
        sampled = rng.choices(pool, k=n_values) if len(pool) < n_values else rng.sample(pool, n_values)
        field = type_cfg.get("value_field", "col:2")
        if field == "col:2":
            vals = [c.name for c in sampled]
        elif field == "col:5":
            vals = [f"{c.latitude:.4f}" for c in sampled]
        elif field == "col:6":
            vals = [f"{c.longitude:.4f}" for c in sampled]
        else:
            vals = [c.name for c in sampled]
        return vals, sampled

    if source == "admin1CodesASCII.txt":
        pool_a: list[Admin1Record] = []
        for cc in countries:
            pool_a.extend(ctx.admin1_by_country.get(cc, []))
        if not pool_a:
            return [], None
        sampled_a = (rng.choices(pool_a, k=n_values) if len(pool_a) < n_values
                     else rng.sample(pool_a, n_values))
        return [a.name for a in sampled_a], None

    if source == "admin2Codes.txt":
        # Schema: code, name, asciiname, geonameid.  Codes like "US.CA.001".
        # We didn't pre-index admin2 by country to save memory; load on demand.
        # For v21 v1 this is good-enough.
        return [], None

    if source == "countryInfo.txt":
        field = type_cfg.get("value_field", "country_name")
        if field == "country_name":
            pool_c = [c for c in ctx.countries]
        else:
            pool_c = ctx.countries
        sampled_c = (rng.choices(pool_c, k=n_values) if len(pool_c) < n_values
                     else rng.sample(pool_c, n_values))
        if field == "country_name":
            vals = [c.country_name for c in sampled_c]
        elif field == "iso2":
            vals = [c.iso2 for c in sampled_c]
        elif field == "iso3":
            vals = [c.iso3 for c in sampled_c]
        elif field == "continent_code":
            vals = [c.continent_code for c in sampled_c]
        else:
            vals = [c.country_name for c in sampled_c]
        return vals, None

    if source.startswith("postal/"):
        pool_p: list[str] = []
        for cc in countries:
            pool_p.extend(ctx.postal_by_country.get(cc, []))
        if not pool_p:
            return [], None
        sampled_p = (rng.choices(pool_p, k=n_values) if len(pool_p) < n_values
                     else rng.sample(pool_p, n_values))
        return sampled_p, None

    return [], None


def apply_localization(values: list[str], city_recs: list[CityRecord] | None,
                       isolanguages: list[str], ctx: GenContext) -> list[str]:
    """For city/region/country types where localize: true — substitute a
    localized alternate name when available."""
    if not city_recs:
        return values
    out = []
    for i, v in enumerate(values):
        rec = city_recs[i]
        langs = ctx.alt_names.get(rec.geonameid, {})
        picks = []
        for lang in isolanguages:
            picks.extend(langs.get(lang, []))
        if picks and ctx.rng.random() < 0.7:
            out.append(ctx.rng.choice(picks))
        else:
            out.append(v)
    return out


def apply_noise(values: list[str], recipe: str, ctx: GenContext,
                city_recs: list[CityRecord] | None, country_code: str | None,
                is_latitude: bool | None = None) -> list[str]:
    fn = NOISE_FNS.get(recipe)
    if fn is None:
        return values
    sig_kwargs = {}
    # Bind helpers based on recipe — passes only what the recipe expects.
    if recipe == "city_state":
        sig_kwargs["city_recs"] = city_recs
    elif recipe == "city_country":
        sig_kwargs["city_recs"] = city_recs
        sig_kwargs["country_name_by_cc"] = ctx.country_name_by_cc
    elif recipe == "mixed_language":
        sig_kwargs["alt_names_for"] = {
            gid: [name for langs in v.values() for name in langs]
            for gid, v in ctx.alt_names.items()
        } if ctx.alt_names else None
        sig_kwargs["city_recs"] = city_recs
    elif recipe in ("with_space", "with_hyphen", "country_prefixed"):
        sig_kwargs["country_code"] = country_code
    elif recipe == "dms_notation":
        sig_kwargs["is_latitude"] = bool(is_latitude)
    return fn(values, ctx.rng, **sig_kwargs)


def generate_columns_for(
    type_key: str,
    type_cfg: dict,
    locale: str,
    locale_cfg: dict,
    n_columns: int,
    values_per_column: int,
    ctx: GenContext,
) -> list[ColumnRow]:
    out: list[ColumnRow] = []
    recipes_for_type = (
        ctx.noise_recipes.get("per_type", {}).get(type_key)
        or ctx.noise_recipes.get("default", {})
    )
    pool_key = locale_cfg["header_pool"]
    type_aliased = type_cfg.get("finetype_alias", type_key)
    headers_for_type = (
        ctx.header_pools.get("pools", {}).get(pool_key, {}).get(type_aliased)
        or ctx.header_pools.get("pools", {}).get("english", {}).get(type_aliased)
        or [type_aliased.split(".")[-1]]
    )
    is_latitude = "latitude" in type_key
    localize = type_cfg.get("localize", False)
    isolanguages = locale_cfg.get("isolanguages", [])

    for _ in range(n_columns):
        values, city_recs = sample_values_for_type(
            type_key, type_cfg, locale_cfg, ctx, values_per_column
        )
        if not values:
            continue
        if localize:
            values = apply_localization(values, city_recs, isolanguages, ctx)
        recipe = pick_recipe(recipes_for_type, ctx.rng)
        # For postal_code recipes that need country context: pick the first country
        # in the locale's list deterministically (per column).
        cc = locale_cfg["country_codes"][0] if locale_cfg.get("country_codes") else None
        noisy = apply_noise(values, recipe, ctx, city_recs, cc, is_latitude)
        header = ctx.rng.choice(headers_for_type)
        out.append(ColumnRow(
            final_label=type_aliased,
            sample_values=noisy,
            column_name=header,
            locale=locale,
            recipe=recipe,
        ))
    return out


# ────────────────────────────────────────────────────────────────────
#  Main
# ────────────────────────────────────────────────────────────────────

def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--seed", type=int, default=20260524, help="Deterministic seed (default: 20260524).")
    p.add_argument("--columns-per-type-per-locale", type=int, default=200,
                   help="Columns to emit per (type × locale). With 15 locales and 10 types this is "
                        "30,000 columns; the spec target of ~3,000 per type after blending is comfortably exceeded.")
    p.add_argument("--values-per-column", type=int, default=100,
                   help="Sample values per generated column (default 100, matches the existing pipeline).")
    p.add_argument("--out-dir", type=Path, default=OUT_DIR)
    return p.parse_args()


def main() -> int:
    args = parse_args()
    geonames_root = resolve_geonames_root()
    type_map = load_yaml(CFG_DIR / "type_mapping.yaml")
    locale_cfg_root = load_yaml(CFG_DIR / "locale_targets.yaml")
    header_pools = load_yaml(CFG_DIR / "header_pools.yaml")
    noise_recipes = load_yaml(CFG_DIR / "noise_recipes.yaml")

    types = type_map.get("types") or {}
    locales = locale_cfg_root.get("locales") or {}

    rng = random.Random(args.seed)

    # ── Load static GeoNames data once ────────────────────────────
    print(f"loading countries from {geonames_root}/countryInfo.txt", file=sys.stderr)
    countries = load_countries(geonames_root / "countryInfo.txt")
    country_name_by_cc = {c.iso2: c.country_name for c in countries}

    print(f"loading admin1 divisions", file=sys.stderr)
    admin1 = load_admin1(geonames_root / "admin1CodesASCII.txt")
    admin1_by_country: dict[str, list[Admin1Record]] = defaultdict(list)
    for a in admin1:
        if a.country_code:
            admin1_by_country[a.country_code].append(a)

    print(f"loading cities500", file=sys.stderr)
    cities = load_cities(geonames_root / "cities500.txt")
    cities_by_country: dict[str, list[CityRecord]] = defaultdict(list)
    for c in cities:
        cities_by_country[c.country_code].append(c)
    print(f"  {len(cities):,} cities across {len(cities_by_country)} countries", file=sys.stderr)

    # ── Per-locale country code union for postal/alternate loading ─
    all_ccs = set()
    all_isolangs = set()
    for _lname, lcfg in locales.items():
        all_ccs.update(lcfg.get("country_codes", []))
        all_isolangs.update(lcfg.get("isolanguages", []))

    print(f"loading postal codes for {len(all_ccs)} countries", file=sys.stderr)
    postal_by_country = load_postal_by_country(
        geonames_root / "postal" / "allCountries.txt", all_ccs
    )
    print(f"  {sum(len(v) for v in postal_by_country.values()):,} postal codes loaded "
          f"across {len(postal_by_country)} countries", file=sys.stderr)

    # ── Decide which geonameids we need alternate names for ───────
    # (Sample a representative subset of cities per locale to bound memory.)
    needed_ids: set[int] = set()
    cities_sample_per_country = 2000
    for cc, recs in cities_by_country.items():
        if cc not in all_ccs:
            continue
        if len(recs) <= cities_sample_per_country:
            sample = recs
        else:
            sample = rng.sample(recs, cities_sample_per_country)
        for r in sample:
            needed_ids.add(r.geonameid)
    # Plus admin1 geonameids for region localization
    for a in admin1:
        if a.country_code in all_ccs:
            needed_ids.add(a.geonameid)
    # Plus country geonameids
    for c in countries:
        if c.iso2 in all_ccs and c.geonameid:
            needed_ids.add(c.geonameid)
    print(f"loading alternate names for {len(needed_ids):,} geonameids "
          f"× {len(all_isolangs)} languages (this is the big read — ~1m)",
          file=sys.stderr)
    alt_names = load_alternate_names(
        geonames_root / "alternateNamesV2.txt", needed_ids, all_isolangs
    )
    print(f"  alt_names covering {len(alt_names):,} geonameids", file=sys.stderr)

    ctx = GenContext(
        cities_by_country=dict(cities_by_country),
        admin1_by_country=dict(admin1_by_country),
        countries=countries,
        country_name_by_cc=country_name_by_cc,
        postal_by_country=postal_by_country,
        alt_names=alt_names,
        header_pools=header_pools,
        noise_recipes=noise_recipes,
        rng=rng,
    )

    # ── Generate ─────────────────────────────────────────────────
    args.out_dir.mkdir(parents=True, exist_ok=True)
    out_csv = args.out_dir / "geonames_geography.csv.gz"
    manifest: dict = {
        "seed": args.seed,
        "columns_per_type_per_locale": args.columns_per_type_per_locale,
        "values_per_column": args.values_per_column,
        "snapshot_reference": "eval/datasets/snapshots/geonames-2026-05-24.json",
        "per_type_locale_counts": {},
        "per_type_total": {},
        "per_locale_total": {},
        "per_recipe_total": defaultdict(int),
    }
    per_type_total: dict[str, int] = defaultdict(int)
    per_locale_total: dict[str, int] = defaultdict(int)

    print(f"writing {out_csv}", file=sys.stderr)
    written = 0
    with gzip.open(out_csv, "wt", newline="", encoding="utf-8") as fout:
        writer = csv.writer(fout)
        writer.writerow(["final_label", "sample_values", "column_name"])
        for type_key, type_cfg in types.items():
            for locale_name, lcfg in locales.items():
                rows = generate_columns_for(
                    type_key, type_cfg, locale_name, lcfg,
                    args.columns_per_type_per_locale, args.values_per_column, ctx,
                )
                if not rows:
                    continue
                key = f"{type_key}::{locale_name}"
                manifest["per_type_locale_counts"][key] = len(rows)
                for r in rows:
                    per_type_total[r.final_label] += 1
                    per_locale_total[r.locale] += 1
                    manifest["per_recipe_total"][r.recipe] += 1
                    writer.writerow([
                        r.final_label,
                        json.dumps(r.sample_values, ensure_ascii=False),
                        r.column_name,
                    ])
                    written += 1
                print(f"  {type_key:<40s} {locale_name:<10s} +{len(rows):>4d}", file=sys.stderr)
    manifest["per_type_total"] = dict(per_type_total)
    manifest["per_locale_total"] = dict(per_locale_total)
    manifest["per_recipe_total"] = dict(manifest["per_recipe_total"])
    manifest["total_columns_written"] = written

    with (args.out_dir / "manifest.json").open("w") as f:
        json.dump(manifest, f, indent=2, ensure_ascii=False)

    print(f"\ndone — {written:,} columns -> {out_csv}", file=sys.stderr)
    print(f"manifest: {args.out_dir / 'manifest.json'}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
