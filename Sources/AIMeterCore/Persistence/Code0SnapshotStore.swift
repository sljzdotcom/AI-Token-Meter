import Foundation

public enum Code0SnapshotStoreError: Error, Equatable, Sendable {
    case persistenceFailed
    case invalidSnapshot
}

/// The Code0 session owns this separate cache. A save must complete atomically
/// before the session publishes its candidate; removal never touches other services.
@MainActor
public protocol Code0SnapshotStoring: AnyObject {
    func load() throws -> UsageSnapshot?
    func save(_ snapshot: UsageSnapshot) throws
    func remove() throws
}
