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
Created ./0-Inbox/meeting-notes.md

$ tk new --project website-redesign
Created ./1-Projects/website-redesign/index.md

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
| `mv <item> <category>`                                    | Move a file or project/area directory to `inbox`, `project`, `area`, `resource`, or `archive`. Archiving preserves which category the item came from                                                                                                                                                                                                |
| `list <category> [filter]`                                | List items in a category (`inbox`, `project`, `area`, `resource`, or `archive`), optionally filtered by name                                                                                                                                                                                                                                        |
| `status`                                                  | Show item counts per category and flag stale projects/areas                                                                                                                                                                                                                                                                                         |
| `review`                                                  | Walk through projects and areas one by one for a weekly review                                                                                                                                                                                                                                                                                      |
| `config [init\|edit] [-g\|--global]`                      | View the effective config, or initialize/edit `.tick.toml`; `-g` targets `~/.tick.toml` instead of the local one                                                                                                                                                                                                                                    |
| `completions <shell>`                                     | Generate a shell completion script                                                                                                                                                                                                                                                                                                                  |

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
Created ./0-Inbox/website-improvement-ideas.md

$ tk new my-file
Created ./0-Inbox/my-file.md

$ tk new --project my-project
Created ./1-Projects/my-project/index.md
```

### `daily`

```
tk daily
```

Creates (or opens) today's daily note in the Inbox, named for the current date. Sugar for `tk new --daily`. The first run of the day creates the note non-interactively from the `daily` template and prints its path; running it again the same day opens the existing note in `$EDITOR` instead of recreating it.

```
$ tk daily
Created ./0-Inbox/2026-06-30.md

$ tk daily
Opening $EDITOR...
```

### `mv`

```
tk mv <item> <inbox|project|area|resource|archive>
```

Moves an existing file or project/area directory to the given category. Moving a flat file into `project` or `area` wraps it into a new directory with an `index.md`; moving to `archive` preserves which category the item came from, filing it under a matching subfolder.

```
$ tk mv my-file.md project
Moved ./0-Inbox/my-file.md to ./1-Projects/my-file/index.md

$ tk mv my-project archive
Moved ./1-Projects/my-project to ./4-Archive/Projects/my-project
```

Moving a `project`/`area` directory to `inbox` or `resource` (unwrapping a directory back into a flat file) is not yet supported — `tk mv` rejects it with an error rather than guessing which file to keep.

### `list`

```
tk list <inbox|project|area|resource|archive> [filter]
```

Lists items in a category, optionally filtered to names containing `filter`. For `project` and `area`, this lists the item directories (not the `index.md` files inside them); for `resource`, `inbox`, and `archive`, it lists flat files.

```
$ tk list project
./1-Projects/my-project
./1-Projects/website-redesign

$ tk list project website
./1-Projects/website-redesign
```

### `status`

```
tk status
```

Shows how many items are in each category, and flags projects or areas whose `index.md` hasn't been touched in a while.

```
$ tk status
Inbox      2
Projects   3 (1 stale)
Areas      2
Resources  5
Archive    12
```

### `review`

```
tk review
```

Walks through each project and area one at a time (by its `index.md`), prompting you to keep, update, or archive it — a guided version of PARA's weekly review ritual.

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
inbox = "0-Inbox"      # default
projects = "1-Projects" # default
areas = "2-Areas"       # default
resources = "3-Resources" # default
archive = "4-Archive"   # local, overrides user

[defaults]
extension = "md" # default

[templates]
note = "..."    # default
daily = "..."   # user
project = "..." # default
area = "..."    # default
resource = "..." # default
```

### `completions`

```
tk completions <bash|zsh|fish|powershell>
```

Prints a shell completion script to stdout.

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
