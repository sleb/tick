mod common;

use tempfile::tempdir;

#[test]
fn new_with_filename_creates_note_via_real_dispatch() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());

    let output = common::tk(&["new", "my-file"], dir.path(), None, None);

    assert!(output.status.success());
    let expected_path = dir
        .path()
        .canonicalize()
        .unwrap()
        .join("0-Inbox/my-file.md");
    assert_eq!(
        common::stdout(&output),
        "Created inbox/my-file.md\nNext: tk list to see it, or tk status for an overview.\n"
    );
    assert!(expected_path.is_file());
}

#[test]
fn new_without_filename_captures_via_real_editor_and_confirm_prompt() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());
    // The daily/note template contains `{{cursor}}`, so `RealEditor::capture`
    // invokes the editor with a leading `+<line>` argument before the file
    // path — take the *last* argument rather than assuming `$1` is the file.
    let editor = common::write_fake_editor(
        dir.path(),
        "fake-editor.sh",
        "for f in \"$@\"; do file=\"$f\"; done\necho '# Title' >> \"$file\"",
    );

    // Empty line accepts the confirm prompt's suggested default filename.
    let output = common::tk(&["new"], dir.path(), Some(&editor), Some("\n"));

    assert!(output.status.success());
    let expected_path = dir.path().canonicalize().unwrap().join("0-Inbox/title.md");
    assert_eq!(
        common::stdout(&output),
        "Opening $EDITOR...\nCreated inbox/title.md\nNext: tk list to see it, or tk status for an overview.\n"
    );
    assert_eq!(common::stderr(&output), "Create \"title.md\"? [title.md] ");
    let content = std::fs::read_to_string(&expected_path).unwrap();
    assert!(content.contains("# Title"));
}

#[test]
fn new_project_flag_scaffolds_directory_via_real_dispatch() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());

    let output = common::tk(
        &["new", "--project", "website-redesign"],
        dir.path(),
        None,
        None,
    );

    assert!(output.status.success());
    let expected_path = dir
        .path()
        .canonicalize()
        .unwrap()
        .join("1-Projects/website-redesign/index.md");
    assert_eq!(
        common::stdout(&output),
        "Created projects/website-redesign/index.md\nNext: tk list to see it, or tk status for an overview.\n"
    );
    assert!(expected_path.is_file());
}

#[test]
fn new_outside_workspace_surfaces_discovery_error() {
    let dir = tempdir().unwrap();

    let output = common::tk(&["new", "my-file"], dir.path(), None, None);

    assert!(!output.status.success());
    assert!(common::stderr(&output).contains("failed to find a PARA workspace"));
}
