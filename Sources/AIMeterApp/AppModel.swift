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
        static let thresholdEvaluator = "thresholdEvaluator"
    }

    private let coordinator: RefreshCoordinator
    private let secretStore: any SecretStore
    private let budgetTracker: DeepSeekBudgetTracker
    private let launchAtLoginService: LaunchAtLoginService
    private let claudeWorkspaceSetupLauncher: ClaudeWorkspaceSetupLauncher
    private let defaults: UserDefaults
    private let detailAutoHidePreferenceStore: DetailAutoHidePreferenceStore
    private let isDemoMode: Bool
    private var thresholdEvaluator: ThresholdEvaluator
    private var refreshLoop: Task<Void, Never>?

    private(set) var snapshots: [UsageSnapshot] = []
    private(set) var isRefreshing = false
    private(set) var lastUpdatedAt: Date?
    private(set) var apiKeyConfigured = false
    private(set) var launchAtLoginEnabled = false
    private(set) var settingsMessage: String?

    var showFloatingStrip: Bool
    var notificationsEnabled: Bool
    var monthlyBudget: Double
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
        self.budgetTracker = DeepSeekBudgetTracker(defaults: defaults)
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
        let storedBudget = defaults.double(forKey: DefaultsKey.monthlyBudget)
        monthlyBudget = storedBudget > 0 ? storedBudget : 100
        detailAutoHideSeconds = detailAutoHidePreferenceStore.load().rawValue

        let cacheDirectory = FileManager.default.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        )[0].appendingPathComponent("AI Meter", isDirectory: true)
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
        snapshots = collected.map(applyingLocalBudget)
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

    func setMonthlyBudget(_ value: Double) {
        monthlyBudget = max(value, 1)
        defaults.set(monthlyBudget, forKey: DefaultsKey.monthlyBudget)
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
              balanceMetric.kind == .balance,
              let currency = currency(for: balanceMetric.unit) else {
            return snapshot
        }

        let spent = budgetTracker.record(
            balance: balanceMetric.current,
            currency: currency,
            isFresh: snapshot.collectionStatus == .fresh
        )
        let budgetMetric = UsageMetric(
            label: "Local monthly budget",
            current: spent,
            limit: monthlyBudget,
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
            statusMessage: snapshot.statusMessage
        )
    }

    private func currency(for unit: UsageUnit) -> DeepSeekCurrency? {
        switch unit {
        case .cny: .cny
        case .usd: .usd
        default: nil
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
                    label: "Local monthly budget",
                    current: 52,
                    limit: 100,
                    unit: .cny,
                    kind: .localBudget
                )
            ),
        ]
    }
}
