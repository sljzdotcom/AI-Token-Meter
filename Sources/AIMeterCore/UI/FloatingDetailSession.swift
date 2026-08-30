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
    @ObservationIgnored private var selectionPresentedAtUptime: TimeInterval?
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
        let taskGeneration = generation
        selectionPresentedAtUptime = ProcessInfo.processInfo.systemUptime
        setSelection(provider)
        guard !isShutdown,
              generation == taskGeneration,
              selectedProvider == provider else { return }
        let sleep = sleep
        autoHideTask = Task { [weak self] in
            do { try await sleep(duration) } catch { return }
            guard self?.generation == taskGeneration else { return }
            guard self?.selectedProvider == provider else { return }
            self?.dismiss()
        }
    }

    public func dismiss() {
        autoHideTask?.cancel()
        autoHideTask = nil
        generation &+= 1
        selectionPresentedAtUptime = nil
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
        selectionPresentedAtUptime = nil
        if selectedProvider != nil {
            setSelection(nil)
        }
        onSelectionChange = nil
    }

    private func setSelection(_ provider: UsageProvider?) {
        selectedProvider = provider
        onSelectionChange?(provider)
    }
}
