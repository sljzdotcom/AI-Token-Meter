import Darwin
import Foundation
import Testing
@testable import AIMeterApp
@testable import AIMeterCore

@Suite("Antigravity login recovery end-to-end", .serialized)
@MainActor
struct GeminiLoginRecoveryEndToEndTests {
    @Test("Production launcher handles isolated PTY callback delivery and acknowledged expiry", arguments: [0, 1, 2, 3, 4, 5])
    func productionLauncherHandshake(completionExitCode: Int) async throws {
        try await runProductionLauncherHandshake(completionExitCode: completionExitCode)
    }

    @Test("Running-CLI expiry waits through watchdog readiness delayed beyond two seconds")
    func delayedRunningCLIExpiry() async throws {
        try await runProductionLauncherHandshake(completionExitCode: 3, watchdogDelaySeconds: 3)
    }

    @Test("Pre-exec expiry acknowledges cancellation without requiring a CLI PID")
    func preExecExpiryIsSeparateFromRunningCLIExpiry() async throws {
        try await runProductionLauncherHandshake(completionExitCode: 3, watchdogDelaySeconds: 3, cancelBeforeExec: true)
    }

    private func runProductionLauncherHandshake(
        completionExitCode: Int, watchdogDelaySeconds: Int = 0, cancelBeforeExec: Bool = false
    ) async throws {
        let root = try temporaryRoot()
        var fixture: TemporaryFullFlowPTY?
        var preserveRoot = true // v19 diagnostic policy: retain every synthetic root.
        defer {
            if !preserveRoot, fixture?.mustPreserveRoot != true { removeUnlessPreserved(root) }
        }
        let authenticationDirectory = root.appendingPathComponent("Authentication", isDirectory: true)
        let cliLog = root.appendingPathComponent("agy.log")
        let cliPIDFile = root.appendingPathComponent("agy.pid")
        let callbackLog = root.appendingPathComponent("callbacks.log")
        let watchdogPIDFile = root.appendingPathComponent("watchdog.pid")
        // The fake CLI proves it can read the private controlling tty before it
        // exits. Expiry keeps that same tracked PID alive using an exec'd sleep.
        let expiryGate = FullFlowExpiryGate()
        defer { Task { await expiryGate.cancel() } }
        let fakeCLI = try writeExecutable(
            "#!/bin/sh\nprintf '%s' \"$$\" > '\(cliPIDFile.path)'\nprintf 'agy-started\\n' > '\(cliLog.path)'\nprintf 'fake-cli-ready\\n'\nIFS= read -r approval </dev/tty || exit 9\n[ \"$approval\" = approved ] || exit 10\nprintf 'fake-cli-approved\\n'\n"
                + (completionExitCode == 3 ? "exec /bin/sleep 30\n" : "exit 0\n"),
            named: "agy", in: root
        )
        let fakeOpen = try writeExecutable(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '\(callbackLog.path)'\nexit \(completionExitCode == 1 ? 1 : 0)\n",
            named: "open", in: root
        )
        let watchdogBody: String
        switch completionExitCode {
        case 4: watchdogBody = "exit 1\n"
        case 5: watchdogBody = "exit 0\n"
        default: watchdogBody = "exec /usr/bin/perl \"$@\"\n"
        }
        let watchdogContents = watchdogDelaySeconds > 0
            ? "#!/usr/bin/perl\nuse Time::HiRes qw(sleep); open my $f, '>', '\(watchdogPIDFile.path)' or die $!; print {$f} \"$$\"; close $f; sleep \(watchdogDelaySeconds); exec '/usr/bin/perl', @ARGV;\n"
            : "#!/bin/sh\nprintf '%s' \"$$\" > '\(watchdogPIDFile.path)'\n" + watchdogBody
        let fakeWatchdog = try writeExecutable(watchdogContents, named: "watchdog", in: root)
        var openedScriptURL: URL?
        var openedToken: String?
        defer { fixture?.cleanupOwnedWrapper() }
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: authenticationDirectory,
            executableLocator: TemporaryFullFlowLocator(executable: fakeCLI),
            openURL: { scriptURL in
                guard scriptURL.deletingLastPathComponent().standardizedFileURL == authenticationDirectory.standardizedFileURL,
                      scriptURL.lastPathComponent.hasPrefix("Open Antigravity Login "),
                      scriptURL.pathExtension == "command",
                      let generated = try? String(contentsOf: scriptURL, encoding: .utf8),
                      generated.contains("open_command='\(fakeOpen.path)'"),
                      !generated.contains("/usr/bin/open") else { return false }
                if completionExitCode == 2,
                   let pipe = try? FileManager.default.contentsOfDirectory(at: authenticationDirectory, includingPropertiesForKeys: nil)
                    .first(where: { $0.pathExtension == "token" }) {
                    try? FileManager.default.removeItem(at: pipe)
                }
                do {
                    fixture = try TemporaryFullFlowPTY(scriptURL: scriptURL, in: root)
                    openedScriptURL = scriptURL
                    return true
                } catch {
                    preserveRoot = true
                    return false
                }
            },
            completionOpenExecutableURL: fakeOpen,
            watchdogExecutableURL: fakeWatchdog,
            geminiStopAcknowledgementTimeout: .seconds(3)
        )
        let defaultsName = "TemporaryFullGeminiLauncherFlowTests.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: defaultsName))
        defer { defaults.removePersistentDomain(forName: defaultsName) }
        let model = AppModel(
            defaults: defaults,
            applicationSupportDirectoryURL: root.appendingPathComponent("app-support", isDirectory: true),
            secretStore: TemporaryFullFlowSecretStore(),
            launchAtLoginStatusOperation: { false },
            authenticationLauncher: launcher,
            widgetSnapshotPublisher: nil,
            isDemoMode: false,
            geminiDiagnosticStore: GeminiDiagnosticStore(fileURL: nil),
            refreshOperation: { [] },
            geminiPauseReasonOperation: { .unknown },
            geminiRecoveryBeginOperation: { _ in true },
            geminiRecoveryAuthorizeOperation: { _ in GeminiQuotaRecoveryAuthorization(id: UUID()) },
            geminiQuotaCheckOperation: { _ in
                let metric = UsageMetric(label: "Shared weekly", current: 80, limit: 100, unit: .percent)
                let sharedFiveHour = UsageMetric(
                    label: "Claude/GPT · Five hour", current: 80, limit: 100,
                    unit: .percent, kind: .officialLimit, resetAt: Date(timeIntervalSince1970: 1_800)
                )
                let sharedWeekly = UsageMetric(
                    label: "Claude/GPT · Weekly", current: 20, limit: 100,
                    unit: .percent, kind: .officialLimit, resetAt: Date(timeIntervalSince1970: 2_400)
                )
                return UsageSnapshot(
                    provider: .gemini, primaryMetric: metric, fetchedAt: Date(), collectionStatus: .fresh,
                    geminiQuotaMetrics: [metric], antigravitySharedQuotaMetrics: [sharedFiveHour, sharedWeekly]
                )
            },
            serviceAccountRefreshOperation: { _ in [] },
            geminiAuthenticationOpenOperation: { token in
                openedToken = token
                _ = try launcher.open(provider: .gemini, completionToken: token)
            },
            geminiAuthenticationCancelOperation: { launcher.cancelPendingGeminiLogin(token: $0) },
            geminiAuthenticationStopOperation: { launcher.requestGeminiLoginStop(token: $0) },
            geminiAuthenticationFailureOperation: { launcher.hasGeminiTerminalFailure(token: $0) },
            geminiAuthenticationCleanupUncertainOperation: { launcher.hasGeminiCleanupUncertainty(token: $0) },
            geminiAuthenticationExecConfirmedOperation: { launcher.hasGeminiCLIExecConfirmed(token: $0) },
            geminiRecoveryTimeout: completionExitCode == 3 ? .seconds(2) : .seconds(310),
            signInSleep: { duration in
                if completionExitCode == 3, !cancelBeforeExec { try await expiryGate.sleep(duration: duration) }
                else { try await Task.sleep(for: duration) }
            }
        )
        defer { model.stop() }
        await model.refresh()
        #expect(model.isGeminiRefreshPaused)
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        let scriptURL = try #require(openedScriptURL)
        let pty = try #require(fixture)
        let groupMarker = try #require(try FileManager.default.contentsOfDirectory(
            at: authenticationDirectory, includingPropertiesForKeys: nil
        ).first(where: { $0.pathExtension == "status" })).appendingPathExtension("process-group")
        if completionExitCode == 3, !cancelBeforeExec {
            let entered = await expiryGate.waitUntilSleeping(timeout: .seconds(3))
            guard entered else {
                preserveRoot = true
                Issue.record("Expiry monitor did not enter its bounded test gate")
                await expiryGate.cancel()
                return
            }
        }
        let runningReady: (@MainActor () async -> Void)?
        if completionExitCode == 3 && !cancelBeforeExec {
            runningReady = { await expiryGate.release() }
        } else {
            runningReady = nil
        }
        let result = await pty.finish(
            cliPIDFile: cliPIDFile, watchdogPIDFile: watchdogPIDFile, groupMarker: groupMarker,
            cliRequired: [0, 1, 3].contains(completionExitCode) && !cancelBeforeExec, watchdogRequired: completionExitCode != 2,
            synthesizeStaleReadiness: completionExitCode == 5,
            runningReady: runningReady
        )
        // Snapshot A precedes the next suspension and any callback parsing failure.
        var diagnosticProgress = "afterFinish"
        G11DiagnosticV19.emit(
            phase: "A", progress: diagnosticProgress, completionExitCode: completionExitCode,
            watchdogDelaySeconds: watchdogDelaySeconds, cancelBeforeExec: cancelBeforeExec,
            root: root, scriptURL: scriptURL, groupMarker: groupMarker,
            cliLog: cliLog, cliPIDFile: cliPIDFile, watchdogPIDFile: watchdogPIDFile, callbackLog: callbackLog,
            token: token, modelState: String(describing: model.geminiOneTimeRecoveryState), isRefreshPaused: model.isGeminiRefreshPaused,
            finished: result.finished, cleanupSucceeded: result.cleanupSucceeded,
            terminationStatus: result.terminationStatus, ptyOutput: result.output
        )
        // Registered after model.stop/fixture defers, so B runs before them.
        defer {
            G11DiagnosticV19.emit(
                phase: "B", progress: diagnosticProgress, completionExitCode: completionExitCode,
                watchdogDelaySeconds: watchdogDelaySeconds, cancelBeforeExec: cancelBeforeExec,
                root: root, scriptURL: scriptURL, groupMarker: groupMarker,
                cliLog: cliLog, cliPIDFile: cliPIDFile, watchdogPIDFile: watchdogPIDFile, callbackLog: callbackLog,
                token: token, modelState: String(describing: model.geminiOneTimeRecoveryState), isRefreshPaused: model.isGeminiRefreshPaused
            )
        }
        await expiryGate.cancel()
        #expect(result.finished, "PTY transcript: \(result.output)")
        #expect(result.cleanupSucceeded, "Synthetic process ownership was not fully confirmed")
        if completionExitCode == 2 {
            #expect(result.terminationStatus == 124)
            #expect(!FileManager.default.fileExists(atPath: cliLog.path))
        } else if completionExitCode == 3 {
            #expect(result.terminationStatus == 130)
            let deadline = ContinuousClock.now.advanced(by: .seconds(3))
            while model.geminiOneTimeRecoveryState == .stoppingLogin, ContinuousClock.now < deadline {
                await Task.yield()
            }
            #expect(model.geminiOneTimeRecoveryState == .loginExpired)
            #expect(FileManager.default.fileExists(atPath: cliLog.path) == !cancelBeforeExec)
        } else if completionExitCode == 4 || completionExitCode == 5 {
            #expect(result.terminationStatus == 127)
            #expect(!FileManager.default.fileExists(atPath: cliLog.path))
        } else {
            #expect(result.terminationStatus == 0)
            #expect(FileManager.default.fileExists(atPath: cliLog.path))
        }
        let callback = (try? String(contentsOf: callbackLog, encoding: .utf8)) ?? ""
        if completionExitCode == 0 || completionExitCode == 1 { #expect(callback.contains(token)) }
        if completionExitCode == 0 {
            #expect(callback.contains("result=success"))
            #expect(launcher.hasGeminiCLIExecConfirmed(token: token))
            diagnosticProgress = "beforeCallbackRequire"
            let callbackText = try #require(callback.split(whereSeparator: \.isWhitespace).last.map(String.init))
            let callbackURL = try #require(URL(string: callbackText))
            let receipt = try #require(GeminiLoginReceipt(url: callbackURL))
            diagnosticProgress = "afterReceiptParsed"
            #expect(receipt.token == token)
            #expect(receipt.result == .success)
            await model.completeGeminiInteractiveSignIn(token: receipt.token, result: receipt.result)
            diagnosticProgress = "afterCompletion"
            #expect(model.geminiOneTimeRecoveryState == .succeeded)
            #expect(model.snapshots.first(where: { $0.provider == .gemini })?.geminiQuotaMetrics?.first?.current == 80)
            #expect(model.snapshots.first(where: { $0.provider == .gemini })?.antigravitySharedQuotaMetrics?.map(\.current) == [80, 20])
        } else if completionExitCode != 3 {
            let deadline = ContinuousClock.now.advanced(by: .seconds(3))
            while model.geminiOneTimeRecoveryState == .awaitingLogin || model.geminiOneTimeRecoveryState == .stoppingLogin {
                guard ContinuousClock.now < deadline else { break }
                await Task.yield()
            }
            #expect(model.geminiOneTimeRecoveryState == .loginFailed)
        }
        #expect(model.isGeminiRefreshPaused)
        #expect(!FileManager.default.fileExists(atPath: scriptURL.path))
        let statuses = try FileManager.default.contentsOfDirectory(at: authenticationDirectory, includingPropertiesForKeys: nil)
            .filter { $0.pathExtension == "status" || $0.pathExtension == "exec" || $0.pathExtension == "lock" || $0.pathExtension == "cancel" || $0.pathExtension == "stopped" }
        #expect(statuses.isEmpty)
        diagnosticProgress = "afterFinalAssertions"
    }

    @Test("An unlaunched helper cannot acknowledge expiry; the launcher retains its coordination files")
    func productionLauncherPreservesUnacknowledgedExpiry() async throws {
        let root = try temporaryRoot()
        // The injected opener intentionally launches no process in this case.
        // Retention is asserted before this process-free fixture directory is removed.
        defer { try? FileManager.default.removeItem(at: root) }
        let fakeCLI = try writeExecutable("#!/bin/sh\nexit 0\n", named: "agy", in: root)
        let fakeOpen = try writeExecutable("#!/bin/sh\nexit 0\n", named: "open", in: root)
        var scriptURL: URL?
        var openedToken: String?
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root.appendingPathComponent("Authentication"),
            executableLocator: TemporaryFullFlowLocator(executable: fakeCLI),
            openURL: { scriptURL = $0; return true },
            completionOpenExecutableURL: fakeOpen,
            geminiStopAcknowledgementTimeout: .milliseconds(20)
        )
        let suiteName = "TemporaryUnacknowledgedFullFlow.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let model = AppModel(
            defaults: defaults,
            applicationSupportDirectoryURL: root.appendingPathComponent("support"),
            secretStore: TemporaryFullFlowSecretStore(),
            launchAtLoginStatusOperation: { false },
            authenticationLauncher: launcher,
            widgetSnapshotPublisher: nil, isDemoMode: false,
            geminiDiagnosticStore: GeminiDiagnosticStore(fileURL: nil),
            refreshOperation: { [] }, geminiPauseReasonOperation: { .unknown },
            geminiRecoveryBeginOperation: { _ in true },
            geminiRecoveryAuthorizeOperation: { _ in nil },
            serviceAccountRefreshOperation: { _ in [] },
            geminiAuthenticationOpenOperation: { token in
                openedToken = token
                _ = try launcher.open(provider: .gemini, completionToken: token)
            },
            geminiAuthenticationCancelOperation: { launcher.cancelPendingGeminiLogin(token: $0) },
            geminiAuthenticationStopOperation: { launcher.requestGeminiLoginStop(token: $0) },
            geminiAuthenticationFailureOperation: { launcher.hasGeminiTerminalFailure(token: $0) },
            geminiAuthenticationCleanupUncertainOperation: { launcher.hasGeminiCleanupUncertainty(token: $0) },
            geminiAuthenticationExecConfirmedOperation: { launcher.hasGeminiCLIExecConfirmed(token: $0) },
            geminiRecoveryTimeout: .milliseconds(5)
        )
        defer { model.stop() }
        await model.refresh()
        await model.beginGeminiOneTimeRecovery()
        let deadline = ContinuousClock.now.advanced(by: .seconds(3))
        while model.isGeminiOneTimeRecoveryInProgress, ContinuousClock.now < deadline { await Task.yield() }
        #expect(model.geminiOneTimeRecoveryState == .loginFailed)
        #expect(model.isGeminiRefreshPaused)
        let token = try #require(openedToken)
        let retainedScriptURL = try #require(scriptURL)
        #expect(!launcher.hasGeminiCLIExecConfirmed(token: token))
        #expect(FileManager.default.fileExists(atPath: retainedScriptURL.path))
        let directory = root.appendingPathComponent("Authentication")
        let status = try #require(try FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil)
            .first(where: { $0.pathExtension == "status" }))
        #expect(try String(contentsOf: status, encoding: .utf8) == "ready\n")
        #expect(try String(contentsOf: status.appendingPathExtension("cancel"), encoding: .utf8) == "cancelled\n")
        #expect(FileManager.default.fileExists(atPath: status.appendingPathExtension("lock").path))
        #expect(FileManager.default.fileExists(atPath: status.appendingPathExtension("exec").path))
    }

    @Test("The isolated observer cannot block MainActor on a stalled query or inherited stdout", arguments: [false, true])
    func boundedObserverKeepsRecoveryActorResponsive(inheritedPipe: Bool) async throws {
        let root = try temporaryRoot()
        // These intentional uncertain observations retain their evidence root.
        let started = root.appendingPathComponent("query-started")
        let executable = try writeExecutable(
            "#!/usr/bin/perl\nopen my $f, '>', '\(started.path)' or die $!; close $f;\n"
                + (inheritedPipe ? "my $p = fork(); if (!$p) { select undef, undef, undef, 0.6; exit 0; } exit 0;\n" : "select undef, undef, undef, 30;\n"),
            named: "observer", in: root
        )
        let start = ContinuousClock.now
        var queryReturned = false
        let query = Task { @MainActor in
            let result = await TemporaryFullFlowPTY.queryProcessState(executable: executable, arguments: [], in: root)
            queryReturned = true
            return result
        }
        await Task.yield()
        let actorProbe = Task { @MainActor in !queryReturned }
        #expect(await actorProbe.value)
        #expect(await query.value == nil)
        #expect(start.duration(to: ContinuousClock.now) < .seconds(2))
        #expect(FileManager.default.fileExists(atPath: started.path))
        #expect(FileManager.default.fileExists(atPath: root.appendingPathComponent(".preserve-for-inspection").path))
    }

    @Test("Native observer descriptor ownership separates a normal query from a concurrent stalled query")
    func observerWritersRemainPrivateDuringConcurrentSpawn() async throws {
        let root = try temporaryRoot()
        // The intentional unknown sibling retains this evidence root.
        let prepared = root.appendingPathComponent("first-prepared")
        let siblingStarted = root.appendingPathComponent("sibling-started")
        let first = try writeExecutable("#!/bin/sh\nprintf independent\nexit 0\n", named: "first-query", in: root)
        let sibling = try writeExecutable("#!/usr/bin/perl\nopen my $f,'>','\(siblingStarted.path)' or die $!; close $f; select undef,undef,undef,30;\n", named: "sibling-query", in: root)
        let a = Task.detached {
            FullFlowProcessQuery.run(executable: first, arguments: [], in: root, onPrepared: {
                try? Data().write(to: prepared)
                let deadline = ContinuousClock.now.advanced(by: .seconds(2))
                while !FileManager.default.fileExists(atPath: siblingStarted.path), ContinuousClock.now < deadline { Thread.sleep(forTimeInterval: 0.002) }
            })
        }
        let deadline = ContinuousClock.now.advanced(by: .seconds(2))
        while !FileManager.default.fileExists(atPath: prepared.path), ContinuousClock.now < deadline { await Task.yield() }
        try #require(FileManager.default.fileExists(atPath: prepared.path))
        let b = Task.detached { FullFlowProcessQuery.run(executable: sibling, arguments: [], in: root, timeout: .seconds(1)) }
        let result = await a.value
        #expect(result?.exitStatus == 0)
        #expect(result?.text == "independent")
        #expect(await b.value == nil)
    }

    // Bug oracle: AppModel's early non-uncertain failure branch calls real
    // cancelPendingGeminiLogin before cleanup proof, deleting observable mapping/files.
    // Consumer characterization only; producer-delay fixes use the composite oracle below.
    @Test("Early foreground failure retains real launcher coordination until cleanup proof")
    func earlyForegroundFailureRetainsRealLauncherCoordination() async throws {
        try await runC2RetentionRegression(production: false)
    }

    // Bug oracle: actual foreground failure precedes a failed bounded cleanup query.
    // Either deferring that failure publication or retaining coordination must protect it.
    @Test("Unknown production cleanup after foreground failure retains ownership evidence")
    func foregroundFailureDuringUnknownCleanupRetainsProductionOwnership() async throws {
        try await runC2RetentionRegression(production: true)
    }

    private func runC2RetentionRegression(production: Bool) async throws {
        let root = try temporaryRoot()
        let directory = root.appendingPathComponent("Authentication")
        let monitor = C2MonitorStep()
        let calls = C2RecoveryCalls()
        var pty: TemporaryFullFlowPTY?
        var productionWrapperURL: URL?
        var model: AppModel?
        var launcher: CLIAuthenticationLauncher?
        var token: String?
        var status: URL?
        var setupError: Error?
        var handled = 0
        var appStopCalls = 0
        let release = root.appendingPathComponent("release-query")
        let watchdogRelease = root.appendingPathComponent("release-watchdog")
        // Unknown production ownership is intentionally retained for isolated guest
        // shutdown, never erased or signalled using recorded PID/PGID evidence.
        defer {
            try? Data().write(to: release)
            try? Data().write(to: watchdogRelease)
            Task { await monitor.cancel() }
            _ = model?.stop()
            if production {
                try? Data("C2 cleanup intentionally unconfirmed; guest shutdown required\n".utf8)
                    .write(to: root.appendingPathComponent(".preserve-for-inspection"))
                if let pty { C2RetainedPTY.contexts.append(pty) }
            } else {
                if let token { launcher?.cancelPendingGeminiLogin(token: token) }
                removeUnlessPreserved(root)
            }
        }
        let fakeCLI = try writeExecutable("#!/bin/sh\nexit 99\n", named: "agy", in: root)
        let callbacks = root.appendingPathComponent("callbacks")
        let fakeOpen = try writeExecutable(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \(c2Quote(callbacks.path))\nexit 0\n",
            named: "callback", in: root
        )
        let query = try writeExecutable(C2FixtureSources.query, named: "query", in: root)
        let watchdog = try writeExecutable(C2FixtureSources.watchdog, named: "watchdog", in: root)
        try Data(C2FixtureSources.perlModule.utf8).write(to: root.appendingPathComponent("AIMeterC2Barrier.pm"))
        let entered = root.appendingPathComponent("cleanup-entered")
        let failed = root.appendingPathComponent("foreground-injected")
        let queryFailed = root.appendingPathComponent("query-failed")
        let realLauncher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: directory,
            executableLocator: TemporaryFullFlowLocator(executable: fakeCLI),
            openURL: { scriptURL in
                do {
                    let files = try FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil)
                    let responses = files.filter { $0.pathExtension == "status" }
                    let pipes = files.filter { $0.pathExtension == "token" }
                    guard responses.count == 1, pipes.count == 1 else {
                        throw CocoaError(.fileReadCorruptFile)
                    }
                    let response = responses[0]
                    status = response
                    if production {
                        // Existing public builder inputs only. The real launcher still
                        // owns the mapping, token writer, consumers and stop operation.
                        let script = try CLIAuthenticationScriptBuilder().build(
                            provider: .gemini, executableURL: fakeCLI,
                            completionTokenPipeURL: pipes[0], completionStatusFileURL: response,
                            completionExecStatusFileURL: response.appendingPathExtension("exec"),
                            cancellationRequestFileURL: response.appendingPathExtension("cancel"),
                            completionOpenExecutableURL: fakeOpen, watchdogExecutableURL: watchdog,
                            processStateExecutableURL: query
                        )
                        try Data(script.utf8).write(to: scriptURL)
                        let environment = [
                            "PERL5LIB": root.path, "PERL5OPT": "-MAIMeterC2Barrier",
                            "AI_METER_C2_STATUS": response.path, "AI_METER_C2_QUERY": query.path,
                            "AI_METER_C2_FAILED": failed.path, "AI_METER_C2_ENTERED": entered.path,
                            "AI_METER_C2_RELEASE": release.path, "AI_METER_C2_QUERY_FAILED": queryFailed.path,
                            "AI_METER_C2_WATCHDOG_RELEASE": watchdogRelease.path
                        ]
                        let wrapper = "#!/bin/zsh\nexec /usr/bin/env "
                            + environment.keys.sorted().map { c2Quote("\($0)=\(environment[$0]!)") }.joined(separator: " ")
                            + " /bin/zsh " + c2Quote(scriptURL.path) + "\n"
                        let wrapperURL = try writeExecutable(wrapper, named: "private-pty", in: root)
                        productionWrapperURL = wrapperURL
                    }
                    return true
                } catch { setupError = error; return false }
            },
            completionOpenExecutableURL: fakeOpen,
            geminiStopAcknowledgementTimeout: .seconds(2)
        )
        launcher = realLauncher
        let suite = "C2Retention.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let realModel = AppModel(
            defaults: defaults, applicationSupportDirectoryURL: root.appendingPathComponent("support"),
            secretStore: TemporaryFullFlowSecretStore(), launchAtLoginStatusOperation: { false },
            authenticationLauncher: realLauncher, widgetSnapshotPublisher: nil, isDemoMode: false,
            geminiDiagnosticStore: GeminiDiagnosticStore(fileURL: nil),
            refreshOperation: { await calls.refresh(); return [] },
            geminiPauseReasonOperation: { .unknown }, geminiRecoveryBeginOperation: { _ in true },
            geminiRecoveryAuthorizeOperation: { _ in await calls.authorize(); return nil },
            geminiQuotaCheckOperation: { _ in await calls.quota(); throw CocoaError(.userCancelled) },
            serviceAccountRefreshOperation: { _ in [] },
            geminiAuthenticationOpenOperation: {
                token = $0; _ = try realLauncher.open(provider: .gemini, completionToken: $0)
            },
            geminiAuthenticationCancelOperation: { realLauncher.cancelPendingGeminiLogin(token: $0); handled += 1 },
            geminiAuthenticationStopOperation: {
                let operation = realLauncher.requestGeminiLoginStop(token: $0)
                appStopCalls += 1; handled += 1; return operation
            },
            geminiAuthenticationFailureOperation: { realLauncher.hasGeminiTerminalFailure(token: $0) },
            geminiAuthenticationCleanupUncertainOperation: { realLauncher.hasGeminiCleanupUncertainty(token: $0) },
            geminiAuthenticationExecConfirmedOperation: { realLauncher.hasGeminiCLIExecConfirmed(token: $0) },
            geminiRecoveryTimeout: .seconds(30), signInSleep: { _ in try await monitor.sleep() }
        )
        model = realModel
        await realModel.refresh()
        await realModel.beginGeminiOneTimeRecovery()
        if let setupError { throw setupError }
        let attempt = try #require(token)
        let response = try #require(status)
        try #require(await monitor.waitForEntry(1))
        if production {
            let wrapper = try #require(productionWrapperURL)
            pty = try TemporaryFullFlowPTY(scriptURL: wrapper, in: root)
            let fixture = try #require(pty)
            try #require(await fixture.c2WaitForFile(entered))
            try #require(FileManager.default.fileExists(atPath: failed.path))
        } else {
            try Data("inert evidence; not a signalling identity\n".utf8)
                .write(to: response.appendingPathExtension("guardian"))
            try Data("terminal_foreground_failed\n".utf8).write(to: response)
        }
        await monitor.release()
        // A branch acknowledgement follows real cancellation/stop invocation, or
        // the next real monitor sleep follows a safe producer-deferred failure.
        let deadline = ContinuousClock.now.advanced(by: .seconds(3))
        while handled == 0, !(await monitor.hasEntered(2)), ContinuousClock.now < deadline {
            await Task.yield()
        }
        let reachedNextSleep = await monitor.hasEntered(2)
        try #require(handled > 0 || reachedNextSleep)
        let deferredProducer = reachedNextSleep && handled == 0
        let stopsBeforeLateStatus = appStopCalls
        for url in [response, response.appendingPathExtension("exec"),
                    response.appendingPathExtension("cancel"), response.appendingPathExtension("lock"),
                    response.appendingPathExtension("guardian")] {
            #expect(FileManager.default.fileExists(atPath: url.path), "Coordination lost before cleanup proof: \(url.lastPathComponent)")
        }
        if production {
            try Data().write(to: release)
            let fixture = try #require(pty)
            try #require(await fixture.c2WaitForFile(queryFailed))
            try #require(await fixture.c2WaitForFile(response, text: "helper_stop_unconfirmed\n"))
        } else {
            // This is the producer input for consumer characterization, not a
            // fabricated acknowledgement or a claim of production helper behavior.
            try Data("helper_stop_unconfirmed\n".utf8).write(to: response)
        }
        // I1: deliver actual late uncertainty to the suspended failure monitor.
        // Only the producer-deferred path owns sleep 2; the retention path may
        // already be awaiting the same real stop and must not be resumed again.
        if deferredProducer {
            try #require(await monitor.releaseIfSleeping(at: 2))
        }
        let requiredAppStops = deferredProducer ? stopsBeforeLateStatus + 1 : max(stopsBeforeLateStatus, 1)
        let settlementDeadline = ContinuousClock.now.advanced(by: .seconds(3))
        while (appStopCalls < requiredAppStops || realModel.geminiOneTimeRecoveryState != .loginFailed),
              ContinuousClock.now < settlementDeadline {
            await Task.yield()
        }
        // Only the App's delegated stop closure increments appStopCalls. The
        // direct launcher retry below cannot satisfy this composition oracle.
        try #require(appStopCalls >= requiredAppStops)
        try #require(realModel.geminiOneTimeRecoveryState == .loginFailed)
        #expect(realLauncher.hasGeminiCleanupUncertainty(token: attempt))
        #expect(!FileManager.default.fileExists(atPath: response.appendingPathExtension("stopped").path))
        #expect(await !realLauncher.requestGeminiLoginStop(token: attempt).value)
        #expect(try String(contentsOf: response, encoding: .utf8) == "helper_stop_unconfirmed\n")
        for suffix in ["exec", "cancel", "lock", "guardian"] {
            #expect(FileManager.default.fileExists(atPath: response.appendingPathExtension(suffix).path))
        }
        await realModel.completeGeminiInteractiveSignIn(token: attempt, result: .success)
        #expect(realModel.isGeminiRefreshPaused)
        #expect(realModel.geminiPauseReason == .unknown)
        #expect(await calls.noRecoveryOrExtraRefresh())
        let callbackText = (try? String(contentsOf: callbacks, encoding: .utf8)) ?? ""
        #expect(!callbackText.contains("result=success"))
    }

    private func c2Quote(_ value: String) -> String {
        "'\(value.replacingOccurrences(of: "'", with: "'\\''"))'"
    }

    private func temporaryRoot() throws -> URL {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("ai-meter-full-auth-flow-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        return root
    }

    private func removeUnlessPreserved(_ root: URL) {
        guard !FileManager.default.fileExists(atPath: root.appendingPathComponent(".preserve-for-inspection").path) else { return }
        try? FileManager.default.removeItem(at: root)
    }

    private func writeExecutable(_ text: String, named name: String, in root: URL) throws -> URL {
        let url = root.appendingPathComponent(name)
        try Data(text.utf8).write(to: url)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: url.path)
        return url
    }
}

// Scoped adaptation of the candidate's forkpty fixture. It never opens the
// user's Terminal or URL handler, nor signals a recorded descendant PID/PGID.
@MainActor
private final class TemporaryFullFlowPTY {
    struct Result {
        let finished: Bool
        let cleanupSucceeded: Bool
        let terminationStatus: Int32?
        let output: String
    }
    private let root: URL
    private let childPID: pid_t
    private var masterDescriptor: Int32 = -1
    private var childReaped = false
    private var ownershipUnknown = false
    private(set) var mustPreserveRoot = false
    private var childStatus: Int32 = 0
    private var output = Data()

    init(scriptURL: URL, in root: URL) throws {
        self.root = root
        let arguments = ["/usr/bin/env", "-i", "HOME=\(root.path)", "ZDOTDIR=\(root.path)", "TMPDIR=\(root.path)",
                         "PATH=\(root.path):/usr/bin:/bin:/usr/sbin:/sbin", "LANG=C", "LC_ALL=C", "/bin/zsh", scriptURL.path]
        let pointers = arguments.map { strdup($0)! } + [nil]
        defer { pointers.forEach { free($0) } }
        let executable = strdup("/usr/bin/env")!
        defer { free(executable) }
        var descriptor: Int32 = -1
        let pid = pointers.withUnsafeBufferPointer { argv in
            let pid = forkpty(&descriptor, nil, nil, nil)
            if pid == 0 {
                execv(executable, argv.baseAddress!)
                Darwin._exit(127)
            }
            return pid
        }
        guard pid > 0, descriptor >= 0 else { throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno)) }
        childPID = pid
        masterDescriptor = descriptor
        let flags = fcntl(descriptor, F_GETFL)
        guard flags >= 0, fcntl(descriptor, F_SETFL, flags | O_NONBLOCK) == 0 else {
            let error = errno
            cleanupOwnedWrapper()
            throw NSError(domain: NSPOSIXErrorDomain, code: Int(error))
        }
    }

    func finish(cliPIDFile: URL, watchdogPIDFile: URL, groupMarker: URL,
                cliRequired: Bool, watchdogRequired: Bool, synthesizeStaleReadiness: Bool,
                runningReady: (@MainActor () async -> Void)? = nil) async -> Result {
        let deadline = ContinuousClock.now.advanced(by: .seconds(15))
        var groupID: pid_t?
        var inputSent = false
        var staleReadinessSent = false
        var runningReadyReleased = false
        let readinessDeadline = ContinuousClock.now.advanced(by: .seconds(10))
        while ContinuousClock.now < deadline, !childReaped, !ownershipUnknown, !mustPreserveRoot {
            if groupID == nil,
               let text = try? String(contentsOf: groupMarker, encoding: .utf8),
               let observed = pid_t(text.trimmingCharacters(in: .whitespacesAndNewlines)), observed > 1 { groupID = observed }
            drainOutput()
            if !inputSent, String(decoding: output, as: UTF8.self).contains("fake-cli-ready") {
                inputSent = writeInput("approved\n")
                if !inputSent { break }
            }
            if let runningReady, !runningReadyReleased {
                let execProof = (try? String(contentsOf: groupMarker.deletingPathExtension().appendingPathExtension("exec"), encoding: .utf8))?
                    .trimmingCharacters(in: .whitespacesAndNewlines)
                if execProof == "exec_confirmed", inputSent,
                   String(decoding: output, as: UTF8.self).contains("fake-cli-approved") {
                    await runningReady()
                    runningReadyReleased = true
                } else if ContinuousClock.now >= readinessDeadline {
                    preserve("Bounded exec and private terminal input handshake timed out")
                    break
                }
            }
            if synthesizeStaleReadiness, !staleReadinessSent,
               FileManager.default.fileExists(atPath: watchdogPIDFile.path), await trackedExited(watchdogPIDFile, required: true) {
                // A ready marker written after this tracked fake watchdog exits
                // must never let the generated supervisor exec the fake CLI.
                do {
                    try Data().write(to: groupMarker.deletingPathExtension().appendingPathExtension("ready"))
                    staleReadinessSent = true
                } catch { preserve("Could not write synthetic stale readiness"); break }
            }
            observeWrapper()
            if !childReaped, !ownershipUnknown { try? await Task.sleep(for: .milliseconds(10)) }
        }
        let finished = childReaped
        if !finished { cleanupOwnedWrapper() }
        let cleanupDeadline = ContinuousClock.now.advanced(by: .seconds(3))
        var cleanupSucceeded = false
        while ContinuousClock.now < cleanupDeadline {
            let groupGone = groupID.map { Darwin.kill(-$0, 0) != 0 && errno == ESRCH } ?? !cliRequired
            let cliGone = await trackedExited(cliPIDFile, required: cliRequired)
            let watchdogGone = await trackedExited(watchdogPIDFile, required: watchdogRequired)
            cleanupSucceeded = childReaped && !ownershipUnknown && !mustPreserveRoot && groupGone && cliGone && watchdogGone
            if cleanupSucceeded { break }
            try? await Task.sleep(for: .milliseconds(20))
        }
        if !cleanupSucceeded { preserve("Synthetic CLI, watchdog or group is live or ownership is unknown") }
        return Result(finished: finished, cleanupSucceeded: cleanupSucceeded,
                      terminationStatus: childReaped && childStatus & 0x7f == 0 ? (childStatus >> 8) & 0xff : nil,
                      output: String(decoding: output, as: UTF8.self))
    }

    func cleanupOwnedWrapper() {
        guard masterDescriptor >= 0 else { return }
        defer { Darwin.close(masterDescriptor); masterDescriptor = -1 }
        observeWrapper()
        if !childReaped || ownershipUnknown {
            // Emergency wrapper cleanup alone cannot prove descendant cleanup.
            preserve("Emergency PTY teardown requires synthetic descendant inspection")
        }
        // Only this directly spawned, still-unreaped child can be signalled.
        for signal in [SIGHUP, SIGTERM, SIGKILL] where !childReaped && !ownershipUnknown {
            _ = Darwin.kill(childPID, signal)
            let deadline = ContinuousClock.now.advanced(by: .seconds(1))
            while !childReaped, !ownershipUnknown, ContinuousClock.now < deadline {
                observeWrapper()
                if !childReaped { Thread.sleep(forTimeInterval: 0.01) }
            }
        }
        if !childReaped || ownershipUnknown { preserve("PTY wrapper cleanup ownership is unknown") }
    }

    private func observeWrapper() {
        guard !childReaped, !ownershipUnknown else { return }
        let result = waitpid(childPID, &childStatus, WNOHANG)
        if result == childPID { childReaped = true }
        else if result < 0, errno != EINTR { ownershipUnknown = true }
    }

    private func drainOutput() {
        guard masterDescriptor >= 0 else { return }
        var bytes = [UInt8](repeating: 0, count: 4096)
        let count = Darwin.read(masterDescriptor, &bytes, bytes.count)
        if count > 0 {
            guard output.count + count <= 65_536 else { preserve("Synthetic terminal output exceeded limit"); return }
            output.append(contentsOf: bytes.prefix(count))
        } else if count < 0, errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR && errno != EIO {
            preserve("Synthetic terminal observation failed")
        }
    }

    private func writeInput(_ text: String) -> Bool {
        let bytes = Array(text.utf8)
        let deadline = ContinuousClock.now.advanced(by: .seconds(1))
        var written = 0
        while written < bytes.count, ContinuousClock.now < deadline {
            let count = bytes.withUnsafeBytes { buffer in
                Darwin.write(masterDescriptor, buffer.baseAddress!.advanced(by: written), bytes.count - written)
            }
            if count > 0 { written += count }
            else if count < 0, errno == EAGAIN || errno == EWOULDBLOCK || errno == EINTR { Thread.sleep(forTimeInterval: 0.01) }
            else { return false }
        }
        return written == bytes.count
    }

    private func trackedExited(_ pidFile: URL, required: Bool) async -> Bool {
        guard FileManager.default.fileExists(atPath: pidFile.path) else { return !required }
        guard let text = try? String(contentsOf: pidFile, encoding: .utf8),
              let pid = pid_t(text.trimmingCharacters(in: .whitespacesAndNewlines)), pid > 0 else { return false }
        guard Darwin.kill(pid, 0) == 0 else { return errno == ESRCH }
        let query = await Self.queryProcessState(
            executable: URL(fileURLWithPath: "/bin/ps"), arguments: ["-o", "stat=", "-p", String(pid)], in: root
        )
        guard let query, query.exitStatus == 0 else { preserve("Tracked child observation unknown"); return false }
        let state = query.text.trimmingCharacters(in: .whitespacesAndNewlines)
        return state.isEmpty || state.contains("Z")
    }

    static func queryProcessState(executable: URL, arguments: [String], in root: URL) async -> FullFlowProcessQuery.Output? {
        await Task.detached(priority: .utility) {
            FullFlowProcessQuery.run(executable: executable, arguments: arguments, in: root)
        }.value
    }

    private func preserve(_ reason: String) {
        mustPreserveRoot = true
        try? Data("\(reason)\n".utf8).write(to: root.appendingPathComponent(".preserve-for-inspection"), options: .atomic)
    }
}

private struct TemporaryFullFlowLocator: ExecutableLocating {
    let executable: URL
    func locate(named name: String) -> URL? { name == "agy" ? executable : nil }
}

private struct TemporaryFullFlowSecretStore: SecretStore {
    func read() throws -> String? { nil }
    func save(_ secret: String) throws {}
    func delete() throws {}
}

// Private test observer. O_CLOEXEC is applied atomically by open; there
// is no parent writer descriptor or raw pipe-to-fcntl inheritance window.
// Native spawn explicitly supplies only stdio and closes all other descriptors.
private enum FullFlowProcessQuery {
    struct Output: Sendable, Equatable { let exitStatus: Int32; let text: String }

    static func run(executable: URL, arguments: [String], in root: URL,
                    timeout: Duration = .milliseconds(250), onPrepared: (@Sendable () -> Void)? = nil) -> Output? {
        var rootValidated = false
        func unknown() -> Output? {

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

private actor FullFlowExpiryGate {
    private var continuation: CheckedContinuation<Void, Error>?
    private var started = false
    private var released = false
    private var cancelled = false

    func sleep(duration: Duration) async throws {
        guard !cancelled else { throw CancellationError() }
        started = true
        if !released {
            try await withTaskCancellationHandler {
                try await withCheckedThrowingContinuation { continuation = $0 }
            } onCancel: {
                Task { await self.cancel() }
            }
        }
        guard !cancelled else { throw CancellationError() }
        try await Task.sleep(for: duration)
    }

    func waitUntilSleeping(timeout: Duration) async -> Bool {
        let deadline = ContinuousClock.now.advanced(by: timeout)
        while !started, !cancelled, !Task.isCancelled, ContinuousClock.now < deadline { await Task.yield() }
        return started
    }

    func release() {
        released = true
        continuation?.resume()
        continuation = nil
    }

    func cancel() {
        cancelled = true
        continuation?.resume(throwing: CancellationError())
        continuation = nil
    }
}

// C2-only monitor barrier. Cancellation resumes the same suspended monitor;
// entry/branch acknowledgements, not elapsed sleeps, control the test ordering.
private actor C2MonitorStep {
    private var continuation: CheckedContinuation<Void, Error>?
    private var entries = 0
    private var cancelled = false

    func sleep() async throws {
        try Task.checkCancellation()
        guard !cancelled else { throw CancellationError() }
        try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation { (next: CheckedContinuation<Void, Error>) in
                if cancelled || Task.isCancelled { next.resume(throwing: CancellationError()) }
                else { continuation = next; entries += 1 }
            }
        } onCancel: { Task { await self.cancel() } }
    }

    func hasEntered(_ count: Int) -> Bool { entries >= count }

    func waitForEntry(_ count: Int) async -> Bool {
        let deadline = ContinuousClock.now.advanced(by: .seconds(3))
        while entries < count, !cancelled, !Task.isCancelled, ContinuousClock.now < deadline {
            await Task.yield()
        }
        return entries >= count
    }

    func release() { continuation?.resume(); continuation = nil }
    func releaseIfSleeping(at entry: Int) -> Bool {
        guard !cancelled, entries == entry, let pending = continuation else { return false }
        continuation = nil
        pending.resume()
        return true
    }
    func cancel() {
        cancelled = true
        continuation?.resume(throwing: CancellationError())
        continuation = nil
    }
}

private actor C2RecoveryCalls {
    private var authorizations = 0
    private var quotaChecks = 0
    private var refreshes = 0
    func authorize() { authorizations += 1 }
    func quota() { quotaChecks += 1 }
    func refresh() { refreshes += 1 }
    func noRecoveryOrExtraRefresh() -> Bool {
        authorizations == 0 && quotaChecks == 0 && refreshes == 1
    }
}

@MainActor
private enum C2RetainedPTY {
    // Retain direct-child descriptors/context on intentional uncertain cleanup.
    // Guest shutdown remains required; this is not a descendant-cleanup claim.
    static var contexts: [TemporaryFullFlowPTY] = []
}

private extension TemporaryFullFlowPTY {
    func c2WaitForFile(_ file: URL, text: String? = nil) async -> Bool {
        let deadline = ContinuousClock.now.advanced(by: .seconds(3))
        while ContinuousClock.now < deadline, !ownershipUnknown, !mustPreserveRoot {
            drainOutput()
            observeWrapper()
            if let data = try? String(contentsOf: file, encoding: .utf8), text == nil || data == text { return true }
            await Task.yield()
        }
        preserve("C2 fixture marker/status not observed within bound")
        return false
    }
}

private enum C2FixtureSources {
    static let perlModule = #"""
    package AIMeterC2Barrier;
    use strict;
    use warnings;
    BEGIN {
      require POSIX;
      require Fcntl;
      require Time::HiRes;
      my $real_foreground = \&POSIX::tcsetpgrp;
      no warnings 'redefine';
      *POSIX::tcsetpgrp = sub {
        my ($descriptor, $group) = @_;
        my $identity = '';
        if (open my $f, '<', $ENV{AI_METER_C2_STATUS} . '.guardian') {
          $identity = <$f> // ''; close $f;
        }
        if ($identity =~ /^([1-9][0-9]*) [0-9a-f]{32}\n$/ && $group == $1) {
          open my $f, '>', $ENV{AI_METER_C2_FAILED} or die "foreground marker: $!";
          print {$f} "injected\n"; close $f;
          return -1;
        }
        return $real_foreground->(@_);
      };
      *CORE::GLOBAL::fork = sub {
        # The production observer starts its clock after fork returns. This test
        # gate therefore controls ordering without spending its 250ms budget.
        my $identity = '';
        if (open my $f, '<', $ENV{AI_METER_C2_STATUS} . '.guardian') {
          $identity = <$f> // ''; close $f;
        }
        my ($guardian) = $identity =~ /^([1-9][0-9]*) [0-9a-f]{32}\n$/;
        if (defined($guardian) && @ARGV == 6
            && $ARGV[0] eq $ENV{AI_METER_C2_QUERY}
            && $ARGV[1] eq '--owned-query-completion'
            && $ARGV[2] eq '-o' && $ARGV[3] eq 'stat='
            && $ARGV[4] eq '-p' && $ARGV[5] eq $guardian
            && -e $ENV{AI_METER_C2_FAILED}
            && sysopen(my $claim, $ENV{AI_METER_C2_ENTERED},
                       Fcntl::O_WRONLY() | Fcntl::O_CREAT() | Fcntl::O_EXCL(), 0600)) {
          print {$claim} "$$\n"; close $claim;
          my $deadline = Time::HiRes::clock_gettime(Time::HiRes::CLOCK_MONOTONIC()) + 10;
          while (!-e $ENV{AI_METER_C2_RELEASE}) {
            POSIX::_exit(125) if Time::HiRes::clock_gettime(Time::HiRes::CLOCK_MONOTONIC()) >= $deadline;
            select undef, undef, undef, 0.005;
          }
        }
        return CORE::fork();
      };
    }
    1;
    """#
    static let query = #"""
    #!/bin/sh
    if [ -e "$AI_METER_C2_FAILED" ]; then
      printf 'failed\n' > "$AI_METER_C2_QUERY_FAILED"
      exit 42
    fi
    exec /bin/ps "$@"
    """#
    static let watchdog = #"""
    #!/usr/bin/perl
    use strict;
    use warnings;
    use Time::HiRes qw(clock_gettime CLOCK_MONOTONIC);
    @ARGV == 9 && $ARGV[0] eq '-e' && $ARGV[2] =~ /^[1-9][0-9]*$/ or die 'watchdog fixture argv';
    open my $f, '>', $ARGV[6] or die "ready: $!"; close $f;
    my $parent = getppid();
    my $deadline = clock_gettime(CLOCK_MONOTONIC) + 10;
    while (!-e $ENV{AI_METER_C2_WATCHDOG_RELEASE}) {
      exit 125 if getppid() != $parent || clock_gettime(CLOCK_MONOTONIC) >= $deadline;
      select undef, undef, undef, 0.005;
    }
    exit 0;
    """#
}


// G11 v19 diagnostic collection only. This helper never controls test acceptance
// or process cleanup, and never opens the launch lock or the token FIFO.
private enum G11DiagnosticV19 {
    private static let prefix = "G11_DIAG_V19 "

    private struct Sample {
        var exists: Bool?
        var observation: String
        var errorNumber: Int32?
        var kind: String?
        var data: Data?
        var capturedBytes = 0
        var truncated: Bool?

        var facts: [String: Any] {
            ["exists": exists.map { $0 as Any } ?? NSNull(),
             "observation": observation,
             "readError": observation != "sampled" && observation != "metadataOnly" && observation != "missing",
             "errno": errorNumber.map { Int($0) as Any } ?? NSNull(),
             "kind": kind.map { $0 as Any } ?? NSNull(),
             "capturedBytes": capturedBytes,
             "truncated": truncated.map { $0 as Any } ?? NSNull()]
        }
    }

    private static func kind(_ mode: mode_t) -> String {
        switch mode & mode_t(S_IFMT) {
        case mode_t(S_IFREG): return "regular"
        case mode_t(S_IFDIR): return "directory"
        case mode_t(S_IFLNK): return "symlinkRejected"
        case mode_t(S_IFIFO): return "fifoRejected"
        default: return "otherRejected"
        }
    }

    private static func sameMetadata(_ a: stat, _ b: stat) -> Bool {
        a.st_dev == b.st_dev && a.st_ino == b.st_ino && a.st_mode == b.st_mode
            && a.st_uid == b.st_uid && a.st_nlink == b.st_nlink && a.st_size == b.st_size
            && a.st_mtimespec.tv_sec == b.st_mtimespec.tv_sec
            && a.st_mtimespec.tv_nsec == b.st_mtimespec.tv_nsec
            && a.st_ctimespec.tv_sec == b.st_ctimespec.tv_sec
            && a.st_ctimespec.tv_nsec == b.st_ctimespec.tv_nsec
    }

    private static func sample(_ name: String, in directory: Int32, limit: Int?) -> Sample {
        guard directory >= 0, !name.isEmpty, !name.contains("/"), name != ".", name != ".." else {
            return Sample(exists: nil, observation: "scopeUnavailable")
        }
        var before = stat()
        guard name.withCString({ fstatat(directory, $0, &before, AT_SYMLINK_NOFOLLOW) }) == 0 else {
            let code = errno
            return Sample(exists: code == ENOENT ? false : nil,
                          observation: code == ENOENT ? "missing" : "metadataError", errorNumber: code)
        }
        let fileKind = kind(before.st_mode)
        guard let limit else {
            return Sample(exists: true, observation: "metadataOnly", kind: fileKind)
        }
        guard (0...8_192).contains(limit), fileKind == "regular", before.st_nlink == 1, before.st_size >= 0 else {
            return Sample(exists: true, observation: "nonRegularOrLinkedRejected", kind: fileKind)
        }
        let descriptor = name.withCString {
            openat(directory, $0, O_RDONLY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC)
        }
        guard descriptor >= 0 else {
            return Sample(exists: true, observation: "openError", errorNumber: errno, kind: fileKind)
        }
        defer { Darwin.close(descriptor) }
        var opened = stat()
        guard fstat(descriptor, &opened) == 0 else {
            return Sample(exists: true, observation: "openedMetadataError", errorNumber: errno, kind: fileKind)
        }
        guard sameMetadata(before, opened), kind(opened.st_mode) == "regular" else {
            return Sample(exists: true, observation: "changedBeforeRead", kind: fileKind)
        }
        // Exactly one nonblocking bounded read; no retry on EINTR or short read.
        var bytes = [UInt8](repeating: 0, count: limit + 1)
        let count = bytes.withUnsafeMutableBytes { Darwin.read(descriptor, $0.baseAddress!, $0.count) }
        guard count >= 0 else {
            return Sample(exists: true, observation: "readError", errorNumber: errno, kind: fileKind)
        }
        var after = stat()
        var current = stat()
        guard fstat(descriptor, &after) == 0,
              name.withCString({ fstatat(directory, $0, &current, AT_SYMLINK_NOFOLLOW) }) == 0 else {
            return Sample(exists: true, observation: "afterReadMetadataError", errorNumber: errno, kind: fileKind)
        }
        guard sameMetadata(before, after), sameMetadata(before, current) else {
            return Sample(exists: true, observation: "changedDuringRead", kind: fileKind)
        }
        let expectedRead = min(Int64(before.st_size), Int64(limit + 1))
        guard Int64(count) == expectedRead else {
            return Sample(exists: true, observation: "shortRead", kind: fileKind,
                          capturedBytes: count, truncated: before.st_size > Int64(limit))
        }
        return Sample(exists: true, observation: "sampled", kind: fileKind,
                      data: Data(bytes.prefix(count)), capturedBytes: count,
                      truncated: count > limit || before.st_size > Int64(limit))
    }

    private static func redact(_ value: String, token: String, nonces: [String] = []) -> String {
        var text = value
        var variants = [token]
        for allowed in [CharacterSet.alphanumerics, .urlQueryAllowed, .urlPathAllowed] {
            if let encoded = token.addingPercentEncoding(withAllowedCharacters: allowed) { variants.append(encoded) }
        }
        let firstEncodingVariants = variants
        for encoded in firstEncodingVariants {
            if let encodedAgain = encoded.addingPercentEncoding(withAllowedCharacters: .alphanumerics) { variants.append(encodedAgain) }
        }
        for variant in Set(variants).sorted(by: {
            $0.utf8.count == $1.utf8.count ? $0 < $1 : $0.utf8.count > $1.utf8.count
        }) where !variant.isEmpty {
            text = text.replacingOccurrences(of: variant, with: "[token]", options: .caseInsensitive)
            // A bounded captured file/PTY may end midway through the known token.
            // Suppress even that suffix before performing any output truncation.
            text = redactCapturedSuffix(text, value: variant, marker: "[token-fragment]")
        }
        // Nonces are bounded validated local samples, never emitted or hashed.
        for nonce in Set(nonces) where nonce.utf8.count == 32 && nonce.utf8.allSatisfy({
            ($0 >= 48 && $0 <= 57) || ($0 >= 97 && $0 <= 102)
        }) {
            text = text.replacingOccurrences(of: nonce, with: "[nonce]", options: .caseInsensitive)
            text = redactCapturedSuffix(text, value: nonce, marker: "[nonce-fragment]")
        }
        for (pattern, replacement) in [
            (#"aitokenmeter(?:://|%3a%2f%2f|%253a%252f%252f)[^\s\x00-\x20\"'<>]*"#, "[callback-url]"),
            (#"https?(?:://|%3a%2f%2f|%253a%252f%252f)[^\s\x00-\x20\"'<>]*"#, "[url]"),
            (#"[0-9a-f]{32}"#, "[nonce]")
        ] {
            if let expression = try? NSRegularExpression(pattern: pattern, options: .caseInsensitive) {
                text = expression.stringByReplacingMatches(in: text, range: NSRange(text.startIndex..., in: text), withTemplate: replacement)
            }
        }
        return text
    }

    private static func redactCapturedSuffix(_ text: String, value: String, marker: String) -> String {
        // CRLF is one Swift Character. Split using scalar indices, never a
        // Character count derived from individual CR/LF code points.
        let scalars = text.unicodeScalars
        let newlineCount = scalars.reversed().prefix(while: { $0.value == 10 || $0.value == 13 }).count
        let split = scalars.index(scalars.endIndex, offsetBy: -newlineCount)
        var body = String(scalars[..<split])
        let trailingNewlines = String(scalars[split...])
        let characters = Array(value)
        if characters.count > 1 {
            for count in stride(from: characters.count - 1, through: 1, by: -1) {
                let fragment = String(characters.prefix(count))
                if body.lowercased().hasSuffix(fragment.lowercased()) {
                    body.removeLast(count)
                    body += marker
                    break
                }
            }
        }
        return body + trailingNewlines
    }

    private static func clipUTF8(_ text: String, budget: Int) -> String {
        var result = ""
        var bytes = 0
        for character in text {
            let next = String(character)
            guard bytes + next.utf8.count <= budget else { break }
            result += next
            bytes += next.utf8.count
        }
        return result
    }

    private static func textFact(_ text: String, token: String, budget: Int) -> [String: Any] {
        let redacted = redact(text, token: token)
        let clipped = clipUTF8(redacted, budget: budget)
        return ["value": clipped, "capturedUTF8Bytes": text.utf8.count,
                "redactedUTF8Bytes": redacted.utf8.count, "valueTruncated": clipped != redacted]
    }

    private static func textSample(_ value: Sample, token: String, limit: Int) -> [String: Any] {
        var facts = value.facts
        // Output only known control values; unexpected file text is metadata-only.
        let allowed: Set<String> = [
            "ready", "pending", "exec_confirmed", "exec_failed", "launch_indeterminate", "cancelled",
            "helper_cancelled", "helper_stop_unconfirmed", "process_state_unavailable",
            "terminal_foreground_failed", "terminal_foreground_restore_failed",
            "guardian_identity_mismatch", "guardian_identity_unavailable", "watchdog_launch_failed",
            "token_missing", "token_open_failed", "token_timeout", "invalid_token",
            "cli_exec_failed", "helper_started", "login_timeout", "success", "failure"
        ]
        if let data = value.data, let text = String(data: data, encoding: .utf8) {
            let normalized = text.trimmingCharacters(in: .whitespacesAndNewlines)
            facts["utf8Valid"] = true
            if value.truncated == false, allowed.contains(normalized) {
                facts["text"] = textFact(normalized, token: token, budget: limit)
                facts["knownControlValue"] = true
                facts["whitespaceNormalized"] = true
                facts["unexpectedContentOmitted"] = false
            } else {
                facts["text"] = NSNull()
                facts["knownControlValue"] = false
                facts["unexpectedContentOmitted"] = true
            }
        } else {
            facts["text"] = NSNull()
            facts["utf8Valid"] = value.data == nil ? NSNull() : false
            facts["knownControlValue"] = NSNull()
            facts["unexpectedContentOmitted"] = value.data != nil
        }
        return facts
    }

    private static func positiveInteger(_ text: String) -> Int32? {
        let digits = text.hasSuffix("\n") ? String(text.dropLast()) : text
        guard !digits.isEmpty, digits.utf8.allSatisfy({ $0 >= 48 && $0 <= 57 }),
              digits.first != "0", let number = Int32(digits), number > 0 else { return nil }
        return number
    }

    private static func groupFact(_ value: Sample) -> [String: Any] {
        var facts = value.facts
        let text = value.truncated == false ? value.data.flatMap { String(data: $0, encoding: .utf8) } : nil
        let parsed = text.flatMap(positiveInteger)
        facts["empty"] = value.data.map { $0.isEmpty as Any } ?? NSNull()
        facts["utf8Valid"] = value.data == nil || value.truncated == true ? NSNull() : (text != nil) as Any
        facts["validPositiveInteger"] = text.map { _ in (parsed != nil) as Any } ?? NSNull()
        facts["parsedValue"] = parsed.map { Int($0) as Any } ?? NSNull()
        if value.data == nil { facts["integerObservation"] = "unavailable" }
        else if value.truncated == true { facts["integerObservation"] = "truncated" }
        else if text == nil { facts["integerObservation"] = "invalidUTF8" }
        else if text?.isEmpty == true { facts["integerObservation"] = "empty" }
        else { facts["integerObservation"] = parsed == nil ? "invalid" : "validPositiveInteger" }
        return facts
    }

    private static func guardianPID(_ value: Sample) -> Int32? {
        guard value.truncated == false, let data = value.data, data.last == 10,
              let body = String(data: Data(data.dropLast()), encoding: .utf8) else { return nil }
        let parts = body.split(separator: " ", omittingEmptySubsequences: false)
        guard parts.count == 2, parts[1].utf8.count == 32,
              parts[1].utf8.allSatisfy({ ($0 >= 48 && $0 <= 57) || ($0 >= 97 && $0 <= 102) }) else { return nil }
        return positiveInteger(String(parts[0]))
    }

    private static func guardianNonce(_ value: Sample) -> String? {
        guard guardianPID(value) != nil, let data = value.data,
              let body = String(data: Data(data.dropLast()), encoding: .utf8) else { return nil }
        return body.split(separator: " ", omittingEmptySubsequences: false).last.map(String.init)
    }

    private static func readyFact(_ value: Sample) -> [String: Any] {
        var facts = value.facts
        facts["empty"] = value.data.map { $0.isEmpty as Any } ?? NSNull()
        return facts
    }

    private static func guardianFact(_ value: Sample) -> [String: Any] {
        var facts = value.facts
        let pid = guardianPID(value)
        facts["validIdentityFormat"] = value.data != nil && value.truncated == false ? (pid != nil) as Any : NSNull()
        facts["parsedPID"] = pid.map { Int($0) as Any } ?? NSNull()
        // Neither the nonce nor the original identity text enters this object.
        return facts
    }

    private static func callbackFact(_ value: Sample, token: String) -> [String: Any] {
        var facts = value.facts
        let sample = value.data.flatMap { String(data: Data($0.prefix(8_192)), encoding: .utf8) }
        facts["sampleUTF8Valid"] = value.data == nil ? NSNull() : (sample != nil)
        facts["sampleBytes"] = value.data.map { min($0.count, 8_192) as Any } ?? NSNull()
        for (name, needle) in [("sampleContainsToken", token), ("sampleContainsSuccess", "result=success"),
                               ("sampleContainsFailure", "result=failure"), ("sampleContainsTimeout", "result=timeout"),
                               ("sampleContainsCancelled", "result=cancelled")] {
            facts[name] = sample.map { $0.contains(needle) as Any } ?? NSNull()
        }
        return facts
    }

    private static func encode(_ value: [String: Any]) -> Data? {
        try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
    }

    private static func shrinkTexts(_ value: Any, budget: Int) -> Any {
        if var dictionary = value as? [String: Any] {
            for (key, nested) in dictionary {
                if key == "value", let text = nested as? String {
                    let clipped = clipUTF8(text, budget: budget)
                    dictionary[key] = clipped
                    if clipped != text { dictionary["valueTruncated"] = true }
                } else { dictionary[key] = shrinkTexts(nested, budget: budget) }
            }
            return dictionary
        }
        return value
    }

    private static func hasTruncation(_ value: Any) -> Bool {
        guard let dictionary = value as? [String: Any] else { return false }
        for (key, nested) in dictionary {
            if (key == "truncated" || key == "valueTruncated" || key == "metadataBudgetTruncated"),
               nested as? Bool == true { return true }
            if hasTruncation(nested) { return true }
        }
        return false
    }

    private static func capturedFileBytes(_ value: Any) -> Int {
        guard let dictionary = value as? [String: Any] else { return 0 }
        return dictionary.reduce(0) { total, entry in
            total + (entry.key == "capturedBytes" ? ((entry.value as? Int) ?? 0) : capturedFileBytes(entry.value))
        }
    }

    private static func ptyFact(_ output: String, token: String, nonces: [String], nonceContextComplete: Bool) -> [String: Any] {
        guard nonceContextComplete else {
            // Missing or invalid nonce identity cannot justify arbitrary PTY text.
            // Match fixed messages but emit only our constant IDs, never substrings.
            let knownMessages = [
                ("helperStarted", "Antigravity login helper started; waiting for the app handoff."),
                ("handoffMissing", "handoff pipe is missing."),
                ("handoffOpenFailed", "could not open the app handoff pipe."),
                ("handoffTimedOut", "timed out waiting for the app handoff"),
                ("handoffInvalid", "the app handoff was invalid;"),
                ("handoffReceived", "handoff received. Waiting for the Antigravity CLI process to start."),
                ("callbackDeliveryFailed", "could not send the login result back to the app."),
                ("guardianCleanupUnconfirmed", "Antigravity cleanup is unconfirmed; preserving guardian state."),
                ("groupMayStillRun", "the Antigravity process group may still be running; preserving guardian state."),
                ("foregroundRestoreFailed", "could not restore the Terminal foreground process group."),
                ("foregroundRestoreCleanupFailed", "could not restore the Terminal foreground process group during cleanup."),
                ("watchdogStartFailed", "could not start the Antigravity login watchdog; the CLI was not started."),
                ("foregroundHandoffFailed", "could not give the Antigravity CLI the Terminal foreground."),
                ("cliStarted", "Antigravity CLI process started; sign-in may still be in progress."),
                ("cliStartFailed", "the Antigravity CLI could not be started."),
                ("cliStartUnconfirmed", "Antigravity CLI startup could not be confirmed."),
                ("cleanupUnconfirmed", "process cleanup could not be confirmed; preserving guardian state."),
                ("processStateUnconfirmed", "could not confirm the Antigravity process state."),
                ("loginExpiredStopped", "Antigravity sign-in timed out and its login process was stopped."),
                ("loginExpiredCleanupUnconfirmed", "Antigravity sign-in timed out; process cleanup is unconfirmed."),
                ("cliFinishedCallback", "Antigravity CLI sign-in finished. Sending the result to the app."),
                ("cliSignInFailed", "Antigravity CLI sign-in failed with exit code"),
                ("syntheticCLIReady", "fake-cli-ready"),
                ("syntheticCLIApproved", "fake-cli-approved"),
                ("shellReadOnlyVariable", "read-only variable"),
                ("shellCommandNotFound", "command not found")
            ]
            return ["contentsOmitted": true, "omissionReason": "guardianNonceUnavailableOrInvalid",
                    "recognizedMessageIDs": knownMessages.filter { output.contains($0.1) }.map { $0.0 },
                    "capturedUTF8Bytes": output.utf8.count, "valueTruncated": true,
                    "captureMode": "existingResultOnly", "finalDrainAdded": false]
        }
        let redacted = redact(output, token: token, nonces: nonces)
        let characters = Array(redacted)
        var chosen = redacted
        var truncated = false
        if ((try? JSONSerialization.data(withJSONObject: [chosen]).count) ?? Int.max) > 48 * 1_024 {
            truncated = true
            var low = 0
            var high = characters.count
            chosen = "[...omitted...]"
            while low <= high {
                let count = low + (high - low) / 2
                let candidate = String(characters.prefix(count / 2)) + "[...omitted...]" + String(characters.suffix(count - count / 2))
                let bytes = (try? JSONSerialization.data(withJSONObject: [candidate]).count) ?? Int.max
                if bytes <= 48 * 1_024 { chosen = candidate; low = count + 1 }
                else { high = count - 1 }
            }
        }
        return ["value": chosen, "capturedUTF8Bytes": output.utf8.count,
                "redactedUTF8Bytes": redacted.utf8.count, "valueTruncated": truncated,
                "captureMode": "existingResultOnly", "finalDrainAdded": false]
    }

    static func emit(phase: String, progress: String, completionExitCode: Int,
                     watchdogDelaySeconds: Int, cancelBeforeExec: Bool,
                     root: URL, scriptURL: URL, groupMarker: URL,
                     cliLog: URL, cliPIDFile: URL, watchdogPIDFile: URL, callbackLog: URL,
                     token: String, modelState: String, isRefreshPaused: Bool,
                     finished: Bool? = nil, cleanupSucceeded: Bool? = nil,
                     terminationStatus: Int32? = nil, ptyOutput: String? = nil) {
        let statusBase = groupMarker.deletingPathExtension()
        let authenticationDirectory = root.appendingPathComponent("Authentication", isDirectory: true).standardizedFileURL
        let rootDescriptor = root.path.withCString { open($0, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC) }
        defer { if rootDescriptor >= 0 { Darwin.close(rootDescriptor) } }
        let authenticationDescriptor: Int32
        if rootDescriptor >= 0, statusBase.deletingLastPathComponent().standardizedFileURL == authenticationDirectory,
           scriptURL.deletingLastPathComponent().standardizedFileURL == authenticationDirectory {
            authenticationDescriptor = "Authentication".withCString {
                openat(rootDescriptor, $0, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC)
            }
        } else { authenticationDescriptor = -1 }
        defer { if authenticationDescriptor >= 0 { Darwin.close(authenticationDescriptor) } }
        let guardian = sample(statusBase.appendingPathExtension("guardian").lastPathComponent, in: authenticationDescriptor, limit: 128)
        let guardianStop = sample(statusBase.appendingPathExtension("guardian-stop").lastPathComponent, in: authenticationDescriptor, limit: 128)
        let nonces = [guardianNonce(guardian), guardianNonce(guardianStop)].compactMap { $0 }
        let nonceContextComplete = guardianNonce(guardian) != nil &&
            (guardianStop.exists == false || guardianNonce(guardianStop) != nil)
        var stop = guardianFact(guardianStop)
        if guardian.truncated == false, guardianStop.truncated == false,
           let identity = guardian.data, let stopIdentity = guardianStop.data {
            stop["stopMatchesGuardian"] = identity == stopIdentity
        } else { stop["stopMatchesGuardian"] = NSNull() }
        var metadata: [String: Any] = [
            "schema": "g11-diagnostic-v19", "phase": phase, "progress": progress, "acceptance": false,
            "parameter": ["completionExitCode": completionExitCode, "watchdogDelaySeconds": watchdogDelaySeconds, "cancelBeforeExec": cancelBeforeExec],
            "retainedByDiagnosticPolicy": true, "preservationReason": "allSyntheticRootsRetainedForDiagnosis",
            "cleanupSafetyClaim": false, "samplingIsAtomic": false,
            "guardianNonceContextAvailableForRedaction": nonceContextComplete,
            "redactionCoverage": "knownTokenVariantsAndNonces_rawSingleDoublePercentSchemes",
            "root": textFact(root.path, token: token, budget: 1_024),
            "script": ["path": textFact(scriptURL.path, token: token, budget: 1_024),
                       "metadata": sample(scriptURL.lastPathComponent, in: authenticationDescriptor, limit: nil).facts],
            "model": ["state": textFact(modelState, token: token, budget: 128), "isGeminiRefreshPaused": isRefreshPaused],
            "status": textSample(sample(statusBase.lastPathComponent, in: authenticationDescriptor, limit: 256), token: token, limit: 256),
            "exec": textSample(sample(statusBase.appendingPathExtension("exec").lastPathComponent, in: authenticationDescriptor, limit: 256), token: token, limit: 256),
            "cancel": textSample(sample(statusBase.appendingPathExtension("cancel").lastPathComponent, in: authenticationDescriptor, limit: 256), token: token, limit: 256),
            "ready": readyFact(sample(statusBase.appendingPathExtension("ready").lastPathComponent, in: authenticationDescriptor, limit: 0)),
            "processGroup": groupFact(sample(groupMarker.lastPathComponent, in: authenticationDescriptor, limit: 64)),
            "guardian": guardianFact(guardian), "guardianStop": stop,
            "launchGate": sample(statusBase.appendingPathExtension("lock").lastPathComponent, in: authenticationDescriptor, limit: nil).facts,
            "callback": callbackFact(sample(callbackLog.lastPathComponent, in: rootDescriptor, limit: 8_192), token: token),
            "cliLog": sample(cliLog.lastPathComponent, in: rootDescriptor, limit: nil).facts,
            "cliPIDFile": sample(cliPIDFile.lastPathComponent, in: rootDescriptor, limit: nil).facts,
            "watchdogPIDFile": sample(watchdogPIDFile.lastPathComponent, in: rootDescriptor, limit: nil).facts,
            "metadataBudgetTruncated": false
        ]
        if phase == "A" {
            let expectedTermination: Int
            switch completionExitCode {
            case 2: expectedTermination = 124
            case 3: expectedTermination = 130
            case 4, 5: expectedTermination = 127
            default: expectedTermination = 0
            }
            metadata["wrapper"] = ["finished": finished.map { $0 as Any } ?? NSNull(),
                                   "cleanupSucceeded": cleanupSucceeded.map { $0 as Any } ?? NSNull(),
                                   "terminationStatus": terminationStatus.map { Int($0) as Any } ?? NSNull(),
                                   "expectedTermination": expectedTermination]
        }
        metadata["capturedFileBytes"] = capturedFileBytes(metadata)
        metadata["truncated"] = hasTruncation(metadata)
        let overhead = prefix.utf8.count + 1
        let failureLine = prefix + "{\"schema\":\"g11-diagnostic-v19\",\"phase\":\"" + (phase == "A" ? "A" : "B") + "\",\"acceptance\":false,\"truncated\":true,\"capturedFileBytes\":null,\"serializationOrBudgetError\":true}"
        for budget in [128, 64, 16, 0] {
            if let encoded = encode(metadata), encoded.count + overhead <= 8 * 1_024 { break }
            metadata = (shrinkTexts(metadata, budget: budget) as? [String: Any]) ?? [:]
            metadata["metadataBudgetTruncated"] = true
            metadata["truncated"] = true
        }
        guard let metadataData = encode(metadata), metadataData.count + overhead <= 8 * 1_024 else {
            print(failureLine)
            return
        }
        var record = metadata
        if phase == "A", let ptyOutput { record["pty"] = ptyFact(ptyOutput, token: token, nonces: nonces, nonceContextComplete: nonceContextComplete) }
        record["truncated"] = hasTruncation(record)
        let recordLimit = phase == "A" ? 64 * 1_024 : 8 * 1_024
        if let data = encode(record), data.count + overhead <= recordLimit,
           let line = String(data: data, encoding: .utf8) {
            print(prefix + line)
        } else {
            // Output failure does not add an Issue, replace an original error,
            // skip a test assertion, or imply complete capture/clean cleanup.
            print(failureLine)
        }
    }
}
