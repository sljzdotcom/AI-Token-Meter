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

    public static func checking(provider: UsageProvider) -> Self {
        Self(provider: provider, connectionState: .checking, checkedAt: nil)
    }
}

public protocol ServiceAccountReading: Sendable {
    var provider: UsageProvider { get }
    func read() async -> ServiceAccountStatus
}
