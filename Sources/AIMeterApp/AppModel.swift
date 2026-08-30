import AIMeterCore
import Foundation
import Observation

@MainActor
@Observable
final class AppModel {
    private enum DefaultsKey {
        static let showFloatingStrip = "showFloatingStrip"
        static let notificationsEnabled = "notificationsEnabled"
        static let monthlyBudget = "monthlyBudget"
        static let deepSeekBalanceBaseline = "deepSeekBalanceBaseline"
        static let thresholdEvaluator = "thresholdEvaluator"
    }

    private let coordinator: RefreshCoordinator
    private let secretStore: any SecretStore
    private let launchAtLoginService: LaunchAtLoginService
    private let claudeWorkspaceSetupLauncher: ClaudeWorkspaceSetupLauncher
    private let defaults: UserDefaults
    private let detailAutoHidePreferenceStore: DetailAutoHidePreferenceStore
    private let isDemoMode: Bool
    private var thresholdEvaluator: ThresholdEvaluator
    private var refreshLoop: Task<Void, Never>?

    let deepSeekWebSession: DeepSeekWebSession

    private(set) var snapshots: [UsageSnapshot] = []
    private(set) var isRefreshing = false
    private(set) var lastUpdatedAt: Date?
    private(set) var apiKeyConfigured = false
    private(set) var launchAtLoginEnabled = false
    private(set) var settingsMessage: String?

    var showFloatingStrip: Bool
    var notificationsEnabled: Bool
    var deepSeekBalanceBaseline: Double
    var detailAutoHideSeconds: Int

    var floatingVisibilityHandler: ((Bool) -> Void)?
    var notificationHandler: (([ThresholdEvent]) -> Void)?
    var notificationPermissionHandler: (() -> Void)?

    init(
        defaults: UserDefaults = .standard,
        secretStore: any SecretStore = KeychainStore(),
        launchAtLoginService: LaunchAtLoginService = LaunchAtLoginService(),
        claudeWorkspaceSetupLauncher: ClaudeWorkspaceSetupLauncher = ClaudeWorkspaceSetupLauncher()
    ) {
        self.defaults = defaults
        self.secretStore = secretStore
        self.launchAtLoginService = launchAtLoginService
        self.claudeWorkspaceSetupLauncher = claudeWorkspaceSetupLauncher
        self.detailAutoHidePreferenceStore = DetailAutoHidePreferenceStore(defaults: defaults)
        self.isDemoMode = ProcessInfo.processInfo.environment["AI_METER_DEMO_MODE"] == "1"
        if let data = defaults.data(forKey: DefaultsKey.thresholdEvaluator),
           let restored = try? JSONDecoder().decode(ThresholdEvaluator.self, from: data) {
            self.thresholdEvaluator = restored
        } else {
            self.thresholdEvaluator = ThresholdEvaluator()
        }

        if defaults.object(forKey: DefaultsKey.showFloatingStrip) == nil {
            showFloatingStrip = true
        } else {
            showFloatingStrip = defaults.bool(forKey: DefaultsKey.showFloatingStrip)
        }
        notificationsEnabled = defaults.bool(forKey: DefaultsKey.notificationsEnabled)
        let storedBaseline = defaults.double(forKey: DefaultsKey.deepSeekBalanceBaseline)
        let legacyBudget = defaults.double(forKey: DefaultsKey.monthlyBudget)
        let resolvedBaseline = storedBaseline > 0 ? storedBaseline : (legacyBudget > 0 ? legacyBudget : 100)
        deepSeekBalanceBaseline = resolvedBaseline
        defaults.set(resolvedBaseline, forKey: DefaultsKey.deepSeekBalanceBaseline)
        detailAutoHideSeconds = detailAutoHidePreferenceStore.load().rawValue

        let cacheDirectory = FileManager.default.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        )[0].appendingPathComponent("AI Meter", isDirectory: true)
        deepSeekWebSession = DeepSeekWebSession(
            historyStore: DeepSeekHistoryStore(directoryURL: cacheDirectory)
        )
        coordinator = RefreshCoordinator(
            collectors: isDemoMode
                ? []
                : [ClaudeCollector(), CodexCollector(), DeepSeekCollector(secretStore: secretStore)],
            cache: SnapshotCache(directoryURL: cacheDirectory)
        )
        apiKeyConfigured = isDemoMode ? false : ((try? secretStore.read()) ?? nil)?.isEmpty == false
        launchAtLoginEnabled = launchAtLoginService.isEnabled
    }

    var presentations: [ProviderPresentation] {
        snapshots.map(ProviderPresentation.init(snapshot:))
    }

    var menuBarSummary: MenuBarSummary {
        MenuBarSummary(snapshots: snapshots)
    }

    var isRunningDemoMode: Bool { isDemoMode }

    func start() {
        guard refreshLoop == nil else { return }
        deepSeekWebSession.onHistoryChange = { [weak self] history in
            self?.attachDeepSeekHistory(history)
        }
        if isDemoMode {
            snapshots = Self.demoSnapshots
            lastUpdatedAt = Date()
            return
        }
        if notificationsEnabled {
            notificationPermissionHandler?()
        }
        refreshLoop = Task { [weak self] in
            guard let self else { return }
            await refresh()
            while !Task.isCancelled {
                do {
                    try await Task.sleep(for: .seconds(300))
                } catch {
                    return
                }
                await refresh()
            }
        }
    }

    func refresh() async {
        if isDemoMode {
            snapshots = Self.demoSnapshots
            lastUpdatedAt = Date()
            return
        }
        guard !isRefreshing else { return }
        isRefreshing = true
        defer { isRefreshing = false }

        let collected = await coordinator.refresh()
        snapshots = collected.map(applyingLocalBudget).map(applyingDeepSeekHistory)
        lastUpdatedAt = Date()

        guard notificationsEnabled else { return }
        let events = snapshots.flatMap { thresholdEvaluator.evaluate($0) }
        persistThresholdEvaluator()
        if !events.isEmpty {
            notificationHandler?(events)
        }
    }

    func setFloatingStripVisible(_ isVisible: Bool) {
        showFloatingStrip = isVisible
        defaults.set(isVisible, forKey: DefaultsKey.showFloatingStrip)
        floatingVisibilityHandler?(isVisible)
    }

    func setNotificationsEnabled(_ isEnabled: Bool) {
        notificationsEnabled = isEnabled
        defaults.set(isEnabled, forKey: DefaultsKey.notificationsEnabled)
        if isEnabled {
            notificationPermissionHandler?()
        }
    }

    func setDeepSeekBalanceBaseline(_ value: Double) {
        deepSeekBalanceBaseline = max(value, 1)
        defaults.set(deepSeekBalanceBaseline, forKey: DefaultsKey.deepSeekBalanceBaseline)
        snapshots = snapshots.map(applyingLocalBudget)
    }

    func setDetailAutoHideSeconds(_ seconds: Int) {
        let interval = DetailAutoHideInterval(storedSeconds: seconds)
        detailAutoHideSeconds = interval.rawValue
        detailAutoHidePreferenceStore.save(interval)
    }

    func setLaunchAtLogin(_ isEnabled: Bool) {
        do {
            try launchAtLoginService.setEnabled(isEnabled)
            launchAtLoginEnabled = launchAtLoginService.isEnabled
            settingsMessage = nil
        } catch {
            launchAtLoginEnabled = launchAtLoginService.isEnabled
            settingsMessage = "macOS could not update Login Items."
        }
    }

    func openClaudeWorkspaceSetup() {
        do {
            try claudeWorkspaceSetupLauncher.open()
            settingsMessage = "Approve the private AI Meter workspace in Terminal, then refresh."
        } catch {
            settingsMessage = "Claude workspace setup could not be opened."
        }
    }

    func saveDeepSeekAPIKey(_ apiKey: String) {
        do {
            let normalized = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !normalized.isEmpty else {
                settingsMessage = "Enter a DeepSeek API Key first."
                return
            }
            try secretStore.save(normalized)
            apiKeyConfigured = true
            settingsMessage = "DeepSeek API Key saved in Keychain."
            Task { await refresh() }
        } catch {
            settingsMessage = "The API Key could not be saved to Keychain."
        }
    }

    func removeDeepSeekAPIKey() {
        do {
            try secretStore.delete()
            apiKeyConfigured = false
            settingsMessage = "DeepSeek API Key removed."
            Task { await refresh() }
        } catch {
            settingsMessage = "The API Key could not be removed from Keychain."
        }
    }

    private func applyingLocalBudget(to snapshot: UsageSnapshot) -> UsageSnapshot {
        guard snapshot.provider == .deepSeek,
              let balanceMetric = snapshot.primaryMetric,
              balanceMetric.kind == .balance else {
            return snapshot
        }

        let depleted = min(max(deepSeekBalanceBaseline - balanceMetric.current, 0), deepSeekBalanceBaseline)
        let budgetMetric = UsageMetric(
            label: "Balance baseline",
            current: depleted,
            limit: deepSeekBalanceBaseline,
            unit: balanceMetric.unit,
            kind: .localBudget
        )
        return UsageSnapshot(
            provider: snapshot.provider,
            primaryMetric: balanceMetric,
            secondaryMetric: budgetMetric,
            availability: snapshot.availability,
            fetchedAt: snapshot.fetchedAt,
            staleAfter: snapshot.staleAfter,
            sourceVersion: snapshot.sourceVersion,
            collectionStatus: snapshot.collectionStatus,
            statusMessage: snapshot.statusMessage,
            codexResetCredits: snapshot.codexResetCredits,
            deepSeekUsageHistory: snapshot.deepSeekUsageHistory
        )
    }

    private func applyingDeepSeekHistory(to snapshot: UsageSnapshot) -> UsageSnapshot {
        guard snapshot.provider == .deepSeek,
              let history = deepSeekWebSession.history else { return snapshot }
        return snapshot.withDeepSeekHistory(history)
    }

    private func attachDeepSeekHistory(_ history: DeepSeekUsageHistory) {
        snapshots = snapshots.map { snapshot in
            snapshot.provider == .deepSeek ? snapshot.withDeepSeekHistory(history) : snapshot
        }
    }

    private func persistThresholdEvaluator() {
        guard let data = try? JSONEncoder().encode(thresholdEvaluator) else { return }
        defaults.set(data, forKey: DefaultsKey.thresholdEvaluator)
    }

    private static var demoSnapshots: [UsageSnapshot] {
        [
            UsageSnapshot(
                provider: .claude,
                primaryMetric: UsageMetric(
                    label: "Current session",
                    current: 73,
                    limit: 100,
                    unit: .percent,
                    resetDescription: "Resets in 51 min"
                ),
                secondaryMetric: UsageMetric(
                    label: "All models",
                    current: 7,
                    limit: 100,
                    unit: .percent,
                    resetDescription: "Resets at midnight"
                )
            ),
            UsageSnapshot(
                provider: .codex,
                primaryMetric: UsageMetric(
                    label: "5h limit",
                    current: 21,
                    limit: 100,
                    unit: .percent,
                    resetDescription: "Resets in 3h 12m"
                ),
                secondaryMetric: UsageMetric(
                    label: "Weekly limit",
                    current: 34,
                    limit: 100,
                    unit: .percent,
                    resetDescription: "Resets Friday"
                ),
                codexResetCredits: CodexResetCreditsSummary(
                    availableCount: 2,
                    credits: [
                        CodexResetCreditDisplay(
                            title: "Usage reset",
                            expiresAt: Calendar.current.date(byAdding: .day, value: 4, to: Date())
                        ),
                        CodexResetCreditDisplay(
                            title: "Bonus reset",
                            expiresAt: Calendar.current.date(byAdding: .day, value: 12, to: Date())
                        ),
                    ],
                    hasCompleteDetails: true
                )
            ),
            UsageSnapshot(
                provider: .deepSeek,
                primaryMetric: UsageMetric(
                    label: "Available balance",
                    current: 48,
                    limit: nil,
                    unit: .cny,
                    kind: .balance
                ),
                secondaryMetric: UsageMetric(
                    label: "Balance baseline",
                    current: 52,
                    limit: 100,
                    unit: .cny,
                    kind: .localBudget
                ),
                deepSeekUsageHistory: DeepSeekUsageHistory(
                    days: (0..<30).map { offset in
                        DeepSeekDailyUsage(
                            date: Calendar.current.date(byAdding: .day, value: offset - 29, to: Date()) ?? Date(),
                            costCNY: offset.isMultiple(of: 5) ? Double((offset % 7) + 1) * 0.42 : 0.08,
                            requestCount: offset.isMultiple(of: 5) ? 18 + offset : 2,
                            tokenCount: offset.isMultiple(of: 5) ? 120_000 + offset * 1_000 : 8_000
                        )
                    },
                    updatedAt: Date(),
                    statusMessage: "Demo usage"
                )
            ),
        ]
    }
}

private extension UsageSnapshot {
    func withDeepSeekHistory(_ history: DeepSeekUsageHistory) -> UsageSnapshot {
        UsageSnapshot(
            provider: provider,
            primaryMetric: primaryMetric,
            secondaryMetric: secondaryMetric,
            availability: availability,
            fetchedAt: fetchedAt,
            staleAfter: staleAfter,
            sourceVersion: sourceVersion,
            collectionStatus: collectionStatus,
            statusMessage: statusMessage,
            codexResetCredits: codexResetCredits,
            deepSeekUsageHistory: history
        )
    }
}
