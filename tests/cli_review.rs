mod common;

use tempfile::tempdir;

#[test]
fn review_walks_projects_then_areas_via_real_dispatch() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());
    tick::items::create(
        &tick::workspace::Workspace {
            root: dir.path().to_path_buf(),
            config: tick::config::Config::default(),
        },
        tick::category::Category::Project,
        "website-redesign",
        "# Website Redesign\n",
    )
    .unwrap();
    tick::items::create(
        &tick::workspace::Workspace {
            root: dir.path().to_path_buf(),
            config: tick::config::Config::default(),
        },
        tick::category::Category::Area,
        "health",
        "# Health\n",
    )
    .unwrap();

    let output = common::tk(&["review"], dir.path(), None, Some("k\nk\n"));

    assert!(output.status.success());
    let stdout = common::stdout(&output);
    assert!(stdout.contains("Project: website-redesign (last updated today)"));
    assert!(stdout.contains("Area: health (last updated today)"));
    assert!(stdout.contains("[k]eep  [a]rchive  [s]kip?"));
}

#[test]
fn review_reports_nothing_to_review_on_empty_workspace() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());

    let output = common::tk(&["review"], dir.path(), None, None);

    assert!(output.status.success());
    assert_eq!(common::stdout(&output), "Nothing to review.\n");
}

#[test]
fn review_keep_and_archive_apply_real_filesystem_effects() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());
    tick::items::create(
        &tick::workspace::Workspace {
            root: dir.path().to_path_buf(),
            config: tick::config::Config::default(),
        },
        tick::category::Category::Project,
        "keep-me",
        "# Keep Me\n",
    )
    .unwrap();
    tick::items::create(
        &tick::workspace::Workspace {
            root: dir.path().to_path_buf(),
            config: tick::config::Config::default(),
        },
        tick::category::Category::Project,
        "archive-me",
        "# Archive Me\n",
    )
    .unwrap();

    // Walk visits alphabetically: "archive-me" first, then "keep-me". The
    // first response ('k') keeps "archive-me"; the second ('a') archives
    // "keep-me" — despite the item names, which just establish walk order.
    let output = common::tk(&["review"], dir.path(), None, Some("k\na\n"));

    assert!(output.status.success());

    let kept_path = dir.path().join("1-Projects/archive-me/index.md");
    assert!(kept_path.exists());
    let today = chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let kept_content = std::fs::read_to_string(&kept_path).unwrap();
    assert!(kept_content.contains(&format!("last_reviewed: {today}")));

    assert!(!dir.path().join("1-Projects/keep-me").exists());
    assert!(
        dir.path()
            .join("4-Archive/Projects/keep-me/index.md")
            .exists()
    );
}
