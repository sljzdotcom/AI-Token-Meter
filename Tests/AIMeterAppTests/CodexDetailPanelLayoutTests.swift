import Testing
@testable import AIMeterApp

@Suite("Codex detail panel layout")
struct CodexDetailPanelLayoutTests {
    @Test("Grows for additional credits and respects the screen")
    func adaptiveHeight() {
        #expect(CodexDetailPanelLayout.height(creditCount: 0, availableHeight: 900) == 470)
        #expect(CodexDetailPanelLayout.height(creditCount: 1, availableHeight: 900) == 520)
        #expect(CodexDetailPanelLayout.height(creditCount: 3, availableHeight: 900) == 704)
        #expect(CodexDetailPanelLayout.height(creditCount: 8, availableHeight: 640) == 624)
    }
}
