import Foundation

public enum ServiceAccountConnectionState: Equatable, Sendable {
    case connected
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

    public init(
        provider: UsageProvider,
        connectionState: ServiceAccountConnectionState,
        accountLabel: String? = nil,
        accountDetail: String? = nil,
        checkedAt: Date? = Date()
    ) {
        self.provider = provider
        self.connectionState = connectionState
        self.accountLabel = accountLabel
        self.accountDetail = accountDetail
        self.checkedAt = checkedAt
    }

    public static var geminiUnavailable: Self {
        Self(provider: .gemini, connectionState: .unavailable,
             accountDetail: "Gemini CLI account status is not available. Installation and sign-in have not been checked.", checkedAt: nil)
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
    static func fromGeminiSnapshot(_ snapshot: UsageSnapshot) -> Self {
        let state: ServiceAccountConnectionState
        switch snapshot.collectionStatus {
        case .fresh: state = snapshot.geminiQuotaMetrics?.isEmpty == false ? .connected : .unavailable
        case .notInstalled: state = .notInstalled
        case .authenticationRequired: state = .signInRequired
        default: state = .unavailable
        }
        return Self(provider: .gemini, connectionState: state,
                    accountDetail: snapshot.statusMessage ?? (state == .connected ? "Official Gemini CLI quota verified; account identity not provided" : "Gemini CLI quota unavailable"),
                    checkedAt: snapshot.fetchedAt)
    }
}
