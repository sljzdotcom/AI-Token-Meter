use ai_token_meter_windows::platform::windows::conpty::{ConPty, ConPtyError, ConPtySize};

#[cfg(windows)]
mod support;

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
    use std::time::Duration;

    let mut terminal = ConPty::open(ConPtySize {
        columns: 120,
        rows: 40,
    })
    .expect("ConPTY");
    let command = support::native_fixture_invocation(vec![OsString::from("conpty-stdin")]);
    let mut child = terminal
        .spawn(&command, None, &[])
        .expect("attached process");

    let ready = terminal
        .read_until(&["ready"], Duration::from_secs(3), 16 * 1024)
        .expect("process readiness after cursor-position handshake");
    assert!(ready.contains("ready"));
    terminal.send_fixed_input(b"hello\r").expect("fixed input");
    let output = terminal
        .read_until(&["received:hello"], Duration::from_secs(3), 16 * 1024)
        .expect("terminal pattern");
    assert!(output.contains("received:hello"));
    assert_eq!(child.wait(Duration::from_secs(3)).expect("child exit"), 0);
}
