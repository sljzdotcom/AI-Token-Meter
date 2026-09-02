import Foundation

public enum AppBrand {
    public static let displayName = "AI Token Meter"
    public static let subtitle = "Private AI usage monitor"
    public static let author = "Miller"
    public static let authorLine = "Author: \(author)"

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
