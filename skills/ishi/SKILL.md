---
name: ishi
description: Drive the ishi CLI to manage a PARA-method note vault (Projects/Areas/Resources/Archive) non-interactively — orienting in an existing vault, creating notes/projects/areas/resources, and filing or archiving items — without ever triggering an interactive $EDITOR or confirmation prompt that would hang waiting for a human. Use whenever a user asks you to create, file, move, archive, or organize notes in an ishi vault, or asks you to check on one.
---

# ishi

`ishi` is a command-line tool for managing a [PARA](https://fortelabs.com/para)
note vault — Projects, Areas, Resources, and Archive. This skill exists
because most of `ishi`'s commands are interactive by default (they open
`$EDITOR` or wait for a confirmation keypress), which will hang an agent
with no human at the keyboard. Everything below is about using the
non-interactive form of each command instead.

For the full command reference, run `ishi --help` or see this repo's
`README.md` — this skill only covers the parts relevant to unattended use.

## Orient before acting

Never assume a vault exists or guess at its layout. Before creating, moving,
or archiving anything in a directory you haven't inspected yet:

1. Run `ishi status` to see whether a PARA system exists and get a summary —
   counts per category, plus per-project/area facts like how long since each
   was last touched or reviewed. Add `--json` if you want structured output
   to parse instead of the human-formatted summary.
2. Run `ishi list <category>` (`project`, `area`, `resource`, `archive`,
   `inbox`) if you need the actual filenames in a category `status` only
   summarizes.
3. Only run `ishi init` if `status` reports no PARA system found. Don't run
   it speculatively — an existing vault should never be re-initialized.

## Creating items

`ishi new` with no filename opens `$EDITOR` and blocks — do not run any
`ishi new` variant without a filename. Always supply one:

```
ishi new my-file                    # plain note, filed in Inbox
ishi new --project website-redesign # new project
ishi new --area health              # new area
ishi new --resource recipe-ideas    # new resource
ishi new --daily                    # today's daily note (no filename accepted)
```

Each prints the path it created (or, for `--daily`, the existing note's
path). `ishi new` only scaffolds the file from its template — there's no
flag to pass body content through. To populate the note beyond the
template, write to the printed path yourself with your own file-editing
tools after `ishi new` returns.

## Filing and archiving

`ishi move`, `ishi archive`, and `ishi unarchive` show a confirmation
prompt by default. Pass `--yes` to accept the suggested summary and skip
it:

```
ishi move my-file project --yes     # file an Inbox item into Projects
ishi archive website-redesign --yes # shorthand for `ishi move website-redesign archive`
ishi unarchive my-file --yes
```

`ishi mv` is an alias for `ishi move`. Without `--yes`, these commands wait
on a prompt a script or agent can't answer — always include it.

## Commands that require a human

Never run these yourself; if the task calls for one, tell the user what to
run instead of attempting it:

- **`ishi review`** — walks every project/area one at a time, prompting
  `[k]eep`/`[a]rchive`/`[s]kip` interactively. There's a non-interactive
  single-item form for scripted use — `ishi review <item> --keep`,
  `--archive`, or `--skip` — but the bare interactive walk itself is off
  limits for unattended use.
- **`ishi new`** (any variant) invoked without a filename — opens
  `$EDITOR` and blocks.
- **`ishi config edit`** invoked with no existing config to edit — opens
  `$EDITOR` and blocks. (`ishi config init` is fine — it just writes
  defaults to a file.)
