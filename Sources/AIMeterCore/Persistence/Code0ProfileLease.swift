import Darwin
import Foundation

/// Canonicalizes only the trusted native anchor. Owned descendants are always
/// opened without symlinks, including when the anchor was a system alias.
public struct Code0StorageLocation: Equatable, Sendable {
    public let directoryURL: URL
    public var snapshotURL: URL { directoryURL.appendingPathComponent("snapshot.json") }
    public var lockURL: URL { directoryURL.appendingPathComponent("profile.lock") }
    public static func applicationSupport() throws -> Self {
        guard let root = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first else {
            throw Code0ProfileLeaseError.unavailable
        }
        return try Self(applicationSupportRoot: root)
    }
    public init(applicationSupportRoot: URL) throws {
        guard applicationSupportRoot.isFileURL, applicationSupportRoot.path.hasPrefix("/"),
              !applicationSupportRoot.path.utf8.contains(0) else { throw Code0ProfileLeaseError.unsafePath }
        var ancestor = applicationSupportRoot.standardizedFileURL
        var missing: [String] = []
        while true {
            if let resolved = realpath(ancestor.path, nil) {
                let physicalPath = String(cString: resolved)
                free(resolved)
                var info = stat()
                guard lstat(physicalPath, &info) == 0,
                      info.st_mode & mode_t(S_IFMT) == mode_t(S_IFDIR) else { throw Code0ProfileLeaseError.unsafePath }
                var anchor = URL(fileURLWithPath: physicalPath, isDirectory: true)
                for component in missing.reversed() { anchor.appendPathComponent(component, isDirectory: true) }
                directoryURL = anchor.appendingPathComponent("com.millerpan.aitokenmeter", isDirectory: true)
                    .appendingPathComponent("Code0", isDirectory: true)
                return
            }
            // Only missing suffixes may be appended to a verified physical
            // ancestor. Permission, non-directory and all other errors fail closed.
            guard errno == ENOENT else { throw Code0ProfileLeaseError.unsafePath }
            let parent = ancestor.deletingLastPathComponent()
            let component = ancestor.lastPathComponent
            guard parent.path != ancestor.path, !component.isEmpty,
                  component != ".", component != ".." else { throw Code0ProfileLeaseError.unsafePath }
            missing.append(component)
            ancestor = parent
        }
    }
}

public enum Code0ProfileLeaseError: Error, Equatable, Sendable { case busy, unsafePath, unavailable }

@MainActor
public protocol Code0ProfileLeasing: AnyObject {
    var location: Code0StorageLocation { get }
    var isHeld: Bool { get }
    func acquire() throws
    func release()
    /// Caller owns the returned descriptor and must close it. Real OS leases
    /// return a duplicate of the directory bound to their locked inode.
    func duplicateHeldDirectory() throws -> Int32
}

public extension Code0ProfileLeasing {
    func duplicateHeldDirectory() throws -> Int32 {
        guard isHeld else { throw Code0ProfileLeaseError.unsafePath }
        return try Code0SafeFiles.openDirectory(location, create: false)
    }
}

@MainActor
public final class Code0ProfileLease: Code0ProfileLeasing {
    public let location: Code0StorageLocation
    private var descriptor: Int32 = -1
    private var directoryDescriptor: Int32 = -1
    private var authorityLost = false
    public var isHeld: Bool {
        guard descriptor >= 0, directoryDescriptor >= 0, !authorityLost else { return false }
        guard let current = try? Code0SafeFiles.openDirectory(location, create: false) else {
            authorityLost = true; return false
        }
        defer { _ = Darwin.close(current) }
        var bound = stat(), path = stat()
        let valid = fstat(directoryDescriptor, &bound) == 0 && fstat(current, &path) == 0 &&
            bound.st_dev == path.st_dev && bound.st_ino == path.st_ino &&
            (try? Code0SafeFiles.validateRegular(descriptor, maximumSize: nil)) != nil &&
            Code0SafeFiles.matches(current, name: "profile.lock", descriptor: descriptor)
        if !valid { authorityLost = true }
        return valid
    }
    public init(location: Code0StorageLocation) { self.location = location }
    public func acquire() throws {
        if descriptor >= 0 {
            guard isHeld else { throw Code0ProfileLeaseError.unsafePath }
            return
        }
        guard !authorityLost else { throw Code0ProfileLeaseError.unsafePath }
        let directory: Int32
        do { directory = try Code0SafeFiles.openDirectory(location, create: true) }
        catch Code0SafeFiles.Failure.unsafe { throw Code0ProfileLeaseError.unsafePath }
        catch { throw Code0ProfileLeaseError.unavailable }
        var keepDirectory = false
        defer { if !keepDirectory { _ = Darwin.close(directory) } }
        let fd = openat(directory, "profile.lock", O_RDWR | O_CREAT | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK, mode_t(0o600))
        guard fd >= 0 else {
            throw errno == ELOOP || errno == EISDIR ? Code0ProfileLeaseError.unsafePath : .unavailable
        }
        var keep = false
        defer { if !keep { _ = Darwin.close(fd) } }
        do { _ = try Code0SafeFiles.validateRegular(fd, maximumSize: nil) }
        catch { throw Code0ProfileLeaseError.unsafePath }
        guard flock(fd, LOCK_EX | LOCK_NB) == 0 else {
            throw errno == EWOULDBLOCK || errno == EAGAIN ? Code0ProfileLeaseError.busy : .unavailable
        }
        // Verify the pathname still denotes the locked inode. Never unlink a lock.
        guard Code0SafeFiles.matches(directory, name: "profile.lock", descriptor: fd) else {
            _ = flock(fd, LOCK_UN)
            throw Code0ProfileLeaseError.unsafePath
        }
        descriptor = fd
        directoryDescriptor = directory
        keep = true; keepDirectory = true
        guard isHeld else {
            release()
            throw Code0ProfileLeaseError.unsafePath
        }
    }
    public func duplicateHeldDirectory() throws -> Int32 {
        guard isHeld else { throw Code0ProfileLeaseError.unsafePath }
        let duplicate = fcntl(directoryDescriptor, F_DUPFD_CLOEXEC, 0)
        guard duplicate >= 0 else { throw Code0ProfileLeaseError.unavailable }
        return duplicate
    }
    public func release() {
        if descriptor >= 0 { _ = flock(descriptor, LOCK_UN); _ = Darwin.close(descriptor) }
        if directoryDescriptor >= 0 { _ = Darwin.close(directoryDescriptor) }
        descriptor = -1; directoryDescriptor = -1; authorityLost = false
    }
    deinit {
        if descriptor >= 0 { _ = Darwin.close(descriptor) }
        if directoryDescriptor >= 0 { _ = Darwin.close(directoryDescriptor) }
    }
}

/// Descriptor-relative operations prevent a replaced descendant/symlink from
/// redirecting a cache operation. No constructor or read creates a directory.
enum Code0SafeFiles {
    enum Failure: Error { case absent, unsafe, io }
    static func openDirectory(_ location: Code0StorageLocation, create: Bool) throws -> Int32 {
        let components = location.directoryURL.pathComponents.filter { $0 != "/" }
        var current = open("/", O_RDONLY | O_DIRECTORY | O_CLOEXEC)
        guard current >= 0 else { throw Failure.io }
        do {
            for (index, component) in components.enumerated() {
                var next = openat(current, component, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)
                if next < 0, errno == ENOENT, create {
                    guard mkdirat(current, component, mode_t(0o700)) == 0 || errno == EEXIST else { throw Failure.io }
                    next = openat(current, component, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)
                }
                guard next >= 0 else {
                    if errno == ENOENT { throw Failure.absent }
                    if errno == ELOOP || errno == ENOTDIR { throw Failure.unsafe }
                    throw Failure.io
                }
                _ = Darwin.close(current)
                current = next
                if index >= components.count - 2 {
                    var info = stat()
                    guard fstat(current, &info) == 0 else { throw Failure.io }
                    guard info.st_uid == getuid(), info.st_mode & 0o022 == 0,
                          index != components.count - 1 || info.st_mode & 0o077 == 0 else { throw Failure.unsafe }
                }
            }
            return current
        } catch { _ = Darwin.close(current); throw error }
    }
    static func validateRegular(_ descriptor: Int32, maximumSize: Int?) throws -> stat {
        var info = stat()
        guard fstat(descriptor, &info) == 0 else { throw Failure.io }
        guard info.st_mode & mode_t(S_IFMT) == mode_t(S_IFREG), info.st_uid == getuid(), info.st_nlink == 1,
              info.st_mode & 0o077 == 0 else { throw Failure.unsafe }
        if let maximumSize, info.st_size < 0 || info.st_size > off_t(maximumSize) { throw Failure.io }
        return info
    }
    static func matches(_ directory: Int32, name: String, descriptor: Int32) -> Bool {
        var pathInfo = stat(), openInfo = stat()
        return fstatat(directory, name, &pathInfo, AT_SYMLINK_NOFOLLOW) == 0 && fstat(descriptor, &openInfo) == 0 &&
            pathInfo.st_dev == openInfo.st_dev && pathInfo.st_ino == openInfo.st_ino && pathInfo.st_nlink == 1
    }
    static func validateExisting(_ directory: Int32, name: String) throws {
        let fd = openat(directory, name, O_RDONLY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC)
        guard fd >= 0 else {
            if errno == ENOENT { return }
            if errno == ELOOP || errno == EISDIR { throw Failure.unsafe }
            throw Failure.io
        }
        defer { _ = Darwin.close(fd) }
        _ = try validateRegular(fd, maximumSize: nil)
        guard matches(directory, name: name, descriptor: fd) else { throw Failure.unsafe }
    }
}
