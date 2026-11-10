# Songs and demos

`SONGS` opens the card that keeps your work: a name field with `SAVE` and `SAVE A COPY` beside it, two lists, and four
keys along the bottom.

## The two lists

**`DEMO SONGS`** are the five that ship inside the app: **Slow Acid**, **Hard Techno Acid**, **Blues**, **Rock** and
**Funk**. They are read-only — there is no way to overwrite one — and each is a patch plus eight patterns in `A1` to `A8`
([`11-the-bank`](11-the-bank.md)). Clicking one loads it as a **new** song: named `New song`, not yet on disk, `UNSAVED`.

**`MY SONGS`** are the songs you have named. Click a row to load one. If what is on screen has unsaved changes, the first
click only arms the row — it prints `<name> - click again to replace` — and the second click loads it.

Every row has a small `x` at its right end. One click turns it into `SURE`; the second deletes the song. The demos have no
`x`.

## What a save does to a name

This is the part worth reading twice, because it is where work is kept or lost. **Saving is manual by default** — the
lamp says `UNSAVED` until you do it — and what a save writes depends entirely on the name field.

| The name field holds | What `SAVE` does |
|---|---|
| the name of the song you are on | writes that song, in place |
| a different name | writes a **new** file and leaves the old one where it is. If the new name is already taken, it lands beside it — `Acid 1 (2)` — so a save can never destroy a song by being given its name |
| nothing | saves the song the session came from, under its own name. A session that never had a file has nothing to write to, and `SAVE` puts the cursor in the field instead |

**`SAVE A COPY`** always makes a second song under a free name, and never touches the original. That is the safe way to
branch: load a demo, hit `SAVE A COPY`, and rename the copy.

Names are trimmed, may be up to 48 characters, and may not contain `/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|`, or start
with a dot — they become file names in `songs/`.

## The buttons

- **`NEW`** throws the session away and starts from the starter phrase. While the session is unsaved it reads `SURE?`
  first.
- **`SAVE`** and **`SAVE A COPY`** as above.
- **`IMPORT`** copies a song file into your library and loads it. It never overwrites: an imported name that is taken
  lands beside the existing one.
- **`EXPORT`** writes the session out to a file you choose, in exactly the format the library uses — so an exported file
  can be dropped straight into `songs/` and will load there.
- **`CLOSE`** puts the card away.

`IMPORT` and `EXPORT` are the only two file dialogs in the app, and they are drawn by the system's portal rather than by
takka-acid. On a machine without `xdg-desktop-portal` and a backend behind it, those two buttons have no dialog to open
([`02-install`](02-install.md) lists it as a prerequisite). Everything else on the card works regardless.

## Messages and the strip

The middle of the top strip names the session — `SONG New song`, `SONG Acid 1`. When there is something to say, the
notice takes that space for a few seconds and then the name comes back: `saved Acid 2`, `could not import: …`. Nothing
needs dismissing, and nothing is lost when the card closes, because the notice lives on the strip rather than in the card
that caused it.

[`13-settings`](13-settings.md) is the other card.
