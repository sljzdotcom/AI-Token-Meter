import AIMeterCore
import Charts
import SwiftUI
import WebKit

struct DeepSeekAnalyticsView: View {
    @Environment(\.locale) private var locale
    private var localizer: AppLocalizer { AppLocalizer(locale: locale) }
    let snapshot: UsageSnapshot
    @Bindable var webSession: DeepSeekWebSession
    let isDemoMode: Bool
    let onInteractionChange: (Bool) -> Void
    let onOpenServicesSettings: () -> Void
    @State private var isHovering = false

    private var presentation: AppProviderPresentation {
        AppProviderPresentation(snapshot: snapshot, localizer: localizer)
    }

    private var history: DeepSeekUsageHistory? {
        webSession.history ?? snapshot.deepSeekUsageHistory
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            header
            if needsServiceRecovery {
                serviceRecoveryPanel
                if let history {
                    analytics(history)
                }
            } else if shouldShowWebPage {
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
                Text(localizer.text("DeepSeek · Last 30 days"))
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
                Text(localizer.text("current balance"))
                    .aiMeterFont(.caption2)
                    .foregroundStyle(AIMeterVisualTheme.tertiaryText)
                Text(ProviderDetailText.freshness(snapshot, localizer: localizer))
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
            .help(localizer.text("Refresh from DeepSeek"))
        }
    }

    private var shouldShowWebPage: Bool {
        if isDemoMode { return false }
        if case .signedOut = webSession.state { return true }
        return history == nil
    }

    private var needsServiceRecovery: Bool {
        ServiceRecoveryPresentation.detailAction(
            provider: .deepSeek,
            status: snapshot.collectionStatus,
            statusMessage: snapshot.statusMessage
        ) == .openServicesSettings
    }

    private var serviceRecoveryPanel: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(snapshot.collectionStatus == .authenticationRequired
                ? localizer.text("Configure a DeepSeek API Key in Services. Official website sign-in only syncs usage history.")
                : presentation.detailText)
                .aiMeterFont(.caption)
                .foregroundStyle(AIMeterVisualTheme.secondaryText)
            if let status = presentation.statusText {
                Text(status)
                    .aiMeterFont(.caption2)
                    .foregroundStyle(AIMeterVisualTheme.tertiaryText)
            }
            Button(localizer.text("Open Services Settings"), action: onOpenServicesSettings)
                .buttonStyle(.borderedProminent)
                .controlSize(.small)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .aiMeterGlassCard()
    }

    private var loginPanel: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(localizer.text("Sign in on the official DeepSeek page once. %@ keeps the web session on this Mac and stores only daily totals.", AppBrand.displayName))
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
                statCard(title: "Cost", value: "¥" + localizer.decimal(history.totalCostCNY, fractionDigits: 2))
                statCard(title: "API requests", value: localizer.number(Int64(history.totalRequests)))
                statCard(title: "Tokens", value: localizer.number(Int64(history.totalTokens)))
            }
            VStack(alignment: .leading, spacing: 8) {
                Text(localizer.text("Daily cost (CNY)"))
                    .aiMeterFont(.subheadline, weight: .semibold)
                Chart(history.days) { day in
                    BarMark(
                        x: .value(localizer.text("Day"), day.date, unit: .day),
                        y: .value(localizer.text("Cost"), day.costCNY)
                    )
                    .foregroundStyle(accentStyle)
                    .cornerRadius(3)
                }
                .chartXAxis {
                    AxisMarks(values: .stride(by: .day, count: 7)) { value in
                        AxisGridLine().foregroundStyle(Color.white.opacity(0.08))
                        AxisValueLabel(format: .dateTime.month(.abbreviated).day().locale(localizer.language.locale))
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
                Text(localizer.text("Updated %@", localizer.date(history.updatedAt)))
                Spacer()
                Link(localizer.text("Open official usage page"), destination: DeepSeekWebSession.usageURL)
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
            Link(localizer.text("Open official usage page"), destination: DeepSeekWebSession.usageURL)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .foregroundStyle(AIMeterVisualTheme.secondaryText)
    }

    private func statCard(title: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: 7) {
            Text(localizer.text(title))
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
        ProviderDetailText.deepSeekSync(webSession.state, isDemo: isDemoMode, localizer: localizer)
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
