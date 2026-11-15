# Phase 3 — the tape transport, and the row it frees

The tear-up of the panel's transport: the machine stops carrying the transport, the top band grows a conventional tape deck,
and the row the old transport leaves behind becomes device controls instead of a second set of transport keys.

Rounds live here as `NN-topic.md`, one per round of work, each opening with what is wrong today, its design, the tests that make
it safe, a numbered task list and non-goals. The gate is the phase 2 one, unchanged: the three workspace commands,
`docs/guide/check.sh`, and a review by a dispatched subagent before each commit (`AGENTS.md`).

## The round

- [`01-the-tape-transport`](01-the-tape-transport.md) — the tape in the top band, pause as a real state, the bottom row turned
  over to `REVERSE`, `TBC`, `TRANSPOSE −`, `TRANSPOSE +`, the not-yet-wired keys drawn and named, and `SONGS` renamed to
  `BANKS`.

## What comes after, and is not this phase

The wishlist orders 4 to 7 — drum machine, banks/patterns/songs to file, 1/32 and 1/8T grids, the sampler. None of them is
opened by this phase; a phase gets a directory when it earns one.

## Rulings this phase rests on, and what they cost if wrong

| Ruling | Cost if wrong |
|---|---|
| The tape lives in the top band, not the bottom row | The transport sits further from the hand that presses it, and the top band loses its tempo readout to the `−`/`+` pair. The alternative keeps the keys where they are but frees no room, which is the whole point of the round. |
| Pause is a third state on the rack, not a flag on the clock | Every reader of "is the clock running" has to decide what pause means; `pattern_swap` and the strip's caret are the two that matter. A flag would leave both reading "playing" while the sound stopped. |
| Not-yet-wired keys are drawn, clickable, and answer with a notice | Four keys on the face promise features that do not exist. The notice is the promise being honest; the alternative is a key that does nothing and reads as broken. |
| Reverse is a device flag in memory, read mirrored | No `SAVE_VERSION` bump, and a restart forgets it. Stored in the pattern would cost a bump and change what the bank means. |
| Transpose is a performance offset, not the TUNING knob | TUNING is a saved knob; riding it would write every rehearsal into the song. The offset is applied on top and is deliberately not persisted. |
| `SONGS` becomes `BANKS`, and the saved file keeps its name | The panel's word changes while `persist::save_song` and `SavedState` do not: what is saved today is one device's bank, and a song is the document that does not exist yet. Renaming the file's words too would break the save contract for a word. |
