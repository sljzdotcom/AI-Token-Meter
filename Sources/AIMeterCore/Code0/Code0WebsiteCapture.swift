import Foundation

public enum Code0WebsiteCaptureError: Error, Equatable, Sendable {
    case invalidPayload
}

/// Accepts only the three filtered quota numbers, never an account response.
public struct Code0WebsiteCapture: Equatable, Sendable {
    public let balance: Double
    public let cumulativeUsage: Double

    private init(balance: Double, cumulativeUsage: Double) {
        self.balance = balance
        self.cumulativeUsage = cumulativeUsage
    }

    /// Deny-only RED scaffold. Expected keys: quota, usedQuota, divisor.
    public static func decode(filteredPayload: Data) throws -> Self {
        throw Code0WebsiteCaptureError.invalidPayload
    }

    public func makeSnapshot(fetchedAt: Date) throws -> UsageSnapshot {
        throw Code0WebsiteCaptureError.invalidPayload
    }
}
