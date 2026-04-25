#!/bin/bash
# Version management script for edgerun_core
# Usage: ./scripts/version.sh patch|minor|major [version]

set -e

# Read current version from Cargo.toml
VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

# Extract major.minor.patch
MAJOR=$(echo $VERSION | cut -d. -f1)
MINOR=$(echo $VERSION | cut -d. -f2)
PATCH=$(echo $VERSION | cut -d. -f3 | cut -d- -f1)

case "${1:-patch}" in
    major)
        MAJOR=$((MAJOR + 1))
        MINOR=0
        PATCH=0
        ;;
    minor)
        MINOR=$((MINOR + 1))
        PATCH=0
        ;;
    patch)
        PATCH=$((PATCH + 1))
        ;;
    set)
        VERSION="$2"
        ;;
esac

NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"

# Update workspace Cargo.toml
sed -i "s/^version = \".*\"/version = \"${NEW_VERSION}\"/" Cargo.toml

# Update each crate's version to match workspace
for toml in crates/*/Cargo.toml; do
    sed -i "s/^version = \".*\"/version = \"${NEW_VERSION}\"/" "$toml"
done

echo "Version updated: ${VERSION} -> ${NEW_VERSION}"
git diff Cargo.toml | head -20