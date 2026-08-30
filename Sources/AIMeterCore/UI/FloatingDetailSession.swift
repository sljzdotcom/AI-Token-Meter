import Foundation
import Observation

public struct FloatingDetailSelectionID: Equatable, Sendable {
    let generation: UInt64
    let presentedAtUptime: TimeInterval
}

@MainActor
@Observable
public final class FloatingDetailSession {
    public private(set) var selectedProvider: UsageProvider?
    @ObservationIgnored public var onSelectionChange: ((UsageProvider?) -> Void)?
    @ObservationIgnored private let sleep: @Sendable (Duration) async throws -> Void
    @ObservationIgnored private var autoHideTask: Task<Void, Never>?
    @ObservationIgnored private var generation: UInt64 = 0
    @ObservationIgnored private var timerGeneration: UInt64 = 0
    @ObservationIgnored private var selectionPresentedAtUptime: TimeInterval?
    @ObservationIgnored private var lastAutoHideDuration: Duration?
    @ObservationIgnored private var isAutoHidePaused = false
    @ObservationIgnored private var isShutdown = false

    public init() {
        sleep = { duration in
            try await Task.sleep(for: duration)
        }
    }

    init(_ sleep: @escaping @Sendable (Duration) async throws -> Void) {
        self.sleep = sleep
    }

    public var selectionID: FloatingDetailSelectionID? {
        guard selectedProvider != nil,
              let selectionPresentedAtUptime else { return nil }
        return FloatingDetailSelectionID(
            generation: generation,
            presentedAtUptime: selectionPresentedAtUptime
        )
    }

    public func accessibilityValue(for provider: UsageProvider) -> String {
        selectedProvider == provider ? "Detail open" : "Detail closed"
    }

    public func toggle(_ provider: UsageProvider, autoHideAfter duration: Duration) {
        if selectedProvider == provider {
            dismiss()
        } else {
            present(provider, autoHideAfter: duration)
        }
    }

    public func present(_ provider: UsageProvider, autoHideAfter duration: Duration) {
        guard !isShutdown else { return }
        autoHideTask?.cancel()
        generation &+= 1
        timerGeneration &+= 1
        selectionPresentedAtUptime = ProcessInfo.processInfo.systemUptime
        lastAutoHideDuration = duration
        isAutoHidePaused = false
        setSelection(provider)
        scheduleAutoHide(for: provider, after: duration)
    }

    public func setAutoHidePaused(
        _ isPaused: Bool,
        restartAfter duration: Duration? = nil
    ) {
        guard !isShutdown, let provider = selectedProvider else { return }
        isAutoHidePaused = isPaused
        timerGeneration &+= 1
        autoHideTask?.cancel()
        autoHideTask = nil
        guard !isPaused, let duration = duration ?? lastAutoHideDuration else { return }
        lastAutoHideDuration = duration
        scheduleAutoHide(for: provider, after: duration)
    }

    public func dismiss() {
        autoHideTask?.cancel()
        autoHideTask = nil
        generation &+= 1
        timerGeneration &+= 1
        selectionPresentedAtUptime = nil
        lastAutoHideDuration = nil
        isAutoHidePaused = false
        setSelection(nil)
    }

    public func dismiss(ifCurrent selectionID: FloatingDetailSelectionID) {
        guard self.selectionID == selectionID else { return }
        dismiss()
    }

    public func shutdown() {
        guard !isShutdown else { return }
        isShutdown = true
        autoHideTask?.cancel()
        autoHideTask = nil
        generation &+= 1
        timerGeneration &+= 1
        selectionPresentedAtUptime = nil
        lastAutoHideDuration = nil
        isAutoHidePaused = false
        if selectedProvider != nil {
            setSelection(nil)
        }
        onSelectionChange = nil
    }

    private func setSelection(_ provider: UsageProvider?) {
        selectedProvider = provider
        onSelectionChange?(provider)
    }

    private func scheduleAutoHide(for provider: UsageProvider, after duration: Duration) {
        guard !isShutdown, !isAutoHidePaused, selectedProvider == provider else { return }
        let taskTimerGeneration = timerGeneration
        let sleep = sleep
        autoHideTask = Task { [weak self] in
            do { try await sleep(duration) } catch { return }
            guard self?.timerGeneration == taskTimerGeneration else { return }
            guard self?.selectedProvider == provider else { return }
            guard self?.isAutoHidePaused == false else { return }
            self?.dismiss()
        }
    }
}
