import Foundation
import Testing
@testable import AIMeterCore

@Suite("Floating strip position")
struct FloatingStripPositionTests {
    @Test("Defaults to automatic on the right at vertical center")
    func defaults() {
        let suiteName = "AIMeter.FloatingStripPositionTests.defaults"
        let defaults = UserDefaults(suiteName: suiteName)!
        defaults.removePersistentDomain(forName: suiteName)
        defer { defaults.removePersistentDomain(forName: suiteName) }

        let value = FloatingStripPositionStore(defaults: defaults).load()

        #expect(value.preference == .automatic)
        #expect(value.lastResolvedEdge == .right)
        #expect(value.normalizedCenterY == 0.5)
        #expect(value.screenIdentifier == nil)
    }

    @Test("Clamps corrupt vertical positions and rejects unknown edges")
    func corruptValues() {
        let suiteName = "AIMeter.FloatingStripPositionTests.corruptValues"
        let defaults = UserDefaults(suiteName: suiteName)!
        defaults.removePersistentDomain(forName: suiteName)
        defer { defaults.removePersistentDomain(forName: suiteName) }
        defaults.set("future", forKey: "floatingStrip.edgePreference")
        defaults.set("diagonal", forKey: "floatingStrip.lastResolvedEdge")
        defaults.set(4.0, forKey: "floatingStrip.normalizedCenterY")

        let value = FloatingStripPositionStore(defaults: defaults).load()

        #expect(value.preference == .automatic)
        #expect(value.lastResolvedEdge == .right)
        #expect(value.normalizedCenterY == 1)
    }

    @Test("Non-finite vertical positions recover to the screen center")
    func nonFiniteValues() {
        for value in [Double.nan, .infinity, -.infinity] {
            let position = FloatingStripPosition(normalizedCenterY: value)
            #expect(position.normalizedCenterY == 0.5)
        }
    }

    @Test("Persists every placement field and clamps the saved vertical position")
    func roundTrip() {
        let suiteName = "AIMeter.FloatingStripPositionTests.roundTrip"
        let defaults = UserDefaults(suiteName: suiteName)!
        defaults.removePersistentDomain(forName: suiteName)
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let store = FloatingStripPositionStore(defaults: defaults)

        store.save(FloatingStripPosition(
            preference: .left,
            lastResolvedEdge: .left,
            normalizedCenterY: -0.5,
            screenIdentifier: "screen-42"
        ))

        let value = store.load()
        #expect(value.preference == .left)
        #expect(value.lastResolvedEdge == .left)
        #expect(value.normalizedCenterY == 0)
        #expect(value.screenIdentifier == "screen-42")
    }
}
