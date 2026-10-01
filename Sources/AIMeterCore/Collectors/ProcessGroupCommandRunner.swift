import Darwin
import Foundation

/// Runs a command in its own process group so timeout and cancellation can clean up
/// children started by that command without targeting unrelated processes.
public struct ProcessGroupCommandRunner: CommandRunning {
    public init() {}

    public func run(_ request: CommandRequest) async throws -> CommandResult {
        let state = ProcessGroupCommandState()
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
        state: ProcessGroupCommandState
    ) async throws -> CommandResult {
        var outputPipe: [Int32] = [0, 0]
        guard pipe(&outputPipe) == 0 else { throw UsageCollectionError.transportFailure }
        var outputOpen = true
        defer {
            if outputOpen {
                close(outputPipe[0])
                close(outputPipe[1])
            }
        }

        let nullInput = open("/dev/null", O_RDONLY)
        guard nullInput >= 0 else { throw UsageCollectionError.transportFailure }
        defer { close(nullInput) }

        var actions: posix_spawn_file_actions_t?
        guard posix_spawn_file_actions_init(&actions) == 0 else {
            throw UsageCollectionError.transportFailure
        }
        defer { posix_spawn_file_actions_destroy(&actions) }
        guard posix_spawn_file_actions_adddup2(&actions, nullInput, STDIN_FILENO) == 0,
              posix_spawn_file_actions_adddup2(&actions, outputPipe[1], STDOUT_FILENO) == 0,
              posix_spawn_file_actions_adddup2(&actions, outputPipe[1], STDERR_FILENO) == 0,
              posix_spawn_file_actions_addclose(&actions, outputPipe[0]) == 0,
              posix_spawn_file_actions_addclose(&actions, outputPipe[1]) == 0 else {
            throw UsageCollectionError.transportFailure
        }
        if let directory = request.currentDirectoryURL,
           posix_spawn_file_actions_addchdir_np(&actions, directory.path) != 0 {
            throw UsageCollectionError.transportFailure
        }

        var attributes: posix_spawnattr_t?
        guard posix_spawnattr_init(&attributes) == 0 else {
            throw UsageCollectionError.transportFailure
        }
        defer { posix_spawnattr_destroy(&attributes) }
        guard posix_spawnattr_setflags(&attributes, Int16(POSIX_SPAWN_SETSID)) == 0 else {
            throw UsageCollectionError.transportFailure
        }

        let executable = request.executableURL.path
        let arguments = [executable] + request.arguments
        let environment = request.environment ?? ProcessInfo.processInfo.environment
        let argumentPointers = arguments.map { strdup($0) } + [nil]
        let environmentPointers = environment.map { strdup("\($0.key)=\($0.value)") } + [nil]
        defer {
            argumentPointers.compactMap { $0 }.forEach { free($0) }
            environmentPointers.compactMap { $0 }.forEach { free($0) }
        }

        var processID: pid_t = 0
        let spawnError = argumentPointers.withUnsafeBufferPointer { argv in
            environmentPointers.withUnsafeBufferPointer { envp in
                posix_spawn(
                    &processID,
                    executable,
                    &actions,
                    &attributes,
                    argv.baseAddress!,
                    envp.baseAddress!
                )
            }
        }
        guard spawnError == 0 else { throw UsageCollectionError.transportFailure }

        close(outputPipe[1])
        outputOpen = false
        state.register(processID)

        let reader = FileHandle(fileDescriptor: outputPipe[0], closeOnDealloc: true)
        let buffer = ProcessGroupOutput(limit: request.maxOutputBytes) {
            state.stop(reason: .outputLimitExceeded)
        }
        reader.readabilityHandler = { handle in
            buffer.receive(handle.availableData)
        }

        let startedAt = Date()
        let waiter = ProcessGroupWaiter(processID: processID)
        let observedExit = await Task.detached(priority: .utility) { waiter.waitWithoutReaping() }.value
        if !observedExit { state.stop(reason: .completedWithChildren) }
        let didDrain = await buffer.waitForEOF()
        if !didDrain {
            state.stop(reason: .completedWithChildren)
            _ = await buffer.waitForEOF()
        }
        reader.readabilityHandler = nil
        try? reader.close()
        await state.finish(processID)
        let status = await Task.detached(priority: .utility) { waiter.reap() }.value

        if let reason = state.reason {
            switch reason {
            case .timedOut: throw UsageCollectionError.timedOut
            case .outputLimitExceeded: throw UsageCollectionError.unrecognizedOutput
            case .cancelled: throw CancellationError()
            case .completedWithChildren: break
            }
        }
        guard status & 0x7f == 0 else { throw UsageCollectionError.transportFailure }
        return CommandResult(
            output: buffer.text,
            exitCode: (status >> 8) & 0xff,
            duration: Date().timeIntervalSince(startedAt)
        )
    }
}

private enum ProcessGroupStopReason {
    case timedOut
    case outputLimitExceeded
    case cancelled
    case completedWithChildren
}

private final class ProcessGroupWaiter: @unchecked Sendable {
    private let processID: pid_t

    init(processID: pid_t) { self.processID = processID }

    /// Observe leader exit but keep it unreaped while descendants may still hold the pipe.
    /// Its PID therefore cannot be reused during delayed process-group escalation.
    func waitWithoutReaping() -> Bool {
        var info = siginfo_t()
        while waitid(P_PID, id_t(processID), &info, WEXITED | WNOWAIT) == -1 {
            if errno == EINTR { continue }
            return false
        }
        return true
    }

    func reap() -> Int32 {
        var status: Int32 = 0
        while waitpid(processID, &status, 0) == -1, errno == EINTR {}
        return status
    }
}

private final class ProcessGroupCommandState: @unchecked Sendable {
    private let lock = NSLock()
    private var processID: pid_t?
    private var stopReason: ProcessGroupStopReason?
    private var escalationScheduled = false
    private var escalationTask: Task<Void, Never>?
    private var isFinishing = false

    var reason: ProcessGroupStopReason? { lock.withLock { stopReason } }

    func register(_ processID: pid_t) {
        lock.withLock {
            self.processID = processID
            guard stopReason != nil, !escalationScheduled else { return }
            escalationScheduled = true
            scheduleEscalationLocked(for: processID)
        }
    }

    func clear(_ processID: pid_t) {
        lock.withLock {
            if self.processID == processID { self.processID = nil }
        }
    }

    func stop(reason: ProcessGroupStopReason) {
        lock.withLock {
            guard !isFinishing else { return }
            if stopReason == nil { stopReason = reason }
            guard !escalationScheduled, let processID else { return }
            escalationScheduled = true
            scheduleEscalationLocked(for: processID)
        }
    }

    private func scheduleEscalationLocked(for processID: pid_t) {
        _ = kill(-processID, SIGTERM)
        escalationTask = Task.detached(priority: .utility) {
            try? await Task.sleep(for: .milliseconds(500))
            guard self.owns(processID), kill(-processID, 0) == 0 else { return }
            _ = kill(-processID, SIGKILL)
        }
    }

    private func owns(_ processID: pid_t) -> Bool {
        lock.withLock { self.processID == processID }
    }

    func finish(_ processID: pid_t) async {
        let task = lock.withLock { () -> Task<Void, Never>? in
            guard self.processID == processID else { return nil }
            isFinishing = true
            return escalationTask
        }
        await task?.value
        lock.withLock {
            if self.processID == processID { self.processID = nil }
        }
    }
}

private final class ProcessGroupOutput: @unchecked Sendable {
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

    func waitForEOF() async -> Bool {
        for _ in 0..<50 {
            if lock.withLock({ reachedEOF }) { return true }
            try? await Task.sleep(for: .milliseconds(10))
        }
        return lock.withLock { reachedEOF }
    }
}
