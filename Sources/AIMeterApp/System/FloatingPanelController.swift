import AppKit
import AIMeterCore
import SwiftUI

@MainActor
final class FloatingPanelController {
    private let model: AppModel
    private let session = FloatingDetailSession()
    private let displayState: FloatingStripDisplayState
    private let stripPanel: NSPanel
    private let detailPanel: NSPanel
    private var dragStartFrame: CGRect?
    private var pointerDragState = FloatingStripPointerDragState()
    private var localMouseMonitor: Any?
    private var globalMouseMonitor: Any?
    private var screenObserver: NSObjectProtocol?
    private var voiceOverObservation: NSKeyValueObservation?
    private var detailInteraction = FloatingDetailInteractionState()

    init(model: AppModel) {
        self.model = model
        displayState = FloatingStripDisplayState(
            resolvedEdge: model.floatingStripPosition.lastResolvedEdge,
            normalizedCenterY: model.floatingStripPosition.normalizedCenterY
        )
        stripPanel = Self.makePanel(nonactivating: true)
        detailPanel = Self.makePanel(nonactivating: false)

        let stripHost = NSHostingView(rootView: FloatingStripView(
            model: model,
            session: session,
            displayState: displayState,
            onProviderTap: { [weak self] provider in
                guard let self else { return }
                session.toggle(
                    provider,
                    autoHideAfter: .seconds(model.detailAutoHideSeconds)
                )
            },
            onAccessibilityMove: { [weak self] command in
                self?.moveStripForAccessibility(command)
            }
        ))
        stripHost.sizingOptions = []
        stripPanel.contentView = stripHost
        (detailPanel as? InteractivePanel)?.onFocusedControlChange = { [weak self] focused in
            self?.detailInteraction.hasFocusedControl = focused
            self?.applyDetailInteractionState()
        }
        session.onSelectionChange = { [weak self] provider in
            self?.renderSelection(provider)
        }
        voiceOverObservation = NSWorkspace.shared.observe(
            \.isVoiceOverEnabled,
            options: [.initial, .new]
        ) { [weak self] _, change in
            Task { @MainActor in
                guard let self else { return }
                self.detailInteraction.isAccessibilityReaderActive = change.newValue ?? false
                self.applyDetailInteractionState()
            }
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
            matching: [.leftMouseDown, .leftMouseDragged, .leftMouseUp, .rightMouseDown]
        ) { [weak self] event in
            guard let self else { return event }
            if event.type == .leftMouseDown || event.type == .rightMouseDown {
                handleMonitoredClick(event)
            }
            return handleLocalPointerEvent(event)
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

    func reposition() {
        positionPanels()
    }

    private func renderSelection(_ provider: UsageProvider?) {
        detailInteraction = FloatingDetailInteractionState()
        detailInteraction.isAccessibilityReaderActive = NSWorkspace.shared.isVoiceOverEnabled
        detailPanel.makeFirstResponder(nil)
        guard let provider else {
            detailPanel.orderOut(nil)
            return
        }
        let renderedSelectionID = session.selectionID
        let interactionPolicy = FloatingDetailInteractionPolicy(provider: provider)
        let detailHost = NSHostingView(rootView: FloatingDetailView(
            model: model,
            provider: provider,
            onClaudeSetup: model.openClaudeWorkspaceSetup,
            onInteractionChange: { [weak self] isInteracting in
                guard let self else { return }
                guard FloatingDetailInteractionOwnership.accepts(
                    renderedSelectionID: renderedSelectionID,
                    currentSelectionID: session.selectionID
                ) else { return }
                detailInteraction.hasInteractiveContent = isInteracting
                applyDetailInteractionState()
            }
        ))
        detailHost.sizingOptions = []
        detailPanel.contentView = detailHost
        positionPanels()
        if interactionPolicy.activatesApplication {
            NSApp.activate(ignoringOtherApps: true)
            detailPanel.makeKeyAndOrderFront(nil)
        } else {
            detailPanel.resignKey()
            detailPanel.orderFrontRegardless()
        }
        guard interactionPolicy.requestsWebFirstResponder else { return }
        Task { @MainActor [weak self] in
            await Task.yield()
            guard let self,
                  self.session.selectedProvider == provider,
                  self.model.deepSeekWebSession.webView.window === self.detailPanel else { return }
            self.detailPanel.makeFirstResponder(self.model.deepSeekWebSession.webView)
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

    private func handleLocalPointerEvent(_ event: NSEvent) -> NSEvent? {
        switch event.type {
        case .leftMouseDown:
            guard event.window === stripPanel else { return event }
            guard pointerDragState.begin(
                windowPoint: event.locationInWindow,
                screenPoint: Self.screenPoint(for: event),
                panelSize: stripPanel.frame.size,
                edge: displayState.resolvedEdge
            ) else { return event }
            displayState.isDragging = true
            NSCursor.closedHand.set()
            return nil
        case .leftMouseDragged:
            guard let translation = pointerDragState.translation(
                to: Self.screenPoint(for: event)
            ) else { return event }
            updateStripDrag(translation: translation)
            return nil
        case .leftMouseUp:
            guard let translation = pointerDragState.end(
                at: Self.screenPoint(for: event)
            ) else { return event }
            endStripDrag(translation: translation)
            displayState.isDragging = false
            NSCursor.openHand.set()
            return nil
        default:
            return event
        }
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
        guard let context = placementContext() else { return }
        let screen = context.screen
        let edge = context.edge
        displayState.resolvedEdge = edge
        displayState.normalizedCenterY = context.normalizedCenterY
        let stripFrame = FloatingStripLayout.stripFrame(
            in: screen.visibleFrame,
            size: Self.stripSize,
            edge: edge,
            normalizedCenterY: context.normalizedCenterY
        )
        stripPanel.setFrame(stripFrame, display: true, animate: false)
        positionDetail(relativeTo: stripFrame, edge: edge, on: screen, animate: false)
        if context.usesDefaultPlacement {
            model.recoverFloatingStripAfterScreenLoss(
                screenIdentifier: Self.identifier(for: screen)
            )
        }
    }

    private func updateStripDrag(translation: CGSize) {
        if dragStartFrame == nil {
            dragStartFrame = stripPanel.frame
        }
        guard let dragStartFrame else { return }

        var proposedFrame = dragStartFrame
        if model.floatingStripPosition.preference == .automatic {
            proposedFrame.origin.x += translation.width
        }
        proposedFrame.origin.y -= translation.height
        stripPanel.setFrame(proposedFrame, display: true, animate: false)

        let screen = screen(containing: CGPoint(x: proposedFrame.midX, y: proposedFrame.midY))
            ?? preferredScreenForDragging()
        guard let screen else { return }
        let edge = FloatingStripLayout.resolvedEdge(
            preference: model.floatingStripPosition.preference,
            current: displayState.resolvedEdge,
            proposedMidX: proposedFrame.midX,
            visibleFrame: screen.visibleFrame
        )
        displayState.resolvedEdge = edge
        positionDetail(relativeTo: proposedFrame, edge: edge, on: screen, animate: false)
    }

    private func endStripDrag(translation: CGSize) {
        updateStripDrag(translation: translation)
        defer { dragStartFrame = nil }

        let proposedFrame = stripPanel.frame
        guard let screen = screen(containing: CGPoint(x: proposedFrame.midX, y: proposedFrame.midY))
                ?? preferredScreenForDragging() else { return }
        let placement = FloatingStripLayout.resolvedPlacement(
            preference: model.floatingStripPosition.preference,
            current: displayState.resolvedEdge,
            proposedFrame: proposedFrame,
            visibleFrame: screen.visibleFrame
        )
        displayState.resolvedEdge = placement.edge
        displayState.normalizedCenterY = placement.normalizedCenterY
        let finalFrame = FloatingStripLayout.stripFrame(
            in: screen.visibleFrame,
            size: Self.stripSize,
            edge: placement.edge,
            normalizedCenterY: placement.normalizedCenterY
        )
        stripPanel.setFrame(finalFrame, display: true, animate: true)
        model.saveFloatingStripPlacement(
            edge: placement.edge,
            normalizedCenterY: placement.normalizedCenterY,
            screenIdentifier: Self.identifier(for: screen)
        )
        positionDetail(relativeTo: finalFrame, edge: placement.edge, on: screen, animate: true)
    }

    private func positionDetail(
        relativeTo stripFrame: CGRect,
        edge: FloatingStripEdge,
        on screen: NSScreen,
        animate: Bool
    ) {
        let detailSize = preferredDetailSize(availableHeight: screen.visibleFrame.height)
        let detailFrame = FloatingStripLayout.detailFrame(
            size: detailSize,
            stripFrame: stripFrame,
            edge: edge,
            visibleFrame: screen.visibleFrame
        )
        detailPanel.setFrame(detailFrame, display: true, animate: animate)
    }

    private func preferredDetailSize(availableHeight: CGFloat) -> CGSize {
        let detailSize: NSSize
        switch session.selectedProvider {
        case .deepSeek: detailSize = NSSize(width: 620, height: 520)
        case .codex:
            let creditCount = model.snapshots.first(where: { $0.provider == .codex })?
                .codexResetCredits?.credits.count ?? 0
            detailSize = NSSize(
                width: 390,
                height: CodexDetailPanelLayout.height(
                    creditCount: creditCount,
                    availableHeight: availableHeight
                )
            )
        case .claude, .none: detailSize = NSSize(width: 300, height: 260)
        }
        return detailSize
    }

    private func resolvedEdgeForCurrentPreference() -> FloatingStripEdge {
        switch model.floatingStripPosition.preference {
        case .automatic:
            model.floatingStripPosition.lastResolvedEdge
        case .left:
            .left
        case .right:
            .right
        }
    }

    private func placementContext() -> (
        screen: NSScreen,
        edge: FloatingStripEdge,
        normalizedCenterY: Double,
        usesDefaultPlacement: Bool
    )? {
        if let savedIdentifier = model.floatingStripPosition.screenIdentifier,
           !NSScreen.screens.contains(where: { Self.identifier(for: $0) == savedIdentifier }) {
            let resolution = FloatingStripScreenResolver.resolve(
                savedIdentifier: savedIdentifier,
                availableIdentifiers: NSScreen.screens.compactMap(Self.identifier(for:)),
                mainIdentifier: NSScreen.main.flatMap(Self.identifier(for:))
            )
            guard let screen = NSScreen.screens.first(where: {
                Self.identifier(for: $0) == resolution.identifier
            }) ?? NSScreen.main ?? NSScreen.screens.first else { return nil }
            return (
                screen,
                resolution.defaultEdge,
                resolution.defaultNormalizedCenterY,
                resolution.usesDefaultPlacement
            )
        }
        guard let screen = model.floatingStripPosition.screenIdentifier.flatMap({ identifier in
            NSScreen.screens.first(where: { Self.identifier(for: $0) == identifier })
        }) ?? stripPanel.screen ?? NSScreen.main ?? NSScreen.screens.first else { return nil }
        return (
            screen,
            resolvedEdgeForCurrentPreference(),
            model.floatingStripPosition.normalizedCenterY,
            false
        )
    }

    private func preferredScreenForDragging() -> NSScreen? {
        if let savedIdentifier = model.floatingStripPosition.screenIdentifier,
           let savedScreen = NSScreen.screens.first(where: {
               Self.identifier(for: $0) == savedIdentifier
           }) {
            return savedScreen
        }
        return stripPanel.screen ?? NSScreen.main ?? NSScreen.screens.first
    }

    private func screen(containing point: CGPoint) -> NSScreen? {
        NSScreen.screens.first(where: { $0.visibleFrame.contains(point) })
    }

    private static func identifier(for screen: NSScreen) -> String? {
        let key = NSDeviceDescriptionKey("NSScreenNumber")
        return (screen.deviceDescription[key] as? NSNumber)?.stringValue
    }

    private func applyDetailInteractionState() {
        session.setAutoHidePaused(
            detailInteraction.shouldPauseAutoHide,
            restartAfter: .seconds(model.detailAutoHideSeconds)
        )
    }

    private func moveStripForAccessibility(_ command: FloatingStripAccessibilityCommand) {
        let placement = FloatingStripAccessibilityMovement.position(
            after: command,
            currentEdge: displayState.resolvedEdge,
            normalizedCenterY: displayState.normalizedCenterY
        )
        switch command {
        case .moveToLeftEdge:
            model.setFloatingStripEdgePreference(.left)
        case .moveToRightEdge:
            model.setFloatingStripEdgePreference(.right)
        case .moveUp, .moveDown:
            break
        }
        model.saveFloatingStripPlacement(
            edge: placement.edge,
            normalizedCenterY: placement.normalizedCenterY,
            screenIdentifier: preferredScreenForDragging().flatMap(Self.identifier(for:))
        )
        positionPanels()
    }

    private static let stripSize = CGSize(width: 108, height: 356)

    private static func makePanel(nonactivating: Bool) -> NSPanel {
        var styleMask: NSWindow.StyleMask = [.borderless]
        if nonactivating {
            styleMask.insert(.nonactivatingPanel)
        }
        let panel: NSPanel = if nonactivating {
            KeyboardAccessibleStripPanel(
                contentRect: .zero,
                styleMask: styleMask,
                backing: .buffered,
                defer: false
            )
        } else {
            InteractivePanel(
                contentRect: .zero,
                styleMask: styleMask,
                backing: .buffered,
                defer: false
            )
        }
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = false
        panel.hidesOnDeactivate = false
        FloatingPanelPresentationPolicy.apply(to: panel)
        panel.becomesKeyOnlyIfNeeded = nonactivating
        return panel
    }
}
