# User Stories: `tk status`

`status` prints an at-a-glance summary of the PARA system: a count per
category, plus a per-item breakdown for Projects and Areas — the two
categories `tk review` acts on — showing how long ago each was last updated
and last reviewed. Inbox, Resources, and Archive stay counts-only, since they
aren't part of the review loop and can grow large enough that a per-item
listing would be unreadable (see [list](list.md) for a filterable per-item
view of those categories instead).

There is deliberately no staleness threshold or flagging. Earlier drafts of
this doc considered a `stale_after_days` config option, but `status` just
reports the facts (`updated: ...`, `reviewed: ...`) and leaves the judgment
call to the user — see [spec-gaps.md](../spec-gaps.md).

Column/fact definitions, used throughout:

- **Name** / **Title** — same as `list`'s columns (`items::infer_title`):
  Name is the project/area directory name; Title comes from the first
  Markdown heading in `index.md` (skipping a leading frontmatter block),
  falling back to Name if no heading is found.
- **Updated** — how long ago the item's `index.md` was last modified, in the
  same raw-days convention as `list` (`today`, `1 day ago`, `12 days ago`,
  ...).
- **Reviewed** — how long ago the item's `index.md` frontmatter
  `last_reviewed` field was set, in the same convention, or `never` if the
  field is absent. Only `tk review`'s `[k]eep` action writes this field
  (adding it if absent, overwriting it if present) — `[a]rchive` and
  `[s]kip` leave it untouched. New projects/areas created by `tk new` never
  have the field, since their templates don't set it.

Projects and areas are each sorted alphabetically by Name, matching `list`.

---

## User Story 001

- **Summary:** `status` prints a count per category
- **Status:** Not started
- **Depends on:** [new.md](new.md) Story 002, Story 003, Story 004, Story 005 (items to count across all five categories)

### Use Case

- **As a** Tick user
- **I want to** see how many items are in each PARA category
- **so that** I can get a quick sense of where things stand without listing every category individually

### Acceptance Criteria

- **Scenario:** All five categories are counted
- **Given:** I am inside an initialized PARA system with 2 inbox items, 3 projects, 2 areas, 5 resources, and 12 archived items
- **When:** I run `tk status`
- **Then:** Tick prints a line for each category with its name and count, in `Inbox`/`Projects`/`Areas`/`Resources`/`Archive` order:
  ```
  Inbox       2
  Projects    3
  Areas       2
  Resources   5
  Archive     12
  ```

- **Scenario:** An empty PARA system reports all zero counts
- **Given:** I am inside a freshly initialized PARA system with no items in any category
- **When:** I run `tk status`
- **Then:** Tick prints all five category lines with a count of `0`, and no per-item rows follow `Projects`/`Areas`
- **and Then:** the command exits successfully (no error)

---

## User Story 002

- **Summary:** Projects and Areas get a per-item breakdown showing how long ago each was updated
- **Status:** Not started
- **Depends on:** Story 001 (category counts), [list.md](list.md) Story 001, Story 005 (shared Name/Title inference convention)

### Use Case

- **As a** Tick user
- **I want to** see each project/area's name, title, and last-updated age under its category's count
- **so that** I can spot what's gone quiet without running `tk list project` and `tk list area` separately

### Acceptance Criteria

- **Scenario:** Projects list under the Projects count, sorted alphabetically by name
- **Given:** I am inside an initialized PARA system with projects `website-redesign` (`index.md` heading `# Website Redesign`, last modified 2 days ago) and `my-project` (`index.md` heading `# My Project`, last modified 21 days ago)
- **When:** I run `tk status`
- **Then:** Tick prints the `Projects` count line followed by one row per project, sorted alphabetically by name, each showing Name, Title, and `updated: ...`:
  ```
  Projects    2
  `- my-project         My Project         updated: 21 days ago
  `- website-redesign   Website Redesign   updated: 2 days ago
  ```

- **Scenario:** Areas list under the Areas count, using the same row format
- **Given:** I am inside an initialized PARA system with an area `health` (`index.md` heading `# Health`, last modified today)
- **When:** I run `tk status`
- **Then:** Tick prints the `Areas` count line followed by:
  ```
  Areas       1
  `- health   Health   updated: today
  ```

- **Scenario:** A project with no Markdown heading falls back to its Name for Title
- **Given:** I am inside an initialized PARA system with a project `quick-idea` whose `index.md` has no Markdown heading
- **When:** I run `tk status`
- **Then:** Tick prints a row with `quick-idea` in both the Name and Title positions

- **Scenario:** Inbox, Resources, and Archive show only their count, never per-item rows
- **Given:** I am inside an initialized PARA system with 2 inbox items and 5 resources
- **When:** I run `tk status`
- **Then:** the `Inbox` and `Resources` lines are followed immediately by the next category line (or, for `Resources`, by `Archive`), with no `` `- `` rows in between

---

## User Story 003

- **Summary:** Each project/area row also shows how long ago it was last reviewed, or `never`
- **Status:** Not started
- **Depends on:** Story 002 (per-item row this fact is appended to), Story 004 (defines when `last_reviewed` is written)

### Use Case

- **As a** Tick user doing a weekly review
- **I want to** see which projects/areas I've reviewed recently and which I haven't touched in `tk review` at all
- **so that** I know where to focus without re-walking items I already just kept

### Acceptance Criteria

- **Scenario:** A project reviewed via `tk review`'s `[k]eep` shows its reviewed age
- **Given:** I am inside an initialized PARA system with a project `website-redesign` whose `index.md` frontmatter has `last_reviewed` set to 3 days ago
- **When:** I run `tk status`
- **Then:** Tick prints the `website-redesign` row with `reviewed: 3 days ago` alongside its `updated: ...` fact:
  ```
  `- website-redesign   Website Redesign   updated: 2 days ago   reviewed: 3 days ago
  ```

- **Scenario:** A project that has never been kept in a review shows `reviewed: never`
- **Given:** I am inside an initialized PARA system with a project `my-project` created by `tk new --project` and never passed through `tk review`
- **When:** I run `tk status`
- **Then:** Tick prints the `my-project` row with `reviewed: never`
- **and Then:** `my-project`'s `index.md` frontmatter has no `last_reviewed` field — `status` never writes one, only reads

- **Scenario:** Areas report `reviewed: ...` the same way as projects
- **Given:** I am inside an initialized PARA system with an area `finances` whose `index.md` frontmatter has `last_reviewed` set to 4 days ago
- **When:** I run `tk status`
- **Then:** Tick prints the `finances` row with `reviewed: 4 days ago`

---

## User Story 004

- **Summary:** `tk review`'s `[k]eep` action stamps `last_reviewed`; `[a]rchive` and `[s]kip` don't
- **Status:** Not started
- **Depends on:** [review.md](review.md) Story 001, Story 002, Story 003 (the walk and its `[k]eep`/`[a]rchive`/`[s]kip` actions)

### Use Case

- **As a** Tick user running `tk review`
- **I want to** have `last_reviewed` updated only when I actually confirm an item is still relevant
- **so that** `tk status`'s `reviewed: ...` fact reflects a real decision, not just that the item was looked at

### Acceptance Criteria

- **Scenario:** Choosing `[k]eep` sets `last_reviewed` to today, adding the field if absent
- **Given:** I am mid-`tk review` on project `website-redesign`, whose `index.md` frontmatter has no `last_reviewed` field
- **When:** I choose `[k]eep`
- **Then:** `website-redesign`'s `index.md` frontmatter now has `last_reviewed` set to today's date
- **and Then:** every other frontmatter key and the body of `index.md` are unchanged

- **Scenario:** Choosing `[k]eep` again overwrites an existing `last_reviewed`
- **Given:** I am mid-`tk review` on project `website-redesign`, whose `index.md` frontmatter has `last_reviewed` set to 10 days ago
- **When:** I choose `[k]eep`
- **Then:** `website-redesign`'s `index.md` frontmatter `last_reviewed` is updated to today's date, replacing the 10-days-ago value

- **Scenario:** Choosing `[a]rchive` moves the item without touching `last_reviewed`
- **Given:** I am mid-`tk review` on project `website-redesign`, whose `index.md` frontmatter has no `last_reviewed` field
- **When:** I choose `[a]rchive`
- **Then:** `website-redesign` is moved to the Archive the same way `tk mv website-redesign archive` would (origin category preserved)
- **and Then:** the moved `index.md`'s frontmatter still has no `last_reviewed` field — archiving is not treated as a review

- **Scenario:** Choosing `[s]kip` leaves the item and its frontmatter untouched
- **Given:** I am mid-`tk review` on project `website-redesign`, whose `index.md` frontmatter has `last_reviewed` set to 10 days ago
- **When:** I choose `[s]kip`
- **Then:** `website-redesign` is not moved
- **and Then:** its `index.md` frontmatter `last_reviewed` is still 10 days ago, unchanged
