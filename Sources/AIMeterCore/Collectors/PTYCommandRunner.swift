import Darwin
import Foundation

public struct PTYCommandRunner: CommandRunning {
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
        guard openpty(&masterDescriptor, &slaveDescriptor, nil, nil, &windowSize) == 0 else {
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
        terminationWaiter.attach(to: process)

        do {
            try process.run()
        } catch {
            close(activeMasterDescriptor)
            close(slaveDescriptor)
            throw error
        }

        beforeProcessRegistration?()
        processBox.set(process, descriptorBox: descriptorBox)
        close(slaveDescriptor)
        slaveDescriptor = -1

        let reader = Task.detached(priority: .utility) {
            Self.readPTY(activeMasterDescriptor, controller: descriptorBox)
        }

        let input = request.inputLines.joined(separator: "\n") + "\n"
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
        controller: ClosableDescriptor
    ) -> Data {
        var result = Data()
        var buffer = [UInt8](repeating: 0, count: 4_096)
        var remainingStopDrainBytes: Int?

        while true {
            if remainingStopDrainBytes == nil, controller.stopRequested {
                remainingStopDrainBytes = 256 * 1_024
            }
            let count = Darwin.read(descriptor, &buffer, buffer.count)
            if count > 0 {
                result.append(buffer, count: count)
                if let remaining = remainingStopDrainBytes {
                    let updated = remaining - count
                    remainingStopDrainBytes = updated
                    if updated <= 0 { break }
                }
            } else if count < 0 && (errno == EAGAIN || errno == EWOULDBLOCK) {
                if controller.stopRequested { break }
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
    private let group = DispatchGroup()
    private let lock = NSLock()
    private var exitCode: Int32?

    init() {
        group.enter()
    }

    func attach(to process: Process) {
        process.terminationHandler = { [self] process in
            let shouldSignal = lock.withLock {
                guard exitCode == nil else { return false }
                exitCode = process.terminationStatus
                return true
            }
            if shouldSignal { group.leave() }
        }
    }

    func value() async -> Int32 {
        await Task.detached(priority: .utility) { [self] in
            wait()
        }.value
    }

    func wait() -> Int32 {
        group.wait()
        return lock.withLock { exitCode ?? -1 }
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
