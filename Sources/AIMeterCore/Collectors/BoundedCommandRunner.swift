import Darwin
import Foundation

/// Runs a noninteractive command with pipes, bounded output, and deterministic cleanup.
public struct BoundedCommandRunner: CommandRunning {
    public init() {}

    public func run(_ request: CommandRequest) async throws -> CommandResult {
        let state = BoundedCommandState()
        return try await withTaskCancellationHandler {
            try await withThrowingTaskGroup(of: CommandResult.self) { group in
                group.addTask { try await execute(request, state: state) }
                group.addTask {
                    try await Task.sleep(for: .seconds(request.timeout))
                    state.stop(reason: .timedOut)
                    throw UsageCollectionError.timedOut
                }

                defer { group.cancelAll() }
                guard let result = try await group.next() else {
                    throw UsageCollectionError.transportFailure
                }
                try Task.checkCancellation()
                return result
            }
        } onCancel: {
            state.stop(reason: .cancelled)
        }
    }

    private func execute(
        _ request: CommandRequest,
        state: BoundedCommandState
    ) async throws -> CommandResult {
        let process = Process()
        let output = Pipe()
        let waiter = ProcessTerminationWaiter()
        let buffer = BoundedCommandOutput(limit: request.maxOutputBytes) {
            state.stop(reason: .outputLimitExceeded)
        }
        let startedAt = Date()

        process.executableURL = request.executableURL
        process.arguments = request.arguments
        process.standardInput = FileHandle.nullDevice
        process.standardOutput = output.fileHandleForWriting
        process.standardError = output.fileHandleForWriting
        process.environment = request.environment
        process.currentDirectoryURL = request.currentDirectoryURL
        waiter.attach(to: process)
        output.fileHandleForReading.readabilityHandler = { handle in
            buffer.receive(handle.availableData)
        }

        do {
            try process.run()
        } catch {
            output.fileHandleForReading.readabilityHandler = nil
            try? output.fileHandleForReading.close()
            try? output.fileHandleForWriting.close()
            throw UsageCollectionError.transportFailure
        }
        waiter.beginFallbackWait(for: process)
        state.register(process)
        try? output.fileHandleForWriting.close()

        let exitCode = await waiter.value()
        await buffer.waitForEOF()
        output.fileHandleForReading.readabilityHandler = nil
        try? output.fileHandleForReading.close()
        state.clear(process)

        if let reason = state.reason {
            switch reason {
            case .timedOut: throw UsageCollectionError.timedOut
            case .outputLimitExceeded: throw UsageCollectionError.unrecognizedOutput
            case .cancelled: throw CancellationError()
            }
        }
        return CommandResult(
            output: buffer.text,
            exitCode: exitCode,
            duration: Date().timeIntervalSince(startedAt)
        )
    }
}

private enum BoundedCommandStopReason {
    case timedOut
    case outputLimitExceeded
    case cancelled
}

private final class BoundedCommandState: @unchecked Sendable {
    private let lock = NSLock()
    private var process: Process?
    private var stopReason: BoundedCommandStopReason?

    var reason: BoundedCommandStopReason? { lock.withLock { stopReason } }

    func register(_ process: Process) {
        let shouldStop = lock.withLock {
            self.process = process
            return stopReason != nil
        }
        if shouldStop { terminate(process) }
    }

    func clear(_ process: Process) {
        lock.withLock {
            if self.process === process { self.process = nil }
        }
    }

    func stop(reason: BoundedCommandStopReason) {
        let process = lock.withLock {
            if stopReason == nil { stopReason = reason }
            return self.process
        }
        if let process { terminate(process) }
    }

    private func terminate(_ process: Process) {
        guard process.isRunning else { return }
        process.terminate()
        let processIdentifier = process.processIdentifier
        DispatchQueue.global().asyncAfter(deadline: .now() + 0.5) {
            if process.isRunning { kill(processIdentifier, SIGKILL) }
        }
    }
}

private final class BoundedCommandOutput: @unchecked Sendable {
    private let lock = NSLock()
    private let limit: Int
    private let onOverflow: @Sendable () -> Void
    private var data = Data()
    private var reachedEOF = false

    init(limit: Int, onOverflow: @escaping @Sendable () -> Void) {
        self.limit = max(0, limit)
        self.onOverflow = onOverflow
    }

    var text: String { lock.withLock { String(decoding: data, as: UTF8.self) } }

    func receive(_ chunk: Data) {
        if chunk.isEmpty {
            lock.withLock { reachedEOF = true }
            return
        }
        let overflow = lock.withLock {
            guard data.count + chunk.count <= limit else { return true }
            data.append(chunk)
            return false
        }
        if overflow { onOverflow() }
    }

    func waitForEOF() async {
        for _ in 0..<50 {
            if lock.withLock({ reachedEOF }) { return }
            try? await Task.sleep(for: .milliseconds(10))
        }
    }
}
