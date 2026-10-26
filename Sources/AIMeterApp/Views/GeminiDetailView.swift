import AIMeterCore
import SwiftUI

enum GeminiDetailPresentation {
    static func remainingText(for metric: UsageMetric, localizer: AppLocalizer = AppLocalizer(language: .english)) -> String {
        localizer.text("%lld%% remaining", Int64(100 - metric.current))
    }

    static func availableModelsText(
        for info: AntigravityCLIInfo,
        localizer: AppLocalizer = AppLocalizer(language: .english)
    ) -> String? {
        guard let count = info.availableModelCount else { return nil }
        return localizer.text(
            "%lld models · %lld families",
            Int64(count),
            Int64(info.modelFamilies.count)
        )
    }

    static func familyText(for info: AntigravityCLIInfo) -> String? {
        info.modelFamilies.isEmpty ? nil : info.modelFamilies.joined(separator: " · ")
    }

    static func showsMissingSharedQuotaNotice(for snapshot: UsageSnapshot) -> Bool {
        snapshot.provider == .gemini
            && snapshot.geminiQuotaMetrics?.isEmpty == false
            && snapshot.antigravitySharedQuotaMetrics?.isEmpty != false
    }
}

struct GeminiDetailView: View {
    @Environment(\.locale) private var locale
    private var localizer: AppLocalizer { AppLocalizer(locale: locale) }
    let snapshot: UsageSnapshot
    let pauseReason: GeminiPauseReason?
    let isPaused: Bool
    let isRecoveryInProgress: Bool
    let canResumeAutomaticRefresh: Bool
    let isResumeInProgress: Bool
    let recoveryMessage: String?
    let onRetry: () -> Void
    let onSignIn: () -> Void
    let onResumeAutomaticRefresh: () -> Void
    let onCopyDiagnostics: () -> Void

    init(
        snapshot: UsageSnapshot,
        pauseReason: GeminiPauseReason? = nil,
        isPaused: Bool = false,
        isRecoveryInProgress: Bool = false,
        canResumeAutomaticRefresh: Bool = false,
        isResumeInProgress: Bool = false,
        recoveryMessage: String? = nil,
        onRetry: @escaping () -> Void,
        onSignIn: @escaping () -> Void = {},
        onResumeAutomaticRefresh: @escaping () -> Void = {},
        onCopyDiagnostics: @escaping () -> Void = {}
    ) {
        self.snapshot = snapshot
        self.pauseReason = pauseReason
        self.isPaused = isPaused
        self.isRecoveryInProgress = isRecoveryInProgress
        self.canResumeAutomaticRefresh = canResumeAutomaticRefresh
        self.isResumeInProgress = isResumeInProgress
        self.recoveryMessage = recoveryMessage
        self.onRetry = onRetry
        self.onSignIn = onSignIn
        self.onResumeAutomaticRefresh = onResumeAutomaticRefresh
        self.onCopyDiagnostics = onCopyDiagnostics
    }

    private var accentStyle: AnyShapeStyle {
        AnyShapeStyle(UsageProvider.gemini.accentPalette.gradient)
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                HStack {
                    ProviderLogo(provider: .gemini)
                    Text("Google Antigravity")
                        .aiMeterFont(.title2, weight: .semibold)
                        .foregroundStyle(accentStyle)
                }
                if let metrics = snapshot.geminiQuotaMetrics, !metrics.isEmpty {
                    Text(localizer.text(snapshot.collectionStatus == .cached ? "Last available Gemini quota" : "Gemini quota"))
                        .aiMeterFont(.headline)
                    ForEach(metrics) { metric in
                        VStack(alignment: .leading, spacing: 4) {
                            HStack {
                                Text(ProviderDetailText.metricLabel(metric.label, localizer: localizer))
                                Spacer()
                                Text(GeminiDetailPresentation.remainingText(for: metric, localizer: localizer))
                                    .monospacedDigit()
                                    .foregroundStyle(accentStyle)
                            }
                            AIMeterProgressBar(
                                provider: .gemini,
                                fraction: metric.usedFraction ?? 0,
                                semantic: .normal
                            )
                            if let reset = ProviderDetailText.reset(metric, localizer: localizer, prefersTimestamp: true) {
                                Text(reset).aiMeterFont(.caption).foregroundStyle(.secondary)
                            }
                        }
                    }
                } else {
                    Text(localizer.text("Quota unavailable")).aiMeterFont(.headline)
                }
                if let metrics = snapshot.antigravitySharedQuotaMetrics, !metrics.isEmpty {
                    Text(localizer.text("Claude/GPT shared quota")).aiMeterFont(.headline)
                    ForEach(metrics) { metric in
                        VStack(alignment: .leading, spacing: 4) {
                            HStack {
                                Text(ProviderDetailText.metricLabel(metric.label, localizer: localizer))
                                Spacer()
                                Text(GeminiDetailPresentation.remainingText(for: metric, localizer: localizer))
                                    .monospacedDigit()
                                    .foregroundStyle(accentStyle)
                            }
                            AIMeterProgressBar(
                                provider: .gemini,
                                fraction: metric.usedFraction ?? 0,
                                semantic: .normal
                            )
                            if let reset = ProviderDetailText.reset(metric, localizer: localizer, prefersTimestamp: true) {
                                Text(reset).aiMeterFont(.caption).foregroundStyle(.secondary)
                            }
                        }
                    }
                } else if GeminiDetailPresentation.showsMissingSharedQuotaNotice(for: snapshot) {
                    Text(localizer.text("Claude/GPT shared quota is not available in this snapshot."))
                        .aiMeterFont(.caption)
                        .foregroundStyle(.secondary)
                }
                if snapshot.antigravityCLIInfo != nil || snapshot.sourceVersion != nil {
                    VStack(alignment: .leading, spacing: 10) {
                        Text(localizer.text("Antigravity CLI")).aiMeterFont(.headline)
                        VStack(alignment: .leading, spacing: 9) {
                            if let currentModel = snapshot.antigravityCLIInfo?.currentModel {
                                CLIInfoRow(label: localizer.text("Current model"), value: currentModel)
                            }
                            if let info = snapshot.antigravityCLIInfo,
                               let available = GeminiDetailPresentation.availableModelsText(for: info, localizer: localizer) {
                                CLIInfoRow(label: localizer.text("Available models"), value: available)
                                if let families = GeminiDetailPresentation.familyText(for: info) {
                                    Text(families)
                                        .aiMeterFont(.caption)
                                        .foregroundStyle(.secondary)
                                        .fixedSize(horizontal: false, vertical: true)
                                }
                            }
                            if let version = snapshot.sourceVersion {
                                CLIInfoRow(label: localizer.text("CLI version"), value: version)
                            }
                        }
                        .padding(12)
                        .background(.white.opacity(0.045), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                        .overlay {
                            RoundedRectangle(cornerRadius: 14, style: .continuous)
                                .stroke(.white.opacity(0.08), lineWidth: 1)
                        }
                    }
                }
                Text(localizer.text("Updated %@", localizer.date(snapshot.fetchedAt)))
                    .aiMeterFont(.caption).foregroundStyle(.secondary)
                if let message = snapshot.statusMessage { Text(ProviderDetailText.diagnostic(message, localizer: localizer)).foregroundStyle(.secondary) }
                if let recoveryMessage {
                    Text(recoveryMessage)
                        .aiMeterFont(.caption)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                let accountStatus = ServiceAccountStatus.fromGeminiSnapshot(snapshot, pauseReason: pauseReason)
                GeminiInstallationHelp(state: accountStatus.connectionState, pauseReason: pauseReason)
                if isPaused {
                    Text(localizer.text(canResumeAutomaticRefresh || isResumeInProgress
                        ? "Your one-time check succeeded. Choose Resume automatic refresh to enable future scheduled checks."
                        : "Complete the official CLI sign-in and run one quota check. Automatic refresh remains paused."))
                        .aiMeterFont(.caption)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                HStack {
                    if GeminiInstallationGuide.shouldOfferInstallation(
                        for: accountStatus.connectionState,
                        pauseReason: pauseReason
                    ) {
                        Link(localizer.text("Antigravity CLI installation guide"), destination: GeminiInstallationGuide.url)
                    }
                    Spacer()
                    if isPaused {
                        GeminiRecoveryActions(
                            canResume: canResumeAutomaticRefresh,
                            isChecking: isRecoveryInProgress,
                            isResuming: isResumeInProgress,
                            onCheck: onSignIn,
                            onResume: onResumeAutomaticRefresh,
                            onCopyDiagnostics: onCopyDiagnostics
                        )
                    } else {
                        Button(localizer.text("Retry"), action: onRetry)
                    }
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .aiMeterDetailSurface()
    }
}

/// Details and Services share one primary action and its progress/disable rules.
struct GeminiRecoveryActions: View {
    @Environment(\.locale) private var locale
    private var localizer: AppLocalizer { AppLocalizer(locale: locale) }
    let canResume: Bool
    let isChecking: Bool
    let isResuming: Bool
    let onCheck: () -> Void
    let onResume: () -> Void
    let onCopyDiagnostics: () -> Void

    var body: some View {
        HStack {
            Button(localizer.text("Copy diagnostic info"), action: onCopyDiagnostics)
                .buttonStyle(.bordered)
            if canResume || isResuming {
                Button(localizer.text("Resume automatic refresh"), action: onResume)
                    .disabled(isChecking || isResuming)
            } else {
                Button(localizer.text("One-time quota check"), action: onCheck)
                    .disabled(isChecking || isResuming)
            }
            if isChecking || isResuming {
                ProgressView().controlSize(.small)
            }
        }
    }
}

private struct CLIInfoRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 12) {
            Text(label).aiMeterFont(.caption).foregroundStyle(.secondary)
            Spacer(minLength: 8)
            Text(value)
                .aiMeterFont(.caption, weight: .medium)
                .multilineTextAlignment(.trailing)
                .textSelection(.enabled)
        }
    }
}
