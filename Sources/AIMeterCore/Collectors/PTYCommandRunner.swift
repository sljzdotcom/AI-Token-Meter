import Darwin
import Foundation

public struct PTYCommandRunner: CommandRunning {
    private static let allocationLock = NSLock()

    private let beforeProcessRegistration: (@Sendable () -> Void)?

    public init() {
        beforeProcessRegistration = nil
    }

    init(beforeProcessRegistration: @escaping @Sendable () -> Void) {
        self.beforeProcessRegistration = beforeProcessRegistration
    }

    public func run(_ request: CommandRequest) async throws -> CommandResult {
        let processBox = RunningProcessBox()

        return try await withTaskCancellationHandler {
        try await withThrowingTaskGroup(of: CommandResult.self) { group in
            group.addTask {
                try await execute(request, processBox: processBox)
            }
            group.addTask {
                try await Task.sleep(for: .seconds(request.timeout))
                processBox.stop()
                throw UsageCollectionError.timedOut
            }

            defer { group.cancelAll() }
            guard let firstResult = try await group.next() else {
                throw UsageCollectionError.transportFailure
            }
            try Task.checkCancellation()
            return firstResult
        }
        } onCancel: { processBox.stop() }
    }

    private func execute(
        _ request: CommandRequest,
        processBox: RunningProcessBox
    ) async throws -> CommandResult {
        var masterDescriptor: Int32 = -1
        var slaveDescriptor: Int32 = -1
        var windowSize = winsize(ws_row: request.geminiQuotaInteraction ? 50 : 40, ws_col: request.geminiQuotaInteraction ? 140 : 120, ws_xpixel: 0, ws_ypixel: 0)
        let allocationResult = Self.allocationLock.withLock {
            openpty(&masterDescriptor, &slaveDescriptor, nil, nil, &windowSize)
        }
        guard allocationResult == 0 else {
            throw UsageCollectionError.transportFailure
        }
        let activeMasterDescriptor = masterDescriptor
        let descriptorBox = ClosableDescriptor(activeMasterDescriptor)
        let currentFlags = fcntl(activeMasterDescriptor, F_GETFL)
        _ = fcntl(activeMasterDescriptor, F_SETFL, currentFlags | O_NONBLOCK)

        let process = Process()
        let terminationWaiter = ProcessTerminationWaiter {
            descriptorBox.requestStop()
        }
        let startedAt = Date()
        let slaveHandle = FileHandle(fileDescriptor: slaveDescriptor, closeOnDealloc: false)

        process.executableURL = request.executableURL
        process.arguments = request.arguments
        process.standardInput = slaveHandle
        process.standardOutput = slaveHandle
        process.standardError = slaveHandle
        process.environment = request.environment ?? controlledEnvironment()
        process.currentDirectoryURL = request.currentDirectoryURL
        terminationWaiter.attach(to: process)

        do {
            try process.run()
        } catch {
            close(activeMasterDescriptor)
            close(slaveDescriptor)
            throw error
        }
        terminationWaiter.beginFallbackWait(for: process)

        beforeProcessRegistration?()
        processBox.set(process, descriptorBox: descriptorBox)

        let readerStarted = AsyncOneShotSignal()
        let reader = Task.detached(priority: .userInitiated) {
            await Self.readPTY(
                activeMasterDescriptor,
                controller: descriptorBox,
                processBox: processBox,
                stopAfterOutputContains: request.stopAfterOutputContains,
                geminiQuotaInteraction: request.geminiQuotaInteraction,
                started: readerStarted
            )
        }
        await readerStarted.value()

        if request.inputDelay > 0 {
            try? await Task.sleep(for: .seconds(request.inputDelay))
        }
        if !request.geminiQuotaInteraction && !(request.environment != nil && request.inputLines.isEmpty) {
        let terminator = request.inputLineTerminator
        let input = request.inputLines.joined(separator: terminator) + terminator
        let bytes = Array(input.utf8)
        bytes.withUnsafeBytes { buffer in
            var written = 0
            while written < buffer.count {
                let count = Darwin.write(
                    activeMasterDescriptor,
                    buffer.baseAddress!.advanced(by: written),
                    buffer.count - written
                )
                if count <= 0 { break }
                written += count
            }
        }

        }
        let exitCode = await terminationWaiter.value()

        descriptorBox.requestStop()
        try? await Task.sleep(for: .milliseconds(100))
        close(slaveDescriptor)
        slaveDescriptor = -1
        let outputData = await reader.value
        descriptorBox.close()
        processBox.clear(process)
        if let error = outputData.error { throw error }
        let output = String(decoding: outputData.data, as: UTF8.self)

        return CommandResult(
            output: output,
            exitCode: exitCode,
            duration: Date().timeIntervalSince(startedAt)
        )
    }

    private static func readPTY(
        _ descriptor: Int32,
        controller: ClosableDescriptor,
        processBox: RunningProcessBox,
        stopAfterOutputContains stopPhrases: [String],
        geminiQuotaInteraction: Bool,
        started: AsyncOneShotSignal
    ) async -> PTYReadResult {
        var result = Data()
        var buffer = [UInt8](repeating: 0, count: 4_096)
        var drain = PTYReadDrainState()
        var matchedStopPhrase = false
        var interaction = GeminiPTYInteraction()
        started.signal()

        readLoop: while true {
            if geminiQuotaInteraction && !controller.stopRequested {
                interaction.advance(descriptor: descriptor)
                if interaction.error != nil { processBox.stop() }
            }
            let count = Darwin.read(descriptor, &buffer, buffer.count)
            let readError = count < 0 ? errno : 0
            if count > 0 {
                result.append(buffer, count: count)
                if geminiQuotaInteraction {
                    interaction.receive(Data(buffer.prefix(count)))
                    if interaction.error != nil { processBox.stop() }
                }
                if !matchedStopPhrase, !stopPhrases.isEmpty {
                    let output = String(decoding: result, as: UTF8.self)
                    if stopPhrases.contains(where: output.contains) {
                        matchedStopPhrase = true
                        processBox.stop()
                    }
                }
            }

            let observation: PTYReadObservation
            if count > 0 {
                observation = .bytes(count)
            } else if count == 0 || readError == EIO {
                observation = .terminalClosed
            } else if readError == EAGAIN || readError == EWOULDBLOCK || readError == EINTR {
                observation = .noData
            } else {
                observation = .unrecoverableError
            }

            switch drain.observe(
                observation,
                stopRequested: controller.stopRequested,
                now: ProcessInfo.processInfo.systemUptime
            ) {
            case .keepReading:
                continue
            case .wait:
                try? await Task.sleep(for: .milliseconds(10))
            case .finish:
                break readLoop
            }
        }
        if geminiQuotaInteraction { interaction.finish() }
        return PTYReadResult(data: interaction.captured.map { Data($0.utf8) } ?? result, error: interaction.error)
    }

    private func controlledEnvironment() -> [String: String] {
        let source = ProcessInfo.processInfo.environment
        var environment: [String: String] = [:]
        for key in ["PATH", "HOME", "USER", "LANG", "LC_ALL", "TERM"] {
            if let value = source[key] {
                environment[key] = value
            }
        }
        environment["PATH"] = environment["PATH"] ?? "/usr/bin:/bin:/usr/sbin:/sbin"
        environment["TERM"] = environment["TERM"] ?? "xterm-256color"
        return environment
    }
}

final class ProcessTerminationWaiter: @unchecked Sendable {
    static let fallbackWaitQoSClass = DispatchQoS.QoSClass.userInitiated

    private static let fallbackQueue = DispatchQueue(
        label: "com.millerpan.AIMeter.process-termination",
        qos: DispatchQoS(qosClass: fallbackWaitQoSClass, relativePriority: 0),
        attributes: .concurrent
    )

    private let lock = NSLock()
    private let blockingSignal = DispatchSemaphore(value: 0)
    private let onExit: (@Sendable () -> Void)?
    private var exitCode: Int32?
    private var continuation: CheckedContinuation<Int32, Never>?
    private var fallbackWaitStarted = false
    private var isCompleting = false

    init(onExit: (@Sendable () -> Void)? = nil) {
        self.onExit = onExit
    }

    func attach(to process: Process) {
        process.terminationHandler = { [self] process in
            complete(with: process.terminationStatus)
        }
    }

    func beginFallbackWait(for process: Process) {
        let shouldStart = lock.withLock {
            guard exitCode == nil, !fallbackWaitStarted else { return false }
            fallbackWaitStarted = true
            return true
        }
        guard shouldStart else { return }

        Self.fallbackQueue.async { [self] in
            process.waitUntilExit()
            complete(with: process.terminationStatus)
        }
    }

    func value() async -> Int32 {
        await withCheckedContinuation { pendingContinuation in
            let completedExitCode: Int32? = lock.withLock {
                if let exitCode { return exitCode }
                continuation = pendingContinuation
                return nil
            }
            if let completedExitCode {
                pendingContinuation.resume(returning: completedExitCode)
            }
        }
    }

    func wait() -> Int32 {
        if let completedExitCode = lock.withLock({ exitCode }) {
            return completedExitCode
        }
        blockingSignal.wait()
        return lock.withLock { exitCode ?? -1 }
    }

    private func complete(with status: Int32) {
        let claimedCompletion = lock.withLock {
            guard exitCode == nil, !isCompleting else { return false }
            isCompleting = true
            return true
        }
        guard claimedCompletion else { return }

        onExit?()
        let continuationToResume = lock.withLock {
            exitCode = status
            isCompleting = false
            defer { continuation = nil }
            return continuation
        }
        blockingSignal.signal()
        continuationToResume?.resume(returning: status)
    }
}

private final class RunningProcessBox: @unchecked Sendable {
    private let lock = NSLock()
    private var process: Process?
    private var descriptorBox: ClosableDescriptor?
    private var isStopRequested = false

    func set(_ process: Process, descriptorBox: ClosableDescriptor) {
        let stopImmediately = lock.withLock {
            self.process = process
            self.descriptorBox = descriptorBox
            return isStopRequested
        }
        if stopImmediately { stop() }
    }

    func clear(_ process: Process) {
        lock.withLock {
            if self.process === process {
                self.process = nil
                self.descriptorBox = nil
            }
        }
    }

    func stop() {
        let state = lock.withLock {
            isStopRequested = true
            return (process, descriptorBox)
        }
        state.1?.requestStop()
        let runningProcess = state.0
        guard let runningProcess, runningProcess.isRunning else { return }

        runningProcess.terminate()
        let processIdentifier = runningProcess.processIdentifier
        DispatchQueue.global().asyncAfter(deadline: .now() + 0.5) {
            if runningProcess.isRunning {
                kill(processIdentifier, SIGKILL)
            }
        }
    }
}

private final class ClosableDescriptor: @unchecked Sendable {
    private let lock = NSLock()
    private var descriptor: Int32?
    private var isStopRequested = false

    init(_ descriptor: Int32) {
        self.descriptor = descriptor
    }

    var stopRequested: Bool {
        lock.withLock { isStopRequested }
    }

    func requestStop() {
        lock.withLock { isStopRequested = true }
    }

    func close() {
        let descriptorToClose = lock.withLock { () -> Int32? in
            defer { descriptor = nil }
            return descriptor
        }
        if let descriptorToClose {
            Darwin.close(descriptorToClose)
        }
    }
}

private final class AsyncOneShotSignal: @unchecked Sendable {
    private let lock = NSLock()
    private var isSignaled = false
    private var continuation: CheckedContinuation<Void, Never>?

    func signal() {
        let continuationToResume = lock.withLock {
            guard !isSignaled else { return nil as CheckedContinuation<Void, Never>? }
            isSignaled = true
            defer { continuation = nil }
            return continuation
        }
        continuationToResume?.resume()
    }

    func value() async {
        await withCheckedContinuation { pendingContinuation in
            let resumeImmediately = lock.withLock {
                if isSignaled { return true }
                continuation = pendingContinuation
                return false
            }
            if resumeImmediately {
                pendingContinuation.resume()
            }
        }
    }
}

private struct PTYReadResult {
    let data: Data
    let error: UsageCollectionError?
}

enum PTYReadObservation: Equatable {
    case bytes(Int)
    case noData
    case terminalClosed
    case unrecoverableError
}

enum PTYReadAction: Equatable {
    case keepReading
    case wait
    case finish
}

/// Keeps transient terminal closure separate from confirmed process exit.
/// A macOS PTY master can report EOF while no slave is open, then receive data if the slave reopens.
struct PTYReadDrainState {
    private static let byteLimit = 256 * 1_024
    private static let maximumDuration: TimeInterval = 0.75
    private static let terminalQuietDuration: TimeInterval = 0.1

    private var remainingBytes: Int?
    private var deadline: TimeInterval?
    private var terminalQuietDeadline: TimeInterval?

    mutating func observe(
        _ observation: PTYReadObservation,
        stopRequested: Bool,
        now: TimeInterval
    ) -> PTYReadAction {
        if stopRequested, remainingBytes == nil {
            remainingBytes = Self.byteLimit
            deadline = now + Self.maximumDuration
        }
        if let deadline, now >= deadline { return .finish }

        switch observation {
        case let .bytes(count):
            terminalQuietDeadline = nil
            guard let remainingBytes else { return .keepReading }
            let updated = remainingBytes - count
            self.remainingBytes = updated
            return updated <= 0 ? .finish : .keepReading

        case .noData:
            if let deadline, now >= deadline { return .finish }
            return .wait

        case .terminalClosed:
            guard let deadline else {
                terminalQuietDeadline = nil
                return .wait
            }
            if now >= deadline { return .finish }
            let quietDeadline = terminalQuietDeadline ?? min(deadline, now + Self.terminalQuietDuration)
            terminalQuietDeadline = quietDeadline
            return now >= quietDeadline ? .finish : .wait

        case .unrecoverableError:
            return .finish
        }
    }
}

/// Lives on the single reader task, so input scheduling never blocks draining output.
private struct GeminiPTYInteraction {
    private var terminal = GeminiTerminalObservation()
    private var interactionError: UsageCollectionError?
    var error: UsageCollectionError? { terminal.error ?? interactionError }
    var captured: String?
    private var sentModel = false
    private var closing = false
    private var pending: [(TimeInterval, UInt8)] = []
    private var previousScreen = ""
    private var unchangedSince: TimeInterval = 0

    mutating func receive(_ data: Data) { terminal.receive(data) }
    mutating func finish() { terminal.finish() }

    mutating func advance(descriptor: Int32) {
        guard error == nil else { return }
        let now = ProcessInfo.processInfo.systemUptime
        let text = terminal.text
        if text != previousScreen { previousScreen = text; unchangedSince = now }
        if let blocking = GeminiTerminalProtocol.blockingError(text) { interactionError = blocking; pending = []; return }
        if !sentModel && GeminiTerminalProtocol.isReady(text) && now - unchangedSince >= 0.15 {
            schedule("/model\r", at: now); sentModel = true
        }
        if sentModel && !closing && pending.isEmpty && now - unchangedSince >= 0.3 && text.contains("Select Model") && text.contains("(Press Esc to close)") && text.contains("╯") {
            // Validate before closing; the collector receives this captured frame, not the exit screen.
            do { _ = try GeminiUsageParser().parse(text) }
            catch let failure as UsageCollectionError {
                if case .geminiUnavailable = failure { /* close an empty quota dialog normally */ }
                else { interactionError = failure; return }
            } catch { interactionError = .unrecognizedOutput; return }
            captured = text; closing = true
            schedule("\u{1b}", at: now)
            schedule("/quit\r", at: now + 0.8)
        }
        if let next = pending.first, next.0 <= now {
            var byte = next.1
            if Darwin.write(descriptor, &byte, 1) == 1 { pending.removeFirst() }
            else if errno != EAGAIN && errno != EWOULDBLOCK { interactionError = .transportFailure }
        }
    }
    private mutating func schedule(_ text: String, at start: TimeInterval) {
        var time = start
        for byte in text.utf8 {
            if byte == 13 { time += 0.3 }
            pending.append((time, byte)); time += 0.1
        }
    }
}
