import AppKit
import Foundation

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    let model = AppModel()

    private var notificationService: NotificationService?
    private var floatingPanelController: FloatingPanelController?

    func applicationDidFinishLaunching(_ notification: Notification) {
        let panelController = FloatingPanelController(model: model)
        floatingPanelController = panelController

        model.floatingVisibilityHandler = { [weak panelController] isVisible in
            isVisible ? panelController?.show() : panelController?.hide()
        }
        model.floatingPositionHandler = { [weak panelController] in
            panelController?.reposition()
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
    }

    @objc private func didWake() {
        Task { await model.refresh() }
    }
}
