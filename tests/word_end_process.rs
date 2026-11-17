//! The guard of `adapdf::word::end_word_process` (the timeout of stage 3 ends
//! the process that the tool started): it ends a `WINWORD.EXE` and nothing
//! else. It needs no Word, so it runs in a plain `cargo test` on Windows.
#![cfg(windows)]

use std::process::{Child, Command, Stdio};

use adapdf::word::end_word_process;

/// A process that is not Word and that waits (`pause` reads its input, which
/// is a pipe that stays open for as long as the test holds the child).
fn start_other_program() -> Child {
    Command::new("cmd")
        .args(["/c", "pause"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("cmd.exe starts")
}

// Spec 02, "Rules for COM": the tool ends only a Word that it started. The
// check of the image name is what keeps an id that another program has reused
// from being ended.
#[test]
fn a_process_that_is_not_word_is_not_ended() {
    let mut child = start_other_program();
    let pid = child.id();

    let error = end_word_process(pid).expect_err("a process that is not Word is an error");
    let text = error.to_string();
    assert!(
        text.contains("cmd.exe") && text.contains("WINWORD.EXE") && text.contains("not ended"),
        "the message names both programs: {text}"
    );
    assert!(
        child.try_wait().unwrap().is_none(),
        "the process is still running"
    );

    // The test ends the process that it started, through its own handle, and
    // lets the handle go so that the id no longer names a process.
    child.kill().unwrap();
    child.wait().unwrap();
    drop(child);
    assert!(
        !end_word_process(pid).expect("a process that is gone is not an error"),
        "a process that is gone gives Ok(false)"
    );
}
