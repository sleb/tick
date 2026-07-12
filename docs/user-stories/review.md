# User Stories: `ishi review`

`review` walks through every project and area one at a time, prompting
`[k]eep`/`[a]rchive`/`[s]kip` for each — a guided version of PARA's weekly
review ritual. What each choice *does* to `index.md`'s `last_reviewed`
frontmatter is specified in [status.md](status.md)'s User Story 004, since
that's the fact `ishi status` reads back; this file covers the walk itself —
order, the per-item prompt, and what `[a]rchive` does to the filesystem.

---

## User Story 001

- **Summary:** `review` walks every project, then every area, each sorted alphabetically by name
- **Status:** ✅
- **Depends on:** [new.md](new.md) Story 003, Story 004 (projects/areas to walk)

### Use Case

- **As a** Ishi user doing a weekly review
- **I want to** be walked through all my projects and areas in a predictable order
- **so that** I can review my whole active set in one pass without missing anything or seeing the same item twice

### Acceptance Criteria

- **Scenario:** Projects are walked before areas, each group alphabetical by name
- **Given:** I am inside an initialized PARA system with projects `website-redesign` and `my-project`, and areas `health` and `finances`
- **When:** I run `ishi review`
- **Then:** Ishi prompts for `my-project`, then `website-redesign`, then `finances`, then `health`, in that order

- **Scenario:** Each prompt names the item's category, name, and how long ago it was last updated
- **Given:** I am inside an initialized PARA system with a project `website-redesign` last modified 12 days ago
- **When:** `ishi review` reaches `website-redesign`
- **Then:** Ishi prints:
  ```
  Project: website-redesign (last updated 12 days ago)
    [k]eep  [a]rchive  [s]kip?
  ```

- **Scenario:** An area's prompt is labeled `Area`, not `Project`
- **Given:** I am inside an initialized PARA system with an area `finances` last modified 4 days ago
- **When:** `ishi review` reaches `finances`
- **Then:** Ishi prints:
  ```
  Area: finances (last updated 4 days ago)
    [k]eep  [a]rchive  [s]kip?
  ```

- **Scenario:** A system with no projects or areas has nothing to review
- **Given:** I am inside an initialized PARA system with no projects and no areas
- **When:** I run `ishi review`
- **Then:** Ishi prints a message that there is nothing to review
- **and Then:** the command exits successfully without prompting

- **Scenario:** The review ends after the last item with no further prompts
- **Given:** I am inside an initialized PARA system with exactly one area, `health`
- **When:** I run `ishi review` and choose `[k]eep` for `health`
- **Then:** Ishi exits successfully after that one prompt, with no further `[k]eep [a]rchive [s]kip?` prompt

---

## User Story 002

- **Summary:** `[a]rchive` moves the item the same way `ishi move <item> archive` would, and review moves on to the next item
- **Status:** ✅
- **Depends on:** Story 001 (the walk this action operates within)

### Use Case

- **As a** Ishi user doing a weekly review
- **I want to** archive a project or area right from the review prompt
- **so that** I don't have to interrupt the review to run `ishi move` separately for things I've decided are done

### Acceptance Criteria

- **Scenario:** Archiving a project during review files it under the Archive's Projects subfolder
- **Given:** I am mid-`ishi review` on project `website-redesign`
- **When:** I choose `[a]rchive`
- **Then:** `website-redesign` is moved from `1-Projects/website-redesign` to `4-Archive/Projects/website-redesign`, exactly as `ishi move website-redesign archive` would move it (see [move.md](move.md))
- **and Then:** review continues on to the next item in the walk order

- **Scenario:** Archiving an area during review files it under the Archive's Areas subfolder
- **Given:** I am mid-`ishi review` on area `finances`
- **When:** I choose `[a]rchive`
- **Then:** `finances` is moved from `2-Areas/finances` to `4-Archive/Areas/finances`
- **and Then:** review continues on to the next item in the walk order

- **Scenario:** An archived item is not revisited later in the same review
- **Given:** I am inside an initialized PARA system with projects `my-project` and `website-redesign`
- **When:** I run `ishi review`, choose `[a]rchive` for `my-project`, then reach the next prompt
- **Then:** the next prompt is for `website-redesign`, not `my-project` again

---

## User Story 003

- **Summary:** `[k]eep` and `[s]kip` leave the item in place and advance the walk
- **Status:** ✅
- **Depends on:** Story 001 (the walk this action operates within)

### Use Case

- **As a** Ishi user doing a weekly review
- **I want to** keep or skip an item without it moving or disappearing from view
- **so that** I can review items that are still active without accidentally filing them away

### Acceptance Criteria

- **Scenario:** Choosing `[k]eep` leaves the item under its current category and advances to the next item
- **Given:** I am inside an initialized PARA system with projects `my-project` and `website-redesign`
- **When:** I run `ishi review`, choose `[k]eep` for `my-project`
- **Then:** `my-project` is still at `1-Projects/my-project` (frontmatter changes are covered in [status.md](status.md) User Story 004)
- **and Then:** the next prompt is for `website-redesign`

- **Scenario:** Choosing `[s]kip` leaves the item untouched and advances to the next item
- **Given:** I am inside an initialized PARA system with projects `my-project` and `website-redesign`
- **When:** I run `ishi review`, choose `[s]kip` for `my-project`
- **Then:** `my-project` is still at `1-Projects/my-project`, unmoved and unmodified
- **and Then:** the next prompt is for `website-redesign`

---

## User Story 004

- **Summary:** Drive a single item's review decision with a flag instead of the interactive prompt
- **Depends on:** Story 002 (`[a]rchive`'s move effect), Story 003 (`[k]eep`/`[s]kip`'s effects), [status.md](status.md) Story 004 (`last_reviewed` stamping)

### Use Case

- **As an** agent asked to run a weekly review on the user's behalf
- **I want to** run `ishi review <item> --keep`, `ishi review <item> --archive`, or `ishi review <item> --skip` for one named project or area at a time
- **so that** I can act on the user's review decisions without stdin, reusing Ishi's own move/stamp logic instead of hand-editing frontmatter and bypassing the command entirely

### Acceptance Criteria

- **Scenario:** `--keep` has the same effect as choosing `[k]eep` interactively
- **Given:** I am inside an initialized PARA system with a project `website-redesign` whose `index.md` frontmatter has no `last_reviewed` field
- **When:** I run `ishi review website-redesign --keep`
- **Then:** `website-redesign`'s `index.md` frontmatter now has `last_reviewed` set to today's date, and the item is not moved
- **and Then:** the command exits successfully and prints a one-line confirmation (e.g. `Kept website-redesign.`), not the interactive `[k]eep [a]rchive [s]kip?` prompt

- **Scenario:** `--archive` has the same effect as choosing `[a]rchive` interactively
- **Given:** I am inside an initialized PARA system with a project `website-redesign`
- **When:** I run `ishi review website-redesign --archive`
- **Then:** `website-redesign` is moved from `1-Projects/website-redesign` to `4-Archive/Projects/website-redesign`, exactly as `ishi move website-redesign archive` would, and its `last_reviewed` field is left untouched (adding one is not treated as a review, matching Story 002)

- **Scenario:** `--skip` has the same effect as choosing `[s]kip` interactively
- **Given:** I am inside an initialized PARA system with a project `website-redesign` whose `index.md` frontmatter has `last_reviewed` set to 10 days ago
- **When:** I run `ishi review website-redesign --skip`
- **Then:** `website-redesign` is not moved and its `last_reviewed` value is unchanged

- **Scenario:** An area name works the same way as a project name
- **Given:** I am inside an initialized PARA system with an area `finances`
- **When:** I run `ishi review finances --keep`
- **Then:** `finances`'s `index.md` frontmatter `last_reviewed` is set to today's date, the same as a project's `--keep`

- **Scenario:** Naming an item that isn't a project or area is a distinct, scriptable error
- **Given:** I am inside an initialized PARA system with a resource `api-notes` but no project or area named `api-notes`
- **When:** I run `ishi review api-notes --keep`
- **Then:** Ishi exits with an error explaining that `api-notes` isn't a project or area (not silently doing nothing), so an agent can detect the mistake from the exit rather than assuming it succeeded

- **Scenario:** Passing more than one of `--keep`/`--archive`/`--skip` is rejected before anything is touched
- **Given:** I am inside an initialized PARA system with a project `website-redesign`
- **When:** I run `ishi review website-redesign --keep --archive`
- **Then:** Ishi exits with an error that the flags are mutually exclusive, and `website-redesign` is not moved or modified

- **Scenario:** Naming an item without any decision flag still falls back to the interactive prompt
- **Given:** I am inside an initialized PARA system with a project `website-redesign`
- **When:** I run `ishi review website-redesign` with no `--keep`/`--archive`/`--skip` flag
- **Then:** Ishi prompts interactively for just that one item, the same `[k]eep [a]rchive [s]kip?` prompt Story 001 describes, rather than erroring or silently doing nothing
