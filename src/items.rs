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
/// infer titles — see `count`. The per-item `projects`/`areas` breakdown
/// documented as `StatusReport`'s eventual shape in design.md lands with
/// `status.md` 002; this is the counts-only slice.
pub struct StatusReport {
    pub counts: [usize; 5],
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

/// Builds the per-category counts summary for `tk status` (`status.md`
/// 001). Counts-only slice of `StatusReport`'s eventual shape — see
/// `StatusReport`.
pub fn status(ws: &Workspace) -> Result<StatusReport, ItemsError> {
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

    Ok(StatusReport { counts })
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
}
