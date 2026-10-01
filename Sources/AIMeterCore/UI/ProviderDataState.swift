import Foundation

public enum ProviderOperationState: String, Codable, Sendable {
    case idle, refreshing, waiting, paused
    public static func resolve(status: CollectionStatus, refreshing: Bool, needsAction: Bool, paused: Bool = false) -> Self {
        if refreshing || status == .refreshing { return .refreshing }
        if paused { return .paused }
        if needsAction || [.authenticationRequired, .setupRequired, .notInstalled].contains(status) { return .waiting }
        return .idle
    }
}

public enum ProviderDataState {
    public static func freshness(_ snapshot: UsageSnapshot, now: Date = Date()) -> String {
        switch snapshot.collectionStatus {
        case .fresh where !snapshot.isStale(at: now): return "Fresh"
        case .fresh, .cached:
            return "Cached · \(Int(max(now.timeIntervalSince(snapshot.fetchedAt), 0) / 60)) min ago"
        case .refreshing: return "Refreshing"
        case .authenticationRequired: return "Needs sign-in"
        case .setupRequired, .notInstalled: return "Needs setup"
        default: return "Unavailable"
        }
    }
}
