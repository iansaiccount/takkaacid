#!/usr/bin/env bash
# The guide's only test. It builds the book and proves five things:
#   1. the book builds (which also proves every SUMMARY entry names a page that exists);
#   2. every source page is reachable from SUMMARY.md;
#   3. every image a page references exists, and every page a page links to exists;
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

# 4. the facts that must not drift (phase 2's rack, card, colour and clipboard; the themes' source and its credit; phase 3's
#    deck, pause, transpose row, banks and copy/paste; phase 4's drum machine and the drum banks its demos carry)
! grep -rqF 'saved on exit and 2 s after a change' src || { echo "the guide repeats the stale save line"; exit 1; }
# The phase replaced the saved word and moved the name out of the middle; both are claims a rewrite can put back, and neither is
# a positive fact, so each needs its own guard.
! grep -rqE 'on (its )?lit plate' src || { echo "the guide promises a plate the face does not paint"; exit 1; }
! grep -rqF 'in the middle of the top strip' src || { echo "the guide puts the name back in the middle"; exit 1; }
! grep -rqE '`(RUN|STOP)` (is|stays) lit|`SAVED` / `UNSAVED`' src || { echo "the guide describes the retired transport keys"; exit 1; }
# A device no longer comes in on the step the rack happens to be on: it waits for the bar (plan 04).
! grep -rqF 'comes in on the step the others are on' src || { echo "the guide has a device joining mid-bar again"; exit 1; }
# The same kind of miss, three more ways: a retired word printed as the face spells it on a page whose subject was the saved
# state, the bank browser under its old name, and the stop's rule told as the pause's. Each is narrowed to the sentence that
# was the lie — `UNSAVED` is still engraved on the browser's header, and a stop really does return to the top of the bar.
! grep -rqF 'not yet on disk, `UNSAVED`' src || { echo "the guide spells a word the face no longer prints"; exit 1; }
! grep -rqF 'MY SONGS' src || { echo "the guide names the bank browser as it was, not as it is"; exit 1; }
! grep -rqF 'the next start begins from the top of the bar' src || { echo "the guide gives the pause the stop's rule"; exit 1; }
! grep -rqE 'hears it|walked by ear|identify by ear' src || { echo "the guide sells a selection by ear"; exit 1; }
for word in 'manual by default' 'xdg-desktop-portal' 'MIDI: none (retrying)' 'RETRY' '--midi-map' \
            'Ctrl+1' 'F3' 'COLOUR' 'Ctrl+C' 'NOT SAVED YET' 'four machines' 'VINYL NOIR' \
            'media.io/color-palette/music-color-palette.html' "that page's work" \
            'it makes no sound until the bar the rack is in finishes' 'NOTHING TO PAUSE' \
            'BANKS' 'PAUSE' 'REVERSE' 'TRANS -' 'COPY' 'PASTE' \
            '`KIT`' '`MY KITS`' 'Acoustic' \
            'the whole repair' \
            'the next play carries on from there' 'tape deck' 'writes as you go' 'DRUMS' \
            'drum bank' 'KICK' 'LP | BP | HP' 'drum machine' \
            'walk it up and down the grid' 'strikes the selected voice' \
            'move the selected step' 'writes the selected voice into it'; do
  grep -rqF -- "$word" src || { echo "the guide never says: $word"; exit 1; }
done
echo "guide: ok"
