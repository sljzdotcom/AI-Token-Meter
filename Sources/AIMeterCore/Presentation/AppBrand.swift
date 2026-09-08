import Foundation

public enum AppBrand {
    public struct Link: Equatable, Sendable {
        public let label: String
        public let url: URL

        public init(label: String, url: URL) {
            self.label = label
            self.url = url
        }
    }

    public static let displayName = "AI Token Meter"
    public static let subtitle = "Private AI usage monitor"
    public static let author = "Miller"
    public static let authorLine = "Author: \(author)"
    public static let authorLinks = [
        Link(
            label: "@MillerPanYue",
            url: URL(string: "https://twitter.com/MillerPanYue")!
        ),
        Link(
            label: "GitHub",
            url: URL(string: "https://github.com/sljzdotcom/AI-Token-Meter")!
        ),
        Link(
            label: "Telegram @sljzdotcom",
            url: URL(string: "https://t.me/sljzdotcom")!
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
