import Darwin
import Foundation
import Testing
@testable import AIMeterApp
@testable import AIMeterCore

@Suite("CLI authentication launcher", .serialized)
@MainActor
struct CLIAuthenticationLauncherTests {
    @Test("Claude, Codex, and Antigravity login scripts are private and opened")
    func writesPrivateScripts() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        var opened: [URL] = []
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root,
            executableLocator: AuthenticationFixedLocator(),
            openURL: { opened.append($0); return true }
        )

        let claudeURL = try launcher.open(provider: .claude)
        let codexURL = try launcher.open(provider: .codex)
        let completionToken = "12345678-1234-1234-1234-123456789abc"
        let antigravityURL = try launcher.open(
            provider: .gemini,
            completionToken: completionToken
        )

        #expect(claudeURL.lastPathComponent == "Open Claude Login.command")
        #expect(codexURL.lastPathComponent == "Open Codex Login.command")
        #expect(antigravityURL.lastPathComponent.hasPrefix("Open Antigravity Login "))
        #expect(antigravityURL.pathExtension == "command")
        #expect(opened == [claudeURL, codexURL, antigravityURL])
        #expect(permissions(of: claudeURL) == 0o700)
        #expect(permissions(of: codexURL) == 0o700)
        #expect(permissions(of: antigravityURL) == 0o700)
        #expect(try String(contentsOf: claudeURL, encoding: .utf8).contains("auth login"))
        #expect(try String(contentsOf: codexURL, encoding: .utf8).contains("codex' login"))
        #expect(try String(contentsOf: antigravityURL, encoding: .utf8).contains("agy"))
        let script = try String(contentsOf: antigravityURL, encoding: .utf8)
        #expect(script.contains("aitokenmeter://antigravity-login-complete?token=${completion_token}"))
        #expect(!script.contains(completionToken))
        let tokenPipe = try #require(try FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: nil)
            .first(where: { $0.pathExtension == "token" }))
        #expect(permissions(of: tokenPipe) == 0o600)
        launcher.cancelPendingGeminiLogin(token: completionToken)
        #expect(!FileManager.default.fileExists(atPath: tokenPipe.path))
    }

    @Test("Overlapping Antigravity opens keep each launch script at a unique URL")
    func overlappingGeminiOpensUseDistinctScriptURLs() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        var delayedOpenRequests: [URL] = []
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root,
            executableLocator: AuthenticationFixedLocator(),
            openURL: { url in
                delayedOpenRequests.append(url)
                return true
            }
        )
        let firstToken = "12345678-1234-1234-1234-123456789abc"
        let secondToken = "87654321-4321-4321-4321-cba987654321"

        let firstURL = try launcher.open(provider: .gemini, completionToken: firstToken)
        let firstTokenPipe = try #require(try FileManager.default.contentsOfDirectory(
            at: root,
            includingPropertiesForKeys: nil
        ).first(where: { $0.pathExtension == "token" }))
        let secondURL = try launcher.open(provider: .gemini, completionToken: secondToken)
        let tokenPipes = try FileManager.default.contentsOfDirectory(
            at: root,
            includingPropertiesForKeys: nil
        ).filter { $0.pathExtension == "token" }
        let secondTokenPipe = try #require(tokenPipes.first(where: { $0 != firstTokenPipe }))

        // Enumeration may use /private/var while the injected root uses /var.
        // Preserve that root's spelling when checking each full pipe path.
        let firstPipePath = root.appendingPathComponent(firstTokenPipe.lastPathComponent).path
        let secondPipePath = root.appendingPathComponent(secondTokenPipe.lastPathComponent).path

        #expect(firstURL != secondURL)
        #expect(delayedOpenRequests == [firstURL, secondURL])
        let delayedFirstScript = try String(contentsOf: delayedOpenRequests[0], encoding: .utf8)
        let delayedSecondScript = try String(contentsOf: delayedOpenRequests[1], encoding: .utf8)
        #expect(delayedFirstScript.contains(firstPipePath))
        #expect(!delayedFirstScript.contains(secondPipePath))
        #expect(delayedSecondScript.contains(secondPipePath))
        #expect(!delayedSecondScript.contains(firstPipePath))

        launcher.cancelPendingGeminiLogin(token: firstToken)
        launcher.cancelPendingGeminiLogin(token: secondToken)
    }

    @Test("An indeterminate Antigravity exec status is reported as cleanup uncertainty")
    func launchIndeterminateExecStatusReportsCleanupUncertainty() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let token = UUID().uuidString
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root,
            executableLocator: AuthenticationFixedLocator(),
            openURL: { _ in true }
        )
        try launcher.open(provider: .gemini, completionToken: token)
        defer { launcher.cancelPendingGeminiLogin(token: token) }

        let statusURL = try #require(try FileManager.default.contentsOfDirectory(
            at: root,
            includingPropertiesForKeys: nil
        ).first(where: { $0.pathExtension == "status" }))
        let execStatusURL = statusURL.appendingPathExtension("exec")
        try Data("launch_indeterminate\n".utf8).write(to: execStatusURL, options: .atomic)

        #expect(try String(contentsOf: statusURL, encoding: .utf8) == "ready\n")
        #expect(try String(contentsOf: execStatusURL, encoding: .utf8) == "launch_indeterminate\n")
        #expect(launcher.hasGeminiCleanupUncertainty(token: token))
    }

    @Test("Terminal status consumer reads synthetic helper and exec failures")
    func terminalStatusConsumerReadsSyntheticFailureFiles() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let token = UUID().uuidString
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root,
            executableLocator: AuthenticationFixedLocator(),
            openURL: { _ in true }
        )
        try launcher.open(provider: .gemini, completionToken: token)
        defer { launcher.cancelPendingGeminiLogin(token: token) }

        let statusURL = try #require(try FileManager.default.contentsOfDirectory(
            at: root,
            includingPropertiesForKeys: nil
        ).first(where: { $0.pathExtension == "status" }))
        let execStatusURL = statusURL.appendingPathExtension("exec")

        try Data("callback_failed\n".utf8).write(to: statusURL, options: .atomic)
        #expect(launcher.hasGeminiTerminalFailure(token: token))

        try Data("helper_stop_unconfirmed\n".utf8).write(to: statusURL, options: .atomic)
        #expect(launcher.hasGeminiTerminalFailure(token: token))
        #expect(launcher.hasGeminiCleanupUncertainty(token: token))

        try Data("process_state_unavailable\n".utf8).write(to: statusURL, options: .atomic)
        #expect(launcher.hasGeminiTerminalFailure(token: token))
        #expect(launcher.hasGeminiCleanupUncertainty(token: token))

        try Data("ready\n".utf8).write(to: statusURL, options: .atomic)
        try Data("exec_failed\n".utf8).write(to: execStatusURL, options: .atomic)
        #expect(launcher.hasGeminiTerminalFailure(token: token))

        try Data("ready\n".utf8).write(to: statusURL, options: .atomic)
        try Data("exec_confirmed\n".utf8).write(to: execStatusURL, options: .atomic)
        #expect(!launcher.hasGeminiTerminalFailure(token: token))
    }

    @Test("Codex login script uses the nested ChatGPT app-bundled CLI")
    func nestedBundledCodexLoginScript() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let applications = root.appendingPathComponent("Applications", isDirectory: true)
        let home = root.appendingPathComponent("home", isDirectory: true)
        let executable = home.appendingPathComponent(
            "Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex"
        )
        try FileManager.default.createDirectory(
            at: executable.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try Data("#!/bin/sh\nexit 0\n".utf8).write(to: executable)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: executable.path)
        let locator = ExecutableLocator(
            searchPaths: [],
            bundledExecutablePaths: ExecutableLocator.bundledExecutablePaths(
                homeDirectory: home,
                systemApplications: applications
            )
        )
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root.appendingPathComponent("authentication"),
            executableLocator: locator,
            openURL: { _ in true }
        )

        let scriptURL = try launcher.open(provider: .codex)
        let script = try String(contentsOf: scriptURL, encoding: .utf8)

        #expect(script.contains("exec '\(executable.path)' login"))
    }

    @Test("A default test-process launch is refused before writing a login script")
    func defaultTestProcessLaunchIsFailClosed() {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let authenticationDirectory = root.appendingPathComponent("Authentication", isDirectory: true)
        let policy = SystemActionPolicy(
            environment: ["XCTestConfigurationFilePath": "/tmp/fake-tests.xctest"],
            isXCTestBundle: false
        )
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: authenticationDirectory,
            executableLocator: AuthenticationFixedLocator(),
            systemActionPolicy: policy
        )

        #expect(throws: CLIAuthenticationLaunchError.externalActionsDisabled) {
            try launcher.open(
                provider: .gemini,
                completionToken: "12345678-1234-1234-1234-123456789abc"
            )
        }
        #expect(!FileManager.default.fileExists(atPath: authenticationDirectory.path))
    }

    @Test("A test-policy Antigravity script cannot call the system URL opener")
    func testPolicyDisablesScriptCallbackOpener() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let policy = SystemActionPolicy(
            environment: ["XCTestConfigurationFilePath": "/tmp/fake-tests.xctest"],
            isXCTestBundle: false
        )
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root,
            executableLocator: AuthenticationFixedLocator(),
            openURL: { _ in true },
            systemActionPolicy: policy
        )

        let scriptURL = try launcher.open(
            provider: .gemini,
            completionToken: "12345678-1234-1234-1234-123456789abc"
        )
        defer { launcher.cancelPendingGeminiLogin(token: "12345678-1234-1234-1234-123456789abc") }
        let script = try String(contentsOf: scriptURL, encoding: .utf8)

        #expect(script.contains("open_command='/usr/bin/false'"))
        #expect(!script.contains("open_command='/usr/bin/open'"))
    }

    @Test("A missing executable produces a provider-specific failure")
    func missingCLI() {
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: temporaryDirectory(),
            executableLocator: AuthenticationMissingLocator(),
            openURL: { _ in true }
        )

        #expect(throws: CLIAuthenticationLaunchError.notInstalled(.claude)) {
            try launcher.open(provider: .claude)
        }
    }

    @Test("Terminal open failure remains visible to the caller")
    func openFailure() {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root,
            executableLocator: AuthenticationFixedLocator(),
            openURL: { _ in false }
        )

        #expect(throws: CLIAuthenticationLaunchError.couldNotOpenTerminal) {
            try launcher.open(provider: .codex)
        }
    }

    @Test("A busy login gate retries cancellation and preserves it without helper acknowledgement")
    func stopRetriesAfterBusyLoginGateIsReleased() async throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let token = UUID().uuidString
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root,
            executableLocator: AuthenticationFixedLocator(),
            openURL: { _ in true },
            geminiStopGateLockTimeout: .seconds(2),
            geminiStopAcknowledgementTimeout: .milliseconds(250)
        )
        try launcher.open(provider: .gemini, completionToken: token)
        let statusURL = try #require(try FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: nil)
            .first(where: { $0.pathExtension == "status" }))
        let gateURL = statusURL.appendingPathExtension("lock")
        let readyURL = root.appendingPathComponent("gate-held")
        let locker = Process()
        locker.executableURL = URL(fileURLWithPath: "/usr/bin/perl")
        locker.arguments = [
            "-MFcntl",
            "-e",
            "use Fcntl qw(:DEFAULT); my ($path, $ready) = @ARGV; open my $fh, '+<', $path or die; my $lock = pack('qqiss', 0, 0, 0, F_WRLCK, SEEK_SET); fcntl($fh, F_SETLKW, $lock) or die; open my $marker, '>', $ready or die; print {$marker} 'ready'; close $marker; sleep 30;",
            gateURL.path,
            readyURL.path
        ]
        try locker.run()
        defer {
            if locker.isRunning {
                locker.terminate()
                locker.waitUntilExit()
            }
        }
        let readyDeadline = ContinuousClock.now.advanced(by: .seconds(2))
        while !FileManager.default.fileExists(atPath: readyURL.path), ContinuousClock.now < readyDeadline {
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(FileManager.default.fileExists(atPath: readyURL.path))

        let startedAt = ContinuousClock.now
        let stopTask = launcher.requestGeminiLoginStop(token: token)
        let elapsed = startedAt.duration(to: ContinuousClock.now)

        #expect(elapsed < .seconds(1))
        #expect(try String(contentsOf: statusURL, encoding: .utf8) == "ready\n")
        #expect(FileManager.default.fileExists(atPath: gateURL.path))

        locker.terminate()
        locker.waitUntilExit()
        let cancellationDeadline = ContinuousClock.now.advanced(by: .seconds(2))
        while ContinuousClock.now < cancellationDeadline,
              (try? String(contentsOf: statusURL.appendingPathExtension("cancel"), encoding: .utf8)) != "cancelled\n" {
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(try String(contentsOf: statusURL.appendingPathExtension("cancel"), encoding: .utf8) == "cancelled\n")
        #expect(try String(contentsOf: statusURL, encoding: .utf8) == "ready\n")

        try await Task.sleep(for: .milliseconds(400))
        #expect(try String(contentsOf: statusURL.appendingPathExtension("cancel"), encoding: .utf8) == "cancelled\n")
        #expect(try String(contentsOf: statusURL, encoding: .utf8) == "ready\n")
        #expect(FileManager.default.fileExists(atPath: gateURL.path))
        #expect(FileManager.default.fileExists(atPath: statusURL.appendingPathExtension("exec").path))
        #expect(await stopTask.value == false)
    }

    @Test("A busy login gate preserves the fallback cancellation marker without helper acknowledgement")
    func stopWritesCancellationMarkerWhenGateRemainsBusy() async throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let token = UUID().uuidString
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root,
            executableLocator: AuthenticationFixedLocator(),
            openURL: { _ in true },
            geminiStopGateLockTimeout: .milliseconds(150),
            geminiStopAcknowledgementTimeout: .milliseconds(800)
        )
        try launcher.open(provider: .gemini, completionToken: token)
        let statusURL = try #require(try FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: nil)
            .first(where: { $0.pathExtension == "status" }))
        let gateURL = statusURL.appendingPathExtension("lock")
        let readyURL = root.appendingPathComponent("gate-held")
        let locker = Process()
        locker.executableURL = URL(fileURLWithPath: "/usr/bin/perl")
        locker.arguments = [
            "-MFcntl",
            "-e",
            "use Fcntl qw(:DEFAULT); my ($path, $ready) = @ARGV; open my $fh, '+<', $path or die; my $lock = pack('qqiss', 0, 0, 0, F_WRLCK, SEEK_SET); fcntl($fh, F_SETLKW, $lock) or die; open my $marker, '>', $ready or die; print {$marker} 'ready'; close $marker; sleep 30;",
            gateURL.path,
            readyURL.path
        ]
        try locker.run()
        defer {
            if locker.isRunning {
                locker.terminate()
                locker.waitUntilExit()
            }
        }
        let readyDeadline = ContinuousClock.now.advanced(by: .seconds(2))
        while !FileManager.default.fileExists(atPath: readyURL.path), ContinuousClock.now < readyDeadline {
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(FileManager.default.fileExists(atPath: readyURL.path))

        launcher.requestGeminiLoginStop(token: token)
        let cancellationDeadline = ContinuousClock.now.advanced(by: .seconds(2))
        while ContinuousClock.now < cancellationDeadline,
              (try? String(contentsOf: statusURL.appendingPathExtension("cancel"), encoding: .utf8)) != "cancelled\n" {
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(try String(contentsOf: statusURL.appendingPathExtension("cancel"), encoding: .utf8) == "cancelled\n")
        #expect(try String(contentsOf: statusURL, encoding: .utf8) == "ready\n")

        locker.terminate()
        locker.waitUntilExit()
        try await Task.sleep(for: .milliseconds(1_000))
        #expect(try String(contentsOf: statusURL.appendingPathExtension("cancel"), encoding: .utf8) == "cancelled\n")
        #expect(try String(contentsOf: statusURL, encoding: .utf8) == "ready\n")
        #expect(FileManager.default.fileExists(atPath: gateURL.path))
        #expect(FileManager.default.fileExists(atPath: statusURL.appendingPathExtension("exec").path))
    }

    @Test("A helper stop acknowledgement completes the launcher stop task and removes coordination files")
    func stopAcknowledgementReturnsSuccessAndCleansFiles() async throws {
        for finishedFailure in [false, true] {
            let root = temporaryDirectory()
            defer { try? FileManager.default.removeItem(at: root) }
            let token = UUID().uuidString
            let launcher = CLIAuthenticationLauncher(
                authenticationDirectoryURL: root,
                executableLocator: AuthenticationFixedLocator(),
                openURL: { _ in true },
                geminiStopAcknowledgementTimeout: .seconds(1)
            )
            try launcher.open(provider: .gemini, completionToken: token)
            let statusURL = try #require(try FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: nil)
                .first(where: { $0.pathExtension == "status" }))
            let gateURL = statusURL.appendingPathExtension("lock")
            let execStatusURL = statusURL.appendingPathExtension("exec")

            let stoppedURL = statusURL.appendingPathExtension("stopped")
            if finishedFailure {
                try Data("callback_failed\n".utf8).write(to: statusURL, options: .atomic)
                // An unknown or unrelated marker must not release ownership.
                try Data("not-a-cleanup-receipt\n".utf8).write(to: stoppedURL, options: .atomic)
            }
            let stopTask = launcher.requestGeminiLoginStop(token: token)
            let cancellationDeadline = ContinuousClock.now.advanced(by: .seconds(2))
            while ContinuousClock.now < cancellationDeadline,
                  (try? String(contentsOf: statusURL.appendingPathExtension("cancel"), encoding: .utf8)) != "cancelled\n" {
                try await Task.sleep(for: .milliseconds(10))
            }
            #expect(try String(contentsOf: statusURL.appendingPathExtension("cancel"), encoding: .utf8) == "cancelled\n")
            #expect(try String(contentsOf: statusURL, encoding: .utf8) == (finishedFailure ? "callback_failed\n" : "ready\n"))
            if finishedFailure {
                try Data("helper_stopped_v1:\(statusURL.lastPathComponent):callback_failed\n".utf8)
                    .write(to: stoppedURL, options: .atomic)
            } else {
                try Data("helper_cancelled\n".utf8).write(to: statusURL, options: .atomic)
            }

            #expect(await stopTask.value)
            #expect(!FileManager.default.fileExists(atPath: stoppedURL.path))
            #expect(!FileManager.default.fileExists(atPath: statusURL.path))
            #expect(!FileManager.default.fileExists(atPath: gateURL.path))
            #expect(!FileManager.default.fileExists(atPath: execStatusURL.path))
        }
    }

    @Test("A stop that times out can consume a late acknowledgement on retry")
    func stopCanBeRetriedAfterMissingAcknowledgement() async throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let token = UUID().uuidString
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root,
            executableLocator: AuthenticationFixedLocator(),
            openURL: { _ in true },
            geminiStopAcknowledgementTimeout: .milliseconds(20)
        )
        try launcher.open(provider: .gemini, completionToken: token)
        let statusURL = try #require(try FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: nil)
            .first(where: { $0.pathExtension == "status" }))

        let firstStop = launcher.requestGeminiLoginStop(token: token)
        #expect(await !firstStop.value)
        #expect(FileManager.default.fileExists(atPath: statusURL.path))

        try Data("helper_cancelled\n".utf8).write(to: statusURL, options: .atomic)
        let retry = launcher.requestGeminiLoginStop(token: token)

        #expect(await retry.value)
        #expect(!FileManager.default.fileExists(atPath: statusURL.path))
    }

    // Regression: a stop task writing the response file after a busy gate wait
    // would lose either the late acknowledgement or the uncertainty proof.
    @Test("Concurrent stops preserve terminal proofs published while the launch gate is busy",
          arguments: ["helper_cancelled", "helper_stop_unconfirmed", "process_state_unavailable"])
    func concurrentStopsKeepLateTerminalProof(status: String) async throws {
        let root = temporaryDirectory()
        var gateOwner: AuthenticationGateOwner?
        defer {
            if gateOwner?.mustPreserveRoot != true { try? FileManager.default.removeItem(at: root) }
        }
        let token = UUID().uuidString
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root, executableLocator: AuthenticationFixedLocator(), openURL: { _ in true },
            geminiStopGateLockTimeout: .seconds(2), geminiStopAcknowledgementTimeout: .seconds(1)
        )
        let script = try launcher.open(provider: .gemini, completionToken: token)
        let response = try #require(try FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: nil)
            .first(where: { $0.pathExtension == "status" }))
        let request = response.appendingPathExtension("cancel")
        let owner = try AuthenticationGateOwner(gate: response.appendingPathExtension("lock"), in: root)
        gateOwner = owner
        defer { owner.finish() }
        try #require(await owner.waitUntilLocked())
        let first = launcher.requestGeminiLoginStop(token: token)
        let second = launcher.requestGeminiLoginStop(token: token)
        let deadline = ContinuousClock.now.advanced(by: .seconds(1))
        while (try? String(contentsOf: request, encoding: .utf8)) != "cancelled\n", ContinuousClock.now < deadline {
            await Task.yield()
        }
        #expect(try String(contentsOf: request, encoding: .utf8) == "cancelled\n")
        try Data("\(status)\n".utf8).write(to: response, options: .atomic)
        owner.finish()
        let firstResult = await first.value
        let secondResult = await second.value
        #expect(firstResult == (status == "helper_cancelled"))
        #expect(secondResult == firstResult)
        if status == "helper_cancelled" {
            #expect(!FileManager.default.fileExists(atPath: response.path))
            #expect(!FileManager.default.fileExists(atPath: request.path))
            #expect(!FileManager.default.fileExists(atPath: script.path))
        } else {
            #expect(try String(contentsOf: response, encoding: .utf8) == "\(status)\n")
            #expect(FileManager.default.fileExists(atPath: request.path))
            #expect(FileManager.default.fileExists(atPath: script.path))
        }
    }

    @Test("A retry does not overwrite an uncertain cleanup result")
    func uncertainStopStatusRemainsPreservedOnRetry() async throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let token = UUID().uuidString
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root,
            executableLocator: AuthenticationFixedLocator(),
            openURL: { _ in true }
        )
        try launcher.open(provider: .gemini, completionToken: token)
        let statusURL = try #require(try FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: nil)
            .first(where: { $0.pathExtension == "status" }))
        try Data("helper_stop_unconfirmed\n".utf8).write(to: statusURL, options: .atomic)
        let stoppedURL = statusURL.appendingPathExtension("stopped")
        // Even a syntactically bound ACK cannot override an uncertain status.
        try Data("helper_stopped_v1:\(statusURL.lastPathComponent):helper_stop_unconfirmed\n".utf8)
            .write(to: stoppedURL, options: .atomic)

        let retry = launcher.requestGeminiLoginStop(token: token)

        #expect(await !retry.value)
        #expect(try String(contentsOf: statusURL, encoding: .utf8) == "helper_stop_unconfirmed\n")
        #expect(FileManager.default.fileExists(atPath: stoppedURL.path))
    }

    @Test("DeepSeek is rejected before any file is written")
    func rejectsDeepSeek() {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root,
            executableLocator: AuthenticationFixedLocator(),
            openURL: { _ in true }
        )

        #expect(throws: CLIAuthenticationLaunchError.unsupportedProvider) {
            try launcher.open(provider: .deepSeek)
        }
        #expect(!FileManager.default.fileExists(atPath: root.path))
    }

    private func temporaryDirectory() -> URL {
        FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-meter-auth-\(UUID().uuidString)", isDirectory: true)
    }

    private func permissions(of url: URL) -> Int {
        let attributes = try? FileManager.default.attributesOfItem(atPath: url.path)
        return (attributes?[.posixPermissions] as? NSNumber)?.intValue ?? 0
    }
}

private struct AuthenticationFixedLocator: ExecutableLocating {
    func locate(named name: String) -> URL? {
        URL(fileURLWithPath: "/tmp/\(name.uppercased()) CLI/\(name)")
    }
}

private struct AuthenticationMissingLocator: ExecutableLocating {
    func locate(named name: String) -> URL? { nil }
}

@MainActor
private final class AuthenticationGateOwner {
    private let root: URL
    private(set) var mustPreserveRoot = false
    private let pid: pid_t
    private let ready: URL
    private var reaped = false
    private var known = true
    private var status: Int32 = 0

    init(gate: URL, in root: URL) throws {
        self.root = root
        ready = root.appendingPathComponent("held-gate-\(UUID().uuidString)")
        let program = "use Fcntl qw(:DEFAULT); my ($path, $ready) = @ARGV; open my $g, '+<', $path or die $!; my $l = pack('qqiss', 0, 0, 0, F_WRLCK, SEEK_SET); defined(fcntl($g, F_SETLKW, $l)) or die $!; open my $f, '>', $ready or die $!; close $f; sleep 30;"
        let args = ["/usr/bin/env", "-i", "HOME=\(root.path)", "ZDOTDIR=\(root.path)", "TMPDIR=\(root.path)",
                    "PATH=/usr/bin:/bin", "/usr/bin/perl", "-e", program, gate.path, ready.path]
        let pointers = args.map { strdup($0)! } + [nil]
        defer { pointers.forEach { free($0) } }
        let executable = strdup("/usr/bin/env")!
        defer { free(executable) }
        var child: pid_t = 0
        let environment: [UnsafeMutablePointer<CChar>?] = [nil]
        let spawnError = pointers.withUnsafeBufferPointer { argv in
            environment.withUnsafeBufferPointer { env in
                posix_spawn(&child, executable, nil, nil, argv.baseAddress!, env.baseAddress!)
            }
        }
        guard spawnError == 0, child > 0 else {
            throw NSError(domain: NSPOSIXErrorDomain, code: Int(spawnError == 0 ? ECHILD : spawnError))
        }
        pid = child
    }

    func waitUntilLocked() async -> Bool {
        let deadline = ContinuousClock.now.advanced(by: .seconds(2))
        while ContinuousClock.now < deadline, !reaped, known {
            if FileManager.default.fileExists(atPath: ready.path) { return true }
            observe()
            await Task.yield()
        }
        return false
    }

    func finish() {
        observe()
        for signal in [SIGTERM, SIGKILL] where !reaped && known {
            _ = Darwin.kill(pid, signal)
            let deadline = ContinuousClock.now.advanced(by: .milliseconds(200))
            while !reaped, known, ContinuousClock.now < deadline { observe(); Thread.sleep(forTimeInterval: 0.002) }
        }
        if !reaped || !known {
            mustPreserveRoot = true
            try? Data("Gate-owner cleanup identity unknown\n".utf8)
                .write(to: root.appendingPathComponent(".preserve-for-inspection"), options: .atomic)
        }
    }

    private func observe() {
        guard !reaped, known else { return }
        let observed = waitpid(pid, &status, WNOHANG)
        if observed == pid { reaped = true }
        else if observed < 0, errno != EINTR { known = false }
    }
}
