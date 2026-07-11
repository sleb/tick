mod common;

use chrono::Local;
use tempfile::tempdir;

fn today() -> String {
    Local::now().date_naive().format("%Y-%m-%d").to_string()
}

#[test]
fn daily_first_run_creates_non_interactively_via_real_dispatch() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());

    let output = common::tk(&["daily"], dir.path(), None, None);

    assert!(output.status.success());
    let root = dir.path().canonicalize().unwrap();
    let expected_path = root.join(format!("0-Inbox/{}.md", today()));
    assert_eq!(
        common::stdout(&output),
        format!(
            "Created inbox/{}.md\nNext: tk list to see it, or tk status for an overview.\n",
            today()
        )
    );
    assert!(expected_path.is_file());
}

#[test]
fn daily_second_run_reopens_via_real_editor_without_recreating() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());
    common::tk(&["daily"], dir.path(), None, None);
    let root = dir.path().canonicalize().unwrap();
    let expected_path = root.join(format!("0-Inbox/{}.md", today()));
    let content_before = std::fs::read_to_string(&expected_path).unwrap();

    let editor = common::write_fake_editor(dir.path(), "fake-editor.sh", "exit 0");
    let output = common::tk(&["daily"], dir.path(), Some(&editor), None);

    assert!(output.status.success());
    assert_eq!(common::stdout(&output), "Opening $EDITOR...\n");
    assert_eq!(
        std::fs::read_to_string(&expected_path).unwrap(),
        content_before
    );
}

#[test]
fn daily_reopen_without_editor_set_surfaces_error() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());
    common::tk(&["daily"], dir.path(), None, None);

    let output = common::tk(&["daily"], dir.path(), None, None);

    assert!(!output.status.success());
    assert!(common::stderr(&output).contains("$EDITOR"));
}

#[test]
fn new_daily_flag_reaches_same_behavior_as_daily_command() {
    let dir = tempdir().unwrap();
    common::init_workspace(dir.path());

    let output = common::tk(&["new", "--daily"], dir.path(), None, None);

    assert!(output.status.success());
    let root = dir.path().canonicalize().unwrap();
    let expected_path = root.join(format!("0-Inbox/{}.md", today()));
    assert_eq!(
        common::stdout(&output),
        format!(
            "Created inbox/{}.md\nNext: tk list to see it, or tk status for an overview.\n",
            today()
        )
    );
    assert!(expected_path.is_file());
}
