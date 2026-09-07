import Foundation

public enum AppBrand {
    public struct Link: Equatable, Sendable {
        public let label: String
        public let url: URL
        public let systemImage: String

        public init(label: String, url: URL, systemImage: String) {
            self.label = label
            self.url = url
            self.systemImage = systemImage
        }
    }

    public static let displayName = "AI Token Meter"
    public static let subtitle = "Private AI usage monitor"
    public static let author = "Miller"
    public static let authorLine = "Author: \(author)"
    public static let authorLinks = [
        Link(
            label: "@MillerPanYue",
            url: URL(string: "https://twitter.com/MillerPanYue")!,
            systemImage: "bubble.left"
        ),
        Link(
            label: "GitHub",
            url: URL(string: "https://github.com/sljzdotcom/AI-Token-Meter")!,
            systemImage: "chevron.left.forwardslash.chevron.right"
        ),
    ]

    public static func versionText(info: [String: Any]) -> String {
        guard let version = info["CFBundleShortVersionString"] as? String,
              let build = info["CFBundleVersion"] as? String,
              !version.isEmpty,
              !build.isEmpty else {
            return "Version unavailable"
        }
        return "Version \(version) (\(build))"
    }
}
