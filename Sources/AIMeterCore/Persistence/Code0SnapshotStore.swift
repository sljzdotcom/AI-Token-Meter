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
    private static let maximumBytes = 4096
    public init(profile: any Code0ProfileLeasing) { self.profile = profile }
    public func load() throws -> UsageSnapshot? {
        let directory: Int32
        do { directory = try Code0SafeFiles.openDirectory(profile.location, create: false) }
        catch Code0SafeFiles.Failure.absent { return nil }
        catch { throw map(error) }
        defer { _ = Darwin.close(directory) }
        let fd = openat(directory, "snapshot.json", O_RDONLY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC)
        guard fd >= 0 else {
            if errno == ENOENT { return nil }
            throw errno == ELOOP ? Code0SnapshotStoreError.unsafePath : .persistenceFailed
        }
        defer { _ = Darwin.close(fd) }
        do { _ = try Code0SafeFiles.validateRegular(fd, maximumSize: nil) } catch { throw map(error) }
        var bytes = [UInt8](repeating: 0, count: Self.maximumBytes + 1)
        var count = 0
        while count < bytes.count {
            let amount = bytes.withUnsafeMutableBytes { buffer in
                Darwin.read(fd, buffer.baseAddress!.advanced(by: count), buffer.count - count)
            }
            if amount < 0 { if errno == EINTR { continue }; throw Code0SnapshotStoreError.persistenceFailed }
            if amount == 0 { break }
            count += amount
        }
        guard count > 0, count <= Self.maximumBytes,
              Code0SafeFiles.matches(directory, name: "snapshot.json", descriptor: fd) else {
            throw Code0SnapshotStoreError.invalidSnapshot
        }
        return try decode(Data(bytes.prefix(count)))
    }
    public func save(_ snapshot: UsageSnapshot) throws {
        guard profile.isHeld else { throw Code0SnapshotStoreError.leaseRequired }
        let data = try encode(snapshot)
        let directory: Int32
        do { directory = try Code0SafeFiles.openDirectory(profile.location, create: false) }
        catch { throw map(error) }
        defer { _ = Darwin.close(directory) }
        do { try Code0SafeFiles.validateExisting(directory, name: "snapshot.json") } catch { throw map(error) }
        let name = ".snapshot-\(UUID().uuidString).tmp"
        let fd = openat(directory, name, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, mode_t(0o600))
        guard fd >= 0 else { throw Code0SnapshotStoreError.persistenceFailed }
        defer { _ = Darwin.close(fd); _ = unlinkat(directory, name, 0) }
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
        do { try Code0SafeFiles.validateExisting(directory, name: "snapshot.json") } catch { throw map(error) }
        guard renameat(directory, name, directory, "snapshot.json") == 0 else { throw Code0SnapshotStoreError.persistenceFailed }
        // A directory-sync failure is an uncertain persistence outcome, not a
        // successful save. The session retains its cache-cleanup obligation.
        guard fsync(directory) == 0 else { throw Code0SnapshotStoreError.persistenceFailed }
    }
    public func remove() throws {
        guard profile.isHeld else { throw Code0SnapshotStoreError.leaseRequired }
        let directory: Int32
        do { directory = try Code0SafeFiles.openDirectory(profile.location, create: false) }
        catch Code0SafeFiles.Failure.absent { return }
        catch { throw map(error) }
        defer { _ = Darwin.close(directory) }
        do { try Code0SafeFiles.validateExisting(directory, name: "snapshot.json") } catch { throw map(error) }
        guard unlinkat(directory, "snapshot.json", 0) == 0 || errno == ENOENT else { throw Code0SnapshotStoreError.persistenceFailed }
        guard fsync(directory) == 0 else { throw Code0SnapshotStoreError.persistenceFailed }
    }
    private func map(_ error: Error) -> Code0SnapshotStoreError {
        if case Code0SafeFiles.Failure.unsafe = error { return .unsafePath }
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
