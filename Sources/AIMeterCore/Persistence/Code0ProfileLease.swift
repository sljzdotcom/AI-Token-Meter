import Foundation

/// All paths are native-derived beneath one Code0-owned subtree. Tests may
/// supply their own temporary application-support anchor, never a website origin.
public struct Code0StorageLocation: Equatable, Sendable {
    public let directoryURL: URL
    public var snapshotURL: URL { directoryURL.appendingPathComponent("snapshot.json") }
    public var lockURL: URL { directoryURL.appendingPathComponent("profile.lock") }
    public static func applicationSupport() throws -> Self {
        guard let root = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first else {
            throw Code0ProfileLeaseError.unavailable
        }
        return Self(applicationSupportRoot: root)
    }
    public init(applicationSupportRoot: URL) {
        directoryURL = applicationSupportRoot.appendingPathComponent("com.millerpan.aitokenmeter", isDirectory: true)
            .appendingPathComponent("Code0", isDirectory: true)
    }
}

public enum Code0ProfileLeaseError: Error, Equatable, Sendable {
    case busy, unsafePath, unavailable
}

@MainActor
public protocol Code0ProfileLeasing: AnyObject {
    var location: Code0StorageLocation { get }
    var isHeld: Bool { get }
    func acquire() throws
    func release()
}

/// RED checkpoint: construction never creates paths or acquires an OS lock.
@MainActor
public final class Code0ProfileLease: Code0ProfileLeasing {
    public let location: Code0StorageLocation
    public var isHeld: Bool { false }
    public init(location: Code0StorageLocation) { self.location = location }
    public func acquire() throws { throw Code0ProfileLeaseError.busy }
    public func release() {}
}
