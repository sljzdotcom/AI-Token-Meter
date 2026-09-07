import AIMeterCore

@MainActor protocol FloatingStripWindow: AnyObject {
    func dismissDetail()
    func showDetail(for provider: UsageProvider)
    func close()
}

/// Owns window lifetime only; never owns data collection or credentials.
@MainActor final class FloatingStripWindowRegistry<Window: FloatingStripWindow> {
    private(set) var windows: [String: Window] = [:]

    func reconcile(targets: [String], create: (String) -> Window) {
        let wanted = Set(targets)
        for identifier in Array(windows.keys) where !wanted.contains(identifier) {
            windows.removeValue(forKey: identifier)?.close()
        }
        for identifier in targets where windows[identifier] == nil {
            windows[identifier] = create(identifier)
        }
    }

    func present(_ provider: UsageProvider, on identifier: String) {
        guard let target = windows[identifier] else { return }
        for window in windows.values { window.dismissDetail() }
        target.showDetail(for: provider)
    }
}
