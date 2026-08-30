import Foundation
import Testing
@testable import AIMeterCore

@Suite("DeepSeek local budget tracker", .serialized)
struct DeepSeekBudgetTrackerTests {
    @Test("Accumulates observed balance decreases without treating top-ups as negative spend")
    func tracksObservedSpend() {
        let defaults = isolatedDefaults()
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let tracker = DeepSeekBudgetTracker(defaults: defaults, keyPrefix: "test")
        let date = Date(timeIntervalSince1970: 1_787_875_200) // 2026-08-28 UTC

        #expect(tracker.record(balance: 100, currency: .cny, at: date) == 0)
        #expect(tracker.record(balance: 92, currency: .cny, at: date) == 8)
        #expect(tracker.record(balance: 120, currency: .cny, at: date) == 8)
        #expect(tracker.record(balance: 115, currency: .cny, at: date) == 13)
    }

    @Test("Starts a new local tracking cycle when the calendar month changes")
    func resetsEachMonth() {
        let defaults = isolatedDefaults()
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let tracker = DeepSeekBudgetTracker(defaults: defaults, keyPrefix: "test")
        let august = Date(timeIntervalSince1970: 1_787_875_200)
        let september = Date(timeIntervalSince1970: 1_788_220_800)

        _ = tracker.record(balance: 100, currency: .cny, at: august)
        #expect(tracker.record(balance: 90, currency: .cny, at: august) == 10)
        #expect(tracker.record(balance: 88, currency: .cny, at: september) == 0)
    }

    @Test("Cached observations report existing spend without changing the baseline")
    func ignoresCachedChanges() {
        let defaults = isolatedDefaults()
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let tracker = DeepSeekBudgetTracker(defaults: defaults, keyPrefix: "test")
        let date = Date(timeIntervalSince1970: 1_787_875_200)

        _ = tracker.record(balance: 100, currency: .cny, at: date)
        #expect(tracker.record(balance: 70, currency: .cny, at: date, isFresh: false) == 0)
        #expect(tracker.record(balance: 95, currency: .cny, at: date) == 5)
    }

    private var suiteName: String { "com.millerpan.AIMeter.BudgetTrackerTests" }

    private func isolatedDefaults() -> UserDefaults {
        let defaults = UserDefaults(suiteName: suiteName)!
        defaults.removePersistentDomain(forName: suiteName)
        return defaults
    }
}
