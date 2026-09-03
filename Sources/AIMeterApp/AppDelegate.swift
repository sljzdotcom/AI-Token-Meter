import AppKit
import Foundation

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    let model = AppModel()
    let softwareUpdateCoordinator: SoftwareUpdateCoordinator

    private var notificationService: NotificationService?
    private var floatingPanelController: FloatingPanelController?

    override init() {
        let info = Bundle.main.infoDictionary ?? [:]
        softwareUpdateCoordinator = SoftwareUpdateCoordinator(
            engine: SparkleUpdateEngine(),
            currentVersion: info["CFBundleShortVersionString"] as? String ?? "Unavailable",
            currentBuild: info["CFBundleVersion"] as? String ?? "Unavailable"
        )
        super.init()
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        let panelController = FloatingPanelController(model: model)
        floatingPanelController = panelController

        model.floatingVisibilityHandler = { [weak panelController] isVisible in
            isVisible ? panelController?.show() : panelController?.hide()
        }
        model.floatingPositionHandler = { [weak panelController] in
            panelController?.applyUserPositionPreference()
        }
        if !model.isRunningDemoMode {
            let notificationService = NotificationService()
            self.notificationService = notificationService
            notificationService.onOpenProvider = { [weak panelController] provider in
                panelController?.showDetail(for: provider)
            }
            model.notificationHandler = { [weak notificationService] events in
                notificationService?.send(events)
            }
            model.notificationPermissionHandler = { [weak notificationService] in
                notificationService?.requestAuthorization()
            }
        }

        if model.showFloatingStrip {
            panelController.show()
        }
        NSWorkspace.shared.notificationCenter.addObserver(
            self,
            selector: #selector(didWake),
            name: NSWorkspace.didWakeNotification,
            object: nil
        )
        model.start()
    }

    func applicationWillTerminate(_ notification: Notification) {
        NSWorkspace.shared.notificationCenter.removeObserver(self)
        softwareUpdateCoordinator.stop()
        model.stop()
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        guard urls.contains(where: { $0.scheme?.lowercased() == "aitokenmeter" }) else { return }
        application.activate(ignoringOtherApps: true)
        if model.showFloatingStrip {
            floatingPanelController?.show()
        }
        Task { await model.refresh() }
    }

    @objc private func didWake() {
        Task { await model.refresh() }
    }
}
