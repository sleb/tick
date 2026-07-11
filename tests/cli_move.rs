mod common;

use tempfile::tempdir;

#[test]
fn move_relocates_flat_file_into_project_via_real_dispatch() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());
    tick::items::create(
        &tick::workspace::Workspace {
            root: dir.path().to_path_buf(),
            config: tick::config::Config::default(),
        },
        tick::category::Category::Inbox,
        "my-file",
        "hello",
    )
    .unwrap();

    let output = common::tk(&["move", "my-file", "project"], dir.path(), None, None);

    assert!(output.status.success());
    let root = dir.path().canonicalize().unwrap();
    let source_path = root.join("0-Inbox/my-file.md");
    let dest_path = root.join("1-Projects/my-file/index.md");
    assert_eq!(
        common::stdout(&output),
        "Moved inbox/my-file.md to projects/my-file/index.md\n"
    );
    assert!(dest_path.is_file());
    assert!(!source_path.exists());
}

#[test]
fn mv_alias_behaves_identically_to_move() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());
    tick::items::create(
        &tick::workspace::Workspace {
            root: dir.path().to_path_buf(),
            config: tick::config::Config::default(),
        },
        tick::category::Category::Inbox,
        "my-file",
        "hello",
    )
    .unwrap();

    let output = common::tk(&["mv", "my-file", "project"], dir.path(), None, None);

    assert!(output.status.success());
    let root = dir.path().canonicalize().unwrap();
    let dest_path = root.join("1-Projects/my-file/index.md");
    assert!(dest_path.is_file());
}

#[test]
fn archive_alias_moves_item_to_archive_via_real_dispatch() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());
    tick::items::create(
        &tick::workspace::Workspace {
            root: dir.path().to_path_buf(),
            config: tick::config::Config::default(),
        },
        tick::category::Category::Resource,
        "my-file",
        "hello",
    )
    .unwrap();

    let output = common::tk(&["archive", "my-file"], dir.path(), None, Some("\n"));

    assert!(output.status.success());
    let root = dir.path().canonicalize().unwrap();
    let dest_path = root.join("4-Archive/Resources/my-file.md");
    assert!(dest_path.is_file());
    assert!(!root.join("3-Resources/my-file.md").exists());
}

#[test]
fn archive_yes_flag_skips_summary_prompt() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());
    tick::items::create(
        &tick::workspace::Workspace {
            root: dir.path().to_path_buf(),
            config: tick::config::Config::default(),
        },
        tick::category::Category::Resource,
        "my-file",
        "hello",
    )
    .unwrap();

    let output = common::tk(&["archive", "my-file", "--yes"], dir.path(), None, None);

    assert!(output.status.success());
    assert_eq!(common::stderr(&output), "");
    let root = dir.path().canonicalize().unwrap();
    let dest_path = root.join("4-Archive/Resources/my-file.md");
    assert!(dest_path.is_file());
    assert!(!root.join("3-Resources/my-file.md").exists());
}

#[test]
fn unarchive_restores_item_to_its_origin_category_via_real_dispatch() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());
    tick::items::create(
        &tick::workspace::Workspace {
            root: dir.path().to_path_buf(),
            config: tick::config::Config::default(),
        },
        tick::category::Category::Resource,
        "my-file",
        "hello",
    )
    .unwrap();
    common::tk(&["archive", "my-file"], dir.path(), None, Some("\n"));

    let output = common::tk(&["unarchive", "Resources/my-file"], dir.path(), None, None);

    assert!(output.status.success());
    let root = dir.path().canonicalize().unwrap();
    let dest_path = root.join("3-Resources/my-file.md");
    assert!(dest_path.is_file());
    assert!(!root.join("4-Archive/Resources/my-file.md").exists());
}

#[test]
fn unarchive_rejects_a_bare_name_matching_a_live_item() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());
    tick::items::create(
        &tick::workspace::Workspace {
            root: dir.path().to_path_buf(),
            config: tick::config::Config::default(),
        },
        tick::category::Category::Resource,
        "my-file",
        "hello",
    )
    .unwrap();

    let output = common::tk(&["unarchive", "my-file"], dir.path(), None, None);

    assert!(!output.status.success());
    let root = dir.path().canonicalize().unwrap();
    assert!(root.join("3-Resources/my-file.md").exists());
}

#[test]
fn archive_alias_rejects_category_argument() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());
    tick::items::create(
        &tick::workspace::Workspace {
            root: dir.path().to_path_buf(),
            config: tick::config::Config::default(),
        },
        tick::category::Category::Resource,
        "my-file",
        "hello",
    )
    .unwrap();

    let output = common::tk(&["archive", "my-file", "archive"], dir.path(), None, None);

    assert!(!output.status.success());
    let root = dir.path().canonicalize().unwrap();
    assert!(root.join("3-Resources/my-file.md").exists());
}
