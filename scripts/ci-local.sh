#!/bin/bash
# Local CI runner - runs CI checks locally without docker
# Usage: ./scripts/ci-local.sh [check|version|release]

set -euo pipefail

CMD="${1:-check}"

echo "=== Local CI: $CMD ==="

require_bun() {
  if ! command -v bun >/dev/null 2>&1; then
    echo "bun is required for frontend checks" >&2
    exit 1
  fi
}

case $CMD in
  check)
    echo "Running check jobs..."
    
    echo "[1/9] Format check"
    cargo fmt --check
    
    echo "[2/9] Clippy lint"
    cargo clippy --workspace
    
    echo "[3/9] Workspace check"
    cargo check --workspace
    
    echo "[4/9] Release build"
    cargo build --release -p edgerun-node

    echo "[5/9] Frontend dependency lock check"
    require_bun
    (cd frontend && bun install --frozen-lockfile)

    echo "[6/9] Frontend lint"
    (cd frontend && bun run lint)

    echo "[7/9] Frontend typecheck"
    (cd frontend && bun run typecheck)

    echo "[8/9] Frontend tests"
    (cd frontend && bun run test:run)

    echo "[9/9] Frontend build"
    (cd frontend && bun run build)
    
    echo "✓ Check passed"
    ;;

  version)
    echo "Analyzing API changes..."
    
    # Find last tag
    LAST_TAG=$(git tag -l 'v*.*.*' --sort=-version:refname 2>/dev/null | head -1 || echo "")
    
    if [ -z "$LAST_TAG" ]; then
      echo "No previous release, starting at 0.1.0"
      V="0.1.0"
      TYPE="minor"
    else
      V="${LAST_TAG#v}"
      echo "Last: v$V"
      
      CHANGED=$(git diff --name-only "$LAST_TAG" -- 'crates/*/src/lib.rs' 2>/dev/null | cut -d/ -f2 | sort -u || true)
      
      if [ -z "$CHANGED" ]; then
        PATCH=$(echo $V | cut -d. -f3)
        PATCH=$((PATCH + 1))
        V="$(echo $V | cut -d. -f1,2).${PATCH}"
        TYPE="patch"
      else
        # Check with cargo-public-api if available
        if command -v cargo-public-api &> /dev/null; then
          BREAKING=false
          NEW_API=false
          
          for CRATE in $CHANGED; do
            [ -d "crates/$CRATE" ] || continue
            grep -q 'publish = false' "crates/$CRATE/Cargo.toml" 2>/dev/null && continue
            
            cargo public-api dump -p "$CRATE" --no-private-items 2>/dev/null > /tmp/current.txt || continue
            
            git show "$LAST_TAG:crates/$CRATE/src/lib.rs" 2>/dev/null > /tmp/prev.rs
            [ -s /tmp/prev.rs ] || { NEW_API=true; continue; }
            
            cp crates/$CRATE/src/lib.rs /tmp/cur.rs
            git checkout "$LAST_TAG" -- "crates/$CRATE/src/lib.rs"
            cargo public-api dump -p "$CRATE" --no-private-items 2>/dev/null > /tmp/old.txt
            cp /tmp/cur.rs crates/$CRATE/src/lib.rs
            
            [ -s /tmp/old.txt ] && [ -s /tmp/current.txt ] || continue
            
            REMOVED=$(diff /tmp/old.txt /tmp/current.txt 2>/dev/null | grep '^<' | grep -vE '^< (pub |//)' | wc -l || echo 0)
            ADDED=$(diff /tmp/old.txt /tmp/current.txt 2>/dev/null | grep '^>' | grep -vE '^\+ (pub |//)' | wc -l || echo 0)
            
            [ "$REMOVED" -gt 0 ] && BREAKING=true
            [ "$ADDED" -gt 0 ] && NEW_API=true
          done
          
          MAJOR=$(echo $V | cut -d. -f1)
          MINOR=$(echo $V | cut -d. -f2)
          PATCH=$(echo $V | cut -d. -f3)
          
          if [ "$BREAKING" = true ]; then
            MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0; TYPE="major"
          elif [ "$NEW_API" = true ]; then
            MINOR=$((MINOR + 1)); PATCH=0; TYPE="minor"
          else
            PATCH=$((PATCH + 1)); TYPE="patch"
          fi
          
          V="${MAJOR}.${MINOR}.${PATCH}"
        else
          # Fallback: patch bump
          PATCH=$(echo $V | cut -d. -f3)
          PATCH=$((PATCH + 1))
          V="$(echo $V | cut -d. -f1,2).${PATCH}"
          TYPE="patch"
        fi
      fi
    fi
    
    echo "Version: v$V ($TYPE)"
    
    # Update versions
    for toml in Cargo.toml crates/*/Cargo.toml; do
      [ -f "$toml" ] || continue
      grep -q 'publish = false' "$toml" 2>/dev/null && continue
      sed -i "s|^version = \".*\"|version = \"${V}\"|" "$toml"
    done
    
    git config --local user.email "ci@edgerun.ai"
    git config --local user.name "Edgerun CI"
    git add -A
    
    if ! git diff --cached --quiet; then
      git commit -m "release: v${V} (${TYPE})"
      git tag -a "v${V}" -m "Release v${V}"
      echo "Created v${V} (use 'git push origin v${V}' to publish)"
    else
      echo "No changes"
    fi
    ;;

  release)
    echo "Building release..."
    TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "v0.0.0")
    V="${TAG#v}"
    
    cargo build --release -p edgerun-node
    
    # Find the binary (could be in musl or gnu target dir)
    for binary in target/x86_64-unknown-linux-musl/release/edgerund target/release/edgerund; do
      if [ -f "$binary" ]; then
        mkdir -p release
        cp "$binary" "release/edgerund-${V}-linux-amd64"
        chmod +x "release/edgerund-${V}-linux-amd64"
        echo "Release: release/edgerund-${V}-linux-amd64"
        break
      fi
    done
    ;;

  *)
    echo "Usage: $0 [check|version|release]"
    exit 1
    ;;
esac

echo "=== Done ==="
