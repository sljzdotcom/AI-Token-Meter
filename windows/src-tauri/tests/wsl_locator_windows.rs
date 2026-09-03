#![cfg(windows)]

use std::path::PathBuf;

use ai_token_meter_windows::accounts::cli_account::CliProvider;
use ai_token_meter_windows::platform::windows::wsl::build_wsl_invocation;

#[test]
fn distribution_name_is_a_single_argument_even_with_spaces_and_metacharacters() {
    let invocation = build_wsl_invocation(
        PathBuf::from(r"C:\Windows\System32\wsl.exe"),
        "Ubuntu Test & whoami",
        CliProvider::Claude,
        &["auth".to_owned(), "login".to_owned()],
    );

    assert_eq!(invocation.arguments[1], "Ubuntu Test & whoami");
    assert_eq!(invocation.arguments.len(), 6);
}
