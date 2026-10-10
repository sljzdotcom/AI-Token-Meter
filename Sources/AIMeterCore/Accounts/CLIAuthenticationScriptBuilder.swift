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
        completionStatusFileURL: URL? = nil,
        completionOpenExecutableURL: URL = URL(fileURLWithPath: "/usr/bin/open"),
        watchdogExecutableURL: URL = URL(fileURLWithPath: "/usr/bin/perl"),
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
            if let completionStatusFileURL,
               (!completionStatusFileURL.isFileURL || completionStatusFileURL.path.isEmpty) {
                throw CLIAuthenticationScriptError.invalidCompletionTokenPipe
            }
            let completionStatusPath = completionStatusFileURL?.path
                ?? completionTokenPipeURL.appendingPathExtension("status").path
            command = """
            token_pipe=\(shellQuote(completionTokenPipeURL.path))
            status_file=\(shellQuote(completionStatusPath))
            launch_gate="${status_file}.lock"
            watchdog_ready_file="${status_file}.ready"
            process_group_file="${status_file}.process-group"
            rm -f -- "$process_group_file"
            rm -f -- "$watchdog_ready_file"
            trap 'rm -f -- "$watchdog_ready_file"' EXIT
            completion_token=""
            report_status() { print -r -- "$1" >| "$status_file"; }
            print -r -- "AI Token Meter: Antigravity login helper started; waiting for the app handoff."
            if [[ ! -r "$status_file" ]] || [[ "$(<"$status_file")" != ready ]]; then
              if [[ -r "$status_file" ]] && [[ "$(<"$status_file")" == cancelled ]]; then report_status helper_cancelled; fi
              exit 130
            fi
            if [[ ! -p "$token_pipe" ]]; then
              print -u2 -r -- "AI Token Meter: handoff pipe is missing. Close this window and start a new one-time quota check."
              report_status missing_pipe
              rm -f -- "$token_pipe"
              exit 124
            fi
            if ! exec 3<> "$token_pipe"; then
              print -u2 -r -- "AI Token Meter: could not open the app handoff pipe. Close this window and start a new one-time quota check."
              report_status pipe_open_failed
              rm -f -- "$token_pipe"
              exit 124
            fi
            handoff_waited=0
            handoff_received=0
            while (( handoff_waited < \(loginTimeoutSeconds) )); do
              if IFS= read -r -t 1 -u 3 completion_token; then
                handoff_received=1
                break
              fi
              if [[ ! -r "$status_file" ]] || [[ "$(<"$status_file")" != ready ]]; then
                if [[ -r "$status_file" ]] && [[ "$(<"$status_file")" == cancelled ]]; then report_status helper_cancelled; fi
                exec 3>&-
                rm -f -- "$token_pipe"
                exit 130
              fi
              (( handoff_waited += 1 ))
            done
            if (( ! handoff_received )); then
              exec 3>&-
              rm -f -- "$token_pipe"
              print -u2 -r -- "AI Token Meter: timed out waiting for the app handoff after \(loginTimeoutSeconds) seconds. Start a new one-time quota check."
              report_status handoff_timeout
              exit 124
            fi
            exec 3>&-
            rm -f -- "$token_pipe"
            if [[ ! "$completion_token" =~ '^[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}$' ]]; then
              print -u2 -r -- "AI Token Meter: the app handoff was invalid; this login session cannot continue."
              report_status invalid_handoff
              exit 127
            fi
            if [[ ! -r "$status_file" ]] || [[ "$(<"$status_file")" != ready ]]; then
              if [[ -r "$status_file" ]] && [[ "$(<"$status_file")" == cancelled ]]; then report_status helper_cancelled; fi
              exit 130
            fi
            print -r -- "AI Token Meter: handoff received. Starting Antigravity CLI sign-in in this window."
            callback_base="aitokenmeter://antigravity-login-complete?token=${completion_token}"
            open_command=\(shellQuote(completionOpenExecutableURL.path))
            agy_pid=""
            agy_child_reaped=0
            watchdog_pid=""
            timed_out=0

            send_receipt() {
              local result="$1"
              local exit_code="$2"
              if ! "$open_command" -g "${callback_base}&result=${result}&exit_code=${exit_code}"; then
                print -u2 -r -- "AI Token Meter: could not send the login result back to the app."
                report_status callback_failed
                return 1
              fi
              return 0
            }

            owned_group_exists() {
              [[ -n "$agy_pid" ]] && /usr/bin/perl -e 'exit(kill(0, -$ARGV[0]) ? 0 : 1)' "$agy_pid"
            }

            watchdog_is_running() {
              [[ -n "$watchdog_pid" ]] && kill -0 "$watchdog_pid" 2>/dev/null || return 1
              local watchdog_state
              watchdog_state=$(/bin/ps -o stat= -p "$watchdog_pid" 2>/dev/null)
              [[ -n "$watchdog_state" && "$watchdog_state" != *Z* ]]
            }

            stop_owned_processes() {
              if [[ -n "$agy_pid" ]]; then
                # Before the bootstrapper calls setpgid, only its unreaped
                # direct PID is guaranteed to be ours. After wait returns,
                # never signal that PID again; it may have been reused.
                if owned_group_exists; then
                  /usr/bin/perl -e 'kill "TERM", -$ARGV[0]' "$agy_pid" 2>/dev/null || true
                fi
                if (( ! agy_child_reaped )); then
                  /usr/bin/perl -e 'kill "TERM", $ARGV[0]' "$agy_pid" 2>/dev/null || true
                fi
                /bin/sleep 1
                if owned_group_exists; then
                  /usr/bin/perl -e 'kill "KILL", -$ARGV[0]' "$agy_pid" 2>/dev/null || true
                fi
                if (( ! agy_child_reaped )); then
                  /usr/bin/perl -e 'kill "KILL", $ARGV[0]' "$agy_pid" 2>/dev/null || true
                fi
              fi
              if [[ -n "$agy_pid" ]] && (( ! agy_child_reaped )); then
                wait "$agy_pid" 2>/dev/null || true
                agy_child_reaped=1
              fi
              if [[ -n "$watchdog_pid" ]] && kill -0 "$watchdog_pid" 2>/dev/null; then
                kill -TERM "$watchdog_pid" 2>/dev/null || true
                wait "$watchdog_pid" 2>/dev/null || true
              fi
              agy_pid=""
              agy_child_reaped=0
              watchdog_pid=""
            }

            cleanup_watchdog() {
              if [[ -n "$watchdog_pid" ]] && kill -0 "$watchdog_pid" 2>/dev/null; then
                kill -TERM "$watchdog_pid" 2>/dev/null || true
                wait "$watchdog_pid" 2>/dev/null || true
              fi
              watchdog_pid=""
              rm -f -- "$watchdog_ready_file"
              rm -f -- "$process_group_file"
            }

            cancel_login() {
              local exit_code="$1"
              trap - HUP INT TERM USR1
              stop_owned_processes
              send_receipt cancelled "$exit_code"
              exit "$exit_code"
            }

            cancel_from_app() {
              trap - HUP INT TERM USR1 USR2
              stop_owned_processes
              if [[ -r "$status_file" ]] && [[ "$(<"$status_file")" == cancelled ]]; then
                report_status helper_cancelled
              fi
              exit 130
            }

            mark_timeout() { timed_out=1; }

            trap 'cancel_login 129' HUP
            trap 'cancel_login 130' INT
            trap 'cancel_login 143' TERM
            trap 'mark_timeout' USR1
            trap 'cancel_from_app' USR2
            trap 'cleanup_watchdog' EXIT

            \(shellQuote(watchdogExecutableURL.path)) -e 'use Fcntl qw(:DEFAULT); use Time::HiRes qw(time); my ($parent_pid, $seconds, $status_file, $ready_file, $process_group_file, $launch_gate) = @ARGV; my $read_status = sub { my $value = ""; if (open my $handle, "<", $status_file) { $value = <$handle> // ""; close $handle; chomp $value; } return $value; }; my $finish_deadline = sub { return if getppid() != $parent_pid; open my $gate, "+<", $launch_gate or do { kill "USR2", $parent_pid; return; }; my $record_lock = pack("qqiss", 0, 0, 0, F_WRLCK, SEEK_SET); my $locked = 0; for (1..20) { if (defined(fcntl($gate, F_SETLK, $record_lock))) { $locked = 1; last; } if ($read_status->() ne "ready") { close $gate; kill "USR2", $parent_pid; return; } select undef, undef, undef, 0.05; } if (!$locked) { close $gate; kill "USR1", $parent_pid if getppid() == $parent_pid; return; } if ($read_status->() ne "ready") { kill "USR2", $parent_pid; } else { kill "USR1", $parent_pid if getppid() == $parent_pid; } close $gate; }; my $launch_deadline = time + 10; while (1) { exit 0 if getppid() != $parent_pid; if ($read_status->() ne "ready") { kill "USR2", $parent_pid; exit 0; } if (!-e $ready_file) { open my $ready_handle, ">", $ready_file or die "watchdog ready file unavailable"; close $ready_handle; } last if -e $process_group_file; if (time >= $launch_deadline) { $finish_deadline->(); exit 0; } select undef, undef, undef, 0.1; } my $deadline = time + $seconds; while (time < $deadline) { exit 0 if getppid() != $parent_pid; if ($read_status->() ne "ready") { kill "USR2", $parent_pid; exit 0; } select undef, undef, undef, 0.1; } $finish_deadline->();' "$$" \(loginTimeoutSeconds) "$status_file" "$watchdog_ready_file" "$process_group_file" "$launch_gate" &
            watchdog_pid=$!
            watchdog_ready=0
            for _ in {1..50}; do
              if [[ -f "$watchdog_ready_file" ]] && watchdog_is_running; then
                watchdog_ready=1
                break
              fi
              if ! kill -0 "$watchdog_pid" 2>/dev/null; then break; fi
              /bin/sleep 0.1 || true
            done
            if (( ! watchdog_ready )); then
              print -u2 -r -- "AI Token Meter: could not start the Antigravity login watchdog; the CLI was not started."
              report_status watchdog_start_failed
              stop_owned_processes
              exit 127
            fi

            if [[ ! -r "$status_file" ]] || [[ "$(<"$status_file")" != ready ]]; then
              if [[ -r "$status_file" ]] && [[ "$(<"$status_file")" == cancelled ]]; then report_status helper_cancelled; fi
              stop_owned_processes
              exit 130
            fi

            /usr/bin/perl -MFcntl=:DEFAULT -MPOSIX -e 'my ($gate_path, $status_path, $process_group_file, @command) = @ARGV; open my $gate, "+<", $gate_path or exit 130; my $record_lock = pack("qqiss", 0, 0, 0, F_WRLCK, SEEK_SET); defined(fcntl($gate, F_SETLKW, $record_lock)) or exit 130; my $status = ""; if (open my $status_handle, "<", $status_path) { $status = <$status_handle> // ""; close $status_handle; chomp $status; } if ($status ne "ready") { if ($status eq "cancelled" && open my $ack, ">", $status_path) { print {$ack} "helper_cancelled\\n"; close $ack; } exit 130; } POSIX::setpgid(0, 0) == 0 or die "setpgid failed"; open my $group_handle, ">", $process_group_file or die "process group marker unavailable"; chmod 0600, $process_group_file or die "process group marker permissions unavailable"; print {$group_handle} "$$\\n" or die "process group marker write failed"; close $group_handle or die "process group marker close failed"; my $flags = fcntl($gate, F_GETFD, 0); defined($flags) && defined(fcntl($gate, F_SETFD, $flags | FD_CLOEXEC)) or die "gate close-on-exec failed"; exec @command or die $!;' "$launch_gate" "$status_file" "$process_group_file" \(shellQuote(executableURL.path)) &
            agy_pid=$!

            agy_exit_code=0
            while (( ! timed_out )); do
              agy_state=$(/bin/ps -o stat= -p "$agy_pid" 2>/dev/null)
              if [[ -z "$agy_state" || "$agy_state" == *Z* ]]; then
                wait "$agy_pid"
                agy_exit_code=$?
                agy_child_reaped=1
                break
              fi
              /bin/sleep 0.1 || true
            done
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
              print -u2 -r -- "AI Token Meter: Antigravity sign-in timed out and its login process was stopped."
              send_receipt timeout 124
              exit 124
            elif (( agy_exit_code == 0 )); then
              print -r -- "AI Token Meter: Antigravity CLI sign-in finished. Sending the result to the app."
              send_receipt success 0
              exit 0
            else
              print -u2 -r -- "AI Token Meter: Antigravity CLI sign-in failed with exit code $agy_exit_code."
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
