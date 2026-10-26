import Foundation

public enum Code0SnapshotStoreError: Error, Equatable, Sendable {
    case persistenceFailed
    case invalidSnapshot
    case leaseRequired
    case unsafePath
}

/// The Code0 session owns this separate cache. A save must complete atomically
/// before the session publishes its candidate; removal never touches other services.
@MainActor
public protocol Code0SnapshotStoring: AnyObject {
    func load() throws -> UsageSnapshot?
    func save(_ snapshot: UsageSnapshot) throws
    func remove() throws
}

/// RED checkpoint: no filesystem access or successful persistence until the
/// isolated behavioral RED is accepted. Codec projects only Code0's two values.
@MainActor
public final class Code0SnapshotStore: Code0SnapshotStoring {
    private let profile: any Code0ProfileLeasing
    public init(profile: any Code0ProfileLeasing) { self.profile = profile }
    public func load() throws -> UsageSnapshot? { nil }
    public func save(_ snapshot: UsageSnapshot) throws { throw Code0SnapshotStoreError.persistenceFailed }
    public func remove() throws { throw Code0SnapshotStoreError.persistenceFailed }
}
