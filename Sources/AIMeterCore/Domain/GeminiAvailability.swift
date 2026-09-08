import Foundation

public extension UsageSnapshot {
    /// A capability state, not a collected account observation. No percentage or sample time is implied.
    static var geminiUnavailable: UsageSnapshot {
        UsageSnapshot(provider: .gemini, availability: .unavailable,
                      fetchedAt: .distantPast, collectionStatus: .unavailable,
                      statusMessage: "Gemini CLI quota is currently unavailable. Installation and account status have not been checked.")
    }
}
