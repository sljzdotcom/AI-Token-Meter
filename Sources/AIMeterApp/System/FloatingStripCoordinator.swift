import AppKit
import AIMeterCore

struct FloatingStripDisplayChoice: Identifiable, Equatable {
    let id: String
    let name: String
    let isPrimary: Bool
    let isBuiltIn: Bool

    var title: String {
        name + (isBuiltIn ? " · Built-in" : " · External") + (isPrimary ? " · Primary" : "")
    }
}

@MainActor
final class FloatingStripCoordinator {
    private let model: AppModel
    private let registry = FloatingStripWindowRegistry<FloatingPanelController>()
    private var screenObserver: NSObjectProtocol?
    private var shouldShow = false

    init(model: AppModel) {
        self.model = model
        model.floatingAppearanceHandler = { [weak self] in
            guard let self else { return }
            for controller in registry.windows.values { controller.applyAppearance() }
        }
        model.floatingDisplaysHandler = { [weak self] in self?.reconcile() }
        screenObserver = NotificationCenter.default.addObserver(
            forName: NSApplication.didChangeScreenParametersNotification, object: nil, queue: .main
        ) { [weak self] _ in
            Task { @MainActor in self?.reconcile() }
        }
        reconcile()
    }

    isolated deinit {
        if let screenObserver { NotificationCenter.default.removeObserver(screenObserver) }
        registry.reconcile(targets: []) { _ in fatalError("No targets to create") }
    }

    func show() { shouldShow = true; reconcile() }
    func hide() { shouldShow = false; registry.windows.values.forEach { $0.hide() } }

    func showDetail(for provider: UsageProvider) {
        reconcile()
        let choices = model.availableStripDisplays
        let target = choices.first(where: { $0.isPrimary && registry.windows[$0.id] != nil })?.id
            ?? choices.first(where: { registry.windows[$0.id] != nil })?.id
        if let target { registry.present(provider, on: target) }
    }

    func applyUserPositionPreference() {
        for controller in registry.windows.values { controller.applyUserPositionPreference() }
    }

    private func reconcile() {
        let screens = NSScreen.screens
        let pairs = screens.compactMap { screen -> (NSScreen, FloatingStripScreenIdentity)? in
            let number = (screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber)?.uint32Value
            if let number, CGDisplayMirrorsDisplay(number) != kCGNullDirectDisplay { return nil }
            guard let identity = FloatingStripScreenIdentifier.identity(for: screen, mainScreen: screens.first) else { return nil }
            return (screen, identity)
        }
        let choices = pairs.map { screen, identity in
            FloatingStripDisplayChoice(id: identity.stableIdentifier, name: screen.localizedName,
                                       isPrimary: identity.isMain,
                                       isBuiltIn: identity.legacyIdentifier.flatMap(UInt32.init).map { CGDisplayIsBuiltin($0) != 0 } ?? false)
        }
        model.updateAvailableStripDisplays(choices)
        if let old = model.floatingStripDisplays.selectedIdentifier,
           let resolution = FloatingStripScreenResolver.resolve(savedIdentifier: old, screens: pairs.map { $0.1 }),
           let migrated = resolution.migratedIdentifier {
            model.migrateFloatingStripScreenIdentifier(from: old, to: migrated)
        }
        let targets = model.floatingStripDisplays.targetIdentifiers(
            online: choices.map(\.id), primary: choices.first(where: \.isPrimary)?.id)
        registry.reconcile(targets: targets) { [unowned self] identifier in
            FloatingPanelController(model: model, screenIdentifier: identifier,
                                    onProviderRequest: { [weak self] provider in
                self?.registry.present(provider, on: identifier)
            }, onPlacementSaved: { [weak self] identifier, intent in
                guard let self, model.floatingStripDisplays.shouldSelectTarget(after: intent, actualIdentifier: identifier) else { return }
                model.selectFloatingStripDisplay(identifier)
            })
        }
        for controller in registry.windows.values {
            controller.topologyDidChange()
            if shouldShow { controller.show() }
        }
    }
}
