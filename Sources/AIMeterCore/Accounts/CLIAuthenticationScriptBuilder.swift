import Foundation

public enum CLIAuthenticationScriptError: Error, Equatable {
    case unsupportedProvider
    case invalidCompletionTokenPipe
    case invalidLoginTimeout
}

public struct CLIAuthenticationScriptBuilder: Sendable {
    public init() {}

    public func build(
        provider: UsageProvider,
        executableURL: URL,
        completionTokenPipeURL: URL? = nil,
        completionOpenExecutableURL: URL = URL(fileURLWithPath: "/usr/bin/open"),
        loginTimeoutSeconds: Int = 300
    ) throws -> String {
        let command: String
        switch provider {
        case .claude:
            command = "exec \(shellQuote(executableURL.path)) auth login"
        case .codex:
            command = "exec \(shellQuote(executableURL.path)) login"
        case .gemini:
            guard let completionTokenPipeURL, completionTokenPipeURL.isFileURL,
                  !completionTokenPipeURL.path.isEmpty else {
                throw CLIAuthenticationScriptError.invalidCompletionTokenPipe
            }
            guard (1...1_800).contains(loginTimeoutSeconds) else {
                throw CLIAuthenticationScriptError.invalidLoginTimeout
            }
            command = """
            token_pipe=\(shellQuote(completionTokenPipeURL.path))
            completion_token=""
            if [[ ! -p "$token_pipe" ]]; then
              rm -f -- "$token_pipe"
              exit 124
            fi
            if ! exec 3<> "$token_pipe"; then
              rm -f -- "$token_pipe"
              exit 124
            fi
            if ! IFS= read -r -t \(loginTimeoutSeconds) -u 3 completion_token; then
              exec 3>&-
              rm -f -- "$token_pipe"
              exit 124
            fi
            exec 3>&-
            rm -f -- "$token_pipe"
            if [[ ! "$completion_token" =~ '^[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}$' ]]; then
              exit 127
            fi
            callback_base="aitokenmeter://antigravity-login-complete?token=${completion_token}"
            open_command=\(shellQuote(completionOpenExecutableURL.path))
            agy_pid=""
            watchdog_pid=""
            timed_out=0

            send_receipt() {
              local result="$1"
              local exit_code="$2"
              "$open_command" -g "${callback_base}&result=${result}&exit_code=${exit_code}"
            }

            owned_group_exists() {
              [[ -n "$agy_pid" ]] && /usr/bin/perl -e 'exit(kill(0, -$ARGV[0]) ? 0 : 1)' "$agy_pid"
            }

            stop_owned_processes() {
              if owned_group_exists; then
                /usr/bin/perl -e 'kill "TERM", -$ARGV[0]' "$agy_pid" 2>/dev/null || true
                /bin/sleep 1
                /usr/bin/perl -e 'kill "KILL", -$ARGV[0]' "$agy_pid" 2>/dev/null || true
              fi
              if [[ -n "$agy_pid" ]]; then wait "$agy_pid" 2>/dev/null || true; fi
              if [[ -n "$watchdog_pid" ]] && kill -0 "$watchdog_pid" 2>/dev/null; then
                kill -TERM "$watchdog_pid" 2>/dev/null || true
                wait "$watchdog_pid" 2>/dev/null || true
              fi
              agy_pid=""
              watchdog_pid=""
            }

            cancel_login() {
              local exit_code="$1"
              trap - HUP INT TERM USR1
              stop_owned_processes
              send_receipt cancelled "$exit_code"
              exit "$exit_code"
            }

            mark_timeout() { timed_out=1; }

            trap 'cancel_login 129' HUP
            trap 'cancel_login 130' INT
            trap 'cancel_login 143' TERM
            trap 'mark_timeout' USR1

            /usr/bin/perl -MPOSIX -e 'POSIX::setpgid(0, 0) == 0 or die "setpgid failed"; exec @ARGV or die $!;' \(shellQuote(executableURL.path)) &
            agy_pid=$!
            /usr/bin/perl -e 'my ($parent_pid, $child_pid, $seconds) = @ARGV; sleep $seconds; if (kill 0, -$child_pid) { kill "USR1", $parent_pid; kill "TERM", -$child_pid; select undef, undef, undef, 1; kill "KILL", -$child_pid; }' "$$" "$agy_pid" \(loginTimeoutSeconds) &
            watchdog_pid=$!

            wait "$agy_pid"
            agy_exit_code=$?
            # The CLI can exit while descendants remain in its process group.
            # Keep the watchdog alive until that whole owned group is gone.
            while owned_group_exists && (( ! timed_out )); do
              /bin/sleep 0.1 || true
            done
            if (( timed_out )); then
              # USR1 may interrupt wait before the watchdog's delayed KILL. Finish
              # process-group cleanup before stopping the watchdog.
              stop_owned_processes
            else
              agy_pid=""
            fi
            if [[ -n "$watchdog_pid" ]] && kill -0 "$watchdog_pid" 2>/dev/null; then
              kill -TERM "$watchdog_pid" 2>/dev/null || true
              wait "$watchdog_pid" 2>/dev/null || true
              watchdog_pid=""
            fi

            if (( timed_out )); then
              send_receipt timeout 124
              exit 124
            elif (( agy_exit_code == 0 )); then
              send_receipt success 0
              exit 0
            else
              send_receipt failure "$agy_exit_code"
              exit "$agy_exit_code"
            fi
            """
        case .deepSeek:
            throw CLIAuthenticationScriptError.unsupportedProvider
        }

        let startupCleanup = provider == .gemini ? "rm -f -- \"$0\"" : ":"
        return """
        #!/bin/zsh
        set -u
        unsetopt BG_NICE
        \(startupCleanup)
        export PATH=\(shellQuote(executableURL.deletingLastPathComponent().path)):"${PATH:-/usr/bin:/bin:/usr/sbin:/sbin}"
        \(command)

        """
    }

    private func shellQuote(_ value: String) -> String {
        "'\(value.replacingOccurrences(of: "'", with: "'\\''"))'"
    }
}
