import Darwin
import Foundation
import Testing
@testable import AIMeterCore

@Suite("Code0 dedicated projected snapshot store", .serialized)
@MainActor
struct Code0SnapshotStoreTests {
    @Test func saveProjectsOnlyTwoNumbersAndNativeSuccessMetadata() throws {
        let fixture = try StoreFixture()
        defer { fixture.clean() }
        let input = storeSnapshot(balance: 2.5, used: 1.5, statusMessage: "synthetic-do-not-persist")
        var saved = false
        do { try fixture.store.save(input); saved = true } catch {}
        try #require(saved, "A held owned lease must allow an atomic Code0 save")
        let data = try Data(contentsOf: fixture.location.snapshotURL)
        let object = try #require(JSONSerialization.jsonObject(with: data) as? [String: Any])
        #expect(Set(object.keys) == ["schemaVersion", "balance", "cumulativeUsage", "fetchedAt", "sourceVersion"])
        #expect(object["schemaVersion"] as? Int == 1)
        #expect(object["balance"] as? Double == 2.5)
        #expect(object["cumulativeUsage"] as? Double == 1.5)
        #expect(object["fetchedAt"] as? Double == 1_793_016_000)
        #expect(object["sourceVersion"] as? String == "website")
        #expect(!String(decoding: data, as: UTF8.self).contains("synthetic-do-not-persist"))
        let storedValue = try fixture.store.load()
        let loaded = try #require(storedValue)
        #expect(loaded.provider == .code0)
        #expect(loaded.primaryMetric?.current == 2.5)
        #expect(loaded.secondaryMetric?.current == 1.5)
        #expect(loaded.fetchedAt == Date(timeIntervalSince1970: 1_793_016_000))
        #expect(loaded.statusMessage == nil)
        #expect(loaded.primaryMetric?.limit == nil)
        #expect(loaded.primaryMetric?.unit == .siteCredits)
        let attributes = try FileManager.default.attributesOfItem(atPath: fixture.location.snapshotURL.path)
        #expect((attributes[.posixPermissions] as? NSNumber)?.intValue == 0o600)
    }

    @Test func loadIsReadOnlyAndAcceptsACompleteZeroCacheWithoutTakingLease() throws {
        let fixture = try StoreFixture()
        defer { fixture.clean() }
        try fixture.write(#"{"schemaVersion":1,"balance":0,"cumulativeUsage":0,"fetchedAt":1793016000,"sourceVersion":"website"}"#)
        fixture.lease.isHeld = false
        let storedValue = try fixture.store.load()
        let loaded = try #require(storedValue, "Loading a prior committed cache must not authorize a browser")
        #expect(loaded.primaryMetric?.current == 0)
        #expect(loaded.secondaryMetric?.current == 0)
        #expect(!fixture.lease.isHeld)
    }

    @Test func writesAndRemovalRequireTheSharedOwnedLease() throws {
        let fixture = try StoreFixture()
        defer { fixture.clean() }
        fixture.lease.isHeld = false
        #expect(throws: Code0SnapshotStoreError.leaseRequired) { try fixture.store.save(storeSnapshot()) }
        #expect(throws: Code0SnapshotStoreError.leaseRequired) { try fixture.store.remove() }
        #expect(!FileManager.default.fileExists(atPath: fixture.location.snapshotURL.path))
    }

    @Test func malformedOrUnboundedCacheIsNotSilentlyAnEmptyStore() throws {
        let fixture = try StoreFixture()
        defer { fixture.clean() }
        for json in [
            #"{"schemaVersion":1,"balance":2.5,"fetchedAt":1793016000,"sourceVersion":"website"}"#,
            #"{"schemaVersion":1,"balance":true,"cumulativeUsage":1,"fetchedAt":1793016000,"sourceVersion":"website"}"#,
            #"{"schemaVersion":1,"balance":"2.5","cumulativeUsage":1,"fetchedAt":1793016000,"sourceVersion":"website"}"#,
            #"{"schemaVersion":1,"balance":-1,"cumulativeUsage":1,"fetchedAt":1793016000,"sourceVersion":"website"}"#,
            #"{"schemaVersion":1,"balance":1e999,"cumulativeUsage":1,"fetchedAt":1793016000,"sourceVersion":"website"}"#,
            #"{"schemaVersion":2,"balance":2,"cumulativeUsage":1,"fetchedAt":1793016000,"sourceVersion":"website"}"#,
            #"{"schemaVersion":1,"balance":2,"cumulativeUsage":1,"fetchedAt":1793016000,"sourceVersion":"website","identity":"do-not-cache"}"#,
            String(repeating: " ", count: 4097),
        ] {
            try fixture.write(json)
            #expect(throws: Code0SnapshotStoreError.invalidSnapshot) { try fixture.store.load() }
        }
    }

    @Test func invalidSnapshotsCannotReplacePreviouslyCommittedBytes() throws {
        let fixture = try StoreFixture()
        defer { fixture.clean() }
        let committed = #"{"schemaVersion":1,"balance":2.5,"cumulativeUsage":1.5,"fetchedAt":1793016000,"sourceVersion":"website"}"#
        try fixture.write(committed)
        for invalid in [
            storeSnapshot(provider: .claude), storeSnapshot(balance: -1), storeSnapshot(balance: .infinity),
            storeSnapshot(used: .nan), storeSnapshot(unit: .cny), storeSnapshot(limit: 100),
            storeSnapshot(kind: .localBudget),
        ] {
            #expect(throws: Code0SnapshotStoreError.invalidSnapshot) { try fixture.store.save(invalid) }
            #expect(try Data(contentsOf: fixture.location.snapshotURL) == Data(committed.utf8))
        }
    }

    @Test func saveFailureLeavesTheOldAtomicSnapshotAndRemovalLeavesOtherServices() throws {
        let fixture = try StoreFixture()
        defer { fixture.clean() }
        let old = #"{"schemaVersion":1,"balance":2.5,"cumulativeUsage":1.5,"fetchedAt":1793016000,"sourceVersion":"website"}"#
        try fixture.write(old)
        let sibling = fixture.root.appendingPathComponent("deepseek-sentinel")
        try Data("keep-other-service".utf8).write(to: sibling)
        try FileManager.default.setAttributes([.posixPermissions: 0o500], ofItemAtPath: fixture.location.directoryURL.path)
        var writeFailed = false
        do { try fixture.store.save(storeSnapshot(balance: 99)) } catch { writeFailed = true }
        #expect(writeFailed)
        #expect(try Data(contentsOf: fixture.location.snapshotURL) == Data(old.utf8))
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: fixture.location.directoryURL.path)
        var removed = false
        do { try fixture.store.remove(); removed = true } catch {}
        #expect(removed, "Held-lease Disconnect must remove only the dedicated snapshot")
        #expect(!FileManager.default.fileExists(atPath: fixture.location.snapshotURL.path))
        #expect(try Data(contentsOf: sibling) == Data("keep-other-service".utf8))
    }

    @Test func directorySyncFailureAfterRenameCannotRestoreCandidateAsSuccessfulCache() throws {
        let fixture = try StoreFixture()
        defer { fixture.clean() }
        let old = storeSnapshot(balance: 2.5)
        try fixture.store.save(old)
        let candidate = UsageSnapshot(provider: .code0,
            primaryMetric: UsageMetric(label: "Balance", current: 99, limit: nil, unit: .siteCredits, kind: .balance),
            secondaryMetric: UsageMetric(label: "Cumulative usage", current: 2, limit: nil, unit: .siteCredits, kind: .cumulativeUsage),
            fetchedAt: old.fetchedAt.addingTimeInterval(300), sourceVersion: "website")
        var directorySyncCalls = 0
        let failingStore = Code0SnapshotStore(profile: fixture.lease, synchronizeDirectory: { descriptor in
            directorySyncCalls += 1
            if directorySyncCalls == 1 { errno = EIO; return -1 }
            return Darwin.fsync(descriptor)
        })
        #expect(throws: Code0SnapshotStoreError.persistenceFailed) { try failingStore.save(candidate) }
        #expect(directorySyncCalls >= 1, "The real post-rename directory sync must reach the one-shot EIO seam")
        // A fresh store represents restart recovery, with no in-memory knowledge
        // of the failed save. Throwing or no cache is safe; a success must be A/T.
        let restartedStore = Code0SnapshotStore(profile: fixture.lease)
        do {
            if let restored = try restartedStore.load() {
                #expect(restored.primaryMetric?.current == old.primaryMetric?.current)
                #expect(restored.fetchedAt == old.fetchedAt)
                #expect(restored.fetchedAt != candidate.fetchedAt)
            }
        } catch { /* An explicitly unreadable/uncertain cache is not success B/T+300. */ }
    }

    @Test func absentReadDoesNotCreateOwnedDirectoriesAndProjectionRejectsResetFields() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("Code0Absent-\(UUID())")
        defer { try? FileManager.default.removeItem(at: root) }
        let location = try Code0StorageLocation(applicationSupportRoot: root)
        let profile = StoreTestLease(location: location)
        profile.isHeld = false
        let store = Code0SnapshotStore(profile: profile)
        #expect(try store.load() == nil)
        #expect(!FileManager.default.fileExists(atPath: root.path))
        let fixture = try StoreFixture()
        defer { fixture.clean() }
        let input = UsageSnapshot(provider: .code0,
            primaryMetric: UsageMetric(label: "Balance", current: 1, limit: nil, unit: .siteCredits,
                kind: .balance, resetAt: Date(timeIntervalSince1970: 1_793_016_000)),
            secondaryMetric: UsageMetric(label: "Cumulative usage", current: 0, limit: nil, unit: .siteCredits, kind: .cumulativeUsage),
            sourceVersion: "website")
        #expect(throws: Code0SnapshotStoreError.invalidSnapshot) { try fixture.store.save(input) }
        #expect(!FileManager.default.fileExists(atPath: fixture.location.snapshotURL.path))
    }

    @Test func symbolicLinkCacheIsRejectedForReadWriteAndRemoveWithoutTouchingTarget() throws {
        let fixture = try StoreFixture()
        defer { fixture.clean() }
        let target = fixture.root.appendingPathComponent("external-target")
        try Data("keep-target".utf8).write(to: target)
        try FileManager.default.createSymbolicLink(at: fixture.location.snapshotURL, withDestinationURL: target)
        #expect(throws: Code0SnapshotStoreError.unsafePath) { try fixture.store.load() }
        #expect(throws: Code0SnapshotStoreError.unsafePath) { try fixture.store.save(storeSnapshot()) }
        #expect(throws: Code0SnapshotStoreError.unsafePath) { try fixture.store.remove() }
        #expect(try Data(contentsOf: target) == Data("keep-target".utf8))
    }
}

@MainActor
private final class StoreTestLease: Code0ProfileLeasing {
    let location: Code0StorageLocation
    var isHeld = true
    init(location: Code0StorageLocation) { self.location = location }
    func acquire() throws { isHeld = true }
    func release() { isHeld = false }
}

@MainActor
private struct StoreFixture {
    let root: URL
    let location: Code0StorageLocation
    let lease: StoreTestLease
    let store: Code0SnapshotStore
    init() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("Code0Store-\(UUID())")
        let location = try Code0StorageLocation(applicationSupportRoot: root)
        try FileManager.default.createDirectory(at: location.directoryURL, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        self.root = root; self.location = location
        let lease = StoreTestLease(location: location)
        self.lease = lease; self.store = Code0SnapshotStore(profile: lease)
    }
    func write(_ text: String) throws {
        try Data(text.utf8).write(to: location.snapshotURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: location.snapshotURL.path)
    }
    func clean() {
        try? FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: location.directoryURL.path)
        try? FileManager.default.removeItem(at: root)
    }
}

private func storeSnapshot(provider: UsageProvider = .code0, balance: Double = 2.5, used: Double = 1.5,
                           unit: UsageUnit = .siteCredits, kind: UsageMetricKind = .balance,
                           limit: Double? = nil, statusMessage: String? = nil) -> UsageSnapshot {
    UsageSnapshot(provider: provider,
        primaryMetric: UsageMetric(label: "Balance", current: balance, limit: limit, unit: unit, kind: kind),
        secondaryMetric: UsageMetric(label: "Cumulative usage", current: used, limit: nil, unit: .siteCredits, kind: .cumulativeUsage),
        fetchedAt: Date(timeIntervalSince1970: 1_793_016_000), sourceVersion: "website", statusMessage: statusMessage)
}
