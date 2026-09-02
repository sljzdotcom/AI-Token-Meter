import Foundation

struct SoftwareUpdateRelease: Equatable, Sendable {
    static let summaryLimit = 240

    let version: String
    let build: String
    let publishedAt: Date?
    let summary: String?

    var sanitizedSummary: String? {
        guard let summary else { return nil }
        let collapsed = summary
            .components(separatedBy: .whitespacesAndNewlines)
            .filter { !$0.isEmpty }
            .joined(separator: " ")
        guard !collapsed.isEmpty else { return nil }
        return String(collapsed.prefix(Self.summaryLimit))
    }
}

enum SoftwareUpdateState: Equatable, Sendable {
    case idle
    case checking
    case upToDate
    case available(SoftwareUpdateRelease)
    case installing(SoftwareUpdateRelease)
    case failed(String)

    var canCheck: Bool {
        switch self {
        case .checking, .installing:
            false
        case .idle, .upToDate, .available, .failed:
            true
        }
    }

    var canInstall: Bool {
        if case .available = self {
            true
        } else {
            false
        }
    }

    var statusText: String {
        switch self {
        case .idle:
            "Not checked yet"
        case .checking:
            "Checking…"
        case .upToDate:
            "You’re up to date"
        case let .available(release):
            "Version \(release.version) is available"
        case let .installing(release):
            "Preparing version \(release.version)…"
        case let .failed(message):
            message
        }
    }

    var availableRelease: SoftwareUpdateRelease? {
        switch self {
        case let .available(release), let .installing(release):
            release
        case .idle, .checking, .upToDate, .failed:
            nil
        }
    }
}
