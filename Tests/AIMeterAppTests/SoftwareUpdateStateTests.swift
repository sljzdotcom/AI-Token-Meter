import Foundation
import Testing
@testable import AIMeterApp

@Suite("Software update state")
struct SoftwareUpdateStateTests {
    private let release = SoftwareUpdateRelease(
        version: "0.2.1",
        build: "5",
        publishedAt: Date(timeIntervalSince1970: 1_788_192_000),
        summary: "  Safer updates.\n\nFaster checks.  "
    )

    @Test("Only an available release enables Update Now")
    func updateNowAvailability() {
        #expect(!SoftwareUpdateState.idle.canInstall)
        #expect(!SoftwareUpdateState.checking.canInstall)
        #expect(!SoftwareUpdateState.upToDate.canInstall)
        #expect(SoftwareUpdateState.available(release).canInstall)
        #expect(!SoftwareUpdateState.installing(release).canInstall)
        #expect(!SoftwareUpdateState.failed("The update check failed.").canInstall)
    }

    @Test("Busy phases reject another check")
    func busyCheckPolicy() {
        #expect(SoftwareUpdateState.idle.canCheck)
        #expect(!SoftwareUpdateState.checking.canCheck)
        #expect(SoftwareUpdateState.upToDate.canCheck)
        #expect(SoftwareUpdateState.available(release).canCheck)
        #expect(!SoftwareUpdateState.installing(release).canCheck)
        #expect(SoftwareUpdateState.failed("The update check failed.").canCheck)
    }

    @Test("Every phase has concise user visible status")
    func statusText() {
        #expect(SoftwareUpdateState.idle.statusText == "Not checked yet")
        #expect(SoftwareUpdateState.checking.statusText == "Checking…")
        #expect(SoftwareUpdateState.upToDate.statusText == "You’re up to date")
        #expect(SoftwareUpdateState.available(release).statusText == "Version 0.2.1 is available")
        #expect(SoftwareUpdateState.installing(release).statusText == "Preparing version 0.2.1…")
        #expect(SoftwareUpdateState.failed("The update check timed out.").statusText == "The update check timed out.")
    }

    @Test("Only release phases expose release metadata")
    func availableRelease() {
        #expect(SoftwareUpdateState.idle.availableRelease == nil)
        #expect(SoftwareUpdateState.available(release).availableRelease == release)
        #expect(SoftwareUpdateState.installing(release).availableRelease == release)
    }

    @Test("Release summaries collapse whitespace and stay bounded")
    func sanitizedSummary() {
        #expect(release.sanitizedSummary == "Safer updates. Faster checks.")

        let longRelease = SoftwareUpdateRelease(
            version: "0.2.2",
            build: "6",
            publishedAt: nil,
            summary: String(repeating: "x", count: 400)
        )
        #expect(longRelease.sanitizedSummary?.count == 240)
    }

    @Test("A blank release summary is omitted")
    func blankSummary() {
        let blankRelease = SoftwareUpdateRelease(
            version: "0.2.2",
            build: "6",
            publishedAt: nil,
            summary: " \n\t "
        )
        #expect(blankRelease.sanitizedSummary == nil)
    }
}
