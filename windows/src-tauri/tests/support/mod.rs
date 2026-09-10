use ai_token_meter_windows::platform::windows::process::CommandInvocation;

pub fn codex_fixture_invocation() -> CommandInvocation {
    CommandInvocation {
        executable: env!("CARGO_BIN_EXE_codex-app-server-test-fixture").into(),
        arguments: Vec::new(),
    }
}
