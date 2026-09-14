import Foundation
import Testing
@testable import AIMeterCore

@Suite("Antigravity CLI supplemental information parser")
struct AntigravityCLIInfoParserTests {
    @Test func parsesOneCurrentGeminiModel() {
        #expect(
            AntigravityCLIInfoParser.currentGeminiModel(
                from: "gemini-3.8-flash-high\tGemini 3.8 Flash (High)\n"
            ) == "Gemini 3.8 Flash (High)"
        )
    }

    @Test(arguments: [
        "claude-sonnet-4-6\tClaude Sonnet 4.6 (Thinking)",
        "gemini-3.8-flash-high\tGemini 3.8 Flash (High)\ngemini-3.7-flash-high\tGemini 3.7 Flash (High)",
        "gemini-3.8-flash-high",
        "Fetching available models...",
        "gemini-3.8-flash-high\tGemini user@example.com",
        "gemini-3.8-flash-high\tGemini /Users/example/.config",
        "gemini-3.8-flash-high\tGemini sk-proj-secretvalue",
        "gemini-3.8-flash-high\tGemini Claude/GPT",
        "gemini--3.8-flash\tGemini 3.8 Flash",
    ])
    func omitsThirdPartyAmbiguousAndMalformedCurrentModels(_ output: String) {
        #expect(AntigravityCLIInfoParser.currentGeminiModel(from: output) == nil)
    }

    @Test func filtersThirdPartyModelsAndGroupsGeminiFamilies() throws {
        let output = """
        Fetching available models...
        gemini-3.8-flash-high\tGemini 3.8 Flash (High)
        gemini-3.8-flash-medium\tGemini 3.8 Flash (Medium)
        gemini-3.7-flash-low\tGemini 3.7 Flash (Low)
        gemini-3.1-pro-high\tGemini 3.1 Pro (High)
        claude-sonnet-4-6\tClaude Sonnet 4.6 (Thinking)
        gpt-oss-120b-medium\tGPT-OSS 120B (Medium)
        """

        let catalog = try #require(AntigravityCLIInfoParser.geminiCatalog(from: output))

        #expect(catalog.modelCount == 4)
        #expect(catalog.families == [
            "Gemini 3.8 Flash",
            "Gemini 3.7 Flash",
            "Gemini 3.1 Pro",
        ])
    }

    @Test(arguments: [
        "",
        "Fetching available models...",
        "gemini-3.8-flash-high",
        "gemini-3.8-flash-high\tClaude Sonnet",
        "unknown\tGemini 3.8 Flash",
        "gemini-3.8-flash-high\tGemini 3.8 Flash\ngemini-3.7-flash-high\tGemini user@example.com",
    ])
    func omitsUnavailableOrMalformedCatalogs(_ output: String) {
        #expect(AntigravityCLIInfoParser.geminiCatalog(from: output) == nil)
    }
}
