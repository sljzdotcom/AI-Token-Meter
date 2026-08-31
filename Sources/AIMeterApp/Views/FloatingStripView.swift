import AIMeterCore
import AppKit
import SwiftUI

struct FloatingStripView: View {
    @Bindable var model: AppModel
    @Bindable var session: FloatingDetailSession
    @Bindable var displayState: FloatingStripDisplayState
    let onProviderTap: (UsageProvider) -> Void
    let onStripDragChanged: (CGSize) -> Void
    let onStripDragEnded: (CGSize) -> Void
    let onAccessibilityMove: (FloatingStripAccessibilityCommand) -> Void
    @AccessibilityFocusState private var accessibilityFocusedProvider: UsageProvider?

    var body: some View {
        ZStack {
            FloatingStripSurface(edge: displayState.resolvedEdge)
                .contentShape(FloatingStripDragShape(edge: displayState.resolvedEdge), eoFill: true)
                .gesture(stripDragGesture)
                .focusable()
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
                        NSCursor.openHand.set()
                    case .ended:
                        NSCursor.arrow.set()
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

    private var stripDragGesture: some Gesture {
        DragGesture(minimumDistance: 1, coordinateSpace: .global)
            .onChanged { value in
                displayState.isDragging = true
                NSCursor.closedHand.set()
                onStripDragChanged(value.translation)
            }
            .onEnded { value in
                displayState.isDragging = false
                NSCursor.openHand.set()
                onStripDragEnded(value.translation)
            }
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
                compactDetail(snapshot)
                    .onHover(perform: onInteractionChange)
            }
        }
    }

    private func compactDetail(_ snapshot: UsageSnapshot) -> some View {
        let presentation = ProviderPresentation(snapshot: snapshot)
        return VStack(alignment: .leading, spacing: 10) {
                HStack {
                    HStack(spacing: 9) {
                        ProviderLogo(provider: snapshot.provider, size: 25)
                        Text(presentation.title)
                            .font(.headline)
                    }
                    Spacer()
                    Text(presentation.valueText)
                        .font(.system(.title2, design: .rounded, weight: .bold))
                        .foregroundStyle(detailValueStyle(presentation))
                }
                Text(presentation.detailText)
                    .font(.caption)
                    .foregroundStyle(AIMeterVisualTheme.secondaryText)
                if let reset = presentation.primaryResetText {
                    Text(reset)
                        .font(.caption2)
                        .foregroundStyle(AIMeterVisualTheme.tertiaryText)
                }
                if let secondary = snapshot.secondaryMetric,
                   let fraction = secondary.usedFraction {
                    VStack(alignment: .leading, spacing: 5) {
                        HStack {
                            Text(secondary.label)
                            Spacer()
                            Text("\(Int((fraction * 100).rounded()))%")
                        }
                        AIMeterProgressBar(
                            fraction: fraction,
                            semantic: presentation.semantic
                        )
                        if let reset = presentation.secondaryResetText {
                            Text(reset)
                                .foregroundStyle(AIMeterVisualTheme.tertiaryText)
                        }
                    }
                    .font(.caption2)
                }
                if let status = presentation.statusText {
                    Text(status)
                        .font(.caption2)
                        .foregroundStyle(AIMeterVisualTheme.secondaryText)
                }
                if snapshot.provider == .claude,
                   snapshot.collectionStatus == .setupRequired {
                    Button("Open one-time setup", action: onClaudeSetup)
                        .buttonStyle(.borderedProminent)
                        .controlSize(.small)
                }
                Spacer(minLength: 0)
                Text("Updated \(snapshot.fetchedAt.formatted(date: .omitted, time: .shortened))")
                    .font(.caption2)
                    .foregroundStyle(AIMeterVisualTheme.tertiaryText)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .aiMeterDetailSurface()
    }

    private func detailValueStyle(_ presentation: ProviderPresentation) -> AnyShapeStyle {
        if presentation.semantic == .normal {
            return AnyShapeStyle(AIMeterVisualTheme.accentGradient)
        }
        return AnyShapeStyle(presentation.semantic.color)
    }
}
