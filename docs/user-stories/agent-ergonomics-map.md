# Story Map: Agent Ergonomics

Story map for the gaps identified in [../agent-ergonomics.md](../agent-ergonomics.md)
(a temporary analysis doc — this map, plus the per-command stories it links
to, supersedes it as the source of truth once those stories exist). Unlike
[skill.md](skill.md), which packages *documentation* for driving the
existing CLI, everything on this map requires new `ishi` behavior: flags and
output formats an agent needs that don't exist yet.

### Segment

Ishi users who delegate PARA vault upkeep to an AI coding agent (e.g. Claude
Code) instead of, or in addition to, running `ishi` themselves at a
terminal.

### Persona

An LLM agent acting on the user's behalf inside their vault's working
directory. It has shell access to `ishi` and to the vault's files directly,
but no terminal to eyeball human-formatted tables and no stdin to answer
interactive prompts. It needs to plan its next action from *structured*
command output and needs every command it might run to be drivable with
flags alone.

### Narrative

Manage a user's PARA vault end-to-end — survey its state, act on specific
items, run the weekly review, and recover sensibly from mistakes — using
only `ishi`, with no human available to read a table or answer a prompt.

---

## Backbone (Activities → Steps → Tasks)

```
Orient in the vault      →  Resolve & open an item   →  Run review unattended     →  Recover from failure
─────────────────────        ─────────────────────       ─────────────────────        ─────────────────────
List a category as        Get an item's file path      Act on one item without      Tell failure kinds apart
structured data            from the same query           stdin                        from the exit code alone

Check vault counts &                                    (repeat per item until
per-item state as                                        the walk is done)
structured data

Inspect effective config
as structured data
```

### Activity 1: Orient in the vault

**Step 1.1 — List a category as structured data**
- Task: `ishi list <category> --json` emits an array of typed rows (name,
  title, updated_days_ago) instead of an aligned text table →
  [list.md](list.md) Story 006
- Task: archive rows in `--json` mode include the origin category as a
  structured field, not a `Origin/name` string an agent would have to
  split → [list.md](list.md) Story 006

**Step 1.2 — Check vault counts & per-item state as structured data**
- Task: `ishi status --json` emits per-category counts plus the
  project/area per-item breakdown (name, title, updated, reviewed) as
  typed data → [status.md](status.md) Story 005

**Step 1.3 — Inspect effective config as structured data**
- Task: `ishi config --json` emits the effective config plus, per key,
  which layer it came from (default/user/local) as a structured field
  instead of an inline TOML comment → [config.md](config.md) Story 007

### Activity 2: Resolve and open a specific item

**Step 2.1 — Get an item's file path from the same query used to find it**
- Task: `ishi list <category> --json` rows include a resolved `path` field
  (the `index.md` path for project/area, the file path for
  resource/inbox/archive) → [list.md](list.md) Story 006 (folded into the
  same story as Step 1.1 — one JSON shape, not a second flag)

### Activity 3: Run the weekly review unattended

**Step 3.1 — Act on one item without stdin**
- Task: `ishi review <item> --keep|--archive|--skip` performs the same
  effect as the interactive prompt's `[k]/[a]/[s]` choice for a single
  named item, reusing `move`/`last_reviewed`-stamping logic →
  [review.md](review.md) Story 004
- Task: the flag-driven form rejects a name that isn't a project or area
  (nothing to review) with a distinct, scriptable error rather than
  silently no-op'ing → [review.md](review.md) Story 004

*(An agent drives the whole review by calling this once per item — walk
order and "what's left" both come from `ishi status --json`, Activity 1, so
no separate "what's pending" step is needed here.)*

### Activity 4: Recover from failure

**Step 4.1 — Tell failure kinds apart from the exit code alone**
- Task: distinct, documented exit codes for at least "item not found",
  "invalid state for this operation" (e.g. already archived), and
  "invalid config" — replacing the current blanket exit 1 →
  [exit-codes.md](exit-codes.md) Story 001
- Task: every non-zero exit still pairs with a human-readable `Error: ...`
  on stderr, so the exit code adds a machine-checkable signal without
  removing the message a human collaborator would read →
  [exit-codes.md](exit-codes.md) Story 001

---

## Prioritization

- **Release 1 (highest leverage, per agent-ergonomics.md):** Activity 1 +
  Activity 2's task — `--json` on `list`/`status`/`config`, including the
  `path` field. This unblocks every other agent workflow, since "what
  exists and what state is it in" gates every subsequent decision.
- **Release 2:** Activity 3 — `ishi review --keep/--archive/--skip`. Depends
  on Release 1 only insofar as an agent needs `status --json` to know which
  items to walk.
- **Release 3:** Activity 4 — semantic exit codes. Independent of the other
  releases, but lowest priority since agents can currently work around it by
  string-matching stderr (worse ergonomics, not a hard blocker).

## Gaps / open questions surfaced while mapping

- No step here proposes a non-interactive `new`/`move`/`archive` gap —
  those already accept `--yes` (see "Already good" in
  agent-ergonomics.md), so they're intentionally absent from this map.
- The map treats "what's pending to review" as answered by `status --json`
  rather than adding review-specific listing. If that turns out to be
  insufficient (e.g. an agent needs review-only fields status doesn't
  expose), that's a new step under Activity 3, not a reason to expand
  Activity 1's scope.
