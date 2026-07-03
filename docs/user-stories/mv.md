# User Stories: `tk mv`

## User Story 001

- **Summary:** Get a clear error instead of a silent guess when unwrapping a directory item isn't supported
- **Status:** Not started
- **Depends on:** [new.md](new.md) Story 003, Story 004 (project/area scaffolding — the directory items this rejection applies to)

### Use Case

- **As a** Tick user who wants to move a `project` or `area` item back to `inbox` or `resource`
- **I want to** be told that unwrapping a directory into a flat file isn't supported
- **so that** Tick never has to guess which file inside the directory becomes the flat file, and I don't lose the rest of the directory's contents silently

### Acceptance Criteria

- **Scenario:** Moving a project directory to `inbox` or `resource` is rejected
- **Given:** `<item>` exists as a directory under `1-Projects` or `2-Areas`
- **When:** I run `tk mv <item> inbox` or `tk mv <item> resource`
- **Then:** Tick prints an error explaining that unwrapping a directory into a flat file is not yet supported
- **and Then:** no files or directories are moved, created, or modified

- **Scenario:** Moving an area directory to `inbox` or `resource` is rejected
- **Given:** `<item>` exists as a directory under `2-Areas`
- **When:** I run `tk mv <item> inbox` or `tk mv <item> resource`
- **Then:** Tick prints an error explaining that unwrapping a directory into a flat file is not yet supported
- **and Then:** no files or directories are moved, created, or modified
