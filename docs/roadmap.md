# Roadmap

## Status snapshot (as of this doc)

| Command       | State                                                                                                                                                                                                                                                                                                                                                                                                                            |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `init`        | Done                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `new`         | Done — all of user-stories/new.md 001–013 completed, including per-category templates (`daily`/`project`/`area`/`resource`), `{{time}}`/`{{uuid}}` placeholders, and editor-capture with no filename into `--project`/`--area`/`--resource` (story 010)                                                                                                                                                                          |
| `daily`       | Done                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `move`        | Story 001 done (per [docs/lld/012-tk-move.md](lld/012-tk-move.md)); Story 002 (unwrap-rejection guard) remaining                                                                                                                                                                                                                                                                                                              |
| `archive`     | Not started                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `list`        | Stories 001, 002, 003, 005 done (base NAME/TITLE/UPDATED columns, archive qualified naming, substring filter, Title-falls-back-to-Name); 004 (empty-category message) remaining                                                                                                                                                                                                                                                                          |
| `status`      | Stories 001–004 done (per-category counts per [docs/lld/009-status-counts.md](lld/009-status-counts.md); per-item Project/Area rows with `updated:`/`reviewed:` per [docs/lld/010-status-per-item.md](lld/010-status-per-item.md); `last_reviewed` write side per [docs/lld/013-review-keep-archive-skip.md](lld/013-review-keep-archive-skip.md))                                                                                 |
| `review`      | Done — Stories 001–003 complete, per [docs/lld/011-review-walk.md](lld/011-review-walk.md) and [docs/lld/013-review-keep-archive-skip.md](lld/013-review-keep-archive-skip.md)                                                                                                                                                                                                                                                    |
| `config`      | Layering done, and `config init`/`config init -g`/`config edit`/`config edit -g`/`#:schema` file are implemented — bare `tk config` (provenance display) is still open                                                                                                                                                                                                                                                |
| `completions` | Done — `tk completions bash`/`zsh`/`fish`/`powershell`, per [docs/lld/008-completions.md](lld/008-completions.md)                                                                                                                                                                                                                                                                                                               |

## Dependency graph

Item-level view of the sequencing below — an edge means the upstream item is
a real implementation dependency of the downstream one (not just a shared
naming convention; see the footnote after the diagram for two cross-references
that are docs-only and deliberately _not_ drawn here).

```
                    init (done)
                              |
        +-----------------------+-----------------------+
        |                                                |
        v                                                v
 1.  new (done)                                   9. completions
        |                                         (no other deps)
     +--+------------+------------+
     |               |            |
     v               v            v
 4. list         6. move     2. config layering
     |               |       + templates (done)
     v               |         /            \
 5. status           |        v              v
     |               |    3. daily (done) 8. config CLI
     +-------+-------+
             |
             v
        7. review
```

- `list` → `review` (list.md 001 cites review.md 001's "raw days ago"
  phrasing as a shared convention) is a docs-only cross-reference, not a
  build blocker — `list` ships first per the sequencing below regardless.
- `completions` (item 9) lists every other command's Story 001 as a
  "depends on" in completions.md 003, but that's `clap_complete` describing
  what the generated script must cover, not implementation work blocked on
  those commands landing — completions can be built against however much of
  the CLI exists at the time.

## Done

### 1. `new`: wire `--project` / `--area` / `--resource`

`items::create` already took a `Category` and scaffolded directories vs. flat
files correctly (see `src/items.rs` tests) — the only gap was `main.rs`'s
`Commands::New` not exposing the flags. Done: a `clap` `ArgGroup`
(`NewCategory`, `multiple = false`) adds `--project`/`--area`/`--resource` as
mutually-exclusive flags, `cli::run_new` takes a `Category` parameter (used
for the non-interactive named-file path; the no-filename editor-capture path
still always creates in `Inbox` — capturing into a project/area/resource with
no filename is story 010, not yet wired).

- Covers user-stories/new.md 003–006. (Story 002, the named-Inbox-note case,
  is already Completed; init.md 001–004 are Done.)

### 1b. `new`: templates, placeholders, and no-filename category capture

Builds on item 1's flag-wiring above. `Config::templates` grew `daily`/
`project`/`area`/`resource` fields alongside `note` (all rendered through
the same pure `config::render`), plus `{{time}}` and `{{uuid}}`
placeholders next to `{{title}}`/`{{date}}`. The editor-capture path (`new`
with no filename) now pre-populates `$EDITOR` with the rendered template
for whichever `Kind` was requested — including `--project`/`--area`/
`--resource` with no filename (story 010), which scaffolds directly into
that category's directory-vs-flat-file shape instead of always landing in
`Inbox`. `run_new` takes a `Kind` (not just a `Category`) so it can look up
the right template and still derive the filing category via
`Kind::category`. `editor::suggest_filename`'s title-inference skips a
leading frontmatter block, then takes the first Markdown heading line
(any `#` level), falling back to the first non-blank post-frontmatter
line, then to the timestamp fallback.

- Covers user-stories/new.md 001–013, all completed.

### 3. `tk daily`

Implemented as sugar for `tk new --daily`, per
[docs/lld/004-tk-daily.md](lld/004-tk-daily.md). Along the way, `Category`
(where an item is filed) and a new `category::Kind` (what `new`/`daily`
create) were split into two types — a daily note has no folder of its own
(`Kind::Daily` maps to `Category::Inbox`) and `Category::Archive` has no
creation behavior at all, so one enum answering both questions no longer
fit; see design.md's "Filing vocabulary vs. creation vocabulary" for the
full rationale. `Templates::for_category` became the total `for_kind`, and
`items::item_path` was factored out of `create` so `cli::daily_note_exists`
can check for today's note without duplicating the directory-vs-flat-file
branch. `cli::run_daily` creates non-interactively on the first run of the
day and reopens the existing note untouched (via the new `Editor::open`)
on any later run.

- Covers user-stories/daily.md 001–003 and new.md story 013.

### 2. Config layering

Per [docs/lld/005-config-layering.md](lld/005-config-layering.md).
`Config::resolve(local_path, home_path)` replaces `Config::load`, merging
`built-in → ~/.tick.toml → ./.tick.toml` per key and returning a parallel
`ConfigOrigins` recording which layer each effective value came from
(`Source::{Default,User,Local,LocalOverridesUser}`). Along the way, the
TOML schema `Config` parses was fixed to match the nested `[folders]` /
`[defaults]` / `[templates]` tables `README.md` documents (the old flat
`default_extension` / `category_dirs.*` shape is no longer read).
`Workspace::discover` takes a new `home_config: Option<&Path>` parameter,
layering it in on both of its branches; `main.rs` computes the home config
path from `$HOME` once and passes it through. `ConfigOrigins` isn't
consumed anywhere yet — only the future `tk config` display path (item 8)
will read it.

- Covers the config-resolution scenarios in user-stories/config.md 001–002
  (not yet the `tk config` CLI surface — that's item 8, now unblocked).

### 9. `tk completions`

Pure `clap_complete` wiring, no dependencies on anything else in this list.
Implemented per [docs/lld/008-completions.md](lld/008-completions.md):
`tk completions bash`/`zsh`/`fish`/`powershell` prints that shell's
completion script for `tk` to stdout, generated by walking `Cli::command()`
via `clap_complete` rather than a hand-maintained command list.

- Covers user-stories/completions.md 001–003. Story 003 (completions must
  stay current with `tk`'s commands) lists every other command's Story 001
  as a "depends on," but that's describing what the generated script must
  cover, not build order — `clap_complete` derives the script from whatever
  subcommands exist in the CLI definition at the time.

## Next

### 4. `tk list`

Pure read over `items` — no mutation, no new filesystem primitives beyond
what `create` already exercises. Cheapest of the remaining commands and has
no dependents, so it can slot in anywhere in this group.

- **Design update (2026-07-02), not yet implemented:** `list` no longer
  just returns bare paths — `items::list` returns `Vec<ListedItem>`
  (Name/Title/Updated), sorted alphabetically by name, with `filter`
  matching a case-insensitive substring of Name or Title. Needs a new
  `items::infer_title` helper (frontmatter-skip + first-heading, mirroring
  `editor::suggest_filename`'s logic but implemented independently — see
  the updated `items` section of design.md) and an mtime-based
  `updated_days_ago`, the same source item 5 (`status`) needs for its
  per-item `updated: ...` facts. Covers user-stories/list.md.

### 5. `tk status`

Per-category counts, plus a per-item breakdown for Projects/Areas showing
`updated_days_ago` (reuses `list`'s mtime sourcing — sequenced right after
`list` for that reason) and `reviewed_days_ago` (a new `last_reviewed`
frontmatter field, `None` until an item has been kept in a review). No
staleness threshold or flagging — `status` reports the facts and leaves
judgment to the user. Needs `items::read_last_reviewed` /
`items::write_last_reviewed` (see design.md); `write_last_reviewed`'s only
caller is item 7 (`review`)'s `[k]eep` action, per
[docs/lld/013-review-keep-archive-skip.md](lld/013-review-keep-archive-skip.md).
Covers user-stories/status.md.

### 6. `tk move`

More involved: wrapping a flat file into a directory when moving into
`project`/`area`, and preserving origin category as a subfolder when moving
to `archive`. `mv` is an alias for `move`, same as `ls` is for `list`. No
dependents among items 3–5, but **item 7 (`review`) calls `items::mv`
directly** per `docs/design.md`, so it has to land before review.

Story 001 (relocating an item that already exists in a known category) is
done, per [docs/lld/012-tk-move.md](lld/012-tk-move.md). Story 002
(rejecting an unwrap of a `Project`/`Area` directory into `inbox`/
`resource`) remains — 012's `items::mv`, as built, does not special-case
that combination: it falls into the generic "relocate as-is" branch and
would silently move the whole directory under the flat category instead of
erroring. Story 002's LLD must add a guard before that branch runs, ahead
of `archive` (item 6b) and `review` (item 7) building on top of `mv`.

- Covers user-stories/move.md 001–002.

### 6b. `tk archive`

Sugar for `tk move <item> archive` (item 6), plus three self-healing
affordances that keep the archive out of the way of the editor and of any
agent working in the workspace: merging quick-open excludes into
`.vscode/settings.json`/`.zed/settings.json`, ensuring a `CLAUDE.md`
instruction telling agents to skip the archive unless asked, and stamping a
one-line `summary` frontmatter field onto the item being archived (reusing
the same title-inference `list` (item 4) needs). Module ownership for the
editor-exclude and `CLAUDE.md` writers isn't decided yet — not sketched in
`docs/design.md` — since none of this existed before this story was
written; needs an LLD pass before implementation. Depends on item 6 (the
move) and item 4 (title inference for the summary default).

- Covers user-stories/archive.md 001–004.

## Later

### 7. `tk review`

Composes `move` (item 6) with the `Ui` trait already defined in `src/cli.rs`
(`confirm`/`choose` are implemented and tested via `run_init`/`run_new`), plus
`items::write_last_reviewed` (item 5) on `[k]eep`.

Story 001 (the walk itself — order, per-item prompt, empty/end-of-walk
messaging) is done, per
[docs/lld/011-review-walk.md](lld/011-review-walk.md); along the way `Ui::choose`
was reshaped to a two-line header+options form and gained `Ui::info`, its
first real callers. Stories 002 (`[a]rchive` calling `items::mv`) and 003
(`[k]eep`/`[s]kip` writing `last_reviewed` via the new
`items::write_last_reviewed`) are done, per
[docs/lld/013-review-keep-archive-skip.md](lld/013-review-keep-archive-skip.md).

- Covers user-stories/review.md 001–003. Done.

### 8. `tk config` CLI surface

`config`, `config init`, `config init -g`, `config edit`, `config edit -g`,
plus the `#:schema` JSON Schema file. Builds directly on the layering +
provenance tracking from item 2 — deliberately kept separate from it because
nothing else in this roadmap depends on the CLI surface existing, only on the
resolution logic underneath it.

`config init`/`config init -g` are done, per
[docs/lld/006-config-init.md](lld/006-config-init.md). `config edit`/`config
edit -g` are done, per [docs/lld/007-config-edit.md](lld/007-config-edit.md).
The `#:schema` JSON Schema file is done, per
[docs/lld/007-config-schema.md](lld/007-config-schema.md). Still open: bare
`tk config` (provenance display).

- Covers user-stories/config.md 003–006.

## Explicitly out of scope for this pass

Anything not in the README's command table (e.g. sync, plugins, multi-user
config) — not hinted at anywhere in the spec, so not roadmapped until there's
a concrete story for it.
