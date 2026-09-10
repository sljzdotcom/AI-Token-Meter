use ai_token_meter_windows::platform::windows::process::CommandInvocation;
use std::ffi::OsString;

pub fn native_fixture_invocation(arguments: Vec<OsString>) -> CommandInvocation {
    CommandInvocation {
        executable: env!("CARGO_BIN_EXE_native-test-fixture").into(),
        arguments,
    }
}
