import AIMeterCore
import AppKit
import SwiftUI

struct FloatingStripView: View {
    @Bindable var model: AppModel
    @Bindable var session: FloatingDetailSession
    @Bindable var displayState: FloatingStripDisplayState
    let onProviderTap: (UsageProvider) -> Void
    let onAccessibilityMove: (FloatingStripAccessibilityCommand) -> Void
    @AccessibilityFocusState private var accessibilityFocusedProvider: UsageProvider?

    var body: some View {
        ZStack {
            FloatingStripSurface(edge: displayState.resolvedEdge)
                .contentShape(FloatingStripDragShape(edge: displayState.resolvedEdge), eoFill: true)
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
            VStack(spacing: FloatingStripContentLayout.providerSpacing) {
                ForEach(presentations, id: \.provider) { presentation in
                    Button {
                        onProviderTap(presentation.provider)
                    } label: {
                        UsageRing(
                            presentation: presentation,
                            size: FloatingStripContentLayout.providerButtonSize
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
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .aiMeterFontScope(.content(model.displayFontChoice))
        .onChange(of: session.selectedProvider) { oldValue, newValue in
            if let oldValue, newValue == nil {
                accessibilityFocusedProvider = oldValue
            }
        }
    }

    private var accessibilityPositionValue: String {
        let edge = displayState.resolvedEdge == .left ? "Left edge" : "Right edge"
        let verticalPercent = Int((displayState.normalizedCenterY * 100).rounded())
        return "\(edge), vertical position \(verticalPercent) percent"
    }

    private var presentations: [ProviderPresentation] {
        UsageProvider.allCases.map { provider in
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
                        onInteractionChange: onInteractionChange
                    )
                } else if provider == .codex {
                    CodexDetailView(snapshot: snapshot)
                        .onHover(perform: onInteractionChange)
                } else {
                    ClaudeDetailView(snapshot: snapshot, onSetup: onClaudeSetup)
                        .onHover(perform: onInteractionChange)
                }
            }
        }
        .aiMeterFontScope(.content(model.displayFontChoice))
    }

}
