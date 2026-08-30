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
    @ObservationIgnored private var autoHideTask: Task<Void, Never>?
    @ObservationIgnored private var generation: UInt64 = 0
    @ObservationIgnored private var selectionPresentedAtUptime: TimeInterval?

    public init() {}

    public var selectionID: FloatingDetailSelectionID? {
        guard selectedProvider != nil,
              let selectionPresentedAtUptime else { return nil }
        return FloatingDetailSelectionID(
            generation: generation,
            presentedAtUptime: selectionPresentedAtUptime
        )
    }

    public func toggle(_ provider: UsageProvider, autoHideAfter duration: Duration) {
        if selectedProvider == provider {
            dismiss()
        } else {
            present(provider, autoHideAfter: duration)
        }
    }

    public func present(_ provider: UsageProvider, autoHideAfter duration: Duration) {
        autoHideTask?.cancel()
        generation &+= 1
        let taskGeneration = generation
        selectionPresentedAtUptime = ProcessInfo.processInfo.systemUptime
        setSelection(provider)
        autoHideTask = Task { [weak self] in
            do { try await Task.sleep(for: duration) } catch { return }
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

    private func setSelection(_ provider: UsageProvider?) {
        selectedProvider = provider
        onSelectionChange?(provider)
    }
}
