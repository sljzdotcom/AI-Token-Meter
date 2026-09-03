import Foundation
import Testing

@Suite("GitHub Actions toolchain")
struct CIWorkflowTests {
    @Test("CI selects the Swift toolchain required by project language features")
    func selectsSupportedXcode() throws {
        let projectRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let workflow = try String(contentsOf: projectRoot.appending(
            path: ".github/workflows/ci.yml"
        ))

        #expect(workflow.contains("DEVELOPER_DIR: /Applications/Xcode_26.3.app/Contents/Developer"))
        #expect(workflow.contains("uses: actions/checkout@v5"))
        #expect(workflow.contains("run: swift --version"))
    }

    @Test("Full validation isolates PTY resource tests from the general test process")
    func isolatesPTYResourceTests() throws {
        let projectRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let testScript = try String(contentsOf: projectRoot.appending(path: "scripts/test.sh"))

        #expect(testScript.contains("--skip PTYCommandRunnerTests"))
        #expect(testScript.contains("--filter PTYCommandRunnerTests"))
        #expect(testScript.contains("--skip-build"))
    }

    @Test("Cross-platform releases wait for macOS verification and a signed Windows updater")
    func crossPlatformReleaseGate() throws {
        let projectRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let workflow = try String(
            contentsOf: projectRoot.appending(path: ".github/workflows/release.yml"),
            encoding: .utf8
        )

        #expect(workflow.contains("workflow_dispatch:"))
        #expect(workflow.contains("runs-on: macos-15"))
        #expect(workflow.contains("runs-on: windows-latest"))
        #expect(workflow.contains("TAURI_SIGNING_PRIVATE_KEY"))
        #expect(workflow.contains("tauri.release.conf.json"))
        #expect(workflow.contains("tauri.preview.release.conf.json"))
        #expect(workflow.contains("ruby scripts/check-cross-platform-contracts.rb ."))
        #expect(workflow.contains("AI-Token-Meter-${VERSION}-windows-x64-setup.exe"))
        #expect(workflow.contains("latest.json"))
        #expect(workflow.contains("latest-preview.json"))
        #expect(workflow.contains("windows-preview-feed"))
        #expect(workflow.contains("needs: [macos-preflight, windows-release]"))
        #expect(workflow.contains("gh release edit \"v${VERSION}\" --draft=false"))
        #expect(workflow.contains("--example verify_update_signature"))
        #expect(workflow.contains("group: cross-platform-release"))
        #expect(workflow.contains("Back up public update feeds before publication"))
        #expect(workflow.contains("Rollback public update feeds and release visibility"))
        #expect(workflow.contains("prior-appcast.xml"))
        #expect(workflow.contains("prior-preview-feed.json"))
        #expect(workflow.contains("target-release-was-draft"))
        #expect(workflow.contains("publication-attempted"))
        #expect(workflow.contains("release_may_return_to_draft"))
        #expect(workflow.contains("probe_github_release"))
        #expect(workflow.contains("exact public recovery assets"))
        #expect(workflow.contains("--draft=true"))
        let publishRange = try #require(workflow.range(of: "--draft=false"))
        let stableFeedRange = try #require(workflow.range(of: "Publish the stable macOS appcast"))
        #expect(publishRange.lowerBound < stableFeedRange.lowerBound)
    }

    @Test("Local release entry preserves the macOS Keychain boundary")
    func localCrossPlatformReleaseEntry() throws {
        let projectRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let script = try String(
            contentsOf: projectRoot.appending(path: "scripts/package-cross-platform-release.sh"),
            encoding: .utf8
        )

        #expect(script.contains("package-update-release.sh"))
        #expect(script.contains("gh release create"))
        #expect(script.contains("--draft"))
        #expect(script.contains("gh workflow run release.yml"))
        #expect(script.contains("--ref \"v$VERSION\""))
        #expect(script.contains("AI-Token-Meter-${VERSION}-macOS-arm64.zip"))
        #expect(script.contains("AI_METER_RELEASE_CHANNEL"))
        #expect(script.contains("preview-appcast.xml"))
        #expect(script.contains("SmartScreen"))
        #expect(script.contains("unknown publisher"))
        #expect(!script.contains("git add appcast.xml"))
        #expect(!script.contains("TAURI_SIGNING_PRIVATE_KEY="))
        #expect(!script.contains("security export"))
    }
}
