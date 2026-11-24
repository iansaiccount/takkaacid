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
#    deck, pause, transpose row, banks and copy/paste; phase 4's drum machine, the playhead on both faces, and
#    the drum banks its demos carry)
# Every negative guard reads the book with its line breaks flattened to spaces, because a sentence a page wraps is still the
# sentence it was: page 12 once printed `NAMES AND EXTRA DEVICES ARE NOT` and then `SAVED YET`, and a line-based guard for that
# phrase stayed green while the app had stopped printing it. `gone` is the one negative guard; `gone_re` is the same for a
# pattern.
flat="$(mktemp)"
trap 'rm -f "$flat"' EXIT
cat src/*.md | tr '\n' ' ' >"$flat"
gone() { ! grep -qF -- "$1" "$flat" || { echo "$2"; exit 1; }; }
gone_re() { ! grep -qE -- "$1" "$flat" || { echo "$2"; exit 1; }; }

gone 'saved on exit and 2 s after a change' "the guide repeats the stale save line"
# The phase replaced the saved word and moved the name out of the middle; both are claims a rewrite can put back, and neither is
# a positive fact, so each needs its own guard.
gone_re 'on (its )?lit plate' "the guide promises a plate the face does not paint"
gone 'in the middle of the top strip' "the guide puts the name back in the middle"
gone_re '`(RUN|STOP)` (is|stays) lit|`SAVED` / `UNSAVED`' "the guide describes the retired transport keys"
# A device no longer comes in on the step the rack happens to be on: it waits for the bar (plan 04).
gone 'comes in on the step the others are on' "the guide has a device joining mid-bar again"
# The same kind of miss, three more ways: a retired word printed as the face spells it on a page whose subject was the saved
# state, the bank browser under its old name, and the stop's rule told as the pause's. Each is narrowed to the sentence that
# was the lie — `UNSAVED` is still engraved on the browser's header, and a stop really does return to the top of the bar.
gone 'not yet on disk, `UNSAVED`' "the guide spells a word the face no longer prints"
gone 'MY SONGS' "the guide names the bank browser as it was, not as it is"
gone 'the next start begins from the top of the bar' "the guide gives the pause the stop's rule"
gone_re 'hears it|walked by ear|identify by ear' "the guide sells a selection by ear"
# The drum machine's nine controls did print their captions and nothing more, and this round gave every one of them a tip: the
# bullet that said so is deleted from page 22, and this guard is why a rewrite cannot put the claim back.
gone 'No per-voice hover help yet' "the guide says the drum controls have no hover help"
# The transpose was one number for the rack; it is the device on the panel's now. The old claim cannot come back, and the new one
# is pinned below.
gone 'moves the whole rack' "the guide makes the transpose rack-wide again"
gone 'the whole rack forgets together' "the guide says the rack forgets one transpose"
# A song keeps every device now, so the picker's apology and the "first device only" file table are lies the guide may not tell
# again. The new sentences are pinned in the list below.
gone 'NAMES AND EXTRA DEVICES ARE NOT SAVED YET' "the guide apologises for a format that saves everything"
gone 'the first device only' "the guide says the session file holds one device"
# The arrows split at the modifier, and no page may go back to calling the plain pair the tempo's or the shifted pair the
# note's. The pins below keep the new wording; these keep the old one out.
gone 'plain up/down' "the guide puts the tempo on the plain arrows again"
# Phase 5's second round renamed one heading and added two sections, and the words are the point of that round: the card says
# SONGS, PATTERN BANKS and RACKS, and no page may go back to the name a song list had while it was the only list.
gone 'MY BANKS' "the guide names a section the card no longer prints"
# Phase 6 turned the two undecided cells into `GRID` and its readout: no page may still call them TBC, and the three grid
# captions, the key's name and the two new demos are the facts the round is about (`23-pattern-grids`).
gone 'TBC' "the guide still names the undecided cells"
gone 'five demo' "the guide still counts five demos"
# Anchored so the *correct* card line (`shift+up / down: tempo by 1 BPM`, which a page may quote) cannot trip it: the
# character before `up` must not be a `+`.
gone_re '(^|[^+])up / down: tempo' "the guide gives the tempo the plain arrows again"
for word in 'manual by default' 'xdg-desktop-portal' 'MIDI: none (retrying)' 'RETRY' '--midi-map' \
            'Ctrl+1' 'F3' 'COLOUR' 'Ctrl+C' 'four machines' 'VINYL NOIR' \
            'every machine in the rack' 'theme is saved with the device' \
            'media.io/color-palette/music-color-palette.html' "that page's work" \
            'it makes no sound until the bar the rack is in finishes' 'NOTHING TO PAUSE' \
            'BANKS' 'PAUSE' 'REVERSE' 'TRANS -' 'COPY' 'PASTE' \
            '`KIT`' '`MY KITS`' 'Acoustic' \
            'the whole repair' \
            'the next play carries on from there' 'tape deck' 'writes as you go' 'DRUMS' \
            'drum bank' 'KICK' 'LP | BP | HP' 'drum machine' \
            'walk it up and down the grid' 'strikes the selected voice' \
            'move the selected step' 'writes the selected voice into it' \
            '`l`, `b` and `p`' 'control is underlined on it' 'stopped or running' \
            'moves **the device on the panel**' \
            'the step that sounds, not the step the clock is on' \
            'runs the column right to left with the pattern' \
            '| `shift+up` / `shift+down` | tempo' \
            'PATTERN BANKS' 'SAVE BANK' 'SAVE RACK' 'LOAD RACK' 'banks/<name>.json' \
            '1/16' '1/32' '1/8T' 'GRID' 'Acid Jazz' 'Fast Acid' 'newer' \
            'Every control in the row explains itself'; do
  grep -rqF -- "$word" src || { echo "the guide never says: $word"; exit 1; }
done
echo "guide: ok"
