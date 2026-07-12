# Agent ergonomics review (temporary notes)

Analysis of `ishi` from the perspective of an LLM agent managing a PARA vault
on a user's behalf, not just a human at a terminal. Scoped against the
[better-cli](https://github.com/anthropics) checklist (P0: JSON output,
no required interactivity, semantic exit codes, actionable errors; P1/P2:
TTY detection, dry-run, idempotency).

## Already good

- `--yes` on `new`, `move`, and `archive` already bypasses their interactive
  prompts (`src/cli.rs:95,417`).
- Everything is plain files on disk, so an agent can always fall back to
  reading/writing directly if the CLI doesn't cover a case.
- `move`/`archive`/`unarchive` take a single item name + target — already
  flag/positional-minimal and composable.

## Gaps, in priority order

### 1. No `--json` on `list`, `status`, or `config`

`run_list`/`run_status` (`src/cli.rs:244,297`) and `run_config_init`/the
plain `config` printer return pre-formatted human strings — aligned
columns, `updated: 21 days ago`, `# default` provenance comments. An agent
has to regex a table with no stability guarantee instead of parsing a typed
structure. This is the highest-leverage fix: an agent's core loop is
"what items exist, what state are they in" before deciding what to do next.

Also fold in **item path resolution**: `list` shows `Name`/`Title`/`Updated`
but not whether an item is `index.md` inside a directory or a flat file, or
its extension. An agent that wants to `Read`/`Edit` a note directly has to
guess the path or reimplement ishi's resolution logic. `--json` output
should include the resolved `path` field.

### 2. `review` has no non-interactive form

`review` is a hardcoded keep/archive/skip loop reading from stdin
(`choose`, `src/cli.rs:53`) with no flag-driven equivalent. An agent asked
to run a weekly review on the user's behalf currently can't drive it at
all — it would have to bypass `review` entirely and reimplement its effect
(stamping `last_reviewed`) via raw frontmatter edits, defeating the point
of the command existing. Needs something like
`ishi review <item> --keep|--archive|--skip`, reusing existing move/stamp
logic.

### 3. Undifferentiated exit codes

`main` returns `anyhow::Result<()>` (`src/main.rs:292`), so "item not
found," "already archived," and "invalid config TOML" all exit 1 with an
`Error: ...` string. An agent can't branch on failure type (retry vs.
surface to user vs. treat as a no-op) without string-matching stderr.

### 4. `config` output isn't a data structure

`ishi config` prints TOML annotated with inline `# default` /
`# local, overrides user` comments — great for a human skimming, but an
agent that wants to know "is `archive` renamed locally?" has no
programmatic answer short of parsing comments. Covered by item 1's
`--json` work but called out separately since it's a distinct code path.

## Explicitly out of scope

Most P2 items in the better-cli checklist (progress bars/spinners,
`NO_COLOR`, shell completions) are either already handled (completions
exist) or low-value for a fast local single-user tool — not worth chasing
for their own sake.

## Suggested next step

Turn items 1–4 into user stories under `docs/user-stories/`, starting with
`--json` on `list`/`status` since it unblocks the most other agent
workflows, then an LLD per `docs/lld/TEMPLATE.md` before implementation.
