import AIMeterCore
import Foundation

/// macOS display adapter. The shared presentation retains quota and status decisions.
struct AppProviderPresentation {
    let snapshot: UsageSnapshot
    let localizer: AppLocalizer
    var core: ProviderPresentation { ProviderPresentation(snapshot: snapshot) }
    var title: String { core.title }
    var valueText: String { ProviderDetailText.value(core.valueText, localizer: localizer) }
    var detailText: String { ProviderDetailText.metricLabel(core.detailText, localizer: localizer) }
    var statusText: String? { core.statusText.map { ProviderDetailText.diagnostic($0, localizer: localizer) } }
    var primaryResetText: String? { ProviderDetailText.reset(snapshot.primaryMetric, localizer: localizer) }
    var secondaryResetText: String? { ProviderDetailText.reset(snapshot.secondaryMetric, localizer: localizer) }
    var semantic: UsageSemantic { core.semantic }
}

extension AppLocalizer {
    /// Roots inject the explicit app locale. Standalone previews retain the English default.
    init(locale: Locale) {
        self.init(language: AppLanguage(rawValue: locale.identifier.replacingOccurrences(of: "_", with: "-")) ?? .english)
    }
}

enum ProviderDetailText {
    // Only app-authored values are translation keys. Raw server/CLI text can happen
    // to equal other UI keys and must not be sent through the general table lookup.
    private static let metricLabels: Set<String> = [
        "Current session", "All models", "Current week (all models)", "Current week (Sonnet only)",
        "5h limit", "Weekly limit", "Usage limit", "Available balance", "Balance baseline", "Monthly budget",
        "Gemini · Five hour", "Gemini · Weekly", "Claude/GPT · Five hour", "Claude/GPT · Weekly",
        "Account connection required", "One-time Claude Code workspace approval", "CLI was not found",
        "Checking current usage", "Usage format changed", "No current data", "Usage",
    ]

    private static let diagnosticKeys: Set<String> = [
        "Account unavailable", "CLI not installed", "Sign in required", "Approve the private usage workspace once",
        "Usage format is not recognized", "Rate limited; try again later", "Request timed out", "Service temporarily unavailable",
        "Antigravity CLI is not executable", "Antigravity CLI environment uses an unsupported override",
        "Antigravity CLI version is not supported (requires 1.1.28 or later in major version 1)",
        "Antigravity CLI quota is currently unavailable. Installation and account status have not been checked.",
        "Antigravity CLI account status is not available. Installation and sign-in have not been checked.",
        "Official Antigravity CLI quota verified; account identity not provided", "Antigravity CLI quota unavailable",
        "Showing cached usage", "Using cached usage", "Usage is visible but could not be cached",
        "DeepSeek usage page could not be loaded", "Sign in if prompted, or open DeepSeek usage in your browser",
        "Using cached usage; automatic sync needs attention", "Synced from DeepSeek", "Demo usage", "Quota unavailable",
    ]

    static func diagnostic(_ message: String, localizer: AppLocalizer) -> String {
        diagnosticKeys.contains(message) ? localizer.text(message) : message
    }

    static func creditTitle(_ title: String, localizer: AppLocalizer) -> String {
        ["Usage reset", "Bonus reset"].contains(title) ? localizer.text(title) : title
    }

    static func metricLabel(_ label: String, localizer: AppLocalizer) -> String {
        if label.hasSuffix("m limit"), let minutes = Int64(label.dropLast(7)) {
            return localizer.text("%lldm limit", minutes)
        }
        return metricLabels.contains(label) ? localizer.text(label) : label
    }

    static func value(_ value: String, localizer: AppLocalizer) -> String {
        for suffix in [" tokens", " requests"] where value.hasSuffix(suffix) {
            return localizer.text("%@" + suffix, String(value.dropLast(suffix.count)))
        }
        return localizer.text(value)
    }

    static func reset(_ metric: UsageMetric?, localizer: AppLocalizer, prefersTimestamp: Bool = false) -> String? {
        guard let metric else { return nil }
        if prefersTimestamp, let resetAt = metric.resetAt {
            return localizer.text("Resets %@", localizer.date(resetAt))
        }
        // Retain the existing description-first behavior. Unknown CLI payloads stay verbatim.
        if let raw = metric.resetDescription {
            let description = SensitiveTextRedactor.redact(raw)
            if ["Resets at midnight", "Resets Friday"].contains(description) { return localizer.text(description) }
            if description.hasPrefix("Resets in ") {
                let duration = String(description.dropFirst(10))
                return localizer.text("Resets in %@", localizedDuration(duration, localizer: localizer))
            }
            if description.hasPrefix("Resets ") {
                return localizer.text("Resets %@", String(description.dropFirst(7)))
            }
            return description
        }
        return metric.resetAt.map { localizer.text("Resets %@", localizer.date($0)) }
    }

    static func freshness(_ snapshot: UsageSnapshot, now: Date = Date(), localizer: AppLocalizer) -> String {
        let state = ProviderDataState.freshness(snapshot, now: now)
        if state.hasPrefix("Cached · ") {
            let minutes = Int64(max(now.timeIntervalSince(snapshot.fetchedAt), 0) / 60)
            return localizer.text("Cached · %lld min ago", minutes)
        }
        return localizer.text(state)
    }

    static func localStreak(_ summary: CodexLocalActivitySummary, localizer: AppLocalizer) -> String {
        localizer.text(summary.currentStreakDays == 1 ? "%lld day" : "%lld days", Int64(summary.currentStreakDays))
    }

    static func localDuration(_ summary: CodexLocalActivitySummary, localizer: AppLocalizer) -> String {
        // The shared model continues to own rounding and choice of duration components.
        localizedDuration(CodexLocalActivityPresentation(summary: summary).longestSessionText, localizer: localizer)
    }

    private static func localizedDuration(_ duration: String, localizer: AppLocalizer) -> String {
        guard localizer.language != .english else { return duration }
        let tokens = duration.split(separator: " ")
        let unitNames: [Substring: Character] = [
            "d": "d", "day": "d", "days": "d", "h": "h", "hr": "h", "hour": "h", "hours": "h",
            "m": "m", "min": "m", "mins": "m", "minute": "m", "minutes": "m",
        ]
        var parts: [(Int64, Character)] = []
        var index = 0
        while index < tokens.count {
            let token = tokens[index]
            if let number = Int64(token), index + 1 < tokens.count, let suffix = unitNames[tokens[index + 1]] {
                parts.append((number, suffix))
                index += 2
            } else if let suffix = token.last, "dhm".contains(suffix), let number = Int64(token.dropLast()) {
                parts.append((number, suffix))
                index += 1
            } else {
                return duration
            }
        }
        guard !parts.isEmpty else { return duration }
        switch parts.map(\.1) {
        case ["d", "h"]: return localizer.text("%lldd %lldh", parts[0].0, parts[1].0)
        case ["h", "m"]: return localizer.text("%lldh %lldm", parts[0].0, parts[1].0)
        case ["d"]: return localizer.text("%lldd", parts[0].0)
        case ["h"]: return localizer.text("%lldh", parts[0].0)
        case ["m"]: return localizer.text("%lldm", parts[0].0)
        default: return duration
        }
    }

    static func compactCount(_ count: Int64, localizer: AppLocalizer) -> String {
        let value = Double(max(count, 0))
        let units: [(Double, String)] = [(1_000_000_000, "B"), (1_000_000, "M"), (1_000, "K")]
        guard let unit = units.first(where: { value >= $0.0 }) else { return localizer.number(count) }
        let scaled = value / unit.0
        let digits = scaled >= 100 ? 0 : 1
        return String(format: "%.*f", locale: localizer.language.locale, digits, scaled)
            .replacingOccurrences(of: ".0", with: "") + unit.1
    }

    static func creditStatus(_ state: CodexResetCreditExpirationState, localizer: AppLocalizer) -> String {
        switch state {
        case .remaining(let days): localizer.text(days == 1 ? "%lld day remaining" : "%lld days remaining", Int64(days))
        case .today: localizer.text("Expires today")
        case .expired: localizer.text("Expired")
        case .unavailable: localizer.text("Expiration unavailable")
        }
    }

    static func accountText(_ status: ServiceAccountStatus, localizer: AppLocalizer) -> String {
        if let label = status.accountLabel, !label.isEmpty {
            let owned = ["Connected account", "API Key account", "ChatGPT account", "Demo Claude Code account"]
            return owned.contains(label) ? localizer.text(label) : label
        }
        let key = switch status.connectionState {
        case .connected: "Connected"
        case .signInRequired: status.provider == .deepSeek ? "No API Key stored" : "Sign-in required"
        case .notInstalled: "CLI not installed"
        case .checking: "Checking account…"
        case .unavailable: "Account status unavailable"
        }
        return localizer.text(key)
    }

    @MainActor
    static func deepSeekSync(_ state: DeepSeekWebSession.SyncState, isDemo: Bool, localizer: AppLocalizer) -> String {
        if isDemo { return localizer.text("Preview data") }
        switch state {
        case .signedOut: return localizer.text("Official sign-in required")
        case .loading: return localizer.text("Syncing official usage…")
        case .ready: return localizer.text("Official usage synced")
        case .stale(let message): return diagnostic(message, localizer: localizer)
        }
    }

    static func ringAccessibility(_ presentation: ProviderPresentation, localizer: AppLocalizer) -> String {
        let semantic: String? = switch presentation.semantic {
        case .normal: nil
        case .warning: "Warning"
        case .critical: "Critical usage"
        case .stale: "Cached data"
        case .unavailable: "Unavailable"
        }
        return [presentation.title, value(presentation.valueText, localizer: localizer),
                metricLabel(presentation.detailText, localizer: localizer),
                semantic.map { localizer.text($0) }, presentation.statusText.map { diagnostic($0, localizer: localizer) }]
            .compactMap { $0 }.joined(separator: ", ")
    }
}
