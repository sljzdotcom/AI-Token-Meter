import Foundation
import Observation

@MainActor
@Observable
public final class FloatingDetailSession {
    public private(set) var selectedProvider: UsageProvider?
    @ObservationIgnored public var onSelectionChange: ((UsageProvider?) -> Void)?
    @ObservationIgnored private var autoHideTask: Task<Void, Never>?
    @ObservationIgnored private var generation: UInt64 = 0

    public init() {}

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
        setSelection(nil)
    }

    private func setSelection(_ provider: UsageProvider?) {
        selectedProvider = provider
        onSelectionChange?(provider)
    }
}
