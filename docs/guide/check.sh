#!/usr/bin/env bash
# The guide's only test. It builds the book and proves four things:
#   1. the book builds;
#   2. every source page is reachable from SUMMARY.md;
#   3. every image a page references exists;
#   4. the facts that must not drift are present, and the stale save line is gone.
set -euo pipefail
cd "$(dirname "$0")"
mdbook="${MDBOOK:-$HOME/.cargo/bin/mdbook}"

# 1. the book builds
"$mdbook" build >/tmp/guide-build.log 2>&1 || { cat /tmp/guide-build.log; exit 1; }

# 2. every page is reachable from SUMMARY.md, and every SUMMARY entry exists
for page in src/*.md; do
  [ "$page" = src/SUMMARY.md ] && continue
  grep -qF "(${page#src/})" src/SUMMARY.md || { echo "not in SUMMARY: $page"; exit 1; }
done

# 3. every image a page references exists
#    (`|| true` because a book with no images yet is not a broken book, and pipefail would abort on grep's 1)
{ grep -rhoE 'img/[A-Za-z0-9._-]+\.png' src --include='*.md' || true; } | sort -u | while read -r img; do
  [ -f "src/$img" ] || { echo "missing image: src/$img"; exit 1; }
done
# 3b. every page a page links to exists
#     (a cross-reference is written as [`01-what-it-is`](01-what-it-is.md); a typo there is a dead link no reader can
#     follow, and check 2 only guards the table of contents)
{ grep -rhoE '\]\([A-Za-z0-9._-]+\.md\)' src --include='*.md' || true; } | sed 's/^](//; s/)$//' | sort -u |
  while read -r page; do
    [ -f "src/$page" ] || { echo "missing page: src/$page"; exit 1; }
  done

# 4. the facts that must not drift (spec Review Focus 1, 2, 3, 5)
! grep -rqF 'saved on exit and 2 s after a change' src || { echo "the guide repeats the stale save line"; exit 1; }
for word in 'manual by default' 'xdg-desktop-portal' 'MIDI: none (retrying)' 'RETRY' '--midi-map'; do
  grep -rqF -- "$word" src || { echo "the guide never says: $word"; exit 1; }
done
echo "guide: ok"
