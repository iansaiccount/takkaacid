# Songs and demos

`BANKS` opens the browser that keeps your work: a search field, one row of chips that narrow the list by kind, one list of
everything the card knows, and four keys along the bottom. It keeps three different amounts of work: a **song** is the whole
rack, a **pattern bank** is sixty-four patterns and the machine they were written on, and a **rack** is every machine with its
patches and banks, as a file of its own.

## One list you can search

Every row carries its kind in its own column — `SONG`, `BANK`, `RACK`, `DEMO` — so a row says what it is before you click it. The
kinds always come in that order: your songs, then the pattern banks, then the racks, then the seven demos.

A **search field** sits under the title, above the chips. Type into it and the list narrows to the rows whose name contains what
you typed, whatever the case; clear it and everything comes back. It reads names only — no guessing, no ranking — so what it
finds is always what you can see.

The chips are one row of keys, and one of them is lit: `ALL` `SONGS` `BANKS` `RACKS` `DEMOS`. `ALL` shows every kind and is where
the card opens; a chip shows one. The chip and the search compose, so `BANKS` with `easy` typed is the banks whose name holds
"easy". **A change of chip or of search puts the list back at the top**, because it is a different list.

The card remembers neither: close it and it opens again on `ALL` with nothing typed.

## The strip saves the kind you are looking at

Under the list is one row — a name field and the keys that save. **The strip under the list follows the chip you are on:**

| the chip | the field | the keys |
|---|---|---|
| `ALL`, `SONGS`, `DEMOS` | the song's name | `SAVE`, `SAVE A COPY` |
| `BANKS` | the bank's name | `SAVE BANK` |
| `RACKS` | the rack's name | `SAVE RACK` |

Every key names its own verb, so no key's meaning depends on where it sits. `ALL` and `DEMOS` show the song's door because the
session is what is always open; `BANKS` and `RACKS` show their own because that is the thing you are looking at.

The cost, stated: a player on `BANKS` who wants to save the **song** clicks `ALL` or `SONGS` first. That is one click, with the
chips on screen, and it buys a much better thing — the strip and the footer never scroll, so a door is always reachable however
long the list is.

## When the list is longer than the band

The list is one band of a fixed height, and it scrolls. Wheel over it and the rows move; a thin mark down its right edge says
there is more of it, and the mark is drawn only when there is — a library that fits looks like a short list with no mark at all.
Nineteen rows fit at the design size.

A row the band cannot show is reached by scrolling to it, never by pushing a key off the card. That is why `SAVE BANK` and
`SAVE RACK` live in the strip rather than under their own rows.

When the chip and the search leave nothing, the band says which nothing it is: `NOTHING MATCHES` when you have typed something,
and `NO SONGS YET` / `NO BANKS YET` / `NO RACKS YET` when a chip on its own is empty. The strip still saves under an empty list.

## Every key explains itself

**Every key on the card explains itself on hover**, and the keys that write something say *which*: the key's caption is a verb, and
the hint is the whole sentence, including the name it would write. With `Easy 2` in the bank's field, `SAVE BANK` says
`Makes a new file, Easy 2.`; with the field blank and the device on a bank it says `Writes Easy in place.` A row's `x` says which
file it would delete, and `EXPORT` says which session it would write.

The hint follows the `TOOLTIPS ON` switch in the settings card, like every other hint on the panel: turn it off and the keys go
quiet, and the card still works.

## What each kind of row does

**A `SONG` row** is a save you have named. Click it to load one. If what is on screen has unsaved changes, the first click only
arms the row — it prints `<name> - click again to replace` — and the second click loads it.

**A `BANK` row** is a pattern bank you have saved: sixty-four patterns, `A1` to `H8`, and the machine the bank was written on. A
row loads that bank into the **selected device**, replacing all sixty-four of its patterns — what a demo row has always done to a
drum machine's grid, now with a file of your own. If what is on screen has unsaved changes, the first click only arms the row —
it prints `<name> - click again to replace the bank` — and the second loads it. A bank written on the other machine loads and
plays nothing; the status line says so and names the machine the file names.

**A `RACK` row** is one of your saved racks: one `racks/<name>.json` per file, each the whole instrument — every machine with its
kind, name, colour, ports, channel, role, patterns, patch, kit, shuffle and learned controllers. A row loads that rack into the
session, replacing **every** machine in it; the song's name and the tempo stay where they are, because a rack file carries no
transport and no song. If what is on screen has unsaved changes, the first click only arms the row — it prints
`<name> - click again to replace the rack` — and the second loads it.

**A `DEMO` row** is one of the seven that ship inside the app: **Slow Acid**, **Hard Techno Acid**, **Blues**, **Rock**, **Funk**,
**Acid Jazz** (a `1/8T` triplet line) and **Fast Acid** (a `1/32` one). They are read-only — there is no way to overwrite one —
and each is a patch plus eight patterns in `A1` to `A8` ([`23-pattern-grids`](23-pattern-grids.md))
([`11-the-bank`](11-the-bank.md)). Clicking one loads it as a **new** song: named `New song`, not yet on disk — its name sits in
amber until you save it, which is how the face says "not yet written".

On a drum machine the same seven rows load that demo's **drum bank** instead: eight grooves into `A1` to `A8`, at the demo's
tempo and shuffle, with that machine's kit and the session's name left exactly as they were. The tempo is the whole rack's,
though — one cell, every machine on it — so loading a bank also retunes the 303s beside it, and that tempo, unlike a second
device's grooves, is what a save keeps. A drum bank is patterns, not a song — nothing is renamed and no sound is loaded with it.
A save keeps every machine in the rack ([`18-a-rack-of-devices`](18-a-rack-of-devices.md)), so a drum machine's grooves are kept
whichever cell it sits in. [`22-the-drum-machine`](22-the-drum-machine.md) is the drum machine's own page, its grid and its kits.

Every row of a kind that is a file — `SONG`, `BANK` and `RACK` — has a small `x` at its right end. One click turns it into `SURE`;
the second deletes that file. The demo rows have none: they are the app's, not yours.

## The two fields that save something other than the song

`SAVE BANK` keeps the selected device's bank under a name, and its field behaves as the song's does: the bank's own name rewrites
that file, a different name writes a new one — beside a taken one, `Easy (2)`, so a bank never destroys another by being given its
name — and a blank field saves the bank the device came from. There is no `IMPORT` or `EXPORT` for a bank: `banks/` is the folder,
and a file dropped into it loads.

`SAVE RACK` keeps the session's machines under a name, and its field behaves the same way: the rack's own name rewrites that file,
a different name writes a new one — beside a taken one, `Studio (2)` — and a blank field saves the rack the session came from.
That name is remembered for this session and not across a restart, so after relaunching, a name already taken is stepped around
rather than written over.

A rack load brings the machines back as the file wrote them, and what a restart forgets a recall forgets too: every machine comes
back unmuted, running forwards and in tune.

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
with a dot — they become file names in `songs/`, `banks/` or `racks/`.

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
