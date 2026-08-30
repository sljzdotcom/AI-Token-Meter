import Foundation

public protocol ClaudeUsageWorkspaceResolving: Sendable {
    func resolve() throws -> URL
}

public struct ClaudeUsageWorkspaceResolver: ClaudeUsageWorkspaceResolving {
    public init() {}

    public func resolve() throws -> URL {
        let fileManager = FileManager.default
        let base = fileManager.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        )[0]
        let workspace = base
            .appendingPathComponent("AI Meter", isDirectory: true)
            .appendingPathComponent("ClaudeUsageWorkspace", isDirectory: true)
        try fileManager.createDirectory(
            at: workspace,
            withIntermediateDirectories: true
        )
        return workspace
    }
}
