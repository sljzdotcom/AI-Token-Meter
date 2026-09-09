import AIMeterCore
import AppKit
import SwiftUI

struct FloatingStripView: View {
    @Bindable var model: AppModel
    @Bindable var session: FloatingDetailSession
    @Bindable var displayState: FloatingStripDisplayState
    let onProviderTap: (UsageProvider) -> Void
    let onAccessibilityMove: (FloatingStripAccessibilityCommand) -> Void
    @Environment(\.openSettings) private var openSettings
    @AccessibilityFocusState private var accessibilityFocusedProvider: UsageProvider?

    var body: some View {
        ZStack {
            if displayState.isFolded {
                RoundedRectangle(cornerRadius: 6)
                    .fill(Color(red: 0.015, green: 0.04, blue: 0.085))
                    .overlay {
                        if let image = FloatingStripBackgroundAsset.defaultImage {
                            Image(nsImage: image).resizable().scaledToFill()
                                .scaleEffect(x: displayState.resolvedEdge == .left ? -1 : 1, y: 1)
                                .overlay(Color.black.opacity(0.46))
                        }
                    }
                    .clipShape(RoundedRectangle(cornerRadius: 6))
                    .overlay { Capsule().fill(.white.opacity(0.3)).frame(width: 3, height: 40) }
                    .accessibilityLabel("Expand floating meter")
            } else {
            FloatingStripSurface(edge: displayState.resolvedEdge, density: density, providerCount: presentations.count)
                .contentShape(FloatingStripDragShape(edge: displayState.resolvedEdge, density: density, providerCount: presentations.count), eoFill: true)
                .focusable()
                .accessibilityElement(children: .ignore)
                .accessibilityLabel("Move floating meter")
                .accessibilityValue(accessibilityPositionValue)
                .accessibilityHint("Use up or down to move. Left and right set the edge preference")
                .onMoveCommand { direction in
                    switch direction {
                    case .up: onAccessibilityMove(.moveUp)
                    case .down: onAccessibilityMove(.moveDown)
                    case .left: onAccessibilityMove(.moveToLeftEdge)
                    case .right: onAccessibilityMove(.moveToRightEdge)
                    default: break
                    }
                }
                .accessibilityAdjustableAction { direction in
                    switch direction {
                    case .increment: onAccessibilityMove(.moveUp)
                    case .decrement: onAccessibilityMove(.moveDown)
                    @unknown default: break
                    }
                }
                .accessibilityAction(named: "Set edge preference to Left") {
                    onAccessibilityMove(.moveToLeftEdge)
                }
                .accessibilityAction(named: "Set edge preference to Right") {
                    onAccessibilityMove(.moveToRightEdge)
                }
                .onContinuousHover { phase in
                    switch phase {
                    case .active:
                        if !displayState.isDragging {
                            NSCursor.openHand.set()
                        }
                    case .ended:
                        if !displayState.isDragging {
                            NSCursor.arrow.set()
                        }
                    }
                }
            VStack(spacing: density.spacing) {
                ForEach(presentations, id: \.provider) { presentation in
                    Button {
                        onProviderTap(presentation.provider)
                    } label: {
                        UsageRing(
                            presentation: presentation,
                            size: density.ringSize,
                            operation: model.operationState(for: presentation.provider)
                        )
                            .scaleEffect(session.selectedProvider == presentation.provider ? 1.06 : 1)
                    }
                    .buttonStyle(.plain)
                    .accessibilityValue(session.accessibilityValue(for: presentation.provider))
                    .accessibilityFocused(
                        $accessibilityFocusedProvider,
                        equals: presentation.provider
                    )
                    .animation(
                        .spring(response: 0.28, dampingFraction: 0.8),
                        value: session.selectedProvider
                    )
                }
            }
            .padding(.vertical, FloatingStripContentLayout.verticalPadding)
            .padding(.horizontal, FloatingStripContentLayout.horizontalPadding)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .aiMeterFontScope(.content(model.displayFontChoice))
        .onReceive(NotificationCenter.default.publisher(for: .aiMeterOpenSettings)) { _ in
            NSApp.activate(ignoringOtherApps: true)
            openSettings()
        }
        .onChange(of: session.selectedProvider) { oldValue, newValue in
            if let oldValue, newValue == nil {
                accessibilityFocusedProvider = oldValue
            }
        }
    }

    private var density: FloatingStripDensity { model.stripPreferences.density }

    private var accessibilityPositionValue: String {
        let edge = displayState.resolvedEdge == .left ? "Left edge" : "Right edge"
        let verticalPercent = Int((displayState.normalizedCenterY * 100).rounded())
        return "\(edge), vertical position \(verticalPercent) percent"
    }

    private var presentations: [ProviderPresentation] {
        model.stripPreferences.visibleProviders.map { provider in
            if let snapshot = model.snapshots.first(where: { $0.provider == provider }) {
                return ProviderPresentation(snapshot: snapshot)
            }
            return ProviderPresentation(snapshot: UsageSnapshot(
                provider: provider,
                availability: .unknown,
                collectionStatus: .refreshing
            ))
        }
    }
}

struct FloatingDetailView: View {
    @Bindable var model: AppModel
    let provider: UsageProvider
    let onClaudeSetup: () -> Void
    let onOpenServicesSettings: () -> Void
    let onInteractionChange: (Bool) -> Void

    @ViewBuilder
    var body: some View {
        Group {
            if let snapshot = model.snapshots.first(where: { $0.provider == provider }) {
                if provider == .deepSeek {
                    DeepSeekAnalyticsView(
                        snapshot: snapshot,
                        webSession: model.deepSeekWebSession,
                        isDemoMode: model.isRunningDemoMode,
                        onInteractionChange: onInteractionChange,
                        onOpenServicesSettings: onOpenServicesSettings
                    )
                } else if provider == .gemini {
                    GeminiDetailView(snapshot: snapshot) {
                        Task { await model.checkServiceAccount(.gemini) }
                    }
                    .onHover(perform: onInteractionChange)
                } else if provider == .codex {
                    CodexDetailView(
                        snapshot: snapshot,
                        onOpenServicesSettings: onOpenServicesSettings
                    )
                        .onHover(perform: onInteractionChange)
                } else {
                    ClaudeDetailView(
                        snapshot: snapshot,
                        onSetup: onClaudeSetup,
                        onOpenServicesSettings: onOpenServicesSettings
                    )
                        .onHover(perform: onInteractionChange)
                }
            }
        }
        .aiMeterFontScope(.content(model.displayFontChoice))
    }

}
