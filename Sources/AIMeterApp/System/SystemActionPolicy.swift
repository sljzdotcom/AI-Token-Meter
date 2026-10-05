import Foundation

struct SystemActionPolicy {
    private static let testEnvironmentKeys: Set<String> = [
        "XCTestBundlePath",
        "XCTestConfigurationFilePath",
        "XCTestSessionIdentifier",
        "XCInjectBundleInto",
    ]

    let allowsExternalOpen: Bool

    init(environment: [String: String], isXCTestBundle: Bool, processName: String = "") {
        let normalizedProcessName = URL(fileURLWithPath: processName).lastPathComponent.lowercased()
        let isTestRunner = normalizedProcessName == "swiftpm-testing-helper"
            || normalizedProcessName == "xctest"
            || normalizedProcessName.hasSuffix(".xctest")
        let hasXCTestMarker = Self.testEnvironmentKeys.contains { key in
            guard let value = environment[key]?.trimmingCharacters(in: .whitespacesAndNewlines) else {
                return false
            }
            return !value.isEmpty
        }
        allowsExternalOpen = !isXCTestBundle && !isTestRunner && !hasXCTestMarker
    }

    static var current: SystemActionPolicy {
        SystemActionPolicy(
            environment: ProcessInfo.processInfo.environment,
            isXCTestBundle: Bundle.main.bundleURL.pathExtension == "xctest",
            processName: ProcessInfo.processInfo.processName
        )
    }

    func open(_ url: URL, using systemOpen: (URL) -> Bool) -> Bool {
        guard allowsExternalOpen else { return false }
        return systemOpen(url)
    }

    func scriptDirectory(
        name: String,
        applicationSupportDirectory: URL,
        temporaryDirectory: URL,
        uniqueSuffix: String = UUID().uuidString
    ) -> URL {
        let root = allowsExternalOpen
            ? applicationSupportDirectory.appendingPathComponent("AI Meter", isDirectory: true)
            : temporaryDirectory
                .appendingPathComponent("AI Meter Tests", isDirectory: true)
                .appendingPathComponent(uniqueSuffix, isDirectory: true)
        return root.appendingPathComponent(name, isDirectory: true)
    }
}
