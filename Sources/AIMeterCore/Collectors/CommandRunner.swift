import Foundation

public struct CommandRequest: Sendable {
    public let executableURL: URL
    public let arguments: [String]
    public let inputLines: [String]
    public let timeout: TimeInterval

    public init(
        executableURL: URL,
        arguments: [String] = [],
        inputLines: [String],
        timeout: TimeInterval
    ) {
        self.executableURL = executableURL
        self.arguments = arguments
        self.inputLines = inputLines
        self.timeout = timeout
    }
}

public struct CommandResult: Equatable, Sendable {
    public let output: String
    public let exitCode: Int32
    public let duration: TimeInterval

    public init(output: String, exitCode: Int32, duration: TimeInterval) {
        self.output = output
        self.exitCode = exitCode
        self.duration = duration
    }
}

public protocol CommandRunning: Sendable {
    func run(_ request: CommandRequest) async throws -> CommandResult
}

