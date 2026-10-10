import Foundation
import Darwin
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
        #expect(script.contains("handoff received. Starting Antigravity CLI sign-in in this window."))
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
        let cliLaunch = try #require(script.range(of: "/usr/bin/perl -MFcntl=:DEFAULT -MPOSIX -e"))
        #expect(watchdogLaunch.lowerBound < cliLaunch.lowerBound)
        #expect(script.contains("if [[ ! -r \"$status_file\" ]] || [[ \"$(<\"$status_file\")\" != ready ]]; then"))
        #expect(script.contains("if ($status ne \"ready\") { kill \"USR2\", $parent_pid; exit 0; }"))
        #expect(script.contains("watchdog_state=$(/bin/ps -o stat= -p \"$watchdog_pid\" 2>/dev/null)"))
        #expect(script.contains("\"$watchdog_state\" != *Z*"))
        #expect(script.contains("defined(fcntl($gate, F_SETLKW, $record_lock))"))
        #expect(script.contains("fcntl($gate, F_SETFD, $flags | FD_CLOEXEC)"))
        #expect(script.contains("exec @command or die $!;"))
    }

    @Test("Antigravity helper does not start its CLI when cancellation arrives before watchdog readiness")
    func geminiCancellationBeforeWatchdogReadyPreventsCLIStart() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let tokenPipe = root.appendingPathComponent("completion.token")
        let statusFile = tokenPipe.appendingPathExtension("status")
        let cliStarted = root.appendingPathComponent("agy-started")
        let watcherStarted = root.appendingPathComponent("watchdog-started")
        try createTokenPipe(at: tokenPipe)
        let executable = try writeExecutable(
            "#!/bin/sh\nprintf started > '\(cliStarted.path)'\nexit 0\n",
            named: "agy",
            in: root
        )
        let fakeWatcher = try writeExecutable(
            "#!/bin/sh\nprintf started > '\(watcherStarted.path)'\nwhile [ \"$(cat \"$5\")\" = ready ]; do /bin/sleep 0.01; done\n/bin/kill -USR2 \"$3\"\n/bin/sleep 0.05\n/bin/touch \"$6\"\n",
            named: "watchdog",
            in: root
        )
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: executable,
            completionTokenPipeURL: tokenPipe,
            completionStatusFileURL: statusFile,
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

        let watcherDeadline = Date().addingTimeInterval(2)
        while !FileManager.default.fileExists(atPath: watcherStarted.path), Date() < watcherDeadline {
            Thread.sleep(forTimeInterval: 0.01)
        }
        #expect(FileManager.default.fileExists(atPath: watcherStarted.path))
        try Data("cancelled\n".utf8).write(to: statusFile, options: .atomic)
        process.waitUntilExit()

        #expect(process.terminationStatus == 130)
        #expect(!FileManager.default.fileExists(atPath: cliStarted.path))
        #expect(try String(contentsOf: statusFile, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines) == "helper_cancelled")
    }

    @Test("Antigravity cancellation wins the launch gate before CLI exec")
    func geminiCancellationWinsContendedLaunchGate() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let tokenPipe = root.appendingPathComponent("completion.token")
        let statusFile = tokenPipe.appendingPathExtension("status")
        let launchGate = statusFile.appendingPathExtension("lock")
        let cliStarted = root.appendingPathComponent("agy-started")
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
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: executable,
            completionTokenPipeURL: tokenPipe,
            completionStatusFileURL: statusFile,
            completionOpenExecutableURL: URL(fileURLWithPath: "/usr/bin/false"),
            loginTimeoutSeconds: 5
        )
        let scriptURL = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)
        let process = isolatedProcess(executable: URL(fileURLWithPath: "/bin/zsh"), arguments: [scriptURL.path], in: root)
        try process.run()
        try writeToken("12345678-1234-1234-1234-123456789abc", to: tokenPipe)

        let readyFile = statusFile.appendingPathExtension("ready")
        let readyDeadline = Date().addingTimeInterval(3)
        while !FileManager.default.fileExists(atPath: readyFile.path), Date() < readyDeadline {
            Thread.sleep(forTimeInterval: 0.01)
        }
        #expect(FileManager.default.fileExists(atPath: readyFile.path))
        Thread.sleep(forTimeInterval: 0.2)
        #expect(!FileManager.default.fileExists(atPath: cliStarted.path))
        try Data("cancelled\n".utf8).write(to: statusFile, options: .atomic)
        try Data().write(to: releaseGate)
        gateHolder.waitUntilExit()
        process.waitUntilExit()

        #expect(process.terminationStatus == 130)
        #expect(!FileManager.default.fileExists(atPath: cliStarted.path))
    }

    @Test("Antigravity success receipt is emitted only after a zero-exit fake CLI")
    func geminiSuccessReceiptRequiresZeroExit() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
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
    }

    @Test("Antigravity nonzero login exit sends a failure receipt, never a success receipt")
    func geminiNonzeroExitIsFailure() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
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
        defer { try? FileManager.default.removeItem(at: root) }
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
        defer { try? FileManager.default.removeItem(at: root) }
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
        defer { try? FileManager.default.removeItem(at: root) }
        let callbackLog = root.appendingPathComponent("callbacks.txt")
        let childPIDFile = root.appendingPathComponent("agy-child.pid")
        let tokenPipe = root.appendingPathComponent("completion.token")
        try createTokenPipe(at: tokenPipe)
        let executable = try writeExecutable(
            "#!/bin/sh\n/bin/sh -c 'trap \"\" TERM; while :; do /bin/sleep 1; done' &\nprintf '%s' \"$!\" > '\(childPIDFile.path)'\nexit 0\n",
            named: "agy",
            in: root
        )
        let fakeOpen = try writeExecutable(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '\(callbackLog.path)'\n",
            named: "open",
            in: root
        )

        let startedAt = Date()
        try runGeminiScript(executable: executable, fakeOpen: fakeOpen, timeoutSeconds: 2, tokenPipe: tokenPipe, in: root)
        let elapsed = Date().timeIntervalSince(startedAt)
        let childPIDText = try String(contentsOf: childPIDFile, encoding: .utf8)
        let childPID = try #require(pid_t(childPIDText))
        let callback = try String(contentsOf: callbackLog, encoding: .utf8)

        #expect(elapsed < 5)
        #expect(callback.contains("result=timeout"))
        #expect(!isRunning(childPID, in: root))
    }

    @Test("Closing the login terminal cancels only its own process group and sends a failure receipt")
    func geminiLoginCancellationCleansOwnedProcessGroup() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let callbackLog = root.appendingPathComponent("callbacks.txt")
        let childPIDFile = root.appendingPathComponent("agy.pid")
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
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: executable,
            completionTokenPipeURL: tokenPipe,
            completionOpenExecutableURL: fakeOpen,
            loginTimeoutSeconds: 10
        )
        let scriptURL = root.appendingPathComponent("login.command")
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)
        let process = isolatedProcess(executable: URL(fileURLWithPath: "/bin/zsh"), arguments: [scriptURL.path], in: root)
        try process.run()
        try writeToken("12345678-1234-1234-1234-123456789abc", to: tokenPipe)

        let deadline = Date().addingTimeInterval(2)
        while !FileManager.default.fileExists(atPath: childPIDFile.path), Date() < deadline {
            Thread.sleep(forTimeInterval: 0.01)
        }
        let childPIDText = try String(contentsOf: childPIDFile, encoding: .utf8)
        let childPID = try #require(pid_t(childPIDText))
        process.terminate()
        process.waitUntilExit()
        #expect(!FileManager.default.fileExists(atPath: scriptURL.path))

        let cleanupDeadline = Date().addingTimeInterval(2)
        while kill(childPID, 0) == 0, Date() < cleanupDeadline {
            Thread.sleep(forTimeInterval: 0.01)
        }
        let callback = try String(contentsOf: callbackLog, encoding: .utf8)
        #expect(callback.contains("result=cancelled"))
        #expect(callback.contains("exit_code=143"))
        #expect(kill(childPID, 0) != 0)
    }

    @Test("Antigravity token wait is bounded even when no app writer opens the FIFO")
    func geminiTokenWaitIsBoundedWithoutWriter() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
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

    private func runGeminiScript(executable: URL, fakeOpen: URL, timeoutSeconds: Int, tokenPipe: URL, in root: URL) throws {
        let token = "12345678-1234-1234-1234-123456789abc"
        let script = try CLIAuthenticationScriptBuilder().build(
            provider: .gemini,
            executableURL: executable,
            completionTokenPipeURL: tokenPipe,
            completionOpenExecutableURL: fakeOpen,
            loginTimeoutSeconds: timeoutSeconds
        )
        let scriptURL = root.appendingPathComponent("login.command")
        // Keep the generated shell behavior while routing URL opens to a local recorder.
        try Data(script.utf8).write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: scriptURL.path)
        let process = isolatedProcess(executable: URL(fileURLWithPath: "/bin/zsh"), arguments: [scriptURL.path], in: root)
        try process.run()
        try writeToken(token, to: tokenPipe)
        process.waitUntilExit()
        #expect(!FileManager.default.fileExists(atPath: scriptURL.path))
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
        let launchGateURL = statusURL.appendingPathExtension("lock")
        try Data().write(to: launchGateURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: launchGateURL.path)
    }

    private func writeToken(_ token: String, to url: URL) throws {
        let writer = try FileHandle(forWritingTo: url)
        try writer.write(contentsOf: Data("\(token)\n".utf8))
        try writer.close()
    }

    private func writeExecutable(_ contents: String, named name: String, in root: URL) throws -> URL {
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        let url = root.appendingPathComponent(name)
        try Data(contents.utf8).write(to: url)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: url.path)
        return url
    }

    private func isRunning(_ pid: pid_t, in root: URL) -> Bool {
        guard kill(pid, 0) == 0 else { return false }
        let process = isolatedProcess(
            executable: URL(fileURLWithPath: "/bin/ps"),
            arguments: ["-o", "stat=", "-p", String(pid)],
            in: root
        )
        let output = Pipe()
        process.standardOutput = output
        guard (try? process.run()) != nil else { return false }
        process.waitUntilExit()
        let state = String(decoding: output.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return !state.isEmpty && !state.hasPrefix("Z")
    }

    private func temporaryDirectory() -> URL {
        FileManager.default.temporaryDirectory.appendingPathComponent(
            "antigravity-auth-script-\(UUID().uuidString)",
            isDirectory: true
        )
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
}
