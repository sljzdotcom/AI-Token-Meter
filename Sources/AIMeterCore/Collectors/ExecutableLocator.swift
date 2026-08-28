import Foundation

public protocol ExecutableLocating: Sendable {
    func locate(named name: String) -> URL?
}

public struct ExecutableLocator: ExecutableLocating {
    private let searchPaths: [String]

    public init(searchPaths: [String]? = nil) {
        if let searchPaths {
            self.searchPaths = searchPaths
            return
        }

        let environmentPaths = ProcessInfo.processInfo.environment["PATH"]?
            .split(separator: ":")
            .map(String.init) ?? []
        let userLocalPath = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".local/bin").path
        self.searchPaths = environmentPaths + [
            userLocalPath,
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
        ]
    }

    public func locate(named name: String) -> URL? {
        for path in searchPaths {
            let candidate = URL(fileURLWithPath: path).appendingPathComponent(name)
            if FileManager.default.isExecutableFile(atPath: candidate.path) {
                return candidate
            }
        }
        return nil
    }
}
