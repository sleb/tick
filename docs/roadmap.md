# Roadmap

> `docs/lld/NNN-*.md` filenames referenced below are historical: each LLD is
> deleted once its story ships (see `docs/lld/TEMPLATE.md`), so these are
> provenance notes, not live links.

## Remaining work

Two items left. Neither blocks the other — they can land in either order or
in parallel.

### 1. `tk init` archive self-healing affordances

**Not started.** Covers user-stories/init.md 005–006.

Two affordances that should trigger as part of `tk init` (current directory
or a named subdirectory), keeping the archive folder out of the way of the
editor and of any agent working in the workspace from the moment the PARA
system is set up:

- Create `.vscode/settings.json`/`.zed/settings.json` with quick-open
  excludes for the configured archive folder, if neither already exists.
- Create a `CLAUDE.md` with an instruction telling agents to skip the
  archive unless asked, if one doesn't already exist.

In both cases, `init` only ever _creates_ these files — if a
`.zed/settings.json`, `.vscode/settings.json`, or `CLAUDE.md` already
exists, Tick leaves it untouched and prints instructions for the user to
update it manually, rather than parsing and merging into unknown-shape
content. `docs/design.md` notes these trigger from `run_init` (not
`run_move`), but module ownership for the writers isn't decided beyond
that.

**Why not done:** needs an LLD pass — neither affordance's design nor
module ownership is settled yet beyond "triggers from `run_init`".
Unblocked today; `init` itself is done.

### 2. Un-archiving (moving an item back out of `Archive`)

**Not started.** No user-stories/move.md story exists yet for this.

`docs/design.md`'s `items::locate` and `items::mv` sections describe target
behavior for moving an item _out of_ `Archive` (keyed off the target
category, not the origin subfolder recorded under `Archive`), and cite it
as "move.md Story 005" — but that story doesn't exist in
`docs/user-stories/move.md` yet, and `items::locate` still only searches
`Category::archivable()`, never `Archive` itself.

**Why not done:** blocked on writing the user story itself. Draft Story
005 into move.md first, then an LLD, then implement (extending
`items::locate`/`items::mv` to handle `Archive` as a source). Depends on
`move`, which is done.

## Everything else is done

| Command       | Notes                                                                     |
| ------------- | -------------------------------------------------------------------------- |
| `init`        | Stories 001–004 done. (Stories 005–006 are the remaining work above.)     |
| `new`         | Done — includes `--project`/`--area`/`--resource`, templates, placeholders |
| `daily`       | Done                                                                        |
| `move`        | Stories 001, 002, 004 done. (Story 005, un-archiving, is remaining work above.) |
| `archive`     | Done — sugar alias for `tk move <item> archive`                            |
| `list`        | Done                                                                        |
| `status`      | Done                                                                        |
| `review`      | Done                                                                        |
| `config`      | Done — layering, `config init`/`edit` (`-g`), provenance display, JSON Schema |
| `completions` | Done                                                                        |

Implementation notes for finished work (design rationale, which LLD each
story shipped under) live in git history and in `docs/design.md`, not
here.

## Explicitly out of scope for this pass

Anything not in the README's command table (e.g. sync, plugins, multi-user
config) — not hinted at anywhere in the spec, so not roadmapped until
there's a concrete story for it.
