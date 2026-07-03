# User Stories: `tk review`

`review` walks through every project and area one at a time, prompting
`[k]eep`/`[a]rchive`/`[s]kip` for each — a guided version of PARA's weekly
review ritual. What each choice *does* to `index.md`'s `last_reviewed`
frontmatter is specified in [status.md](status.md)'s User Story 004, since
that's the fact `tk status` reads back; this file covers the walk itself —
order, the per-item prompt, and what `[a]rchive` does to the filesystem.

---

## User Story 001

- **Summary:** `review` walks every project, then every area, each sorted alphabetically by name
- **Status:** Not started
- **Depends on:** [new.md](new.md) Story 003, Story 004 (projects/areas to walk)

### Use Case

- **As a** Tick user doing a weekly review
- **I want to** be walked through all my projects and areas in a predictable order
- **so that** I can review my whole active set in one pass without missing anything or seeing the same item twice

### Acceptance Criteria

- **Scenario:** Projects are walked before areas, each group alphabetical by name
- **Given:** I am inside an initialized PARA system with projects `website-redesign` and `my-project`, and areas `health` and `finances`
- **When:** I run `tk review`
- **Then:** Tick prompts for `my-project`, then `website-redesign`, then `finances`, then `health`, in that order

- **Scenario:** Each prompt names the item's category, name, and how long ago it was last updated
- **Given:** I am inside an initialized PARA system with a project `website-redesign` last modified 12 days ago
- **When:** `tk review` reaches `website-redesign`
- **Then:** Tick prints:
  ```
  Project: website-redesign (last updated 12 days ago)
    [k]eep  [a]rchive  [s]kip?
  ```

- **Scenario:** An area's prompt is labeled `Area`, not `Project`
- **Given:** I am inside an initialized PARA system with an area `finances` last modified 4 days ago
- **When:** `tk review` reaches `finances`
- **Then:** Tick prints:
  ```
  Area: finances (last updated 4 days ago)
    [k]eep  [a]rchive  [s]kip?
  ```

- **Scenario:** A system with no projects or areas has nothing to review
- **Given:** I am inside an initialized PARA system with no projects and no areas
- **When:** I run `tk review`
- **Then:** Tick prints a message that there is nothing to review
- **and Then:** the command exits successfully without prompting

- **Scenario:** The review ends after the last item with no further prompts
- **Given:** I am inside an initialized PARA system with exactly one area, `health`
- **When:** I run `tk review` and choose `[k]eep` for `health`
- **Then:** Tick exits successfully after that one prompt, with no further `[k]eep [a]rchive [s]kip?` prompt

---

## User Story 002

- **Summary:** `[a]rchive` moves the item the same way `tk mv <item> archive` would, and review moves on to the next item
- **Status:** Not started
- **Depends on:** Story 001 (the walk this action operates within)

### Use Case

- **As a** Tick user doing a weekly review
- **I want to** archive a project or area right from the review prompt
- **so that** I don't have to interrupt the review to run `tk mv` separately for things I've decided are done

### Acceptance Criteria

- **Scenario:** Archiving a project during review files it under the Archive's Projects subfolder
- **Given:** I am mid-`tk review` on project `website-redesign`
- **When:** I choose `[a]rchive`
- **Then:** `website-redesign` is moved from `1-Projects/website-redesign` to `4-Archive/Projects/website-redesign`, exactly as `tk mv website-redesign archive` would move it (see [mv.md](mv.md))
- **and Then:** review continues on to the next item in the walk order

- **Scenario:** Archiving an area during review files it under the Archive's Areas subfolder
- **Given:** I am mid-`tk review` on area `finances`
- **When:** I choose `[a]rchive`
- **Then:** `finances` is moved from `2-Areas/finances` to `4-Archive/Areas/finances`
- **and Then:** review continues on to the next item in the walk order

- **Scenario:** An archived item is not revisited later in the same review
- **Given:** I am inside an initialized PARA system with projects `my-project` and `website-redesign`
- **When:** I run `tk review`, choose `[a]rchive` for `my-project`, then reach the next prompt
- **Then:** the next prompt is for `website-redesign`, not `my-project` again

---

## User Story 003

- **Summary:** `[k]eep` and `[s]kip` leave the item in place and advance the walk
- **Status:** Not started
- **Depends on:** Story 001 (the walk this action operates within)

### Use Case

- **As a** Tick user doing a weekly review
- **I want to** keep or skip an item without it moving or disappearing from view
- **so that** I can review items that are still active without accidentally filing them away

### Acceptance Criteria

- **Scenario:** Choosing `[k]eep` leaves the item under its current category and advances to the next item
- **Given:** I am inside an initialized PARA system with projects `my-project` and `website-redesign`
- **When:** I run `tk review`, choose `[k]eep` for `my-project`
- **Then:** `my-project` is still at `1-Projects/my-project` (frontmatter changes are covered in [status.md](status.md) User Story 004)
- **and Then:** the next prompt is for `website-redesign`

- **Scenario:** Choosing `[s]kip` leaves the item untouched and advances to the next item
- **Given:** I am inside an initialized PARA system with projects `my-project` and `website-redesign`
- **When:** I run `tk review`, choose `[s]kip` for `my-project`
- **Then:** `my-project` is still at `1-Projects/my-project`, unmoved and unmodified
- **and Then:** the next prompt is for `website-redesign`
