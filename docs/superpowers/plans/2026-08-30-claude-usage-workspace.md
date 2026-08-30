# Claude Isolated Usage Workspace Implementation Plan

> **For AI agents:** Required sub-skill: use superpowers:executing-plans to implement this plan task by task. Track progress with the checkboxes below.

**Goal:** Make Claude usage collection run in a narrow, explicitly trusted AI Meter workspace and replace the misleading timeout with a safe one-time setup flow.

**Architecture:** Extend the PTY command boundary with an optional working directory and output stop phrases. Resolve one stable Application Support workspace for Claude, detect the trust screen without approving it, and expose a setup-required state. The macOS app creates a small `.command` launcher that opens Claude in that workspace so the user can approve only that directory.

**Tech stack:** Swift 6, Swift Testing, Foundation `Process`/PTY APIs, AppKit `NSWorkspace`, SwiftUI, shell fixtures, Swift Package Manager.

---

## File structure

- Modify `Sources/AIMeterCore/Collectors/CommandRunner.swift`: add working-directory and stop-phrase request inputs.
- Modify `Sources/AIMeterCore/Collectors/PTYCommandRunner.swift`: apply the directory and stop a child after a recognized output phrase while retaining captured output.
- Modify `Tests/AIMeterCoreTests/PTYCommandRunnerTests.swift`: cover real child working directory and bounded stop-phrase behavior.
- Modify `Tests/AIMeterCoreTests/Fixtures/fake-interactive-cli.sh`: provide deterministic `pwd` and trust-screen fixtures.
- Create `Sources/AIMeterCore/Collectors/ClaudeUsageWorkspace.swift`: resolve/create the isolated Application Support workspace through a testable protocol.
- Modify `Sources/AIMeterCore/Collectors/ClaudeCollector.swift`: use the isolated workspace, safe textual CLI mode, and trust-screen detection.
- Modify `Sources/AIMeterCore/Collectors/UsageCollector.swift`: add the setup-required collection error.
- Modify `Tests/AIMeterCoreTests/CLICollectorTests.swift`: verify directory propagation, CLI arguments, and trust-screen error mapping.
- Modify `Sources/AIMeterCore/Domain/UsageModels.swift`: add the setup-required collection status.
- Modify `Sources/AIMeterCore/Coordination/RefreshCoordinator.swift`: map setup-required to an actionable snapshot.
- Modify `Sources/AIMeterCore/Presentation/AppPresentation.swift`: format setup-required distinctly from sign-in and unavailable.
- Modify `Tests/AIMeterCoreTests/RefreshCoordinatorTests.swift`: verify coordinator mapping and cache behavior.
- Modify `Tests/AIMeterCoreTests/AppPresentationTests.swift`: verify setup copy and semantic state.
- Create `Sources/AIMeterCore/Collectors/ClaudeSetupScriptBuilder.swift`: build a credential-free, shell-escaped setup launcher.
- Create `Tests/AIMeterCoreTests/ClaudeSetupScriptBuilderTests.swift`: verify workspace and executable paths cannot alter the script.
- Create `Sources/AIMeterApp/System/ClaudeWorkspaceSetupLauncher.swift`: create and open the safe Terminal launcher without embedding credentials.
- Modify `Sources/AIMeterApp/AppModel.swift`: expose the setup action and a non-sensitive error message.
- Modify `Sources/AIMeterApp/Views/ProviderCard.swift`: show the setup action in the menu-bar card.
- Modify `Sources/AIMeterApp/Views/MenuBarPanel.swift`: connect Claude setup action.
- Modify `Sources/AIMeterApp/Views/FloatingStripView.swift`: show the same setup action in floating detail.
- Modify `Sources/AIMeterApp/System/FloatingPanelController.swift`: connect the floating detail action.
- Modify `docs/development/2026-08-28-development-log.md`: record diagnosis, safety decision, tests, installation, and UI evidence.

### Task 1: PTY working directory and stop phrases

**Files:**
- Modify: `Sources/AIMeterCore/Collectors/CommandRunner.swift`
- Modify: `Sources/AIMeterCore/Collectors/PTYCommandRunner.swift`
- Modify: `Tests/AIMeterCoreTests/PTYCommandRunnerTests.swift`
- Modify: `Tests/AIMeterCoreTests/Fixtures/fake-interactive-cli.sh`

- [ ] **Step 1: Write failing tests**

Add tests that would fail if `Process.currentDirectoryURL` were not set or if the runner waited until timeout after a known trust message:

```swift
@Test("Runs the child in the requested working directory")
func usesRequestedWorkingDirectory() async throws {
    let directory = FileManager.default.temporaryDirectory
        .appendingPathComponent("ai-meter-cwd-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: directory) }

    let result = try await PTYCommandRunner().run(CommandRequest(
        executableURL: fixtureExecutable,
        inputLines: ["pwd"],
        timeout: 2,
        currentDirectoryURL: directory
    ))

    #expect(ANSITextSanitizer.sanitize(result.output).contains(directory.path))
}

@Test("Returns captured output when a configured stop phrase appears")
func stopsOnConfiguredOutput() async throws {
    let startedAt = Date()
    let result = try await PTYCommandRunner().run(CommandRequest(
        executableURL: fixtureExecutable,
        inputLines: ["trust"],
        timeout: 2,
        stopAfterOutputContains: ["Permission Required: Accessing workspace"]
    ))

    #expect(result.output.contains("Permission Required: Accessing workspace"))
    #expect(Date().timeIntervalSince(startedAt) < 1)
}
```

Extend the fixture with `pwd)` to print `$PWD` and `trust)` to print the trust screen and sleep.

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```bash
swift test --disable-sandbox --filter PTYCommandRunnerTests
```

Expected: compile failure because `CommandRequest` has no working-directory or stop-phrase parameters.

- [ ] **Step 3: Implement the minimum command boundary**

Extend `CommandRequest` with defaulted properties:

```swift
public let currentDirectoryURL: URL?
public let stopAfterOutputContains: [String]
```

In `PTYCommandRunner`, assign `process.currentDirectoryURL`, pass the stop phrases to the reader, and have the reader call `processBox.stop()` after the accumulated decoded output contains a configured phrase. Continue the existing bounded drain so the matching text remains in `CommandResult.output`.

- [ ] **Step 4: Run focused and full tests and verify GREEN**

Run the focused test, then `swift test --disable-sandbox`. Expected: all tests pass; environment-gated smoke tests remain skipped.

- [ ] **Step 5: Commit**

```bash
git add Sources/AIMeterCore/Collectors/CommandRunner.swift Sources/AIMeterCore/Collectors/PTYCommandRunner.swift Tests/AIMeterCoreTests/PTYCommandRunnerTests.swift Tests/AIMeterCoreTests/Fixtures/fake-interactive-cli.sh
git commit -m "feat: constrain interactive command execution"
```

### Task 2: Isolated Claude collector and setup-required state

**Files:**
- Create: `Sources/AIMeterCore/Collectors/ClaudeUsageWorkspace.swift`
- Modify: `Sources/AIMeterCore/Collectors/ClaudeCollector.swift`
- Modify: `Sources/AIMeterCore/Collectors/UsageCollector.swift`
- Modify: `Tests/AIMeterCoreTests/CLICollectorTests.swift`
- Create: `Tests/AIMeterCoreTests/Fixtures/fake-untrusted-claude.sh`

- [ ] **Step 1: Write failing collector tests**

Define a test resolver and an actor-backed runner in the test file:

```swift
private struct FixedWorkspaceResolver: ClaudeUsageWorkspaceResolving {
    let url: URL
    func resolve() throws -> URL { url }
}

private actor RecordingClaudeRunner: CommandRunning {
    private var requests: [CommandRequest] = []

    func run(_ request: CommandRequest) async throws -> CommandResult {
        requests.append(request)
        let output = request.arguments == ["auth", "status"]
            ? #"{"loggedIn":true}"#
            : "Current session\n73% used\nResets in 51 min\nAll models\n7% used\nResets Thu 12:00 AM\n"
        return CommandResult(output: output, exitCode: 0, duration: 0)
    }

    func recordedRequests() -> [CommandRequest] { requests }
}
```

Add two behaviors:

```swift
@Test("Claude collector runs all commands in the isolated workspace")
func claudeUsesIsolatedWorkspace() async throws {
    let workspace = FileManager.default.temporaryDirectory
        .appendingPathComponent("ai-meter-claude-\(UUID().uuidString)", isDirectory: true)
    let runner = RecordingClaudeRunner()
    let collector = ClaudeCollector(
        runner: runner,
        locator: FixedLocator(url: fixtureExecutable),
        workspaceResolver: FixedWorkspaceResolver(url: workspace)
    )

    _ = try await collector.collect()
    let requests = await runner.recordedRequests()
    #expect(requests.allSatisfy { $0.currentDirectoryURL == workspace })
    #expect(requests.last?.arguments == ["--ax-screen-reader", "--safe-mode"])
}

@Test("Claude trust screen reports setup required")
func claudeTrustScreenRequiresSetup() async {
    let workspace = FileManager.default.temporaryDirectory
        .appendingPathComponent("ai-meter-untrusted-\(UUID().uuidString)", isDirectory: true)
    let collector = ClaudeCollector(
        runner: PTYCommandRunner(),
        locator: FixedLocator(url: untrustedClaudeExecutable),
        workspaceResolver: FixedWorkspaceResolver(url: workspace)
    )

    await #expect(throws: UsageCollectionError.setupRequired) {
        try await collector.collect()
    }
}
```

- [ ] **Step 2: Run focused tests and verify RED**

Run `swift test --disable-sandbox --filter CLICollectorTests`. Expected: compile failure for the missing resolver and `.setupRequired` case.

- [ ] **Step 3: Implement the minimum isolated collector**

Add:

```swift
public protocol ClaudeUsageWorkspaceResolving: Sendable {
    func resolve() throws -> URL
}

public struct ClaudeUsageWorkspaceResolver: ClaudeUsageWorkspaceResolving {
    public func resolve() throws -> URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let workspace = base
            .appendingPathComponent("AI Meter", isDirectory: true)
            .appendingPathComponent("ClaudeUsageWorkspace", isDirectory: true)
        try FileManager.default.createDirectory(at: workspace, withIntermediateDirectories: true)
        return workspace
    }
}
```

Resolve once per collection, pass the URL to both requests, use `--ax-screen-reader --safe-mode` for interactive usage, set the stop phrase to Claude's trust heading, detect the sanitized trust text before parsing, and throw `.setupRequired`. Map directory creation errors to `.transportFailure`.

- [ ] **Step 4: Verify GREEN and regression safety**

Run focused and full tests. Expected: all tests pass and the fake trust process exits well before its timeout.

- [ ] **Step 5: Commit**

```bash
git add Sources/AIMeterCore/Collectors/ClaudeUsageWorkspace.swift Sources/AIMeterCore/Collectors/ClaudeCollector.swift Sources/AIMeterCore/Collectors/UsageCollector.swift Tests/AIMeterCoreTests/CLICollectorTests.swift Tests/AIMeterCoreTests/Fixtures/fake-untrusted-claude.sh
git commit -m "fix: isolate Claude usage collection"
```

### Task 3: Actionable setup presentation and launcher

**Files:**
- Modify: `Sources/AIMeterCore/Domain/UsageModels.swift`
- Modify: `Sources/AIMeterCore/Coordination/RefreshCoordinator.swift`
- Modify: `Sources/AIMeterCore/Presentation/AppPresentation.swift`
- Modify: `Tests/AIMeterCoreTests/RefreshCoordinatorTests.swift`
- Modify: `Tests/AIMeterCoreTests/AppPresentationTests.swift`
- Create: `Sources/AIMeterCore/Collectors/ClaudeSetupScriptBuilder.swift`
- Create: `Tests/AIMeterCoreTests/ClaudeSetupScriptBuilderTests.swift`
- Create: `Sources/AIMeterApp/System/ClaudeWorkspaceSetupLauncher.swift`
- Modify: `Sources/AIMeterApp/AppModel.swift`
- Modify: `Sources/AIMeterApp/Views/ProviderCard.swift`
- Modify: `Sources/AIMeterApp/Views/MenuBarPanel.swift`
- Modify: `Sources/AIMeterApp/Views/FloatingStripView.swift`
- Modify: `Sources/AIMeterApp/System/FloatingPanelController.swift`

- [ ] **Step 1: Write failing core presentation tests**

Add coordinator and presentation assertions:

```swift
#expect(snapshot.collectionStatus == .setupRequired)
#expect(snapshot.statusMessage == "Approve the private usage workspace once")

let presentation = ProviderPresentation(snapshot: snapshot)
#expect(presentation.valueText == "Set up")
#expect(presentation.detailText == "One-time Claude workspace approval")
#expect(presentation.semantic == .unavailable)
```

Name the test breakages explicitly: mapping `.setupRequired` to generic unavailable, or presenting it as sign-in, must fail.

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
swift test --disable-sandbox --filter RefreshCoordinatorTests
swift test --disable-sandbox --filter AppPresentationTests
```

Expected: compile failure because `CollectionStatus.setupRequired` and error mapping do not exist.

- [ ] **Step 3: Implement the core state and presentation**

Add `.setupRequired` to `CollectionStatus`, map the collection error in `RefreshCoordinator`, and return the specified value/detail text from `ProviderPresentation`.

- [ ] **Step 4: Write the failing setup-script safety test**

The production change this test must catch is interpolating a path without quoting it, allowing spaces or single quotes to change the shell command:

```swift
@Test("Setup script quotes workspace and Claude executable paths")
func quotesPaths() {
    let script = ClaudeSetupScriptBuilder().build(
        workspaceURL: URL(fileURLWithPath: "/tmp/AI Meter/Claude's Workspace"),
        executableURL: URL(fileURLWithPath: "/tmp/Claude Tools/claude")
    )

    #expect(script.contains("cd -- '/tmp/AI Meter/Claude'\\''s Workspace'"))
    #expect(script.contains("exec '/tmp/Claude Tools/claude' --ax-screen-reader --safe-mode"))
}
```

Run `swift test --disable-sandbox --filter ClaudeSetupScriptBuilderTests`. Expected: compile failure because the builder does not exist.

- [ ] **Step 5: Implement the safe Terminal launcher**

Create `ClaudeSetupScriptBuilder` in the core target. Its only responsibility is POSIX single-quote escaping and generating the following credential-free shape:

```zsh
#!/bin/zsh
cd -- '<escaped workspace>'
exec '<escaped Claude executable>' --ax-screen-reader --safe-mode
```

Then create an AppKit launcher that resolves the same workspace and executable, writes the builder output to an executable `.command` file in `~/Library/Application Support/AI Meter`, and opens that file with `NSWorkspace`. Do not include credentials or automatically send trust input. `AppModel.openClaudeWorkspaceSetup()` invokes the launcher and exposes only a generic settings message if setup cannot be opened.

- [ ] **Step 6: Connect the action in both UI surfaces**

When Claude has `.setupRequired`, show `Button("Open one-time setup")` in `ProviderCard` and `FloatingDetailView`. Pass the action from `MenuBarPanel` and `FloatingPanelController`. Other providers and states render exactly as before.

- [ ] **Step 7: Verify GREEN, build, and commit**

Run all focused tests, the full suite, and `scripts/build-app.sh`. Expected: tests and build pass with no new warnings.

```bash
git add Sources/AIMeterCore Sources/AIMeterApp Tests/AIMeterCoreTests
git commit -m "feat: guide Claude workspace setup"
```

### Task 4: Real CLI acceptance, installation, and evidence

**Files:**
- Modify: `docs/development/2026-08-28-development-log.md`

- [ ] **Step 1: Build and sign the release app**

Run `scripts/build-app.sh`, validate `dist/AI Meter.app` with `codesign --verify --deep --strict`, `plutil -lint`, and `file` on the executable.

- [ ] **Step 2: Install recoverably**

Quit the current app, move `/Applications/AI Meter.app` to a timestamped directory under `/private/tmp`, copy the new app into `/Applications`, and launch it. Verify installed and `dist` binary hashes match.

- [ ] **Step 3: Complete explicit one-time trust**

Use the UI setup button or open the generated setup launcher. At Claude's prompt, verify the path is exactly the isolated Application Support workspace. The user explicitly chooses trust; AI Meter never types the answer. Exit Claude after its prompt appears.

- [ ] **Step 4: Run the real smoke test**

Run:

```bash
AI_METER_RUN_CLI_SMOKE=1 swift test --disable-sandbox --filter CLIIntegrationSmokeTests/installedClaudeReturnsRecognizedState
```

Expected: Claude collector returns a recognized snapshot without timing out.

- [ ] **Step 5: Verify the installed UI**

Restart AI Meter and inspect its accessibility state. Expected: Claude no longer says `Unavailable`, `Sign in`, or `Set up`; its ring and detail show current usage, and Codex/DeepSeek remain populated.

- [ ] **Step 6: Record evidence and commit**

Append the root cause, security decision, focused/full test counts, real smoke result, package checks, backup path, installed hash match, and UI result to the development log. Do not record OAuth URLs, tokens, raw account responses, or full home paths.

```bash
git add docs/development/2026-08-28-development-log.md
git commit -m "docs: record Claude workspace verification"
```

- [ ] **Step 7: Final verification**

Run `git diff --check`, the full tests, release build/sign checks, and `git status --short`. Expected: all checks pass and the worktree is clean.
