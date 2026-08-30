import AppKit
import AIMeterCore
import SwiftUI

@MainActor
final class FloatingPanelController {
    private let model: AppModel
    private let session = FloatingDetailSession()
    private let stripPanel: NSPanel
    private let detailPanel: NSPanel
    private var localMouseMonitor: Any?
    private var globalMouseMonitor: Any?
    private var screenObserver: NSObjectProtocol?

    init(model: AppModel) {
        self.model = model
        stripPanel = Self.makePanel(nonactivating: true)
        detailPanel = Self.makePanel(nonactivating: false)

        let stripHost = NSHostingView(rootView: FloatingStripView(model: model, session: session) { [weak self] provider in
            guard let self else { return }
            session.toggle(
                provider,
                autoHideAfter: .seconds(model.detailAutoHideSeconds)
            )
        })
        stripHost.sizingOptions = []
        stripPanel.contentView = stripHost
        session.onSelectionChange = { [weak self] provider in
            self?.renderSelection(provider)
        }
        positionPanels()

        screenObserver = NotificationCenter.default.addObserver(
            forName: NSApplication.didChangeScreenParametersNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor in self?.positionPanels() }
        }

        localMouseMonitor = NSEvent.addLocalMonitorForEvents(
            matching: [.leftMouseDown, .rightMouseDown]
        ) { [weak self] event in
            self?.handleMonitoredClick(event)
            return event
        }

        globalMouseMonitor = NSEvent.addGlobalMonitorForEvents(
            matching: [.leftMouseDown, .rightMouseDown]
        ) { [weak self] event in
            MainActor.assumeIsolated {
                self?.handleMonitoredClick(event)
            }
        }
    }

    isolated deinit {
        session.shutdown()
        if let localMouseMonitor {
            NSEvent.removeMonitor(localMouseMonitor)
        }
        if let globalMouseMonitor {
            NSEvent.removeMonitor(globalMouseMonitor)
        }
        if let screenObserver {
            NotificationCenter.default.removeObserver(screenObserver)
        }
    }

    func show() {
        positionPanels()
        stripPanel.orderFrontRegardless()
        if session.selectedProvider != nil {
            detailPanel.orderFrontRegardless()
        }
    }

    func hide() {
        session.dismiss()
        stripPanel.orderOut(nil)
        detailPanel.orderOut(nil)
    }

    func showDetail(for provider: UsageProvider) {
        session.present(
            provider,
            autoHideAfter: .seconds(model.detailAutoHideSeconds)
        )
    }

    private func renderSelection(_ provider: UsageProvider?) {
        guard let provider else {
            detailPanel.orderOut(nil)
            return
        }
        let detailHost = NSHostingView(rootView: FloatingDetailView(
            model: model,
            provider: provider,
            onClaudeSetup: model.openClaudeWorkspaceSetup,
            onInteractionChange: { [weak self] isInteracting in
                guard let self else { return }
                session.setAutoHidePaused(
                    isInteracting,
                    restartAfter: .seconds(model.detailAutoHideSeconds)
                )
            }
        ))
        detailHost.sizingOptions = []
        detailPanel.contentView = detailHost
        positionPanels()
        if provider == .deepSeek {
            NSApp.activate(ignoringOtherApps: true)
            detailPanel.makeKeyAndOrderFront(nil)
        } else {
            detailPanel.orderFrontRegardless()
        }
    }

    private func handleMonitoredClick(_ event: NSEvent) {
        guard let selectionID = session.selectionID else { return }
        let request = FloatingPanelDismissalRequest(
            screenPoint: Self.screenPoint(for: event),
            eventTimestamp: event.timestamp,
            selectionID: selectionID
        )
        dismissForOutsideClick(request)
    }

    private func dismissForOutsideClick(_ request: FloatingPanelDismissalRequest) {
        guard request.requestsDismissal(
            currentSelectionID: session.selectionID,
            strip: stripPanel.frame,
            detail: detailPanel.frame
        ) else { return }
        session.dismiss(ifCurrent: request.selectionID)
    }

    private static func screenPoint(for event: NSEvent) -> CGPoint {
        guard let window = event.window else { return event.locationInWindow }
        return window.convertPoint(toScreen: event.locationInWindow)
    }

    private func positionPanels() {
        let screen = stripPanel.screen ?? NSScreen.main ?? NSScreen.screens.first
        guard let visibleFrame = screen?.visibleFrame else { return }
        let stripSize = NSSize(width: 84, height: 300)
        let stripFrame = NSRect(
            x: visibleFrame.maxX - stripSize.width - 12,
            y: visibleFrame.midY - stripSize.height / 2,
            width: stripSize.width,
            height: stripSize.height
        )
        stripPanel.setFrame(stripFrame, display: true, animate: false)

        let detailSize: NSSize
        switch session.selectedProvider {
        case .deepSeek: detailSize = NSSize(width: 620, height: 520)
        case .codex: detailSize = NSSize(width: 340, height: 360)
        case .claude, .none: detailSize = NSSize(width: 300, height: 260)
        }
        let detailOriginX = max(visibleFrame.minX + 8, stripFrame.minX - detailSize.width - 10)
        let centeredY = stripFrame.midY - detailSize.height / 2
        let detailOriginY = min(
            max(centeredY, visibleFrame.minY + 8),
            visibleFrame.maxY - detailSize.height - 8
        )
        detailPanel.setFrame(
            NSRect(
                x: detailOriginX,
                y: detailOriginY,
                width: detailSize.width,
                height: detailSize.height
            ),
            display: true,
            animate: false
        )
    }

    private static func makePanel(nonactivating: Bool) -> NSPanel {
        var styleMask: NSWindow.StyleMask = [.borderless]
        if nonactivating {
            styleMask.insert(.nonactivatingPanel)
        }
        let panel = NSPanel(
            contentRect: .zero,
            styleMask: styleMask,
            backing: .buffered,
            defer: false
        )
        panel.level = .floating
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = false
        panel.hidesOnDeactivate = false
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary]
        panel.becomesKeyOnlyIfNeeded = true
        return panel
    }
}
