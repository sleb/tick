# Tick: High-Level Design

## Goals

Keep the design simple: a small set of modules with clear, narrow contracts.
Filesystem/business logic stays separate from argument parsing and terminal I/O
so it can be tested without a real shell, editor, or terminal.

## Components

```
            ┌─────────┐
            │   cli   │  parses argv, prompts user, prints output
            └────┬────┘
                 │ calls
     ┌───────────┼───────────┐
     ▼           ▼           ▼
 ┌───────┐  ┌─────────┐  ┌────────┐
 │ items │  │ review  │  │ editor │
 └───┬───┘  └────┬────┘  └────────┘
     │           │
     ▼           ▼
 ┌─────────────────────┐
 │      workspace       │  resolves PARA root + category paths
 └──────────┬───────────┘
            ▼
       ┌─────────┐
       │ config  │  .tick.toml (folder names, default extension)
       └─────────┘

 ┌──────────┐
 │ category │  shared Inbox/Project/Area/Resource/Archive vocabulary
 └──────────┘  (used by cli, workspace, items, review)
```

### `category`

Shared vocabulary type, no I/O.

- `enum Category { Inbox, Project, Area, Resource, Archive }`
- `Category::is_directory_style() -> bool` — true for `Project`/`Area` (scaffolded
  dir + `index.md`), false for `Inbox`/`Resource` (flat file). `Archive` defers to
  the origin category it's preserving.

### `config`

Parses `.tick.toml`. Pure data, one file read.

- `struct Config { category_dirs: [String; 5], default_extension: String, templates: Templates }`
  — `category_dirs` is indexed by `Category as usize`, so `Category`'s
  discriminants and this array's order must stay in sync (enforced by
  `Workspace::category_dir` using the cast directly, rather than a
  hand-written match).
- `struct Templates { note: String, project: String, area: String, resource: String, daily: String }`
  — one field per category template, plus `daily` (not category-indexed,
  since `Category` has no `Daily` variant — `tk daily`/`tk new --daily`
  render it directly rather than going through `Templates::for_category`).
- `Templates::for_category(&self, category: Category) -> &str` — maps
  `Inbox`/`Project`/`Area`/`Resource` to `note`/`project`/`area`/`resource`
  respectively; panics on `Archive`, since `items::create` is never called
  with `Category::Archive` (items only arrive there via `items::mv`).
- `Config::default() -> Config` — `0-Inbox`, `1-Projects`, `2-Areas`,
  `3-Resources`, `4-Archive`, `md`, and the default `note` template.
- `Config::load(path: &Path) -> Result<Config>` — reads `.tick.toml` if present,
  falls back to defaults for any field it omits.
- `render(template: &str, title: &str, date: &str) -> String` — fills in
  `{{date}}` and `{{title}}`, leaving `{{cursor}}` untouched (that marker is
  `Editor`'s job — see below).

### `workspace`

Answers "where do things live?" for every other component.

- `struct Workspace { root: PathBuf, config: Config }`
- `Workspace::discover(start: &Path) -> Result<Workspace>` — walks up from
  `start` looking for `.tick.toml` or the five category dirs.
- `Workspace::category_dir(&self, category: Category) -> PathBuf`
- `struct InitReport { created: Vec<String> }` — names (in
  `Config::default().category_dirs` order) of the category dirs `init`
  actually created; empty means the target was already a complete PARA
  system.
- `check_collision(target: &Path) -> Result<()>` — errors if `target`
  exists and isn't a directory. Called unconditionally by `init` for both
  the current-directory and named-subdirectory forms; it's a no-op for the
  current-directory form since `cwd` is always a directory. Directories
  with unrelated contents (a `README`, `.git`, etc.) are never a
  collision, in either form — `init` just creates whichever category dirs
  are missing alongside them.
- `init(target: &Path) -> Result<InitReport>` — creates `target` if it
  doesn't exist, then creates whichever of the five default-named category
  dirs are missing under it. No `.tick.toml` is written; the created dirs
  are discoverable later via `Workspace::discover`'s bare-category-dirs
  fallback.

### `items`

All filesystem operations. Takes a `Workspace` and `Category`, returns
structured results — no printing, no prompting.

- `create(ws: &Workspace, category: Category, name: &str, content: &str) -> Result<PathBuf>`
  — creates a flat file or a scaffolded `dir/index.md`, appending the default
  extension if the name has none, and writing `content` into it. Returns the
  path created (the `index.md` path for directory-style categories).
  `content` is caller-rendered: `cli::run_new`'s named-file path renders
  `ws.config.templates.for_category(category)` with `{{title}}` set to `name`
  before calling `create`, so every creation path (interactive editor capture
  and non-interactive named creation alike) writes the category's template
  rather than a raw string.
- `mv(ws: &Workspace, item: &Path, target: Category) -> Result<PathBuf>` —
  moves a file or project/area directory; wraps a flat file into a new
  directory when moving into `Project`/`Area`; when moving to `Archive`,
  preserves the item's origin category as a subfolder.
- `struct ListedItem { name: String, title: String, updated_days_ago: u64 }` —
  `name` is the dir/file name (`<OriginCategory>/<name>` for `Archive`);
  `title` comes from `infer_title` below, falling back to `name` if it
  returns `None`; `updated_days_ago` is the age of the item's `index.md`
  (`Project`/`Area`) or file (others) mtime, the same source `status` uses
  for its `updated_days_ago` facts.
- `list(ws: &Workspace, category: Category, filter: Option<&str>) -> Result<Vec<ListedItem>>`
  — rows sorted alphabetically by `name`; `filter`, if given, is matched as a
  case-insensitive substring against `name` or `title`.
- `infer_title(content: &str) -> Option<String>` — skips a leading YAML
  frontmatter block if present, then returns the first Markdown heading
  line's text (any `#` level), or `None` if none is found. A heading line
  with empty text after the marker doesn't count as found; the search
  continues to any heading further down. Conceptually the same
  frontmatter-skip-then-find-heading logic as `editor::suggest_filename`
  (which then slugifies the heading into a filename), implemented
  independently in `items` — `items` and `editor` still don't depend on
  each other, per the module boundaries below.
- `struct StatusItem { name: String, title: String, updated_days_ago: u64, reviewed_days_ago: Option<u64> }`
  — one per `Project`/`Area`; `name`/`title`/`updated_days_ago` mirror
  `ListedItem` (same `infer_title` + mtime sourcing); `reviewed_days_ago` is
  the age of the item's `index.md` frontmatter `last_reviewed` field, or
  `None` if the field is absent (never reviewed).
- `status(ws: &Workspace) -> Result<StatusReport>` where
  `StatusReport { counts: [usize; 5], projects: Vec<StatusItem>, areas: Vec<StatusItem> }`
  — `counts` is per-category totals in `Category` order; `projects`/`areas`
  are sorted alphabetically by `name`, same convention as `list`. There is no
  staleness threshold or flagging — `status` reports the `updated_days_ago`/
  `reviewed_days_ago` facts and leaves judgment to the user.
- `read_last_reviewed(ws: &Workspace, item: &Path) -> Result<Option<u64>>` —
  reads the `last_reviewed` frontmatter field from a `Project`/`Area`'s
  `index.md`, if present, and returns its age in days. Shared by `status`
  (read) and `review` (read, to decide whether to overwrite on `[k]eep`).
- `write_last_reviewed(ws: &Workspace, item: &Path) -> Result<()>` — sets the
  `index.md` frontmatter's `last_reviewed` field to today's date, adding the
  field if absent and preserving every other frontmatter key and the body
  unchanged. Called by `review` on `[k]eep`, never by `status` (read-only).

### `editor`

Isolated so it's mockable in tests — no real `$EDITOR` needed to test the CLI
prompt logic. Splits into one impure entry point and a pure core so the
filename-inference logic is directly unit-testable without spawning a real
editor process or racing the system clock.

- `Editor` trait: `capture(&self, seed: &str) -> Result<(String, String)>` —
  implemented once as `RealEditor` (writes `seed` — the rendered template,
  with `{{title}}` empty and `{{cursor}}` marking the starting line — to a
  scratch file, opens `$EDITOR` on it via a `+<line>` argument when a cursor
  line is present, reads it back) and once per test as a fake. Returns
  `(content, suggested_filename)`.
- `suggest_filename(content: &str) -> String` — pure. Skips a leading YAML
  frontmatter block if present, then looks for the first Markdown heading
  line (any `#` level) with non-blank text after the marker in the
  remainder and slugifies it; a heading line whose text is empty (e.g. a
  pre-populated `# {{cursor}}` title left untouched) doesn't count as
  found — the search continues past it, including to headings further
  down the file. If no such heading is found, falls back to the first
  non-blank line after the frontmatter; if that's also absent (or the only
  candidate was the blank heading with no other content), falls back to a
  timestamp-based name. Internally delegates to a `SystemTime`-parameterized
  helper so the timestamp fallback is deterministic in tests.

### `review`

Orchestrates the weekly-review walk, built on `items` + `editor`'s prompting
pattern.

- `run(ws: &Workspace, ui: &mut dyn Ui) -> Result<()>` — iterates `Project` and
  `Area` items, reads each `index.md`, asks the `Ui` to keep/archive/skip.
  `[k]eep` calls `items::write_last_reviewed`; `[a]rchive` calls `items::mv`
  (origin category preserved as usual, per `mv`) and does not touch
  `last_reviewed`; `[s]kip` calls neither.

### `cli`

The only component that touches argv, stdin, and stdout. A `clap`-derived
`Command` enum matching the command table in the README, dispatching to
`items`/`review`/`editor` and rendering their results.

- `Ui` trait (implemented once for a real terminal, once for tests):
  `confirm(prompt: &str, default: &str) -> Result<String>`,
  `choose(prompt: &str, options: &[&str]) -> Result<char>`.
- `run_new(ws: &Workspace, editor: &dyn Editor, ui: &mut dyn Ui, category: Category, filename: Option<&str>) -> Result<PathBuf>`
  — when `filename` is given, renders `ws.config.templates.for_category(category)`
  with `{{title}}` set to `filename` and `{{date}}` set to today, then calls
  `items::create(ws, category, filename, &rendered)` directly, non-interactively;
  `category` is what makes `--project`/`--area`/`--resource` scaffold into the
  right place (and render the right template) instead of always `Inbox`/`note`.
  When `filename` is `None`, seeds `$EDITOR` with the rendered `note` template
  and prompts for the inferred name exactly as today, always into `Inbox` —
  capturing directly into a project/area/resource with no filename is Story
  010, not yet implemented. `main` maps `--project`/`--area`/`--resource`
  (mutually exclusive, via a `clap` `ArgGroup`) to `Category`, defaulting to
  `Inbox` when none are given.
- `run_init(cwd: &Path, name: Option<&str>) -> Result<String>` — resolves
  the target (`cwd` or `cwd.join(name)`) and its display form (`.` or
  `./<name>`), calls `workspace::init` (which runs `check_collision`
  internally for both forms), and renders the outcome (full create /
  partial fill-in / already-complete) into the exact message `main`
  prints.

## Notes

- `category` and `config` have no dependencies on anything else — they're the
  vocabulary and settings every other module shares.
- `workspace` depends only on `config` + `category`.
- `items` and `editor` depend only on `workspace` — they don't know about each
  other or about `cli`.
- `review` composes `items` with a `Ui`, but doesn't know about `clap` or argv.
- `cli` is the only place that does terminal I/O; every other module returns
  data or `Result`s so it can be unit-tested directly.
