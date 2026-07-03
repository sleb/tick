# User Stories: `tk list`

`tk list` originally just printed bare paths (one per line). These stories
redesign it to print a small table — **Name**, **Title**, **Updated** — so a
user can tell what a note is about and how fresh it is without opening every
file. See gap in [spec-gaps.md](../spec-gaps.md) (§1, `list`) for the
ambiguities these stories resolve: filter semantics, empty-category output,
and the project/area-lists-dirs vs resource/inbox/archive-lists-files
distinction.

Column definitions, used throughout:

- **Name** — the identifier you'd pass to `tk mv`/other commands: the
  directory name for `project`/`area`, the filename (without extension) for
  `resource`/`inbox`, and `<OriginCategory>/<name>` for `archive` (since
  archived items from different origin categories can share a bare name).
- **Title** — inferred from the item's `index.md` (`project`/`area`) or the
  file itself (`resource`/`inbox`/`archive`): skip a leading YAML frontmatter
  block if present, then take the first Markdown heading line's text (any
  `#` level), the same convention `editor::suggest_filename` uses. If no
  heading is found, Title falls back to repeating the Name, so the column is
  never blank.
- **Updated** — how long ago the item's `index.md` (`project`/`area`) or the
  file itself (`resource`/`inbox`/`archive`) was last modified, expressed in
  raw days (`today`, `1 day ago`, `12 days ago`, ...) — the same convention
  `review`'s example transcript uses (`last updated 12 days ago`), and
  sourced from the same mtime `status` uses for its `updated_days_ago` facts
  (see [status.md](status.md)). Neither `list` nor `status` flags staleness
  or applies a threshold — both report raw ages and leave judgment to the
  user.

Rows are sorted alphabetically by Name. This is deterministic across runs,
unlike sorting by Updated.

---

## User Story 001

- **Summary:** Listing a category shows Name, Title, and Updated columns instead of bare paths
- **Depends on:** [new.md](new.md) Story 003, Story 004, Story 005 (project/area/resource items to list), [review.md](review.md) Story 001 (shared raw-days age convention)

### Use Case

- **As a** Tick user checking what's in a category
- **I want to** see each item's name, inferred title, and how recently it was touched
- **so that** I can tell what a note is about and how fresh it is without opening every file

### Acceptance Criteria

- **Scenario:** Listing projects shows the directory name, the title from `index.md`, and days since `index.md` was last modified
- **Given:** I am inside an initialized PARA system with two projects, `website-redesign` (`index.md` heading `# Website Redesign`, last modified 2 days ago) and `my-project` (`index.md` heading `# My Project`, last modified 21 days ago)
- **When:** I run `tk list project`
- **Then:** Tick prints a header row (`NAME`, `TITLE`, `UPDATED`) followed by one row per project, sorted alphabetically by name:
  ```
  NAME               TITLE              UPDATED
  my-project         My Project         21 days ago
  website-redesign   Website Redesign   2 days ago
  ```

- **Scenario:** Listing areas uses the same column format as projects
- **Given:** I am inside an initialized PARA system with an area `health` (`index.md` heading `# Health`, last modified today)
- **When:** I run `tk list area`
- **Then:** Tick prints:
  ```
  NAME     TITLE    UPDATED
  health   Health   today
  ```

- **Scenario:** Listing resources/inbox uses the flat file itself, not a directory
- **Given:** I am inside an initialized PARA system with a resource file `api-notes.md` (heading `# API Design Notes`, last modified 5 days ago)
- **When:** I run `tk list resource`
- **Then:** Tick prints:
  ```
  NAME        TITLE              UPDATED
  api-notes   API Design Notes   5 days ago
  ```
- **and Then:** no directory is created or expected for `api-notes` — it's read and reported as the flat file it is

---

## User Story 002

- **Summary:** Archived items show which category they came from, since names can collide across origins
- **Depends on:** Story 001 (base column format)

### Use Case

- **As a** Tick user browsing the archive
- **I want to** see which original category an archived item came from
- **so that** I can tell apart, say, an archived project and an archived resource that happen to share a name

### Acceptance Criteria

- **Scenario:** Archive listing prefixes Name with the origin category
- **Given:** I am inside an initialized PARA system with an archived project at `4-Archive/Projects/old-project` (heading `# Old Project`, last modified 4 months/~120 days ago) and an archived resource at `4-Archive/Resources/api-notes-v1` (heading `# API Notes v1`, last modified ~180 days ago)
- **When:** I run `tk list archive`
- **Then:** Tick prints Name values qualified with the origin category, sorted alphabetically by that qualified name:
  ```
  NAME                      TITLE            UPDATED
  Projects/old-project      Old Project      120 days ago
  Resources/api-notes-v1    API Notes v1     180 days ago
  ```

---

## User Story 003

- **Summary:** The optional filter matches a substring of either Name or Title, case-insensitively
- **Depends on:** Story 001 (base column format)

### Use Case

- **As a** Tick user with many items in a category
- **I want to** narrow the list to items whose name or title contains a word I remember
- **so that** I don't have to scan the whole category to find the one I want

### Acceptance Criteria

- **Scenario:** Filter matches a substring of Name
- **Given:** I am inside an initialized PARA system with projects `website-redesign` and `my-project`
- **When:** I run `tk list project web`
- **Then:** Tick prints only the `website-redesign` row

- **Scenario:** Filter matches a substring of Title even when Name doesn't contain it
- **Given:** I am inside an initialized PARA system with a project directory named `q3-initiative` whose `index.md` heading is `# Website Redesign Phase 2`
- **When:** I run `tk list project redesign`
- **Then:** Tick prints the `q3-initiative` row, matched on its title rather than its name

- **Scenario:** Filter is case-insensitive
- **Given:** I am inside an initialized PARA system with a project `website-redesign`
- **When:** I run `tk list project WEB`
- **Then:** Tick prints the `website-redesign` row

- **Scenario:** Filter matching nothing prints an empty-result message, not an error
- **Given:** I am inside an initialized PARA system with a project `website-redesign`
- **When:** I run `tk list project nonexistent`
- **Then:** Tick prints `No items in Projects matching "nonexistent".` and exits successfully (no error)

---

## User Story 004

- **Summary:** Listing an empty category prints a friendly message instead of nothing
- **Depends on:** Story 001 (base column format), Story 003 (empty-result message for a filter, extended here to no filter)

### Use Case

- **As a** Tick user checking a category I haven't used yet
- **I want to** get a clear message when there's nothing there
- **so that** I don't mistake silent empty output for a broken command

### Acceptance Criteria

- **Scenario:** Empty category without a filter
- **Given:** I am inside an initialized PARA system with no resources
- **When:** I run `tk list resource`
- **Then:** Tick prints `No items in Resources.` and exits successfully (no error), and no header row is printed

---

## User Story 005

- **Summary:** Title falls back to the item's Name when no heading can be inferred
- **Depends on:** Story 001 (base column format)

### Use Case

- **As a** Tick user with a note that has no Markdown heading (e.g. a quick capture I never titled)
- **I want to** still see a sensible Title column instead of a blank one
- **so that** the table stays readable even for untitled notes

### Acceptance Criteria

- **Scenario:** No frontmatter and no heading
- **Given:** I am inside an initialized PARA system with an inbox file `quick-thought.md` containing only plain text with no Markdown heading
- **When:** I run `tk list inbox`
- **Then:** Tick prints a row with `quick-thought` in both the Name and Title columns

- **Scenario:** Frontmatter present but no heading follows it
- **Given:** I am inside an initialized PARA system with an inbox file `quick-thought.md` containing a YAML frontmatter block followed by plain text with no heading
- **When:** I run `tk list inbox`
- **Then:** Tick prints a row with `quick-thought` in both the Name and Title columns — the frontmatter block itself is never mistaken for a heading
