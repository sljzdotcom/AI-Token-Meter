public struct ClaudeUsageParser: Sendable {
    public init() {}

    public func parse(_ text: String) throws -> UsageSnapshot {
        try TerminalUsageParser.parse(text, provider: .claude)
    }
}
