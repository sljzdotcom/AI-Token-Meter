import Testing
@testable import AIMeterApp

@Suite("Gemini detail panel layout")
struct GeminiDetailPanelLayoutTests {
    @Test func allThreeTiersFitAndSmallScreensRemainScrollable() {
        #expect(GeminiDetailPanelLayout.height(tierCount: 3, availableHeight: 900) >= 450)
        #expect(GeminiDetailPanelLayout.height(tierCount: 3, availableHeight: 360) == 344)
        #expect(GeminiDetailPanelLayout.height(tierCount: 0, availableHeight: 900) == 280)
    }
}
