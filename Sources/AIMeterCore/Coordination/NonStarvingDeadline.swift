import Foundation

enum NonStarvingDeadline {
    private static let timeoutQueue = DispatchQueue(
        label: "com.millerpan.AIMeter.deadline",
        qos: .userInitiated
    )

    static func firstResult<Value: Sendable>(
        from task: Task<Value, Never>,
        timeout: Duration,
        timedOutValue: Value
    ) async -> Value {
        await withCheckedContinuation { continuation in
            let gate = LockedOneShotContinuation(continuation)
            Task.detached(priority: .userInitiated) {
                gate.resume(returning: await task.value)
            }
            timeoutQueue.asyncAfter(deadline: .now() + dispatchInterval(for: timeout)) {
                gate.resume(returning: timedOutValue)
            }
        }
    }

    private static func dispatchInterval(for duration: Duration) -> DispatchTimeInterval {
        let components = duration.components
        guard components.seconds >= 0, components.attoseconds >= 0 else {
            return .nanoseconds(0)
        }
        let nanosecondsPerSecond: Int64 = 1_000_000_000
        let nanosecondsFromAttoseconds = components.attoseconds / nanosecondsPerSecond
        let (wholeSeconds, overflow) = components.seconds.multipliedReportingOverflow(
            by: nanosecondsPerSecond
        )
        guard !overflow else { return .nanoseconds(Int.max) }
        let (total, additionOverflow) = wholeSeconds.addingReportingOverflow(
            nanosecondsFromAttoseconds
        )
        guard !additionOverflow else { return .nanoseconds(Int.max) }
        return .nanoseconds(Int(clamping: total))
    }
}

private final class LockedOneShotContinuation<Value: Sendable>: @unchecked Sendable {
    private let lock = NSLock()
    private var continuation: CheckedContinuation<Value, Never>?

    init(_ continuation: CheckedContinuation<Value, Never>) {
        self.continuation = continuation
    }

    func resume(returning value: Value) {
        let continuation = lock.withLock {
            defer { self.continuation = nil }
            return self.continuation
        }
        continuation?.resume(returning: value)
    }
}
