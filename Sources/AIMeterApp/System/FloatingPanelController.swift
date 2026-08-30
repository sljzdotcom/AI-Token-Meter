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
        stripPanel = Self.makePanel()
        detailPanel = Self.makePanel()

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
            self?.dismissForOutsideClick(at: NSEvent.mouseLocation)
            return event
        }

        globalMouseMonitor = NSEvent.addGlobalMonitorForEvents(
            matching: [.leftMouseDown, .rightMouseDown]
        ) { [weak self] _ in
            Task { @MainActor in
                self?.dismissForOutsideClick(at: NSEvent.mouseLocation)
            }
        }
    }

    isolated deinit {
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
        let detailHost = NSHostingView(rootView: FloatingDetailView(model: model, provider: provider))
        detailHost.sizingOptions = []
        detailPanel.contentView = detailHost
        positionPanels()
        detailPanel.orderFrontRegardless()
    }

    private func dismissForOutsideClick(at point: CGPoint) {
        guard session.selectedProvider != nil else { return }
        guard FloatingPanelHitPolicy.isOutside(
            point,
            strip: stripPanel.frame,
            detail: detailPanel.frame
        ) else { return }
        session.dismiss()
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

        let detailSize = NSSize(width: 262, height: 224)
        detailPanel.setFrame(
            NSRect(
                x: stripFrame.minX - detailSize.width - 10,
                y: stripFrame.midY - detailSize.height / 2,
                width: detailSize.width,
                height: detailSize.height
            ),
            display: true,
            animate: false
        )
    }

    private static func makePanel() -> NSPanel {
        let panel = NSPanel(
            contentRect: .zero,
            styleMask: [.borderless, .nonactivatingPanel],
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
