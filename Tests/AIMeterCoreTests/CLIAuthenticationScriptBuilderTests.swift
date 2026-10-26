import Foundation
import Darwin
import Dispatch
import Testing
@testable import AIMeterCore

@Suite("CLI authentication script builder", .serialized)
struct CLIAuthenticationScriptBuilderTests {
    @Test("Claude and Codex scripts contain only approved login commands")
    func approvedCommands() throws {
        let builder = CLIAuthenticationScriptBuilder()
        let claude = try builder.build(
            provider: .claude,
            executableURL: URL(fileURLWithPath: "/tmp/Claude CLI/claude")
        )
        let codex = try builder.build(
            provider: .codex,
            executableURL: URL(fileURLWithPath: "/tmp/Codex CLI/codex")
        )

        #expect(claude.contains("exec '/tmp/Claude CLI/claude' auth login"))
        #expect(codex.contains("exec '/tmp/Codex CLI/codex' login"))
        #expect(codex.contains("export PATH='/tmp/Codex CLI':\"${PATH:-/usr/bin:/bin:/usr/sbin:/sbin}\""))
        #expect(!claude.localizedCaseInsensitiveContains("token"))
        #expect(!codex.localizedCaseInsensitiveContains("api key"))
    }

    @Test("Executable paths use safe shell quoting")
    func quotesPaths() throws {
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .claude,
            executableURL: URL(fileURLWithPath: "/tmp/Miller's Tools/claude")
        )

        #expect(script.contains("'/tmp/Miller'\\''s Tools/claude'"))
    }

    @Test("Antigravity helper reports FIFO handoff and terminal failures")
    func geminiHelperReportsStartupProgressAndEarlyFailures() throws {
        let root = temporaryDirectory()
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: root.appendingPathComponent("agy"),
            completionTokenPipeURL: root.appendingPathComponent("handoff.token"),
            completionOpenExecutableURL: root.appendingPathComponent("fake-open"),
            loginTimeoutSeconds: 37
        )

        #expect(script.contains("Antigravity login helper started; waiting for the app handoff."))
        #expect(script.contains("handoff pipe is missing."))
        #expect(script.contains("could not open the app handoff pipe."))
        #expect(script.contains("timed out waiting for the app handoff after 37 seconds."))
        #expect(script.contains("the app handoff was invalid"))
        #expect(script.contains("handoff received. Waiting for the Antigravity CLI process to start."))
        #expect(script.contains("could not send the login result back to the app."))
        #expect(script.contains("Antigravity CLI sign-in failed with exit code $agy_exit_code."))
        #expect(script.contains("Antigravity sign-in timed out and its login process was stopped."))
        #expect(script.contains("open_command='\(root.appendingPathComponent("fake-open").path)'"))
    }

    @Test("Antigravity helper gates CLI startup on a ready watchdog and live handoff")
    func geminiCLIStartsOnlyAfterCancellationWatchdogIsReady() throws {
        let root = temporaryDirectory()
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: root.appendingPathComponent("agy"),
            completionTokenPipeURL: root.appendingPathComponent("handoff.token"),
            completionStatusFileURL: root.appendingPathComponent("handoff.status"),
            watchdogExecutableURL: root.appendingPathComponent("watchdog")
        )

        let watchdogLaunch = try #require(script.range(of: "watchdog_pid=$!"))
        let cliLaunch = try #require(script.range(of: "/usr/bin/perl -MFcntl=:DEFAULT -MPOSIX -MTime::HiRes=time,sleep -e"))
        #expect(watchdogLaunch.lowerBound < cliLaunch.lowerBound)
        #expect(script.contains("if [[ ! -r \"$status_file\" ]] || [[ \"$(<\"$status_file\")\" != ready ]] || stop_requested; then"))
        #expect(script.contains("my $read_status = sub"))
        #expect(script.contains("my $finish_deadline = sub"))
        #expect(script.contains("defined(fcntl($gate, F_SETLK, $record_lock))"))
        #expect(script.contains("defined(fcntl($gate, F_SETLK, $record_lock))"))
        #expect(script.contains("\"$process_group_file\" \"$launch_gate\" &"))
        #expect(script.contains("watchdog_state=\"$(process_state -o stat= -p \"$watchdog_pid\")\""))
        #expect(script.contains("\"$watchdog_state\" != *Z*"))
        #expect(script.contains("defined(fcntl($gate, F_SETLK, $record_lock))"))
        #expect(script.contains("fcntl($gate, F_SETFD, $flags | FD_CLOEXEC)"))
        #expect(script.contains("exec_status_reader"))
        #expect(script.contains("exec_status_writer"))
        #expect(script.contains("O_NONBLOCK"))
        #expect(script.contains("if ($stop_requested->()) { $clean_group->(\"TERM\"); }"))
        #expect(script.contains("my ($process_rows, $query_pid) = bounded_process_state($process_state_path"))
        #expect(script.contains("defined($query_pid) && $query_pid > 0 && $query_pid != $$"))
        #expect(script.contains("if ($pid == $query_pid) { return undef if $query_seen++; next; }"))
        #expect(script.contains("return undef unless defined($process_rows)"))
        #expect(script.contains("my $deadline = clock_gettime(CLOCK_MONOTONIC) + 0.25"))
        #expect(script.contains("my $waited = waitpid($probe_pid, WNOHANG)"))
        #expect(script.contains("exec_failed"))
        #expect(script.contains("exec_confirmed"))
        #expect(script.contains("launch_indeterminate"))
    }

    @Test("Antigravity supervisor confirms child stop before publishing its process group")
    func geminiSupervisorWaitsForChildStopBeforePublishingGroup() throws {
        let root = temporaryDirectory()
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: root.appendingPathComponent("agy"),
            completionTokenPipeURL: root.appendingPathComponent("handoff.token"),
            completionStatusFileURL: root.appendingPathComponent("handoff.status")
        )

        let childStop = try #require(script.range(of: "kill \"STOP\", $$"))
        let childStopWait = script.range(of: "waitpid($child, WNOHANG | WUNTRACED)")
        let childStopConfirmation = script.range(of: "WIFSTOPPED(${^CHILD_ERROR_NATIVE}) ? 1 : 0")
        let processGroupPublication = try #require(script.range(of: "print {$group_handle} \"$$\\n\""))

        #expect(childStop.lowerBound < processGroupPublication.lowerBound)
        #expect(childStopWait != nil)
        #expect(childStopConfirmation != nil)
        if let childStopWait, let childStopConfirmation {
            #expect(childStop.lowerBound < childStopWait.lowerBound)
            #expect(childStopWait.lowerBound < childStopConfirmation.lowerBound)
            #expect(childStopConfirmation.lowerBound < processGroupPublication.lowerBound)
        }
    }

    @Test("Antigravity supervisor handles exec pipe short writes and bounds indeterminate cleanup")
    func geminiSupervisorBoundsExecPipeFailureHandling() throws {
        let root = temporaryDirectory()
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: root.appendingPathComponent("agy"),
            completionTokenPipeURL: root.appendingPathComponent("handoff.token"),
            completionStatusFileURL: root.appendingPathComponent("handoff.status")
        )

        let shortWriteLoop = try #require(script.range(of: "while ($write_offset < length($exec_error))"))
        let shortWriteAdvance = try #require(script.range(of: "$write_offset += $written"))
        let indeterminateCleanup = try #require(script.range(of: "if ($exec_state eq \"launch_indeterminate\") { $clean_group->(\"TERM\"); _exit(127); }"))
        let blockingWait = try #require(script.range(of: "waitpid($child, 0)"))

        #expect(shortWriteLoop.lowerBound < shortWriteAdvance.lowerBound)
        #expect(indeterminateCleanup.lowerBound < blockingWait.lowerBound)
        #expect(script.contains("if (!defined($read_count))"))
        #expect(script.contains("$exec_error = \"indeterminate\""))
    }

    @Test("Antigravity supervisor preserves a child already reaped during exec confirmation")
    func geminiSupervisorKeepsChildStatusAfterExecProbe() throws {
        let root = temporaryDirectory()
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: root.appendingPathComponent("agy"),
            completionTokenPipeURL: root.appendingPathComponent("handoff.token"),
            completionStatusFileURL: root.appendingPathComponent("handoff.status")
        )

        #expect(script.contains("if ($probe == $child) { $child_status = $?; $child_done = 1; $exec_state"))
        #expect(script.contains("while (!$child_done) {"))
    }

    @Test("Antigravity stop cleanup remains bounded and preserves uncertain guardian state")
    func geminiStopCleanupFailsClosedWhenGuardianStateIsUnknown() throws {
        let root = temporaryDirectory()
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: root.appendingPathComponent("agy"),
            completionTokenPipeURL: root.appendingPathComponent("handoff.token"),
            completionStatusFileURL: root.appendingPathComponent("handoff.status")
        )

        #expect(script.contains("cleanup_uncertain=0"))
        #expect(script.contains("if (( cleanup_uncertain )); then"))
        #expect(script.contains("guardian_state_preserved=1"))
        #expect(script.contains("return 1"))
        #expect(script.contains("if ! agy_state=\"$(owned_supervisor_state)\"; then"))
        #expect(script.contains("report_status process_state_unavailable"))
        #expect(script.contains("if (( timeout_cleanup_confirmed )); then"))
        #expect(script.contains("process cleanup is unconfirmed"))
        #expect(!script.contains("kill \"KILL\", $ARGV[0]"))
    }

    @Test("Antigravity shell preserves guardian state when a process probe is unknown")
    func geminiProcessProbeUnknownPreservesGuardianState() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: root.appendingPathComponent("agy"),
            completionTokenPipeURL: root.appendingPathComponent("handoff.token"),
            completionStatusFileURL: root.appendingPathComponent("handoff.status")
        )

        #expect(script.contains("process_state_executable='/bin/ps'"))
        #expect(script.contains("return undef unless defined($process_rows)"))
        #expect(script.contains("process_state_unavailable\\n"))
        #expect(script.contains("if [[ -r \"$status_file\" && ( \"$(<\"$status_file\")\" == process_state_unavailable || \"$(<\"$status_file\")\" == helper_stop_unconfirmed ) ]]; then"))
        let groupStart = try #require(script.range(of: "process_group_is_empty() {"))
        let groupEnd = try #require(script.range(of: "watchdog_is_running() {", range: groupStart.upperBound..<script.endIndex))
        let groupCheck = String(script[groupStart.lowerBound..<groupEnd.lowerBound])
        #expect(!groupCheck.contains("process_state -o"))
        #expect(groupCheck.contains("exit 1 if $matched; exit($! == ESRCH ? 0 : 1)"))
        #expect(script.contains("kill \"-0\", $pgid"))
        #expect(script.contains("exit($! == ESRCH ? 0 : 1)"))
        #expect(script.contains("process_group_is_empty"))
        #expect(script.contains("guardian_state_preserved=1"))
        #expect(!script.contains("=$(/bin/ps"))
    }

    @Test("Antigravity process-group probe rejects malformed process rows")
    func geminiProcessGroupProbeRejectsMalformedRows() throws {
        let root = temporaryDirectory()
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: root.appendingPathComponent("agy"),
            completionTokenPipeURL: root.appendingPathComponent("handoff.token"),
            completionStatusFileURL: root.appendingPathComponent("handoff.status")
        )

        #expect(script.contains(#"my ($pid, $state) = $line =~ /^\s*([0-9]+)\s+(\S+)\s*$/;"#))
        #expect(script.contains("return undef unless defined($pid) && defined($state) && $pid > 0"))
        #expect(script.contains("my $self_seen = 0"))
        #expect(script.contains("return undef unless $self_seen"))
    }

    @Test("Antigravity process-state probe rechecks its deadline after interrupted reads")
    func geminiProcessStateProbeRechecksDeadlineAfterInterruptedRead() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: root.appendingPathComponent("agy"),
            completionTokenPipeURL: root.appendingPathComponent("handoff.token"),
            completionStatusFileURL: root.appendingPathComponent("handoff.status")
        )

        #expect(script.contains("next if $! == EINTR && clock_gettime(CLOCK_MONOTONIC) < $deadline"))
    }

    @Test("Antigravity process-state probe times out a hung owned query")
    func geminiProcessStateProbeTimesOutHungOwnedQuery() throws {
        let program = CLIAuthenticationScriptBuilder.boundedProcessStateProbeProgram
        let probe = Process()
        probe.executableURL = URL(fileURLWithPath: "/usr/bin/perl")
        probe.arguments = [
            "-e",
            program,
            "/bin/sleep",
            "0.9"
        ]
        probe.standardOutput = Pipe()
        probe.standardError = Pipe()

        let startedAt = ContinuousClock.now
        try probe.run()
        let deadline = ContinuousClock.now.advanced(by: .seconds(2))
        while probe.isRunning, ContinuousClock.now < deadline {
            Thread.sleep(forTimeInterval: 0.01)
        }
        let returnedWithinBound = !probe.isRunning
        if !returnedWithinBound {
            // The fake query exits naturally in under one second even if the
            // production wrapper regresses and fails to terminate it.
            Thread.sleep(forTimeInterval: 1)
            if probe.isRunning { probe.terminate() }
        }
        probe.waitUntilExit()
        let elapsed = startedAt.duration(to: ContinuousClock.now)

        #expect(returnedWithinBound)
        #expect(elapsed >= .seconds(0.20))
        #expect(elapsed < .seconds(0.75))
        #expect(probe.terminationStatus == 2)
        #expect(program.contains("if (!$child_done && $child_owned)"))
        #expect(program.contains("kill \"TERM\", $probe_pid"))
        #expect(program.contains("kill \"KILL\", $probe_pid"))
        #expect(program.contains("my $term_deadline = clock_gettime(CLOCK_MONOTONIC) + 0.05"))
        #expect(program.contains("my $kill_deadline = clock_gettime(CLOCK_MONOTONIC) + 0.1"))
        #expect(!program.contains("kill \"TERM\", -$probe_pid"))
    }

    @Test("Antigravity process-state probe rejects a failed query and its partial output")
    func geminiProcessStateProbeRejectsFailedQueryOutput() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let fakeQuery = try writeExecutable(
            "#!/bin/sh\nprintf '99999 S\\n'\nexit 42\n",
            named: "fake-ps-failure",
            in: root
        )
        let output = Pipe()
        let probe = Process()
        probe.executableURL = URL(fileURLWithPath: "/usr/bin/perl")
        probe.arguments = [
            "-e",
            CLIAuthenticationScriptBuilder.boundedProcessStateProbeProgram,
            fakeQuery.path
        ]
        probe.standardOutput = output
        probe.standardError = Pipe()

        try probe.run()
        probe.waitUntilExit()
        let observedOutput = output.fileHandleForReading.readDataToEndOfFile()

        #expect(probe.terminationReason == .exit)
        #expect(probe.terminationStatus == 2)
        #expect(observedOutput.isEmpty)
    }

    @Test("Antigravity drains query output after reaping the child", arguments: [0, 42])
    func geminiProcessStateProbeDrainsAfterReaping(queryExitCode: Int) throws {
        let root = temporaryDirectory()
        try FileManager.default.createDirectory(
            at: root, withIntermediateDirectories: false, attributes: [.posixPermissions: 0o700]
        )
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        // Hold reads at EAGAIN until the real query has been reaped. The
        // production probe must then drain its real pipe without reaping again.
        let program = #"""
        use strict; use warnings;
        use Fcntl qw(F_GETFL F_SETFL O_NONBLOCK);
        use POSIX qw(_exit WNOHANG);
        use Errno qw(EINTR EAGAIN EWOULDBLOCK ECHILD);
        use Time::HiRes qw(clock_gettime CLOCK_MONOTONIC);
        use JSON::PP ();
        our ($reaped_pid, $saved_exit, $delayed_reads, $wait_after_reap, $eof_seen, $signals) = (0, -1, 0, 0, 0, 0);
        our @events;
        BEGIN {
          *CORE::GLOBAL::sysread = sub {
            my $status = $?;
            if (!$reaped_pid) {
              ++$delayed_reads;
              push @events, "read_delayed";
              $! = EAGAIN; $? = $status;
              return undef;
            }
            my $count = CORE::sysread($_[0], $_[1], $_[2]);
            my $error = 0 + $!;
            if (defined($count) && $count == 0) { $eof_seen = 1; push @events, "eof_after_reap"; }
            $! = $error; $? = $status;
            return $count;
          };
          *CORE::GLOBAL::waitpid = sub {
            ++$wait_after_reap if $reaped_pid;
            my $waited = CORE::waitpid($_[0], $_[1]);
            my ($status, $error) = ($?, 0 + $!);
            if ($waited == $_[0]) {
              $reaped_pid = $waited; $saved_exit = $status;
              push @events, "reaped";
            } elsif ($waited == -1 && $error == ECHILD) {
              push @events, "echild";
            }
            $! = $error; $? = $status;
            return $waited;
          };
          *CORE::GLOBAL::kill = sub {
            my $status = $?;
            ++$signals;
            my $sent = CORE::kill(@_);
            my $error = 0 + $!;
            $! = $error; $? = $status;
            return $sent;
          };
        }
        \#(CLIAuthenticationScriptBuilder.boundedProcessStateProbeSubroutine)
        my $expected_exit = shift @ARGV;
        die "fixed query exit required" unless defined($expected_exit) && ($expected_exit eq "0" || $expected_exit eq "42");
        my $output = bounded_process_state("/usr/bin/perl", "-e", 'print "S\n"; exit shift;', $expected_exit);
        my $accepted = $expected_exit == 0 ? (defined($output) && $output eq "S\n") : !defined($output);
        my $ok = $accepted && $delayed_reads > 0 && $reaped_pid > 0 && $eof_seen
          && $saved_exit == ($expected_exit << 8) && $wait_after_reap == 0 && $signals == 0;
        print JSON::PP->new->canonical->encode({output => $output, delayed_reads => $delayed_reads,
          reaped_pid => $reaped_pid, saved_exit => $saved_exit, eof_seen => $eof_seen,
          wait_after_reap => $wait_after_reap, signals => $signals, events => \@events}), "\n";
        exit($ok ? 0 : 1);
        """#
        let result = try #require(FixtureProcessQuery.run(
            executable: URL(fileURLWithPath: "/usr/bin/perl"),
            arguments: ["-e", program, String(queryExitCode)],
            in: root,
            timeout: .seconds(2)
        ))
        #expect(result.exitStatus == 0, "Controlled read/wait trace: \(result.text)")
        let trace = try #require(JSONSerialization.jsonObject(with: Data(result.text.utf8)) as? [String: Any])
        #expect((trace["delayed_reads"] as? Int ?? 0) > 0)
        #expect((trace["reaped_pid"] as? Int ?? 0) > 0)
        #expect(trace["saved_exit"] as? Int == queryExitCode << 8)
        #expect(trace["eof_seen"] as? Int == 1)
        #expect(trace["wait_after_reap"] as? Int == 0)
        #expect(trace["signals"] as? Int == 0)
        if queryExitCode == 0 {
            #expect(trace["output"] as? String == "S\n")
        } else {
            #expect(trace["output"] is NSNull)
        }
    }

    @Test("Antigravity preserves state when the owned process-group probe fails")
    func geminiProcessGroupProbeFailurePreservesStateWithoutReceipt() throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let tokenPipe = root.appendingPathComponent("completion.token")
        let statusFile = tokenPipe.appendingPathExtension("status")
        let cliPIDFile = root.appendingPathComponent("agy.pid")
        let cliStartedFile = root.appendingPathComponent("agy.started")
        let watchdogPIDFile = root.appendingPathComponent("watchdog.pid")
        let untrackedWatchdogPIDFile = root.appendingPathComponent("untracked-watchdog.pid")
        let callbackLog = root.appendingPathComponent("callbacks.txt")
        try createTokenPipe(at: tokenPipe)

        let executable = try writeExecutable(
            "#!/bin/sh\n: > '\(cliStartedFile.path)'\nexit 0\n",
            named: "agy",
            in: root
        )
        let fakeOpen = try writeExecutable(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '\(callbackLog.path)'\n",
            named: "open",
            in: root
        )
        let fakeProcessState = try writeExecutable(
            """
            #!/bin/sh
            case " $* " in
              *" -g "*) exit 42 ;;
              *" -p "*)
                [ "$#" -eq 4 ] && [ "$1" = "-o" ] && [ "$2" = "stat=" ] && [ "$3" = "-p" ] || exit 42
                case "$4" in ''|*[!0-9]*) exit 42 ;; esac
                exec /bin/ps "$@"
                ;;
              *) exit 42 ;;
            esac
            """,
            named: "fake-ps",
            in: root
        )
        let watchdog = try writeTrackedPerlWatchdog(pidFile: watchdogPIDFile, in: root)
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: executable,
            completionTokenPipeURL: tokenPipe,
            completionStatusFileURL: statusFile,
            completionOpenExecutableURL: fakeOpen,
            watchdogExecutableURL: watchdog,
            processStateExecutableURL: fakeProcessState,
            loginTimeoutSeconds: 1
        )
        let scriptURL = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)

        let result = try runIsolatedPTYLogin(
            scriptURL: scriptURL,
            tokenPipe: tokenPipe,
            cliPIDFile: cliPIDFile,
            watchdogPIDFile: untrackedWatchdogPIDFile,
            inputAfterOutput: "handoff received.",
            cliStartRequired: false,
            watchdogStartRequired: false,
            in: root
        )

        let finalStatus = try String(contentsOf: statusFile, encoding: .utf8)
        let expectedStatus = "process_state_unavailable\n"
        if !result.cleanupSucceeded || finalStatus != expectedStatus {
            preserveTemporaryDirectory(
                root,
                reason: "process-group failure fixture did not reach expected state: \(result.terminalOutput)"
            )
        }
        #expect(result.cleanupSucceeded)
        #expect(finalStatus == expectedStatus, "PTY transcript: \(result.terminalOutput)")
        #expect(FileManager.default.fileExists(atPath: cliStartedFile.path))
        #expect(FileManager.default.fileExists(atPath: statusFile.appendingPathExtension("guardian").path))
        #expect(!FileManager.default.fileExists(atPath: callbackLog.path))
    }

    @Test("Antigravity app cancellation acknowledges only confirmed helper cleanup")
    func geminiAppCancellationDoesNotAcknowledgeUncertainCleanup() throws {
        let root = temporaryDirectory()
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: root.appendingPathComponent("agy"),
            completionTokenPipeURL: root.appendingPathComponent("handoff.token"),
            completionStatusFileURL: root.appendingPathComponent("handoff.status")
        )

        #expect(script.contains("if stop_owned_processes; then"))
        #expect(script.contains("report_status helper_cancelled"))
        #expect(script.contains("report_status helper_stop_unconfirmed"))
        #expect(script.contains("if (( !guardian_state_preserved )); then"))
    }

    @Test("Antigravity publishes its process group only after the child has stopped")
    func geminiPublishesProcessGroupAfterChildStop() throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }

        let tokenPipe = root.appendingPathComponent("completion.token")
        let cliPIDFile = root.appendingPathComponent("agy.pid")
        let cliStarted = root.appendingPathComponent("agy-started")
        let watchdogPIDFile = root.appendingPathComponent("watchdog.pid")
        let stopAttempt = root.appendingPathComponent("child-stop-attempt")
        let stopRelease = root.appendingPathComponent("child-stop-release")
        let stopAction = root.appendingPathComponent("child-stop-action")
        let processGroupFile = tokenPipe.appendingPathExtension("status")
            .appendingPathExtension("process-group")

        try createTokenPipe(at: tokenPipe)
        let executable = try writeExecutable(
            "#!/bin/sh\nprintf '%s' \"$$\" > '\(cliPIDFile.path)'\nprintf started > '\(cliStarted.path)'\nexit 0\n",
            named: "agy",
            in: root
        )
        let trackedWatchdog = try writeTrackedPerlWatchdog(pidFile: watchdogPIDFile, in: root)
        let perlGateModule = #"""
        package AIMeterSyntheticStopGate;
        use strict;
        use warnings;
        use Time::HiRes qw(time);
        BEGIN {
          *CORE::GLOBAL::kill = sub {
            my ($signal, @targets) = @_;
            if (defined($signal) && $signal eq "STOP" && @targets == 1 && $targets[0] == $$) {
              open my $attempt, ">", $ENV{AI_METER_TEST_STOP_ATTEMPT} or die "stop attempt marker: $!";
              print {$attempt} "$$\n" or die "stop attempt marker write: $!";
              close $attempt or die "stop attempt marker close: $!";
              print STDERR "Synthetic child reached the pre-STOP gate.\n";
              my $deadline = time + 12;
              while (!-e $ENV{AI_METER_TEST_STOP_RELEASE} && time < $deadline) {
                select undef, undef, undef, 0.01;
              }
              my $action = "";
              if (open my $handle, "<", $ENV{AI_METER_TEST_STOP_ACTION}) {
                $action = <$handle> // "";
                close $handle;
                chomp $action;
              }
              return 1 if $action ne "perform-stop";
            }
            return CORE::kill($signal, @targets);
          };
        }
        1;
        """#
        try Data(perlGateModule.utf8).write(to: root.appendingPathComponent("AIMeterSyntheticStopGate.pm"))

        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: executable,
            completionTokenPipeURL: tokenPipe,
            completionOpenExecutableURL: URL(fileURLWithPath: "/usr/bin/false"),
            watchdogExecutableURL: trackedWatchdog,
            loginTimeoutSeconds: 5
        )
        let scriptURL = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)

        var processGroupWasPublishedBeforeStop: Bool?
        let result = try runIsolatedPTYLogin(
            scriptURL: scriptURL,
            tokenPipe: tokenPipe,
            cliPIDFile: cliPIDFile,
            watchdogPIDFile: watchdogPIDFile,
            inputAfterOutput: "Synthetic child reached the pre-STOP gate.",
            onMarker: { _ in
                defer { try? Data().write(to: stopRelease, options: .atomic) }
                let published = FileManager.default.fileExists(atPath: processGroupFile.path)
                processGroupWasPublishedBeforeStop = published
                try Data((published ? "skip-stop\n" : "perform-stop\n").utf8)
                    .write(to: stopAction, options: .atomic)
            },
            additionalEnvironment: [
                "PERL5LIB": root.path,
                "PERL5OPT": "-MAIMeterSyntheticStopGate",
                "AI_METER_TEST_STOP_ATTEMPT": stopAttempt.path,
                "AI_METER_TEST_STOP_RELEASE": stopRelease.path,
                "AI_METER_TEST_STOP_ACTION": stopAction.path,
            ],
            in: root
        )

        #expect(processGroupWasPublishedBeforeStop == false)
        #expect(result.cleanupSucceeded)
        #expect(FileManager.default.fileExists(atPath: stopAttempt.path))
        #expect(FileManager.default.fileExists(atPath: cliStarted.path))
    }

    @Test("Antigravity helper does not start its CLI when cancellation arrives before watchdog readiness")
    func geminiCancellationBeforeWatchdogReadyPreventsCLIStart() throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let tokenPipe = root.appendingPathComponent("completion.token")
        let statusFile = tokenPipe.appendingPathExtension("status")
        try createTokenPipe(at: tokenPipe)
        let requestFile = root.appendingPathComponent("cancel request's.channel")
        #expect(FileManager.default.fileExists(atPath: root.path))
        try Data("pending\n".utf8).write(to: requestFile)
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: requestFile.path)
        let cliStarted = root.appendingPathComponent("agy-started")
        let watcherStarted = root.appendingPathComponent("watchdog-started")
        let executable = try writeExecutable(
            "#!/bin/sh\nprintf started > '\(cliStarted.path)'\nexit 0\n",
            named: "agy",
            in: root
        )
        let fakeWatcher = try writeExecutable(
            "#!/bin/sh\nprintf started > '\(watcherStarted.path)'\nwhile [ \"$(cat \"$6\")\" != cancelled ]; do /bin/sleep 0.01; done\n/bin/kill -USR2 \"$3\"\n/bin/sleep 0.05\n/usr/bin/touch \"$7\"\n",
            named: "watchdog",
            in: root
        )
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: executable,
            completionTokenPipeURL: tokenPipe,
            completionStatusFileURL: statusFile,
            cancellationRequestFileURL: requestFile,
            completionOpenExecutableURL: URL(fileURLWithPath: "/usr/bin/false"),
            watchdogExecutableURL: fakeWatcher,
            loginTimeoutSeconds: 10
        )
        let scriptURL = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)
        let process = isolatedProcess(executable: URL(fileURLWithPath: "/bin/zsh"), arguments: [scriptURL.path], in: root)
        try process.run()
        try writeToken("12345678-1234-1234-1234-123456789abc", to: tokenPipe)

        let watcherDeadline = Date().addingTimeInterval(5)
        while !FileManager.default.fileExists(atPath: watcherStarted.path), Date() < watcherDeadline {
            Thread.sleep(forTimeInterval: 0.01)
        }
        #expect(FileManager.default.fileExists(atPath: watcherStarted.path))
        #expect((try FileManager.default.attributesOfItem(atPath: requestFile.path)[.posixPermissions] as? NSNumber)?.intValue == 0o600)
        try Data("cancelled\n".utf8).write(to: requestFile, options: .atomic)
        process.waitUntilExit()

        #expect(process.terminationStatus == 130)
        #expect(!FileManager.default.fileExists(atPath: cliStarted.path))
        #expect(try String(contentsOf: statusFile, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines) == "helper_cancelled")
    }

    @Test("Antigravity cancellation wins the launch gate before CLI exec")
    func geminiCancellationWinsContendedLaunchGate() throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let tokenPipe = root.appendingPathComponent("completion.token")
        let statusFile = tokenPipe.appendingPathExtension("status")
        let launchGate = statusFile.appendingPathExtension("lock")
        let cliStarted = root.appendingPathComponent("agy-started")
        let watchdogPIDFile = root.appendingPathComponent("watchdog.pid")
        let gateLocked = root.appendingPathComponent("gate-locked")
        let releaseGate = root.appendingPathComponent("release-gate")
        try createTokenPipe(at: tokenPipe)
        let gateHolder = isolatedProcess(
            executable: URL(fileURLWithPath: "/usr/bin/perl"),
            arguments: ["-e", "use Fcntl qw(:DEFAULT); my ($path, $locked, $release) = @ARGV; open my $gate, '+<', $path or die $!; my $record_lock = pack('qqiss', 0, 0, 0, F_WRLCK, SEEK_SET); defined(fcntl($gate, F_SETLKW, $record_lock)) or die $!; open my $ready, '>', $locked or die $!; close $ready; sleep 0.01 until -e $release; my $unlock = pack('qqiss', 0, 0, 0, F_UNLCK, SEEK_SET); defined(fcntl($gate, F_SETLK, $unlock)) or die $!;", launchGate.path, gateLocked.path, releaseGate.path],
            in: root
        )
        try gateHolder.run()
        defer {
            try? Data().write(to: releaseGate)
            if gateHolder.isRunning { gateHolder.terminate() }
            gateHolder.waitUntilExit()
        }
        let holderDeadline = Date().addingTimeInterval(3)
        while !FileManager.default.fileExists(atPath: gateLocked.path), Date() < holderDeadline {
            Thread.sleep(forTimeInterval: 0.01)
        }
        #expect(FileManager.default.fileExists(atPath: gateLocked.path))

        let executable = try writeExecutable(
            "#!/bin/sh\nprintf started > '\(cliStarted.path)'\nexit 0\n",
            named: "agy",
            in: root
        )
        let trackedWatchdog = try writeTrackedPerlWatchdog(pidFile: watchdogPIDFile, in: root)
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: executable,
            completionTokenPipeURL: tokenPipe,
            completionStatusFileURL: statusFile,
            completionOpenExecutableURL: URL(fileURLWithPath: "/usr/bin/false"),
            watchdogExecutableURL: trackedWatchdog,
            loginTimeoutSeconds: 5
        )
        let scriptURL = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)
        let result = try runIsolatedPTYLogin(
            scriptURL: scriptURL,
            tokenPipe: tokenPipe,
            cliPIDFile: root.appendingPathComponent("unused-cli.pid"),
            watchdogPIDFile: watchdogPIDFile,
            inputAfterOutput: "handoff received.",
            cliStartRequired: false,
            onMarker: { _ in
                try Data("cancelled\n".utf8).write(to: statusFile.appendingPathExtension("cancel"), options: .atomic)
                try Data().write(to: releaseGate)
                gateHolder.waitUntilExit()
            },
            in: root
        )
        #expect(result.cleanupSucceeded)
        #expect(result.terminationStatus == 130)
        #expect(!FileManager.default.fileExists(atPath: cliStarted.path))
    }

    @Test("A supervisor blocked by the launch gate exits on cancellation before the holder is released")
    func blockedSupervisorCancellationReapsBeforeGateRelease() throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let tokenPipe = root.appendingPathComponent("completion.token")
        try createTokenPipe(at: tokenPipe)
        let status = tokenPipe.appendingPathExtension("status")
        let locked = root.appendingPathComponent("gate-locked")
        let release = root.appendingPathComponent("release-gate")
        let holder = isolatedProcess(executable: URL(fileURLWithPath: "/usr/bin/perl"), arguments: [
            "-e", "use Fcntl qw(:DEFAULT); my ($p,$l,$r)=@ARGV; open my $g,'+<',$p or die $!; my $f=pack('qqiss',0,0,0,F_WRLCK,SEEK_SET); defined(fcntl($g,F_SETLKW,$f)) or die $!; open my $m,'>',$l or die $!; close $m; select undef,undef,undef,0.01 until -e $r;",
            status.appendingPathExtension("lock").path, locked.path, release.path
        ], in: root)
        try holder.run()
        defer { try? Data().write(to: release); if holder.isRunning { holder.terminate() }; holder.waitUntilExit() }
        let deadline = ContinuousClock.now.advanced(by: .seconds(3))
        while !FileManager.default.fileExists(atPath: locked.path), ContinuousClock.now < deadline { Thread.sleep(forTimeInterval: 0.01) }
        try #require(FileManager.default.fileExists(atPath: locked.path))
        let cliMarker = root.appendingPathComponent("cli-started")
        let cli = try writeExecutable("#!/bin/sh\ntouch '\(cliMarker.path)'\nexit 0\n", named: "agy", in: root)
        let watchdogPID = root.appendingPathComponent("watchdog.pid")
        let watchdog = try writeTrackedPerlWatchdog(pidFile: watchdogPID, in: root)
        let script = try CLIAuthenticationScriptBuilder().build(provider: .gemini, executableURL: cli,
            completionTokenPipeURL: tokenPipe, completionOpenExecutableURL: URL(fileURLWithPath: "/usr/bin/false"), watchdogExecutableURL: watchdog)
        let url = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: url)
        let result = try runIsolatedPTYLogin(
            scriptURL: url, tokenPipe: tokenPipe, cliPIDFile: root.appendingPathComponent("unused-cli.pid"), watchdogPIDFile: watchdogPID,
            inputAfterOutput: "handoff received.", cliStartRequired: false, supervisorStartRequired: true,
            onGuardianReady: { try Data("cancelled\n".utf8).write(to: status.appendingPathExtension("cancel"), options: .atomic) }, in: root
        )
        #expect(result.cleanupSucceeded)
        #expect(result.terminationStatus == 130)
        #expect(!FileManager.default.fileExists(atPath: release.path))
        #expect(!FileManager.default.fileExists(atPath: cliMarker.path))
        #expect(try String(contentsOf: status, encoding: .utf8) == "helper_cancelled\n")
    }

    @Test("Cancellation in the background spawn-to-PID gap cannot acknowledge an unreaped owner", arguments: [SIGUSR2, SIGHUP, SIGINT, SIGTERM])
    func spawnPublicationGapDefersFinalAcknowledgement(signal: Int32) throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let pipe = root.appendingPathComponent("completion.token")
        try createTokenPipe(at: pipe)
        let status = pipe.appendingPathExtension("status")
        let release = root.appendingPathComponent("release-publication")
        let deferred = root.appendingPathComponent("cancel-deferred")
        let cliMarker = root.appendingPathComponent("cli-executed")
        let cli = try writeExecutable("#!/bin/sh\ntouch '\(cliMarker.path)'\nexit 0\n", named: "agy", in: root)
        let watchdogPID = root.appendingPathComponent("watchdog.pid")
        let watchdog = try writeTrackedPerlWatchdog(pidFile: watchdogPID, in: root)
        let callback = root.appendingPathComponent("callback.log")
        let opener = try writeExecutable("#!/bin/sh\nprintf '%s\\n' \"$*\" > '\(callback.path)'\nexit 0\n", named: "open", in: root)
        var script = try CLIAuthenticationScriptBuilder().build(provider: .gemini, executableURL: cli, completionTokenPipeURL: pipe,
            completionOpenExecutableURL: opener, watchdogExecutableURL: watchdog)
        // Only the synthetic test script is instrumented. The live owner may
        // start, but the parent is held before claiming $!, forcing the trap gap.
        script = script.replacingOccurrences(of: "agy_pid=$!", with:
            "print -r -- publication-gap\nwhile [[ ! -e \(shellQuote(release.path)) ]]; do /bin/sleep 0.01; done\nagy_pid=$!")
        script = script.replacingOccurrences(of: "deferred_app_cancel=1", with:
            "deferred_app_cancel=1; print -r -- deferred >| \(shellQuote(deferred.path))")
        script = script.replacingOccurrences(of: "deferred_cancel_exit=\"$1\"", with:
            "deferred_cancel_exit=\"$1\"; print -r -- deferred >| \(shellQuote(deferred.path))")
        let url = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: url)
        defer { try? Data().write(to: release) }
        let result = try runIsolatedPTYLogin(
            scriptURL: url, tokenPipe: pipe, cliPIDFile: root.appendingPathComponent("unused-cli.pid"), watchdogPIDFile: watchdogPID,
            inputAfterOutput: "publication-gap", cliStartRequired: false, supervisorStartRequired: true,
            onMarker: { ownedWrapperPID in
                if signal == SIGUSR2 {
                    try Data("cancelled\n".utf8).write(to: status.appendingPathExtension("cancel"), options: .atomic)
                }
                _ = Darwin.kill(ownedWrapperPID, signal)
                let deadline = ContinuousClock.now.advanced(by: .seconds(2))
                while !FileManager.default.fileExists(atPath: deferred.path),
                      (try? String(contentsOf: status, encoding: .utf8)) != "helper_cancelled\n", ContinuousClock.now < deadline {
                    Thread.sleep(forTimeInterval: 0.01)
                }
                #expect((try? String(contentsOf: status, encoding: .utf8)) != "helper_cancelled\n")
                #expect(FileManager.default.fileExists(atPath: deferred.path))
                try Data().write(to: release)
            }, in: root
        )
        #expect(result.cleanupSucceeded)
        let expectedExit: Int32 = signal == SIGHUP ? 129 : signal == SIGTERM ? 143 : 130
        #expect(result.terminationStatus == expectedExit)
        #expect(!FileManager.default.fileExists(atPath: cliMarker.path))
        if signal == SIGUSR2 {
            #expect(try String(contentsOf: status, encoding: .utf8) == "helper_cancelled\n")
        } else {
            #expect(try String(contentsOf: status, encoding: .utf8) != "helper_cancelled\n")
            #expect(try String(contentsOf: callback, encoding: .utf8).contains("result=cancelled"))
        }
    }

    @Test("Antigravity launch timeout reaps a bootstrapper blocked on its launch gate")
    func geminiLaunchTimeoutReapsBlockedBootstrapper() throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let tokenPipe = root.appendingPathComponent("completion.token")
        let statusFile = tokenPipe.appendingPathExtension("status")
        let launchGate = statusFile.appendingPathExtension("lock")
        let cliStarted = root.appendingPathComponent("agy-started")
        let gateLocked = root.appendingPathComponent("gate-locked")
        let releaseGate = root.appendingPathComponent("release-gate")
        let watchdogPIDFile = root.appendingPathComponent("watchdog.pid")
        try createTokenPipe(at: tokenPipe)
        let gateHolder = isolatedProcess(
            executable: URL(fileURLWithPath: "/usr/bin/perl"),
            arguments: ["-e", "use Fcntl qw(:DEFAULT); my ($path, $locked, $release) = @ARGV; open my $gate, '+<', $path or die $!; my $record_lock = pack('qqiss', 0, 0, 0, F_WRLCK, SEEK_SET); defined(fcntl($gate, F_SETLKW, $record_lock)) or die $!; open my $ready, '>', $locked or die $!; close $ready; sleep 0.01 until -e $release; my $unlock = pack('qqiss', 0, 0, 0, F_UNLCK, SEEK_SET); defined(fcntl($gate, F_SETLK, $unlock)) or die $!;", launchGate.path, gateLocked.path, releaseGate.path],
            in: root
        )
        try gateHolder.run()
        defer {
            try? Data().write(to: releaseGate)
            if gateHolder.isRunning { gateHolder.terminate() }
            gateHolder.waitUntilExit()
        }
        let holderDeadline = Date().addingTimeInterval(3)
        while !FileManager.default.fileExists(atPath: gateLocked.path), Date() < holderDeadline {
            Thread.sleep(forTimeInterval: 0.01)
        }
        #expect(FileManager.default.fileExists(atPath: gateLocked.path))

        let executable = try writeExecutable("#!/bin/sh\nprintf started > '\(cliStarted.path)'\nexit 0\n", named: "agy", in: root)
        let fakeOpen = try writeExecutable("#!/bin/sh\nexit 0\n", named: "open", in: root)
        let trackedWatchdog = try writeTrackedPerlWatchdog(pidFile: watchdogPIDFile, in: root)
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: executable,
            completionTokenPipeURL: tokenPipe,
            completionOpenExecutableURL: fakeOpen,
            watchdogExecutableURL: trackedWatchdog,
            loginTimeoutSeconds: 1
        )
        let scriptURL = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)
        // The real watchdog's startup limit is 10 seconds. Keep the launch gate
        // held across that deadline to prove it never blocks forever on the gate.
        let result = try runIsolatedPTYLogin(
            scriptURL: scriptURL,
            tokenPipe: tokenPipe,
            cliPIDFile: root.appendingPathComponent("unused-cli.pid"),
            watchdogPIDFile: watchdogPIDFile,
            inputAfterOutput: "handoff received.",
            cliStartRequired: false,
            supervisorStartRequired: true,
            in: root
        )
        #expect(result.cleanupSucceeded)
        #expect(result.terminationStatus == 124)
        #expect(!FileManager.default.fileExists(atPath: releaseGate.path))
        try Data().write(to: releaseGate)
        gateHolder.waitUntilExit()
        Thread.sleep(forTimeInterval: 0.2)
        #expect(!FileManager.default.fileExists(atPath: cliStarted.path))
        #expect(!FileManager.default.fileExists(atPath: statusFile.path + ".process-group"))
    }

    @Test("Antigravity success receipt is emitted only after a zero-exit fake CLI")
    func geminiSuccessReceiptRequiresZeroExit() throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let callbackLog = root.appendingPathComponent("callbacks.txt")
        let tokenPipe = root.appendingPathComponent("completion.token")
        try createTokenPipe(at: tokenPipe)
        let executable = try writeExecutable("#!/bin/sh\nexit 0\n", named: "agy", in: root)
        let fakeOpen = try writeExecutable(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '\(callbackLog.path)'\n",
            named: "open",
            in: root
        )

        try runGeminiScript(executable: executable, fakeOpen: fakeOpen, timeoutSeconds: 8, tokenPipe: tokenPipe, in: root)

        let callback = try String(contentsOf: callbackLog, encoding: .utf8)
        #expect(callback.contains("token=12345678-1234-1234-1234-123456789abc"))
        #expect(callback.contains("result=success"))
        #expect(callback.contains("exit_code=0"))
        let status = try String(contentsOf: tokenPipe.appendingPathExtension("status.exec"), encoding: .utf8)
        #expect(status.trimmingCharacters(in: .whitespacesAndNewlines) == "exec_confirmed")
    }

    @Test("A failed PTY marker callback reaps its wrapper and preserves the isolated root")
    func ptyMarkerCallbackFailurePreservesRoot() throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let tokenPipe = root.appendingPathComponent("completion.token")
        try createTokenPipe(at: tokenPipe)
        let executable = try writeExecutable("#!/bin/sh\nexit 0\n", named: "agy", in: root)
        let watchdog = try writeTrackedPerlWatchdog(pidFile: root.appendingPathComponent("watchdog.pid"), in: root)
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: executable,
            completionTokenPipeURL: tokenPipe,
            completionStatusFileURL: tokenPipe.appendingPathExtension("status"),
            watchdogExecutableURL: watchdog,
            loginTimeoutSeconds: 5
        )
        let scriptURL = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)

        #expect(throws: NSError.self) {
            try runIsolatedPTYLogin(
                scriptURL: scriptURL,
                tokenPipe: tokenPipe,
                cliPIDFile: root.appendingPathComponent("cli.pid"),
                watchdogPIDFile: root.appendingPathComponent("watchdog.pid"),
                inputAfterOutput: "handoff received.",
                onMarker: { _ in throw NSError(domain: "PTYMarkerCallback", code: 1) },
                in: root
            )
        }

        #expect(FileManager.default.fileExists(atPath: root.path))
        let preservationReason = try String(
            contentsOf: root.appendingPathComponent(".preserve-for-inspection"),
            encoding: .utf8
        )
        #expect(preservationReason.contains("marker callback failed"))
    }

    @Test("Antigravity missing executable is reported before a failure receipt")
    func geminiExecFailureIsReportedInStatus() throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let tokenPipe = root.appendingPathComponent("completion.token")
        let statusFile = tokenPipe.appendingPathExtension("status")
        let watchdogPIDFile = root.appendingPathComponent("watchdog.pid")
        let callbackLog = root.appendingPathComponent("callbacks.txt")
        try createTokenPipe(at: tokenPipe)
        let fakeOpen = try writeExecutable(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '\(callbackLog.path)'\n",
            named: "open",
            in: root
        )
        let trackedWatchdog = try writeTrackedPerlWatchdog(pidFile: watchdogPIDFile, in: root)
        let missingExecutable = root.appendingPathComponent("missing-agy")
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: missingExecutable,
            completionTokenPipeURL: tokenPipe,
            completionStatusFileURL: statusFile,
            completionOpenExecutableURL: fakeOpen,
            watchdogExecutableURL: trackedWatchdog,
            loginTimeoutSeconds: 5
        )
        let scriptURL = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)

        let result = try runIsolatedPTYLogin(
            scriptURL: scriptURL,
            tokenPipe: tokenPipe,
            cliPIDFile: root.appendingPathComponent("unused-cli.pid"),
            watchdogPIDFile: watchdogPIDFile,
            inputAfterOutput: "handoff received.",
            cliStartRequired: false,
            in: root
        )

        #expect(result.cleanupSucceeded)
        #expect(result.terminationStatus == 127)
        #expect(try String(contentsOf: statusFile.appendingPathExtension("exec"), encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines) == "exec_failed")
        let callback = try String(contentsOf: callbackLog, encoding: .utf8)
        #expect(callback.contains("result=failure"))
        #expect(callback.contains("exit_code=127"))
        #expect(!callback.contains("result=success"))
    }

    @Test("Antigravity resumes only its stopped foreground child and preserves its exit code", arguments: [0, 7])
    func geminiResumesOnlyForegroundChildPreservingExitCode(exitCode: Int) throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let tokenPipe = root.appendingPathComponent("completion.token")
        let cliPIDFile = root.appendingPathComponent("agy.pid")
        let watchdogPIDFile = root.appendingPathComponent("watchdog.pid")
        let callbackLog = root.appendingPathComponent("callbacks.txt")
        let continueLog = root.appendingPathComponent("continue.txt")
        let groupFile = tokenPipe.appendingPathExtension("status").appendingPathExtension("process-group")
        try createTokenPipe(at: tokenPipe)
        let executable = try writeExecutable(
            "#!/bin/sh\nprintf '%s' \"$$\" > '\(cliPIDFile.path)'\nprintf 'resume-regression-ready\\n'\nIFS= read -r response </dev/tty || exit 74\n[ \"$response\" = approved ] || exit 75\nexit \(exitCode)\n",
            named: "agy", in: root
        )
        let fakeOpen = try writeExecutable(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '\(callbackLog.path)'\nexit 0\n",
            named: "open", in: root
        )
        let watchdog = try writeTrackedPerlWatchdog(pidFile: watchdogPIDFile, in: root)
        let observer = #"""
        package AIMeterSyntheticContinueObserver;
        use strict;
        use warnings;
        use POSIX ();
        BEGIN {
          *CORE::GLOBAL::kill = sub {
            my ($signal, @targets) = @_;
            if (defined($signal) && $signal eq "CONT") {
              my $foreground = POSIX::tcgetpgrp(0);
              defined($foreground) && $foreground > 0 or die "foreground observation failed";
              open my $handle, ">>", $ENV{AI_METER_TEST_CONTINUE_LOG} or die "continue log open failed";
              print {$handle} join(" ", $$, @targets, $foreground)."\n" or die "continue log write failed";
              close $handle or die "continue log close failed";
            }
            return CORE::kill($signal, @targets);
          };
        }
        1;
        """#
        try Data(observer.utf8).write(to: root.appendingPathComponent("AIMeterSyntheticContinueObserver.pm"))
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini, executableURL: executable,
            completionTokenPipeURL: tokenPipe, completionOpenExecutableURL: fakeOpen,
            watchdogExecutableURL: watchdog, loginTimeoutSeconds: 10
        )
        let scriptURL = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)
        var guardianPID: Int32?
        let result = try runIsolatedPTYLogin(
            scriptURL: scriptURL, tokenPipe: tokenPipe, cliPIDFile: cliPIDFile,
            watchdogPIDFile: watchdogPIDFile, inputAfterOutput: "resume-regression-ready",
            onMarker: { _ in
                guardianPID = Int32(try String(contentsOf: groupFile, encoding: .utf8)
                    .trimmingCharacters(in: .whitespacesAndNewlines))
            },
            additionalEnvironment: [
                "PERL5LIB": root.path, "PERL5OPT": "-MAIMeterSyntheticContinueObserver",
                "AI_METER_TEST_CONTINUE_LOG": continueLog.path,
            ],
            in: root
        )
        #expect(result.cleanupSucceeded, "PTY transcript: \(result.terminalOutput)")
        #expect(result.terminationStatus == Int32(exitCode), "PTY transcript: \(result.terminalOutput)")
        let guardian = try #require(guardianPID)
        let cli = try #require(Int32(try String(contentsOf: cliPIDFile, encoding: .utf8)
            .trimmingCharacters(in: .whitespacesAndNewlines)))
        let rows = try String(contentsOf: continueLog, encoding: .utf8).split(separator: "\n")
        #expect(rows.count == 1)
        let row = try #require(rows.first)
        let fields = row.split(separator: " ").compactMap { Int32($0) }
        #expect(fields == [guardian, cli, guardian])
        let callback = try String(contentsOf: callbackLog, encoding: .utf8)
        #expect(callback.contains(exitCode == 0 ? "result=success" : "result=failure"))
        #expect(callback.contains("exit_code=\(exitCode)"))
        #expect(!callback.contains("exit_code=19"))
    }

    @Test("Antigravity fails closed while waiting to resume its foreground child",
          arguments: ["query-failed", "foreground-missing", "continue-failed", "cancel-wait"])
    func geminiForegroundResumeFailuresDoNotExecuteCLI(failureMode: String) throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let tokenPipe = root.appendingPathComponent("completion.token")
        let statusFile = tokenPipe.appendingPathExtension("status")
        let cancellation = root.appendingPathComponent("cancel.request")
        let cliPIDFile = root.appendingPathComponent("agy.pid")
        let watchdogPIDFile = root.appendingPathComponent("watchdog.pid")
        let callbackLog = root.appendingPathComponent("callbacks.txt")
        let queryMarker = root.appendingPathComponent("foreground-query")
        let continueLog = root.appendingPathComponent("continue-attempts")
        try createTokenPipe(at: tokenPipe)
        let executable = try writeExecutable(
            "#!/bin/sh\nprintf '%s' \"$$\" > '\(cliPIDFile.path)'\nexit 0\n",
            named: "agy", in: root
        )
        let fakeOpen = try writeExecutable(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '\(callbackLog.path)'\nexit 0\n",
            named: "open", in: root
        )
        let watchdog = try writeTrackedPerlWatchdog(pidFile: watchdogPIDFile, in: root)
        let observer = #"""
        package AIMeterSyntheticForegroundFailure;
        use strict;
        use warnings;
        use POSIX ();
        use Errno qw(EIO ESRCH);
        BEGIN {
          my $real_query = \&POSIX::tcgetpgrp;
          my $cancel_written = 0;
          no warnings "redefine";
          *POSIX::tcgetpgrp = sub {
            my $mode = $ENV{AI_METER_TEST_FOREGROUND_FAILURE};
            if (!-e $ENV{AI_METER_TEST_FOREGROUND_QUERY}) {
              open my $handle, ">", $ENV{AI_METER_TEST_FOREGROUND_QUERY} or die "query marker open";
              print {$handle} "$$\n" or die "query marker write";
              close $handle or die "query marker close";
            }
            if ($mode eq "query-failed") { $! = EIO; return -1; }
            if ($mode eq "foreground-missing") { return $$ + 1; }
            if ($mode eq "cancel-wait") {
              if (!$cancel_written) {
                open my $handle, ">", $ENV{AI_METER_TEST_CANCEL_REQUEST} or die "cancel request open";
                print {$handle} "cancelled\n" or die "cancel request write";
                close $handle or die "cancel request close";
                $cancel_written = 1;
              }
              return $$ + 1;
            }
            return $real_query->(@_);
          };
          *CORE::GLOBAL::kill = sub {
            my ($signal, @targets) = @_;
            if (defined($signal) && $signal eq "CONT") {
              open my $handle, ">>", $ENV{AI_METER_TEST_CONTINUE_ATTEMPTS} or die "continue log open";
              print {$handle} join(" ", $$, @targets)."\n" or die "continue log write";
              close $handle or die "continue log close";
              if ($ENV{AI_METER_TEST_FOREGROUND_FAILURE} eq "continue-failed") {
                $! = ESRCH;
                return 0;
              }
            }
            return CORE::kill($signal, @targets);
          };
        }
        1;
        """#
        try Data(observer.utf8).write(to: root.appendingPathComponent("AIMeterSyntheticForegroundFailure.pm"))
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini, executableURL: executable,
            completionTokenPipeURL: tokenPipe, completionStatusFileURL: statusFile,
            cancellationRequestFileURL: cancellation, completionOpenExecutableURL: fakeOpen,
            watchdogExecutableURL: watchdog, loginTimeoutSeconds: 30
        )
        let scriptURL = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)
        let result = try runIsolatedPTYLogin(
            scriptURL: scriptURL, tokenPipe: tokenPipe, cliPIDFile: cliPIDFile,
            watchdogPIDFile: watchdogPIDFile, inputAfterOutput: "unused-foreground-failure-marker",
            cliStartRequired: false, supervisorStartRequired: true,
            additionalEnvironment: [
                "PERL5LIB": root.path, "PERL5OPT": "-MAIMeterSyntheticForegroundFailure",
                "AI_METER_TEST_FOREGROUND_FAILURE": failureMode,
                "AI_METER_TEST_FOREGROUND_QUERY": queryMarker.path,
                "AI_METER_TEST_CANCEL_REQUEST": cancellation.path,
                "AI_METER_TEST_CONTINUE_ATTEMPTS": continueLog.path,
            ],
            in: root
        )
        #expect(result.cleanupSucceeded, "PTY transcript: \(result.terminalOutput)")
        let wrapperStatus = try #require(result.terminationStatus)
        #expect(wrapperStatus != 0)
        #expect(FileManager.default.fileExists(atPath: queryMarker.path))
        #expect(!FileManager.default.fileExists(atPath: cliPIDFile.path))
        let attempts = (try? String(contentsOf: continueLog, encoding: .utf8)) ?? ""
        #expect(attempts.split(separator: "\n").count == (failureMode == "continue-failed" ? 1 : 0))
        let callback = (try? String(contentsOf: callbackLog, encoding: .utf8)) ?? ""
        #expect(!callback.contains("result=success"))
        let finalStatus = try String(contentsOf: statusFile, encoding: .utf8)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        if failureMode == "cancel-wait" {
            #expect(["helper_cancelled", "helper_stop_unconfirmed"].contains(finalStatus))
        } else {
            #expect(wrapperStatus == 127, "PTY transcript: \(result.terminalOutput)")
            #expect(finalStatus == "helper_stop_unconfirmed")
        }
    }

    @Test("Antigravity interactive sign-in CLI runs with terminal input and can receive a line")
    func geminiInteractiveCLIReceivesTerminalInput() throws {
        let root = temporaryDirectory()
        var mayRemoveTemporaryDirectory = false
        defer {
            if mayRemoveTemporaryDirectory {
                try? FileManager.default.removeItem(at: root)
            }
        }
        let observation = root.appendingPathComponent("interactive-cli.txt")
        let callbackLog = root.appendingPathComponent("callbacks.txt")
        let cliPIDFile = root.appendingPathComponent("fake-agy.pid")
        let watchdogPIDFile = root.appendingPathComponent("watchdog.pid")
        let tokenPipe = root.appendingPathComponent("completion.token")
        let statusFile = tokenPipe.appendingPathExtension("status")
        try createTokenPipe(at: tokenPipe)
        let executable = try writeExecutable(
            "#!/bin/zsh\ntty0=no; [[ -t 0 ]] && tty0=yes\ntty1=no; [[ -t 1 ]] && tty1=yes\ntty2=no; [[ -t 2 ]] && tty2=yes\nprint -r -- \"$$\" > '\(cliPIDFile.path)'\nprint -r -- \"tty0=$tty0 tty1=$tty1 tty2=$tty2\" > '\(observation.path)'\n/bin/ps -o pid=,ppid=,pgid=,tpgid=,stat= -p $$ >> '\(observation.path)'\nprint -r -- ready\nif [[ -t 0 && -t 1 && -t 2 ]]; then print -r -- tty >> '\(observation.path)'; else exit 73; fi\nprint -r -- ready >> '\(observation.path)'\nIFS= read -r response || exit 74\nprint -r -- \"input:$response\" >> '\(observation.path)'\nprint -r -- fake-exit\nexit 0\n",
            named: "agy",
            in: root
        )
        let fakeOpen = try writeExecutable(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '\(callbackLog.path)'\n",
            named: "open",
            in: root
        )
        let watchdog = try writeTrackedPerlWatchdog(pidFile: watchdogPIDFile, in: root)
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: executable,
            completionTokenPipeURL: tokenPipe,
            completionStatusFileURL: statusFile,
            completionOpenExecutableURL: fakeOpen,
            watchdogExecutableURL: watchdog,
            loginTimeoutSeconds: 10
        )
        let scriptURL = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)

        let result = try runIsolatedPTYLogin(
            scriptURL: scriptURL,
            tokenPipe: tokenPipe,
            cliPIDFile: cliPIDFile,
            watchdogPIDFile: watchdogPIDFile,
            inputAfterOutput: "ready",
            in: root
        )
        try #require(result.cleanupSucceeded)
        mayRemoveTemporaryDirectory = true

        let cliResult = try String(contentsOf: observation, encoding: .utf8)
        let callback = try String(contentsOf: callbackLog, encoding: .utf8)
        #expect(cliResult.contains("tty\n"))
        #expect(cliResult.contains("input:approved"))
        #expect(callback.contains("result=success"))
    }

    @Test("Antigravity interactive sign-in CLI can read its controlling terminal")
    func geminiInteractiveCLIReadsControllingTerminal() throws {
        let root = temporaryDirectory()
        var mayRemoveTemporaryDirectory = false
        defer {
            if mayRemoveTemporaryDirectory {
                try? FileManager.default.removeItem(at: root)
            }
        }
        let observation = root.appendingPathComponent("controlling-terminal.txt")
        let callbackLog = root.appendingPathComponent("callbacks.txt")
        let cliPIDFile = root.appendingPathComponent("fake-agy.pid")
        let watchdogPIDFile = root.appendingPathComponent("watchdog.pid")
        let tokenPipe = root.appendingPathComponent("completion.token")
        let statusFile = tokenPipe.appendingPathExtension("status")
        try createTokenPipe(at: tokenPipe)
        let executable = try writeExecutable(
            "#!/bin/zsh\nprint -r -- \"$$\" > '\(cliPIDFile.path)'\nprint -r -- started > '\(observation.path)'\n/bin/ps -o pid=,ppid=,pgid=,tpgid=,stat= -p $$ >> '\(observation.path)'\nprint -r -- started\nIFS= read -r response < /dev/tty || exit 74\nprint -r -- \"input:$response\" >> '\(observation.path)'\nprint -r -- fake-exit\nexit 0\n",
            named: "agy",
            in: root
        )
        let fakeOpen = try writeExecutable(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '\(callbackLog.path)'\n",
            named: "open",
            in: root
        )
        let watchdog = try writeTrackedPerlWatchdog(pidFile: watchdogPIDFile, in: root)
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: executable,
            completionTokenPipeURL: tokenPipe,
            completionStatusFileURL: statusFile,
            completionOpenExecutableURL: fakeOpen,
            watchdogExecutableURL: watchdog,
            loginTimeoutSeconds: 10
        )
        let scriptURL = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)

        let result = try runIsolatedPTYLogin(
            scriptURL: scriptURL,
            tokenPipe: tokenPipe,
            cliPIDFile: cliPIDFile,
            watchdogPIDFile: watchdogPIDFile,
            inputAfterOutput: "started",
            in: root
        )
        try #require(result.cleanupSucceeded)
        mayRemoveTemporaryDirectory = true

        let cliResult = try String(contentsOf: observation, encoding: .utf8)
        let callback = try String(contentsOf: callbackLog, encoding: .utf8)
        #expect(cliResult.contains("input:approved"))
        #expect(callback.contains("result=success"))
    }

    @Test("Antigravity nonzero login exit sends a failure receipt, never a success receipt")
    func geminiNonzeroExitIsFailure() throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let callbackLog = root.appendingPathComponent("callbacks.txt")
        let tokenPipe = root.appendingPathComponent("completion.token")
        try createTokenPipe(at: tokenPipe)
        let executable = try writeExecutable("#!/bin/sh\nexit 7\n", named: "agy", in: root)
        let fakeOpen = try writeExecutable(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '\(callbackLog.path)'\n",
            named: "open",
            in: root
        )

        try runGeminiScript(executable: executable, fakeOpen: fakeOpen, timeoutSeconds: 8, tokenPipe: tokenPipe, in: root)

        let callback = try String(contentsOf: callbackLog, encoding: .utf8)
        #expect(callback.contains("result=failure"))
        #expect(callback.contains("exit_code=7"))
        #expect(!callback.contains("result=success"))
    }

    @Test("Antigravity login watchdog terminates its fake CLI and reports timeout")
    func geminiLoginWatchdogIsBounded() throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let callbackLog = root.appendingPathComponent("callbacks.txt")
        let tokenPipe = root.appendingPathComponent("completion.token")
        try createTokenPipe(at: tokenPipe)
        let executable = try writeExecutable("#!/bin/sh\nsleep 8\n", named: "agy", in: root)
        let fakeOpen = try writeExecutable(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '\(callbackLog.path)'\n",
            named: "open",
            in: root
        )

        let startedAt = Date()
        try runGeminiScript(executable: executable, fakeOpen: fakeOpen, timeoutSeconds: 4, tokenPipe: tokenPipe, in: root)
        let elapsed = Date().timeIntervalSince(startedAt)

        let callback = try String(contentsOf: callbackLog, encoding: .utf8)
        #expect(elapsed < 7)
        #expect(callback.contains("result=timeout"))
        #expect(!callback.contains("result=success"))
    }

    @Test("Antigravity watchdog kills a CLI process group that ignores TERM")
    func geminiWatchdogKillsTermResistantProcessGroup() throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let callbackLog = root.appendingPathComponent("callbacks.txt")
        let childPIDFile = root.appendingPathComponent("agy.pid")
        let tokenPipe = root.appendingPathComponent("completion.token")
        try createTokenPipe(at: tokenPipe)
        let executable = try writeExecutable(
            "#!/bin/sh\ntrap '' TERM\nprintf '%s' \"$$\" > '\(childPIDFile.path)'\nwhile :; do /bin/sleep 1; done\n",
            named: "agy",
            in: root
        )
        let fakeOpen = try writeExecutable(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '\(callbackLog.path)'\n",
            named: "open",
            in: root
        )

        let startedAt = Date()
        try runGeminiScript(executable: executable, fakeOpen: fakeOpen, timeoutSeconds: 1, tokenPipe: tokenPipe, in: root)
        let elapsed = Date().timeIntervalSince(startedAt)

        let childPIDText = try String(contentsOf: childPIDFile, encoding: .utf8)
        let childPID = try #require(pid_t(childPIDText))
        let callback = try String(contentsOf: callbackLog, encoding: .utf8)
        #expect(elapsed < 4)
        #expect(callback.contains("result=timeout"))
        #expect(kill(childPID, 0) != 0)
    }

    @Test("Antigravity watchdog cleans descendants after the CLI leader exits")
    func geminiWatchdogCleansDescendantsAfterLeaderExit() throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let callbackLog = root.appendingPathComponent("callbacks.txt")
        let childPIDFile = root.appendingPathComponent("agy-child.pid")
        let leaderOwnership = root.appendingPathComponent("agy-leader-ownership.txt")
        let tokenPipe = root.appendingPathComponent("completion.token")
        try createTokenPipe(at: tokenPipe)
        let executable = try writeExecutable(
            "#!/bin/sh\nprintf '%s %s ' \"$$\" \"$PPID\" > '\(leaderOwnership.path)'\n/bin/ps -o pgid= -p \"$$\" | /usr/bin/tr -d ' ' >> '\(leaderOwnership.path)'\n/bin/sh -c 'trap \"\" TERM; while :; do /bin/sleep 1; done' &\nprintf '%s' \"$!\" > '\(childPIDFile.path)'\nexit 0\n",
            named: "agy",
            in: root
        )
        let fakeOpen = try writeExecutable(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '\(callbackLog.path)'\n",
            named: "open",
            in: root
        )

        let startedAt = Date()
        try runGeminiScript(
            executable: executable,
            fakeOpen: fakeOpen,
            timeoutSeconds: 2,
            tokenPipe: tokenPipe,
            additionalTrackedProcessIDFiles: [childPIDFile],
            in: root
        )
        let elapsed = Date().timeIntervalSince(startedAt)
        let childPIDText = try String(contentsOf: childPIDFile, encoding: .utf8)
        let childPID = try #require(pid_t(childPIDText))
        let ownershipFields = try String(contentsOf: leaderOwnership, encoding: .utf8)
            .split(whereSeparator: \.isWhitespace)
        let leaderPID = try #require(ownershipFields.first.flatMap { pid_t($0) })
        let guardianPID = try #require(ownershipFields.dropFirst().first.flatMap { pid_t($0) })
        let leaderProcessGroupID = try #require(ownershipFields.dropFirst(2).first.flatMap { pid_t($0) })
        let callback = try String(contentsOf: callbackLog, encoding: .utf8)

        #expect(elapsed < 5)
        #expect(callback.contains("result=timeout"))
        #expect(!isRunning(childPID, in: root))
        #expect(leaderPID != guardianPID)
        #expect(guardianPID == leaderProcessGroupID)
        #expect(guardianPID != leaderPID)
    }

    @Test("Closing the login terminal cancels only its own process group and sends a failure receipt")
    func geminiLoginCancellationCleansOwnedProcessGroup() throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let callbackLog = root.appendingPathComponent("callbacks.txt")
        let childPIDFile = root.appendingPathComponent("agy.pid")
        let watchdogPIDFile = root.appendingPathComponent("watchdog.pid")
        let tokenPipe = root.appendingPathComponent("completion.token")
        try createTokenPipe(at: tokenPipe)
        let executable = try writeExecutable(
            "#!/bin/sh\nprintf '%s' \"$$\" > '\(childPIDFile.path)'\nexec /bin/sleep 30\n",
            named: "agy",
            in: root
        )
        let fakeOpen = try writeExecutable(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '\(callbackLog.path)'\n",
            named: "open",
            in: root
        )
        let trackedWatchdog = try writeTrackedPerlWatchdog(pidFile: watchdogPIDFile, in: root)
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: executable,
            completionTokenPipeURL: tokenPipe,
            completionOpenExecutableURL: fakeOpen,
            watchdogExecutableURL: trackedWatchdog,
            loginTimeoutSeconds: 10
        )
        let scriptURL = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)
        let result = try runIsolatedPTYLogin(
            scriptURL: scriptURL,
            tokenPipe: tokenPipe,
            cliPIDFile: childPIDFile,
            watchdogPIDFile: watchdogPIDFile,
            inputAfterOutput: "handoff received.",
            signalChildAfterFileExists: childPIDFile,
            in: root
        )
        #expect(result.cleanupSucceeded)
        #expect(result.terminationStatus == 143)
        #expect(!FileManager.default.fileExists(atPath: scriptURL.path))
        #expect(!FileManager.default.fileExists(atPath: tokenPipe.path + ".status.process-group"))

        #expect(FileManager.default.fileExists(atPath: childPIDFile.path))
        let callback = try String(contentsOf: callbackLog, encoding: .utf8)
        #expect(callback.contains("result=cancelled"))
        #expect(callback.contains("exit_code=143"))
    }

    @Test("Antigravity token wait is bounded even when no app writer opens the FIFO")
    func geminiTokenWaitIsBoundedWithoutWriter() throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let tokenPipe = root.appendingPathComponent("completion.token")
        let startedFile = root.appendingPathComponent("agy-started")
        try createTokenPipe(at: tokenPipe)
        let executable = try writeExecutable(
            "#!/bin/sh\nprintf started > '\(startedFile.path)'\nexit 0\n",
            named: "agy",
            in: root
        )
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: executable,
            completionTokenPipeURL: tokenPipe,
            completionOpenExecutableURL: URL(fileURLWithPath: "/usr/bin/false"),
            loginTimeoutSeconds: 1
        )
        let scriptURL = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)
        let process = isolatedProcess(executable: URL(fileURLWithPath: "/bin/zsh"), arguments: [scriptURL.path], in: root)
        let startedAt = Date()
        try process.run()

        let deadline = startedAt.addingTimeInterval(2.5)
        while process.isRunning && Date() < deadline {
            Thread.sleep(forTimeInterval: 0.01)
        }
        let exitedBeforeDeadline = !process.isRunning
        if !exitedBeforeDeadline {
            process.terminate()
        }
        process.waitUntilExit()

        #expect(exitedBeforeDeadline)
        #expect(process.terminationReason == .exit)
        #expect(process.terminationStatus == 124)
        #expect(!FileManager.default.fileExists(atPath: startedFile.path))
        #expect(!FileManager.default.fileExists(atPath: tokenPipe.path))
    }

    @Test("Antigravity scripts require a completion token pipe")
    func requiresCompletionTokenPipe() {
        let builder = CLIAuthenticationScriptBuilder()
        #expect(throws: CLIAuthenticationScriptError.invalidCompletionTokenPipe) {
            try builder.build(provider: .gemini, executableURL: URL(fileURLWithPath: "/tmp/agy"))
        }
    }

    @Test("DeepSeek can never be routed to a CLI authentication script")
    func rejectsDeepSeek() {
        #expect(throws: CLIAuthenticationScriptError.unsupportedProvider) {
            try CLIAuthenticationScriptBuilder().build(
                provider: .deepSeek,
                executableURL: URL(fileURLWithPath: "/tmp/deepseek")
            )
        }
    }

    private func runGeminiScript(
        executable: URL,
        fakeOpen: URL,
        timeoutSeconds: Int,
        tokenPipe: URL,
        additionalTrackedProcessIDFiles: [URL] = [],
        in root: URL
    ) throws {
        let trackedCLIPIDFile = root.appendingPathComponent("generated-agy.pid")
        let trackedWatchdogPIDFile = root.appendingPathComponent("generated-watchdog.pid")
        let trackedExecutable = try writeExecutable(
            "#!/bin/sh\nprintf '%s' \"$$\" > '\(trackedCLIPIDFile.path)'\nexec \(shellQuote(executable.path)) \"$@\"\n",
            named: "tracked-agy",
            in: root
        )
        let trackedWatchdog = try writeTrackedPerlWatchdog(pidFile: trackedWatchdogPIDFile, in: root)
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: trackedExecutable,
            completionTokenPipeURL: tokenPipe,
            completionOpenExecutableURL: fakeOpen,
            watchdogExecutableURL: trackedWatchdog,
            loginTimeoutSeconds: timeoutSeconds
        )
        let scriptURL = root.appendingPathComponent("login.command")
        // Keep the generated shell behavior while routing URL opens to a local recorder.
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)
        let result = try runIsolatedPTYLogin(
            scriptURL: scriptURL,
            tokenPipe: tokenPipe,
            cliPIDFile: trackedCLIPIDFile,
            watchdogPIDFile: trackedWatchdogPIDFile,
            inputAfterOutput: "handoff received.",
            cliStartRequired: timeoutSeconds > 1,
            additionalTrackedProcessIDFiles: additionalTrackedProcessIDFiles,
            in: root
        )
        guard result.cleanupSucceeded else {
            throw NSError(
                domain: "CLIAuthenticationScriptBuilderTests",
                code: 1,
                userInfo: [NSLocalizedDescriptionKey: "Generated Antigravity login script did not clean up its owned synthetic processes.\n\(result.terminalOutput)"]
            )
        }
        #expect(!FileManager.default.fileExists(atPath: scriptURL.path))
    }

    @Test("Timeout cleanup rejects malformed and unrelated process group markers")
    func timeoutCleanupRequiresMatchingPositiveOwnedGroup() {
        #expect(ownedProcessGroupID(markerText: "0", childPIDText: "0") == nil)
        #expect(ownedProcessGroupID(markerText: "-1", childPIDText: "-1") == nil)
        #expect(ownedProcessGroupID(markerText: "not-a-pid", childPIDText: "not-a-pid") == nil)
        #expect(ownedProcessGroupID(markerText: "123", childPIDText: "456") == nil)
        #expect(ownedProcessGroupID(markerText: "123", childPIDText: "123") == 123)
    }

    @Test("Temporary cleanup preserves a root marked after an ownership-unknown failure")
    func cleanupPreservesMarkedTemporaryRoot() throws {
        let root = temporaryDirectory()
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        preserveTemporaryDirectory(root, reason: "synthetic child ownership unknown")

        removeTemporaryDirectoryUnlessPreserved(root)

        #expect(FileManager.default.fileExists(atPath: root.path))
        try FileManager.default.removeItem(at: root)
    }

    @Test("Temporary cleanup retains a root when preservation marker creation fails")
    func cleanupRetainsRootWhenMarkerCreationFails() throws {
        let root = temporaryDirectory()
        try Data("fixture file, not a directory".utf8).write(to: root)
        preserveTemporaryDirectory(root, reason: "synthetic child ownership unknown")

        removeTemporaryDirectoryUnlessPreserved(root)

        #expect(FileManager.default.fileExists(atPath: root.path))
        #expect(!FileManager.default.fileExists(atPath: root.appendingPathComponent(".preserve-for-inspection").path))
        try FileManager.default.removeItem(at: root)
    }

    @Test("Synthetic cleanup probes process-group liveness without signalling the group")
    func processGroupProbeOnlyChecksLiveness() throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        let leaderPIDFile = root.appendingPathComponent("leader.pid")
        try Data("\(getpgrp())\n".utf8).write(to: leaderPIDFile)

        let groupID = try #require(pid_t(try String(contentsOf: leaderPIDFile, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)))
        #expect(syntheticCLIProcessGroupHasMembers(groupID: groupID))
    }

    @Test("Fixture observers bound an owned stalled query and an inherited open stdout",
          arguments: [false, true])
    func fixtureObserverBoundsSyntheticStall(inheritedPipe: Bool) throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let started = root.appendingPathComponent("observer-started")
        // Perl forks only a short-lived synthetic writer in the inherited-pipe
        // case. The query helper may signal its own direct child, never that PID.
        let executable = try writeExecutable(
            "#!/usr/bin/perl\nopen my $f, '>', '\(started.path)' or die $!; close $f;\n"
                + (inheritedPipe ? "my $p = fork(); if (!$p) { select undef, undef, undef, 0.6; exit 0; } exit 0;\n" : "select undef, undef, undef, 30;\n"),
            named: "fixture-observer", in: root
        )
        let start = ContinuousClock.now
        let result = FixtureProcessQuery.run(executable: executable, arguments: [], in: root)
        #expect(FileManager.default.fileExists(atPath: started.path))
        #expect(result == nil)
        #expect(start.duration(to: ContinuousClock.now) < .seconds(2))
        #expect(FileManager.default.fileExists(atPath: root.appendingPathComponent(".preserve-for-inspection").path))
    }

    @Test("A normal observer does not inherit an unrelated stalled observer's writer")
    func independentObserverWriterLifetimes() async throws {
        let root = temporaryDirectory()
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        let prepared = root.appendingPathComponent("first-prepared")
        let siblingStarted = root.appendingPathComponent("sibling-started")
        let first = try writeExecutable("#!/bin/sh\nprintf owned-query\nexit 0\n", named: "first-query", in: root)
        let sibling = try writeExecutable("#!/usr/bin/perl\nopen my $f,'>','\(siblingStarted.path)' or die $!; close $f; select undef,undef,undef,30;\n", named: "sibling-query", in: root)
        let a = Task.detached {
            FixtureProcessQuery.run(executable: first, arguments: [], in: root, onPrepared: {
                try? Data().write(to: prepared)
                let deadline = ContinuousClock.now.advanced(by: .seconds(2))
                while !FileManager.default.fileExists(atPath: siblingStarted.path), ContinuousClock.now < deadline { Thread.sleep(forTimeInterval: 0.002) }
            })
        }
        let deadline = ContinuousClock.now.advanced(by: .seconds(2))
        while !FileManager.default.fileExists(atPath: prepared.path), ContinuousClock.now < deadline { await Task.yield() }
        try #require(FileManager.default.fileExists(atPath: prepared.path))
        let b = Task.detached { FixtureProcessQuery.run(executable: sibling, arguments: [], in: root, timeout: .seconds(1)) }
        let result = await a.value
        #expect(result?.exitStatus == 0)
        #expect(result?.text == "owned-query")
        #expect(await b.value == nil)
    }

    private func ownedProcessGroupID(markerText: String?, childPIDText: String?) -> pid_t? {
        guard let markerText,
              let markerPID = pid_t(markerText.trimmingCharacters(in: .whitespacesAndNewlines)),
              markerPID > 1,
              let childPIDText,
              let childPID = pid_t(childPIDText.trimmingCharacters(in: .whitespacesAndNewlines)),
              childPID > 1,
              markerPID == childPID else {
            return nil
        }
        return markerPID
    }

    private func processParentID(_ pid: pid_t, in root: URL) -> pid_t? {
        guard let output = FixtureProcessQuery.run(
            executable: URL(fileURLWithPath: "/bin/ps"), arguments: ["-o", "ppid=", "-p", String(pid)], in: root
        ), output.exitStatus == 0 else { preserveTemporaryDirectory(root, reason: "Parent query unknown"); return nil }
        return pid_t(output.text.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    private func shellQuote(_ value: String) -> String {
        "'" + value.replacingOccurrences(of: "'", with: "'\\''") + "'"
    }

    private func createTokenPipe(at url: URL) throws {
        try FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(),
            withIntermediateDirectories: true,
            attributes: [.posixPermissions: 0o700]
        )
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: url.deletingLastPathComponent().path)
        let result = url.path.withCString { Darwin.mkfifo($0, mode_t(S_IRUSR | S_IWUSR)) }
        #expect(result == 0)
        let statusURL = url.appendingPathExtension("status")
        try Data("ready\n".utf8).write(to: statusURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: statusURL.path)
        let requestURL = statusURL.appendingPathExtension("cancel")
        try Data("pending\n".utf8).write(to: requestURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: requestURL.path)
        let launchGateURL = statusURL.appendingPathExtension("lock")
        try Data().write(to: launchGateURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: launchGateURL.path)
    }

    private func writeToken(_ token: String, to url: URL) throws {
        let deadline = Date().addingTimeInterval(3)
        var descriptor: Int32 = -1
        while Date() < deadline {
            descriptor = url.path.withCString { Darwin.open($0, O_WRONLY | O_NONBLOCK) }
            if descriptor >= 0 { break }
            guard errno == ENXIO || errno == EINTR else {
                throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
            }
            Thread.sleep(forTimeInterval: 0.01)
        }
        guard descriptor >= 0 else {
            throw NSError(domain: NSPOSIXErrorDomain, code: Int(ETIMEDOUT))
        }
        defer { Darwin.close(descriptor) }
        let data = Data("\(token)\n".utf8)
        let written = data.withUnsafeBytes { bytes in
            Darwin.write(descriptor, bytes.baseAddress, bytes.count)
        }
        guard written == data.count else {
            throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno == 0 ? EIO : errno))
        }
    }

    private func writeExecutable(_ contents: String, named name: String, in root: URL) throws -> URL {
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: root.path)
        let url = root.appendingPathComponent(name)
        try Data(contents.utf8).write(to: url)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: url.path)
        return url
    }

    private func isRunning(_ pid: pid_t, in root: URL) -> Bool {
        guard kill(pid, 0) == 0 else {
            if errno == ESRCH { return false }
            preserveTemporaryDirectory(root, reason: "Child liveness unknown")
            return true
        }
        guard let output = FixtureProcessQuery.run(
            executable: URL(fileURLWithPath: "/bin/ps"), arguments: ["-o", "stat=", "-p", String(pid)], in: root
        ), output.exitStatus == 0 else { preserveTemporaryDirectory(root, reason: "Child query unknown"); return true }
        let state = output.text.trimmingCharacters(in: .whitespacesAndNewlines)
        return !state.isEmpty && !state.contains("Z")
    }

    private func temporaryDirectory() -> URL {
        FileManager.default.temporaryDirectory.appendingPathComponent(
            "antigravity-auth-script-\(UUID().uuidString)",
            isDirectory: true
        )
    }

    private func preserveTemporaryDirectory(_ root: URL, reason: String) {
        TemporaryRootRetention.shared.retain(root)
        do {
            try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
            let marker = root.appendingPathComponent(".preserve-for-inspection")
            try Data("\(reason)\n".utf8).write(to: marker, options: .atomic)
        } catch {
            // Cleanup must still fail closed if the evidence marker cannot be written.
            // Callers report cleanup failure and never signal an unverified process group.
        }
    }

    private func removeTemporaryDirectoryUnlessPreserved(_ root: URL) {
        let marker = root.appendingPathComponent(".preserve-for-inspection")
        guard !TemporaryRootRetention.shared.contains(root),
              !FileManager.default.fileExists(atPath: marker.path) else { return }
        try? FileManager.default.removeItem(at: root)
    }

    private func isolatedProcess(executable: URL, arguments: [String], in root: URL) -> Process {
        let process = Process()
        process.executableURL = executable
        process.arguments = arguments
        process.currentDirectoryURL = root
        process.environment = [
            "HOME": root.path,
            "ZDOTDIR": root.path,
            "TMPDIR": root.path,
            "PATH": "\(root.path):/usr/bin:/bin:/usr/sbin:/sbin",
            "LANG": "C",
            "LC_ALL": "C",
        ]
        return process
    }

    private struct IsolatedPTYLoginResult {
        let cleanupSucceeded: Bool
        let terminationStatus: Int32?
        var terminalOutput = ""
    }

    private func runIsolatedPTYLogin(
        scriptURL: URL,
        tokenPipe: URL,
        cliPIDFile: URL,
        watchdogPIDFile: URL,
        inputAfterOutput marker: String,
        cliStartRequired: Bool = true,
        supervisorStartRequired: Bool = false,
        watchdogStartRequired: Bool = true,
        additionalTrackedProcessIDFiles: [URL] = [],
        onMarker: ((pid_t) throws -> Void)? = nil,
        onGuardianReady: (() throws -> Void)? = nil,
        signalChildAfterFileExists: URL? = nil,
        additionalEnvironment: [String: String] = [:],
        in root: URL
    ) throws -> IsolatedPTYLoginResult {
        var arguments = [
            "/usr/bin/env", "-i",
            "HOME=\(root.path)",
            "ZDOTDIR=\(root.path)",
            "TMPDIR=\(root.path)",
            "PATH=\(root.path):/usr/bin:/bin:/usr/sbin:/sbin",
            "LANG=C", "LC_ALL=C",
            "/bin/zsh", scriptURL.path,
        ]
        arguments.insert(
            contentsOf: additionalEnvironment.keys.sorted().map { key in
                "\(key)=\(additionalEnvironment[key]!)"
            },
            at: 2
        )
        let argumentPointers = arguments.map { strdup($0)! } + [nil]
        defer { argumentPointers.forEach { free($0) } }
        let executable = strdup("/usr/bin/env")!
        defer { free(executable) }

        var masterDescriptor: Int32 = -1
        let childPID = argumentPointers.withUnsafeBufferPointer { argv in
            let pid = forkpty(&masterDescriptor, nil, nil, nil)
            if pid == 0 {
                execv(executable, argv.baseAddress!)
                Darwin._exit(127)
            }
            return pid
        }
        guard childPID > 0, masterDescriptor >= 0 else {
            throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
        }
        defer { Darwin.close(masterDescriptor) }

        let descriptorFlags = fcntl(masterDescriptor, F_GETFL)
        guard descriptorFlags >= 0,
              fcntl(masterDescriptor, F_SETFL, descriptorFlags | O_NONBLOCK) == 0 else {
            if !terminateAndReapPTYChild(childPID) {
                preserveTemporaryDirectory(root, reason: "PTY wrapper cleanup unknown after descriptor setup failure")
            }
            throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
        }

        do {
            try writeToken("12345678-1234-1234-1234-123456789abc", to: tokenPipe)
        } catch {
            if !terminateAndReapPTYChild(childPID) {
                preserveTemporaryDirectory(root, reason: "PTY wrapper cleanup unknown after token handoff failure")
            }
            throw error
        }
        let deadline = Date().addingTimeInterval(15)
        var output = Data()
        var processGroupID: pid_t?
        var guardianPID: pid_t?
        var guardianActionPerformed = false
        let guardianFile = tokenPipe.appendingPathExtension("status").appendingPathExtension("guardian")
        let guardianPIDFile = root.appendingPathComponent("fixture-supervisor.pid")
        let processGroupMarkerURL = tokenPipe.appendingPathExtension("status")
            .appendingPathExtension("process-group")
        var inputSent = false
        var childExited = false
        var childOwnershipUnknown = false
        var childSignalSent = false
        var markerActionPerformed = false
        var childStatus: Int32 = 0

        while Date() < deadline {
            if guardianPID == nil,
               let identity = try? String(contentsOf: guardianFile, encoding: .utf8) {
                let fields = identity.split(whereSeparator: \.isWhitespace)
                if fields.count == 2, let observed = pid_t(fields[0]), observed > 1,
                   fields[1].count == 32, fields[1].allSatisfy({ $0.isHexDigit }) {
                    guardianPID = observed
                    if processGroupID == nil { processGroupID = observed }
                    do { try Data("\(observed)\n".utf8).write(to: guardianPIDFile) } catch {
                        _ = terminateAndReapPTYChild(childPID)
                        preserveTemporaryDirectory(root, reason: "Early supervisor tracking could not be recorded")
                        throw error
                    }
                }
            }
            if guardianPID != nil, !guardianActionPerformed {
                do { try onGuardianReady?() } catch {
                    _ = terminateAndReapPTYChild(childPID)
                    preserveTemporaryDirectory(root, reason: "Guardian callback failed with owned descendants uncertain")
                    throw error
                }
                guardianActionPerformed = true
            }
            if processGroupID == nil,
               let text = try? String(contentsOf: processGroupMarkerURL, encoding: .utf8),
               let observedGroupID = pid_t(text.trimmingCharacters(in: .whitespacesAndNewlines)),
               observedGroupID > 1 {
                processGroupID = observedGroupID
            }
            var buffer = [UInt8](repeating: 0, count: 4096)
            let count = Darwin.read(masterDescriptor, &buffer, buffer.count)
            if count > 0 {
                output.append(contentsOf: buffer.prefix(count))
            } else if count < 0 && errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR && errno != EIO {
                break
            }

            if !markerActionPerformed, String(decoding: output, as: UTF8.self).contains(marker),
               !supervisorStartRequired || guardianPID != nil {
                do {
                    try onMarker?(childPID)
                } catch {
                    // The marker callback is test code, but it can fail after
                    // the PTY wrapper has started. Reap only our unreaped,
                    // directly-owned child and retain the root because other
                    // synthetic descendants may still have uncertain ownership.
                    let wrapperReaped = terminateAndReapPTYChild(childPID)
                    preserveTemporaryDirectory(
                        root,
                        reason: wrapperReaped
                            ? "PTY marker callback failed; isolated root retained for inspection"
                            : "PTY marker callback failed and wrapper cleanup ownership is unknown"
                    )
                    throw error
                }
                markerActionPerformed = true
                if !inputSent {
                    guard writePTYInput("approved\n", to: masterDescriptor) else { break }
                    inputSent = true
                }
            }

            if !childSignalSent,
               let signalChildAfterFileExists,
               FileManager.default.fileExists(atPath: signalChildAfterFileExists.path) {
                _ = Darwin.kill(childPID, SIGTERM)
                childSignalSent = true
            }

            switch observePTYChild(childPID, status: &childStatus) {
            case .exited:
                childExited = true
                break
            case .running:
                break
            case .ownershipUnknown:
                childOwnershipUnknown = true
                break
            }
            if childExited || childOwnershipUnknown { break }
            Thread.sleep(forTimeInterval: 0.01)
        }

        guard !childOwnershipUnknown else {
            preserveTemporaryDirectory(root, reason: "PTY wrapper ownership became unknown")
            return IsolatedPTYLoginResult(cleanupSucceeded: false, terminationStatus: nil)
        }
        if !childExited {
            // The PTY child is a direct unreaped child, so its PID cannot be
            // reused before waitpid. HUP closes this private controlling tty and
            // lets the generated shell run its normal cancellation cleanup.
            _ = Darwin.kill(childPID, SIGHUP)
            switch waitForPTYChild(childPID, status: &childStatus, timeout: 2) {
            case .exited:
                childExited = true
            case .running:
                break
            case .ownershipUnknown:
                preserveTemporaryDirectory(root, reason: "PTY wrapper ownership became unknown after HUP")
                return IsolatedPTYLoginResult(cleanupSucceeded: false, terminationStatus: nil)
            }
        }
        if !childExited {
            _ = Darwin.kill(childPID, SIGTERM)
            switch waitForPTYChild(childPID, status: &childStatus, timeout: 1) {
            case .exited:
                childExited = true
            case .running:
                break
            case .ownershipUnknown:
                preserveTemporaryDirectory(root, reason: "PTY wrapper ownership became unknown after TERM")
                return IsolatedPTYLoginResult(cleanupSucceeded: false, terminationStatus: nil)
            }
        }
        if !childExited {
            _ = Darwin.kill(childPID, SIGKILL)
            childExited = waitForPTYChild(childPID, status: &childStatus, timeout: 1) == .exited
        }
        guard childExited else {
            preserveTemporaryDirectory(root, reason: "PTY wrapper did not exit after bounded cleanup")
            return IsolatedPTYLoginResult(cleanupSucceeded: false, terminationStatus: nil)
        }

        let supervisorExited = trackedChildExited(pidFile: guardianPIDFile, required: supervisorStartRequired || cliStartRequired, in: root)
        let cliExited = trackedChildExited(pidFile: cliPIDFile, required: cliStartRequired, in: root)
        let watchdogExited = trackedChildExited(
            pidFile: watchdogPIDFile,
            required: watchdogStartRequired,
            in: root
        )
        let additionalChildrenExited = additionalTrackedProcessIDFiles.allSatisfy {
            trackedChildExited(pidFile: $0, in: root)
        }
        let cliGroupHasMembers = processGroupID.map(syntheticCLIProcessGroupHasMembers(groupID:))
        let cliGroupCheckIsKnown = cliGroupHasMembers != nil || !cliStartRequired
        guard supervisorExited && cliExited && watchdogExited && additionalChildrenExited
                && cliGroupCheckIsKnown && cliGroupHasMembers != true else {
            print("PTY_CLEANUP_UNKNOWN supervisor=\(supervisorExited) cli=\(cliExited) watchdog=\(watchdogExited) additional=\(additionalChildrenExited) groupKnown=\(cliGroupCheckIsKnown) groupHasMembers=\(String(describing: cliGroupHasMembers)) root=\(root.path)")
            preserveTemporaryDirectory(root, reason: "synthetic CLI, descendant, watchdog, or CLI process group remained live or had unknown identity")
            return IsolatedPTYLoginResult(
                cleanupSucceeded: false,
                terminationStatus: childStatus & 0x7f == 0 ? (childStatus >> 8) & 0xff : nil,
                terminalOutput: String(decoding: output, as: UTF8.self)
            )
        }
        return IsolatedPTYLoginResult(
            cleanupSucceeded: true,
            terminationStatus: childStatus & 0x7f == 0 ? (childStatus >> 8) & 0xff : nil,
            terminalOutput: String(decoding: output, as: UTF8.self)
        )
    }

    private func writePTYInput(_ input: String, to descriptor: Int32) -> Bool {
        let bytes = Array(input.utf8)
        let deadline = Date().addingTimeInterval(1)
        var written = 0
        while written < bytes.count && Date() < deadline {
            let count = bytes.withUnsafeBytes { buffer in
                Darwin.write(descriptor, buffer.baseAddress!.advanced(by: written), bytes.count - written)
            }
            if count > 0 {
                written += count
            } else if count < 0 && (errno == EAGAIN || errno == EWOULDBLOCK || errno == EINTR) {
                Thread.sleep(forTimeInterval: 0.01)
            } else {
                return false
            }
        }
        return written == bytes.count
    }

    private enum PTYChildObservation: Equatable {
        case exited
        case running
        case ownershipUnknown
    }

    private func observePTYChild(_ pid: pid_t, status: inout Int32) -> PTYChildObservation {
        let result = waitpid(pid, &status, WNOHANG)
        if result == pid { return .exited }
        if result == 0 { return .running }
        return errno == EINTR ? .running : .ownershipUnknown
    }

    private func waitForPTYChild(
        _ pid: pid_t,
        status: inout Int32,
        timeout: TimeInterval
    ) -> PTYChildObservation {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            let observation = observePTYChild(pid, status: &status)
            if observation != .running { return observation }
            Thread.sleep(forTimeInterval: 0.01)
        }
        return .running
    }

    private func terminateAndReapPTYChild(_ pid: pid_t) -> Bool {
        guard pid > 0 else { return false }
        var status: Int32 = 0
        switch observePTYChild(pid, status: &status) {
        case .exited: return true
        case .running: break
        case .ownershipUnknown: return false
        }
        _ = Darwin.kill(pid, SIGHUP)
        switch waitForPTYChild(pid, status: &status, timeout: 1) {
        case .exited: return true
        case .running: break
        case .ownershipUnknown: return false
        }
        _ = Darwin.kill(pid, SIGKILL)
        return waitForPTYChild(pid, status: &status, timeout: 1) == .exited
    }

    private func trackedChildExited(pidFile: URL, required: Bool = true, in root: URL) -> Bool {
        guard FileManager.default.fileExists(atPath: pidFile.path) else { return !required }
        guard let pidText = try? String(contentsOf: pidFile, encoding: .utf8),
              let pid = pid_t(pidText.trimmingCharacters(in: .whitespacesAndNewlines)),
              pid > 0 else { return false }
        let deadline = Date().addingTimeInterval(1)
        while Date() < deadline {
            switch observeTrackedChild(pid: pid, in: root) {
            case .gone:
                return true
            case .unknown:
                return false
            case .running:
                break
            }
            Thread.sleep(forTimeInterval: 0.02)
        }
        // Never signal a recorded numeric child PID: it could exit and that PID
        // could be reused. The shell's HUP path owns cleanup; if it leaves a
        // child behind, fail and preserve this test root.
        let finalObservation = observeTrackedChild(pid: pid, in: root)
        if finalObservation != .gone {
            print("TRACKED_CHILD_NOT_EXITED pid=\(pid) observation=\(finalObservation) file=\(pidFile.lastPathComponent)")
        }
        return finalObservation == .gone
    }

    private func syntheticCLIProcessGroupHasMembers(groupID: pid_t) -> Bool {
        guard groupID > 1 else { return true }
        // This ID is captured from the generated script's private process-group
        // marker while the PTY wrapper is alive; no group signal is sent here.
        let result = Darwin.kill(-groupID, 0)
        return result == 0 || errno != ESRCH
    }

    private func writeTrackedPerlWatchdog(pidFile: URL, in root: URL) throws -> URL {
        try writeExecutable(
            "#!/bin/sh\nprintf '%s' \"$$\" > '\(pidFile.path)'\nexec /usr/bin/perl \"$@\"\n",
            named: "tracked-watchdog",
            in: root
        )
    }

    private enum TrackedProcessObservation: Equatable {
        case gone
        case running
        case unknown
    }

    private func observeTrackedChild(pid: pid_t, in root: URL) -> TrackedProcessObservation {
        guard Darwin.kill(pid, 0) == 0 else {
            let probeErrno = errno
            if probeErrno != ESRCH { print("TRACKED_CHILD_PROBE_UNKNOWN pid=\(pid) errno=\(probeErrno)") }
            return probeErrno == ESRCH ? .gone : .unknown
        }
        let queryStarted = ContinuousClock.now
        let queryOutput = FixtureProcessQuery.run(
            executable: URL(fileURLWithPath: "/bin/ps"), arguments: ["-o", "stat=", "-p", String(pid)], in: root
        )
        guard let output = queryOutput, output.exitStatus == 0 else {
            print("TRACKED_CHILD_QUERY_UNKNOWN pid=\(pid) query=\(String(reflecting: queryOutput)) elapsed=\(queryStarted.duration(to: .now))")
            preserveTemporaryDirectory(root, reason: "Tracked child query unknown")
            return .unknown
        }
        let state = output.text.trimmingCharacters(in: .whitespacesAndNewlines)
        return state.isEmpty || state.contains("Z") ? .gone : .running
    }

    @Test("Antigravity cancellation requests cannot collide with coordinator response paths")
    func rejectsCollidingCancellationRequests() {
        let status = URL(fileURLWithPath: "/tmp/hand-off status's.status")
        for request in [status, status.appendingPathExtension("exec"), status.appendingPathExtension("lock"),
                        status.appendingPathExtension("ready"), status.appendingPathExtension("guardian"),
                        URL(fileURLWithPath: "/tmp/handoff.token")] {
            #expect(throws: CLIAuthenticationScriptError.invalidCompletionTokenPipe) {
                try CLIAuthenticationScriptBuilder().build(
                    provider: .gemini, executableURL: URL(fileURLWithPath: "/tmp/agy"),
                    completionTokenPipeURL: URL(fileURLWithPath: "/tmp/handoff.token"),
                    completionStatusFileURL: status, cancellationRequestFileURL: request
                )
            }
        }
    }


    @Test("Guardian ignores its reaped process query and still detects a live peer", arguments: [false, true])
    func geminiGuardianExcludesOnlyItsCompletedQuery(livePeer: Bool) throws {
        let root = FileManager.default.temporaryDirectory.resolvingSymlinksInPath()
            .appendingPathComponent("guardian-peer-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: false,
                                                 attributes: [.posixPermissions: 0o700])
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        do {
            let script = try CLIAuthenticationScriptBuilder().build(
                provider: .gemini, executableURL: root.appendingPathComponent("never-executed-cli"),
                completionTokenPipeURL: root.appendingPathComponent("never-opened-token")
            )
            let start = try #require(script.range(of: "sub group_peers {"))
            let end = try #require(script.range(of: "my $stop_requested = sub", range: start.upperBound..<script.endIndex))
            // Exercise the emitted guardian function and its real bounded /bin/ps
            // child. No CLI, wrapper, Terminal or process-group signal is used.
            let peers = String(script[start.lowerBound..<end.lowerBound])
            let program = #"""
            use strict; use warnings; use Fcntl qw(:DEFAULT O_NONBLOCK);
            use POSIX qw(getpgrp waitpid WNOHANG _exit); use Errno qw(EINTR EAGAIN EWOULDBLOCK);
            use Time::HiRes qw(clock_gettime CLOCK_MONOTONIC sleep);
            \#(CLIAuthenticationScriptBuilder.boundedProcessStateProbeSubroutine)
            if (getpgrp() != $$) { POSIX::setpgid(0, 0) == 0 or die "private process group unavailable"; }
            my $process_state_path = "/bin/ps";
            \#(peers)
            my $peer;
            if (\#(livePeer ? "1" : "0")) {
              pipe(my $ready_r, my $ready_w) or die $!;
              $peer = fork(); defined($peer) or die $!;
              if ($peer == 0) { close $ready_r; print {$ready_w} "ready\n"; close $ready_w; sleep 2; _exit(0); }
              close $ready_w; my $ready = <$ready_r>; close $ready_r; die "peer barrier" unless $ready eq "ready\n";
            }
            my $actual = group_peers();
            if (defined($peer)) { waitpid($peer, 0) == $peer or die "peer reap"; die "peer exit" if $?; }
            print "actual=", defined($actual) ? $actual : "unknown", "\n";
            exit(defined($actual) && $actual == \#(livePeer ? "1" : "0") ? 0 : 1);
            """#
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/usr/bin/perl")
            process.arguments = ["-e", program]
            process.currentDirectoryURL = root
            process.environment = ["PATH": "/usr/bin:/bin", "HOME": root.path, "TMPDIR": root.path + "/"]
            process.standardInput = FileHandle.nullDevice
            let stdoutURL = root.appendingPathComponent("stdout.raw")
            let stderrURL = root.appendingPathComponent("stderr.raw")
            let stdout = try watchdogFixtureOutput(stdoutURL)
            let stderr = try watchdogFixtureOutput(stderrURL)
            defer { try? stdout.close(); try? stderr.close() }
            process.standardOutput = stdout
            process.standardError = stderr
            let ended = DispatchSemaphore(value: 0)
            process.terminationHandler = { _ in ended.signal() }
            try process.run()
            guard ended.wait(timeout: .now() + 5) == .success else {
                WatchdogFixtureProcessRetention.shared.retain(process)
                preserveTemporaryDirectory(root, reason: "Guardian query fixture retained; no signals")
                throw NSError(domain: "GuardianPeerFixtureUnknown", code: 1)
            }
            process.waitUntilExit()
            for url in [stdoutURL, stderrURL] {
                let size = try url.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? Int.max
                _ = try #require(size <= 32768)
            }
            let output = try String(contentsOf: stdoutURL, encoding: .utf8)
            let errors = try String(contentsOf: stderrURL, encoding: .utf8)
            print("GUARDIAN_PEER live=\(livePeer) \(output)\(errors)")
            try #require(process.terminationReason == .exit && process.terminationStatus == 0)
            #expect(output == "actual=\(livePeer ? 1 : 0)\n")
        } catch {
            preserveTemporaryDirectory(root, reason: "Guardian peer regression failed: \(error)")
            throw error
        }
    }

    @Test("Group cleanup accepts ESRCH and rejects a still-live private group", arguments: [false, true])
    func geminiGroupAbsenceUsesKernelConfirmation(liveGroup: Bool) throws {
        let root = FileManager.default.temporaryDirectory.resolvingSymlinksInPath()
            .appendingPathComponent("group-absence-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: false,
                                                 attributes: [.posixPermissions: 0o700])
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        do {
            let script = try CLIAuthenticationScriptBuilder().build(
                provider: .gemini, executableURL: root.appendingPathComponent("never-executed-cli"),
                completionTokenPipeURL: root.appendingPathComponent("never-opened-token")
            )
            let start = try #require(script.range(of: "process_state() {"))
            let end = try #require(script.range(of: "watchdog_is_running() {", range: start.upperBound..<script.endIndex))
            let functions = String(script[start.lowerBound..<end.lowerBound])
            let shell = "agy_pid=\"$1\"\nprocess_state_executable=/bin/ps\n" + functions
                + "\nif process_group_is_empty; then print -r -- actual=empty; exit \(liveGroup ? 1 : 0); "
                + "else print -r -- actual=not-confirmed; exit \(liveGroup ? 0 : 1); fi\n"
            // The absent target is created and reaped here; the live target is
            // this same private process group. kill -0 observes, never signals.
            let starter = #"""
            use strict; use warnings; use POSIX qw(getpgrp setpgid waitpid _exit); use Errno qw(ESRCH);
            if (getpgrp() != $$) { setpgid(0, 0) == 0 or die "private group unavailable"; }
            my $target = $$;
            if (!$ARGV[0]) {
              my $child = fork(); defined($child) or die $!;
              if ($child == 0) { setpgid(0, 0) == 0 or _exit(3); _exit(0); }
              waitpid($child, 0) == $child && $? == 0 or die "child reap";
              $target = $child;
              { local $! = 0; my $matched = kill "-0", $target; die "group not absent" if $matched || $! != ESRCH; }
            }
            exec '/bin/zsh', '-c', $ARGV[1], 'private-group-fixture', "$target"; _exit(127);
            """#
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/usr/bin/perl")
            process.arguments = ["-e", starter, liveGroup ? "1" : "0", shell]
            process.currentDirectoryURL = root
            process.environment = ["PATH": "/usr/bin:/bin", "HOME": root.path, "TMPDIR": root.path + "/"]
            process.standardInput = FileHandle.nullDevice
            let stdoutURL = root.appendingPathComponent("stdout.raw")
            let stderrURL = root.appendingPathComponent("stderr.raw")
            let stdout = try watchdogFixtureOutput(stdoutURL)
            let stderr = try watchdogFixtureOutput(stderrURL)
            defer { try? stdout.close(); try? stderr.close() }
            process.standardOutput = stdout
            process.standardError = stderr
            let ended = DispatchSemaphore(value: 0)
            process.terminationHandler = { _ in ended.signal() }
            try process.run()
            guard ended.wait(timeout: .now() + 5) == .success else {
                WatchdogFixtureProcessRetention.shared.retain(process)
                preserveTemporaryDirectory(root, reason: "Group absence fixture retained; no signals")
                throw NSError(domain: "GroupAbsenceFixtureUnknown", code: 1)
            }
            process.waitUntilExit()
            for url in [stdoutURL, stderrURL] {
                let size = try url.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? Int.max
                _ = try #require(size <= 32768)
            }
            let output = try String(contentsOf: stdoutURL, encoding: .utf8)
            let errors = try String(contentsOf: stderrURL, encoding: .utf8)
            print("GROUP_ABSENCE live=\(liveGroup) \(output)\(errors)")
            try #require(process.terminationReason == .exit && process.terminationStatus == 0)
            #expect(output == (liveGroup ? "actual=not-confirmed\n" : "actual=empty\n"))
        } catch {
            preserveTemporaryDirectory(root, reason: "Group absence regression failed: \(error)")
            throw error
        }
    }

    @Test("Real Antigravity watchdog separates diagnostics, cancellation and deadlines")
    func realWatchdogClassifiesStateWithoutCancellingDiagnostics() throws {
        let root = FileManager.default.temporaryDirectory.resolvingSymlinksInPath()
            .appendingPathComponent("watchdog-state-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: false,
                                                 attributes: [.posixPermissions: 0o700])
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        do {
            let harnessURL = try #require(Bundle.module.url(forResource: "watchdog_state_regression", withExtension: "py"))
            let casesURL = try #require(Bundle.module.url(forResource: "watchdog_state_cases", withExtension: "json"))
            let harness = try Data(contentsOf: harnessURL)
            let table = try Data(contentsOf: casesURL)
            let rows = try #require(JSONSerialization.jsonObject(with: table) as? [[String: Any]])
            let script = try CLIAuthenticationScriptBuilder().build(
                provider: .gemini, executableURL: root.appendingPathComponent("never-executed-cli"),
                completionTokenPipeURL: root.appendingPathComponent("never-opened-token"),
                completionStatusFileURL: root.appendingPathComponent("never-written-status"),
                loginTimeoutSeconds: 1
            )
            for row in rows {
                let caseID = try #require(row["id"] as? String)
                _ = try #require(!caseID.isEmpty && !caseID.contains("/") && !caseID.contains(".."))
                let caseRoot = root.appendingPathComponent(caseID, isDirectory: true)
                try FileManager.default.createDirectory(at: caseRoot, withIntermediateDirectories: false,
                                                         attributes: [.posixPermissions: 0o700])
                try writeWatchdogFixtureData(harness, to: caseRoot.appendingPathComponent("watchdog_state_regression.py"))
                try writeWatchdogFixtureData(table, to: caseRoot.appendingPathComponent("watchdog_state_cases.json"))
                try writeWatchdogFixtureData(Data(script.utf8), to: caseRoot.appendingPathComponent("input.txt"))
                let process = Process()
                process.executableURL = URL(fileURLWithPath: "/usr/bin/python3")
                process.arguments = ["-I", "-S", "-B", caseRoot.appendingPathComponent("watchdog_state_regression.py").path,
                                     caseRoot.path, caseID]
                process.currentDirectoryURL = caseRoot
                process.environment = ["PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "HOME": caseRoot.path,
                                       "TMPDIR": caseRoot.path + "/", "LANG": "C", "LC_ALL": "C"]
                process.standardInput = FileHandle.nullDevice
                let stdout = try watchdogFixtureOutput(caseRoot.appendingPathComponent("runner.stdout"))
                let stderr = try watchdogFixtureOutput(caseRoot.appendingPathComponent("runner.stderr"))
                defer { try? stdout.close(); try? stderr.close() }
                process.standardOutput = stdout
                process.standardError = stderr
                let ended = DispatchSemaphore(value: 0)
                process.terminationHandler = { _ in ended.signal() }
                try process.run()
                guard ended.wait(timeout: .now() + 8) == .success else {
                    WatchdogFixtureProcessRetention.shared.retain(process)
                    preserveTemporaryDirectory(root, reason: "Watchdog fixture context retained; no signals or next case")
                    throw NSError(domain: "WatchdogFixtureUnknown", code: 1)
                }
                process.waitUntilExit() // Termination was already observed; never waits on a running unknown child.
                let receiptURL = caseRoot.appendingPathComponent("receipt.json")
                let size = try receiptURL.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? Int.max
                guard size <= 32768 else { throw NSError(domain: "WatchdogFixtureReceipt", code: 1) }
                let raw = try Data(contentsOf: receiptURL)
                let record = try #require(JSONSerialization.jsonObject(with: raw) as? [String: Any])
                print("WATCHDOG_REAL_CASE \(caseID) \(String(decoding: raw, as: UTF8.self))")
                _ = try #require(record["lifecycle_complete"] as? Bool == true)
                _ = try #require(record["watchdog_reaped"] as? Bool == true)
                let passed = record["semantic_passed"] as? Bool == true
                if !passed { preserveTemporaryDirectory(root, reason: "Real watchdog state regression failed: \(caseID)") }
                try #require(process.terminationReason == .exit && process.terminationStatus == 0,
                        "\(caseID): complete raw/receipt preserved")
                try #require(passed, "\(caseID): \(String(decoding: raw, as: UTF8.self))")
            }
        } catch {
            preserveTemporaryDirectory(root, reason: "Watchdog behavior regression evidence incomplete: \(error)")
            throw error
        }
    }

    @Test("Owned child observation distinguishes real ps absence from failed live queries",
          arguments: ["normal-gone", "stop-gone", "normal-failed", "normal-partial", "stop-failed", "stop-partial"])
    func geminiOwnedChildObservationHandlesGoneAndUnknownChild(fixtureCase: String) throws {
        let root = FileManager.default.temporaryDirectory.resolvingSymlinksInPath()
            .appendingPathComponent("owned-child-observation-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: false,
                                                 attributes: [.posixPermissions: 0o700])
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        do {
            let emitted = try CLIAuthenticationScriptBuilder().build(
                provider: .gemini, executableURL: root.appendingPathComponent("never-executed-cli"),
                completionTokenPipeURL: root.appendingPathComponent("never-opened-token")
            )
            // Execute these emitted regions verbatim, including bounded process_state,
            // the actual stop branch, and the normal branch's own wait/group check.
            // This fixture omits CLI, watchdog, callback and terminal startup boundaries.
            let functionsStart = try #require(emitted.range(of: "process_state() {"))
            let functionsEnd = try #require(emitted.range(of: "cancel_login() {",
                range: functionsStart.upperBound..<emitted.endIndex))
            let normalStart = try #require(emitted.range(of: "agy_exit_code=0\n"))
            let normalEnd = try #require(emitted.range(of: "# The supervisor remains",
                range: normalStart.upperBound..<emitted.endIndex))
            let functions = String(emitted[functionsStart.lowerBound..<functionsEnd.lowerBound])
            let normal = String(emitted[normalStart.lowerBound..<normalEnd.lowerBound])
            try writeWatchdogFixtureData(Data(emitted.utf8), to: root.appendingPathComponent("emitted.txt"))
            try writeWatchdogFixtureData(Data((fixtureCase + "\n").utf8), to: root.appendingPathComponent("case.txt"))
            let child = #"""
            use strict; use warnings; use POSIX qw(setpgid _exit);
            use Time::HiRes qw(clock_gettime CLOCK_MONOTONIC sleep);
            my ($mode) = @ARGV;
            setpgid(0, 0) == 0 or die "owned private group unavailable";
            my $identity = "$$ fixture-owned-nonce-v20\n";
            open my $g, '>', 'guardian.state' or die $!; print {$g} $identity; close $g or die $!;
            open my $r, '>', 'child.ready' or die $!; print {$r} "$$\n"; close $r or die $!;
            my $deadline = clock_gettime(CLOCK_MONOTONIC) + 2;
            while (clock_gettime(CLOCK_MONOTONIC) < $deadline) {
              if ($mode eq 'normal-gone' && -e 'release') { _exit(23); }
              if ($mode eq 'stop-gone' && open(my $s, '<', 'guardian.stop')) {
                local $/; my $request = <$s>; close $s;
                if ($request eq $identity) {
                  open my $a, '>', 'authenticated.ack' or die $!; print {$a} $identity; close $a or die $!;
                  _exit(31);
                }
              }
              sleep 0.005;
            }
            # Negative controls remain live through classification, then self-exit.
            _exit($mode =~ /-(failed|partial)$/ ? 47 : 97);
            """#
            let probe = #"""
            #!/usr/bin/perl
            use strict; use warnings; use POSIX qw(WNOHANG WIFEXITED WEXITSTATUS _exit);
            use Errno qw(ESRCH EINTR); use Time::HiRes qw(clock_gettime CLOCK_MONOTONIC sleep);
            sub read_file { open my $f, '<', $_[0] or die $!; local $/; my $v=<$f> // ''; close $f; return $v; }
            sub invalid { open my $f, '>>', 'barrier.failed' or die $!; print {$f} "$_[0]\n"; close $f; exit 96; }
            my $mode=read_file('case.txt'); chomp $mode;
            my $owned=read_file('owned.pid'); chomp $owned;
            invalid('invalid-query') unless @ARGV==4 && $ARGV[0] eq '-o' && $ARGV[1] eq 'stat='
              && $ARGV[2] eq '-p' && $ARGV[3] eq $owned && $owned =~ /^[1-9][0-9]*$/;
            # Independent signal-0 observation before the real ps invocation.
            my $absent; my $barrier_end=clock_gettime(CLOCK_MONOTONIC)+0.05;
            while (1) { local $!=0; my $n=kill 0,$owned;
              $absent=$n ? 0 : $! == ESRCH ? 1 : 2;
              last unless $absent==0 && $mode eq 'stop-gone' && -e 'guardian.stop'
                && clock_gettime(CLOCK_MONOTONIC)<$barrier_end;
              sleep 0.002;
            }
            invalid('natural-not-absent') if $mode eq 'normal-gone' && $absent!=1;
            invalid('stop-not-absent') if $mode eq 'stop-gone' && -e 'guardian.stop' && $absent!=1;
            invalid('negative-not-live') if $mode =~ /-(failed|partial)$/ && $absent!=0;
            invalid('invalid-mode') unless $mode =~ /^(normal|stop)-(gone|failed|partial)$/;
            my $abort=0; $SIG{TERM}=$SIG{INT}=$SIG{HUP}=sub { $abort=1; };
            my $pid=fork(); defined($pid) or invalid('ps-fork-failed');
            if (!$pid) {
              open STDOUT, '>', 'ps-last.raw' or _exit(98);
              open STDERR, '>>', 'ps.stderr' or _exit(98);
              if ($mode =~ /-failed$/) { exec('/bin/ps', '-o', 'ai_meter_fixture_invalid_field=', '-p', $owned) or _exit(98); }
              else { exec('/bin/ps', @ARGV) or _exit(98); }
            }
            # This adapter owns and reaps its real ps child, including timeout.
            # Its 100+20+20ms budget leaves room inside the emitted 250ms probe.
            my ($raw,$unknown);
            my $poll=sub {
              my $r=waitpid($pid,WNOHANG);
              if ($r==$pid) { $raw=$?; return 1; }
              if ($r<0 && $! != EINTR) { $unknown=1; return 1; }
              return 0;
            };
            my $end=clock_gettime(CLOCK_MONOTONIC)+0.1;
            while (!$poll->() && !$abort && clock_gettime(CLOCK_MONOTONIC)<$end) { sleep 0.002; }
            if (!defined($raw) && !$unknown) {
              for my $signal ('TERM','KILL') {
                kill $signal,$pid; # Only this unreaped direct child, never an unknown PID/group.
                $end=clock_gettime(CLOCK_MONOTONIC)+0.02;
                while (!$poll->() && clock_gettime(CLOCK_MONOTONIC)<$end) { sleep 0.001; }
                last if defined($raw) || $unknown;
              }
              invalid('ps-timeout-or-abort');
            }
            invalid('ps-wait-unknown') if $unknown || !defined($raw) || !WIFEXITED($raw);
            my $real_rc=WEXITSTATUS($raw);
            invalid('oversize-ps') if -s 'ps-last.raw' > 4096;
            my $output=read_file('ps-last.raw'); my $nonempty=length($output) ? 1 : 0;
            my $returned_rc=$mode =~ /-partial$/ ? 42 : $real_rc;
            open my $e, '>>', 'query-events.raw' or die $!;
            print {$e} "$real_rc:$returned_rc:$nonempty:$absent:$pid:1\n"; close $e or die $!;
            open my $o, '>>', 'query-outputs.raw' or die $!; print {$o} $output; close $o or die $!;
            print $output; exit $returned_rc;
            """#
            try writeWatchdogFixtureData(Data(child.utf8), to: root.appendingPathComponent("child.pl"))
            let probeURL = root.appendingPathComponent("query.pl")
            try writeWatchdogFixtureData(Data(probe.utf8), to: probeURL)
            try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: probeURL.path)
            let setup = #"""
            set -u
            umask 077
            fixture_case="$1"
            status_file="$PWD/helper.status"
            cancel_request_file="$PWD/not-cancelled"
            exec_status_file="$PWD/not-created.exec"
            process_group_file="$PWD/not-created.group"
            guardian_state_file="$PWD/guardian.state"
            guardian_stop_file="$PWD/guardian.stop"
            watchdog_ready_file="$PWD/not-created.ready"
            process_state_executable="$PWD/query.pl"
            agy_pid=""; agy_child_reaped=0; watchdog_pid=""
            terminal_foreground_transferred=0; guardian_state_preserved=0; cleanup_uncertain=0; timed_out=0
            fixture_product_wait_calls=0; fixture_wait_status=""
            report_status() { print -r -- "$1" >| "$status_file"; }
            stop_requested() { [[ -r "$cancel_request_file" ]] && [[ "$(<"$cancel_request_file")" == cancelled ]]; }
            send_receipt() { print -r -- "$1:$2" >> callback-boundary.raw; return 0; }
            # Transparent observation of the real zsh builtin; no fabricated status.
            wait() {
              builtin wait "$@"
              local fixture_rc=$?
              if [[ "$#" == 1 && "$1" == "$fixture_owned" ]]; then
                (( fixture_product_wait_calls += 1 ))
                fixture_wait_status="$fixture_rc"
              fi
              return "$fixture_rc"
            }
            fixture_finish() {
              local fixture_branch_exit="$1"
              trap - EXIT
              /usr/bin/perl -MErrno=ESRCH -e 'my $n=kill 0,$ARGV[0]; exit($n ? 0 : $! == ESRCH ? 1 : 2);' "$fixture_owned"
              local fixture_live_before_wait=$?
              if (( fixture_product_wait_calls == 0 )); then
                builtin wait "$fixture_owned"
                fixture_wait_status=$?
              fi
              /usr/bin/perl -MErrno=ESRCH -e 'my $n=kill "-0",$ARGV[0]; exit(!$n && $! == ESRCH ? 0 : 1);' "$fixture_owned"
              local fixture_group_absent=$?
              print -rl -- "owned_pid=$fixture_owned" "branch_exit=$fixture_branch_exit" "wait_status=$fixture_wait_status" \
                "product_wait_calls=$fixture_product_wait_calls" "pid_before_final_wait=$fixture_live_before_wait" \
                "group_esrch=$fixture_group_absent" "cleanup_uncertain=$cleanup_uncertain" \
                "guardian_preserved=$guardian_state_preserved" >| receipt.txt
              exit "$fixture_branch_exit"
            }
            /usr/bin/perl child.pl "$fixture_case" &
            fixture_owned=$!
            agy_pid="$fixture_owned"
            print -r -- "$fixture_owned" >| owned.pid
            trap 'fixture_finish $?' EXIT
            /usr/bin/perl -MTime::HiRes=clock_gettime,CLOCK_MONOTONIC,sleep -e '
              my $end=clock_gettime(CLOCK_MONOTONIC)+1;
              while (!-e "child.ready") { exit 95 if clock_gettime(CLOCK_MONOTONIC)>=$end; sleep 0.005; }'
            [[ "$?" == 0 && "$(<child.ready)" == "$fixture_owned" ]] || exit 95
            if [[ "$fixture_case" == normal-gone ]]; then
              : > release
              fixture_absent=0
              # Return to zsh between observations so its normal background-job
              # handler can reap the exited direct child while retaining wait status.
              for fixture_attempt in {1..50}; do
                /bin/sleep 0.01
                if /usr/bin/perl -MErrno=ESRCH -e 'my $n=kill 0,$ARGV[0]; exit(!$n && $! == ESRCH ? 0 : 1);' "$fixture_owned"; then
                  fixture_absent=1; break
                fi
              done
              (( fixture_absent )) || exit 95
            fi
            """#
            let dispatch = #"""
            if [[ "$fixture_case" == stop-* ]]; then
              stop_owned_processes
              exit $?
            fi
            """#
            let shell = setup + "\n" + functions + "\n" + dispatch + "\n" + normal + "\nexit \"$agy_exit_code\"\n"
            let shellURL = root.appendingPathComponent("fixture.zsh")
            try writeWatchdogFixtureData(Data(shell.utf8), to: shellURL)
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/bin/zsh")
            process.arguments = ["-f", shellURL.path, fixtureCase]
            process.currentDirectoryURL = root
            process.environment = ["PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "HOME": root.path,
                                   "TMPDIR": root.path + "/", "LANG": "C", "LC_ALL": "C"]
            process.standardInput = FileHandle.nullDevice
            let stdoutURL = root.appendingPathComponent("stdout.raw")
            let stderrURL = root.appendingPathComponent("stderr.raw")
            let stdout = try watchdogFixtureOutput(stdoutURL)
            let stderr = try watchdogFixtureOutput(stderrURL)
            defer { try? stdout.close(); try? stderr.close() }
            process.standardOutput = stdout; process.standardError = stderr
            let ended = DispatchSemaphore(value: 0)
            process.terminationHandler = { _ in ended.signal() }
            try process.run()
            guard ended.wait(timeout: .now() + 8) == .success else {
                WatchdogFixtureProcessRetention.shared.retain(process)
                preserveTemporaryDirectory(root, reason: "Owned-child regression timed out; no signal or retry")
                throw NSError(domain: "OwnedChildRegressionUnknown", code: 1)
            }
            process.waitUntilExit() // Already observed terminated; never waits on an unknown live process.
            func read(_ name: String, required: Bool = true) throws -> String {
                let url = root.appendingPathComponent(name)
                if !FileManager.default.fileExists(atPath: url.path) && !required { return "" }
                let size = try url.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? Int.max
                _ = try #require(size <= 32768)
                return try String(contentsOf: url, encoding: .utf8)
            }
            let raw = try read("receipt.txt")
            let errors = try read("stderr.raw")
            let output = try read("stdout.raw")
            let queryOutput = try read("query-outputs.raw")
            let psErrors = try read("ps.stderr", required: false)
            let eventsRaw = try read("query-events.raw")
            let queries = try eventsRaw.split(separator: "\n").map { line -> [Int] in
                let pieces = line.split(separator: ":", omittingEmptySubsequences: false)
                let fields = pieces.compactMap { Int($0) }
                _ = try #require(pieces.count == 6 && fields.count == 6 && fields[4] > 0 && fields[5] == 1)
                return fields
            }
            var record: [String: String] = [:]
            for line in raw.split(separator: "\n") {
                let pair = line.split(separator: "=", maxSplits: 1).map(String.init)
                _ = try #require(pair.count == 2 && record[pair[0]] == nil)
                record[pair[0]] = pair[1]
            }
            // The shell producer emits one key=value per line, with exactly these fields.
            _ = try #require(Set(record.keys) == Set([
                "owned_pid", "branch_exit", "wait_status", "product_wait_calls",
                "pid_before_final_wait", "group_esrch", "cleanup_uncertain", "guardian_preserved"
            ]))
            print("OWNED_CHILD_REGRESSION \(fixtureCase)\n\(raw)\(eventsRaw)\(output)\(errors)\(queryOutput)\(psErrors)")
            // These preconditions separate real behavior from setup, missing-function,
            // timeout, PID reuse and child-lifecycle failures. None is a RED substitute.
            _ = try #require(process.terminationReason == .exit)
            _ = try #require(record["branch_exit"] == String(process.terminationStatus))
            _ = try #require(try read("barrier.failed", required: false).isEmpty)
            _ = try #require(!errors.contains("command not found") && !errors.contains("parse error"))
            _ = try #require(!queries.isEmpty && queries.count <= 35)
            _ = try #require(record["group_esrch"] == "0")
            _ = try #require(try read("callback-boundary.raw", required: false).isEmpty)
            let gone = fixtureCase.hasSuffix("-gone")
            let stop = fixtureCase.hasPrefix("stop-")
            let expectedWait = gone ? (stop ? "31" : "23") : "47"
            _ = try #require(record["wait_status"] == expectedWait)
            if gone {
                _ = try #require(record["pid_before_final_wait"] == "1")
                _ = try #require(queries.last.map { Array($0.prefix(4)) } == [1, 1, 0, 1],
                                 "Real /bin/ps must fail on the already absent PID")
                if stop {
                    _ = try #require(queries.count >= 2 && queries.first.map { Array($0.prefix(4)) } == [0, 0, 1, 0])
                    let identity = try read("guardian.state")
                    let acknowledgement = try read("authenticated.ack")
                    let request = try read("guardian.stop")
                    _ = try #require(acknowledgement == identity && request == identity)
                }
            } else {
                if fixtureCase.hasSuffix("-failed") {
                    _ = try #require(psErrors.contains("ps: ai_meter_fixture_invalid_field: keyword not found\n"))
                    _ = try #require(psErrors.contains("ps: no valid keywords; valid keywords:\n"))
                    _ = try #require(!queryOutput.isEmpty)
                }
                let expectedQuery = fixtureCase.hasSuffix("-partial") ? [0, 42, 1, 0] : [1, 1, 1, 0]
                _ = try #require(queries.allSatisfy { Array($0.prefix(4)) == expectedQuery })
            }
            let ownedPID = try #require(record["owned_pid"].flatMap(Int.init))
            _ = try #require(ownedPID > 0)
            let helperStatus = try read("helper.status", required: false)
            let expectedExit: Int32 = gone ? (stop ? 0 : 23) : (stop ? 1 : 127)
            let passed = process.terminationStatus == expectedExit
                && record["product_wait_calls"] == (gone ? "1" : "0")
                && record["cleanup_uncertain"] == (gone ? "0" : "1")
                && record["guardian_preserved"] == (gone ? "0" : "1")
                && (gone || (helperStatus == "helper_stop_unconfirmed\n" && record["pid_before_final_wait"] == "0"))
            let realWaitObserved = record["wait_status"] == expectedWait
            let groupAbsentObserved = record["group_esrch"] == "0"
            let queryChildrenReaped = queries.allSatisfy { $0[5] == 1 }
            let lifecycleComplete = process.terminationReason == .exit && realWaitObserved
                && groupAbsentObserved && queryChildrenReaped
            func emit(_ phase: String) throws {
                let json: [String: Any] = ["schema": "ai-meter-v20-owned-child-regression-actual-v1",
                    "case_id": fixtureCase, "phase": phase, "owned_pid": ownedPID,
                    "fixture_exit": process.terminationStatus, "expected_exit": expectedExit,
                    "receipt": record, "query_events": queries,
                    "real_wait_status_observed": realWaitObserved, "kernel_group_esrch_observed": groupAbsentObserved,
                    "ps_children_reaped": queryChildrenReaped, "lifecycle_complete": lifecycleComplete,
                    "semantic_passed": passed, "root": root.path]
                let encoded = try JSONSerialization.data(withJSONObject: json, options: [.sortedKeys])
                _ = try #require(encoded.count <= 32768)
                print("OWNED_CHILD_V20 \(String(decoding: encoded, as: UTF8.self))")
            }
            // All actual lifecycle/query proofs precede any intentional RED assertion.
            _ = try #require(lifecycleComplete)
            try emit("lifecycle_verified_before_assertion")
            if !passed { preserveTemporaryDirectory(root, reason: "Owned-child behavior mismatch with lifecycle verified") }
            #expect(passed, "\(fixtureCase): expected branch exit \(expectedExit), actual lifecycle/query evidence verified; \(raw)")
            try emit("finish_assertion_evaluated")
        } catch {
            preserveTemporaryDirectory(root, reason: "Owned-child regression failed or setup incomplete: \(error)")
            throw error
        }
    }

    @Test("Owned query completion distinguishes complete absence from failed or timed-out queries")
    func geminiOwnedQueryCompletionRejectsUnknownQueryResults() throws {
        let root = FileManager.default.temporaryDirectory.resolvingSymlinksInPath()
            .appendingPathComponent("owned-query-completion-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: false,
                                                attributes: [.posixPermissions: 0o700])
        defer { removeTemporaryDirectoryUnlessPreserved(root) }
        do {
            // Actual linked builder output is the SUT. No host source extraction.
            let emitted = try CLIAuthenticationScriptBuilder().build(
                provider: .gemini, executableURL: root.appendingPathComponent("never-executed-cli"),
                completionTokenPipeURL: root.appendingPathComponent("never-opened-token"))
            let start = try #require(emitted.range(of: "process_state() {"))
            let end = try #require(emitted.range(of: "process_group_is_empty() {",
                                               range: start.upperBound..<emitted.endIndex))
            let originalFunctions = String(emitted[start.lowerBound..<end.lowerBound])
            let program = CLIAuthenticationScriptBuilder.boundedProcessStateProbeProgram
            func quote(_ value: String) -> String {
                "'" + value.replacingOccurrences(of: "'", with: "'\\''") + "'"
            }
            _ = try #require(originalFunctions.contains("owned_supervisor_state() {"))
            _ = try #require(originalFunctions.components(separatedBy: quote(program)).count == 2)
            _ = try #require(!FileManager.default.fileExists(atPath: "/etc/zshenv"))
            // Test-only trace: original syscall counts/arguments/results are retained.
            // Notes save/restore errno and child status. No syscall result is replaced.
            let tracePrefix = #"""
            # Component-only instrumentation: every original process syscall below is retained.
            # Notes save/restore both Perl errno and child-status; no synthetic syscall results.
            use Fcntl qw(O_WRONLY O_APPEND O_NOFOLLOW);
            my $component_trace_path = $ENV{COMPONENT_TRACE};
            defined($component_trace_path) or die "missing trace path";
            sysopen(my $component_trace, $component_trace_path, O_WRONLY|O_APPEND|O_NOFOLLOW) or die "trace open";
            my @component_stat = stat($component_trace);
            @component_stat && -f $component_trace && $component_stat[4] == $< && ($component_stat[2]&0777)==0600 && $component_stat[3]==1 or die "trace identity";
            sub component_note {
              my ($saved_errno, $saved_status) = (0+$!, $?);
              my $line = join("\t", $$, @_) . "\n";
              my @size = stat($component_trace);
              @size && $size[7] + length($line) <= 32768 or POSIX::_exit(125);
              my $written = CORE::syswrite($component_trace, $line);
              defined($written) && $written == length($line) or POSIX::_exit(125);
              $! = $saved_errno; $? = $saved_status;
              return;
            }

            END { my $component_final_exit = $?; component_note("program_exit", $component_final_exit); $? = $component_final_exit; }
            """#
            var changes: [(String, String, Int)] = []
            changes.append(("my $probe_pid = fork();", "my $probe_pid = fork();\n  component_note(\"fork\", defined($probe_pid) ? $probe_pid : \"undef\", 0+$!, 0+$?);", 1))
            changes.append(("my $count = sysread($reader, my $chunk, 1024);", "my $count = sysread($reader, my $chunk, 1024);\n      component_note(\"read\", defined($count) ? $count : \"undef\", 0+$!, 0+$?);", 1))
            changes.append(("my $waited = waitpid($probe_pid, WNOHANG);", "my $waited = waitpid($probe_pid, WNOHANG);\n      component_note(\"waitpid\", $probe_pid, $waited, 0+$!, 0+$?);", 3))
            changes.append(("if ($child_done && $eof && !$read_failed && $child_status == 0) {\n    close $reader;", "if ($child_done && $eof && !$read_failed && $child_status == 0) {\n    component_note(\"reader_close\", \"normal\", $eof, $child_done, $read_failed, $child_status);\n    close $reader;", 1))
            changes.append(("  close $reader;\n  if (!$child_done && $child_owned) {", "  component_note(\"reader_close\", \"failure\", $eof, $child_done, $read_failed, $child_status);\n  close $reader;\n  if (!$child_done && $child_owned) {", 1))
            changes.append(("kill \"TERM\", $probe_pid;", "my $component_kill_term = kill \"TERM\", $probe_pid;\n    component_note(\"kill\", \"TERM\", $probe_pid, $component_kill_term, 0+$!, 0+$?);", 1))
            changes.append(("kill \"KILL\", $probe_pid;", "my $component_kill_kill = kill \"KILL\", $probe_pid;\n      component_note(\"kill\", \"KILL\", $probe_pid, $component_kill_kill, 0+$!, 0+$?);", 1))
            var tracedBody = program
            for (before, after, count) in changes {
                _ = try #require(tracedBody.components(separatedBy: before).count == count + 1)
                tracedBody = tracedBody.replacingOccurrences(of: before, with: after)
            }
            var restored = tracedBody
            for (before, after, count) in changes.reversed() {
                _ = try #require(restored.components(separatedBy: after).count == count + 1)
                restored = restored.replacingOccurrences(of: after, with: before)
            }
            _ = try #require(restored == program)
            let traced = tracePrefix + "\n" + tracedBody
            let functions = originalFunctions.replacingOccurrences(of: quote(program), with: quote(traced))
            _ = try #require(functions.replacingOccurrences(of: quote(traced), with: quote(program)) == originalFunctions)
            try writeWatchdogFixtureData(Data(program.utf8), to: root.appendingPathComponent("actual-program.pl"))
            try writeWatchdogFixtureData(Data(traced.utf8), to: root.appendingPathComponent("traced-program.pl"))
            try writeWatchdogFixtureData(Data(originalFunctions.utf8), to: root.appendingPathComponent("actual-functions.zsh"))
            let target = #"""
            use Time::HiRes ();
            use strict; use warnings; use POSIX qw(setpgid getpgrp); use Fcntl qw(O_WRONLY O_CREAT O_EXCL O_NOFOLLOW);
            my ($dir) = @ARGV;
            defined($dir) && $dir eq "$ENV{COMPONENT_ROOT}/runtime/cases/$ENV{COMPONENT_CASE}" or die "dir";
            setpgid(0,0)==0 or die "setpgid";
            sysopen(my $ready, "$dir/target.ready", O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW, 0600) or die "ready";
            print $ready join("\t", $$, getppid(), getpgrp()), "\n" or die "ready write";
            close($ready) or die "ready close";
            my $deadline = Time::HiRes::clock_gettime(Time::HiRes::CLOCK_MONOTONIC()) + 20;
            while (!-e "$dir/target.release") {
              Time::HiRes::clock_gettime(Time::HiRes::CLOCK_MONOTONIC()) < $deadline or die "target release deadline";
              select undef,undef,undef,0.01;
            }
            exit 23;
            """#
            let query = #"""
            #!/bin/zsh -f
            set -u
            umask 077
            [[ $# == 4 && "$1" == -o && "$2" == stat= && "$3" == -p && "$4" == "$COMPONENT_TARGET_PID" ]] || exit 124
            [[ "$COMPONENT_CASE_DIR" == "$COMPONENT_ROOT/runtime/cases/$COMPONENT_CASE" ]] || exit 124
            [[ ! -e "$COMPONENT_CASE_DIR/query.pid" ]] || exit 124
            print -r -- "$$" > "$COMPONENT_CASE_DIR/query.pid" || exit 124
            case "$COMPONENT_CASE" in
              gone-exit1|live-exit1) exec /usr/bin/false ;;
              gone-timeout) exec /bin/sleep 2 ;;
              gone-exit42) exit 42 ;;
              gone-nonempty-exit1) print -r -- partial-query-output; exit 1 ;;
              *) exit 124 ;;
            esac
            """#
            let proof = #"""
            use strict; use warnings; use Errno qw(ESRCH); use POSIX qw(getpgrp);
            my ($mode, $target, $query) = @ARGV;
            defined($target) && $target =~ /^[1-9][0-9]*$/ && $target>1 && $target<=2147483647 or die "target";
            local $SIG{ALRM}=sub {exit 124}; alarm 1;
            sub absent {
              my ($pid,$group)=@_;
              my $matched=$group ? kill("-0",$pid) : kill(0,$pid);
              my $errno=0+$!;
              return (!$matched && $errno==ESRCH) ? 1 : 0;
            }
            if ($mode eq "live") {
              my $matched=kill 0,$target;
              $matched==1 or exit 125;
              print "target_live\t$target\n"; exit 0;
            }
            if ($mode eq "gone") {
              my $matched=kill 0,$target;
              my $errno=0+$!;
              if ($matched==1) { exit 3; }
              !$matched && $errno==ESRCH or exit 125;
              print "target_absent\t$target\n"; exit 0;
            }
            $mode eq "closed" && defined($query) && $query =~ /^[1-9][0-9]*$/ && $query>1 && $query<=2147483647 && $query!=$target or die "closed args";
            absent($target,0) or exit 125;
            absent($target,1) or exit 125;
            absent($query,0) or exit 125;
            print "target_absent\t$target\ntarget_group_absent\t$target\nquery_absent\t$query\n";
            exit 0;
            """#
            let setup = #"""
            #!/bin/zsh -f
            set -u
            setopt no_bgnice
            umask 077
            ROOT="$COMPONENT_ROOT"
            park() {
              print -r -- "NATIVE_QUERY_UNKNOWN_PRESERVE ${COMPONENT_CASE:-unset}"
              while true; do /bin/sleep 1; done
            }
            """#
            let body = #"""
            process_state_executable="$ROOT/query.sh"
            COMPONENT_CASE="$1"
              export COMPONENT_CASE
              COMPONENT_CASE_DIR="$ROOT/runtime/cases/$COMPONENT_CASE"
              export COMPONENT_CASE_DIR
              COMPONENT_TRACE="$COMPONENT_CASE_DIR/query.trace"
              export COMPONENT_TRACE
              /usr/bin/perl "$ROOT/target.pl" "$COMPONENT_CASE_DIR" &
              agy_pid=$!
              export COMPONENT_TARGET_PID="$agy_pid"
              print -r -- "$agy_pid" > "$COMPONENT_CASE_DIR/target.pid" || park
              ready=0
              for _ in {1..100}; do
                if [[ -f "$COMPONENT_CASE_DIR/target.ready" ]]; then ready=1; break; fi
                /bin/sleep 0.01 || park
              done
              [[ "$ready" == 1 ]] || park
              if [[ "$COMPONENT_CASE" != live-exit1 ]]; then
                print -r -- release > "$COMPONENT_CASE_DIR/target.release" || park
                gone=0
                for _ in {1..100}; do
                  /usr/bin/perl "$ROOT/pid-proof.pl" gone "$agy_pid" > "$COMPONENT_CASE_DIR/precheck.raw"
                  gone_status=$?
                  if [[ "$gone_status" == 0 ]]; then gone=1; break; fi
                  [[ "$gone_status" == 3 ]] || park
                  /bin/sleep 0.01 || park
                done
                [[ "$gone" == 1 ]] || park
              else
                /usr/bin/perl "$ROOT/pid-proof.pl" live "$agy_pid" > "$COMPONENT_CASE_DIR/precheck.raw" || park
              fi
              helper_output="$(owned_supervisor_state)"
              helper_status=$?
              print -r -- "$helper_status" > "$COMPONENT_CASE_DIR/helper.status" || park
              print -rn -- "$helper_output" > "$COMPONENT_CASE_DIR/helper.stdout" || park
              if [[ "$COMPONENT_CASE" == live-exit1 ]]; then
                print -r -- release > "$COMPONENT_CASE_DIR/target.release" || park
              fi
              wait "$agy_pid"
              target_wait=$?
              print -r -- "$target_wait" > "$COMPONENT_CASE_DIR/target.wait" || park
              [[ "$target_wait" == 23 && -f "$COMPONENT_CASE_DIR/query.pid" ]] || park
              query_pid="$(<"$COMPONENT_CASE_DIR/query.pid")"
              /usr/bin/perl "$ROOT/pid-proof.pl" closed "$agy_pid" "$query_pid" > "$COMPONENT_CASE_DIR/lifecycle-native.raw" || park
              print -r -- "CASE_LIFECYCLE_READY $COMPONENT_CASE"
              agy_pid=""
            print -r -- "NATIVE_QUERY_CASE_CLOSED $COMPONENT_CASE"
            exit 0
            """#
            for (name, value) in [("target.pl", target), ("query.sh", query), ("pid-proof.pl", proof)] {
                try writeWatchdogFixtureData(Data((value + "\n").utf8), to: root.appendingPathComponent(name))
            }
            try FileManager.default.setAttributes([.posixPermissions: 0o700],
                                                  ofItemAtPath: root.appendingPathComponent("query.sh").path)
            let runtime = root.appendingPathComponent("runtime/cases", isDirectory: true)
            try FileManager.default.createDirectory(at: runtime, withIntermediateDirectories: true,
                                                    attributes: [.posixPermissions: 0o700])
            let suiteURL = root.appendingPathComponent("fixture.zsh")
            try writeWatchdogFixtureData(Data((setup + "\n" + functions + "\n" + body + "\n").utf8), to: suiteURL)
            let cases = ["gone-exit1", "gone-timeout", "gone-exit42", "gone-nonempty-exit1", "live-exit1"]
            var records: [[String: Any]] = []
            func emit(_ record: [String: Any], phase: String) throws {
                var value = record; value["phase"] = phase
                let raw = try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
                _ = try #require(raw.count <= 32768)
                print("OWNED_QUERY_COMPLETION_V21 \(String(decoding: raw, as: UTF8.self))")
            }
            for caseID in cases {
                let folder = runtime.appendingPathComponent(caseID, isDirectory: true)
                try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: false,
                                                        attributes: [.posixPermissions: 0o700])
                try writeWatchdogFixtureData(Data(), to: folder.appendingPathComponent("query.trace"))
                let process = Process(); process.executableURL = URL(fileURLWithPath: "/bin/zsh")
                process.arguments = ["-f", suiteURL.path, caseID]; process.currentDirectoryURL = root
                process.environment = ["PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "HOME": root.path,
                    "TMPDIR": root.path + "/", "LANG": "C", "LC_ALL": "C", "COMPONENT_ROOT": root.path]
                process.standardInput = FileHandle.nullDevice
                let stdout = try watchdogFixtureOutput(folder.appendingPathComponent("runner.stdout"))
                let stderr = try watchdogFixtureOutput(folder.appendingPathComponent("runner.stderr"))
                defer { try? stdout.close(); try? stderr.close() }
                process.standardOutput = stdout; process.standardError = stderr
                let ended = DispatchSemaphore(value: 0)
                process.terminationHandler = { _ in ended.signal() }
                try process.run()
                guard ended.wait(timeout: .now() + 8) == .success else {
                    WatchdogFixtureProcessRetention.shared.retain(process)
                    preserveTemporaryDirectory(root, reason: "Owned query unknown; no signals, retry or next case")
                    print("OWNED_QUERY_UNKNOWN_OWNER_WAIT case=\(caseID) pid=\(process.processIdentifier) root_prefix=\(String(root.path.prefix(1024)))")
                    // Unbounded UNKNOWN wait preserves this native OS parent; it is not a test success budget.
                    // Only the same terminationHandler may release this wait; no signal, spawn or next case.
                    ended.wait()
                    process.waitUntilExit() // The same fixture termination was actually observed.
                    throw NSError(domain: "OwnedQueryCompletionUnknown", code: 1)
                }
                process.waitUntilExit() // Only after observed termination; never blocks on unknown.
                func read(_ name: String, limit: Int = 32768) throws -> String {
                    let url = folder.appendingPathComponent(name)
                    let size = try url.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? Int.max
                    _ = try #require(size <= limit)
                    return try String(contentsOf: url, encoding: .utf8)
                }
                let output = try read("runner.stdout", limit: 65536)
                let errors = try read("runner.stderr", limit: 65536)
                _ = try #require(process.terminationReason == .exit && process.terminationStatus == 0)
                _ = try #require(output.components(separatedBy: "NATIVE_QUERY_CASE_CLOSED \(caseID)\n").count == 2)
                _ = try #require(errors.isEmpty)
                func positive(_ value: String) throws -> Int {
                    let n = try #require(Int(value)); _ = try #require(n > 1 && n <= 2147483647); return n
                }
                let targetPID = try positive(read("target.pid").trimmingCharacters(in: .newlines))
                let queryPID = try positive(read("query.pid").trimmingCharacters(in: .newlines))
                let shellPID = Int(process.processIdentifier)
                _ = try #require(targetPID != queryPID && targetPID != shellPID && queryPID != shellPID)
                _ = try #require(try read("target.ready") == "\(targetPID)\t\(shellPID)\t\(targetPID)\n")
                let targetWaitRaw = try read("target.wait")
                let targetWait = try #require(Int(targetWaitRaw.trimmingCharacters(in: .newlines)))
                _ = try #require(targetWait == 23 && targetWaitRaw == "\(targetWait)\n")
                let wasLive = caseID == "live-exit1"
                _ = try #require(try read("precheck.raw") == "\(wasLive ? "target_live" : "target_absent")\t\(targetPID)\n")
                let closedProof = try read("lifecycle-native.raw")
                _ = try #require(closedProof == "target_absent\t\(targetPID)\ntarget_group_absent\t\(targetPID)\nquery_absent\t\(queryPID)\n")
                let traceRaw = try read("query.trace")
                let rows = traceRaw.split(separator: "\n").map { $0.split(separator: "\t", omittingEmptySubsequences: false).map(String.init) }
                _ = try #require(!rows.isEmpty && rows.count <= 1024 && rows.allSatisfy { $0.count >= 3 })
                let forks = rows.filter { $0[1] == "fork" && $0[2] != "0" && $0[2] != "undef" }
                _ = try #require(forks.count == 1 && forks[0].count == 5)
                let probePID = try positive(forks[0][0]); _ = try #require(try positive(forks[0][2]) == queryPID)
                _ = try #require(![targetPID, queryPID, shellPID].contains(probePID))
                let childForks = rows.filter { $0[1] == "fork" && $0[2] == "0" }
                _ = try #require(childForks.count == 1 && childForks[0][0] == String(queryPID) && rows.filter { $0[1] == "fork" }.count == 2)
                var waits: [(Int, Int)] = [], closes: [(Int, [String])] = [], exits: [(Int, Int)] = []
                var signals: [(Int, String)] = [], reads: [[String]] = []
                for (index, row) in rows.enumerated() {
                    let pid = try positive(row[0]); _ = try #require(pid == probePID || pid == queryPID)
                    if pid == queryPID { _ = try #require(row[1] == "fork" && row[2] == "0") }
                    switch row[1] {
                    case "fork":
                        _ = try #require(row.count == 5 && Int(row[3]) != nil && Int(row[4]) != nil)
                    case "waitpid":
                        _ = try #require(pid == probePID && row.count == 6 && row[2] == String(queryPID))
                        let actual = try #require(Int(row[3])); _ = try #require(actual == 0 || actual == queryPID)
                        _ = try #require(Int(row[4]) != nil)
                        let status = try #require(Int(row[5])); if actual == queryPID { waits.append((index, status)) }
                    case "reader_close":
                        _ = try #require(pid == probePID && row.count == 7); closes.append((index, row))
                    case "read":
                        _ = try #require(pid == probePID && row.count == 5 && Int(row[3]) != nil && Int(row[4]) != nil)
                        if row[2] == "undef" {
                            _ = try #require([Int(EINTR), Int(EAGAIN), Int(EWOULDBLOCK)].contains(try #require(Int(row[3]))))
                        } else {
                            let count = try #require(Int(row[2])); _ = try #require((0...1024).contains(count))
                        }
                        reads.append(row)
                    case "kill":
                        _ = try #require(pid == probePID && row.count == 7 && ["TERM", "KILL"].contains(row[2]) && row[3] == String(queryPID))
                        let sent = try #require(Int(row[4])); let errno = try #require(Int(row[5]))
                        _ = try #require(Int(row[6]) != nil)
                        _ = try #require(sent == 1 || (sent == 0 && errno == Int(ESRCH)))
                        signals.append((index, row[2]))
                    case "program_exit":
                        _ = try #require(pid == probePID && row.count == 3)
                        exits.append((index, try #require(Int(row[2]))))
                    default: throw NSError(domain: "OwnedQueryUnexpectedTrace", code: 1)
                    }
                }
                _ = try #require(waits.count == 1 && closes.count == 1 && exits.count == 1)
                _ = try #require(exits[0].0 == rows.count - 1 && waits[0].0 < exits[0].0 && closes[0].0 < exits[0].0)
                _ = try #require((0...255).contains(exits[0].1))
                let close = closes[0].1
                let closeValues = try close.dropFirst(3).map { try #require(Int($0)) }
                let timeout = caseID == "gone-timeout"
                if timeout {
                    _ = try #require(close[2] == "failure" && closeValues == [0, 0, 0, 0])
                    _ = try #require(closes[0].0 < waits[0].0 && signals.allSatisfy { closes[0].0 < $0.0 && $0.0 < waits[0].0 })
                    _ = try #require([ ["TERM"], ["TERM", "KILL"] ].contains(signals.map { $0.1 }))
                    _ = try #require([0, 15, 9].contains(waits[0].1) && !reads.contains { $0[2] == "0" })
                } else {
                    let expected = (caseID == "gone-exit42" ? 42 : 1) << 8
                    _ = try #require(closeValues == [1, 1, 0, expected] && waits[0].1 == expected && signals.isEmpty)
                    _ = try #require(waits[0].0 < closes[0].0 && reads.contains { $0[2] == "0" })
                    if caseID == "gone-nonempty-exit1" { _ = try #require(reads.contains { (Int($0[2]) ?? 0) > 0 }) }
                }
                let statusRaw = try read("helper.status")
                let status = try #require(Int(statusRaw.trimmingCharacters(in: .newlines)))
                _ = try #require(statusRaw == "\(status)\n" && (0...255).contains(status))
                let helperOutput = try read("helper.stdout")
                let expectedSuccess = caseID == "gone-exit1"
                let semanticPassed = (status == 0) == expectedSuccess && helperOutput.isEmpty
                // Each true proof follows validated actual syscall/file observations above.
                let lifecycle = process.terminationReason == .exit && process.terminationStatus == 0
                    && waits.count == 1 && closes.count == 1 && exits.count == 1
                let record: [String: Any] = ["schema": "ai-meter-v21-owned-query-native-actual-v1",
                    "case_id": caseID, "root": folder.path, "target_pid": targetPID, "query_pid": queryPID,
                    "probe_pid": probePID, "shell_pid": shellPID, "target_wait": targetWait, "closed_native_proof": closedProof, "query_wait_status": waits[0].1,
                    "query_internal_eof_observed": closeValues[0] == 1, "reader_closed_before_cleanup": closes[0].0 < waits[0].0,
                    "query_cleanup_signals": signals.map { $0.1 }, "program_exit_code": exits[0].1,
                    "helper_actual_exit": status, "helper_stdout_bytes": helperOutput.utf8.count,
                    "expected_helper_success": expectedSuccess, "lifecycle_complete": lifecycle,
                    "semantic_passed": semanticPassed, "trace": rows]
                _ = try #require(lifecycle)
                records.append(record)
                try emit(record, phase: "lifecycle_verified_before_assertion")
            }
            _ = try #require(records.count == 5)
            print("OWNED_QUERY_ALL_FIVE_LIFECYCLES_VERIFIED")
            // No semantic assertion can skip any of the five controlled lifecycles.
            for record in records {
                let passed = try #require(record["semantic_passed"] as? Bool)
                let caseID = try #require(record["case_id"] as? String)
                if !passed { preserveTemporaryDirectory(root, reason: "Owned-query completion semantic mismatch: \(caseID)") }
                #expect(passed, "\(caseID): all five native fixture lifecycles verified")
                try emit(record, phase: "finish_assertion_evaluated")
            }
        } catch {
            preserveTemporaryDirectory(root, reason: "Owned-query fixture unknown/setup incomplete: \(error)")
            throw error
        }
    }
    private func writeWatchdogFixtureData(_ data: Data, to url: URL) throws {
        let descriptor = url.path.withCString { Darwin.open($0, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0o600) }
        guard descriptor >= 0 else { throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno)) }
        let handle = FileHandle(fileDescriptor: descriptor, closeOnDealloc: true)
        defer { try? handle.close() }
        try handle.write(contentsOf: data)
        try handle.synchronize()
    }

    private func watchdogFixtureOutput(_ url: URL) throws -> FileHandle {
        let descriptor = url.path.withCString { Darwin.open($0, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0o600) }
        guard descriptor >= 0 else { throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno)) }
        return FileHandle(fileDescriptor: descriptor, closeOnDealloc: true)
    }

    @Test("Antigravity exec status cannot overwrite the handoff status")
    func rejectsCollidingGeminiStatusFiles() {
        let statusFile = URL(fileURLWithPath: "/tmp/antigravity.status")
        #expect(throws: CLIAuthenticationScriptError.invalidCompletionTokenPipe) {
            try CLIAuthenticationScriptBuilder().build(
                provider: .gemini,
                executableURL: URL(fileURLWithPath: "/tmp/agy"),
                completionTokenPipeURL: URL(fileURLWithPath: "/tmp/antigravity.token"),
                completionStatusFileURL: statusFile,
                completionExecStatusFileURL: statusFile
            )
        }
        #expect(throws: CLIAuthenticationScriptError.invalidCompletionTokenPipe) {
            try CLIAuthenticationScriptBuilder().build(
                provider: .gemini,
                executableURL: URL(fileURLWithPath: "/tmp/agy"),
                completionTokenPipeURL: URL(fileURLWithPath: "/tmp/antigravity.token"),
                completionExecStatusFileURL: URL(fileURLWithPath: "/tmp/antigravity.token.status")
            )
        }
    }
}


private final class WatchdogFixtureProcessRetention: @unchecked Sendable {
    static let shared = WatchdogFixtureProcessRetention()
    private let lock = NSLock()
    private var processes: [Process] = []
    func retain(_ process: Process) {
        lock.lock()
        processes.append(process)
        lock.unlock()
    }
}

private final class TemporaryRootRetention: @unchecked Sendable {
    static let shared = TemporaryRootRetention()

    private let lock = NSLock()
    private var retainedPaths = Set<String>()

    func retain(_ root: URL) {
        lock.lock()
        retainedPaths.insert(root.standardizedFileURL.path)
        lock.unlock()
    }

    func contains(_ root: URL) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return retainedPaths.contains(root.standardizedFileURL.path)
    }
}

// Private test observer. O_CLOEXEC is applied atomically by open; there
// is no parent writer descriptor or raw pipe-to-fcntl inheritance window.
// Native spawn explicitly supplies only stdio and closes all other descriptors.
private enum FixtureProcessQuery {
    struct Output: Sendable, Equatable { let exitStatus: Int32; let text: String }

    static func run(executable: URL, arguments: [String], in root: URL,
                    timeout: Duration = .milliseconds(250), onPrepared: (@Sendable () -> Void)? = nil) -> Output? {
        var rootValidated = false
        func unknown() -> Output? {
            TemporaryRootRetention.shared.retain(root)
            if rootValidated {
                try? Data("Native bounded fixture observer could not establish process state\n".utf8)
                    .write(to: root.appendingPathComponent(".preserve-for-inspection"), options: .atomic)
            }
            return nil
        }
        var rootInfo = stat()
        guard root.path.withCString({ Darwin.lstat($0, &rootInfo) }) == 0,
              rootInfo.st_mode & mode_t(S_IFMT) == mode_t(S_IFDIR), rootInfo.st_uid == getuid(),
              rootInfo.st_mode & 0o777 == 0o700 else { return unknown() }
        rootValidated = true
        let fifo = root.appendingPathComponent("observer-output-\(UUID().uuidString).fifo")
        guard fifo.path.withCString({ Darwin.mkfifo($0, 0o600) }) == 0 else { return unknown() }
        var fifoInfo = stat()
        guard fifo.path.withCString({ Darwin.lstat($0, &fifoInfo) }) == 0,
              fifoInfo.st_mode & mode_t(S_IFMT) == mode_t(S_IFIFO), fifoInfo.st_uid == getuid(), fifoInfo.st_nlink == 1,
              fifo.path.withCString({ Darwin.chmod($0, 0o600) }) == 0 else { return unknown() }
        var reader = fifo.path.withCString { Darwin.open($0, O_RDONLY | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW) }
        guard reader >= 0 else { return unknown() }
        defer { if reader >= 0 { Darwin.close(reader) } }
        let fdFlags = fcntl(reader, F_GETFD)
        var openedInfo = stat()
        guard fdFlags >= 0, fdFlags & FD_CLOEXEC != 0, fstat(reader, &openedInfo) == 0,
              openedInfo.st_dev == fifoInfo.st_dev, openedInfo.st_ino == fifoInfo.st_ino, openedInfo.st_nlink == 1,
              openedInfo.st_mode & 0o777 == 0o600 else { return unknown() }

        var actions: posix_spawn_file_actions_t?
        guard posix_spawn_file_actions_init(&actions) == 0 else { return unknown() }
        var actionsInitialized = true
        defer { if actionsInitialized { posix_spawn_file_actions_destroy(&actions) } }
        var attributes: posix_spawnattr_t?
        guard posix_spawnattr_init(&attributes) == 0 else { return unknown() }
        var attributesInitialized = true
        defer { if attributesInitialized { posix_spawnattr_destroy(&attributes) } }
        guard posix_spawnattr_setflags(&attributes, Int16(POSIX_SPAWN_CLOEXEC_DEFAULT)) == 0,
              posix_spawn_file_actions_addopen(&actions, STDIN_FILENO, "/dev/null", O_RDONLY, 0) == 0,
              posix_spawn_file_actions_addopen(&actions, STDOUT_FILENO, fifo.path, O_WRONLY | O_NONBLOCK | O_NOFOLLOW, 0) == 0,
              posix_spawn_file_actions_addopen(&actions, STDERR_FILENO, "/dev/null", O_WRONLY, 0) == 0 else { return unknown() }
        let argv = ([executable.path] + arguments).map { strdup($0) } + [nil]
        let environment = ["HOME=\(root.path)", "ZDOTDIR=\(root.path)", "TMPDIR=\(root.path)",
                           "PATH=\(root.path):/usr/bin:/bin:/usr/sbin:/sbin", "LANG=C", "LC_ALL=C"]
            .map { value in value.withCString { strdup($0) } } + [nil]
        defer { argv.forEach { free($0) }; environment.forEach { free($0) } }
        guard argv.dropLast().allSatisfy({ $0 != nil }), environment.dropLast().allSatisfy({ $0 != nil }) else { return unknown() }
        onPrepared?()
        var pid: pid_t = 0
        let spawnResult = argv.withUnsafeBufferPointer { arguments in
            environment.withUnsafeBufferPointer { variables in
                posix_spawn(&pid, executable.path, &actions, &attributes, arguments.baseAddress!, variables.baseAddress!)
            }
        }
        guard spawnResult == 0, pid > 0 else { return unknown() }
        // No read occurs before successful spawn/file actions have opened stdout.
        // An early FIFO EOF is accepted only together with a known owned reap.
        var status: Int32 = 0
        var reaped = false
        var owned = true
        var eof = false
        var output = Data()
        func observe() {
            guard !reaped, owned else { return }
            let result = waitpid(pid, &status, WNOHANG)
            if result == pid { reaped = true }
            else if result < 0, errno != EINTR { owned = false }
        }
        func stopOwnedQuery() {
            observe()
            for signal in [SIGTERM, SIGKILL] where !reaped && owned {
                _ = Darwin.kill(pid, signal)
                let deadline = ContinuousClock.now.advanced(by: .milliseconds(150))
                while !reaped, owned, ContinuousClock.now < deadline {
                    observe()
                    if !reaped { Thread.sleep(forTimeInterval: 0.002) }
                }
            }
        }
        let deadline = ContinuousClock.now.advanced(by: timeout)
        var failed = false
        while ContinuousClock.now < deadline, owned, !failed {
            if !eof {
                var buffer = [UInt8](repeating: 0, count: 1024)
                let count = Darwin.read(reader, &buffer, buffer.count)
                if count > 0 {
                    if output.count + count > 4096 { failed = true }
                    else { output.append(contentsOf: buffer.prefix(count)) }
                } else if count == 0 { eof = true }
                else if errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR { failed = true }
            }
            observe()
            if reaped && eof { break }
            Thread.sleep(forTimeInterval: 0.002)
        }
        guard !failed, owned, reaped, eof, status & 0x7f == 0 else {
            stopOwnedQuery()
            return unknown()
        }
        // Successful setup-object disposal is part of the known-ended result.
        let attributeCleanup = posix_spawnattr_destroy(&attributes)
        attributesInitialized = false
        let actionCleanup = posix_spawn_file_actions_destroy(&actions)
        actionsInitialized = false
        guard attributeCleanup == 0, actionCleanup == 0 else { return unknown() }
        // Only a known-ended query/output lifecycle permits disposal of our
        // own FIFO inode. Unknown/replaced paths or descendants keep evidence.
        guard Darwin.close(reader) == 0 else { reader = -1; return unknown() }
        reader = -1
        var currentInfo = stat()
        guard fifo.path.withCString({ Darwin.lstat($0, &currentInfo) }) == 0,
              currentInfo.st_dev == fifoInfo.st_dev, currentInfo.st_ino == fifoInfo.st_ino,
              currentInfo.st_mode & mode_t(S_IFMT) == mode_t(S_IFIFO), currentInfo.st_nlink == 1,
              fifo.path.withCString({ Darwin.unlink($0) }) == 0 else { return unknown() }
        return Output(exitStatus: (status >> 8) & 0xff, text: String(decoding: output, as: UTF8.self))
    }
}
