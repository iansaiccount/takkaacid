# Songs and demos

`BANKS` opens the browser that keeps your work: a name field with `SAVE` and `SAVE A COPY` beside it, four lists, and four
keys along the bottom. It keeps three different amounts of work: a **song** is the whole rack, a **pattern bank** is sixty-four
patterns and the machine they were written on, and a **rack** is a document of its own that this build only has keys for.

## The four lists
**`DEMO BANKS`** are the five that ship inside the app: **Slow Acid**, **Hard Techno Acid**, **Blues**, **Rock** and
**Funk**. They are read-only — there is no way to overwrite one — and each is a patch plus eight patterns in `A1` to `A8`
([`11-the-bank`](11-the-bank.md)). Clicking one loads it as a **new** song: named `New song`, not yet on disk — its name
sits in amber until you save it, which is how the face says "not yet written".

On a drum machine the same five rows load that demo's **drum bank** instead: eight grooves into `A1` to `A8`, at the
demo's tempo and shuffle, with that machine's kit and the session's name left exactly as they were. The tempo is the whole
rack's, though — one cell, every machine on it — so loading a bank also retunes the 303s beside it, and that tempo, unlike a
second device's grooves, is what a save keeps. A drum bank is patterns, not a song — nothing is renamed and no sound is loaded with it.
A save keeps every machine in the rack ([`18-a-rack-of-devices`](18-a-rack-of-devices.md)), so a drum machine's grooves are
kept whichever cell it sits in. [`22-the-drum-machine`](22-the-drum-machine.md) is the drum machine's own page, its grid and
its kits.

**`PATTERN BANKS`** are the banks you have saved: sixty-four patterns each, `A1` to `H8`, and the machine the bank was written
on. A row loads that bank into the **selected device**, replacing all sixty-four of its patterns — what a demo row has always done
to a drum machine's grid, now with a file of your own. If what is on screen has unsaved changes, the first click only arms the row
— it prints `<name> - click again to replace the bank` — and the second loads it. A bank written on the other machine loads and
plays nothing; the status line says so and names the machine the file names.

`SAVE BANK`, under those rows, keeps the selected device's bank under a name, and its field behaves as the song's does: the
bank's own name rewrites that file, a different name writes a new one — beside a taken one, `Easy (2)`, so a bank never destroys
another by being given its name — and a blank field saves the bank the device came from. A bank's door is on its own heading
rather than in the footer, and there is no `IMPORT` or `EXPORT` for one: `banks/` is the folder, and a file dropped into it loads.

**`SONGS`** are the saves you have named. Click a row to load one. If what is on screen has unsaved changes, the first
click only arms the row — it prints `<name> - click again to replace` — and the second click loads it.

Every row of `PATTERN BANKS` and `SONGS` has a small `x` at its right end. One click turns it into `SURE`; the second deletes
that file. The demos have no `x`.

**`RACKS`** holds `SAVE RACK` and `LOAD RACK`, and it is the one place on the card that does nothing yet: a rack of its own —
the machines, their patches and their banks as one document — is a later round. On this build both keys say so on the status
line and open nothing.

## One song, the whole rack

A song is the rack: every machine's kind, name, colour, ports, channel, role, patterns, patch, kit's name, shuffle and learned
controllers, with device 1 also written in the older flat shape so a build from before the rack can still read it. A save
therefore keeps a second, third or fourth machine as it keeps the first, and a restart brings all of them back in the order
they were in ([`18-a-rack-of-devices`](18-a-rack-of-devices.md)).

Two things each machine still forgets: its `REVERSE` direction and its own transpose. Both are performance, not patch, so a
restart brings every device back running forwards and in tune. A kit's *folder* is written beside the file, so an edited kit
comes back with the machine that plays it.

## What a save does to a name

This is the part worth reading twice, because it is where work is kept or lost. **Saving is manual by default** — the
name stays amber until you do it — and what a save writes depends entirely on the name field.

| The name field holds | What `SAVE` does |
|---|---|
| the name of the song you are on | writes that song, in place |
| a different name | writes a **new** file and leaves the old one where it is. If the new name is already taken, it lands beside it — `Acid 1 (2)` — so a save can never destroy a song by being given its name |
| nothing | saves the song the session came from, under its own name. A session that never had a file has nothing to write to, and `SAVE` puts the cursor in the field instead |

**`SAVE A COPY`** always makes a second song under a free name, and never touches the original. That is the safe way to
branch: load a demo, hit `SAVE A COPY`, and rename the copy.

Names are trimmed, may be up to 48 characters, and may not contain `/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|`, or start
with a dot — they become file names in `songs/` or `banks/`.

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

The band's left cluster, after the `?`, names the session — `SONG New song`, `SONG Acid 1`. When there is something to say, the
notice takes that space for a few seconds and then the name comes back: `saved Acid 2`, `could not import: …`. Nothing
needs dismissing, and nothing is lost when the card closes, because the notice lives on the strip rather than in the card
that caused it.

[`13-settings`](13-settings.md) is the other card.
