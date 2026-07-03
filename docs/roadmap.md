# Roadmap

Gap analysis between the target behavior in [README.md](../README.md) /
[user-stories](user-stories/) and what's implemented in `src/` today, sequenced
by dependency rather than by date. Each milestone unlocks the one after it —
see the _Why here_ line.

## Status snapshot (as of this doc)

| Command       | State                                                                                                                                                                       |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `init`        | Done                                                                                                                                                                        |
| `new`         | Partial — Inbox capture + editor-inferred filename work; `--project`/`--area`/`--resource` flags aren't wired to the CLI yet, and no template is applied to created content |
| `daily`       | Not started                                                                                                                                                                 |
| `mv`          | Not started                                                                                                                                                                 |
| `list`        | Not started                                                                                                                                                                 |
| `status`      | Not started                                                                                                                                                                 |
| `review`      | Not started (module stub only in `docs/design.md`)                                                                                                                          |
| `config`      | Not started — `Config::load` reads one TOML file with no layering; no `templates` table; no `config` subcommand                                                             |
| `completions` | Not started                                                                                                                                                                 |

## Now

### 1. Finish `new`: wire `--project` / `--area` / `--resource`

`items::create` already takes a `Category` and scaffolds directories vs. flat
files correctly (see `src/items.rs` tests) — the only gap is `main.rs`'s
`Commands::New` doesn't expose the flags yet. Smallest remaining piece of an
already-started command.

- Closes user-stories/new.md 003–006.

## Next

### 2. Config layering + templates

Today `Config::load` reads a single path with no merge step, and has no
`templates` field at all — `new`/`daily`/etc. write raw editor content or an
empty string instead of a rendered template. This is the one piece of
plumbing that several later commands need, so it goes first in this group:

- Add `templates: Templates` to `Config`, with `{{title}}`/`{{date}}`
  rendering.
- Implement the `built-in → ~/.tick.toml → ./.tick.toml` merge (currently
  `Workspace::discover` only knows about bare category dirs, not config
  layering at all).
- **Why here:** `daily` (item 3) needs a rendered `daily` template, and
  `config` (item 8) needs the same layering logic to report provenance.
  Doing it once now avoids reworking `new`'s content path twice.
- Covers the config-resolution scenarios in user-stories/config.md 001–002
  (not yet the `tk config` CLI surface — that's item 8).
- **Design update (2026-07-02), not yet implemented:** templates gained
  `{{time}}`, `{{cursor}}`, and `{{uuid}}` placeholders alongside
  `{{title}}`/`{{date}}`, and the editor-capture path (`new` with no
  filename, and `--project`/`--area`/`--resource` with no filename) now
  pre-populates `$EDITOR` with the rendered template instead of a blank
  scratch file — `{{title}}` renders empty and `{{cursor}}` marks where the
  editor's cursor should start (via a `+<line>` argument). This means
  `editor::suggest_filename`'s title-inference can no longer assume the
  title is the file's literal first line: it now skips a leading
  frontmatter block, then takes the first Markdown heading line (any `#`
  level), falling back to the first non-blank post-frontmatter line, then
  to the timestamp fallback. `src/editor.rs`'s current `suggest_filename`
  and the `Editor::capture` signature both need updating when this item is
  picked up — see user-stories/new.md 001/007/010 and the updated
  `editor` section of design.md. Worth checking `sleb/knap` before
  implementing — it may already have LSP support, or heading/frontmatter
  parsing logic, that could be reused as a shared library instead of
  reimplementing here.

### 3. `tk daily`

Straightforward once item 2 lands: resolve today's date, render the `daily`
template, create-or-open in the Inbox. Implemented as sugar for `tk new
--daily` — the `--daily` flag needs wiring alongside `--project`/`--area`/
`--resource` from item 1, plus the create-vs-open branch on whether today's
note already exists. Covers user-stories/daily.md and new.md story 013.

### 4. `tk list`

Pure read over `items` — no mutation, no new filesystem primitives beyond
what `create` already exercises. Cheapest of the remaining commands and has
no dependents, so it can slot in anywhere in this group.

### 5. `tk status`

Needs a `stale` check (index.md mtime) that `list`'s traversal makes easy to
build on top of. Sequenced right after `list` for that reason.

### 6. `tk mv`

More involved: wrapping a flat file into a directory when moving into
`project`/`area`, and preserving origin category as a subfolder when moving
to `archive`. No dependents among items 3–5, but **item 7 (`review`) calls
`items::mv` directly** per `docs/design.md`, so it has to land before review.

## Later

### 7. `tk review`

Composes `mv` (item 6) with the `Ui` trait already defined in `src/cli.rs`
(`confirm`/`choose` are implemented and tested via `run_init`/`run_new`).
Blocked only on item 6.

### 8. `tk config` CLI surface

`config`, `config init`, `config init -g`, `config edit`, `config edit -g`,
plus the `#:schema` JSON Schema file. Builds directly on the layering +
provenance tracking from item 2 — deliberately kept separate from it because
nothing else in this roadmap depends on the CLI surface existing, only on the
resolution logic underneath it.

- Covers user-stories/config.md 003–006.

### 9. `tk completions`

Pure `clap_complete` wiring, no dependencies on anything else in this list.
Lowest priority because it has no user-facing value until the rest of the
command set stabilizes — completion scripts for half-finished commands are
churn.

## Explicitly out of scope for this pass

Anything not in the README's command table (e.g. sync, plugins, multi-user
config) — not hinted at anywhere in the spec, so not roadmapped until there's
a concrete story for it.
