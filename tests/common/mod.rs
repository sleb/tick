#![allow(dead_code)]

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// Scaffolds a real PARA workspace at `dir` without going through the `tk`
/// binary, so tests that aren't exercising `init` itself can set up their
/// fixture cheaply.
pub fn init_workspace(dir: &Path) {
    tick::workspace::init(dir).expect("failed to init workspace");
}

/// Writes an executable shell script at `dir/name` to stand in for
/// `$EDITOR`. `body` is the script's shell code; the file it's invoked on is
/// available as `$1`.
pub fn write_fake_editor(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("failed to write fake editor");
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

/// Runs the real `tk` binary with `args` in `dir`. `editor`, if set, becomes
/// `$EDITOR`; otherwise `$EDITOR` is unset. `stdin`, if set, is written to
/// the child's stdin and then closed (EOF), for tests that need to answer a
/// `Ui::confirm` prompt.
pub fn tk(args: &[&str], dir: &Path, editor: Option<&Path>, stdin: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tk"));
    cmd.args(args)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    match editor {
        Some(editor) => cmd.env("EDITOR", editor),
        None => cmd.env_remove("EDITOR"),
    };

    let mut child = cmd.spawn().expect("failed to spawn tk");

    if let Some(input) = stdin {
        let mut child_stdin = child.stdin.take().expect("stdin was piped");
        child_stdin
            .write_all(input.as_bytes())
            .expect("failed to write to tk's stdin");
    }

    child
        .wait_with_output()
        .expect("failed to wait on tk's output")
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout was not valid utf-8")
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr was not valid utf-8")
}
