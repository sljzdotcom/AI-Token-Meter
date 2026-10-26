import Foundation
import Testing
@testable import AIMeterApp
@testable import AIMeterCore

@Suite("Code0 native preparation failure boundaries", .serialized)
@MainActor
struct Code0NativeFailureBoundaryTests {
    @Test func zeroResourceCreationFailureReleasesActualSessionAndLeaseWithoutBrowser() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("Code0NoResource-\(UUID())")
        defer { try? FileManager.default.removeItem(at: root) }
        let profile = Code0ProfileLease(location: Code0StorageLocation(applicationSupportRoot: root))
        defer { profile.release() }
        var resourceReads = 0
        let driver = Code0NativeWebDriver(profile: profile, canonicalSourceLoader: {
            resourceReads += 1
            throw Code0SessionFailure.validationRequired
        })
        let clock = PreparationFailureClock()
        let session = Code0WebSession(clock: clock, driver: driver, store: Code0SnapshotStore(profile: profile))
        defer { session.stop(); driver.releaseProfileLease() }
        #expect(session.connect() == .started)
        #expect(resourceReads == 1)
        // A failed prepare has zero owned objects. It must still finish the real
        // request's release protocol rather than require a fabricated delegate.
        let deadline = ContinuousClock.now.advanced(by: .seconds(1))
        while session.state == .closing, ContinuousClock.now < deadline {
            try await Task.sleep(for: .milliseconds(10))
        }
        #expect(session.state == .paused)
        #expect(session.failure == .navigationFailed)
        #expect(!session.isAutomaticallySyncing)
        #expect(session.snapshot == nil)
        clock.advance(11)
        session.processScheduledWork()
        #expect(session.state != .cleanupPending, "No native object was ever created, so there is no uncertain close")
        session.stop()
        #expect(!profile.isHeld, "The real session must be able to release its OS lease after zero-object failure")
        driver.releaseProfileLease() // bounded test cleanup even on the known RED path
    }

    @Test func unknownAndLongKeysAreRejectedBeforeEnvelopeSerialization() {
        let profile = Code0ProfileLease(location: Code0StorageLocation(applicationSupportRoot:
            FileManager.default.temporaryDirectory.appendingPathComponent("Code0Keys-\(UUID())")))
        let driver = Code0NativeWebDriver(profile: profile)
        for key in ["unknownField", String(repeating: "k", count: 4096)] {
            var serializationCalls = 0
            let result = driver.encodeBridgePayload([key: 0], serialize: { payload in
                serializationCalls += 1
                return try JSONSerialization.data(withJSONObject: payload)
            })
            #expect(result == nil)
            #expect(serializationCalls == 0, "Keys must be bounded and recognized before allocating serialized Data")
        }
        var validSerializationCalls = 0
        let valid = driver.encodeBridgePayload([
            "version": 1, "nonce": UUID().uuidString, "document": UUID().uuidString,
            "sequence": 1, "kind": "ready",
        ], serialize: { payload in
            validSerializationCalls += 1
            return try JSONSerialization.data(withJSONObject: payload)
        })
        #expect(valid != nil, "The pre-allocation guard must preserve valid canonical envelopes")
        #expect(validSerializationCalls == 1)
        #expect(!profile.isHeld)
    }
}

@MainActor
private final class PreparationFailureClock: Code0SessionClock {
    var now = ContinuousClock.now
    var wallNow = Date(timeIntervalSince1970: 1_793_016_000)
    func advance(_ seconds: Int) {
        now = now.advanced(by: .seconds(seconds))
        wallNow = wallNow.addingTimeInterval(Double(seconds))
    }
}
