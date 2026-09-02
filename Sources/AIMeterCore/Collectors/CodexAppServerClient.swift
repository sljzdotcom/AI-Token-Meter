import Foundation

struct CodexAppServerClient: Sendable {
    private let beforeProcessRegistration: (@Sendable () -> Void)?
    private let environmentOverrides: [String: String]

    init(
        beforeProcessRegistration: (@Sendable () -> Void)? = nil,
        environmentOverrides: [String: String] = [:]
    ) {
        self.beforeProcessRegistration = beforeProcessRegistration
        self.environmentOverrides = environmentOverrides
    }

    func readRateLimits(executableURL: URL, timeout: TimeInterval = 10) async throws -> UsageSnapshot {
        let envelope: CodexRateLimitsEnvelope = try await request(
            executableURL: executableURL,
            method: "account/rateLimits/read",
            params: .null,
            timeout: timeout
        )
        guard let result = envelope.result else {
            throw UsageCollectionError.invalidResponse
        }
        return snapshot(from: result)
    }

    func readAccount(executableURL: URL, timeout: TimeInterval = 10) async throws -> CodexAccountResult {
        let envelope: CodexAccountEnvelope = try await request(
            executableURL: executableURL,
            method: "account/read",
            params: .refreshToken(false),
            timeout: timeout
        )
        guard let result = envelope.result else {
            throw UsageCollectionError.invalidResponse
        }
        return result
    }

    private func request<Response: Decodable & Sendable>(
        executableURL: URL,
        method: String,
        params: CodexRequestParameters,
        timeout: TimeInterval
    ) async throws -> Response {
        let processBox = CodexProcessBox()

        return try await withThrowingTaskGroup(of: Response.self) { group in
            group.addTask {
                do {
                    return try await Task.detached(priority: .utility) {
                        try requestBlocking(
                            executableURL: executableURL,
                            method: method,
                            params: params,
                            processBox: processBox
                        ) as Response
                    }.value
                } catch {
                    if processBox.timeoutWasRequested {
                        throw UsageCollectionError.timedOut
                    }
                    throw error
                }
            }
            group.addTask {
                try await Task.sleep(for: .seconds(timeout))
                processBox.stopForTimeout()
                throw UsageCollectionError.timedOut
            }

            defer { group.cancelAll() }
            guard let result = try await group.next() else {
                throw UsageCollectionError.transportFailure
            }
            return result
        }
    }

    private func requestBlocking<Response: Decodable>(
        executableURL: URL,
        method: String,
        params: CodexRequestParameters,
        processBox: CodexProcessBox
    ) throws -> Response {
        let process = Process()
        let terminationWaiter = ProcessTerminationWaiter()
        let input = Pipe()
        let output = Pipe()

        process.executableURL = executableURL
        process.arguments = ["app-server", "--stdio"]
        process.standardInput = input
        process.standardOutput = output
        process.standardError = FileHandle.nullDevice
        process.environment = processEnvironment(for: executableURL)
        terminationWaiter.attach(to: process)

        try process.run()
        terminationWaiter.beginFallbackWait(for: process)
        beforeProcessRegistration?()
        processBox.set(
            process: process,
            input: input.fileHandleForWriting,
            output: output.fileHandleForReading
        )
        defer {
            processBox.stop()
            _ = terminationWaiter.wait()
            processBox.clear(process: process)
        }

        try writeJSON([
            "id": 1,
            "method": "initialize",
            "params": [
                "clientInfo": ["name": "ai-meter", "version": "0.2.1"],
                "capabilities": ["experimentalApi": true],
            ],
        ], to: input.fileHandleForWriting)
        _ = try response(withID: 1, from: output.fileHandleForReading)

        try writeJSON(["method": "initialized"], to: input.fileHandleForWriting)
        try writeJSON([
            "id": 2,
            "method": method,
            "params": params.jsonObject,
        ], to: input.fileHandleForWriting)

        let responseData = try response(withID: 2, from: output.fileHandleForReading)
        return try JSONDecoder().decode(Response.self, from: responseData)
    }

    private func processEnvironment(for executableURL: URL) -> [String: String] {
        var environment = ProcessInfo.processInfo.environment.merging(
            environmentOverrides,
            uniquingKeysWith: { _, override in override }
        )
        let executableDirectory = executableURL.deletingLastPathComponent().path
        let configuredPath = environment["PATH"]?.trimmingCharacters(in: .whitespacesAndNewlines)
        let inheritedPath: String
        if let configuredPath, !configuredPath.isEmpty {
            inheritedPath = configuredPath
        } else {
            inheritedPath = "/usr/bin:/bin:/usr/sbin:/sbin"
        }
        let pathParts = inheritedPath.split(separator: ":").map(String.init)
        if !pathParts.contains(executableDirectory) {
            environment["PATH"] = "\(executableDirectory):\(inheritedPath)"
        }
        return environment
    }

    private func response(withID expectedID: Int, from handle: FileHandle) throws -> Data {
        for _ in 0..<100 {
            guard let line = try readLine(from: handle) else {
                throw UsageCollectionError.transportFailure
            }
            guard let object = try JSONSerialization.jsonObject(with: line) as? [String: Any] else {
                continue
            }
            if object["id"] as? Int == expectedID {
                return line
            }
        }
        throw UsageCollectionError.invalidResponse
    }

    private func readLine(from handle: FileHandle) throws -> Data? {
        var line = Data()
        while true {
            guard let byte = try handle.read(upToCount: 1), !byte.isEmpty else {
                return line.isEmpty ? nil : line
            }
            if byte.first == 0x0A { return line }
            line.append(byte)
            if line.count > 1_048_576 {
                throw UsageCollectionError.invalidResponse
            }
        }
    }

    private func writeJSON(_ object: [String: Any], to handle: FileHandle) throws {
        var data = try JSONSerialization.data(withJSONObject: object)
        data.append(0x0A)
        try handle.write(contentsOf: data)
    }

    private func snapshot(from result: CodexRateLimitsResult) -> UsageSnapshot {
        let selected = result.rateLimits

        return UsageSnapshot(
            provider: .codex,
            primaryMetric: selected.primary.map(metric),
            secondaryMetric: selected.secondary.map(metric),
            availability: .available,
            sourceVersion: "codex-app-server",
            collectionStatus: .fresh,
            codexResetCredits: resetCredits(from: result.rateLimitResetCredits)
        )
    }

    private func resetCredits(
        from summary: CodexRateLimitResetCreditsSummary?
    ) -> CodexResetCreditsSummary? {
        guard let summary else { return nil }
        let available = summary.credits?
            .filter { $0.status == "available" }
            .map {
                CodexResetCreditDisplay(
                    title: $0.title,
                    expiresAt: $0.expiresAt.map {
                        Date(timeIntervalSince1970: TimeInterval($0))
                    }
                )
            } ?? []
        return CodexResetCreditsSummary(
            availableCount: max(summary.availableCount, 0),
            credits: available,
            hasCompleteDetails: summary.credits != nil && available.count >= summary.availableCount
        )
    }

    private func metric(from window: CodexRateLimitWindow) -> UsageMetric {
        UsageMetric(
            label: label(for: window.windowDurationMins),
            current: window.usedPercent,
            limit: 100,
            unit: .percent,
            resetAt: window.resetsAt.map { Date(timeIntervalSince1970: TimeInterval($0)) }
        )
    }

    private func label(for minutes: Int64?) -> String {
        switch minutes {
        case 300: "5h limit"
        case 10_080: "Weekly limit"
        case let value?: "\(value)m limit"
        case nil: "Usage limit"
        }
    }
}

private enum CodexRequestParameters: Sendable {
    case null
    case refreshToken(Bool)

    var jsonObject: Any {
        switch self {
        case .null: NSNull()
        case .refreshToken(let refreshToken): ["refreshToken": refreshToken]
        }
    }
}

private struct CodexRateLimitsEnvelope: Decodable {
    let result: CodexRateLimitsResult?
}

struct CodexAccountEnvelope: Decodable, Sendable {
    let result: CodexAccountResult?
}

struct CodexAccountResult: Decodable, Sendable {
    let account: CodexAccount?
    let requiresOpenaiAuth: Bool

    private enum CodingKeys: String, CodingKey {
        case account
        case requiresOpenaiAuth
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        guard container.contains(.account) else {
            throw DecodingError.keyNotFound(
                CodingKeys.account,
                .init(codingPath: decoder.codingPath, debugDescription: "Missing Codex account field")
            )
        }
        account = try container.decodeIfPresent(CodexAccount.self, forKey: .account)
        requiresOpenaiAuth = try container.decode(Bool.self, forKey: .requiresOpenaiAuth)
    }
}

enum CodexAccount: Decodable, Sendable {
    case chatGPT(email: String?, planType: String?)
    case apiKey

    private enum CodingKeys: String, CodingKey {
        case type
        case email
        case planType
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(String.self, forKey: .type) {
        case "chatgpt":
            self = .chatGPT(
                email: try container.decodeIfPresent(String.self, forKey: .email),
                planType: try container.decodeIfPresent(String.self, forKey: .planType)
            )
        case "apiKey":
            self = .apiKey
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .type,
                in: container,
                debugDescription: "Unsupported Codex account type"
            )
        }
    }
}

private struct CodexRateLimitsResult: Decodable {
    let rateLimits: CodexRateLimitSnapshot
    let rateLimitsByLimitId: [String: CodexRateLimitSnapshot]?
    let rateLimitResetCredits: CodexRateLimitResetCreditsSummary?
}

private struct CodexRateLimitResetCreditsSummary: Decodable {
    let availableCount: Int
    let credits: [CodexRateLimitResetCredit]?
}

private struct CodexRateLimitResetCredit: Decodable {
    let status: String
    let title: String?
    let expiresAt: Int64?
}

private struct CodexRateLimitSnapshot: Decodable {
    let primary: CodexRateLimitWindow?
    let secondary: CodexRateLimitWindow?
}

private struct CodexRateLimitWindow: Decodable {
    let usedPercent: Double
    let windowDurationMins: Int64?
    let resetsAt: Int64?
}

private final class CodexProcessBox: @unchecked Sendable {
    private let lock = NSLock()
    private var process: Process?
    private var input: FileHandle?
    private var output: FileHandle?
    private var isStopRequested = false
    private var didRequestTimeout = false

    var timeoutWasRequested: Bool {
        lock.withLock { didRequestTimeout }
    }

    func set(process: Process, input: FileHandle, output: FileHandle) {
        let stopImmediately = lock.withLock {
            self.process = process
            self.input = input
            self.output = output
            return isStopRequested
        }
        if stopImmediately { stop() }
    }

    func clear(process: Process) {
        lock.withLock {
            if self.process === process {
                self.process = nil
                self.input = nil
                self.output = nil
            }
        }
    }

    func stop() {
        let state = lock.withLock {
            isStopRequested = true
            return (process, input, output)
        }
        try? state.1?.close()
        try? state.2?.close()
        guard let process = state.0, process.isRunning else { return }
        process.terminate()
        let processIdentifier = process.processIdentifier
        DispatchQueue.global().asyncAfter(deadline: .now() + 0.5) {
            if process.isRunning {
                kill(processIdentifier, SIGKILL)
            }
        }
    }

    func stopForTimeout() {
        lock.withLock { didRequestTimeout = true }
        stop()
    }
}
