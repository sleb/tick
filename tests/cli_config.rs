mod common;

use tempfile::tempdir;

#[test]
fn config_init_creates_local_file_via_real_dispatch() {
    let dir = tempdir().unwrap();

    let output = common::tk(&["config", "init"], dir.path(), None, None);

    assert!(output.status.success());
    assert_eq!(common::stdout(&output), "Created ./.tick.toml\n");
    assert!(dir.path().join(".tick.toml").is_file());
}

#[test]
fn config_init_writes_schema_file_via_real_dispatch() {
    let dir = tempdir().unwrap();

    let output = common::tk(&["config", "init"], dir.path(), None, None);

    assert!(output.status.success());
    let toml_contents = std::fs::read_to_string(dir.path().join(".tick.toml")).unwrap();
    assert!(toml_contents.starts_with("#:schema ./.tick.schema.json"));
    assert!(dir.path().join(".tick.schema.json").is_file());
}

#[test]
fn config_init_refuses_when_local_file_already_exists() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join(".tick.toml"), "custom content").unwrap();

    let output = common::tk(&["config", "init"], dir.path(), None, None);

    assert!(!output.status.success());
    assert!(common::stderr(&output).contains("already exists"));
    assert_eq!(
        std::fs::read_to_string(dir.path().join(".tick.toml")).unwrap(),
        "custom content"
    );
}

#[test]
fn config_init_global_succeeds_when_local_exists_and_leaves_it_untouched() {
    let cwd = tempdir().unwrap();
    let home = tempdir().unwrap();
    std::fs::write(cwd.path().join(".tick.toml"), "custom content").unwrap();

    let output = common::tk_with_home(&["config", "init", "-g"], cwd.path(), home.path());

    assert!(output.status.success());
    assert_eq!(common::stdout(&output), "Created ~/.tick.toml\n");
    assert!(home.path().join(".tick.toml").is_file());
    assert_eq!(
        std::fs::read_to_string(cwd.path().join(".tick.toml")).unwrap(),
        "custom content"
    );
}

#[test]
fn config_init_global_refuses_when_global_file_already_exists() {
    let cwd = tempdir().unwrap();
    let home = tempdir().unwrap();
    std::fs::write(home.path().join(".tick.toml"), "custom content").unwrap();

    let output = common::tk_with_home(&["config", "init", "-g"], cwd.path(), home.path());

    assert!(!output.status.success());
    assert!(common::stderr(&output).contains("already exists"));
    assert_eq!(
        std::fs::read_to_string(home.path().join(".tick.toml")).unwrap(),
        "custom content"
    );
}

#[test]
fn config_edit_opens_existing_local_file_via_real_dispatch() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join(".tick.toml"), "custom content").unwrap();
    let editor = common::write_fake_editor(dir.path(), "fake-editor", "echo opened >> \"$1\"");

    let output = common::tk(&["config", "edit"], dir.path(), Some(&editor), None);

    assert!(output.status.success());
    assert_eq!(common::stdout(&output), "Opening $EDITOR...\n");
    assert_eq!(
        std::fs::read_to_string(dir.path().join(".tick.toml")).unwrap(),
        "custom contentopened\n"
    );
}

#[test]
fn config_edit_does_not_recreate_schema_file_for_existing_config() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join(".tick.toml"), "custom content").unwrap();
    let editor = common::write_fake_editor(dir.path(), "fake-editor", "true");

    let output = common::tk(&["config", "edit"], dir.path(), Some(&editor), None);

    assert!(output.status.success());
    assert!(!dir.path().join(".tick.schema.json").exists());
}

#[test]
fn config_edit_creates_defaults_then_opens_when_local_file_missing() {
    let dir = tempdir().unwrap();
    let editor = common::write_fake_editor(dir.path(), "fake-editor", "true");

    let output = common::tk(&["config", "edit"], dir.path(), Some(&editor), None);

    assert!(output.status.success());
    assert_eq!(
        common::stdout(&output),
        "Created ./.tick.toml\nOpening $EDITOR...\n"
    );
    assert!(dir.path().join(".tick.toml").is_file());
}

#[test]
fn config_edit_global_writes_schema_file_when_missing() {
    let cwd = tempdir().unwrap();
    let home = tempdir().unwrap();
    let editor = common::write_fake_editor(cwd.path(), "fake-editor", "true");

    let output = common::tk_with_home_and_editor(
        &["config", "edit", "-g"],
        cwd.path(),
        home.path(),
        &editor,
    );

    assert!(output.status.success());
    assert!(home.path().join(".tick.schema.json").is_file());
}

#[test]
fn config_edit_global_targets_home_file_and_leaves_local_untouched() {
    let cwd = tempdir().unwrap();
    let home = tempdir().unwrap();
    std::fs::write(cwd.path().join(".tick.toml"), "custom content").unwrap();
    let editor = common::write_fake_editor(cwd.path(), "fake-editor", "true");

    let output = common::tk_with_home_and_editor(
        &["config", "edit", "-g"],
        cwd.path(),
        home.path(),
        &editor,
    );

    assert!(output.status.success());
    assert_eq!(
        common::stdout(&output),
        "Created ~/.tick.toml\nOpening $EDITOR...\n"
    );
    assert!(home.path().join(".tick.toml").is_file());
    assert_eq!(
        std::fs::read_to_string(cwd.path().join(".tick.toml")).unwrap(),
        "custom content"
    );
}
