import Foundation
import Testing
@testable import AIMeterApp
@testable import AIMeterCore

@Suite("Antigravity login recovery end-to-end", .serialized)
@MainActor
struct GeminiLoginRecoveryEndToEndTests {
    @Test("Production launcher handles fake callback delivery success and failure", arguments: [0, 1, 2, 3, 4, 5])
    func productionLauncherHandshake(completionExitCode: Int) async throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("ai-meter-full-auth-flow-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(
            at: root,
            withIntermediateDirectories: true,
            attributes: [.posixPermissions: 0o700]
        )
        defer { try? FileManager.default.removeItem(at: root) }

        let cliLog = root.appendingPathComponent("agy.log")
        let callbackLog = root.appendingPathComponent("callbacks.log")
        let fakeCLI = root.appendingPathComponent("agy")
        let fakeOpen = root.appendingPathComponent("open")
        let fakeWatchdog = root.appendingPathComponent("watchdog")
        let watchdogPidFile = root.appendingPathComponent("watchdog.pid")
        let fakeCLIContents = completionExitCode == 3 || completionExitCode == 4
            ? "#!/bin/sh\nexec /bin/sleep 30\n"
            : "#!/bin/sh\nprintf 'agy-started\\n' > '\(cliLog.path)'\nexit 0\n"
        try Data(fakeCLIContents.utf8).write(to: fakeCLI)
        try Data("#!/bin/sh\nprintf '%s\\n' \"$*\" >> '\(callbackLog.path)'\nexit \(completionExitCode == 1 ? 1 : 0)\n".utf8).write(to: fakeOpen)
        let fakeWatchdogContents = completionExitCode == 5
            ? "#!/bin/sh\nprintf '%s' \"$$\" > '\(watchdogPidFile.path)'\nexit 0\n"
            : "#!/bin/sh\nexit 1\n"
        try Data(fakeWatchdogContents.utf8).write(to: fakeWatchdog)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: fakeCLI.path)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: fakeOpen.path)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: fakeWatchdog.path)

        var scriptProcess: Process?
        var openedToken: String?
        let launcher = CLIAuthenticationLauncher(
            authenticationDirectoryURL: root.appendingPathComponent("Authentication", isDirectory: true),
            executableLocator: TemporaryFullFlowLocator(executable: fakeCLI),
            openURL: { scriptURL in
                let expectedScript = root.appendingPathComponent("Authentication/Open Antigravity Login.command")
                guard scriptURL.standardizedFileURL == expectedScript.standardizedFileURL else { return false }
                guard let generated = try? String(contentsOf: scriptURL, encoding: .utf8) else { return false }
                guard generated.contains("open_command='\(fakeOpen.path)'"),
                      !generated.contains("/usr/bin/open") else { return false }
                if completionExitCode == 2 {
                    let pipe = try? FileManager.default.contentsOfDirectory(at: expectedScript.deletingLastPathComponent(), includingPropertiesForKeys: nil)
                        .first(where: { $0.pathExtension == "token" })
                    if let pipe { try? FileManager.default.removeItem(at: pipe) }
                }
                let process = Process()
                process.executableURL = URL(fileURLWithPath: "/bin/zsh")
                process.arguments = [scriptURL.path]
                process.currentDirectoryURL = root
                process.environment = [
                    "HOME": root.path,
                    "ZDOTDIR": root.path,
                    "TMPDIR": root.path,
                    "PATH": "\(root.path):/usr/bin:/bin:/usr/sbin:/sbin",
                    "LANG": "C",
                    "LC_ALL": "C",
                ]
                do {
                    try process.run()
                    scriptProcess = process
                    return true
                } catch {
                    return false
                }
            },
            completionOpenExecutableURL: fakeOpen,
            watchdogExecutableURL: completionExitCode == 4 || completionExitCode == 5
                ? fakeWatchdog
                : URL(fileURLWithPath: "/usr/bin/perl")
        )

        let defaultsName = "TemporaryFullGeminiLauncherFlowTests.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: defaultsName))
        defer { defaults.removePersistentDomain(forName: defaultsName) }
        let model = AppModel(
            defaults: defaults,
            applicationSupportDirectoryURL: root.appendingPathComponent("app-support", isDirectory: true),
            widgetSnapshotPublisher: nil,
            isDemoMode: false,
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
                    provider: .gemini,
                    primaryMetric: metric,
                    fetchedAt: Date(),
                    collectionStatus: .fresh,
                    geminiQuotaMetrics: [metric],
                    antigravitySharedQuotaMetrics: [sharedFiveHour, sharedWeekly]
                )
            },
            geminiAuthenticationOpenOperation: { token in
                openedToken = token
                _ = try launcher.open(provider: .gemini, completionToken: token)
            },
            geminiAuthenticationCancelOperation: { launcher.cancelPendingGeminiLogin(token: $0) },
            geminiAuthenticationStopOperation: { launcher.requestGeminiLoginStop(token: $0) },
            geminiAuthenticationFailureOperation: { launcher.hasGeminiTerminalFailure(token: $0) },
            geminiRecoveryTimeout: completionExitCode == 3 ? .milliseconds(500) : .seconds(310)
        )

        await model.refresh()
        #expect(model.isGeminiRefreshPaused)
        await model.beginGeminiOneTimeRecovery()
        let token = try #require(openedToken)
        let process = try #require(scriptProcess)
        if completionExitCode == 5 {
            let pidDeadline = Date().addingTimeInterval(3)
            var watchdogPid = ""
            while watchdogPid.isEmpty, Date() < pidDeadline {
                watchdogPid = (try? String(contentsOf: watchdogPidFile, encoding: .utf8))?
                    .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
                if !watchdogPid.isEmpty { break }
                try await Task.sleep(for: .milliseconds(10))
            }
            #expect(!watchdogPid.isEmpty)
            let statusFile = try #require(try FileManager.default.contentsOfDirectory(
                at: root.appendingPathComponent("Authentication"),
                includingPropertiesForKeys: nil
            ).first(where: { $0.pathExtension == "status" }))
            let readyFile = statusFile.appendingPathExtension("ready")
            let watcherDeadline = Date().addingTimeInterval(3)
            var watchdogState = ""
            while Date() < watcherDeadline {
                let ps = Process()
                ps.executableURL = URL(fileURLWithPath: "/bin/ps")
                ps.arguments = ["-o", "stat=", "-p", watchdogPid]
                let output = Pipe()
                ps.standardOutput = output
                try ps.run()
                ps.waitUntilExit()
                watchdogState = String(data: output.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8)?
                    .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
                if watchdogState.isEmpty || watchdogState.contains("Z") { break }
                try await Task.sleep(for: .milliseconds(10))
            }
            #expect(watchdogState.isEmpty || watchdogState.contains("Z"))
            try Data().write(to: readyFile)
        }
        let deadline = Date().addingTimeInterval(8)
        while process.isRunning, Date() < deadline {
            try await Task.sleep(for: .milliseconds(20))
        }
        let finished = !process.isRunning
        if !finished { process.terminate() }
        process.waitUntilExit()
        #expect(finished)
        if completionExitCode == 2 {
            #expect(process.terminationStatus == 124)
            #expect(!FileManager.default.fileExists(atPath: cliLog.path))
        } else if completionExitCode == 3 {
            #expect(process.terminationStatus == 130)
            #expect(model.geminiOneTimeRecoveryState == AppModel.GeminiOneTimeRecoveryState.loginExpired)
            #expect(!FileManager.default.fileExists(atPath: cliLog.path))
        } else if completionExitCode == 4 || completionExitCode == 5 {
            #expect(process.terminationStatus == 127)
            #expect(!FileManager.default.fileExists(atPath: cliLog.path))
        } else {
            #expect(process.terminationStatus == 0)
            #expect(FileManager.default.fileExists(atPath: cliLog.path))
        }
        let callback = (try? String(contentsOf: callbackLog, encoding: .utf8)) ?? ""
        if completionExitCode != 2 && completionExitCode != 3 && completionExitCode != 4 && completionExitCode != 5 {
            #expect(callback.contains(token))
        }
        if completionExitCode == 0 {
            #expect(callback.contains("result=success"))
            let callbackURLText = try #require(callback.split(whereSeparator: \.isWhitespace).last.map(String.init))
            let callbackURL = try #require(URL(string: callbackURLText))
            let receipt = try #require(GeminiLoginReceipt(url: callbackURL))
            #expect(receipt.token == token)
            #expect(receipt.result == .success)
            await model.completeGeminiInteractiveSignIn(token: receipt.token, result: receipt.result)
            #expect(model.geminiOneTimeRecoveryState == AppModel.GeminiOneTimeRecoveryState.succeeded)
            #expect(model.snapshots.first(where: { $0.provider == UsageProvider.gemini })?.geminiQuotaMetrics?.first?.current == 80)
            #expect(model.snapshots.first(where: { $0.provider == UsageProvider.gemini })?.antigravitySharedQuotaMetrics?.map(\.current) == [80, 20])
        } else if completionExitCode == 1 || completionExitCode == 2 || completionExitCode == 4 || completionExitCode == 5 {
            let failureDeadline = Date().addingTimeInterval(3)
            while model.geminiOneTimeRecoveryState == AppModel.GeminiOneTimeRecoveryState.awaitingLogin,
                  Date() < failureDeadline {
                try await Task.sleep(for: .milliseconds(20))
            }
            #expect(model.geminiOneTimeRecoveryState == AppModel.GeminiOneTimeRecoveryState.loginFailed)
        }
        #expect(model.isGeminiRefreshPaused)
        #expect(!FileManager.default.fileExists(atPath: root.appendingPathComponent("Authentication/Open Antigravity Login.command").path))
        let authenticationDirectory = root.appendingPathComponent("Authentication", isDirectory: true)
        func statusFiles() -> [URL] {
            ((try? FileManager.default.contentsOfDirectory(
                at: authenticationDirectory,
                includingPropertiesForKeys: nil
            )) ?? []).filter { $0.pathExtension == "status" }
        }
        if completionExitCode == 3 {
            let cleanupDeadline = Date().addingTimeInterval(3)
            while !statusFiles().isEmpty, Date() < cleanupDeadline {
                try await Task.sleep(for: .milliseconds(20))
            }
        }
        #expect(statusFiles().isEmpty)
    }
}

private struct TemporaryFullFlowLocator: ExecutableLocating {
    let executable: URL
    func locate(named name: String) -> URL? { name == "agy" ? executable : nil }
}
