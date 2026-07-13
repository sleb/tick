# User Stories: `ishi` agent skill

> Unlike the other files in this directory, this one doesn't document a CLI
> subcommand — it documents a bundled, installable Claude Code Skill (a
> `SKILL.md` and supporting files) that ships from this repo and teaches an
> agent how to drive the existing `ishi` CLI non-interactively. No new `ishi`
> subcommand or flag is introduced by these stories; the CLI is already
> scriptable enough (`ishi new <filename>` skips `$EDITOR` entirely, `--yes`
> skips confirmation prompts) for an agent to use today. What's missing is
> packaging and instructions.

## User Story 001 ✅

- **Summary:** Install the `ishi` skill so an agent already knows how to manage a PARA vault
- **Depends on:** None

### Use Case

- **As a** Ishi user who collaborates with an AI coding agent
- **I want to** install the `ishi` skill (e.g. `bunx skills install sleb/ishi`)
- **so that** my agent already knows how to drive `ishi` to manage my PARA vault, instead of me re-explaining the CLI's commands and conventions every session

### Acceptance Criteria

- **Scenario:** Installing the skill
- **Given:** I have an agent runtime that supports installable skills (e.g. Claude Code)
- **When:** I run the skill installer's install command for this repo (e.g. `bunx skills install sleb/ishi`)
- **Then:** a `SKILL.md` for `ishi` is placed somewhere my agent discovers skills from, with no manual copying of files

- **Scenario:** Skill content stays in this repo, not hand-duplicated elsewhere
- **Given:** the skill is installed
- **When:** I inspect the installed `SKILL.md`
- **Then:** its content originates from a single source of truth versioned in this repo (e.g. `skills/ishi/SKILL.md`), so a new `ishi` release can update the skill the same way it updates the binary

---

## User Story 002 ✅

- **Summary:** The skill tells an agent how to orient itself in an existing vault before acting
- **Depends on:** Story 001 (the installed skill), [status.md](status.md) Story 001, [list.md](list.md) Story 001

### Use Case

- **As an** agent that has just been asked to do something in a user's PARA vault
- **I want to** the skill to document how to confirm a vault exists and see what's in it
- **so that** I check my surroundings with `ishi status` / `ishi list` before creating or moving anything, instead of guessing at the vault's current state

### Acceptance Criteria

- **Scenario:** Skill documents discovering and summarizing the vault
- **Given:** the `ishi` skill is installed and loaded
- **When:** an agent is asked to work in a directory it hasn't inspected yet
- **Then:** the skill's instructions direct it to run `ishi status` (and `ishi list <category>` as needed) first, and to run `ishi init` only if no PARA system is found — never to assume one exists

---

## User Story 003 ✅

- **Summary:** The skill tells an agent how to create notes without a human at the keyboard
- **Depends on:** Story 001 (the installed skill), [new.md](new.md) Story 002, Story 003, Story 004, Story 005 (the non-interactive named-creation forms this documents)

### Use Case

- **As an** agent creating a note, project, area, or resource on a user's behalf
- **I want to** the skill to document `ishi new <filename>` (and its `--project`/`--area`/`--resource` variants) as the way to create items
- **so that** I never trigger the `$EDITOR`-based interactive flow, which would hang waiting for a human

### Acceptance Criteria

- **Scenario:** Skill documents the non-interactive creation form
- **Given:** the `ishi` skill is installed and loaded
- **When:** an agent needs to create a new project, area, resource, or Inbox note
- **Then:** the skill's instructions direct it to always supply a filename argument (e.g. `ishi new --project apollo`) and never to invoke `ishi new` (or any variant) without one

- **Scenario:** Skill documents writing content after scaffolding, not through `ishi`
- **Given:** the `ishi` skill is installed and loaded
- **When:** an agent needs to populate a newly created note's body beyond the rendered template
- **Then:** the skill's instructions direct it to write that content itself (using its own file-editing tools) to the path `ishi new` printed, rather than looking for an `ishi` flag to pass body content through

---

## User Story 004 ✅

- **Summary:** The skill tells an agent how to file and archive items without being prompted
- **Depends on:** Story 001 (the installed skill), [move.md](move.md) Story 001, Story 002 (the `--yes` non-interactive forms this documents)

### Use Case

- **As an** agent reorganizing or archiving items on a user's behalf
- **I want to** the skill to document `ishi move`/`ishi archive`/`ishi unarchive` with `--yes`
- **so that** I don't block on the interactive summary-confirmation prompt those commands show by default

### Acceptance Criteria

- **Scenario:** Skill documents the non-interactive filing form
- **Given:** the `ishi` skill is installed and loaded
- **When:** an agent needs to move, archive, or unarchive an item
- **Then:** the skill's instructions direct it to pass `--yes` on `ishi move`/`ishi archive` to accept the suggested summary without a confirmation prompt, and to treat `ishi review`'s interactive walk as off-limits for unattended use

---

## User Story 005 ✅

- **Summary:** The skill warns an agent away from commands that can't run unattended
- **Depends on:** Story 001 (the installed skill), [review.md](review.md) Story 001

### Use Case

- **As an** agent operating without a human available to answer prompts
- **I want to** the skill to call out which `ishi` commands are inherently interactive
- **so that** I don't invoke something like `ishi review` (which walks every project/area prompting keep/archive/skip) and hang indefinitely

### Acceptance Criteria

- **Scenario:** Skill documents interactive-only commands as out of bounds
- **Given:** the `ishi` skill is installed and loaded
- **When:** an agent is deciding which `ishi` command to run for a given task
- **Then:** the skill's instructions explicitly list `ishi review` (and `ishi new`/`ishi config edit` invoked without the arguments that skip `$EDITOR`) as commands that require a human, and direct the agent to surface those as suggestions to the user rather than running them itself
