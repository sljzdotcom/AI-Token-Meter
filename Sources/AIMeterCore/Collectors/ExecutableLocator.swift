import Foundation

public protocol ExecutableLocating: Sendable {
    func locate(named name: String) -> URL?
}

public struct ExecutableLocator: ExecutableLocating {
    private let searchPaths: [String]
    private let bundledExecutablePaths: [String: [String]]

    public init(searchPaths: [String]? = nil) {
        if let searchPaths {
            self.searchPaths = searchPaths
            self.bundledExecutablePaths = Self.defaultBundledExecutablePaths()
            return
        }

        let environmentPaths = ProcessInfo.processInfo.environment["PATH"]?
            .split(separator: ":")
            .map(String.init) ?? []
        let userLocalPath = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".local/bin").path
        self.searchPaths = environmentPaths + [
            userLocalPath,
        ] + Self.nodeManagerSearchPaths(
            homeDirectory: FileManager.default.homeDirectoryForCurrentUser
        ) + [
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
        ]
        self.bundledExecutablePaths = Self.defaultBundledExecutablePaths()
    }

    init(
        searchPaths: [String],
        bundledExecutablePaths: [String: [String]]
    ) {
        self.searchPaths = searchPaths
        self.bundledExecutablePaths = bundledExecutablePaths
    }

    public func locate(named name: String) -> URL? {
        for path in searchPaths {
            let candidate = URL(fileURLWithPath: path).appendingPathComponent(name)
            if FileManager.default.isExecutableFile(atPath: candidate.path) {
                return candidate
            }
        }
        for path in bundledExecutablePaths[name] ?? [] {
            if FileManager.default.isExecutableFile(atPath: path) {
                return URL(fileURLWithPath: path)
            }
        }
        return nil
    }

    private static func defaultBundledExecutablePaths() -> [String: [String]] {
        let home = FileManager.default.homeDirectoryForCurrentUser
        let userApplications = home.appendingPathComponent("Applications", isDirectory: true)
        let systemApplications = URL(fileURLWithPath: "/Applications", isDirectory: true)
        let appNames = ["ChatGPT.app", "Codex.app"]
        let candidates = [userApplications, systemApplications].flatMap { applications in
            appNames.map { appName in
                applications
                    .appendingPathComponent(appName, isDirectory: true)
                    .appendingPathComponent("Contents/Resources/codex").path
            }
        }
        return ["codex": candidates]
    }

    static func nvmSearchPaths(
        homeDirectory: URL,
        fileManager: FileManager = .default
    ) -> [String] {
        let nvmRoot = homeDirectory.appendingPathComponent(".nvm", isDirectory: true)
        let versionsRoot = nvmRoot.appendingPathComponent("versions/node", isDirectory: true)
        let versions = (try? fileManager.contentsOfDirectory(
            at: versionsRoot,
            includingPropertiesForKeys: [.isDirectoryKey],
            options: [.skipsHiddenFiles]
        )) ?? []
        let versionBins = versions
            .filter { (try? $0.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true }
            .sorted {
                $0.lastPathComponent.compare(
                    $1.lastPathComponent,
                    options: [.numeric, .caseInsensitive]
                ) == .orderedDescending
            }
            .map { $0.appendingPathComponent("bin", isDirectory: true).path }
        let currentBin = nvmRoot.appendingPathComponent("current/bin", isDirectory: true)
        var isDirectory: ObjCBool = false
        let currentPaths = fileManager.fileExists(atPath: currentBin.path, isDirectory: &isDirectory)
            && isDirectory.boolValue
            ? [currentBin.path]
            : []
        return currentPaths + versionBins
    }

    private static func nodeManagerSearchPaths(homeDirectory: URL) -> [String] {
        [
            homeDirectory.appendingPathComponent(".npm-global/bin", isDirectory: true).path,
            homeDirectory.appendingPathComponent(".volta/bin", isDirectory: true).path,
            homeDirectory.appendingPathComponent(".asdf/shims", isDirectory: true).path,
            homeDirectory.appendingPathComponent(".local/share/mise/shims", isDirectory: true).path,
        ] + nvmSearchPaths(homeDirectory: homeDirectory)
    }
}
