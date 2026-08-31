import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("AI Meter visual system")
struct VisualSystemTests {
    @Test("Provider logos use one optical calibration table")
    func providerLogoScales() {
        #expect(ProviderLogoStyle.opticalScale(for: .claude) > 1)
        #expect(ProviderLogoStyle.opticalScale(for: .codex) == 1)
        #expect(ProviderLogoStyle.opticalScale(for: .deepSeek) < 1)
    }

    @Test("Both island orientations fill the complete window without transparent edge gaps")
    func floatingStripBounds() {
        let rect = CGRect(x: 0, y: 0, width: 108, height: 356)

        #expect(FloatingStripShape(edge: .right).path(in: rect).boundingRect == rect)
        #expect(FloatingStripShape(edge: .left).path(in: rect).boundingRect == rect)
    }
}
