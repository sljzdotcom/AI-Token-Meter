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
        let processBox = CodexProcessBox()

        return try await withThrowingTaskGroup(of: UsageSnapshot.self) { group in
            group.addTask {
                do {
                    return try await Task.detached(priority: .utility) {
                        try readRateLimitsBlocking(executableURL: executableURL, processBox: processBox)
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

    private func readRateLimitsBlocking(
        executableURL: URL,
        processBox: CodexProcessBox
    ) throws -> UsageSnapshot {
        let process = Process()
        let terminationWaiter = ProcessTerminationWaiter()
        let input = Pipe()
        let output = Pipe()

        process.executableURL = executableURL
        process.arguments = ["app-server", "--stdio"]
        process.standardInput = input
        process.standardOutput = output
        process.standardError = FileHandle.nullDevice
        if !environmentOverrides.isEmpty {
            process.environment = ProcessInfo.processInfo.environment.merging(
                environmentOverrides,
                uniquingKeysWith: { _, override in override }
            )
        }
        terminationWaiter.attach(to: process)

        try process.run()
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
                "clientInfo": ["name": "ai-meter", "version": "0.1.0"],
                "capabilities": ["experimentalApi": true],
            ],
        ], to: input.fileHandleForWriting)
        _ = try response(withID: 1, from: output.fileHandleForReading)

        try writeJSON(["method": "initialized"], to: input.fileHandleForWriting)
        try writeJSON([
            "id": 2,
            "method": "account/rateLimits/read",
            "params": NSNull(),
        ], to: input.fileHandleForWriting)

        let responseData = try response(withID: 2, from: output.fileHandleForReading)
        let envelope = try JSONDecoder().decode(CodexRateLimitsEnvelope.self, from: responseData)
        guard let result = envelope.result else {
            throw UsageCollectionError.invalidResponse
        }
        return snapshot(from: result)
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
        var selected = result.rateLimits
        if selected.secondary == nil,
           let richer = result.rateLimitsByLimitId?
            .sorted(by: { $0.key < $1.key })
            .map(\.value)
            .first(where: { $0.secondary != nil }) {
            selected = richer
        }

        return UsageSnapshot(
            provider: .codex,
            primaryMetric: selected.primary.map(metric),
            secondaryMetric: selected.secondary.map(metric),
            availability: .available,
            sourceVersion: "codex-app-server",
            collectionStatus: .fresh
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

private struct CodexRateLimitsEnvelope: Decodable {
    let result: CodexRateLimitsResult?
}

private struct CodexRateLimitsResult: Decodable {
    let rateLimits: CodexRateLimitSnapshot
    let rateLimitsByLimitId: [String: CodexRateLimitSnapshot]?
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
