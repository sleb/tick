use crate::category::Category;
use crate::cli::{self, Ui};
use crate::items;
use crate::workspace::Workspace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Keep,
    Archive,
    Skip,
}

impl Decision {
    fn from_choice(choice: char) -> Self {
        match choice {
            'k' => Decision::Keep,
            'a' => Decision::Archive,
            's' => Decision::Skip,
            _ => unreachable!("Ui::choose only returns a char from the options it was given"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReviewError {
    #[error(transparent)]
    Items(#[from] items::ItemsError),
    #[error(transparent)]
    Ui(#[from] cli::UiError),
    #[error("\"{name}\" isn't a project or area")]
    NotReviewable { name: String },
}

/// Walks every `Project`, then every `Area`, alphabetically within each
/// group (review.md 001 scenario 1), prompting once per item via
/// `ui.choose`. If both groups are empty, reports via `ui.info` and
/// returns immediately without prompting (scenario 4); otherwise ends
/// silently after the last item (scenario 5) — no summary line. The
/// `char` `ui.choose` returns is currently discarded: interpreting
/// `[a]rchive`/`[k]eep`/`[s]kip` is story 002/003's job, added as match
/// arms on this same call site by those LLDs, not a new loop shape.
pub fn run(ws: &Workspace, ui: &mut dyn Ui) -> Result<(), ReviewError> {
    let projects = items::review_items(ws, Category::Project)?;
    let areas = items::review_items(ws, Category::Area)?;

    if projects.is_empty() && areas.is_empty() {
        ui.info("Nothing to review.");
        return Ok(());
    }

    for item in &projects {
        prompt_one(ws, ui, Category::Project, "Project", item)?;
    }
    for item in &areas {
        prompt_one(ws, ui, Category::Area, "Area", item)?;
    }
    Ok(())
}

fn prompt_one(
    ws: &Workspace,
    ui: &mut dyn Ui,
    category: Category,
    label: &str,
    item: &items::StatusItem,
) -> Result<(), ReviewError> {
    let header = format!(
        "{label}: {} (last updated {})",
        item.name,
        cli::format_age(item.updated_days_ago)
    );
    let choice = ui.choose(&header, &[('k', "eep"), ('a', "rchive"), ('s', "kip")])?;
    let source_path = ws.category_dir(category).join(&item.name);
    apply_decision(
        ws,
        category,
        &item.name,
        &source_path,
        Decision::from_choice(choice),
    )
}

/// Applies `decision`'s effect to a located `Project`/`Area` item — the
/// single place keep/archive/skip's filesystem effects are defined, called
/// by both the full walk (`run`) and the single-item form (`run_one`).
fn apply_decision(
    ws: &Workspace,
    category: Category,
    name: &str,
    source_path: &std::path::Path,
    decision: Decision,
) -> Result<(), ReviewError> {
    match decision {
        Decision::Keep => {
            let index_path = items::item_path(ws, category, name);
            items::write_last_reviewed(&index_path)?;
        }
        Decision::Archive => {
            items::mv(ws, category, source_path, name, Category::Archive)?;
        }
        Decision::Skip => {}
    }
    Ok(())
}

/// Drives one named `Project`/`Area` item's review decision without walking
/// the rest. `flag_decision` mirrors `--keep`/`--archive`/`--skip`: `Some`
/// applies it directly with no prompt and returns a one-line confirmation
/// (review.md 004 scenarios 1-4); `None` falls back to the same interactive
/// `[k]eep [a]rchive [s]kip?` prompt `run`'s walk uses, for just this one
/// item, and returns `None` since the interactive path already communicates
/// its own outcome via `Ui` (scenario 7). Resolves `name` via
/// `items::locate` and rejects anything that isn't `Project` or `Area` —
/// including no match at all — with `NotReviewable`, since neither case is
/// something review can act on (scenario 5).
pub fn run_one(
    ws: &Workspace,
    ui: &mut dyn Ui,
    name: &str,
    flag_decision: Option<Decision>,
) -> Result<Option<String>, ReviewError> {
    let located = items::locate(ws, name)?;
    let (category, source_path) = match located {
        Some((category @ (Category::Project | Category::Area), path)) => (category, path),
        _ => {
            return Err(ReviewError::NotReviewable {
                name: name.to_string(),
            });
        }
    };

    match flag_decision {
        Some(decision) => {
            apply_decision(ws, category, name, &source_path, decision)?;
            let message = match decision {
                Decision::Keep => format!("Kept {name}."),
                Decision::Archive => format!("Archived {name}."),
                Decision::Skip => format!("Skipped {name}."),
            };
            Ok(Some(message))
        }
        None => {
            let label = match category {
                Category::Project => "Project",
                Category::Area => "Area",
                _ => unreachable!("category is Project or Area, checked above"),
            };
            let items = items::review_items(ws, category)?;
            let item = items
                .iter()
                .find(|item| item.name == name)
                .expect("item located above by items::locate must appear in review_items");
            prompt_one(ws, ui, category, label, item)?;
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::UiError;
    use crate::config::Config;
    use std::cell::RefCell;
    use tempfile::tempdir;

    fn workspace(root: &std::path::Path) -> Workspace {
        Workspace {
            root: root.to_path_buf(),
            config: Config::default(),
        }
    }

    struct FakeUi {
        headers: RefCell<Vec<String>>,
        responses: RefCell<Vec<char>>,
        info_messages: RefCell<Vec<String>>,
    }

    impl FakeUi {
        fn with_responses(responses: Vec<char>) -> Self {
            FakeUi {
                headers: RefCell::new(Vec::new()),
                responses: RefCell::new(responses),
                info_messages: RefCell::new(Vec::new()),
            }
        }
    }

    impl Ui for FakeUi {
        fn confirm(&mut self, _prompt: &str, _default: &str) -> Result<String, UiError> {
            unimplemented!("not exercised by review.md 001")
        }

        fn choose(&mut self, header: &str, _options: &[(char, &str)]) -> Result<char, UiError> {
            self.headers.borrow_mut().push(header.to_string());
            Ok(self.responses.borrow_mut().remove(0))
        }

        fn info(&mut self, message: &str) {
            self.info_messages.borrow_mut().push(message.to_string());
        }
    }

    fn backdate(path: &std::path::Path, days_ago: u64) {
        let modified =
            std::time::SystemTime::now() - std::time::Duration::from_secs(days_ago * 86400);
        let file = std::fs::File::open(path).unwrap();
        file.set_modified(modified).unwrap();
    }

    #[test]
    fn walks_projects_before_areas_alphabetical_within_each_group() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Project, "website-redesign", "# W\n").unwrap();
        items::create(&ws, Category::Project, "my-project", "# M\n").unwrap();
        items::create(&ws, Category::Area, "health", "# H\n").unwrap();
        items::create(&ws, Category::Area, "finances", "# F\n").unwrap();
        let mut ui = FakeUi::with_responses(vec!['k', 'k', 'k', 'k']);

        run(&ws, &mut ui).unwrap();

        let headers = ui.headers.borrow();
        assert_eq!(headers.len(), 4);
        assert!(headers[0].starts_with("Project: my-project"));
        assert!(headers[1].starts_with("Project: website-redesign"));
        assert!(headers[2].starts_with("Area: finances"));
        assert!(headers[3].starts_with("Area: health"));
    }

    #[test]
    fn project_header_matches_documented_format() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let path = items::create(&ws, Category::Project, "website-redesign", "# W\n").unwrap();
        backdate(&path, 12);
        let mut ui = FakeUi::with_responses(vec!['k']);

        run(&ws, &mut ui).unwrap();

        assert_eq!(
            ui.headers.borrow()[0],
            "Project: website-redesign (last updated 12 days ago)"
        );
    }

    #[test]
    fn area_header_uses_area_label() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let path = items::create(&ws, Category::Area, "finances", "# F\n").unwrap();
        backdate(&path, 4);
        let mut ui = FakeUi::with_responses(vec!['k']);

        run(&ws, &mut ui).unwrap();

        assert_eq!(
            ui.headers.borrow()[0],
            "Area: finances (last updated 4 days ago)"
        );
    }

    #[test]
    fn empty_workspace_reports_nothing_to_review_without_prompting() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let mut ui = FakeUi::with_responses(vec![]);

        run(&ws, &mut ui).unwrap();

        assert_eq!(ui.headers.borrow().len(), 0);
        assert_eq!(
            *ui.info_messages.borrow(),
            vec!["Nothing to review.".to_string()]
        );
    }

    #[test]
    fn walk_ends_after_last_item_with_no_extra_prompt() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Area, "health", "# H\n").unwrap();
        let mut ui = FakeUi::with_responses(vec!['k']);

        run(&ws, &mut ui).unwrap();

        assert_eq!(ui.headers.borrow().len(), 1);
    }

    #[test]
    fn keep_stamps_last_reviewed_and_leaves_path_untouched() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Project, "my-project", "# My Project\n").unwrap();
        let mut ui = FakeUi::with_responses(vec!['k']);

        run(&ws, &mut ui).unwrap();

        let path = dir.path().join("1-Projects/my-project/index.md");
        assert!(path.exists());
        let today = chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains(&format!("last_reviewed: {today}")));
    }

    #[test]
    fn skip_touches_neither_path_nor_frontmatter() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let content = "---\nlast_reviewed: 2020-01-01\n---\n# My Project\n";
        let path = items::create(&ws, Category::Project, "my-project", content).unwrap();
        let mut ui = FakeUi::with_responses(vec!['s']);

        run(&ws, &mut ui).unwrap();

        assert!(path.exists());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
    }

    #[test]
    fn archive_moves_project_under_archive_projects() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Project, "website-redesign", "# W\n").unwrap();
        let mut ui = FakeUi::with_responses(vec!['a']);

        run(&ws, &mut ui).unwrap();

        assert!(!dir.path().join("1-Projects/website-redesign").exists());
        assert!(
            dir.path()
                .join("4-Archive/Projects/website-redesign/index.md")
                .exists()
        );
    }

    #[test]
    fn archive_moves_area_under_archive_areas() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Area, "finances", "# F\n").unwrap();
        let mut ui = FakeUi::with_responses(vec!['a']);

        run(&ws, &mut ui).unwrap();

        assert!(!dir.path().join("2-Areas/finances").exists());
        assert!(
            dir.path()
                .join("4-Archive/Areas/finances/index.md")
                .exists()
        );
    }

    #[test]
    fn archiving_one_item_does_not_revisit_it_walk_continues() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Project, "my-project", "# M\n").unwrap();
        items::create(&ws, Category::Project, "website-redesign", "# W\n").unwrap();
        let mut ui = FakeUi::with_responses(vec!['a', 'k']);

        run(&ws, &mut ui).unwrap();

        let headers = ui.headers.borrow();
        assert_eq!(headers.len(), 2);
        assert!(headers[1].starts_with("Project: website-redesign"));
    }

    #[test]
    fn archive_does_not_add_or_modify_last_reviewed() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Project, "my-project", "# My Project\n").unwrap();
        let mut ui = FakeUi::with_responses(vec!['a']);

        run(&ws, &mut ui).unwrap();

        let path = dir.path().join("4-Archive/Projects/my-project/index.md");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.contains("last_reviewed"));
    }

    #[test]
    fn run_one_keep_stamps_last_reviewed_and_leaves_path_untouched() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Project, "my-project", "# My Project\n").unwrap();
        let mut ui = FakeUi::with_responses(vec![]);

        let message = run_one(&ws, &mut ui, "my-project", Some(Decision::Keep)).unwrap();

        let path = dir.path().join("1-Projects/my-project/index.md");
        assert!(path.exists());
        let today = chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains(&format!("last_reviewed: {today}")));
        assert_eq!(message, Some("Kept my-project.".to_string()));
    }

    #[test]
    fn run_one_archive_moves_item_and_leaves_last_reviewed_untouched() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Project, "my-project", "# My Project\n").unwrap();
        let mut ui = FakeUi::with_responses(vec![]);

        let message = run_one(&ws, &mut ui, "my-project", Some(Decision::Archive)).unwrap();

        assert!(!dir.path().join("1-Projects/my-project").exists());
        let path = dir.path().join("4-Archive/Projects/my-project/index.md");
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.contains("last_reviewed"));
        assert_eq!(message, Some("Archived my-project.".to_string()));
    }

    #[test]
    fn run_one_skip_leaves_item_byte_for_byte_unchanged() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let content = "---\nlast_reviewed: 2020-01-01\n---\n# My Project\n";
        let path = items::create(&ws, Category::Project, "my-project", content).unwrap();
        let mut ui = FakeUi::with_responses(vec![]);

        let message = run_one(&ws, &mut ui, "my-project", Some(Decision::Skip)).unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
        assert_eq!(message, Some("Skipped my-project.".to_string()));
    }

    #[test]
    fn run_one_keep_on_area_behaves_like_project() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Area, "finances", "# Finances\n").unwrap();
        let mut ui = FakeUi::with_responses(vec![]);

        let message = run_one(&ws, &mut ui, "finances", Some(Decision::Keep)).unwrap();

        let path = dir.path().join("2-Areas/finances/index.md");
        assert!(path.exists());
        let today = chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains(&format!("last_reviewed: {today}")));
        assert_eq!(message, Some("Kept finances.".to_string()));
    }

    #[test]
    fn run_one_on_a_resource_errors_and_leaves_it_untouched() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let content = "# Recipe Ideas\n";
        let path = items::create(&ws, Category::Resource, "recipe-ideas", content).unwrap();
        let mut ui = FakeUi::with_responses(vec![]);

        let result = run_one(&ws, &mut ui, "recipe-ideas", Some(Decision::Keep));

        assert!(matches!(result, Err(ReviewError::NotReviewable { .. })));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
    }

    #[test]
    fn run_one_on_an_unmatched_name_errors_as_not_reviewable() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let mut ui = FakeUi::with_responses(vec![]);

        let result = run_one(&ws, &mut ui, "nonexistent", Some(Decision::Keep));

        assert!(matches!(result, Err(ReviewError::NotReviewable { .. })));
    }

    #[test]
    fn run_one_with_no_flag_falls_back_to_interactive_prompt_for_just_that_item() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let path = items::create(&ws, Category::Project, "website-redesign", "# W\n").unwrap();
        backdate(&path, 12);
        let mut ui = FakeUi::with_responses(vec!['k']);

        let message = run_one(&ws, &mut ui, "website-redesign", None).unwrap();

        assert_eq!(
            ui.headers.borrow()[0],
            "Project: website-redesign (last updated 12 days ago)"
        );
        let index_path = dir.path().join("1-Projects/website-redesign/index.md");
        let today = chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let content = std::fs::read_to_string(&index_path).unwrap();
        assert!(content.contains(&format!("last_reviewed: {today}")));
        assert_eq!(message, None);
    }
}
