import Foundation

public protocol UsageCollector: Sendable {
    var provider: UsageProvider { get }
    func collect() async throws -> UsageSnapshot
}

public protocol OneTimeGeminiQuotaCollecting: UsageCollector {
    func collectQuotaOnce() async throws -> UsageSnapshot
}

public enum UsageCollectionError: Error, Equatable, Sendable {
    case geminiUnavailable(String)
    case notInstalled
    case authenticationRequired
    case setupRequired
    case timedOut
    case outputLimitExceeded
    case unrecognizedOutput
    case transportFailure
    case invalidResponse
    case unsupportedVersion
    case environmentRejected
    case executableUnavailable
    case rateLimited
    case rateLimitedRetryAfter(TimeInterval)
}
