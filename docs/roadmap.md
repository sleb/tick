# Roadmap

## Status snapshot (as of this doc)

| Command       | State                                                                                                                                                                                                                                                                                                                                                                                                                            |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `init`        | Done                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `new`         | Done — all of user-stories/new.md 001–013 completed, including per-category templates (`daily`/`project`/`area`/`resource`), `{{time}}`/`{{uuid}}` placeholders, and editor-capture with no filename into `--project`/`--area`/`--resource` (story 010)                                                                                                                                                                          |
| `daily`       | Done                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `mv`          | Not started                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `list`        | Not started                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `status`      | Not started                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `review`      | Not started (module stub only in `docs/design.md`)                                                                                                                                                                                                                                                                                                                                                                               |
| `config`      | Partial — `templates` table now covers all five categories, but `Config::load` still reads a single TOML file with no `built-in → ~/.tick.toml → ./.tick.toml` merge, and there's no `config` subcommand                                                                                                                                                                                                                        |
| `completions` | Not started                                                                                                                                                                                                                                                                                                                                                                                                                      |

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
 4. list         6. mv      2. config layering
     |               |          + templates
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

## Next

### 2. Config layering

`templates` now has all five fields (`note`/`daily`/`project`/`area`/
`resource`) rendered through `config::render`, so the template side of
this item is done — see item 1b above. What's left is the layering
mechanism itself: `Config::load` still reads a single path with no merge
step (`Workspace::discover` only knows about bare category dirs, not
config layering at all).

- Implement the `built-in → ~/.tick.toml → ./.tick.toml` merge.
- **Why here:** `config` (item 8) needs the same layering logic to report
  provenance.
- Covers the config-resolution scenarios in user-stories/config.md 001–002
  (not yet the `tk config` CLI surface — that's item 8).

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
`items::write_last_reviewed` (see design.md); `write_last_reviewed` has no
caller until item 7 (`review`) lands, but the read side is exercised by
`status` alone. Covers user-stories/status.md.

### 6. `tk mv`

More involved: wrapping a flat file into a directory when moving into
`project`/`area`, and preserving origin category as a subfolder when moving
to `archive`. No dependents among items 3–5, but **item 7 (`review`) calls
`items::mv` directly** per `docs/design.md`, so it has to land before review.

- Covers user-stories/mv.md 001.

## Later

### 7. `tk review`

Composes `mv` (item 6) with the `Ui` trait already defined in `src/cli.rs`
(`confirm`/`choose` are implemented and tested via `run_init`/`run_new`), plus
`items::write_last_reviewed` (item 5) on `[k]eep`. Blocked on items 5 and 6.

- Covers user-stories/review.md 001–003.

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

- Covers user-stories/completions.md 001–003. Story 003 (completions must
  stay current with `tk`'s commands) lists every other command's Story 001
  as a "depends on," but that's describing what the generated script must
  cover, not build order — `clap_complete` derives the script from whatever
  subcommands exist in the CLI definition at the time.

## Explicitly out of scope for this pass

Anything not in the README's command table (e.g. sync, plugins, multi-user
config) — not hinted at anywhere in the spec, so not roadmapped until there's
a concrete story for it.
