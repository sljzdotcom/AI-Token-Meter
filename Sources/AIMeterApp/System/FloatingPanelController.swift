import AppKit
import AIMeterCore
import SwiftUI

@MainActor
final class FloatingPanelController {
    private let model: AppModel
    private let stripPanel: NSPanel
    private let detailPanel: NSPanel
    private var selectedProvider: UsageProvider?
    private var screenObserver: NSObjectProtocol?

    init(model: AppModel) {
        self.model = model
        stripPanel = Self.makePanel()
        detailPanel = Self.makePanel()

        let stripHost = NSHostingView(rootView: FloatingStripView(model: model) { [weak self] provider in
            self?.select(provider)
        })
        stripHost.sizingOptions = []
        stripPanel.contentView = stripHost
        positionPanels()

        screenObserver = NotificationCenter.default.addObserver(
            forName: NSApplication.didChangeScreenParametersNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor in self?.positionPanels() }
        }
    }

    func show() {
        positionPanels()
        stripPanel.orderFrontRegardless()
        if selectedProvider != nil {
            detailPanel.orderFrontRegardless()
        }
    }

    func hide() {
        stripPanel.orderOut(nil)
        detailPanel.orderOut(nil)
    }

    func showDetail(for provider: UsageProvider) {
        select(provider)
    }

    private func select(_ provider: UsageProvider?) {
        selectedProvider = provider
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

        let detailSize = NSSize(width: 262, height: 190)
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
