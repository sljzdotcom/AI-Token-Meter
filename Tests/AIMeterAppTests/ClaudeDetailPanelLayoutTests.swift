import Foundation
import Testing
@testable import AIMeterApp

@Suite("Claude detail panel layout")
struct ClaudeDetailPanelLayoutTests {
    @Test("Uses the approved rich detail size on standard screens")
    func standardSize() {
        #expect(ClaudeDetailPanelLayout.size(availableHeight: 900) == CGSize(width: 390, height: 560))
    }

    @Test("Keeps a useful height while respecting compact screens")
    func compactScreens() {
        #expect(ClaudeDetailPanelLayout.size(availableHeight: 540) == CGSize(width: 390, height: 516))
        #expect(ClaudeDetailPanelLayout.size(availableHeight: 510) == CGSize(width: 390, height: 486))
        #expect(ClaudeDetailPanelLayout.size(availableHeight: 20) == CGSize(width: 390, height: 0))
    }
}
