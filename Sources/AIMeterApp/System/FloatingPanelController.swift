import AppKit
import AIMeterCore
import SwiftUI

enum FloatingStripPositionPersistenceAction: Equatable {
    case preserve
    case migrate(from: String, to: String)
    case save(to: String)
}

struct FloatingStripResolvedScreenPlacement: Equatable {
    let edge: FloatingStripEdge
    let normalizedCenterY: Double
    let persistenceAction: FloatingStripPositionPersistenceAction
}

enum FloatingStripPositionPersistencePolicy {
    static func action(
        savedIdentifier: String?,
        resolution: FloatingStripScreenResolution
    ) -> FloatingStripPositionPersistenceAction {
        guard let savedIdentifier,
              let migratedIdentifier = resolution.migratedIdentifier,
              savedIdentifier != migratedIdentifier else { return .preserve }
        return .migrate(from: savedIdentifier, to: migratedIdentifier)
    }

    static func resolvedPlacement(
        position: FloatingStripPosition,
        resolution: FloatingStripScreenResolution,
        userInitiated: Bool = false
    ) -> FloatingStripResolvedScreenPlacement {
        let edge: FloatingStripEdge = switch position.preference {
        case .automatic:
            position.lastResolvedEdge
        case .left:
            .left
        case .right:
            .right
        }
        let persistenceAction: FloatingStripPositionPersistenceAction
        if userInitiated, let selectedIdentifier = resolution.selectedIdentifier {
            persistenceAction = .save(to: selectedIdentifier)
        } else {
            persistenceAction = action(
                savedIdentifier: position.screenIdentifier,
                resolution: resolution
            )
        }
        return FloatingStripResolvedScreenPlacement(
            edge: edge,
            normalizedCenterY: position.normalizedCenterY,
            persistenceAction: persistenceAction
        )
    }
}

@MainActor
final class FloatingPanelController: NSObject, NSMenuDelegate, FloatingStripWindow {
    private let model: AppModel
    private let screenIdentifier: String?
    private let onProviderRequest: ((UsageProvider) -> Void)?
    private let onPlacementSaved: ((String, FloatingStripPlacementIntent) -> Void)?
    private let session = FloatingDetailSession()
    private let displayState: FloatingStripDisplayState
    private let stripPanel: NSPanel
    private let detailPanel: NSPanel
    private var dragStartFrame: CGRect?
    private var pointerDragState = FloatingStripPointerDragState()
    private var localMouseMonitor: Any?
    private var globalMouseMonitor: Any?
    private var activeSpaceObserver: ActiveSpaceChangeObserver?
    private var voiceOverObservation: NSKeyValueObservation?
    private var detailInteraction = FloatingDetailInteractionState()
    private var foldState = FloatingStripFoldState()
    private var foldTimer: Timer?
    private var visibilityTransitionTask: Task<Void, Never>?
    private var pendingFoldedState: Bool?
    private var appearanceUpdatePendingDuringDrag = false
    private var menuIsOpen = false
    private var temporarilyHidden = false

    init(model: AppModel, screenIdentifier: String? = nil,
         onProviderRequest: ((UsageProvider) -> Void)? = nil,
         onPlacementSaved: ((String, FloatingStripPlacementIntent) -> Void)? = nil) {
        self.model = model
        self.screenIdentifier = screenIdentifier
        self.onProviderRequest = onProviderRequest
        self.onPlacementSaved = onPlacementSaved
        displayState = FloatingStripDisplayState(
            resolvedEdge: model.floatingStripPosition.lastResolvedEdge,
            normalizedCenterY: model.floatingStripPosition.normalizedCenterY
        )
        stripPanel = Self.makePanel(nonactivating: true, role: .strip)
        detailPanel = Self.makePanel(nonactivating: false, role: .detail)
        super.init()
        foldTimer = Timer.scheduledTimer(
            withTimeInterval: FloatingStripFoldState.pollingInterval,
            repeats: true
        ) { [weak self] _ in
            MainActor.assumeIsolated { self?.tickFold() }
        }

        let stripHost = NSHostingView(rootView: FloatingStripView(
            model: model,
            session: session,
            displayState: displayState,
            onProviderTap: { [weak self] provider in
                guard let self else { return }
                if session.selectedProvider == provider { dismissDetail() }
                else if let onProviderRequest { onProviderRequest(provider) }
                else { showDetail(for: provider) }
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
        activeSpaceObserver = ActiveSpaceChangeObserver { [weak self] in
            self?.handleActiveSpaceChange()
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
        foldTimer?.invalidate()
        visibilityTransitionTask?.cancel()
        activeSpaceObserver?.invalidate()
        if let localMouseMonitor {
            NSEvent.removeMonitor(localMouseMonitor)
        }
        if let globalMouseMonitor {
            NSEvent.removeMonitor(globalMouseMonitor)
        }
    }

    func dismissDetail() {
        session.dismiss()
        detailPanel.makeFirstResponder(nil)
        detailPanel.orderOut(nil)
        detailPanel.contentView = nil
    }

    func close() {
        hide()
        foldTimer?.invalidate()
        foldTimer = nil
        visibilityTransitionTask?.cancel()
        visibilityTransitionTask = nil
        activeSpaceObserver?.invalidate()
        activeSpaceObserver = nil
        voiceOverObservation?.invalidate()
        voiceOverObservation = nil
        if let localMouseMonitor { NSEvent.removeMonitor(localMouseMonitor); self.localMouseMonitor = nil }
        if let globalMouseMonitor { NSEvent.removeMonitor(globalMouseMonitor); self.globalMouseMonitor = nil }
        stripPanel.contentView = nil
        detailPanel.contentView = nil
        stripPanel.close()
        detailPanel.close()
    }

    func topologyDidChange() {
        pointerDragState = FloatingStripPointerDragState()
        dragStartFrame = nil
        displayState.isDragging = false
        positionPanels()
    }

    func applyAppearance() {
        if let selected = session.selectedProvider,
           !model.stripPreferences.visibleProviders.contains(selected) { dismissDetail() }
        guard !displayState.isDragging else {
            appearanceUpdatePendingDuringDrag = true
            visibilityTransitionTask?.cancel()
            visibilityTransitionTask = nil
            pendingFoldedState = nil
            return
        }
        foldState.update(
            now: ProcessInfo.processInfo.systemUptime,
            revealDelay: 0,
            collapseDelay: 0,
            hovering: true,
            lockedOpen: true
        )
        visibilityTransitionTask?.cancel()
        pendingFoldedState = nil
        displayState.isFolded = false
        displayState.showsExpandedContent = true
        positionPanels()
        tickFold()
    }

    func show() {
        guard !model.stripPreferences.isTemporarilyHidden(now: Date().timeIntervalSince1970) else { return }
        positionPanels()
        stripPanel.orderFrontRegardless()
        if session.selectedProvider != nil {
            detailPanel.orderFrontRegardless()
        }
    }

    func hide() {
        dismissDetail()
        stripPanel.orderOut(nil)
        detailPanel.orderOut(nil)
    }

    func showDetail(for provider: UsageProvider) {
        session.present(
            provider,
            autoHideAfter: .seconds(model.detailAutoHideSeconds)
        )
    }

    func applyUserPositionPreference() {
        guard let screen = preferredScreenForDragging(),
              let identifier = Self.identity(for: screen)?.stableIdentifier else { return }
        let preferences = model.floatingStripDisplays
        if preferences.shouldSelectTarget(after: .edit, actualIdentifier: identifier) {
            let previous = preferences.placement(for: preferences.selectedIdentifier ?? identifier)
            model.saveFloatingStripPlacement(edge: previous.edge, normalizedCenterY: previous.normalizedCenterY,
                                             screenIdentifier: identifier)
            onPlacementSaved?(identifier, .edit)
        }
        positionPanels()
    }

    private func renderSelection(_ provider: UsageProvider?) {
        detailInteraction = FloatingDetailInteractionState()
        detailInteraction.isAccessibilityReaderActive = NSWorkspace.shared.isVoiceOverEnabled
        detailPanel.makeFirstResponder(nil)
        guard let provider else {
            detailPanel.orderOut(nil)
            detailPanel.contentView = nil
            return
        }
        let renderedSelectionID = session.selectionID
        let interactionPolicy = FloatingDetailInteractionPolicy(provider: provider)
        let detailHost = NSHostingView(rootView: FloatingDetailView(
            model: model,
            provider: provider,
            onClaudeSetup: model.openClaudeWorkspaceSetup,
            onOpenServicesSettings: { [weak self] in
                self?.model.requestSettings(.services)
            },
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
        if event.type == .rightMouseDown, event.window === stripPanel { return }
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
        case .rightMouseDown:
            guard event.window === stripPanel else { return event }
            openContextMenu(event)
            return nil
        case .leftMouseDown:
            guard event.window === stripPanel else { return event }
            guard !displayState.isFolded else { tickFold(forceExpanded: true); return nil }
            guard pointerDragState.begin(
                windowPoint: event.locationInWindow,
                screenPoint: Self.screenPoint(for: event),
                panelSize: stripPanel.frame.size,
                edge: displayState.resolvedEdge,
                density: model.stripPreferences.density,
                providerCount: model.stripPreferences.visibleProviders.count
            ) else { return event }
            dragStartFrame = stripPanel.frame
            displayState.isDragging = true
            NSCursor.closedHand.set()
            return nil
        case .leftMouseDragged:
            guard let translation = pointerDragState.translation(
                to: Self.screenPoint(for: event)
            ) else { return event }
            updateStripDrag(translation: translation, pointer: Self.screenPoint(for: event))
            return nil
        case .leftMouseUp:
            guard let translation = pointerDragState.end(
                at: Self.screenPoint(for: event)
            ) else { return event }
            displayState.isDragging = false
            endStripDrag(translation: translation, pointer: Self.screenPoint(for: event))
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

    private func handleActiveSpaceChange() {
        session.dismiss()
        detailPanel.makeFirstResponder(nil)
        detailPanel.orderOut(nil)
        positionPanels()
        if stripPanel.isVisible {
            stripPanel.orderFrontRegardless()
        }
    }

    private static func screenPoint(for event: NSEvent) -> CGPoint {
        guard let window = event.window else { return event.locationInWindow }
        return window.convertPoint(toScreen: event.locationInWindow)
    }

    private func positionPanels(userInitiated: Bool = false) {
        guard !displayState.isDragging else { return }
        guard let context = placementContext(userInitiated: userInitiated) else { return }
        let screen = context.screen
        let edge = context.edge
        displayState.resolvedEdge = edge
        displayState.normalizedCenterY = context.normalizedCenterY
        let expandedFrame = FloatingStripLayout.anchoredFrame(
            in: screen.visibleFrame,
            size: expandedStripSize,
            edge: edge,
            normalizedCenterY: context.normalizedCenterY
        )
        let stripFrame = displayState.isFolded ? FloatingStripLayout.foldedFrame(from: expandedFrame, edge: edge) : expandedFrame
        // The transparent window and its nonlinear mask must always be committed at an
        // exact endpoint. Scaling the NSPanel frame produces a different, pointed contour.
        stripPanel.setFrame(stripFrame, display: true, animate: false)
        positionDetail(relativeTo: stripFrame, edge: edge, on: screen, animate: false)
        switch context.persistenceAction {
        case .preserve:
            break
        case let .migrate(from: oldIdentifier, to: newIdentifier):
            model.migrateFloatingStripScreenIdentifier(
                from: oldIdentifier,
                to: newIdentifier
            )
        case let .save(to: screenIdentifier):
            model.saveFloatingStripPlacement(
                edge: edge,
                normalizedCenterY: context.normalizedCenterY,
                screenIdentifier: screenIdentifier
            )
        }
    }

    private func updateStripDrag(translation: CGSize, pointer: CGPoint) {
        if dragStartFrame == nil {
            dragStartFrame = stripPanel.frame
        }
        guard let dragStartFrame else { return }

        var proposedFrame = FloatingStripDragPolicy.translated(dragStartFrame, by: translation)
        if model.floatingStripDisplays.mode == .all, let screen = preferredScreenForDragging() {
            proposedFrame.origin.x = min(max(proposedFrame.minX, screen.visibleFrame.minX),
                                         screen.visibleFrame.maxX - proposedFrame.width)
            proposedFrame.origin.y = min(max(proposedFrame.minY, screen.visibleFrame.minY),
                                         max(screen.visibleFrame.minY, screen.visibleFrame.maxY - proposedFrame.height))
        }
        stripPanel.setFrame(proposedFrame, display: true, animate: false)

        let screen = dragTarget(at: pointer)
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

    private func endStripDrag(translation: CGSize, pointer: CGPoint) {
        updateStripDrag(translation: translation, pointer: pointer)
        defer {
            dragStartFrame = nil
            applyAppearancePendingAfterDrag()
        }

        let proposedFrame = stripPanel.frame
        guard let screen = dragTarget(at: pointer)
                ?? preferredScreenForDragging() else { return }
        let placement = FloatingStripLayout.resolvedPlacement(
            preference: model.floatingStripPosition.preference,
            current: displayState.resolvedEdge,
            proposedFrame: proposedFrame,
            visibleFrame: screen.visibleFrame
        )
        displayState.resolvedEdge = placement.edge
        let anchor = FloatingStripLayout.anchorNormalizedY(for: proposedFrame, in: screen.visibleFrame)
        displayState.normalizedCenterY = anchor
        let finalFrame = FloatingStripLayout.anchoredFrame(
            in: screen.visibleFrame,
            size: stripSize,
            edge: placement.edge,
            normalizedCenterY: anchor
        )
        // The strip uses a nonlinear transparent mask, so even drag snapping must
        // commit the exact endpoint without scaling the window contents.
        stripPanel.setFrame(finalFrame, display: true, animate: false)
        model.saveFloatingStripPlacement(
            edge: placement.edge,
            normalizedCenterY: anchor,
            screenIdentifier: Self.identity(for: screen)?.stableIdentifier
        )
        positionDetail(relativeTo: finalFrame, edge: placement.edge, on: screen, animate: true)
        if let identifier = Self.identity(for: screen)?.stableIdentifier { onPlacementSaved?(identifier, .drag) }
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
        case .gemini:
            let tierCount = model.snapshots.first(where: { $0.provider == .gemini })?.geminiQuotaMetrics?.count ?? 0
            detailSize = NSSize(width: 420, height: GeminiDetailPanelLayout.height(tierCount: tierCount, availableHeight: availableHeight))
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
        case .claude:
            detailSize = ClaudeDetailPanelLayout.size(availableHeight: availableHeight)
        case .none: detailSize = NSSize(width: 300, height: 260)
        }
        return detailSize
    }

    private func placementContext(userInitiated: Bool = false) -> (
        screen: NSScreen,
        edge: FloatingStripEdge,
        normalizedCenterY: Double,
        persistenceAction: FloatingStripPositionPersistenceAction
    )? {
        let pairs = screenIdentityPairs()
        if let screenIdentifier {
            guard let screen = pairs.first(where: { $0.identity.stableIdentifier == screenIdentifier })?.screen else { return nil }
            let preferences = model.floatingStripDisplays
            // A disconnected selected display borrows the primary screen, not its saved position.
            let positionID = preferences.mode == .selected ? preferences.selectedIdentifier ?? screenIdentifier : screenIdentifier
            let placement = preferences.placement(for: positionID, preference: model.floatingStripPosition.preference)
            return (screen, placement.edge, placement.normalizedCenterY, .preserve)
        }
        let savedIdentifier = model.floatingStripPosition.screenIdentifier
        guard let resolution = FloatingStripScreenResolver.resolve(
            savedIdentifier: savedIdentifier,
            screens: pairs.map(\.identity)
        ),
        let selectedIdentifier = resolution.selectedIdentifier,
        let screen = pairs.first(where: {
            $0.identity.stableIdentifier == selectedIdentifier
        })?.screen else { return nil }
        let placement = FloatingStripPositionPersistencePolicy.resolvedPlacement(
            position: model.floatingStripPosition,
            resolution: resolution,
            userInitiated: userInitiated
        )
        return (
            screen,
            placement.edge,
            placement.normalizedCenterY,
            placement.persistenceAction
        )
    }

    private func preferredScreenForDragging() -> NSScreen? {
        let pairs = screenIdentityPairs()
        if let screenIdentifier {
            return pairs.first(where: { $0.identity.stableIdentifier == screenIdentifier })?.screen
        }
        if let savedIdentifier = model.floatingStripPosition.screenIdentifier,
           let savedPair = pairs.first(where: {
               $0.identity.stableIdentifier == savedIdentifier
                   || $0.identity.legacyIdentifier == savedIdentifier
           }) {
            return savedPair.screen
        }
        return stripPanel.screen ?? NSScreen.screens.first
    }

    private func dragTarget(at point: CGPoint) -> NSScreen? {
        let pairs = screenIdentityPairs()
        let frames = Dictionary(pairs.map { ($0.identity.stableIdentifier, $0.screen.frame) },
                                uniquingKeysWith: { first, _ in first })
        let target = FloatingStripDragPolicy.target(at: point, screens: frames,
                    pinned: model.floatingStripDisplays.mode == .all ? screenIdentifier : nil)
        return pairs.first(where: { $0.identity.stableIdentifier == target })?.screen
    }

    private func screenIdentityPairs() -> [(screen: NSScreen, identity: FloatingStripScreenIdentity)] {
        NSScreen.screens.compactMap { screen in
            Self.identity(for: screen).map { (screen, $0) }
        }
    }

    private static func identity(for screen: NSScreen) -> FloatingStripScreenIdentity? {
        FloatingStripScreenIdentifier.identity(for: screen, mainScreen: NSScreen.screens.first)
    }

    private func applyDetailInteractionState() {
        session.setAutoHidePaused(
            detailInteraction.shouldPauseAutoHide || menuIsOpen,
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
        case .moveToLeftEdge: model.setFloatingStripEdgePreference(.left)
        case .moveToRightEdge: model.setFloatingStripEdgePreference(.right)
        case .moveUp, .moveDown: break
        }
        model.saveFloatingStripPlacement(
            edge: placement.edge,
            normalizedCenterY: placement.normalizedCenterY,
            screenIdentifier: preferredScreenForDragging().flatMap(Self.identity(for:))?
                .stableIdentifier
        )
        if let identifier = preferredScreenForDragging().flatMap(Self.identity(for:))?.stableIdentifier {
            onPlacementSaved?(identifier, .edit)
        }
        positionPanels()
    }

    private var stripSize: CGSize {
        if displayState.isFolded { return FloatingStripLayout.foldedSize }
        return expandedStripSize
    }

    var stripFrameForTesting: CGRect { stripPanel.frame }
    var stripContentBoundsForTesting: CGRect { stripPanel.contentView?.bounds ?? .zero }
    var stripIsFoldedForTesting: Bool { displayState.isFolded }
    var stripShowsExpandedContentForTesting: Bool { displayState.showsExpandedContent }

    func setStripDraggingForTesting(_ dragging: Bool) {
        displayState.isDragging = dragging
        if !dragging { applyAppearancePendingAfterDrag() }
    }

    func transitionStripForTesting(toFolded folded: Bool) {
        transitionStrip(toFolded: folded)
    }

    func suspendFoldPollingForTesting() {
        foldTimer?.invalidate()
        foldTimer = nil
    }

    private func applyAppearancePendingAfterDrag() {
        guard appearanceUpdatePendingDuringDrag else { return }
        appearanceUpdatePendingDuringDrag = false
        applyAppearance()
    }

    private var expandedStripSize: CGSize {
        let value = model.stripPreferences
        return CGSize(width: value.density.width, height: value.density.height(providerCount: value.visibleProviders.count))
    }

    private func tickFold(forceExpanded: Bool = false) {
        let hidden = model.stripPreferences.isTemporarilyHidden(now: Date().timeIntervalSince1970)
        if hidden {
            if !temporarilyHidden { hide(); temporarilyHidden = true }
            return
        }
        if temporarilyHidden {
            temporarilyHidden = false
            if model.showFloatingStrip { show() }
        }
        guard model.showFloatingStrip, stripPanel.isVisible else { return }
        let point = NSEvent.mouseLocation
        let hovering = stripPanel.frame.contains(point)
        let lockedOpen = forceExpanded || (pendingFoldedState == true && hovering)
            || session.selectedProvider != nil || displayState.isDragging || menuIsOpen
            || stripPanel.isKeyWindow || settingsWindowIsVisible || NSWorkspace.shared.isVoiceOverEnabled
            || model.isRefreshing || model.isAuthenticating || model.serviceAccounts.values.contains { $0.connectionState == .checking }
        foldState.update(
            now: ProcessInfo.processInfo.systemUptime,
            revealDelay: Double(model.stripPreferences.revealDelayMilliseconds) / 1_000,
            collapseDelay: Double(model.stripPreferences.collapseDelayMilliseconds) / 1_000,
            hovering: hovering,
            lockedOpen: lockedOpen,
            automaticallyCollapses: model.stripPreferences.automaticallyCollapses
        )
        transitionStrip(toFolded: foldState.isFolded)
    }

    private var settingsWindowIsVisible: Bool {
        NSApp.windows.contains {
            $0.identifier?.rawValue == "com_apple_SwiftUI_Settings_window" && $0.isVisible
        }
    }

    private func transitionStrip(toFolded folded: Bool) {
        if pendingFoldedState == folded { return }
        if displayState.isFolded == folded, pendingFoldedState == nil {
            if !folded { displayState.showsExpandedContent = true }
            return
        }
        visibilityTransitionTask?.cancel()
        pendingFoldedState = folded
        let reduceMotion = NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
        let transition = FloatingStripVisibilityTransitionPlan.make(
            destination: folded ? .folded : .expanded,
            reduceMotion: reduceMotion
        )
        if folded {
            displayState.showsExpandedContent = false
            if reduceMotion {
                displayState.isFolded = true
                pendingFoldedState = nil
                positionPanels()
                return
            }
            visibilityTransitionTask = Task { @MainActor [weak self] in
                try? await Task.sleep(for: transition.contentDelay)
                guard let self, !Task.isCancelled, self.pendingFoldedState == true else { return }
                self.displayState.isFolded = true
                self.pendingFoldedState = nil
                self.positionPanels()
            }
        } else {
            displayState.isFolded = false
            displayState.showsExpandedContent = false
            positionPanels()
            if reduceMotion {
                displayState.showsExpandedContent = true
                pendingFoldedState = nil
                return
            }
            visibilityTransitionTask = Task { @MainActor [weak self] in
                try? await Task.sleep(for: transition.contentDelay)
                guard let self, !Task.isCancelled, self.pendingFoldedState == false else { return }
                // Re-read the latest density before revealing content. This also settles
                // any Settings change that arrived while the opacity transition was active.
                self.positionPanels()
                self.displayState.showsExpandedContent = true
                self.pendingFoldedState = nil
            }
        }
    }

    private func openContextMenu(_ event: NSEvent) {
        let menu = NSMenu()
        menu.autoenablesItems = false
        menu.delegate = self
        for (title, action) in [("Refresh now", #selector(refreshFromMenu)),
                                ("Hide for 1 hour", #selector(hideFromMenu)),
                                ("Settings…", #selector(settingsFromMenu)),
                                ("Quit AI Token Meter", #selector(quitFromMenu))] {
            if title == "Settings…" { menu.addItem(.separator()) }
            let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
            item.target = self
            item.isEnabled = title != "Refresh now" || !model.isRefreshing
            menu.addItem(item)
        }
        if let view = stripPanel.contentView { NSMenu.popUpContextMenu(menu, with: event, for: view) }
    }
    func menuWillOpen(_ menu: NSMenu) { menuIsOpen = true; applyDetailInteractionState(); tickFold(forceExpanded: true) }
    func menuDidClose(_ menu: NSMenu) { menuIsOpen = false; applyDetailInteractionState(); tickFold() }
    @objc private func refreshFromMenu() { Task { await model.refresh() } }
    @objc private func hideFromMenu() { model.hideStripForOneHour() }
    @objc private func settingsFromMenu() { model.requestSettings(.floatingStrip) }
    @objc private func quitFromMenu() { NSApp.terminate(nil) }

    private static func makePanel(
        nonactivating: Bool,
        role: FloatingPanelPresentationRole
    ) -> NSPanel {
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
        panel.isReleasedWhenClosed = false
        panel.backgroundColor = .clear
        panel.hasShadow = false
        panel.hidesOnDeactivate = false
        FloatingPanelPresentationPolicy.apply(to: panel, role: role)
        panel.becomesKeyOnlyIfNeeded = nonactivating
        return panel
    }
}
