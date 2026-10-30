import CoreFoundation
import Darwin
import Foundation

public enum Code0SnapshotStoreError: Error, Equatable, Sendable {
    case persistenceFailed, invalidSnapshot, leaseRequired, unsafePath
}

@MainActor
public protocol Code0SnapshotStoring: AnyObject {
    func load() throws -> UsageSnapshot?
    func save(_ snapshot: UsageSnapshot) throws
    func remove() throws
}

/// Persists only a bounded, versioned numeric projection, never browser/account
/// objects. Every mutation requires the same lease held by the session driver.
@MainActor
public final class Code0SnapshotStore: Code0SnapshotStoring {
    private let profile: any Code0ProfileLeasing
    private let synchronizeDirectory: (Int32) -> Int32
    private static let maximumBytes = 4096
    public init(profile: any Code0ProfileLeasing) {
        self.profile = profile
        self.synchronizeDirectory = { Darwin.fsync($0) }
    }
    /// Fault injection only for the candidate post-rename directory sync.
    init(profile: any Code0ProfileLeasing, synchronizeDirectory: @escaping (Int32) -> Int32) {
        self.profile = profile
        self.synchronizeDirectory = synchronizeDirectory
    }
    private static let pendingBytes = Data("code0-pending-v1\n".utf8)
    public func load() throws -> UsageSnapshot? {
        let directory: Int32
        do { directory = try Code0SafeFiles.openDirectory(profile.location, create: false) }
        catch Code0SafeFiles.Failure.absent { return nil }
        catch { throw map(error) }
        defer { _ = Darwin.close(directory) }
        guard try !pendingExists(directory) else { throw Code0SnapshotStoreError.persistenceFailed }
        let fd = openat(directory, "snapshot.json", O_RDONLY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC)
        guard fd >= 0 else {
            if errno == ENOENT { return nil }
            throw errno == ELOOP ? Code0SnapshotStoreError.unsafePath : .persistenceFailed
        }
        defer { _ = Darwin.close(fd) }
        let data = try readDescriptor(fd, directory: directory, name: "snapshot.json", limit: Self.maximumBytes)
        guard try !pendingExists(directory), Code0SafeFiles.matches(directory, name: "snapshot.json", descriptor: fd) else {
            throw Code0SnapshotStoreError.persistenceFailed
        }
        return try decode(data)
    }
    public func save(_ snapshot: UsageSnapshot) throws {
        try requireAuthority()
        let data = try encode(snapshot)
        let directory: Int32
        do { directory = try profile.duplicateHeldDirectory() } catch { throw map(error) }
        defer { _ = Darwin.close(directory) }
        guard try !pendingExists(directory) else { throw Code0SnapshotStoreError.persistenceFailed }
        let old = try readFile(directory, name: "snapshot.json", limit: Self.maximumBytes)
        if let old { _ = try decode(old) }
        var replaced = false
        do {
            try establishPending(directory)
            try replaceSnapshot(data, directory: directory)
            replaced = true
            // This fault seam stays AFTER candidate rename, never on a new
            // pre-commit marker sync. A failed candidate cannot bypass pending.
            guard synchronizeDirectory(directory) == 0 else { throw Code0SnapshotStoreError.persistenceFailed }
            try requireAuthority()
            try retirePending(directory)
            // Atomic barrier retirement is the visibility commit point. No
            // fallible save step follows it. A crash may resurrect pending and
            // conservatively hide a durable success, never bless a failed one.
        } catch {
            if replaced {
                // Exactly one bounded restoration attempt. On any second error
                // the durable pending barrier remains authoritative for load.
                try? restore(old, directory: directory)
            }
            throw map(error)
        }
    }
    public func remove() throws {
        try requireAuthority()
        let directory: Int32
        do { directory = try profile.duplicateHeldDirectory() } catch { throw map(error) }
        defer { _ = Darwin.close(directory) }
        let pending = try fileExists(directory, name: "snapshot.pending")
        let existing = try fileExists(directory, name: "snapshot.json")
        if !pending, !existing { return }
        if !pending { try establishPending(directory) }
        try requireAuthority()
        guard unlinkat(directory, "snapshot.json", 0) == 0 || errno == ENOENT else { throw Code0SnapshotStoreError.persistenceFailed }
        guard fsync(directory) == 0 else { throw Code0SnapshotStoreError.persistenceFailed }
        try requireAuthority()
        try retirePending(directory)
    }
    private func requireAuthority() throws {
        guard profile.isHeld else { throw Code0SnapshotStoreError.leaseRequired }
    }
    private func readFile(_ directory: Int32, name: String, limit: Int) throws -> Data? {
        let fd = openat(directory, name, O_RDONLY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC)
        guard fd >= 0 else {
            if errno == ENOENT { return nil }
            throw errno == ELOOP ? Code0SnapshotStoreError.unsafePath : .persistenceFailed
        }
        defer { _ = Darwin.close(fd) }
        return try readDescriptor(fd, directory: directory, name: name, limit: limit)
    }
    private func readDescriptor(_ fd: Int32, directory: Int32, name: String, limit: Int) throws -> Data {
        do { _ = try Code0SafeFiles.validateRegular(fd, maximumSize: nil) } catch { throw map(error) }
        var bytes = [UInt8](repeating: 0, count: limit + 1)
        var count = 0
        while count < bytes.count {
            let amount = bytes.withUnsafeMutableBytes { buffer in
                Darwin.read(fd, buffer.baseAddress!.advanced(by: count), buffer.count - count)
            }
            if amount < 0 { if errno == EINTR { continue }; throw Code0SnapshotStoreError.persistenceFailed }
            if amount == 0 { break }
            count += amount
        }
        guard count > 0, count <= limit else { throw Code0SnapshotStoreError.invalidSnapshot }
        guard Code0SafeFiles.matches(directory, name: name, descriptor: fd) else { throw Code0SnapshotStoreError.unsafePath }
        return Data(bytes.prefix(count))
    }
    private func pendingExists(_ directory: Int32) throws -> Bool {
        guard let bytes = try readFile(directory, name: "snapshot.pending", limit: 64) else { return false }
        guard bytes == Self.pendingBytes else { throw Code0SnapshotStoreError.persistenceFailed }
        return true
    }
    private func writeExclusive(_ data: Data, directory: Int32, name: String) throws {
        let fd = openat(directory, name, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, mode_t(0o600))
        guard fd >= 0 else { throw Code0SnapshotStoreError.persistenceFailed }
        defer { _ = Darwin.close(fd) }
        var offset = 0
        try data.withUnsafeBytes { buffer in
            while offset < buffer.count {
                let amount = Darwin.write(fd, buffer.baseAddress!.advanced(by: offset), buffer.count - offset)
                if amount < 0, errno == EINTR { continue }
                guard amount > 0 else { throw Code0SnapshotStoreError.persistenceFailed }
                offset += amount
            }
        }
        guard fsync(fd) == 0 else { throw Code0SnapshotStoreError.persistenceFailed }
    }
    private func establishPending(_ directory: Int32) throws {
        try requireAuthority()
        try writeExclusive(Self.pendingBytes, directory: directory, name: "snapshot.pending")
        guard fsync(directory) == 0 else { throw Code0SnapshotStoreError.persistenceFailed }
    }
    private func replaceSnapshot(_ data: Data, directory: Int32) throws {
        let name = ".snapshot-\(UUID().uuidString).tmp"
        defer { _ = unlinkat(directory, name, 0) }
        try writeExclusive(data, directory: directory, name: name)
        do { try Code0SafeFiles.validateExisting(directory, name: "snapshot.json") } catch { throw map(error) }
        try requireAuthority()
        guard renameat(directory, name, directory, "snapshot.json") == 0 else { throw Code0SnapshotStoreError.persistenceFailed }
    }
    private func restore(_ old: Data?, directory: Int32) throws {
        try requireAuthority()
        if let old { try replaceSnapshot(old, directory: directory) }
        else {
            guard unlinkat(directory, "snapshot.json", 0) == 0 || errno == ENOENT else { throw Code0SnapshotStoreError.persistenceFailed }
        }
        guard fsync(directory) == 0 else { throw Code0SnapshotStoreError.persistenceFailed }
        try requireAuthority()
        try retirePending(directory)
    }
    private func fileExists(_ directory: Int32, name: String) throws -> Bool {
        let fd = openat(directory, name, O_RDONLY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC)
        guard fd >= 0 else {
            if errno == ENOENT { return false }
            throw errno == ELOOP ? Code0SnapshotStoreError.unsafePath : .persistenceFailed
        }
        defer { _ = Darwin.close(fd) }
        do { _ = try Code0SafeFiles.validateRegular(fd, maximumSize: nil) } catch { throw map(error) }
        guard Code0SafeFiles.matches(directory, name: name, descriptor: fd) else { throw Code0SnapshotStoreError.unsafePath }
        return true
    }
    private func retirePending(_ directory: Int32) throws {
        guard try fileExists(directory, name: "snapshot.pending") else { throw Code0SnapshotStoreError.persistenceFailed }
        guard unlinkat(directory, "snapshot.pending", 0) == 0 else { throw Code0SnapshotStoreError.persistenceFailed }
    }
    private func map(_ error: Error) -> Code0SnapshotStoreError {
        if let error = error as? Code0SnapshotStoreError { return error }
        if case Code0SafeFiles.Failure.unsafe = error { return .unsafePath }
        if case Code0ProfileLeaseError.unsafePath = error { return .leaseRequired }
        return .persistenceFailed
    }
    private func encode(_ snapshot: UsageSnapshot) throws -> Data {
        guard snapshot.provider == .code0, snapshot.sourceVersion == "website",
              let balance = snapshot.primaryMetric, let used = snapshot.secondaryMetric,
              balance.unit == .siteCredits, used.unit == .siteCredits,
              balance.kind == .balance, used.kind == .cumulativeUsage,
              balance.limit == nil, used.limit == nil,
              balance.resetAt == nil, used.resetAt == nil, balance.resetDescription == nil, used.resetDescription == nil,
              valid(balance.current), valid(used.current), valid(snapshot.fetchedAt.timeIntervalSince1970) else {
            throw Code0SnapshotStoreError.invalidSnapshot
        }
        let data = try JSONSerialization.data(withJSONObject: ["schemaVersion": 1, "balance": balance.current,
            "cumulativeUsage": used.current, "fetchedAt": snapshot.fetchedAt.timeIntervalSince1970,
            "sourceVersion": "website"], options: [.sortedKeys])
        guard data.count <= Self.maximumBytes else { throw Code0SnapshotStoreError.invalidSnapshot }
        return data
    }
    private func decode(_ data: Data) throws -> UsageSnapshot {
        guard let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              Set(object.keys) == ["schemaVersion", "balance", "cumulativeUsage", "fetchedAt", "sourceVersion"],
              number(object["schemaVersion"]) == 1, object["sourceVersion"] as? String == "website",
              let balance = number(object["balance"]), let used = number(object["cumulativeUsage"]),
              let fetched = number(object["fetchedAt"]) else { throw Code0SnapshotStoreError.invalidSnapshot }
        return UsageSnapshot(provider: .code0,
            primaryMetric: UsageMetric(label: "Balance", current: balance, limit: nil, unit: .siteCredits, kind: .balance),
            secondaryMetric: UsageMetric(label: "Cumulative usage", current: used, limit: nil, unit: .siteCredits, kind: .cumulativeUsage),
            fetchedAt: Date(timeIntervalSince1970: fetched), sourceVersion: "website")
    }
    private func number(_ value: Any?) -> Double? {
        guard let value = value as? NSNumber, CFGetTypeID(value) != CFBooleanGetTypeID(), valid(value.doubleValue) else { return nil }
        return value.doubleValue
    }
    private func valid(_ value: Double) -> Bool { value.isFinite && value >= 0 }
}
