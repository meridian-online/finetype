#!/usr/bin/env bash
# scripts/fetch_geonames.sh — idempotent GeoNames snapshot fetch + register.
#
# Per spec 2026-05-24-v21-geonames-geography ac-01.
#
# Downloads the GeoNames dumps needed by the v21 geography generator to
# $HOME/datasets/geonames/<YYYY-MM-DD>/, extracts each, and invokes
# scripts/dataset_register.py to write the snapshot + sources.yaml entry.
#
# Idempotent: re-running with the same date is a no-op once every
# expected file is present. Re-running with a new date fetches a fresh
# snapshot to a new dated directory (old snapshots are preserved).
#
# Usage:
#   ./scripts/fetch_geonames.sh                # snapshot today
#   ./scripts/fetch_geonames.sh 2026-05-24     # snapshot a specific date
#   ./scripts/fetch_geonames.sh --skip-register   # download + extract only
#
# Disk: ~920 MB extracted (zips removed after successful extraction).
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO"

DATE=""
SKIP_REGISTER=false
for arg in "$@"; do
    case "$arg" in
        --skip-register) SKIP_REGISTER=true ;;
        --help|-h)
            sed -n '2,/^set -/p' "$0" | grep '^#' | sed 's/^# \?//'; exit 0
            ;;
        -*) echo "Unknown flag: $arg" >&2; exit 1 ;;
        *)  DATE="$arg" ;;
    esac
done
DATE="${DATE:-$(date +%Y-%m-%d)}"

DEST_ROOT="/Users/hugh/datasets/geonames"
DEST="$DEST_ROOT/$DATE"
mkdir -p "$DEST"
cd "$DEST"

BASE="https://download.geonames.org/export/dump"
ZIP_BASE="https://download.geonames.org/export/zip"

# fetch <url> <local_basename> [extracted-marker]
# Skips download if the local zip OR its extracted marker is already present.
fetch() {
    local url="$1" out="$2" marker="${3:-}"
    if [[ -n "$marker" && -f "$marker" ]]; then
        echo "  cached (extracted): $marker"
        return
    fi
    if [[ -f "$out" ]]; then
        echo "  cached (zip):       $out"
        return
    fi
    echo "  fetch:              $url"
    curl -sSL --fail --retry 3 --retry-delay 5 -o "$out.tmp" "$url"
    mv "$out.tmp" "$out"
}

# extract_zip <zip> <expected-extracted-file> [destdir]
# Skips extraction if expected file already exists. Keeps the zip alongside
# so subsequent runs see a complete file set and don't re-download.
extract_zip() {
    local zip="$1" expected="$2" destdir="${3:-.}"
    if [[ -f "$destdir/$expected" ]]; then
        echo "  extracted: $destdir/$expected"
        return
    fi
    if [[ ! -f "$zip" ]]; then
        echo "error: $zip missing and $destdir/$expected not extracted" >&2
        return 1
    fi
    echo "  unzip:    $zip -> $destdir/"
    mkdir -p "$destdir"
    unzip -q -o "$zip" -d "$destdir"
    # Drop readme.txt files shipped inside the zips — we keep our own
    # snapshot manifest + the upstream URLs in sources.yaml.
    [[ -f "$destdir/readme.txt" ]] && rm -f "$destdir/readme.txt"
}

echo "── Fetch GeoNames dumps to $DEST ─────────────────────────────────"
fetch "$BASE/allCountries.zip"          allCountries.zip            allCountries.txt
fetch "$BASE/alternateNamesV2.zip"      alternateNamesV2.zip        alternateNamesV2.txt
fetch "$BASE/cities500.zip"             cities500.zip               cities500.txt
fetch "$BASE/admin1CodesASCII.txt"      admin1CodesASCII.txt
fetch "$BASE/admin2Codes.txt"           admin2Codes.txt
fetch "$BASE/countryInfo.txt"           countryInfo.txt
fetch "$BASE/featureCodes_en.txt"       featureCodes_en.txt
fetch "$BASE/iso-languagecodes.txt"     iso-languagecodes.txt
fetch "$ZIP_BASE/allCountries.zip"      postalCodes_allCountries.zip postal/allCountries.txt

echo "── Extract zipped dumps ──────────────────────────────────────────"
extract_zip allCountries.zip          allCountries.txt
extract_zip alternateNamesV2.zip      alternateNamesV2.txt
extract_zip cities500.zip             cities500.txt
extract_zip postalCodes_allCountries.zip allCountries.txt postal

# Show final inventory + disk usage
echo
echo "── Inventory ─────────────────────────────────────────────────────"
( cd "$DEST" && du -sh . && find . -type f -printf "  %s\t%P\n" 2>/dev/null \
    || find . -type f | xargs -I{} sh -c 'printf "  %s\t%s\n" "$(stat -f %z "$1")" "${1#./}"' _ {} )

if [[ "$SKIP_REGISTER" == "true" ]]; then
    echo
    echo "── Skipping registration (--skip-register) ──────────────────────"
    exit 0
fi

echo
echo "── Register snapshot ─────────────────────────────────────────────"
cd "$REPO"
python3 scripts/dataset_register.py geonames "$DEST" \
    --dataset-version "$DATE" \
    --role train --licence CC-BY-4.0 \
    --attribution "GeoNames gazetteer ($DATE snapshot) — global gazetteer of geographic features (cities, admin divisions, postal codes, multilingual alternate names) sourced from geonames.org under CC-BY 4.0. Used by FineType as the v21+ training-data source for geography.location.* and geography.address.postal_code types per spec 2026-05-24-v21-geonames-geography. Attribution: 'Source: GeoNames (geonames.org), CC BY 4.0'."

echo
echo "── Verify ────────────────────────────────────────────────────────"
python3 scripts/dataset_verify.py geonames
