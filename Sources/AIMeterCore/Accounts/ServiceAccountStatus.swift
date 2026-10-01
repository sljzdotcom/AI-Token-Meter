import Foundation

public enum ServiceAccountConnectionState: Equatable, Sendable {
    case connected
    case lastKnown
    case signInRequired
    case notInstalled
    case checking
    case unavailable
}

public struct ServiceAccountStatus: Equatable, Sendable {
    public let provider: UsageProvider
    public let connectionState: ServiceAccountConnectionState
    public let accountLabel: String?
    public let accountDetail: String?
    public let checkedAt: Date?
    public let refreshPauseReason: GeminiPauseReason?

    public init(
        provider: UsageProvider,
        connectionState: ServiceAccountConnectionState,
        accountLabel: String? = nil,
        accountDetail: String? = nil,
        checkedAt: Date? = Date(),
        refreshPauseReason: GeminiPauseReason? = nil
    ) {
        self.provider = provider
        self.connectionState = connectionState
        self.accountLabel = accountLabel
        self.accountDetail = accountDetail
        self.checkedAt = checkedAt
        self.refreshPauseReason = refreshPauseReason
    }

    public static var geminiUnavailable: Self {
        Self(provider: .gemini, connectionState: .unavailable,
             accountDetail: "Antigravity CLI account status is not available. Installation and sign-in have not been checked.", checkedAt: nil)
    }

    public static func checking(provider: UsageProvider) -> Self {
        Self(provider: provider, connectionState: .checking, checkedAt: nil)
    }
}

public protocol ServiceAccountReading: Sendable {
    var provider: UsageProvider { get }
    func read() async -> ServiceAccountStatus
}

public extension ServiceAccountStatus {
    static func fromGeminiSnapshot(
        _ snapshot: UsageSnapshot,
        pauseReason: GeminiPauseReason? = nil
    ) -> Self {
        let state: ServiceAccountConnectionState
        if pauseReason == .authenticationRequired {
            state = .signInRequired
        } else {
            switch snapshot.collectionStatus {
            case .fresh: state = snapshot.geminiQuotaMetrics?.isEmpty == false ? .connected : .unavailable
            case .cached: state = snapshot.geminiQuotaMetrics?.isEmpty == false ? .lastKnown : .unavailable
            case .notInstalled: state = .notInstalled
            case .authenticationRequired: state = .signInRequired
            default: state = .unavailable
            }
        }
        return Self(provider: .gemini, connectionState: state,
                    accountDetail: snapshot.statusMessage ?? (state == .connected ? "Official Antigravity CLI quota verified; account identity not provided" : "Antigravity CLI quota unavailable"),
                    checkedAt: snapshot.fetchedAt,
                    refreshPauseReason: pauseReason)
    }
}
