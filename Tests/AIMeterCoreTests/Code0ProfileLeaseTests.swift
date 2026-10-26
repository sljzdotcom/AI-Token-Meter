import Darwin
import Foundation
import Testing
@testable import AIMeterCore

@Suite("Code0 owned OS profile lease", .serialized)
@MainActor
struct Code0ProfileLeaseTests {
    @Test func missingAnchorUsesVerifiedPhysicalAncestorWithoutCreatingDirectories() throws {
        let root = temporaryLeaseRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let ancestor = root.deletingLastPathComponent()
        let physical = try #require(realpath(ancestor.path, nil))
        let physicalPath = String(cString: physical)
        free(physical)
        let location = try Code0StorageLocation(applicationSupportRoot: root)
        let expected = URL(fileURLWithPath: physicalPath, isDirectory: true)
            .appendingPathComponent(root.lastPathComponent, isDirectory: true)
            .appendingPathComponent("com.millerpan.aitokenmeter", isDirectory: true)
            .appendingPathComponent("Code0", isDirectory: true)
        #expect(location.directoryURL.path == expected.path)
        #expect(!FileManager.default.fileExists(atPath: root.path))
        let regular = FileManager.default.temporaryDirectory.appendingPathComponent("Code0AnchorFile-\(UUID())")
        defer { try? FileManager.default.removeItem(at: regular) }
        try Data("file-not-directory".utf8).write(to: regular)
        #expect(throws: Code0ProfileLeaseError.unsafePath) {
            _ = try Code0StorageLocation(applicationSupportRoot: regular.appendingPathComponent("missing"))
        }
    }

    @Test func independentLeaseHandlesContendAndReleaseWithoutUnlinkingLock() throws {
        let root = temporaryLeaseRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let location = try Code0StorageLocation(applicationSupportRoot: root)
        let first = Code0ProfileLease(location: location)
        let second = Code0ProfileLease(location: location)
        defer { first.release(); second.release() }
        var acquired = false
        do { try first.acquire(); acquired = true } catch {}
        try #require(acquired, "A safe owned profile must acquire its real non-blocking flock")
        #expect(first.isHeld)
        #expect(throws: Code0ProfileLeaseError.busy) { try second.acquire() }
        #expect(!second.isHeld)
        var before = stat()
        #expect(lstat(location.lockURL.path, &before) == 0)
        #expect(before.st_uid == getuid())
        #expect(before.st_nlink == 1)
        #expect(before.st_mode & 0o777 == 0o600)
        first.release()
        first.release()
        var after = stat()
        #expect(lstat(location.lockURL.path, &after) == 0)
        #expect(before.st_ino == after.st_ino)
        #expect(before.st_dev == after.st_dev)
        var secondAcquired = false
        do { try second.acquire(); secondAcquired = true } catch {}
        #expect(secondAcquired)
        #expect(second.isHeld)
    }

    @Test(arguments: [false, true])
    func replacementWhileHeldRevokesOldAuthorityAndCannotTouchReplacement(replaceDirectory: Bool) throws {
        let root = temporaryLeaseRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let location = try Code0StorageLocation(applicationSupportRoot: root)
        let oldLease = Code0ProfileLease(location: location)
        let newLease = Code0ProfileLease(location: location)
        defer { oldLease.release(); newLease.release() }
        try oldLease.acquire()
        let oldStore = Code0SnapshotStore(profile: oldLease)
        let retained = root.appendingPathComponent(replaceDirectory ? "retained-directory" : "retained-lock")
        if replaceDirectory {
            try FileManager.default.moveItem(at: location.directoryURL, to: retained)
            try FileManager.default.createDirectory(at: location.directoryURL, withIntermediateDirectories: false,
                attributes: [.posixPermissions: 0o700])
        } else {
            try FileManager.default.moveItem(at: location.lockURL, to: retained)
        }
        try newLease.acquire()
        #expect(newLease.isHeld)
        #expect(!oldLease.isHeld, "The old locked inode is no longer authority for this fixed profile path")
        var reacquireFailed = false
        do { try oldLease.acquire() } catch { reacquireFailed = true }
        #expect(reacquireFailed, "Re-acquire must not silently accept an already-invalid old lock")
        #expect(!(oldLease.isHeld && newLease.isHeld))
        let sentinel = Data("replacement-owned-by-new-lease".utf8)
        try sentinel.write(to: location.snapshotURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: location.snapshotURL.path)
        let candidate = UsageSnapshot(provider: .code0,
            primaryMetric: UsageMetric(label: "Balance", current: 9, limit: nil, unit: .siteCredits, kind: .balance),
            secondaryMetric: UsageMetric(label: "Cumulative usage", current: 1, limit: nil, unit: .siteCredits, kind: .cumulativeUsage),
            fetchedAt: Date(timeIntervalSince1970: 1_793_016_000), sourceVersion: "website")
        var saveFailed = false
        do { try oldStore.save(candidate) } catch { saveFailed = true }
        #expect(saveFailed)
        #expect(try Data(contentsOf: location.snapshotURL) == sentinel)
        // Reset only the test's sentinel so the two mutation oracles are independent.
        try sentinel.write(to: location.snapshotURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: location.snapshotURL.path)
        var removalFailed = false
        do { try oldStore.remove() } catch { removalFailed = true }
        #expect(removalFailed)
        #expect(FileManager.default.fileExists(atPath: location.snapshotURL.path))
        if FileManager.default.fileExists(atPath: location.snapshotURL.path) {
            #expect(try Data(contentsOf: location.snapshotURL) == sentinel)
        }
        oldLease.release()
        #expect(newLease.isHeld, "Releasing the stale inode must not release its successor")
    }

    @Test func existingKernelFlockIsBusyWithoutDeletingOrChangingTheOwnerFile() throws {
        let root = temporaryLeaseRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let location = try Code0StorageLocation(applicationSupportRoot: root)
        try FileManager.default.createDirectory(at: location.directoryURL, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        let descriptor = open(location.lockURL.path, O_CREAT | O_EXCL | O_RDWR | O_NOFOLLOW, mode_t(0o600))
        try #require(descriptor >= 0)
        defer { _ = flock(descriptor, LOCK_UN); _ = close(descriptor) }
        try #require(flock(descriptor, LOCK_EX | LOCK_NB) == 0)
        let owned = Data("owned-lock-sentinel".utf8)
        try owned.write(to: location.lockURL)
        let lease = Code0ProfileLease(location: location)
        defer { lease.release() }
        #expect(throws: Code0ProfileLeaseError.busy) { try lease.acquire() }
        #expect(!lease.isHeld)
        #expect(try Data(contentsOf: location.lockURL) == owned)
    }

    @Test func symbolicLinkDirectoryAndLockAreRejectedWithoutTouchingTargets() throws {
        for linkDirectory in [true, false] {
            let root = temporaryLeaseRoot()
            defer { try? FileManager.default.removeItem(at: root) }
            let location = try Code0StorageLocation(applicationSupportRoot: root)
            let target = root.appendingPathComponent("external")
            try FileManager.default.createDirectory(at: target, withIntermediateDirectories: true)
            if linkDirectory {
                try FileManager.default.createDirectory(at: location.directoryURL.deletingLastPathComponent(), withIntermediateDirectories: true)
                try FileManager.default.createSymbolicLink(at: location.directoryURL, withDestinationURL: target)
            } else {
                try FileManager.default.createDirectory(at: location.directoryURL, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
                let file = target.appendingPathComponent("lock")
                try Data("do-not-change".utf8).write(to: file)
                try FileManager.default.createSymbolicLink(at: location.lockURL, withDestinationURL: file)
            }
            let lease = Code0ProfileLease(location: location)
            defer { lease.release() }
            #expect(throws: Code0ProfileLeaseError.unsafePath) { try lease.acquire() }
            #expect(!lease.isHeld)
            if !linkDirectory { #expect(try Data(contentsOf: target.appendingPathComponent("lock")) == Data("do-not-change".utf8)) }
        }
    }

    @Test func nonRegularOrMultiplyLinkedLockCannotBecomeAProfileLease() throws {
        for hardLink in [false, true] {
            let root = temporaryLeaseRoot()
            defer { try? FileManager.default.removeItem(at: root) }
            let location = try Code0StorageLocation(applicationSupportRoot: root)
            try FileManager.default.createDirectory(at: location.directoryURL, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
            if hardLink {
                let external = root.appendingPathComponent("linked-file")
                try Data("keep".utf8).write(to: external)
                try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: external.path)
                try FileManager.default.linkItem(at: external, to: location.lockURL)
            } else { try FileManager.default.createDirectory(at: location.lockURL, withIntermediateDirectories: false) }
            let lease = Code0ProfileLease(location: location)
            #expect(throws: Code0ProfileLeaseError.unsafePath) { try lease.acquire() }
            #expect(!lease.isHeld)
        }
    }
}

private func temporaryLeaseRoot() -> URL {
    FileManager.default.temporaryDirectory.appendingPathComponent("Code0Lease-\(UUID())")
}
