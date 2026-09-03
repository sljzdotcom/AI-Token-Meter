use ai_token_meter_windows::platform::windows::conpty::{ConPty, ConPtyError, ConPtySize};

#[test]
fn invalid_terminal_dimensions_are_rejected_before_platform_access() {
    assert!(matches!(
        ConPty::open(ConPtySize {
            columns: 0,
            rows: 40,
        }),
        Err(ConPtyError::InvalidSize)
    ));
}

#[cfg(windows)]
#[test]
fn conpty_can_be_created_at_the_fixed_terminal_size_and_resized() {
    let mut terminal = ConPty::open(ConPtySize {
        columns: 120,
        rows: 40,
    })
    .expect("ConPTY");

    terminal
        .resize(ConPtySize {
            columns: 132,
            rows: 48,
        })
        .expect("resize ConPTY");
}

#[cfg(windows)]
#[test]
fn conpty_attaches_a_process_sends_fixed_input_and_waits_for_a_pattern() {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use ai_token_meter_windows::platform::windows::process::CommandInvocation;

    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("process-fixture.js");
    let mut terminal = ConPty::open(ConPtySize {
        columns: 120,
        rows: 40,
    })
    .expect("ConPTY");
    let command = CommandInvocation {
        executable: find_node(),
        arguments: vec![fixture.into_os_string(), OsString::from("stdin")],
    };
    let mut child = terminal
        .spawn(&command, None, &[])
        .expect("attached process");

    terminal
        .send_fixed_input(b"hello\r\n")
        .expect("fixed input");
    let output = terminal
        .read_until(&["received:hello"], Duration::from_secs(3), 16 * 1024)
        .expect("terminal pattern");
    assert!(output.contains("received:hello"));
    assert_eq!(child.wait(Duration::from_secs(3)).expect("child exit"), 0);

    fn find_node() -> PathBuf {
        std::env::var_os("PATH")
            .into_iter()
            .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
            .map(|directory| directory.join("node.exe"))
            .find_map(|path| path.canonicalize().ok().filter(|path| path.is_file()))
            .expect("Windows CI provides Node.js")
    }
}
