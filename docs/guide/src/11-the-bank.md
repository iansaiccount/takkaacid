# The bank

There are **sixty-four patterns** in the machine: **eight banks of eight**. The readout at the right of the top strip
names the one you are on, from `A1` through `H8` — bank letter first, then the pattern's number inside the bank.

A **song** is all sixty-four plus the patch, the tempo, the swing and the switches in settings. So switching patterns and
switching songs are different moves: the first is how you play a set from one patch, the second is how you change
everything at once.

Each demo song arrives with **`A1` to `A8` filled**: the main line, the same line an octave down and an octave up, a
sparser reading of it to build a break on, and the same four for the fill. `B1` onward are empty and waiting.

## Moving between patterns

- **`shift+left`** and **`shift+right`**: previous and next pattern. It happens at once, and the selection goes back to
  step 1, because a new pattern starts at its first step.
- **A learned controller** (CC 21 unless you have taught it somewhere else — `14-midi`). This one is made for playing:
  see below.

The **`BANK`**, **`PATTERN`** and **`MODE`** keys in the switch row are engraved but silent — there is no bank browser
behind them yet. The keys and the controller are how the bank moves today.

## The swap that waits for the bar

Patterns are a bar long, so a controller that moves the selection as it is turned would restart the bar on every message
and chop the groove to pieces. It does not.

Instead the panel **shows the pattern you are pointing at** while the old one keeps playing, and the swap lands at the end
of the bar, just before the next bar's first note. Turning the knob through four patterns mid-bar does not play four
sixteenths of each; it plays out the bar and starts the one you stopped on.

Stopped, there is no bar to wait for, so the swap happens immediately.

If your controller's knob is not where the pattern is, nothing happens until the knob reaches the pattern you are on — the
pickup rule in `14-midi`, there to stop a message from the controller jumping the bank somewhere you did not ask for.

Moving with the keys is immediate: the keyboard is a decision, and the readout follows it at once.

`12-songs-and-demos` is what happens when you want to keep all this.
