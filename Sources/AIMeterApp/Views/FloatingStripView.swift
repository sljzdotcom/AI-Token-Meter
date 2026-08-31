import AIMeterCore
import SwiftUI

struct FloatingStripView: View {
    @Bindable var model: AppModel
    @Bindable var session: FloatingDetailSession
    @Bindable var displayState: FloatingStripDisplayState
    let onProviderTap: (UsageProvider) -> Void
    let onStripDragChanged: (CGSize) -> Void
    let onStripDragEnded: (CGSize) -> Void

    var body: some View {
        VStack(spacing: 12) {
            Capsule()
                .fill(Color.white.opacity(displayState.isDragging ? 0.5 : 0.24))
                .frame(width: 25, height: 3)
                .frame(width: 50, height: 18)
                .contentShape(Rectangle())
                .gesture(
                    DragGesture(minimumDistance: 1, coordinateSpace: .global)
                        .onChanged { value in
                            displayState.isDragging = true
                            onStripDragChanged(value.translation)
                        }
                        .onEnded { value in
                            displayState.isDragging = false
                            onStripDragEnded(value.translation)
                        }
                )
                .accessibilityLabel("Move floating meter")
                .accessibilityHint("Drag vertically, or drag to another edge in Automatic mode")
            ForEach(presentations, id: \.provider) { presentation in
                Button {
                    onProviderTap(presentation.provider)
                } label: {
                    UsageRing(presentation: presentation, size: 60)
                        .scaleEffect(session.selectedProvider == presentation.provider ? 1.06 : 1)
                }
                .buttonStyle(.plain)
                .accessibilityValue(session.accessibilityValue(for: presentation.provider))
                .animation(
                    .spring(response: 0.28, dampingFraction: 0.8),
                    value: session.selectedProvider
                )
            }
        }
        .padding(.vertical, 17)
        .padding(.horizontal, 11)
        .background(
            FloatingStripShape(edge: displayState.resolvedEdge)
                .fill(AIMeterVisualTheme.floatingGlass)
                .shadow(
                    color: .black.opacity(0.34),
                    radius: 18,
                    x: displayState.resolvedEdge == .right ? -6 : 6,
                    y: 8
                )
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity)
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
                    Label(presentation.title, systemImage: snapshot.provider.symbolName)
                        .font(.headline)
                    Spacer()
                    Text(presentation.valueText)
                        .font(.system(.title2, design: .rounded, weight: .bold))
                        .foregroundStyle(presentation.semantic.color)
                }
                Text(presentation.detailText)
                    .font(.caption)
                    .foregroundStyle(.white.opacity(0.7))
                if let reset = presentation.primaryResetText {
                    Text(reset)
                        .font(.caption2)
                        .foregroundStyle(.white.opacity(0.55))
                }
                if let secondary = snapshot.secondaryMetric,
                   let fraction = secondary.usedFraction {
                    VStack(alignment: .leading, spacing: 5) {
                        HStack {
                            Text(secondary.label)
                            Spacer()
                            Text("\(Int((fraction * 100).rounded()))%")
                        }
                        ProgressView(value: fraction)
                            .tint(presentation.semantic.color)
                        if let reset = presentation.secondaryResetText {
                            Text(reset)
                                .foregroundStyle(.white.opacity(0.55))
                        }
                    }
                    .font(.caption2)
                }
                if let status = presentation.statusText {
                    Text(status)
                        .font(.caption2)
                        .foregroundStyle(.white.opacity(0.6))
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
                    .foregroundStyle(.white.opacity(0.45))
            }
            .foregroundStyle(.white)
            .padding(16)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .background(
                RoundedRectangle(cornerRadius: 22, style: .continuous)
                    .fill(Color.black.opacity(0.94))
                    .shadow(color: .black.opacity(0.24), radius: 18, x: -4, y: 8)
            )
            .padding(5)
    }
}
