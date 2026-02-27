#!/usr/bin/env python3
"""Extract CLDR date/time patterns and map them to FineType datetime types.

Reads CLDR JSON (downloaded by download_cldr.sh) and produces:
1. A mapping table: LDML pattern shape → FineType type
2. Month and weekday name tables per locale (wide + abbreviated)
3. A summary report of coverage and gaps

Usage:
    python3 scripts/extract_cldr_patterns.py [--cldr-dir data/cldr/json]

Output files (written to data/cldr/):
    cldr_date_patterns.tsv      — date format patterns per locale with FineType mapping
    cldr_time_patterns.tsv      — time format patterns per locale with FineType mapping
    cldr_month_names.tsv        — month names per locale (wide + abbreviated)
    cldr_weekday_names.tsv      — weekday names per locale (wide + abbreviated)
    cldr_mapping_report.txt     — summary statistics and coverage analysis
"""

import argparse
import json
import os
import re
import sys
from collections import defaultdict
from pathlib import Path

# ═══════════════════════════════════════════════════════════════════════════════
# LDML → FineType mapping rules
#
# CLDR uses LDML (Locale Data Markup Language) patterns.
# Key tokens:
#   y/yy/yyyy = year, M/MM = month number, MMM = month abbrev, MMMM = month name
#   d/dd = day, EEEE = weekday name, EEE = weekday abbrev
#   h = 12h hour, H = 24h hour, mm = minute, ss = second, a = AM/PM
#   Quoted literals like 'de' are locale-specific decorators
# ═══════════════════════════════════════════════════════════════════════════════

# CJK locales to exclude — these use y年M月d日 patterns that require new
# taxonomy types. Out of scope for this phase (see plan).
CJK_LOCALES = {"ja", "zh", "zh-Hans", "zh-Hant", "zh-Hans-HK", "zh-Hans-MO",
               "zh-Hans-SG", "zh-Hant-HK", "zh-Hant-MO", "ko", "yue",
               "yue-Hans"}

# Locales we want to target for training data — broad coverage
# of Latin-script and Cyrillic-script locales
TARGET_LOCALES = {
    # English variants
    "en", "en-US", "en-GB", "en-AU", "en-CA", "en-NZ", "en-IE", "en-ZA",
    # Western European
    "de", "de-AT", "de-CH", "fr", "fr-CA", "fr-CH", "fr-BE",
    "es", "es-MX", "es-AR", "es-CO", "it", "it-CH",
    "nl", "nl-BE", "pt", "pt-BR",
    # Nordic
    "sv", "da", "nb", "nn", "fi",
    # Central/Eastern European
    "pl", "cs", "sk", "hu", "ro", "hr", "sl", "bg", "sr-Latn",
    # Baltic
    "lt", "lv", "et",
    # Other Latin-script
    "tr", "el", "uk", "ru",
    # Arabic (RTL but Latin-digit dates are common in data)
    "ar",
}


def normalize_pattern(pattern: str) -> str:
    """Strip quoted literals and whitespace variants to get the structural shape."""
    # Remove quoted literals like 'de', 'г'.
    stripped = re.sub(r"'[^']*'\.?", "", pattern)
    # Remove Unicode directional marks
    stripped = re.sub(r"[\u200f\u200e\u202f]", "", stripped)
    # Collapse multiple spaces
    stripped = re.sub(r"\s+", " ", stripped).strip()
    # Remove trailing punctuation that's locale-specific
    stripped = stripped.rstrip(".,،")
    return stripped


def classify_date_pattern(pattern: str) -> tuple[str, str]:
    """Map an LDML date pattern to a FineType type and format variant.

    Returns (finetype_type, format_note) where format_note describes the
    specific variant (e.g., "MDY slash" vs "DMY slash").
    """
    norm = normalize_pattern(pattern)

    # === Short patterns (numeric only) ===

    # ISO-style: y-MM-dd or yyyy-MM-dd
    if re.match(r"^y+-MM?-dd?$", norm):
        return "datetime.date.iso", "YMD dash"

    # US slash: M/d/yy or M/d/y
    if re.match(r"^M/d/y+$", norm):
        return "datetime.date.us_slash", "MDY slash"
    if re.match(r"^MM?/dd?/y+$", norm):
        return "datetime.date.us_slash", "MDY slash padded"

    # EU slash: d/M/yy or dd/MM/y
    if re.match(r"^d/M/y+$", norm):
        return "datetime.date.eu_slash", "DMY slash"
    if re.match(r"^dd?/MM?/y+$", norm):
        return "datetime.date.eu_slash", "DMY slash padded"

    # EU dot: dd.MM.yy or d.MM.y or d.M.y (with optional spaces around dots)
    if re.match(r"^dd?\.\s*MM?\.\s*y+\.?$", norm):
        return "datetime.date.eu_dot", "DMY dot"

    # Space-dot variant: d. M. y (Serbian, Slovak, etc.)
    if re.match(r"^dd?\.\s+M\.\s+y+\.?$", norm):
        return "datetime.date.eu_dot", "DMY space-dot"

    # NL dash: dd-MM-y
    if re.match(r"^dd?-MM?-y+$", norm):
        return "datetime.date.eu_slash", "DMY dash (eu_slash variant)"

    # Hungarian YMD dot: y. MM. dd.
    if re.match(r"^y+\.\s*MM?\.\s*dd?\.?$", norm):
        return "datetime.date.iso", "YMD dot (Hungarian style)"

    # YMD slash: y/MM/dd (en-ZA)
    if re.match(r"^y+/MM?/dd?$", norm):
        return "datetime.date.iso", "YMD slash"

    # YMD slash short: yy/M/d
    if re.match(r"^y{2}/M/d$", norm):
        return "datetime.date.iso", "short YMD slash"

    # Comma-separated: dd,MM,y
    if re.match(r"^dd?,MM?,y+$", norm):
        return "datetime.date.eu_slash", "DMY comma"

    # Space-separated: d MM y (Yoruba etc.)
    if re.match(r"^dd?\s+MM\s+y+$", norm):
        return "datetime.date.eu_slash", "DMY space"

    # kkj format: dd/MM y (date with space before year)
    if re.match(r"^dd?/MM\s+y+$", norm):
        return "datetime.date.eu_slash", "DMY slash-space"

    # Short YMD: yy-MM-dd
    if re.match(r"^y{2}-MM-dd$", norm):
        return "datetime.date.short_ymd", "short YMD dash"

    # Short MDY: MM-dd-yy
    if re.match(r"^MM-dd-y{2}$", norm):
        return "datetime.date.short_mdy", "short MDY dash"

    # Short DMY: dd-MM-yy
    if re.match(r"^dd-MM-y{2}$", norm):
        return "datetime.date.short_dmy", "short DMY dash"

    # Compact YMD: yyyyMMdd
    if re.match(r"^y{4}MM?dd?$", norm):
        return "datetime.date.compact_ymd", "compact YMD"

    # === Medium patterns (abbreviated month) ===

    # "MMM d, y" or "d MMM y" or "d. MMM y" etc.
    if "MMM" in norm and "MMMM" not in norm:
        return "datetime.date.abbreviated_month", "abbreviated month"

    # === Long patterns (full month name) ===

    # With weekday
    if "EEEE" in norm and "MMMM" in norm:
        return "datetime.date.weekday_full_month", "weekday + full month"
    if ("EEE" in norm or "cccc" in norm) and "MMMM" in norm:
        return "datetime.date.weekday_full_month", "weekday + full month (alt)"

    # Without weekday
    if "MMMM" in norm:
        return "datetime.date.long_full_month", "full month"

    # === Fallback ===
    return "UNMAPPED", f"unrecognized: {norm}"


def classify_time_pattern(pattern: str) -> tuple[str, str]:
    """Map an LDML time pattern to a FineType type and format variant."""
    norm = normalize_pattern(pattern)
    # Remove timezone specifiers for classification
    no_tz = re.sub(r"\s*z+\s*", "", norm).strip()

    has_12h = "h" in no_tz.lower().split(":")[0] if ":" in no_tz else False
    has_ampm = "a" in no_tz
    has_seconds = no_tz.count(":") >= 2 or "ss" in no_tz

    # 12h with AM/PM
    if has_ampm or (has_12h and "H" not in no_tz):
        if has_seconds:
            return "datetime.time.hms_12h", "12h with seconds"
        else:
            return "datetime.time.hm_12h", "12h without seconds"

    # 24h
    if has_seconds:
        return "datetime.time.hms_24h", "24h with seconds"
    else:
        return "datetime.time.hm_24h", "24h without seconds"


def extract_names(greg_data: dict, section: str, width: str) -> list[str]:
    """Extract month or day names from CLDR gregorian calendar data.

    section: 'months' or 'days'
    width: 'wide' or 'abbreviated'
    """
    try:
        container = greg_data[section]["format"][width]
    except KeyError:
        # Try stand-alone as fallback
        try:
            container = greg_data[section]["stand-alone"][width]
        except KeyError:
            return []

    if section == "months":
        # Months are keyed "1" through "12"
        return [container[str(i)] for i in range(1, 13) if str(i) in container]
    elif section == "days":
        # Days are keyed by name
        day_order = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]
        return [container[d] for d in day_order if d in container]
    return []


def process_locale(cldr_dir: str, locale: str) -> dict | None:
    """Process a single locale's CLDR data."""
    greg_path = os.path.join(cldr_dir, "cldr-dates-full", "main", locale, "ca-gregorian.json")
    if not os.path.exists(greg_path):
        return None

    with open(greg_path) as f:
        data = json.load(f)

    greg = data["main"][locale]["dates"]["calendars"]["gregorian"]

    result = {
        "locale": locale,
        "date_formats": {},
        "time_formats": {},
        "months_wide": [],
        "months_abbreviated": [],
        "weekdays_wide": [],
        "weekdays_abbreviated": [],
    }

    # Date formats
    for length in ("short", "medium", "long", "full"):
        pattern = greg.get("dateFormats", {}).get(length)
        if pattern and isinstance(pattern, str):
            ft_type, note = classify_date_pattern(pattern)
            result["date_formats"][length] = {
                "pattern": pattern,
                "finetype": ft_type,
                "note": note,
            }

    # Time formats
    for length in ("short", "medium", "long", "full"):
        pattern = greg.get("timeFormats", {}).get(length)
        if pattern and isinstance(pattern, str):
            ft_type, note = classify_time_pattern(pattern)
            result["time_formats"][length] = {
                "pattern": pattern,
                "finetype": ft_type,
                "note": note,
            }

    # Month and weekday names
    result["months_wide"] = extract_names(greg, "months", "wide")
    result["months_abbreviated"] = extract_names(greg, "months", "abbreviated")
    result["weekdays_wide"] = extract_names(greg, "days", "wide")
    result["weekdays_abbreviated"] = extract_names(greg, "days", "abbreviated")

    return result


def write_date_patterns_tsv(results: list[dict], output_path: str):
    """Write date format patterns TSV."""
    with open(output_path, "w") as f:
        f.write("locale\tlength\tldml_pattern\tnormalized\tfinetype_type\tnote\n")
        for r in results:
            for length in ("short", "medium", "long", "full"):
                if length in r["date_formats"]:
                    entry = r["date_formats"][length]
                    norm = normalize_pattern(entry["pattern"])
                    f.write(f"{r['locale']}\t{length}\t{entry['pattern']}\t{norm}\t"
                            f"{entry['finetype']}\t{entry['note']}\n")


def write_time_patterns_tsv(results: list[dict], output_path: str):
    """Write time format patterns TSV."""
    with open(output_path, "w") as f:
        f.write("locale\tlength\tldml_pattern\tfinetype_type\tnote\n")
        for r in results:
            for length in ("short", "medium", "long", "full"):
                if length in r["time_formats"]:
                    entry = r["time_formats"][length]
                    f.write(f"{r['locale']}\t{length}\t{entry['pattern']}\t"
                            f"{entry['finetype']}\t{entry['note']}\n")


def write_month_names_tsv(results: list[dict], output_path: str):
    """Write month names TSV."""
    with open(output_path, "w") as f:
        f.write("locale\twidth\t" + "\t".join(f"month_{i}" for i in range(1, 13)) + "\n")
        for r in results:
            if r["months_wide"]:
                f.write(f"{r['locale']}\twide\t" + "\t".join(r["months_wide"]) + "\n")
            if r["months_abbreviated"]:
                f.write(f"{r['locale']}\tabbreviated\t" + "\t".join(r["months_abbreviated"]) + "\n")


def write_weekday_names_tsv(results: list[dict], output_path: str):
    """Write weekday names TSV."""
    with open(output_path, "w") as f:
        f.write("locale\twidth\t" + "\t".join(["mon", "tue", "wed", "thu", "fri", "sat", "sun"]) + "\n")
        for r in results:
            if r["weekdays_wide"]:
                f.write(f"{r['locale']}\twide\t" + "\t".join(r["weekdays_wide"]) + "\n")
            if r["weekdays_abbreviated"]:
                f.write(f"{r['locale']}\tabbreviated\t" + "\t".join(r["weekdays_abbreviated"]) + "\n")


def write_report(results: list[dict], skipped_cjk: list[str], output_path: str):
    """Write summary report."""
    with open(output_path, "w") as f:
        f.write("=" * 72 + "\n")
        f.write("CLDR → FineType Pattern Mapping Report\n")
        f.write("=" * 72 + "\n\n")

        f.write(f"Locales processed:  {len(results)}\n")
        f.write(f"CJK locales excluded: {len(skipped_cjk)} ({', '.join(sorted(skipped_cjk))})\n\n")

        # Date format coverage
        f.write("─" * 40 + "\n")
        f.write("DATE FORMAT COVERAGE\n")
        f.write("─" * 40 + "\n\n")

        type_counts: dict[str, int] = defaultdict(int)
        unmapped_patterns: list[tuple[str, str, str]] = []

        for r in results:
            for length, entry in r["date_formats"].items():
                ft = entry["finetype"]
                type_counts[ft] += 1
                if ft == "UNMAPPED":
                    unmapped_patterns.append((r["locale"], length, entry["pattern"]))

        f.write("FineType type distribution:\n")
        for ft, count in sorted(type_counts.items(), key=lambda x: -x[1]):
            f.write(f"  {ft:50s}  {count:4d}\n")
        f.write(f"\n  Total patterns: {sum(type_counts.values())}\n")
        f.write(f"  Mapped:         {sum(v for k, v in type_counts.items() if k != 'UNMAPPED')}\n")
        f.write(f"  Unmapped:       {type_counts.get('UNMAPPED', 0)}\n\n")

        if unmapped_patterns:
            f.write("Unmapped date patterns:\n")
            for locale, length, pattern in unmapped_patterns:
                f.write(f"  {locale:10s} {length:8s}  {pattern}\n")
            f.write("\n")

        # Time format coverage
        f.write("─" * 40 + "\n")
        f.write("TIME FORMAT COVERAGE\n")
        f.write("─" * 40 + "\n\n")

        time_type_counts: dict[str, int] = defaultdict(int)
        for r in results:
            for length, entry in r["time_formats"].items():
                time_type_counts[entry["finetype"]] += 1

        f.write("FineType type distribution:\n")
        for ft, count in sorted(time_type_counts.items(), key=lambda x: -x[1]):
            f.write(f"  {ft:50s}  {count:4d}\n")
        f.write(f"\n  Total patterns: {sum(time_type_counts.values())}\n\n")

        # Month/weekday name coverage
        f.write("─" * 40 + "\n")
        f.write("MONTH & WEEKDAY NAME COVERAGE\n")
        f.write("─" * 40 + "\n\n")

        locales_with_months = sum(1 for r in results if r["months_wide"])
        locales_with_weekdays = sum(1 for r in results if r["weekdays_wide"])
        f.write(f"Locales with month names (wide):      {locales_with_months}\n")
        f.write(f"Locales with month names (abbrev):     "
                f"{sum(1 for r in results if r['months_abbreviated'])}\n")
        f.write(f"Locales with weekday names (wide):     {locales_with_weekdays}\n")
        f.write(f"Locales with weekday names (abbrev):   "
                f"{sum(1 for r in results if r['weekdays_abbreviated'])}\n\n")

        # Format diversity analysis
        f.write("─" * 40 + "\n")
        f.write("FORMAT DIVERSITY PER FINETYPE TYPE\n")
        f.write("─" * 40 + "\n\n")

        type_locales: dict[str, set[str]] = defaultdict(set)
        type_patterns: dict[str, set[str]] = defaultdict(set)
        for r in results:
            for length, entry in r["date_formats"].items():
                ft = entry["finetype"]
                if ft != "UNMAPPED":
                    type_locales[ft].add(r["locale"])
                    type_patterns[ft].add(entry["pattern"])

        for ft in sorted(type_locales.keys()):
            f.write(f"  {ft}:\n")
            f.write(f"    Locales:  {len(type_locales[ft])}\n")
            f.write(f"    Patterns: {len(type_patterns[ft])}\n")
            for p in sorted(type_patterns[ft]):
                f.write(f"      {p}\n")
            f.write("\n")


def main():
    parser = argparse.ArgumentParser(description="Extract CLDR patterns for FineType")
    parser.add_argument("--cldr-dir", default="data/cldr/json",
                        help="Path to CLDR JSON data directory")
    parser.add_argument("--output-dir", default="data/cldr",
                        help="Output directory for extracted data")
    args = parser.parse_args()

    if not os.path.exists(args.cldr_dir):
        print(f"Error: CLDR data not found at {args.cldr_dir}", file=sys.stderr)
        print("Run scripts/download_cldr.sh first.", file=sys.stderr)
        sys.exit(1)

    # Discover available locales
    dates_dir = os.path.join(args.cldr_dir, "cldr-dates-full", "main")
    if not os.path.exists(dates_dir):
        print(f"Error: cldr-dates-full not found at {dates_dir}", file=sys.stderr)
        sys.exit(1)

    available_locales = set(os.listdir(dates_dir))

    # Filter to target locales (and check which CJK locales exist)
    skipped_cjk = []
    target_found = []

    for loc in sorted(available_locales):
        if loc in CJK_LOCALES or loc.startswith(("ja-", "zh-", "ko-", "yue-")):
            skipped_cjk.append(loc)
        elif loc in TARGET_LOCALES:
            target_found.append(loc)

    # Also include any available locale not in CJK (for completeness)
    # but prioritise the target set
    extra_locales = []
    for loc in sorted(available_locales):
        if (loc not in CJK_LOCALES
                and not loc.startswith(("ja-", "zh-", "ko-", "yue-"))
                and loc not in TARGET_LOCALES):
            extra_locales.append(loc)

    # Process target locales first, then extras
    print(f"Processing {len(target_found)} target locales...")
    results = []
    for loc in sorted(target_found):
        r = process_locale(args.cldr_dir, loc)
        if r:
            results.append(r)

    print(f"Processing {len(extra_locales)} additional locales...")
    extra_results = []
    for loc in sorted(extra_locales):
        r = process_locale(args.cldr_dir, loc)
        if r:
            extra_results.append(r)

    all_results = results + extra_results
    print(f"Total locales processed: {len(all_results)}")
    print(f"CJK locales excluded: {len(skipped_cjk)}")

    # Write output files
    os.makedirs(args.output_dir, exist_ok=True)

    write_date_patterns_tsv(all_results,
                            os.path.join(args.output_dir, "cldr_date_patterns.tsv"))
    write_time_patterns_tsv(all_results,
                            os.path.join(args.output_dir, "cldr_time_patterns.tsv"))
    write_month_names_tsv(all_results,
                          os.path.join(args.output_dir, "cldr_month_names.tsv"))
    write_weekday_names_tsv(all_results,
                            os.path.join(args.output_dir, "cldr_weekday_names.tsv"))
    write_report(all_results, skipped_cjk,
                 os.path.join(args.output_dir, "cldr_mapping_report.txt"))

    print(f"\nOutput written to {args.output_dir}/:")
    print(f"  cldr_date_patterns.tsv")
    print(f"  cldr_time_patterns.tsv")
    print(f"  cldr_month_names.tsv")
    print(f"  cldr_weekday_names.tsv")
    print(f"  cldr_mapping_report.txt")


if __name__ == "__main__":
    main()
