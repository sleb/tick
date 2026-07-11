# Tick

Tick is a command-line tool for managing a [PARA](https://fortelabs.com/para) system — a simple method for organizing your notes and files into four categories:

| Category      | Purpose                                              |
| ------------- | ---------------------------------------------------- |
| **P**rojects  | Short-term efforts with a specific goal and deadline |
| **A**reas     | Ongoing responsibilities with a standard to maintain |
| **R**esources | Topics or themes of ongoing interest                 |
| **A**rchive   | Inactive items from the other three categories       |

> PARA was created by Tiago Forte. See his [original post](https://fortelabs.com/para) for background on the method itself.

Tick manages the directory structure and file bookkeeping so you can focus on capturing and organizing your notes:

```
.
├── 0-Inbox
├── 1-Projects
│   └── website-redesign
│       └── index.md
├── 2-Areas
│   └── health
│       └── index.md
├── 3-Resources
└── 4-Archive
```

Projects and areas are directories, not single files — real projects accumulate drafts, attachments, and other supporting material, and `index.md` is the entry point Tick reads for a project or area's title and status. Resources and inbox captures are usually single notes, so they stay flat files.

## Installation

```
cargo install tick
```

Or build from source:

```
git clone https://github.com/sleb/tick.git
cd tick
cargo install --path .
```

This installs a `tk` binary — the crate is published as `tick`, but the command stays short.

## Quick start

```
$ tk init my-para
Created PARA system in ./my-para

$ cd my-para
$ tk new meeting-notes
Created inbox/meeting-notes.md

$ tk new --project website-redesign
Created projects/website-redesign/index.md

$ tk status
Inbox      1
Projects   1
Areas      0
Resources  0
Archive    0
```

## Commands

| Command                                                   | Description                                                                                                                                                                                                                                                                                                                                         |
| --------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `init [name]`                                             | Initialize a new PARA system                                                                                                                                                                                                                                                                                                                        |
| `new [filename] [--project\|--area\|--resource\|--daily]` | Capture a new note. Defaults to the Inbox; pass `--project` or `--area` to scaffold a directory with an `index.md`, or `--resource` for a flat file. Omit `filename` to capture in `$EDITOR`, which will suggest a name for you to confirm or override. `--daily` creates (or opens) today's note and takes no `filename`; see [`tk daily`](#daily) |
| `daily`                                                   | Create (or open) today's daily note in the Inbox                                                                                                                                                                                                                                                                                                    |
| `move <item> <category>` (alias `mv`)                     | Move a file or project/area directory to `inbox`, `project`, `area`, `resource`, or `archive`. Archiving preserves which category the item came from. Also accepts an already-archived item's `<OriginCategory>/<name>` — moving it to anything other than `archive` un-archives it                                                               |
| `archive <item>`                                          | Sugar for `move <item> archive`. Also stamps a summary into the item's frontmatter                                                                                                                                                                                                                                                                  |
| `unarchive <OriginCategory>/<name>`                       | Restore an archived item to the category it was archived from. Sugar for `move` with the origin implied by the qualified name, so you don't have to spell out the destination                                                                                                                                                                     |
| `list <category> [filter]`                                | List items in a category (`inbox`, `project`, `area`, `resource`, or `archive`) with their inferred title and last-modified time, optionally filtered by name or title                                                                                                                                                                              |
| `status`                                                  | Show item counts per category, plus last-updated/last-reviewed facts for projects and areas                                                                                                                                                                                                                                                                                         |
| `review`                                                  | Walk through projects and areas one by one for a weekly review                                                                                                                                                                                                                                                                                      |
| `config [init\|edit] [-g\|--global]`                      | View the effective config, or initialize/edit `.tick.toml`; `-g` targets `~/.tick.toml` instead of the local one                                                                                                                                                                                                                                    |
| `completions <shell>`                                     | Print a shell completion script's registration snippet to stdout. Once installed, item names (and, for `unarchive`, their `<OriginCategory>/<name>` qualified form) tab-complete straight from the current directory's PARA system                                                                                                                |

Files created without an extension default to `.md`.

### `init`

```
tk init [name]
```

Initializes a new PARA system in the current directory, or in `./<name>` if given.

```
$ tk init my-para
Created PARA system in ./my-para

$ ls my-para
0-Inbox  1-Projects  2-Areas  3-Resources  4-Archive
```

### `new`

```
tk new [filename] [--project | --area | --resource | --daily]
```

Creates a new note. With no arguments, opens `$EDITOR` pre-populated with the category's rendered template — frontmatter and all, cursor positioned where the title goes — instead of a blank scratch file. Once you save and exit, Tick suggests a filename from what you wrote (or a timestamp, if you left the template unchanged or emptied the file) and prompts you to confirm it or type a different one before creating the note in the Inbox. With a `filename`, creates it directly — in the Inbox by default, or under `--project`, `--area`, or `--resource` if given — rendering the template non-interactively instead.

For `--project` and `--area`, this scaffolds a directory named after `filename` containing an `index.md`, so the project can grow to hold other files. For `--resource` (and the Inbox), it's a single flat file.

`--daily` creates (or opens) today's note in the Inbox — see [`tk daily`](#daily), which is sugar for `tk new --daily`. It doesn't take a `filename` (the name is always today's date) and can't be combined with `--project`/`--area`/`--resource`.

```
$ tk new
Opening $EDITOR...
Create "website-improvement-ideas.md"?
Created inbox/website-improvement-ideas.md

$ tk new my-file
Created inbox/my-file.md

$ tk new --project my-project
Created projects/my-project/index.md
```

### `daily`

```
tk daily
```

Creates (or opens) today's daily note in the Inbox, named for the current date. Sugar for `tk new --daily`. The first run of the day creates the note non-interactively from the `daily` template and prints its path; running it again the same day opens the existing note in `$EDITOR` instead of recreating it.

```
$ tk daily
Created inbox/2026-06-30.md

$ tk daily
Opening $EDITOR...
```

### `move`

```
tk move <item> <inbox|project|area|resource|archive>
```

`mv` is an alias for `move`.

Moves an existing file or project/area directory to the given category. Moving a flat file into `project` or `area` wraps it into a new directory with an `index.md`; moving to `archive` preserves which category the item came from, filing it under a matching subfolder.

`<item>` also accepts an already-archived item's qualified `<OriginCategory>/<name>` form (as shown by `tk list archive`); moving it to anything other than `archive` un-archives it — sugar for this is [`tk unarchive`](#unarchive).

```
$ tk move my-file.md project
Moved inbox/my-file.md to projects/my-file/index.md

$ tk mv my-project archive
Moved projects/my-project to archive/projects/my-project

$ tk mv Projects/my-project project
Moved archive/projects/my-project to projects/my-project
```

Moving a `project`/`area` directory to `inbox` or `resource` (unwrapping a directory back into a flat file) is not yet supported — `tk move` rejects it with an error rather than guessing which file to keep.

### `archive`

```
tk archive <item>
```

Sugar for `tk move <item> archive` — files an item away without having to name the destination category. It takes no category argument, since the destination is always `archive`.

It prompts for a one-line summary of the item (defaulting to its inferred title, or its existing `summary` frontmatter field if it has one), and stamps it into the item's frontmatter before moving it — so a listing or a quick look at the frontmatter is enough to know what an archived item was, without reading the whole thing.

```
$ tk archive my-project
Summary for my-project? [My Project]
Moved projects/my-project to archive/projects/my-project
```

`tk init` keeps `.vscode/settings.json` (`files.exclude`/`search.exclude`) and `.zed/settings.json` (`file_scan_exclude`) up to date with the configured archive folder name, and keeps a `CLAUDE.md` instruction not to read the archive folder unless explicitly asked or there's a strong reason to — both set up once, at init time, rather than on every archiving move.

### `unarchive`

```
tk unarchive <OriginCategory>/<name>
```

Restores an archived item to the category it was archived from — sugar for `tk move <OriginCategory>/<name> <origin-as-target>`, so un-archiving never requires spelling out a destination the qualified name already encodes. `<OriginCategory>/<name>` is the qualified name `tk list archive` shows for the item.

```
$ tk list archive
NAME                         TITLE            UPDATED
Projects/my-project          My Project       21 days ago

$ tk unarchive Projects/my-project
Moved archive/projects/my-project to projects/my-project
```

### `list`

```
tk list <inbox|project|area|resource|archive> [filter]
```

Lists items in a category as a table of **Name**, **Title**, and **Updated**, sorted alphabetically by name. For `project` and `area`, Name/Title/Updated come from the item directory and its `index.md` (not the `index.md` path itself); for `resource`, `inbox`, and `archive`, they come from the flat file. `archive` prefixes Name with the item's origin category (`Projects/...`, `Resources/...`, etc.), since archived items from different origins can share a bare name.

- **Title** is inferred the same way `tk new`'s editor capture infers a filename: skip a leading frontmatter block, then take the first Markdown heading's text. If no heading is found, Title falls back to repeating Name.
- **Updated** is how long ago the item was last modified, in days (`today`, `1 day ago`, `12 days ago`, ...) — the same convention `tk review`'s transcript uses.

`filter`, if given, matches a case-insensitive substring of either Name or Title.

```
$ tk list project
NAME               TITLE              UPDATED
my-project         My Project         21 days ago
website-redesign   Website Redesign   2 days ago

$ tk list project web
NAME               TITLE              UPDATED
website-redesign   Website Redesign   2 days ago

$ tk list resource
No items in Resources.
```

### `status`

```
tk status
```

Shows an at-a-glance summary of the PARA system: item counts per category,
plus a per-item breakdown for Projects and Areas — the two categories a
weekly review acts on — showing how long ago each was last updated and last
reviewed. Inbox, Resources, and Archive stay counts-only, since they can grow
large and aren't part of the review loop. "Updated" is the same
modification-time signal `list` uses; "Reviewed" reflects `tk review`, which
stamps an item's `index.md` with a `last_reviewed` date whenever you `[k]eep`
it — an item you've never kept in a review reports `reviewed: never`.

```
$ tk status
Inbox       2
Projects    3
`- my-project         My Project         updated: 21 days ago   reviewed: never
`- q3-initiative      Q3 Initiative      updated: 5 days ago    reviewed: 10 days ago
`- website-redesign   Website Redesign   updated: 2 days ago    reviewed: 3 days ago
Areas       2
`- finances   Finances   updated: 4 days ago   reviewed: 4 days ago
`- health     Health     updated: today        reviewed: never
Resources   5
Archive     12
```

### `review`

```
tk review
```

Walks through each project and area one at a time (by its `index.md`), prompting you to keep, update, or archive it — a guided version of PARA's weekly review ritual. Choosing `[k]eep` stamps the item's `index.md` frontmatter with `last_reviewed: {{date}}` (adding the field if absent, overwriting it if present), which is what [`tk status`](#status) reads to show `reviewed: ...`. `[a]rchive` and `[s]kip` leave `last_reviewed` untouched — `[a]rchive` moves the item out of the active review set entirely, and `[s]kip` explicitly defers judgment rather than confirming the item's current state.

```
$ tk review
Project: website-redesign (last updated 12 days ago)
  [k]eep  [a]rchive  [s]kip?
```

### `config`

```
tk config [init | edit] [-g | --global]
```

With no arguments, prints the effective config — [defaults](#configuration) layered with any `~/.tick.toml` and `./.tick.toml` overrides — annotating each setting with where it came from. `tk config init` writes a `.tick.toml` populated with the defaults, ready to customize; `tk config edit` opens it in `$EDITOR`. Both target the local `./.tick.toml` by default; pass `-g`/`--global` to target `~/.tick.toml` instead.

```
$ tk config init
Created ./.tick.toml

$ tk config
[folders]
inbox = "0-Inbox" # default
projects = "1-Projects" # default
areas = "2-Areas" # default
resources = "3-Resources" # default
archive = "4-Archive" # local, overrides user

[defaults]
extension = "md" # default

[templates]
# default
note = """
..."""

# user
daily = """
..."""

# default
project = """
..."""

# default
area = """
..."""

# default
resource = """
..."""
```

### `completions`

```
tk completions <bash|zsh|fish|powershell>
```

Prints a small registration snippet to stdout — shell glue that calls back into `tk` (via `$PATH`) whenever you press tab, rather than a fixed script baked with today's commands. That means completions always match whichever `tk` is currently installed, with nothing to regenerate after an upgrade — and it's how item names get completed: `tk move`, `tk archive`, and `tk unarchive`'s `<item>`/`<name>` argument tab-completes from the current directory's actual PARA system (bare names for `move`/`archive`, the `<OriginCategory>/<name>` qualified form for `unarchive`), read fresh on every request. Outside a PARA system, item-name completion just offers nothing rather than erroring.

```
$ tk completions zsh > ~/.zsh/completions/_tk
```

## Configuration

Tick reads an optional `.tick.toml` from the root of your PARA system, and another from `~/.tick.toml` for personal defaults that apply across every system. It lets you rename the numbered folders, change the default file extension, and customize the templates used for new notes instead of relying on the built-in defaults.

Configuration is layered, with each level overriding only the keys it sets:

1. built-in defaults
2. `~/.tick.toml` (user-level)
3. `./.tick.toml` (local to the current PARA system)

So a template you like everywhere can live in `~/.tick.toml`, while a project-specific folder rename goes in `./.tick.toml` without needing to repeat the rest of your personal config. `tk config init`/`tk config edit` operate on `./.tick.toml` by default and `~/.tick.toml` with `-g`; see [`config`](#config) above.

```toml
[folders]
inbox = "0-Inbox"
projects = "1-Projects"
areas = "2-Areas"
resources = "3-Resources"
archive = "4-Archive"

[defaults]
extension = "md"

[templates]
note = """
---
last_updated: {{date}}
---
# {{cursor}}{{title}}
"""

daily = """
---
date: {{date}}
last_updated: {{date}}
---
# {{date}}

## Tasks

[ ] -

## Notes

{{cursor}}
"""

project = """
---
last_updated: {{date}}
---

# {{cursor}}{{title}}

Status: active
"""

area = """
---
last_updated: {{date}}
---

# {{cursor}}{{title}}

Standard:
"""

resource = """
---
last_updated: {{date}}
---

# {{cursor}}{{title}}
"""
```

Every category has a template: `note` is used for Inbox captures and `--resource` notes, `daily` for `tk daily`, and `project`/`area` for the `index.md` scaffolded by `tk new --project`/`--area`. Templates are plain text, filled in when the note is created, with these placeholders:

| Placeholder  | Renders to                                                                                                                                                                                                                                                                                                                                                                            |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `{{date}}`   | Today's date (`2026-07-02`)                                                                                                                                                                                                                                                                                                                                                           |
| `{{time}}`   | The current time (`14:32`)                                                                                                                                                                                                                                                                                                                                                            |
| `{{title}}`  | The note's title. Filled in from the given `filename` when creating a note non-interactively. Left empty when the template is used to pre-populate `$EDITOR` for an editor capture (`tk new`, or `--project`/`--area`/`--resource` with no `filename`) — the title isn't known yet, since that's what the editor capture infers                                                       |
| `{{cursor}}` | Not rendered as text — marks the line where `$EDITOR`'s cursor should start, for the editor-capture paths above. Tick opens `$EDITOR` with a `+<line>` argument pointing at that line, the convention understood by vi/vim/neovim, nano, and `emacs -nw`; editors that don't support it just open at the top of the file. Renders as an empty string outside the editor-capture paths |
| `{{uuid}}`   | A randomly generated unique id (e.g. `f47ac10b-58cc-4372-a567-0e02b2c3d479`), for Zettelkasten-style permanent note IDs. Not used in the built-in defaults above, but available in custom templates                                                                                                                                                                                   |

`tk config init` writes the defaults above as a starting point.

When an editor capture is saved, Tick infers the title (and from it, the suggested filename) by skipping a leading frontmatter block, if present, then looking for the first Markdown heading line (any `#` level) in what follows; if no heading is found, it falls back to the first non-blank line after the frontmatter; if that's also missing, it falls back to a timestamp-based name.

### Schema and autocomplete

`tk config init` (and `tk config edit`, if no config exists yet) writes a [`#:schema`](https://taplo.tamasfe.dev/configuration/directives.html) directive as the first line of `.tick.toml`, pointing at a JSON Schema that describes the `folders`, `defaults`, and `templates` keys:

```toml
#:schema ./.tick.schema.json

[folders]
inbox = "0-Inbox"
...
```

Editors with Taplo-based TOML support — notably VS Code's [Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml) extension — read that directive to offer autocomplete and inline validation for `.tick.toml`, with no extra setup. Tick writes the referenced schema file alongside `.tick.toml` so the reference resolves locally, without a network fetch. This is a Taplo-specific convention, not a universal TOML standard, so editors without Taplo support won't do anything with the directive beyond treating it as a comment.
