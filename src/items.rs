use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;

use thiserror::Error;

use crate::category::Category;
use crate::workspace::Workspace;

#[derive(Debug, Error)]
pub enum ItemsError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(
        "unwrapping a directory item into a flat file is not yet supported (moving \"{name}\" from {from} to {to})"
    )]
    UnwrapNotSupported {
        name: String,
        from: &'static str,
        to: &'static str,
    },
}

/// Computes the path `create` would write to, without touching the
/// filesystem — the directory-vs-flat-file branch, factored out so
/// callers can check existence (`cli::run_daily`) before deciding whether
/// to create or reopen.
pub fn item_path(ws: &Workspace, category: Category, name: &str) -> PathBuf {
    let category_dir = ws.category_dir(category);
    if category.is_directory_style() {
        category_dir
            .join(name)
            .join(format!("index.{}", ws.config.default_extension))
    } else {
        category_dir.join(with_extension(name, &ws.config.default_extension))
    }
}

/// Creates a flat file or a scaffolded `dir/index.md`, appending the
/// default extension to `name` if it has none, and writing `content`
/// into it. Returns the path created (the `index.md` path for
/// directory-style categories).
pub fn create(
    ws: &Workspace,
    category: Category,
    name: &str,
    content: &str,
) -> Result<PathBuf, ItemsError> {
    let path = item_path(ws, category, name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, content)?;
    Ok(path)
}

fn with_extension(name: &str, default_extension: &str) -> String {
    if name.contains('.') {
        name.to_string()
    } else {
        format!("{name}.{default_extension}")
    }
}

pub struct ListedItem {
    pub name: String,
    pub title: String,
    pub updated_days_ago: u64,
}

/// Thin wrapper over `gist::parser::first_heading_text`: skips a leading
/// YAML frontmatter block if present, then returns the first Markdown
/// heading line's text (any `#` level), or `None` if none is found.
pub fn infer_title(content: &str) -> Option<String> {
    gist::parser::first_heading_text(content)
}

/// Lists `category`'s items: for a directory-style category (`Project`/
/// `Area`), one row per subdirectory, sourced from its `index.md`; for a
/// flat category (`Resource`/`Inbox`), one row per file, name being the
/// file stem. Rows are sorted alphabetically by name. Returns `Ok(vec![])`
/// if `category`'s directory doesn't exist yet, rather than erroring — an
/// empty/not-yet-created category is a normal state, not a fault.
pub fn list(
    ws: &Workspace,
    category: Category,
    filter: Option<&str>,
) -> Result<Vec<ListedItem>, ItemsError> {
    list_at(ws, category, filter, SystemTime::now())
}

/// Scans `dir`'s immediate children as either directory-style entries
/// (subdirectories, source path `<entry>/index.<extension>`) or flat-file
/// entries (files, source path the file itself, name = file stem).
/// Returns `Ok(vec![])` if `dir` doesn't exist yet, rather than erroring.
fn scan_dir(
    dir: &std::path::Path,
    directory_style: bool,
    extension: &str,
) -> Result<Vec<(String, PathBuf)>, ItemsError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.into()),
    };

    let mut results = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        let (name, source_path) = if directory_style {
            if !path.is_dir() {
                continue;
            }
            let name = path
                .file_name()
                .expect("directory entry has a file name")
                .to_string_lossy()
                .into_owned();
            let index = path.join(format!("index.{extension}"));
            (name, index)
        } else {
            if !path.is_file() {
                continue;
            }
            let name = path
                .file_stem()
                .expect("file entry has a file name")
                .to_string_lossy()
                .into_owned();
            (name, path.clone())
        };

        results.push((name, source_path));
    }

    Ok(results)
}

/// Reads `source_path`'s content/mtime and builds the `ListedItem` for
/// `name` (title inferred, falling back to `name`; age relative to `now`).
fn build_listed_item(
    name: String,
    source_path: &std::path::Path,
    now: SystemTime,
) -> Result<ListedItem, ItemsError> {
    let content = fs::read_to_string(source_path)?;
    let title = infer_title(&content).unwrap_or_else(|| name.clone());
    let modified = fs::metadata(source_path)?.modified()?;
    let updated_days_ago = now.duration_since(modified).unwrap_or_default().as_secs() / 86400;

    Ok(ListedItem {
        name,
        title,
        updated_days_ago,
    })
}

/// True if `filter` is absent, or a case-insensitive substring of `item`'s
/// name or title.
fn matches_filter(item: &ListedItem, filter: Option<&str>) -> bool {
    match filter {
        None => true,
        Some(f) => {
            let f = f.to_lowercase();
            item.name.to_lowercase().contains(&f) || item.title.to_lowercase().contains(&f)
        }
    }
}

fn list_at(
    ws: &Workspace,
    category: Category,
    filter: Option<&str>,
    now: SystemTime,
) -> Result<Vec<ListedItem>, ItemsError> {
    let extension = &ws.config.default_extension;
    let mut items = Vec::new();

    if category == Category::Archive {
        let archive_dir = ws.category_dir(Category::Archive);
        for origin in Category::archivable() {
            let origin_dir = archive_dir.join(origin.archive_origin_name());
            for (name, source_path) in
                scan_dir(&origin_dir, origin.is_directory_style(), extension)?
            {
                let qualified = format!("{}/{name}", origin.archive_origin_name());
                let item = build_listed_item(qualified, &source_path, now)?;
                if matches_filter(&item, filter) {
                    items.push(item);
                }
            }
        }
    } else {
        let category_dir = ws.category_dir(category);
        for (name, source_path) in
            scan_dir(&category_dir, category.is_directory_style(), extension)?
        {
            let item = build_listed_item(name, &source_path, now)?;
            if matches_filter(&item, filter) {
                items.push(item);
            }
        }
    }

    items.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(items)
}

/// Per-category item counts, indexed by `Category as usize` (same
/// convention as `Config::category_dirs`). Doesn't read file content or
/// infer titles — see `count`.
pub struct StatusReport {
    pub counts: [usize; 5],
    pub projects: Vec<StatusItem>,
    pub areas: Vec<StatusItem>,
}

pub struct StatusItem {
    pub name: String,
    pub title: String,
    pub updated_days_ago: u64,
    pub reviewed_days_ago: Option<u64>,
}

/// Counts `category`'s items without reading file content — cheaper than
/// `list` for count-only categories (`Inbox`/`Resource`/`Archive`, which
/// can grow large and never need a per-item breakdown per status.md's
/// design). For `Archive`, sums counts across all four origin subfolders,
/// mirroring `list_at`'s `Archive` branch minus the content read.
fn count(ws: &Workspace, category: Category) -> Result<usize, ItemsError> {
    let extension = &ws.config.default_extension;

    if category == Category::Archive {
        let archive_dir = ws.category_dir(Category::Archive);
        let mut total = 0;
        for origin in Category::archivable() {
            let origin_dir = archive_dir.join(origin.archive_origin_name());
            total += scan_dir(&origin_dir, origin.is_directory_style(), extension)?.len();
        }
        Ok(total)
    } else {
        let category_dir = ws.category_dir(category);
        Ok(scan_dir(&category_dir, category.is_directory_style(), extension)?.len())
    }
}

/// Builds the per-category counts summary for `tk status`, plus the
/// per-item breakdown for `Project`/`Area` (`status.md` 001–003).
pub fn status(ws: &Workspace) -> Result<StatusReport, ItemsError> {
    status_at(ws, SystemTime::now(), chrono::Local::now().date_naive())
}

fn status_at(
    ws: &Workspace,
    now: SystemTime,
    today: chrono::NaiveDate,
) -> Result<StatusReport, ItemsError> {
    let categories = [
        Category::Inbox,
        Category::Project,
        Category::Area,
        Category::Resource,
        Category::Archive,
    ];

    let mut counts = [0; 5];
    for category in categories {
        counts[category as usize] = count(ws, category)?;
    }

    let projects = status_items_for(ws, Category::Project, now, today)?;
    let areas = status_items_for(ws, Category::Area, now, today)?;

    Ok(StatusReport {
        counts,
        projects,
        areas,
    })
}

/// Sorted alphabetically by name; same mtime-sourced `updated_days_ago`
/// and title inference as `status`'s per-item rows. Reuses
/// `StatusItem`/`status_items_for` rather than a second directory-scan
/// implementation — review's prompt only needs `name`/`updated_days_ago`
/// today, but `title`/`reviewed_days_ago` come along for free and story
/// 003's `[k]eep` path will want the same fresh-content read regardless.
pub fn review_items(ws: &Workspace, category: Category) -> Result<Vec<StatusItem>, ItemsError> {
    status_items_for(
        ws,
        category,
        SystemTime::now(),
        chrono::Local::now().date_naive(),
    )
}

/// Builds the sorted per-item breakdown for a directory-style category
/// (`Project`/`Area` only — the only categories `status` shows rows for).
fn status_items_for(
    ws: &Workspace,
    category: Category,
    now: SystemTime,
    today: chrono::NaiveDate,
) -> Result<Vec<StatusItem>, ItemsError> {
    let extension = &ws.config.default_extension;
    let category_dir = ws.category_dir(category);
    let mut items = Vec::new();
    for (name, source_path) in scan_dir(&category_dir, true, extension)? {
        items.push(build_status_item(name, &source_path, now, today)?);
    }
    items.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(items)
}

/// Mirrors `build_listed_item` (same title inference, same mtime-based
/// `updated_days_ago`) plus `reviewed_days_ago`.
fn build_status_item(
    name: String,
    source_path: &std::path::Path,
    now: SystemTime,
    today: chrono::NaiveDate,
) -> Result<StatusItem, ItemsError> {
    let content = fs::read_to_string(source_path)?;
    let title = infer_title(&content).unwrap_or_else(|| name.clone());
    let modified = fs::metadata(source_path)?.modified()?;
    let updated_days_ago = now.duration_since(modified).unwrap_or_default().as_secs() / 86400;
    let reviewed_days_ago = parse_last_reviewed(source_path, &content, today);

    Ok(StatusItem {
        name,
        title,
        updated_days_ago,
        reviewed_days_ago,
    })
}

/// Parses the `last_reviewed` frontmatter field from `content`, if
/// present, and returns its age in days relative to `today`. Pure — no
/// I/O — so `build_status_item` (content already in hand) and
/// `read_last_reviewed` (fresh read from disk) share it without a second
/// file read in the `status` path.
fn parse_last_reviewed(
    path: &std::path::Path,
    content: &str,
    today: chrono::NaiveDate,
) -> Option<u64> {
    let note = gist::parser::parse(path, content);
    let value = note
        .frontmatter?
        .fields
        .iter()
        .find(|f| f.key == "last_reviewed")?
        .value
        .clone()?;
    let date = chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d").ok()?;
    Some((today - date).num_days().max(0) as u64)
}

/// Fresh-read entry point for `last_reviewed`, independent of `status`
/// (which reuses `parse_last_reviewed` on content it already read).
pub fn read_last_reviewed(item: &std::path::Path) -> Result<Option<u64>, ItemsError> {
    let content = fs::read_to_string(item)?;
    Ok(parse_last_reviewed(
        item,
        &content,
        chrono::Local::now().date_naive(),
    ))
}

/// Sets `index.md`'s `last_reviewed` frontmatter field to today's date,
/// adding the field if absent and preserving every other frontmatter key
/// and the body unchanged (status.md 004, scenarios 1-2). Byte-splices
/// rather than re-serializing the whole file, so untouched keys/formatting
/// (quoting, key order, comments `gist` doesn't model) survive exactly:
///
/// - Field present with a scalar value: replaces just
///   `FrontmatterField::value_range`'s bytes with today's date.
/// - Field absent, frontmatter block present: inserts a new
///   `last_reviewed: <date>` line immediately before the closing `---`,
///   found via `gist::parser::frontmatter_body_offset`.
/// - No frontmatter block at all: prepends a fresh
///   `---\nlast_reviewed: <date>\n---\n` block. Not exercised by any
///   acceptance scenario (every template that reaches `review` already
///   has a block) — included so a hand-edited `index.md` with a stripped
///   frontmatter block doesn't corrupt on `[k]eep`, at negligible cost.
///
/// A field whose existing value isn't a plain scalar (block scalar `|`/`>`,
/// inline list `[...]`) has `value_range: None`; that falls through to the
/// insert-new-line path, which would produce a duplicate `last_reviewed:`
/// key. Acceptable only because no template ever writes `last_reviewed` as
/// anything but a plain date scalar.
pub fn write_last_reviewed(item: &std::path::Path) -> Result<(), ItemsError> {
    let content = fs::read_to_string(item)?;
    let today = chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let note = gist::parser::parse(item, &content);

    let new_content = match note.frontmatter {
        Some(fm) => {
            match fm
                .fields
                .iter()
                .find(|f| f.key == "last_reviewed" && f.value_range.is_some())
            {
                Some(field) => {
                    let range = field.value_range.clone().unwrap();
                    format!(
                        "{}{}{}",
                        &content[..range.start],
                        today,
                        &content[range.end..]
                    )
                }
                None => {
                    let insert_at = gist::parser::frontmatter_body_offset(&content) - 5;
                    format!(
                        "{}\nlast_reviewed: {today}{}",
                        &content[..insert_at],
                        &content[insert_at..]
                    )
                }
            }
        }
        None => format!("---\nlast_reviewed: {today}\n---\n{content}"),
    };

    fs::write(item, new_content)?;
    Ok(())
}

/// Searches `Category::archivable()` (`Inbox`, `Project`, `Area`,
/// `Resource`, in that order) for an item named `name`, returning the
/// category it was found in and its actual on-disk root path — the
/// directory itself for a directory-style category (not its `index.md`),
/// or the file itself (with whatever extension it actually has, not
/// necessarily `ws.config.default_extension`) for a flat category.
/// `Ok(None)` if no category has a match; never searches `Archive`.
pub fn locate(ws: &Workspace, name: &str) -> Result<Option<(Category, PathBuf)>, ItemsError> {
    for category in Category::archivable() {
        let dir = ws.category_dir(category);
        if category.is_directory_style() {
            let candidate = dir.join(name);
            if candidate.is_dir() {
                return Ok(Some((category, candidate)));
            }
        } else {
            for (entry_name, path) in scan_dir(&dir, false, &ws.config.default_extension)? {
                if entry_name == name {
                    return Ok(Some((category, path)));
                }
            }
        }
    }
    Ok(None)
}

/// Moves `source_path` (an item already located by `locate`, in category
/// `source`, named `name`) to `target`. Rejects unwrapping a directory item
/// (`Project`/`Area`) into a flat-file category (`Inbox`/`Resource`) up
/// front, before touching the filesystem (move.md 002) — archiving a
/// directory is not unwrapping, so `target == Archive` is exempt. Otherwise
/// three shapes, decided purely from `source`/`target`'s
/// `is_directory_style`/`Archive`-ness — no scenario in move.md 001 needs
/// anything finer-grained than this:
///
/// - `target` is directory-style and `source` isn't: **wrap** —
///   `<target_dir>/<name>/index.<default_extension>`.
/// - `target` is `Archive`: **archive** — `<archive_dir>/
///   <source.archive_origin_name()>/<basename>`, `basename` being `name`
///   for a directory source or `source_path`'s actual filename for a flat
///   one. Applies uniformly whether `source` is directory-style or flat.
/// - Anything else (matching shapes, e.g. `Project`<->`Area`, or
///   `Inbox`<->`Resource`): **relocate as-is** — same `basename` rule as
///   the archive case, just under `<target_dir>` with no origin
///   subfolder.
///
/// `basename` reuses `source_path`'s real filename (not a
/// recomputed-from-`name` one) for flat sources so an item's original
/// extension survives a relocate untouched, matching `locate`'s own
/// extension-agnostic lookup.
pub fn mv(
    ws: &Workspace,
    source: Category,
    source_path: &std::path::Path,
    name: &str,
    target: Category,
) -> Result<PathBuf, ItemsError> {
    if source.is_directory_style() && !target.is_directory_style() && target != Category::Archive {
        return Err(ItemsError::UnwrapNotSupported {
            name: name.to_string(),
            from: source.display_name(),
            to: target.display_name(),
        });
    }

    let basename = |source_path: &std::path::Path| -> PathBuf {
        if source.is_directory_style() {
            PathBuf::from(name)
        } else {
            PathBuf::from(
                source_path
                    .file_name()
                    .expect("located item has a file name"),
            )
        }
    };

    let dest = if target.is_directory_style() && !source.is_directory_style() {
        ws.category_dir(target)
            .join(name)
            .join(format!("index.{}", ws.config.default_extension))
    } else if target == Category::Archive {
        ws.category_dir(Category::Archive)
            .join(source.archive_origin_name())
            .join(basename(source_path))
    } else {
        ws.category_dir(target).join(basename(source_path))
    };

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(source_path, &dest)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use tempfile::tempdir;

    fn workspace(root: &std::path::Path) -> Workspace {
        Workspace {
            root: root.to_path_buf(),
            config: Config::default(),
        }
    }

    #[test]
    fn creates_inbox_file_with_default_extension() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let path = create(&ws, Category::Inbox, "my-note", "hello").unwrap();

        assert_eq!(path, dir.path().join("0-Inbox/my-note.md"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn does_not_double_append_extension_when_already_present() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let path = create(&ws, Category::Inbox, "my-note.md", "hello").unwrap();

        assert_eq!(path, dir.path().join("0-Inbox/my-note.md"));
    }

    #[test]
    fn item_path_for_flat_category() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let path = item_path(&ws, Category::Inbox, "2026-07-04");

        assert_eq!(path, dir.path().join("0-Inbox/2026-07-04.md"));
    }

    #[test]
    fn item_path_for_directory_style_category() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let path = item_path(&ws, Category::Project, "foo");

        assert_eq!(path, dir.path().join("1-Projects/foo/index.md"));
    }

    #[test]
    fn creates_scaffolded_project_directory_with_index() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let path = create(&ws, Category::Project, "website-redesign", "").unwrap();

        assert_eq!(
            path,
            dir.path().join("1-Projects/website-redesign/index.md")
        );
        assert!(path.exists());
    }

    #[test]
    fn infer_title_none_when_no_heading() {
        assert_eq!(infer_title("plain text\nno heading"), None);
    }

    #[test]
    fn infer_title_none_when_frontmatter_has_no_heading_after_it() {
        assert_eq!(infer_title("---\nk: v\n---\nplain text"), None);
    }

    fn set_mtime(path: &std::path::Path, days_ago: u64, now: SystemTime) {
        let modified = now - std::time::Duration::from_secs(days_ago * 86400);
        let file = fs::File::open(path).unwrap();
        file.set_modified(modified).unwrap();
    }

    fn fixed_now() -> SystemTime {
        SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_782_916_245)
    }

    #[test]
    fn list_at_directory_style_category_returns_name_title_and_age() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let now = fixed_now();

        let path1 = create(
            &ws,
            Category::Project,
            "website-redesign",
            "# Website Redesign\n",
        )
        .unwrap();
        set_mtime(&path1, 2, now);
        let path2 = create(&ws, Category::Project, "my-project", "# My Project\n").unwrap();
        set_mtime(&path2, 21, now);

        let items = list_at(&ws, Category::Project, None, now).unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "my-project");
        assert_eq!(items[0].title, "My Project");
        assert_eq!(items[0].updated_days_ago, 21);
        assert_eq!(items[1].name, "website-redesign");
        assert_eq!(items[1].title, "Website Redesign");
        assert_eq!(items[1].updated_days_ago, 2);
    }

    #[test]
    fn list_at_flat_category_uses_file_stem_as_name() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let now = fixed_now();

        let path = create(&ws, Category::Resource, "api-notes", "# API Design Notes\n").unwrap();
        set_mtime(&path, 5, now);

        let items = list_at(&ws, Category::Resource, None, now).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "api-notes");
        assert_eq!(items[0].title, "API Design Notes");
        assert_eq!(items[0].updated_days_ago, 5);
    }

    #[test]
    fn list_at_sorts_alphabetically_by_name() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let now = fixed_now();

        create(&ws, Category::Project, "website-redesign", "").unwrap();
        create(&ws, Category::Project, "my-project", "").unwrap();

        let items = list_at(&ws, Category::Project, None, now).unwrap();

        assert_eq!(items[0].name, "my-project");
        assert_eq!(items[1].name, "website-redesign");
    }

    #[test]
    fn list_at_missing_category_directory_returns_empty() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let items = list_at(&ws, Category::Area, None, fixed_now()).unwrap();

        assert!(items.is_empty());
    }

    #[test]
    fn list_at_archive_qualifies_name_with_origin_category() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let now = fixed_now();

        let project_index = dir.path().join("4-Archive/Projects/old-project/index.md");
        fs::create_dir_all(project_index.parent().unwrap()).unwrap();
        fs::write(&project_index, "# Old Project\n").unwrap();
        set_mtime(&project_index, 120, now);

        let resource_path = dir.path().join("4-Archive/Resources/api-notes-v1.md");
        fs::create_dir_all(resource_path.parent().unwrap()).unwrap();
        fs::write(&resource_path, "# API Notes v1\n").unwrap();
        set_mtime(&resource_path, 180, now);

        let items = list_at(&ws, Category::Archive, None, now).unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "Projects/old-project");
        assert_eq!(items[0].title, "Old Project");
        assert_eq!(items[0].updated_days_ago, 120);
        assert_eq!(items[1].name, "Resources/api-notes-v1");
        assert_eq!(items[1].title, "API Notes v1");
        assert_eq!(items[1].updated_days_ago, 180);
    }

    #[test]
    fn list_at_archive_missing_dir_returns_empty() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let items = list_at(&ws, Category::Archive, None, fixed_now()).unwrap();

        assert!(items.is_empty());
    }

    #[test]
    fn list_at_archive_missing_origin_subfolder_is_skipped() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let now = fixed_now();

        let project_index = dir.path().join("4-Archive/Projects/old-project/index.md");
        fs::create_dir_all(project_index.parent().unwrap()).unwrap();
        fs::write(&project_index, "# Old Project\n").unwrap();
        set_mtime(&project_index, 10, now);

        let items = list_at(&ws, Category::Archive, None, now).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Projects/old-project");
    }

    #[test]
    fn list_at_falls_back_to_name_when_no_heading() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        create(&ws, Category::Inbox, "quick-thought", "just plain text").unwrap();

        let items = list_at(&ws, Category::Inbox, None, fixed_now()).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "quick-thought");
        assert_eq!(items[0].title, "quick-thought");
    }

    #[test]
    fn list_at_filter_matches_substring_of_name() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let now = fixed_now();

        create(&ws, Category::Project, "website-redesign", "").unwrap();
        create(&ws, Category::Project, "my-project", "").unwrap();

        let items = list_at(&ws, Category::Project, Some("web"), now).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "website-redesign");
    }

    #[test]
    fn list_at_filter_matches_substring_of_title() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let now = fixed_now();

        create(
            &ws,
            Category::Project,
            "q3-initiative",
            "# Website Redesign Phase 2\n",
        )
        .unwrap();

        let items = list_at(&ws, Category::Project, Some("redesign"), now).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "q3-initiative");
    }

    #[test]
    fn list_at_filter_is_case_insensitive() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let now = fixed_now();

        create(&ws, Category::Project, "website-redesign", "").unwrap();

        let items = list_at(&ws, Category::Project, Some("WEB"), now).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "website-redesign");
    }

    #[test]
    fn list_at_filter_matching_nothing_returns_empty() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let now = fixed_now();

        create(&ws, Category::Project, "website-redesign", "").unwrap();

        let items = list_at(&ws, Category::Project, Some("nonexistent"), now).unwrap();

        assert!(items.is_empty());
    }

    #[test]
    fn count_counts_flat_file_categories_without_reading_content() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        create(&ws, Category::Inbox, "a", "").unwrap();
        create(&ws, Category::Inbox, "b", "").unwrap();

        create(&ws, Category::Resource, "r1", "").unwrap();
        create(&ws, Category::Resource, "r2", "").unwrap();
        create(&ws, Category::Resource, "r3", "").unwrap();
        create(&ws, Category::Resource, "r4", "").unwrap();
        create(&ws, Category::Resource, "r5", "no heading here").unwrap();

        assert_eq!(count(&ws, Category::Inbox).unwrap(), 2);
        assert_eq!(count(&ws, Category::Resource).unwrap(), 5);
    }

    #[test]
    fn count_counts_directory_style_categories() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        create(&ws, Category::Project, "p1", "").unwrap();
        create(&ws, Category::Project, "p2", "").unwrap();
        create(&ws, Category::Project, "p3", "").unwrap();

        create(&ws, Category::Area, "a1", "").unwrap();
        create(&ws, Category::Area, "a2", "").unwrap();

        assert_eq!(count(&ws, Category::Project).unwrap(), 3);
        assert_eq!(count(&ws, Category::Area).unwrap(), 2);
    }

    #[test]
    fn count_sums_across_all_origin_subfolders_for_archive() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let archive_dir = dir.path().join("4-Archive");

        for i in 0..3 {
            let path = archive_dir.join("Inbox").join(format!("i{i}.md"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "").unwrap();
        }
        for i in 0..3 {
            let path = archive_dir
                .join("Projects")
                .join(format!("p{i}"))
                .join("index.md");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "").unwrap();
        }
        for i in 0..3 {
            let path = archive_dir
                .join("Areas")
                .join(format!("a{i}"))
                .join("index.md");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "").unwrap();
        }
        for i in 0..3 {
            let path = archive_dir.join("Resources").join(format!("r{i}.md"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "").unwrap();
        }

        assert_eq!(count(&ws, Category::Archive).unwrap(), 12);
    }

    #[test]
    fn count_returns_zero_for_missing_directory() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        assert_eq!(count(&ws, Category::Inbox).unwrap(), 0);
        assert_eq!(count(&ws, Category::Project).unwrap(), 0);
        assert_eq!(count(&ws, Category::Area).unwrap(), 0);
        assert_eq!(count(&ws, Category::Resource).unwrap(), 0);
        assert_eq!(count(&ws, Category::Archive).unwrap(), 0);
    }

    #[test]
    fn status_returns_counts_indexed_by_category() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let archive_dir = dir.path().join("4-Archive");

        create(&ws, Category::Inbox, "a", "").unwrap();
        create(&ws, Category::Inbox, "b", "").unwrap();

        create(&ws, Category::Project, "p1", "").unwrap();
        create(&ws, Category::Project, "p2", "").unwrap();
        create(&ws, Category::Project, "p3", "").unwrap();

        create(&ws, Category::Area, "a1", "").unwrap();
        create(&ws, Category::Area, "a2", "").unwrap();

        create(&ws, Category::Resource, "r1", "").unwrap();
        create(&ws, Category::Resource, "r2", "").unwrap();
        create(&ws, Category::Resource, "r3", "").unwrap();
        create(&ws, Category::Resource, "r4", "").unwrap();
        create(&ws, Category::Resource, "r5", "").unwrap();

        for i in 0..3 {
            let path = archive_dir.join("Inbox").join(format!("i{i}.md"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "").unwrap();
        }
        for i in 0..3 {
            let path = archive_dir
                .join("Projects")
                .join(format!("p{i}"))
                .join("index.md");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "").unwrap();
        }
        for i in 0..3 {
            let path = archive_dir
                .join("Areas")
                .join(format!("a{i}"))
                .join("index.md");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "").unwrap();
        }
        for i in 0..3 {
            let path = archive_dir.join("Resources").join(format!("r{i}.md"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "").unwrap();
        }

        let report = status(&ws).unwrap();

        assert_eq!(report.counts, [2, 3, 2, 5, 12]);
    }

    #[test]
    fn status_on_empty_workspace_returns_all_zero_counts() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let report = status(&ws).unwrap();

        assert_eq!(report.counts, [0, 0, 0, 0, 0]);
    }

    #[test]
    fn status_at_projects_sorted_alphabetically_with_updated_days_ago() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let now = fixed_now();

        let path1 = create(
            &ws,
            Category::Project,
            "website-redesign",
            "# Website Redesign\n",
        )
        .unwrap();
        set_mtime(&path1, 2, now);
        let path2 = create(&ws, Category::Project, "my-project", "# My Project\n").unwrap();
        set_mtime(&path2, 21, now);

        let report = status_at(
            &ws,
            now,
            chrono::NaiveDate::from_ymd_opt(2026, 7, 8).unwrap(),
        )
        .unwrap();

        assert_eq!(report.projects.len(), 2);
        assert_eq!(report.projects[0].name, "my-project");
        assert_eq!(report.projects[0].updated_days_ago, 21);
        assert_eq!(report.projects[1].name, "website-redesign");
        assert_eq!(report.projects[1].updated_days_ago, 2);
    }

    #[test]
    fn status_at_areas_listed_the_same_way() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let now = fixed_now();

        create(&ws, Category::Area, "health", "# Health\n").unwrap();

        let report = status_at(
            &ws,
            now,
            chrono::NaiveDate::from_ymd_opt(2026, 7, 8).unwrap(),
        )
        .unwrap();

        assert_eq!(report.areas.len(), 1);
        assert_eq!(report.areas[0].name, "health");
        assert_eq!(report.areas[0].updated_days_ago, 0);
    }

    #[test]
    fn status_at_project_title_falls_back_to_name_when_no_heading() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let now = fixed_now();

        create(&ws, Category::Project, "quick-idea", "no heading here").unwrap();

        let report = status_at(
            &ws,
            now,
            chrono::NaiveDate::from_ymd_opt(2026, 7, 8).unwrap(),
        )
        .unwrap();

        assert_eq!(report.projects[0].title, "quick-idea");
    }

    #[test]
    fn parse_last_reviewed_returns_age_in_days_when_present() {
        let content = "---\nlast_reviewed: 2026-07-05\n---\n# Title\n";
        let today = chrono::NaiveDate::from_ymd_opt(2026, 7, 8).unwrap();

        let result = parse_last_reviewed(std::path::Path::new("index.md"), content, today);

        assert_eq!(result, Some(3));
    }

    #[test]
    fn parse_last_reviewed_returns_none_when_field_absent() {
        let content = "---\ntitle: Something\n---\n# Title\n";
        let today = chrono::NaiveDate::from_ymd_opt(2026, 7, 8).unwrap();

        let result = parse_last_reviewed(std::path::Path::new("index.md"), content, today);

        assert_eq!(result, None);
    }

    #[test]
    fn parse_last_reviewed_returns_none_when_no_frontmatter() {
        let content = "# Title\nplain content\n";
        let today = chrono::NaiveDate::from_ymd_opt(2026, 7, 8).unwrap();

        let result = parse_last_reviewed(std::path::Path::new("index.md"), content, today);

        assert_eq!(result, None);
    }

    #[test]
    fn parse_last_reviewed_returns_none_on_malformed_date() {
        let content = "---\nlast_reviewed: not-a-date\n---\n# Title\n";
        let today = chrono::NaiveDate::from_ymd_opt(2026, 7, 8).unwrap();

        let result = parse_last_reviewed(std::path::Path::new("index.md"), content, today);

        assert_eq!(result, None);
    }

    fn backdate(path: &std::path::Path, days_ago: u64) {
        let modified = SystemTime::now() - std::time::Duration::from_secs(days_ago * 86400);
        let file = fs::File::open(path).unwrap();
        file.set_modified(modified).unwrap();
    }

    #[test]
    fn review_items_returns_project_rows_sorted_alphabetically_with_updated_days_ago() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let path1 = create(
            &ws,
            Category::Project,
            "website-redesign",
            "# Website Redesign\n",
        )
        .unwrap();
        backdate(&path1, 2);
        let path2 = create(&ws, Category::Project, "my-project", "# My Project\n").unwrap();
        backdate(&path2, 21);

        let items = review_items(&ws, Category::Project).unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "my-project");
        assert_eq!(items[0].updated_days_ago, 21);
        assert_eq!(items[1].name, "website-redesign");
        assert_eq!(items[1].updated_days_ago, 2);
    }

    #[test]
    fn review_items_returns_the_same_shape_for_area() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let path = create(&ws, Category::Area, "finances", "# Finances\n").unwrap();
        backdate(&path, 4);

        let items = review_items(&ws, Category::Area).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "finances");
        assert_eq!(items[0].updated_days_ago, 4);
    }

    #[test]
    fn read_last_reviewed_reads_fresh_from_disk() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let today = chrono::Local::now().date_naive();
        let last_reviewed = today - chrono::Duration::days(4);
        let content = format!(
            "---\nlast_reviewed: {}\n---\n# Title\n",
            last_reviewed.format("%Y-%m-%d")
        );

        let path = create(&ws, Category::Project, "my-project", &content).unwrap();

        let result = read_last_reviewed(&path).unwrap();

        assert_eq!(result, Some(4));
    }

    #[test]
    fn locate_finds_flat_item_in_inbox_by_name_any_extension() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let path = create(&ws, Category::Inbox, "my-file", "hello").unwrap();

        let result = locate(&ws, "my-file").unwrap();

        assert_eq!(result, Some((Category::Inbox, path)));
    }

    #[test]
    fn locate_finds_directory_item_in_project_by_name() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        create(&ws, Category::Project, "website-redesign", "").unwrap();

        let result = locate(&ws, "website-redesign").unwrap();

        assert_eq!(
            result,
            Some((
                Category::Project,
                dir.path().join("1-Projects/website-redesign")
            ))
        );
    }

    #[test]
    fn locate_returns_none_when_no_category_has_a_match() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let result = locate(&ws, "nonexistent").unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn locate_searches_inbox_before_project_area_resource() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        create(&ws, Category::Resource, "my-item", "").unwrap();
        create(&ws, Category::Inbox, "my-item", "").unwrap();

        let result = locate(&ws, "my-item").unwrap();

        assert_eq!(result.unwrap().0, Category::Inbox);
    }

    #[test]
    fn mv_wraps_flat_file_into_project() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let source_path = create(&ws, Category::Inbox, "my-file", "hello").unwrap();

        let dest = mv(
            &ws,
            Category::Inbox,
            &source_path,
            "my-file",
            Category::Project,
        )
        .unwrap();

        assert_eq!(dest, dir.path().join("1-Projects/my-file/index.md"));
        assert_eq!(fs::read_to_string(&dest).unwrap(), "hello");
    }

    #[test]
    fn mv_wraps_flat_file_into_area() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let source_path = create(&ws, Category::Inbox, "my-file", "hello").unwrap();

        let dest = mv(
            &ws,
            Category::Inbox,
            &source_path,
            "my-file",
            Category::Area,
        )
        .unwrap();

        assert_eq!(dest, dir.path().join("2-Areas/my-file/index.md"));
        assert_eq!(fs::read_to_string(&dest).unwrap(), "hello");
    }

    #[test]
    fn mv_relocates_flat_file_without_wrapping_extension_preserved() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let source_path = create(&ws, Category::Resource, "my-file", "hello").unwrap();

        let dest = mv(
            &ws,
            Category::Resource,
            &source_path,
            "my-file",
            Category::Inbox,
        )
        .unwrap();

        assert_eq!(dest, dir.path().join("0-Inbox/my-file.md"));
        assert!(!dest.parent().unwrap().join("my-file").is_dir());
    }

    #[test]
    fn mv_relocates_directory_between_project_and_area_as_is() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let source_path = create(
            &ws,
            Category::Project,
            "website-redesign",
            "# Website Redesign\n",
        )
        .unwrap();
        let extra_file = source_path.parent().unwrap().join("notes.md");
        fs::write(&extra_file, "extra").unwrap();
        let source_dir = source_path.parent().unwrap().to_path_buf();

        let dest = mv(
            &ws,
            Category::Project,
            &source_dir,
            "website-redesign",
            Category::Area,
        )
        .unwrap();

        assert_eq!(dest, dir.path().join("2-Areas/website-redesign"));
        assert_eq!(
            fs::read_to_string(dest.join("index.md")).unwrap(),
            "# Website Redesign\n"
        );
        assert_eq!(fs::read_to_string(dest.join("notes.md")).unwrap(), "extra");
    }

    #[test]
    fn mv_archives_directory_under_origin_subfolder() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let source_path = create(&ws, Category::Project, "website-redesign", "").unwrap();
        let source_dir = source_path.parent().unwrap().to_path_buf();

        let dest = mv(
            &ws,
            Category::Project,
            &source_dir,
            "website-redesign",
            Category::Archive,
        )
        .unwrap();

        assert_eq!(dest, dir.path().join("4-Archive/Projects/website-redesign"));
    }

    #[test]
    fn mv_archives_flat_file_under_origin_subfolder() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let source_path = create(&ws, Category::Resource, "my-file", "hello").unwrap();

        let dest = mv(
            &ws,
            Category::Resource,
            &source_path,
            "my-file",
            Category::Archive,
        )
        .unwrap();

        assert_eq!(dest, dir.path().join("4-Archive/Resources/my-file.md"));
        assert_eq!(fs::read_to_string(&dest).unwrap(), "hello");
    }

    #[test]
    fn mv_rejects_unwrapping_project_into_inbox() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let source_path = create(&ws, Category::Project, "website-redesign", "").unwrap();
        let source_dir = source_path.parent().unwrap().to_path_buf();

        let err = mv(
            &ws,
            Category::Project,
            &source_dir,
            "website-redesign",
            Category::Inbox,
        )
        .unwrap_err();

        assert!(matches!(err, ItemsError::UnwrapNotSupported { .. }));
        assert!(source_dir.is_dir());
    }

    #[test]
    fn mv_rejects_unwrapping_project_into_resource() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let source_path = create(&ws, Category::Project, "website-redesign", "").unwrap();
        let source_dir = source_path.parent().unwrap().to_path_buf();

        let err = mv(
            &ws,
            Category::Project,
            &source_dir,
            "website-redesign",
            Category::Resource,
        )
        .unwrap_err();

        assert!(matches!(err, ItemsError::UnwrapNotSupported { .. }));
        assert!(source_dir.is_dir());
    }

    #[test]
    fn mv_rejects_unwrapping_area_into_inbox() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let source_path = create(&ws, Category::Area, "my-area", "").unwrap();
        let source_dir = source_path.parent().unwrap().to_path_buf();

        let err = mv(&ws, Category::Area, &source_dir, "my-area", Category::Inbox).unwrap_err();

        assert!(matches!(err, ItemsError::UnwrapNotSupported { .. }));
        assert!(source_dir.is_dir());
    }

    #[test]
    fn mv_rejects_unwrapping_area_into_resource() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let source_path = create(&ws, Category::Area, "my-area", "").unwrap();
        let source_dir = source_path.parent().unwrap().to_path_buf();

        let err = mv(
            &ws,
            Category::Area,
            &source_dir,
            "my-area",
            Category::Resource,
        )
        .unwrap_err();

        assert!(matches!(err, ItemsError::UnwrapNotSupported { .. }));
        assert!(source_dir.is_dir());
    }

    #[test]
    fn mv_allows_archiving_a_directory_item() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let source_path = create(&ws, Category::Project, "website-redesign", "").unwrap();
        let source_dir = source_path.parent().unwrap().to_path_buf();

        let dest = mv(
            &ws,
            Category::Project,
            &source_dir,
            "website-redesign",
            Category::Archive,
        )
        .unwrap();

        assert_eq!(dest, dir.path().join("4-Archive/Projects/website-redesign"));
    }

    #[test]
    fn status_at_wires_reviewed_days_ago_into_status_item() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let now = fixed_now();
        let today = chrono::NaiveDate::from_ymd_opt(2026, 7, 8).unwrap();

        create(
            &ws,
            Category::Project,
            "website-redesign",
            "---\nlast_reviewed: 2026-07-05\n---\n# Website Redesign\n",
        )
        .unwrap();
        create(&ws, Category::Project, "my-project", "# My Project\n").unwrap();

        let report = status_at(&ws, now, today).unwrap();

        assert_eq!(report.projects[0].name, "my-project");
        assert_eq!(report.projects[0].reviewed_days_ago, None);
        assert_eq!(report.projects[1].name, "website-redesign");
        assert_eq!(report.projects[1].reviewed_days_ago, Some(3));
    }

    #[test]
    fn write_last_reviewed_overwrites_existing_scalar_leaving_other_keys_and_body_untouched() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let content = "---\nlast_reviewed: 2020-01-01\ntags: [x]\n---\n# Title\nbody text\n";
        let path = create(&ws, Category::Project, "my-project", content).unwrap();
        let today = chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();

        write_last_reviewed(&path).unwrap();

        let new_content = fs::read_to_string(&path).unwrap();
        assert!(new_content.contains(&format!("last_reviewed: {today}")));
        assert!(new_content.contains("tags: [x]"));
        assert!(new_content.contains("# Title\nbody text\n"));
        assert!(!new_content.contains("2020-01-01"));
    }

    #[test]
    fn write_last_reviewed_adds_absent_field_leaving_existing_frontmatter_and_body_untouched() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let content = "---\nlast_updated: 2020-01-01\n---\n# Title\nbody text\n";
        let path = create(&ws, Category::Project, "my-project", content).unwrap();
        let today = chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();

        write_last_reviewed(&path).unwrap();

        let new_content = fs::read_to_string(&path).unwrap();
        assert!(new_content.contains(&format!("last_reviewed: {today}")));
        assert!(new_content.contains("last_updated: 2020-01-01"));
        assert!(new_content.contains("# Title\nbody text\n"));
    }

    #[test]
    fn write_last_reviewed_prepends_block_when_none_exists() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let content = "# Title\nbody text\n";
        let path = create(&ws, Category::Project, "my-project", content).unwrap();
        let today = chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();

        write_last_reviewed(&path).unwrap();

        let new_content = fs::read_to_string(&path).unwrap();
        assert_eq!(
            new_content,
            format!("---\nlast_reviewed: {today}\n---\n# Title\nbody text\n")
        );
    }
}
