# lean-ctx — Token Optimization for Pi (COMMENTED OUT — pi-lean-ctx extension uninstalled as an experiment)

This project uses the **pi-lean-ctx** extension. It exposes `ctx_*` tools backed by **lean-ctx**,
and runs an embedded MCP bridge (on by default) that holds a **persistent session cache**.

## What to do (as Pi agent)

Prefer the `ctx_*` tools over Pi's built-ins — only the `ctx_*` tools are compressed and cached;
the native `read`/`bash`/`grep`/`find`/`ls` are **not** routed through lean-ctx in additive mode.

| Prefer | Over (native) | Why |
|--------|---------------|-----|
| `ctx_read` | `read`, `cat`/`head`/`tail` | Cached + compressed; unchanged re-reads cost ~13 tokens |
| `ctx_shell` | `bash` | Shell output compressed via 95+ patterns |
| `ctx_search` | `grep` | Compact, ranked matches |
| `ctx_glob` | `find` | Compressed, .gitignore-aware file matching |
| `ctx_tree` | `ls` | Compact directory maps |

- Use `ctx_shell` for commands with side effects (build/test/git/etc.); set `raw=true` when exact
  output matters.
- Use `ctx_read` with `mode=anchored` for files you will edit, then patch by anchor via
  `ctx_patch` (reachable through `ctx_call` in the default `lean` profile) — no old-text echo.
  For line ranges pass `offset`/`limit` (aliases of `start_line`) or `mode=lines:N-M` — all cached
  through the bridge, so repeated reads stay cheap.

## Advanced lean-ctx commands

Prefer the `lean_ctx` tool (installed by the extension) to run `lean-ctx` directly:

- `lean-ctx overview`
- `lean-ctx session …`
- `lean-ctx knowledge …`
- `lean-ctx gain` / `lean-ctx stats`
- `lean-ctx index …`

## MCP bridge

The embedded bridge is on by default and shows up in `/lean-ctx` (it reports `connected` plus a
tool count). To force the one-shot CLI path (no cross-call cache), set `LEAN_CTX_PI_ENABLE_MCP=0`.

# DEVELOPER NOTES
When progressing, note that large tool call payloads cause the provider stream to stall, so keep each append small (1 class per call when writing files). When writing the tests, generate and write a single test at a time and save them to the test file.

Plans are to be written into a plans subfolder.

## Writing files
When using the file write tool, use small blocks instead of writing the entire file in a single tool execution.

### For code files

Provide only the specific functions or lines that need to be changed or added. If you are generating a new file, scaffold the file first with empty definitions, then fill them in one by one to keep the indiviual write size small. Write individual functions to a temporary file first, then append them to the destination file. 

### For text files / markdown

Write one paragraph at a time. Write individual paragraphs to a temporary file first, then append them to the destination file. 

## UI ICONS

Icon-style buttons (PLAY/STOP, ◀/▶, +/−, ×/✎, chain arrows, loop, …) must **not** use a font glyph as their face — the bundled pygame font is missing several of these (✎ × − ◀ ▶ ↑ ↓ render as boxes/blank on some systems).

To make an icon button:
1. Create the icon in `scripts/generate_images.py` — add a branch to `icon(shape, …)` (dark disc + `theme.CELL_ON` shape, `m = size // 3` margins), save it in `main()`, and keep the module docstring **Produces:** list and the final `print` loop in sync.
2. Re-run it headless from the project root — `.venv/bin/python scripts/generate_images.py` — so the PNG lands in `assets/images/`. Generated PNGs are committed there.
3. Load it with `takkadrums/ui/icons.py::load_icon(name)` and pass `Button(icon=...)`; keep a plain-ASCII text label as the fallback shown when the image file is missing.

Do **not** cross-import a private icon loader between views — everything goes through `takkadrums/ui/icons.py::load_icon`.

## Progressing the plan
Progress 1 step at a time and update progress and notes in the plan file as you go to facilitate easy resumption if there is a problem.
