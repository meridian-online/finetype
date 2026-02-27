#!/usr/bin/env bash
# Download CLDR JSON packages for FineType training data enrichment.
#
# Fetches cldr-dates-full and cldr-numbers-full npm packages to data/cldr/json/.
# These provide locale-specific date/time format patterns, month/day names,
# and number formatting rules used by extract_cldr_patterns.py.
#
# Usage: ./scripts/download_cldr.sh
#
# Version pinned to CLDR 46.0.0 (Unicode 16.0, 2024-10-28).
# To update: change CLDR_VERSION below and re-run.

set -euo pipefail

CLDR_VERSION="46.0.0"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CLDR_DIR="$REPO_ROOT/data/cldr/json"

PACKAGES=(
    "cldr-dates-full"
    "cldr-numbers-full"
    "cldr-core"
)

echo "=== CLDR JSON Download ==="
echo "Version: $CLDR_VERSION"
echo "Target:  $CLDR_DIR"
echo ""

# Check for curl
if ! command -v curl &> /dev/null; then
    echo "Error: curl is required but not installed." >&2
    exit 1
fi

# Clean previous download
if [ -d "$CLDR_DIR" ]; then
    echo "Removing previous CLDR data..."
    rm -rf "$CLDR_DIR"
fi
mkdir -p "$CLDR_DIR"

# Download each package from npm registry
for pkg in "${PACKAGES[@]}"; do
    echo "Downloading $pkg@$CLDR_VERSION..."

    TARBALL_URL="https://registry.npmjs.org/$pkg/-/$pkg-$CLDR_VERSION.tgz"
    TARBALL="$CLDR_DIR/$pkg.tgz"

    curl -sSfL "$TARBALL_URL" -o "$TARBALL"

    # Extract — npm tarballs have a `package/` prefix
    echo "  Extracting..."
    mkdir -p "$CLDR_DIR/$pkg"
    tar xzf "$TARBALL" -C "$CLDR_DIR/$pkg" --strip-components=1
    rm "$TARBALL"

    echo "  Done."
done

# Write version manifest
cat > "$CLDR_DIR/VERSION" <<EOF
cldr-version: $CLDR_VERSION
downloaded: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
packages: ${PACKAGES[*]}
EOF

echo ""
echo "=== Download complete ==="
echo ""

# Quick stats
for pkg in "${PACKAGES[@]}"; do
    if [ "$pkg" = "cldr-dates-full" ]; then
        LOCALE_COUNT=$(find "$CLDR_DIR/$pkg/main" -maxdepth 1 -type d | wc -l)
        echo "$pkg: $((LOCALE_COUNT - 1)) locales"
    elif [ "$pkg" = "cldr-numbers-full" ]; then
        LOCALE_COUNT=$(find "$CLDR_DIR/$pkg/main" -maxdepth 1 -type d | wc -l)
        echo "$pkg: $((LOCALE_COUNT - 1)) locales"
    elif [ "$pkg" = "cldr-core" ]; then
        echo "$pkg: supplemental data"
    fi
done
echo ""
echo "Version manifest: $CLDR_DIR/VERSION"
