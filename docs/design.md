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
 └───┬───┘  └────┬────┘  └───┬────┘
     │           │           │
     ▼           ▼           ▼
 ┌──────────────────────┐  ┌──────┐
 │      workspace       │  │ gist │  external crate: Markdown/frontmatter
 └──────────┬───────────┘  └──────┘  parsing (see `gist` below)
            ▼
       ┌─────────┐
       │ config  │  .tick.toml (folder names, default extension)
       └─────────┘

 ┌──────────┐
 │ category │  two vocabularies, no I/O: Category (filing) + Kind (creation)
 └──────────┘  (used by cli, workspace, items, review, config)
```

### `category`

Shared vocabulary types, no I/O. Two distinct enums, deliberately not one —
see "Filing vocabulary vs. creation vocabulary" below for why.

- `enum Category { Inbox, Project, Area, Resource, Archive }` — **where an
  item is filed.** Consumed by every command that manages items that
  already exist: `workspace::category_dir`, `items::mv`'s destination,
  `items::list`/`status`'s per-category iteration, and `Archive`'s
  origin-folder tracking.
- `Category::is_directory_style() -> bool` — true for `Project`/`Area` (scaffolded
  dir + `index.md`), false for `Inbox`/`Resource` (flat file). `Archive` defers to
  the origin category it's preserving.
- `Category::archive_origin_name(&self) -> &'static str` — the fixed subfolder
  name under `Archive` an item of this category is filed under when archived
  (e.g. `Project` -> `"Projects"`); never called with `Archive` itself. A
  separate fixed vocabulary from `Config::category_dirs` — origin subfolders
  aren't user-configurable independently of the top-level Archive folder name.
  `Category::archivable() -> [Category; 4]` is the four categories an item can
  be archived from (every variant except `Archive`), used to iterate origin
  subfolders in `items::list`/`mv`.
- `Category::display_name(&self) -> &'static str` — plural display name for
  user-facing messages (`list`'s no-match/empty messages), covering all five
  variants including `Archive` itself. Total where `archive_origin_name` is
  deliberately partial.
- `enum Kind { Inbox, Project, Area, Resource, Daily }` — **what `new`/`daily`
  create.** Consumed only by the creation path: `cli::run_new`'s dispatch
  and `Templates::for_kind`.
- `Kind::category(&self) -> Category` — the `Category` a created item of
  this `Kind` files into. `Kind::Inbox` and `Kind::Daily` both map to
  `Category::Inbox` — a daily note has no filing location of its own, just
  a different template and a different creation/reopen lifecycle than a
  plain Inbox capture.

#### Filing vocabulary vs. creation vocabulary

`Category` and `Kind` look almost identical (four of five variants line up
1:1: `Inbox`/`Project`/`Area`/`Resource`) but answer different questions,
and conflating them is a recurring trap worth naming explicitly:

- **`Category` = "where does this item live?"** — a filing-location fact
  that's true of an item forever, independent of how it was created. It's
  the vocabulary `mv`, `list`, `status`, and archive-origin tracking need,
  because those commands only care about current location, never about
  how the item came to be there.
- **`Kind` = "what is being created?"** — a creation-flavor fact that only
  matters at the moment `new`/`daily` runs (which template to render,
  which control flow to follow — interactive capture vs. non-interactive
  named file vs. daily's create-or-reopen). Once the file exists on disk,
  its `Kind` is forgotten; only its `Category` (i.e., which folder it's
  sitting in) persists.

The mismatch at the edges is the signal that these needed to be two types
instead of one:

- A daily note needs its own template and lifecycle but **has no folder of
  its own** — it always lands in `Category::Inbox`. That's why it's a
  `Kind` (`Kind::Daily`) with no matching `Category` variant, rather than a
  `Category::Daily` that would just be a second name for the Inbox folder
  everywhere `Category` is matched (`mv`, `list`, `status`, `Archive` would
  all have to special-case a category that behaves identically to `Inbox`).
- `Category::Archive` needs a folder (and origin-subfolder tracking) but
  **items never arrive there via `new`** — only via `mv`. That's why it has
  no matching `Kind` variant, rather than `Templates`/`run_new` having to
  special-case or panic on a creation flavor nothing ever creates (which is
  exactly what the old, single-enum design did: `Templates::for_category`
  had to `panic!` on `Archive`).

**Rule of thumb for future additions:** a new artifact type that needs its
own template or creation control-flow, but always resolves into an
existing folder, is a new `Kind` variant (mapped onto whichever `Category`
it files into). A new artifact type that needs its own folder is a new
`Category` variant (plus a matching `Kind` variant only if `new` should be
able to create it directly — some categories, like `Archive`, might not
be).

### `config`

Parses `.tick.toml` and layers it across three sources — built-in defaults,
`~/.tick.toml` (user), `./.tick.toml` (local) — per key.

- `struct Config { category_dirs: [String; 5], default_extension: String, templates: Templates }`
  — `category_dirs` is indexed by `Category as usize`, so `Category`'s
  discriminants and this array's order must stay in sync (enforced by
  `Workspace::category_dir` using the cast directly, rather than a
  hand-written match).
- `struct Templates { note: String, daily: String, project: String, area: String, resource: String }`
  — one field per `Kind` (see `category` above): `note` for `Kind::Inbox`,
  `daily` for `Kind::Daily`, etc.
- `Templates::for_kind(&self, kind: Kind) -> &str` — maps `Kind::Inbox` to
  `note`, `Kind::Daily` to `daily`, `Kind::Project`/`Area`/`Resource` to
  `project`/`area`/`resource`. Total — every `Kind` has a template, so
  unlike the old `Category`-indexed lookup this needs no panic branch
  (there's no `Kind::Archive` to be missing a template for).
- `Config::default() -> Config` — `0-Inbox`, `1-Projects`, `2-Areas`,
  `3-Resources`, `4-Archive`, `md`, and the default `note` template.
- `enum Source { Default, User, Local, LocalOverridesUser }` — which layer an
  effective value came from; `Source::comment(self) -> &'static str` gives
  the exact annotation `tk config` will print (`default`/`user`/`local`/
  `local, overrides user`).
- `struct ConfigOrigins { category_dirs: [Source; 5], default_extension: Source, templates: TemplateOrigins }`
  and `struct TemplateOrigins { note: Source, daily: Source, project: Source, area: Source, resource: Source }`
  — same shape as `Config`/`Templates`, parallel and provenance-only; no
  consumer besides the future `tk config` display path reads these.
- `Config::resolve(local_path: &Path, home_path: Option<&Path>) -> Result<(Config, ConfigOrigins)>`
  — reads `local_path` and, if given, `home_path`, and layers them over
  `Config::default()` independently per key (local wins over user, user
  wins over the built-in default). Neither file needs to exist; a missing
  file behaves as if it set no keys at all. Replaces the old single-file
  `Config::load`.
- `ConfigError` also has `AlreadyExists { path }` and `Write { path, source }`
  variants, for the writer below.
- `SCHEMA_JSON: &str` — the JSON Schema (draft 2020-12) for `.tick.toml`,
  embedded at compile time via `include_str!` from
  `assets/tick.schema.json` (hand-maintained, checked into the repo).
- `SCHEMA_FILENAME: &str = ".tick.schema.json"` — the sibling filename the
  `#:schema` directive points at and that `init` writes.
- `default_toml() -> String` — renders `Config::default()` as the exact
  `.tick.toml` shape documented in README.md's Configuration section: a
  leading `#:schema ./.tick.schema.json` directive (config.md 006),
  followed by nested `[folders]`/`[defaults]`/`[templates]` tables,
  templates as triple-quoted strings. Pure — no filesystem access; the
  directive just names the sibling file, `init` is responsible for
  actually writing it.
- `init(path: &Path) -> Result<()>` — writes `SCHEMA_JSON` to
  `path.parent().join(SCHEMA_FILENAME)`, then `default_toml()` to `path`
  (schema first, so a failed write leaves nothing for the caller to clean
  up and a retry doesn't hit a stale `AlreadyExists`). Errors with
  `AlreadyExists` (leaving `path` untouched, no schema file written either)
  if `path` already exists, rather than overwriting a user's
  customizations. Backs `tk config init`/`tk config init -g` (see
  `cli::run_config_init` below).
- `render(template: &str, title: &str, date: &str, time: &str, uuid: &str) -> String`
  — fills in `{{date}}`, `{{title}}`, `{{time}}`, and `{{uuid}}`, leaving
  `{{cursor}}` untouched (that marker is `Editor`'s job — see below). All
  five values are caller-supplied plain strings, so `render` stays a pure
  function with no dependency on `chrono`/`uuid` internals — `cli::run_new`
  computes `date`/`time` from one `Local::now()` call and generates one
  `Uuid::new_v4()` per invocation, even when the active template doesn't
  reference `{{time}}`/`{{uuid}}` (negligible cost, and avoids branching on
  template contents).

### `workspace`

Answers "where do things live?" for every other component.

- `struct Workspace { root: PathBuf, config: Config }`
- `Workspace::discover(start: &Path, home_config: Option<&Path>) -> Result<Workspace>`
  — walks up from `start` looking for `.tick.toml` or the five category
  dirs, layering `home_config` in via `Config::resolve` on whichever branch
  matches (so a user-level config still applies even when there's no local
  `.tick.toml` to discover by). `ConfigOrigins` is discarded here — nothing
  under `workspace` needs provenance, only the future `tk config` display
  path does, and that path calls `Config::resolve` directly.
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

### `gist`

An external crate ([sleb/gist](https://github.com/sleb/gist)), not a tick
module — pinned in `Cargo.toml` (`gist = { git = "...", tag = "v0.1.0" }`).
Parses a single note's Markdown/frontmatter (headings, tags, links, code
fences) with no filesystem access of its own; tick calls it with content
already read from disk. `items` and `editor` are the only tick modules that
use it, each independently — `gist` replaces what used to be two
independent hand-rolled implementations of "skip frontmatter, then find the
first heading" (one in `items::infer_title`, one in
`editor::suggest_filename`), so the module boundary that kept `items` and
`editor` from depending on each other is preserved by having both depend on
`gist` instead of on each other.

- `parser::first_heading_text(content: &str) -> Option<String>` — skips a
  leading YAML frontmatter block if present, then returns the first
  Markdown heading line's text (any `#` level) with non-blank text after
  the marker, or `None` if none is found. This is the function `items` and
  `editor` both call.
- `parser::frontmatter_body_offset(content: &str) -> usize` — the byte
  offset where the document body starts (`0` if there's no frontmatter
  block). `editor::suggest_filename` uses this to locate its
  first-non-blank-line fallback in the post-frontmatter body.
- `parser::parse`, `parser::extract_frontmatter`, `index::build`, and the
  rest of `gist`'s backlink/tag/link-resolution surface are not currently
  used by tick — `gist` was originally built for a different note-vault
  tool, and only its title-inference primitives overlap with tick's needs
  today.

### `items`

All filesystem operations. Takes a `Workspace` and `Category`, returns
structured results — no printing, no prompting.

- `item_path(ws: &Workspace, category: Category, name: &str) -> PathBuf` —
  pure path computation, no I/O: the directory-vs-flat-file branch `create`
  needs, factored out so callers can check whether an item already exists
  (`cli::run_daily`, deciding create-vs-reopen) without duplicating that
  branch or touching the filesystem themselves.
- `create(ws: &Workspace, category: Category, name: &str, content: &str) -> Result<PathBuf>`
  — computes the path via `item_path`, creates its parent directory, and
  writes `content` into it. Returns the path created (the `index.md` path
  for directory-style categories). `content` is caller-rendered:
  `cli::run_new`'s named-file path renders `ws.config.templates.for_kind(kind)`
  with `{{title}}` set to `name` before calling `create`, so every creation
  path (interactive editor capture and non-interactive named creation
  alike) writes the right template rather than a raw string.
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
- `infer_title(content: &str) -> Option<String>` — a thin wrapper over
  [`gist`](#gist)'s `parser::first_heading_text`: skips a leading YAML
  frontmatter block if present, then returns the first Markdown heading
  line's text (any `#` level), or `None` if none is found. A heading line
  with empty text after the marker doesn't count as found; the search
  continues to any heading further down. `editor::suggest_filename` calls
  the same `gist` function for its heading-detection step (then slugifies
  the result into a filename) — `items` and `editor` share `gist` as a
  common dependency rather than depending on each other, per the module
  boundaries below.
- `status(ws: &Workspace) -> Result<StatusReport>` where
  `StatusReport { counts: [usize; 5] }` — `counts` is per-category totals in
  `Category` order, computed by a private `count` that scans the same
  directories `list` does but skips the content read/`infer_title` call
  entirely. Implemented per
  [009-status-counts.md](lld/009-status-counts.md) (`status.md` 001).
- `struct StatusItem { name: String, title: String, updated_days_ago: u64, reviewed_days_ago: Option<u64> }`
  — one per `Project`/`Area`; `name`/`title`/`updated_days_ago` mirror
  `ListedItem` (same `infer_title` + mtime sourcing); `reviewed_days_ago` is
  the age of the item's `index.md` frontmatter `last_reviewed` field, or
  `None` if the field is absent (never reviewed). Lands with `status.md` 002,
  along with `StatusReport` gaining `projects: Vec<StatusItem>, areas:
  Vec<StatusItem>` fields (sorted alphabetically by `name`, same convention
  as `list`). There is no staleness threshold or flagging — `status` reports
  the `updated_days_ago`/`reviewed_days_ago` facts and leaves judgment to the
  user.
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

- `Editor` trait:
  - `capture(&self, seed: &str) -> Result<(String, String)>` —
    implemented once as `RealEditor` (writes `seed` — the rendered template,
    with `{{title}}` empty and `{{cursor}}` marking the starting line — to a
    scratch file, opens `$EDITOR` on it via a `+<line>` argument when a cursor
    line is present, reads it back) and once per test as a fake. Returns
    `(content, suggested_filename)`.
  - `open(&self, path: &Path) -> Result<()>` — opens `$EDITOR` directly on
    an existing file at `path`, no scratch file, no seed, no filename
    inference. Used only by `cli::run_daily`'s reopen-existing-note path,
    where the content is already final and there's nothing to infer.
- `suggest_filename(content: &str) -> String` — pure. Calls
  [`gist`](#gist)'s `parser::first_heading_text` (frontmatter-skip then
  first-non-blank-heading, same semantics as `items::infer_title` above)
  and slugifies the result if found. If no such heading is found, falls
  back to the first non-blank line after the frontmatter (using `gist`'s
  `parser::frontmatter_body_offset` to locate where the body starts); if
  that's also absent (or the only candidate was a blank heading with no
  other content), falls back to a timestamp-based name. These two
  fallbacks are tick-specific — `gist` only surfaces the heading, not a
  filename — so `suggest_filename` still owns them. Internally delegates
  to a `SystemTime`-parameterized helper so the timestamp fallback is
  deterministic in tests.

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
- `run_new(ws: &Workspace, editor: &dyn Editor, ui: &mut dyn Ui, kind: Kind, filename: Option<&str>) -> Result<PathBuf>`
  — when `filename` is given, renders `ws.config.templates.for_kind(kind)`
  with `{{title}}` set to `filename` and `{{date}}` set to today, then calls
  `items::create(ws, kind.category(), filename, &rendered)` directly,
  non-interactively; `kind` is what makes `--project`/`--area`/`--resource`
  scaffold into the right place (and render the right template) instead of
  always `Inbox`/`note`. When `filename` is `None`, seeds `$EDITOR` with
  `ws.config.templates.for_kind(kind)` rendered with `{{title}}` empty
  (unknown yet) and `{{date}}` set to today, then prompts for the inferred
  name and calls `items::create(ws, kind.category(), ...)` — the same `kind`
  used by the non-interactive branch, so `--project`/`--area`/`--resource`
  scaffold into the right place (and seed the right template) from an
  editor capture too, not just `Inbox`/`note`. The confirm prompt's
  suggested default only appends `ws.config.default_extension` when
  `!kind.category().is_directory_style()` — `Project`/`Area` suggest a bare
  directory name (`Create "website-redesign"?`), while `Inbox`/`Resource`
  suggest a filename (`Create "website-improvement-ideas.md"?`). `main`
  maps `--project`/`--area`/`--resource` (mutually exclusive, via a `clap`
  `ArgGroup`) to `Kind`, defaulting to `Kind::Inbox` when none are given.
  `run_new` is never called with `Kind::Daily` — `main` dispatches that to
  `run_daily` instead (see below), since daily's create-or-reopen lifecycle
  doesn't fit `run_new`'s capture-or-named-file shape.
- `enum DailyOutcome { Created(PathBuf), Reopened(PathBuf) }` — lets `main`
  decide whether to print a `Created ...` line.
- `daily_note_exists(ws: &Workspace) -> bool` — true if today's daily note
  already exists. Lets `main` print `Opening $EDITOR...` *before* handing
  control to a blocking editor process, the same convention `run_new`'s
  no-filename path uses, without `run_daily` itself needing a callback.
- `run_daily(ws: &Workspace, editor: &dyn Editor) -> Result<DailyOutcome>`
  — if today's note (`items::item_path(ws, Category::Inbox, today)`)
  already exists, calls `editor.open` on it untouched and returns
  `Reopened`; otherwise renders `ws.config.templates.for_kind(Kind::Daily)`
  with `{{title}}`/`{{date}}` set to today, calls
  `items::create(ws, Category::Inbox, today, &rendered)`, and returns
  `Created`. No `Ui` parameter — there's no filename to confirm and no
  choice to prompt, just a create/reopen fork.
- `run_init(cwd: &Path, name: Option<&str>) -> Result<String>` — resolves
  the target (`cwd` or `cwd.join(name)`) and its display form (`.` or
  `./<name>`), calls `workspace::init` (which runs `check_collision`
  internally for both forms), and renders the outcome (full create /
  partial fill-in / already-complete) into the exact message `main`
  prints.
- `run_config_init(path: &Path, display: &str) -> Result<String>` — calls
  `config::init(path)` and, on success, returns `"Created {display}"`;
  `display` is the caller-computed human-readable form (`"./.tick.toml"` or
  `"~/.tick.toml"`), mirroring `run_init`'s `(target, display)` split.
  `config::ConfigError::AlreadyExists`'s message (using the same path
  passed to `config::init`) is what surfaces for config.md 004 — no
  separate error variant needed here. Backs `main`'s
  `Commands::Config { action: ConfigAction::Init { global } }` dispatch,
  which computes `path`/`display` from `-g` before calling this. See
  `docs/lld/006-config-init.md`.
- `run_config_edit(path: &Path, editor: &dyn Editor) -> Result<bool>` — calls
  `config::init(path)`, treating `Ok(())` as "created" and
  `Err(ConfigError::AlreadyExists { .. })` as "already there" (any other
  error propagates), then calls `editor.open(path)` and returns whether it
  had to create the file first. Matching `AlreadyExists` by variant (not a
  `path.exists()` pre-check) avoids a TOCTOU gap, the same reasoning
  `config::init` itself already encodes. Backs `main`'s
  `Commands::Config { action: ConfigAction::Edit { global } }` dispatch,
  which computes `path`/`display` from `-g` via the same helper
  (`config_target`) `ConfigAction::Init`'s arm uses, then prints
  `Created {display}` only when created and `Opening $EDITOR...`
  unconditionally. See `docs/lld/007-config-edit.md`. Bare `tk config`
  (provenance display) is still open.

## Notes

- `category` and `config` have no dependencies on anything else — they're the
  vocabulary and settings every other module shares.
- `workspace` depends only on `config` + `category`.
- `items` and `editor` depend only on `workspace` and the external `gist`
  crate — they don't know about each other or about `cli`.
- `review` composes `items` with a `Ui`, but doesn't know about `clap` or argv.
- `cli` is the only place that does terminal I/O; every other module returns
  data or `Result`s so it can be unit-tested directly.
- `NewCategory`, `ListCategory`, `config_target`, `CompletionShell`, and
  `render_completions` are `main`-only argv-parsing plumbing with no
  business logic of their own, so they get no `cli`/`design.md` writeup
  beyond this line — see `docs/lld/008-completions.md`'s `main` section for
  `CompletionShell`/`render_completions`.
