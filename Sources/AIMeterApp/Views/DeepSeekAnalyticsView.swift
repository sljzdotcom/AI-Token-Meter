import AIMeterCore
import Charts
import SwiftUI
import WebKit

struct DeepSeekAnalyticsView: View {
    let snapshot: UsageSnapshot
    @Bindable var webSession: DeepSeekWebSession
    let isDemoMode: Bool
    let onInteractionChange: (Bool) -> Void
    @State private var isHovering = false

    private var presentation: ProviderPresentation {
        ProviderPresentation(snapshot: snapshot)
    }

    private var history: DeepSeekUsageHistory? {
        webSession.history ?? snapshot.deepSeekUsageHistory
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            header
            if shouldShowWebPage {
                loginPanel
            } else if let history {
                analytics(history)
            } else {
                unavailablePanel
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .aiMeterDetailSurface()
        .onAppear {
            if !isDemoMode {
                webSession.syncIfNeeded()
            }
            updateAutoHidePause()
        }
        .onDisappear {
            onInteractionChange(false)
        }
        .onHover { hovering in
            isHovering = hovering
            updateAutoHidePause()
        }
        .onChange(of: webSession.state) { _, _ in
            updateAutoHidePause()
        }
    }

    private var header: some View {
        HStack(spacing: 12) {
            ProviderLogo(provider: .deepSeek, size: 28)
            VStack(alignment: .leading, spacing: 2) {
                Text("DeepSeek · Last 30 days")
                    .aiMeterFont(.headline)
                    .foregroundStyle(accentStyle)
                Text(syncText)
                    .aiMeterFont(.caption2)
                    .foregroundStyle(AIMeterVisualTheme.secondaryText)
            }
            Spacer()
            VStack(alignment: .trailing, spacing: 1) {
                Text(presentation.valueText)
                    .aiMeterFont(.title2, design: .rounded, weight: .bold)
                    .foregroundStyle(accentStyle)
                Text("current balance")
                    .aiMeterFont(.caption2)
                    .foregroundStyle(AIMeterVisualTheme.tertiaryText)
            }
            Button {
                webSession.syncIfNeeded(force: true)
            } label: {
                Image(systemName: "arrow.clockwise")
                    .aiMeterSymbolFont(.body)
            }
            .buttonStyle(.borderless)
            .help("Refresh from DeepSeek")
        }
    }

    private var shouldShowWebPage: Bool {
        if isDemoMode { return false }
        if case .signedOut = webSession.state { return true }
        return history == nil
    }

    private var loginPanel: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Sign in on the official DeepSeek page once. \(AppBrand.displayName) keeps the web session on this Mac and stores only daily totals.")
                .aiMeterFont(.caption)
                .foregroundStyle(AIMeterVisualTheme.secondaryText)
            DeepSeekWebView(webView: webSession.webView)
                .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: 14, style: .continuous)
                        .stroke(
                            UsageProvider.deepSeek.accentPalette.startColor.opacity(0.42),
                            lineWidth: 1
                        )
                )
        }
    }

    private func analytics(_ history: DeepSeekUsageHistory) -> some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(spacing: 12) {
                statCard(title: "Cost", value: String(format: "¥%.2f", history.totalCostCNY))
                statCard(title: "API requests", value: history.totalRequests.formatted())
                statCard(title: "Tokens", value: history.totalTokens.formatted())
            }
            VStack(alignment: .leading, spacing: 8) {
                Text("Daily cost (CNY)")
                    .aiMeterFont(.subheadline, weight: .semibold)
                Chart(history.days) { day in
                    BarMark(
                        x: .value("Day", day.date, unit: .day),
                        y: .value("Cost", day.costCNY)
                    )
                    .foregroundStyle(accentStyle)
                    .cornerRadius(3)
                }
                .chartXAxis {
                    AxisMarks(values: .stride(by: .day, count: 7)) { value in
                        AxisGridLine().foregroundStyle(Color.white.opacity(0.08))
                        AxisValueLabel(format: .dateTime.month(.abbreviated).day())
                    }
                }
                .chartYAxis {
                    AxisMarks(position: .leading) { value in
                        AxisGridLine().foregroundStyle(Color.white.opacity(0.10))
                        AxisValueLabel()
                    }
                }
                .frame(minHeight: 260)
            }
            .padding(14)
            .aiMeterGlassCard()
            HStack {
                Text("Updated \(history.updatedAt.formatted(date: .abbreviated, time: .shortened))")
                Spacer()
                Link("Open official usage page", destination: DeepSeekWebSession.usageURL)
            }
            .aiMeterFont(.caption2)
            .foregroundStyle(AIMeterVisualTheme.secondaryText)
        }
    }

    private var unavailablePanel: some View {
        VStack(spacing: 12) {
            Image(systemName: "chart.bar.xaxis")
                .font(.largeTitle)
            Text(syncText)
            Link("Open official usage page", destination: DeepSeekWebSession.usageURL)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .foregroundStyle(AIMeterVisualTheme.secondaryText)
    }

    private func statCard(title: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: 7) {
            Text(title)
                .aiMeterFont(.caption)
                .foregroundStyle(AIMeterVisualTheme.secondaryText)
            Text(value)
                .aiMeterFont(.title2, design: .rounded, weight: .semibold)
                .foregroundStyle(accentStyle)
                .lineLimit(1)
                .minimumScaleFactor(0.7)
        }
        .padding(13)
        .frame(maxWidth: .infinity, alignment: .leading)
        .aiMeterGlassCard()
    }

    private var syncText: String {
        if isDemoMode { return "Preview data" }
        return switch webSession.state {
        case .signedOut: "Official sign-in required"
        case .loading: "Syncing official usage…"
        case .ready: "Official usage synced"
        case .stale(let message): message
        }
    }

    private var accentStyle: AnyShapeStyle {
        presentation.semantic.accentStyle(for: .deepSeek)
    }

    private func updateAutoHidePause() {
        onInteractionChange(isHovering || (!isDemoMode && webSession.shouldPauseAutoHide))
    }
}

private struct DeepSeekWebView: NSViewRepresentable {
    let webView: WKWebView

    func makeNSView(context: Context) -> WKWebView {
        webView
    }

    func updateNSView(_ nsView: WKWebView, context: Context) {}
}
