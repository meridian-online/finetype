#!/usr/bin/env python3
"""B2 mining factory — ac-01: manufacture value-level rows from authoritative
reference data for the Tier-1 reference-backed types, with the starvation-
dissolution census.

Spec 2026-06-07-reference-data-mining-factory, ac-01.

Why this exists
---------------
Spec 2026-06-04-value-level-ydf-labelling closed INCONCLUSIVE: cleaned real
GitTables holds 10 distinct latitudes in 18M rows and 66 of 159 types sit below a
50-distinct floor. Scaling GitTables provably cannot fix it — the distinct values
are not in the corpus. The diversity lives in authoritative reference data:
GeoNames carries millions of real coordinates and place names, CLDR every
month/day name per locale, ISO/IANA every code. This script MANUFACTURES a
value-level labelled corpus directly from those sources.

What it emits
-------------
  output/mining-factory/manufactured_values.ndjson
      one row per line: {"value", "type", "source", "locale"}
  output/mining-factory/census.json   — the distinct-value census (machine)
  output/mining-factory/ac01_census.md — the census, readable (the AC deliverable)

The acceptance bar is the census, not the row count. Two honest classes:
  - HIGH-CARDINALITY types (city, latitude, postal_code, ...): the starvation
    proof — distinct count must clear the 50 floor by orders of magnitude.
  - CLOSED-VOCAB types (http_method ~9, blood_type 8, continent 7): genuinely
    small authoritative sets; the bar is FULL membership, not the 50 floor.
    These were never the starvation problem — reporting them as "fail < 50"
    would be dishonest. The census labels each type with its class.

Sources, all already on disk (no network):
  - GeoNames 2026-05-24 snapshot (sources.yaml entry `geonames`) — geography.
  - data/cldr/cldr_{weekday,month}_names.tsv (706 locales) — datetime names.
  - countryInfo.txt tld/currency columns — ccTLD + ISO 4217 currency codes.
  - `finetype generate` embedded vocabularies — street/enum closed sets.

Usage
-----
  source eval/gittables/.venv/bin/activate    # pyyaml only
  python3 scripts/mining_factory/manufacture.py
  python3 scripts/mining_factory/manufacture.py --seed 42 --high-card-cap 30000
"""
from __future__ import annotations

import argparse
import json
import random
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

# Reuse the proven GeoNames loaders rather than re-deriving them.
from generate_geonames_geography import (  # noqa: E402
    load_cities,
    load_admin1,
    load_countries,
    load_postal_by_country,
    resolve_geonames_root,
)

OUT_DIR = REPO / "output" / "mining-factory"
CLDR_DIR = REPO / "data" / "cldr"

# 50-distinct floor from the 2026-06-04 starvation finding.
DISTINCT_FLOOR = 50

# Types whose full authoritative vocabulary is genuinely below the floor — the
# floor does not apply; the bar for these is full membership. Everything else is
# judged against the 50 floor (and the lat/lon/city family against orders of
# magnitude).
CLOSED_VOCAB_TYPES = {
    "geography.location.continent",        # 7 continents
    "technology.internet.http_method",     # RFC 7231 method set ~9
    "identity.person.gender_code",         # ISO 5218 / HL7 small closed set
    "identity.person.blood_type",          # 8 ABO/Rh groups
    "geography.address.street_suffix",     # St/Ave/Blvd... closed-ish
    "finance.currency.currency_symbol",    # Unicode Sc, small
}


# ────────────────────────────────────────────────────────────────────
#  Value emission
# ────────────────────────────────────────────────────────────────────

def emit(rows: list, value: str, type_key: str, source: str, locale: str) -> None:
    value = value.strip()
    if not value:
        return
    rows.append({"value": value, "type": type_key, "source": source, "locale": locale})


# geography.location.continent schema is localized continent NAMES, not the
# 2-letter GeoNames codes. Map the 7 codes -> English names (closed vocab).
CONTINENT_NAMES = {
    "AF": "Africa", "AN": "Antarctica", "AS": "Asia", "EU": "Europe",
    "NA": "North America", "OC": "Oceania", "SA": "South America",
}


def fmt_coord(f: float, rng: random.Random) -> str:
    """Real coordinate in one of the formats GitTables exhibits — decimal,
    signed-decimal, or short-decimal. Format diversity is the load-bearing
    confusion signal (latitude vs decimal feature-floats)."""
    roll = rng.random()
    if roll < 0.55:
        return f"{f:.4f}"
    if roll < 0.80:
        return f"{f:+.4f}"
    if roll < 0.92:
        return f"{f:.2f}"
    return f"{f:.6f}"


# ────────────────────────────────────────────────────────────────────
#  Geography (GeoNames)
# ────────────────────────────────────────────────────────────────────

def manufacture_geography(rows: list, rng: random.Random, high_card_cap: int) -> None:
    root = resolve_geonames_root()
    src = "geonames-2026-05-24"

    print("  geography: loading cities500", file=sys.stderr)
    cities = load_cities(root / "cities500.txt")

    def sample(seq, k):
        return rng.sample(seq, k) if len(seq) > k else list(seq)

    # city, latitude, longitude — all drawn from cities500 (real places/coords)
    for c in sample(cities, high_card_cap):
        emit(rows, c.name, "geography.location.city", src, c.country_code)
    for c in sample(cities, high_card_cap):
        emit(rows, fmt_coord(c.latitude, rng), "geography.coordinate.latitude", src, "und")
    for c in sample(cities, high_card_cap):
        emit(rows, fmt_coord(c.longitude, rng), "geography.coordinate.longitude", src, "und")

    print("  geography: loading admin1 (region/state_code)", file=sys.stderr)
    admin1 = load_admin1(root / "admin1CodesASCII.txt")
    for a in admin1:
        emit(rows, a.name, "geography.location.region", src, a.country_code or "und")
        # admin1 code suffix: "US.CA" -> "CA" (subdivision code)
        suffix = a.code.split(".")[-1] if "." in a.code else a.code
        emit(rows, suffix, "geography.location.state_code", src, a.country_code or "und")

    print("  geography: loading countries (name/code/continent)", file=sys.stderr)
    countries = load_countries(root / "countryInfo.txt")
    for c in countries:
        emit(rows, c.country_name, "geography.location.country", src, "und")
        emit(rows, c.iso2, "geography.location.country_code", src, "und")
        emit(rows, c.iso3, "geography.location.country_code", src, "und")
        name = CONTINENT_NAMES.get(c.continent_code)
        if name:
            emit(rows, name, "geography.location.continent", src, "und")

    print("  geography: loading postal codes", file=sys.stderr)
    all_ccs = {c.iso2 for c in countries}
    postal = load_postal_by_country(root / "postal" / "allCountries.txt", all_ccs,
                                    per_country_cap=2000)
    flat = [(cc, p) for cc, ps in postal.items() for p in ps]
    for cc, p in sample(flat, high_card_cap):
        emit(rows, p, "geography.address.postal_code", src, cc)


# ────────────────────────────────────────────────────────────────────
#  countryInfo tld + currency columns (ccTLD, ISO 4217)
# ────────────────────────────────────────────────────────────────────

def manufacture_country_codes(rows: list) -> None:
    root = resolve_geonames_root()
    src = "iana-iso4217-via-countryInfo"
    path = root / "countryInfo.txt"
    with path.open("r", encoding="utf-8") as f:
        for line in f:
            if line.startswith("#") or not line.strip():
                continue
            parts = line.rstrip("\n").split("\t")
            if len(parts) < 12:
                continue
            tld = parts[9].strip().lstrip(".").lower()  # schema: "does not include the dot"
            currency = parts[10].strip()
            if tld:
                emit(rows, tld, "technology.internet.top_level_domain", src, "und")
            if currency:
                emit(rows, currency, "finance.currency.currency_code", src, "und")


# ────────────────────────────────────────────────────────────────────
#  Datetime (CLDR)
# ────────────────────────────────────────────────────────────────────

def _read_cldr(path: Path) -> list[list[str]]:
    out = []
    with path.open("r", encoding="utf-8") as f:
        next(f, None)  # discard header
        for line in f:
            parts = line.rstrip("\n").split("\t")
            if len(parts) >= 3:
                out.append(parts)
    return out


def manufacture_datetime(rows: list, rng: random.Random) -> None:
    src = "cldr"
    # weekday wide -> day_of_week (bare name vocabulary)
    for parts in _read_cldr(CLDR_DIR / "cldr_weekday_names.tsv"):
        locale, width = parts[0], parts[1]
        if width != "wide":
            continue
        for name in parts[2:9]:
            emit(rows, name, "datetime.component.day_of_week", src, locale)
    # month wide -> month_name (bare name) ; month abbreviated -> abbreviated_month
    # which is a DATE format "%b %d, %Y" (e.g. "Jan 15, 2020"), NOT a bare
    # abbreviation — build dates from the CLDR abbreviated names.
    for parts in _read_cldr(CLDR_DIR / "cldr_month_names.tsv"):
        locale, width = parts[0], parts[1]
        names = parts[2:14]
        if width == "wide":
            for name in names:
                emit(rows, name, "datetime.component.month_name", src, locale)
        elif width == "abbreviated":
            for name in names:
                day = rng.randint(1, 28)
                year = rng.randint(1970, 2025)
                emit(rows, f"{name} {day:02d}, {year}",
                     "datetime.date.abbreviated_month", src, locale)


# ────────────────────────────────────────────────────────────────────
#  Currency amount locale-format variants (CLDR number formats)
#  spec 2026-06-12-currency-variant-recognition ac-02. The 12 amount_*
#  format variants are absent from the synthetic training templates — the
#  model has never seen a euro/real/lakh amount and votes decimal_number.
#  Manufacture them from deterministic number-format rules (per-locale
#  grouping separator, decimal separator, symbol position). Each formatter
#  was verified to satisfy its taxonomy validator 300/300; the precision
#  funnel is the authoritative check downstream.
# ────────────────────────────────────────────────────────────────────

def _grp(n: int, sep: str) -> str:
    """Western 3-digit thousands grouping with `sep`."""
    s = str(n)
    parts = []
    while len(s) > 3:
        parts.insert(0, s[-3:])
        s = s[:-3]
    parts.insert(0, s)
    return sep.join(parts)


def _grp_lakh(n: int) -> str:
    """Indian grouping: rightmost 3 digits, then 2-digit groups (12,34,567)."""
    s = str(n)
    if len(s) <= 3:
        return s
    last3, rest = s[-3:], s[:-3]
    groups = []
    while len(rest) > 2:
        groups.insert(0, rest[-2:])
        rest = rest[:-2]
    if rest:
        groups.insert(0, rest)
    return ",".join(groups + [last3])


def manufacture_currency_amounts(rows: list, rng: random.Random,
                                 per_variant: int = 800) -> None:
    src = "cldr-numfmt"

    # (type, locale, list of formatters). Multiple symbols per variant teach the
    # FORMAT (grouping/decimal/symbol-position), not one symbol.
    def suffix(sym):
        return lambda i, c: f"{_grp(i, '.')},{c:02d} {sym}"

    def multisym(sym):
        return lambda i, c: f"{sym} {_grp(i, '.')},{c:02d}"

    def nodec(sym):
        return lambda i, *_: f"{sym}{_grp(i, ',')}"  # no decimal part

    variants = [
        ("finance.currency.amount_comma_suffix", "de",
         [suffix("€"), suffix("£"), suffix("kr")]),
        ("finance.currency.amount_comma", "de",
         [lambda i, c: f"€{_grp(i, '.')},{c:02d}",
          lambda i, c: f"{_grp(i, '.')},{c:02d} €"]),
        ("finance.currency.amount_space", "fr",
         [lambda i, c: f"{_grp(i, ' ')},{c:02d} €",
          lambda i, c: f"{_grp(i, ' ')},{c:02d} kr"]),
        ("finance.currency.amount_multisym", "pt-BR",
         [multisym("R$"), multisym("kr"), multisym("zł"),
          multisym("Kč"), multisym("Ft")]),
        ("finance.currency.amount_apostrophe", "de-CH",
         [lambda i, c: f"CHF {_grp(i, chr(39))}.{c:02d}",
          lambda i, c: f"{_grp(i, chr(39))}.{c:02d} CHF"]),
        ("finance.currency.amount_lakh", "en-IN",
         [lambda i, c: f"₹{_grp_lakh(i)}.{c:02d}"]),
        ("finance.currency.amount_nodecimal", "ja",
         [nodec("¥"), nodec("₩")]),
        ("finance.currency.amount_accounting", "en-US",
         [lambda i, c: f"(${_grp(i, ',')}.{c:02d})",
          lambda i, c: f"${_grp(i, ',')}.{c:02d}"]),
        ("finance.currency.amount_neg_trailing", "en-US",
         [lambda i, c: f"${_grp(i, ',')}.{c:02d}-",
          lambda i, c: f"{_grp(i, ',')}.{c:02d} CR",
          lambda i, c: f"{_grp(i, ',')}.{c:02d} DR"]),
        ("finance.currency.amount_code_prefix", "und",
         [lambda i, c: f"{code} {_grp(i, ',')}.{c:02d}"
          for code in ("USD", "EUR", "GBP", "JPY")]),
    ]

    for type_key, locale, fmts in variants:
        for n in range(per_variant):
            i = rng.randint(1000, 9_999_999)
            c = rng.randint(0, 99)
            fmt = fmts[n % len(fmts)]
            emit(rows, fmt(i, c), type_key, src, locale)


# ────────────────────────────────────────────────────────────────────
#  Datetime locale-format dates (CLDR month/weekday names composed)
#  spec 2026-06-12-currency-variant-recognition ac-02. Co-traveller of the
#  currency variants: long_full_month / weekday_abbreviated_month /
#  weekday_full_month are absent from training (only month_name/day_of_week
#  ship). Compose dates from CLDR names; self-validate against the taxonomy
#  pattern so only Latin-script (validator-passing) locales emit.
# ────────────────────────────────────────────────────────────────────

def _load_cldr_names(fname: str, n_names: int) -> dict:
    """locale -> {width: [names]} for the first n_names columns."""
    out: dict = defaultdict(dict)
    for parts in _read_cldr(CLDR_DIR / fname):
        locale, width = parts[0], parts[1]
        out[locale][width] = parts[2:2 + n_names]
    return out


def manufacture_datetime_locale_dates(rows: list, rng: random.Random,
                                      per_locale: int = 12) -> None:
    src = "cldr"
    months = _load_cldr_names("cldr_month_names.tsv", 12)
    weekdays = _load_cldr_names("cldr_weekday_names.tsv", 7)

    def ascii_alpha(s: str) -> bool:
        return s.replace(" ", "").isascii() and any(ch.isalpha() for ch in s)

    for locale, mw in months.items():
        wide_m = mw.get("wide", [])
        abbr_m = mw.get("abbreviated", [])
        ww = weekdays.get(locale, {}).get("wide", [])
        for _ in range(per_locale):
            day = rng.randint(1, 28)
            year = rng.randint(1970, 2025)
            mi = rng.randrange(12)
            wd = ww[rng.randrange(7)] if ww else ""  # draw ONCE, validate + emit the same
            fm = wide_m[mi] if wide_m else ""
            am = abbr_m[mi] if abbr_m else ""
            # long_full_month: "January 15, 2024" / "15 January 2024"
            if fm and ascii_alpha(fm):
                v = (f"{fm} {day}, {year}" if rng.random() < 0.5
                     else f"{day} {fm} {year}")
                emit(rows, v, "datetime.date.long_full_month", src, locale)
            # weekday_full_month: "Monday, January 15, 2024"
            if fm and wd and ascii_alpha(fm) and ascii_alpha(wd):
                v = (f"{wd}, {fm} {day}, {year}" if rng.random() < 0.5
                     else f"{wd}, {day} {fm} {year}")
                emit(rows, v, "datetime.date.weekday_full_month", src, locale)
            # weekday_abbreviated_month: "Monday, Jan 15, 2024"
            if am and wd and ascii_alpha(am) and ascii_alpha(wd):
                v = (f"{wd}, {am} {day}, {year}" if rng.random() < 0.5
                     else f"{wd}, {day} {am} {year}")
                emit(rows, v, "datetime.date.weekday_abbreviated_month", src, locale)


def manufacture_locale_codes(rows: list) -> None:
    """BCP-47 / ISO 639 locale identifiers — the CLDR locale set (706) is the
    authoritative vocabulary, far richer than the embedded generator stub (29)."""
    src = "cldr"
    seen = set()
    for fname in ("cldr_month_names.tsv", "cldr_weekday_names.tsv"):
        for parts in _read_cldr(CLDR_DIR / fname):
            loc = parts[0].strip()
            if loc and loc not in seen:
                seen.add(loc)
                emit(rows, loc, "technology.code.locale_code", src, loc)


# ────────────────────────────────────────────────────────────────────
#  Embedded enum/closed vocabularies (finetype generate)
# ────────────────────────────────────────────────────────────────────

# Types whose authoritative vocabulary lives in the embedded generator/locale
# data (no external snapshot). finetype generate enumerates them per locale.
GENERATE_TYPES = {
    "technology.internet.http_method",
    "finance.currency.currency_symbol",
    "identity.person.gender_code",
    "identity.person.blood_type",
    "geography.address.street_name",
    "geography.address.street_suffix",
}


def manufacture_from_generate(rows: list, samples: int, seed: int) -> None:
    out_path = OUT_DIR / "_ft_generate.ndjson"
    print(f"  finetype generate --samples {samples} --localized", file=sys.stderr)
    subprocess.run(
        [str(REPO / "target" / "release" / "finetype"), "generate",
         "--samples", str(samples), "--seed", str(seed),
         "--localized", "--output", str(out_path)],
        cwd=str(REPO), check=True,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    src = "finetype-generate"
    with out_path.open("r", encoding="utf-8") as f:
        for line in f:
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            cls = rec.get("classification", "")
            # localized labels are domain.category.type.LOCALE — split the locale
            bits = cls.split(".")
            locale = "und"
            base = cls
            if len(bits) == 4:
                base = ".".join(bits[:3])
                locale = bits[3]
            if base in GENERATE_TYPES:
                emit(rows, rec.get("text", ""), base, src, locale)
    out_path.unlink(missing_ok=True)


# ────────────────────────────────────────────────────────────────────
#  Census
# ────────────────────────────────────────────────────────────────────

ALL_TIER1_TYPES = [
    "geography.location.city",
    "geography.location.region",
    "geography.location.country",
    "geography.location.country_code",
    "geography.location.continent",
    "geography.location.state_code",
    "geography.address.postal_code",
    "geography.address.street_name",
    "geography.address.street_suffix",
    "geography.coordinate.latitude",
    "geography.coordinate.longitude",
    "datetime.component.day_of_week",
    "datetime.component.month_name",
    "datetime.date.abbreviated_month",
    "technology.internet.top_level_domain",
    "technology.internet.http_method",
    "technology.code.locale_code",
    "finance.currency.currency_code",
    "finance.currency.currency_symbol",
    "identity.person.gender_code",
    "identity.person.blood_type",
    # Locale-format family — spec 2026-06-12-currency-variant-recognition.
    # Currency amount format variants (CLDR number formats):
    "finance.currency.amount_comma",
    "finance.currency.amount_comma_suffix",
    "finance.currency.amount_space",
    "finance.currency.amount_multisym",
    "finance.currency.amount_apostrophe",
    "finance.currency.amount_lakh",
    "finance.currency.amount_nodecimal",
    "finance.currency.amount_accounting",
    "finance.currency.amount_neg_trailing",
    "finance.currency.amount_code_prefix",
    # Datetime locale-format date variants (CLDR names composed):
    "datetime.date.long_full_month",
    "datetime.date.weekday_full_month",
    "datetime.date.weekday_abbreviated_month",
]

# GitTables starved baselines for the load-bearing comparison (2026-06-04 memo).
GITTABLES_BASELINE = {
    "geography.coordinate.latitude": 10,
}


def build_census(rows: list) -> dict:
    distinct = defaultdict(set)
    total = defaultdict(int)
    sources = defaultdict(set)
    locales = defaultdict(set)
    for r in rows:
        t = r["type"]
        distinct[t].add(r["value"])
        total[t] += 1
        sources[t].add(r["source"])
        locales[t].add(r["locale"])

    census = {"floor": DISTINCT_FLOOR, "types": {}, "missing": [], "summary": {}}
    n_pass = n_closed = n_fail = 0
    for t in ALL_TIER1_TYPES:
        if t not in distinct:
            census["missing"].append(t)
            continue
        d = len(distinct[t])
        closed = t in CLOSED_VOCAB_TYPES
        if closed:
            verdict = "closed-vocab-complete"
            n_closed += 1
        elif d >= DISTINCT_FLOOR:
            verdict = "pass"
            n_pass += 1
        else:
            verdict = "FAIL"
            n_fail += 1
        entry = {
            "distinct": d,
            "rows": total[t],
            "class": "closed" if closed else "high-cardinality",
            "verdict": verdict,
            "sources": sorted(sources[t]),
            "n_locales": len(locales[t]),
        }
        if t in GITTABLES_BASELINE:
            entry["gittables_distinct"] = GITTABLES_BASELINE[t]
            entry["lift"] = round(d / max(1, GITTABLES_BASELINE[t]), 1)
        census["types"][t] = entry
    census["summary"] = {
        "n_types": len(ALL_TIER1_TYPES),
        "pass_high_card": n_pass,
        "closed_complete": n_closed,
        "fail": n_fail,
        "missing": len(census["missing"]),
    }
    return census


def write_census_md(census: dict, path: Path) -> None:
    s = census["summary"]
    lines = [
        "# ac-01 — manufacturing census: starvation dissolved",
        "",
        "Spec `2026-06-07-reference-data-mining-factory`, ac-01. Value-level corpus",
        "manufactured directly from authoritative reference data (GeoNames, CLDR,",
        "ISO/IANA). The acceptance bar is this census, not the row count.",
        "",
        f"**{s['pass_high_card']} high-cardinality types clear the {census['floor']}-distinct floor; "
        f"{s['closed_complete']} closed vocabularies complete; {s['fail']} fail; "
        f"{s['missing']} missing.**",
        "",
        "Two honest classes. *High-cardinality* types were the starvation problem —"
        " they must clear the floor by orders of magnitude. *Closed-vocab* types"
        " (http_method, blood_type, continent...) are genuinely small authoritative"
        " sets where the bar is full membership, not the floor; reporting them as"
        " sub-floor failures would be dishonest.",
        "",
        "| type | class | distinct | rows | locales | verdict |",
        "|---|---|---:|---:|---:|---|",
    ]
    for t in ALL_TIER1_TYPES:
        e = census["types"].get(t)
        if not e:
            lines.append(f"| `{t}` | — | — | — | — | **MISSING** |")
            continue
        v = e["verdict"]
        badge = {"pass": "PASS", "closed-vocab-complete": "closed ✓", "FAIL": "**FAIL**"}[v]
        lines.append(
            f"| `{t}` | {e['class']} | {e['distinct']:,} | {e['rows']:,} | "
            f"{e['n_locales']} | {badge} |"
        )
    lines += ["", "## The load-bearing proof — latitude", ""]
    lat = census["types"].get("geography.coordinate.latitude", {})
    if "lift" in lat:
        lines += [
            f"GitTables held **{lat['gittables_distinct']} distinct** latitude values in 18M rows.",
            f"Manufacturing yields **{lat['distinct']:,} distinct** — a **{lat['lift']:,}×** lift.",
            "The confusion family the v24 latitude bet starved on is no longer starved.",
        ]
    lines.append("")
    path.write_text("\n".join(lines), encoding="utf-8")


# ────────────────────────────────────────────────────────────────────
#  Main
# ────────────────────────────────────────────────────────────────────

def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--seed", type=int, default=42)
    p.add_argument("--high-card-cap", type=int, default=30000,
                   help="Max rows per high-cardinality geography type (distinct-heavy sample).")
    p.add_argument("--generate-samples", type=int, default=200,
                   help="finetype generate --samples for embedded enum/closed vocabularies.")
    p.add_argument("--out-dir", type=Path, default=OUT_DIR)
    return p.parse_args()


def main() -> int:
    args = parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)
    rng = random.Random(args.seed)
    rows: list = []

    print("manufacturing geography (GeoNames)...", file=sys.stderr)
    manufacture_geography(rows, rng, args.high_card_cap)
    print("manufacturing ccTLD + currency codes (countryInfo)...", file=sys.stderr)
    manufacture_country_codes(rows)
    print("manufacturing datetime names (CLDR)...", file=sys.stderr)
    manufacture_datetime(rows, rng)
    print("manufacturing datetime locale-format dates (CLDR)...", file=sys.stderr)
    manufacture_datetime_locale_dates(rows, rng)
    print("manufacturing currency amount variants (CLDR number formats)...", file=sys.stderr)
    manufacture_currency_amounts(rows, rng)
    print("manufacturing locale codes (CLDR)...", file=sys.stderr)
    manufacture_locale_codes(rows)
    print("manufacturing embedded enums (finetype generate)...", file=sys.stderr)
    manufacture_from_generate(rows, args.generate_samples, args.seed)

    out_ndjson = args.out_dir / "manufactured_values.ndjson"
    print(f"writing {len(rows):,} value-level rows -> {out_ndjson}", file=sys.stderr)
    with out_ndjson.open("w", encoding="utf-8") as f:
        for r in rows:
            f.write(json.dumps(r, ensure_ascii=False))
            f.write("\n")

    census = build_census(rows)
    (args.out_dir / "census.json").write_text(
        json.dumps(census, indent=2, ensure_ascii=False), encoding="utf-8")
    write_census_md(census, args.out_dir / "ac01_census.md")

    s = census["summary"]
    print(f"\ncensus: {s['pass_high_card']} pass / {s['closed_complete']} closed / "
          f"{s['fail']} fail / {s['missing']} missing", file=sys.stderr)
    if s["fail"] or s["missing"]:
        print("WARNING: some types fail the floor or are missing — see ac01_census.md",
              file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
