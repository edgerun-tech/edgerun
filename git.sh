#!/usr/bin/env bash
set -euo pipefail

git stash
git pull --rebase
if git stash apply; then
  git add .
  git commit -am "chore: update $(date +%Y-%m-%d)" || echo "nothing to commit"
else
  echo "conflicts - resolve manually then commit"
  exit 1
fi
git push
