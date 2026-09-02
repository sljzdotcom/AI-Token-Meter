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

        return try await withThrowingTaskGroup(of: CommandResult.self) { group in
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
            return firstResult
        }
    }

    private func execute(
        _ request: CommandRequest,
        processBox: RunningProcessBox
    ) async throws -> CommandResult {
        var masterDescriptor: Int32 = -1
        var slaveDescriptor: Int32 = -1
        var windowSize = winsize(ws_row: 40, ws_col: 120, ws_xpixel: 0, ws_ypixel: 0)
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
        let terminationWaiter = ProcessTerminationWaiter()
        let startedAt = Date()
        let slaveHandle = FileHandle(fileDescriptor: slaveDescriptor, closeOnDealloc: false)

        process.executableURL = request.executableURL
        process.arguments = request.arguments
        process.standardInput = slaveHandle
        process.standardOutput = slaveHandle
        process.standardError = slaveHandle
        process.environment = controlledEnvironment()
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
        close(slaveDescriptor)
        slaveDescriptor = -1

        let reader = Task.detached(priority: .utility) {
            Self.readPTY(
                activeMasterDescriptor,
                controller: descriptorBox,
                processBox: processBox,
                stopAfterOutputContains: request.stopAfterOutputContains
            )
        }

        if request.inputDelay > 0 {
            try await Task.sleep(for: .seconds(request.inputDelay))
        }
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

        let exitCode = await terminationWaiter.value()

        descriptorBox.requestStop()
        let outputData = await reader.value
        descriptorBox.close()
        processBox.clear(process)
        let output = String(decoding: outputData, as: UTF8.self)

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
        stopAfterOutputContains stopPhrases: [String]
    ) -> Data {
        var result = Data()
        var buffer = [UInt8](repeating: 0, count: 4_096)
        var remainingStopDrainBytes: Int?
        var stopDrainDeadline: Date?
        var matchedStopPhrase = false

        while true {
            if remainingStopDrainBytes == nil, controller.stopRequested {
                remainingStopDrainBytes = 256 * 1_024
                stopDrainDeadline = Date().addingTimeInterval(0.75)
            }
            let count = Darwin.read(descriptor, &buffer, buffer.count)
            if count > 0 {
                result.append(buffer, count: count)
                if !matchedStopPhrase, !stopPhrases.isEmpty {
                    let output = String(decoding: result, as: UTF8.self)
                    if stopPhrases.contains(where: output.contains) {
                        matchedStopPhrase = true
                        processBox.stop()
                    }
                }
                if let remaining = remainingStopDrainBytes {
                    let updated = remaining - count
                    remainingStopDrainBytes = updated
                    if updated <= 0 { break }
                }
            } else if count < 0 && (errno == EAGAIN || errno == EWOULDBLOCK) {
                if let stopDrainDeadline {
                    if Date() >= stopDrainDeadline { break }
                }
                usleep(10_000)
            } else {
                break
            }
        }
        return result
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
    private static let fallbackQueue = DispatchQueue(
        label: "com.millerpan.AIMeter.process-termination",
        qos: .utility,
        attributes: .concurrent
    )

    private let lock = NSLock()
    private let blockingSignal = DispatchSemaphore(value: 0)
    private var exitCode: Int32?
    private var continuation: CheckedContinuation<Int32, Never>?
    private var fallbackWaitStarted = false

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
        let completion: (didComplete: Bool, continuation: CheckedContinuation<Int32, Never>?) = lock.withLock {
            guard exitCode == nil else { return (false, nil) }
            exitCode = status
            defer { continuation = nil }
            return (true, continuation)
        }
        guard completion.didComplete else { return }
        blockingSignal.signal()
        completion.continuation?.resume(returning: status)
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
