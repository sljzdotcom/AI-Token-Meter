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
    private let displayFontPreferenceStore: DisplayFontPreferenceStore
    private let floatingStripPositionStore: FloatingStripPositionStore
    private let widgetSnapshotPublisher: WidgetSnapshotPublisher?
    private let refreshOperation: @Sendable () async -> [UsageSnapshot]
    private let serviceAccountRefreshOperation: @Sendable (UsageProvider?) async -> [ServiceAccountStatus]
    private let authenticationOpenOperation: (UsageProvider) throws -> Void
    private let codexInstallGuideOpenOperation: () -> Bool
    private let deepSeekReplaceOperation: @Sendable (String) async throws -> ServiceAccountStatus
    private let signInPollAttempts: Int
    private let signInPollInterval: Duration
    private let signInSleep: @Sendable (Duration) async throws -> Void
    private let isDemoMode: Bool
    private var thresholdEvaluator: ThresholdEvaluator
    private var refreshLoop: Task<Void, Never>?
    private var signInTasks: [UsageProvider: Task<Void, Never>] = [:]
    private var signInTokens: [UsageProvider: UUID] = [:]
    private var isRefreshingServiceAccounts = false

    let deepSeekWebSession: DeepSeekWebSession

    private(set) var snapshots: [UsageSnapshot] = []
    private(set) var isRefreshing = false
    private(set) var lastUpdatedAt: Date?
    private(set) var apiKeyConfigured = false
    private(set) var serviceAccounts: [UsageProvider: ServiceAccountStatus] = [
        .claude: .checking(provider: .claude),
        .codex: .checking(provider: .codex),
        .deepSeek: .checking(provider: .deepSeek),
    ]
    private(set) var isReplacingDeepSeekAPIKey = false
    private(set) var launchAtLoginEnabled = false
    private(set) var settingsMessage: String?
    private(set) var settingsMessageKind: SettingsMessageKind?
    private(set) var displayFontChoice: DisplayFontChoice
    private(set) var floatingStripPosition: FloatingStripPosition

    var showFloatingStrip: Bool
    var notificationsEnabled: Bool
    var deepSeekBalanceBaseline: Double
    var detailAutoHideSeconds: Int

    var floatingVisibilityHandler: ((Bool) -> Void)?
    var floatingPositionHandler: (() -> Void)?
    var notificationHandler: (([ThresholdEvent]) -> Void)?
    var notificationPermissionHandler: (() -> Void)?

    init(
        defaults: UserDefaults = .standard,
        secretStore: any SecretStore = KeychainStore(),
        launchAtLoginService: LaunchAtLoginService = LaunchAtLoginService(),
        claudeWorkspaceSetupLauncher: ClaudeWorkspaceSetupLauncher = ClaudeWorkspaceSetupLauncher(),
        widgetSnapshotPublisher: WidgetSnapshotPublisher? = WidgetSnapshotPublisher.production(),
        isDemoMode: Bool? = nil,
        refreshOperation: (@Sendable () async -> [UsageSnapshot])? = nil,
        serviceAccountRefreshOperation: (@Sendable (UsageProvider?) async -> [ServiceAccountStatus])? = nil,
        authenticationOpenOperation: ((UsageProvider) throws -> Void)? = nil,
        codexInstallGuideOpenOperation: (() -> Bool)? = nil,
        deepSeekReplaceOperation: (@Sendable (String) async throws -> ServiceAccountStatus)? = nil,
        signInPollAttempts: Int = 40,
        signInPollInterval: Duration = .seconds(3),
        signInSleep: @escaping @Sendable (Duration) async throws -> Void = { duration in
            try await Task.sleep(for: duration)
        }
    ) {
        self.defaults = defaults
        self.secretStore = secretStore
        self.launchAtLoginService = launchAtLoginService
        self.claudeWorkspaceSetupLauncher = claudeWorkspaceSetupLauncher
        self.detailAutoHidePreferenceStore = DetailAutoHidePreferenceStore(defaults: defaults)
        self.displayFontPreferenceStore = DisplayFontPreferenceStore(defaults: defaults)
        self.displayFontChoice = self.displayFontPreferenceStore.load()
        self.floatingStripPositionStore = FloatingStripPositionStore(defaults: defaults)
        self.floatingStripPosition = self.floatingStripPositionStore.load()
        self.widgetSnapshotPublisher = widgetSnapshotPublisher
        let deepSeekCredentialManager = DeepSeekCredentialManager(secretStore: secretStore)
        let accountCoordinator = ServiceAccountCoordinator(
            deepSeekReader: deepSeekCredentialManager
        )
        self.serviceAccountRefreshOperation = serviceAccountRefreshOperation ?? { provider in
            if let provider {
                return [await accountCoordinator.read(provider)]
            }
            return await accountCoordinator.readAll()
        }
        let authenticationLauncher = CLIAuthenticationLauncher()
        self.authenticationOpenOperation = authenticationOpenOperation ?? { provider in
            try authenticationLauncher.open(provider: provider)
        }
        let codexInstallationGuideLauncher = CodexInstallationGuideLauncher()
        self.codexInstallGuideOpenOperation = codexInstallGuideOpenOperation ?? {
            codexInstallationGuideLauncher.open()
        }
        self.deepSeekReplaceOperation = deepSeekReplaceOperation ?? { candidate in
            try await deepSeekCredentialManager.replace(with: candidate)
        }
        self.signInPollAttempts = max(signInPollAttempts, 1)
        self.signInPollInterval = signInPollInterval
        self.signInSleep = signInSleep
        self.isDemoMode = isDemoMode
            ?? (ProcessInfo.processInfo.environment["AI_METER_DEMO_MODE"] == "1")
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

        // Legacy storage compatibility: the visible rename must not orphan existing data.
        let cacheDirectory = FileManager.default.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        )[0].appendingPathComponent("AI Meter", isDirectory: true)
        deepSeekWebSession = DeepSeekWebSession(
            historyStore: DeepSeekHistoryStore(directoryURL: cacheDirectory)
        )
        let coordinator = RefreshCoordinator(
            collectors: self.isDemoMode
                ? []
                : [ClaudeCollector(), CodexCollector(), DeepSeekCollector(secretStore: secretStore)],
            cache: SnapshotCache(directoryURL: cacheDirectory)
        )
        self.coordinator = coordinator
        self.refreshOperation = refreshOperation ?? { await coordinator.refresh() }
        apiKeyConfigured = false
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
            setDemoServiceAccounts()
            lastUpdatedAt = Date()
            publishWidgetSnapshot()
            return
        }
        if notificationsEnabled {
            notificationPermissionHandler?()
        }
        refreshLoop = Task { [weak self] in
            guard let self else { return }
            await refreshServiceAccounts()
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

    func stop() {
        refreshLoop?.cancel()
        refreshLoop = nil
        signInTasks.values.forEach { $0.cancel() }
        signInTasks.removeAll()
        signInTokens.removeAll()
    }

    func refresh() async {
        if isDemoMode {
            snapshots = Self.demoSnapshots
            lastUpdatedAt = Date()
            publishWidgetSnapshot()
            return
        }
        guard !isRefreshing else { return }
        isRefreshing = true
        defer { isRefreshing = false }

        let collected = await refreshOperation()
        updateAPIKeyConfiguration(from: collected)
        snapshots = collected.map(applyingLocalBudget).map(applyingDeepSeekHistory)
        lastUpdatedAt = Date()
        publishWidgetSnapshot()

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

    func setFloatingStripEdgePreference(_ preference: FloatingStripEdgePreference) {
        floatingStripPosition.preference = preference
        switch preference {
        case .automatic:
            break
        case .left:
            floatingStripPosition.lastResolvedEdge = .left
        case .right:
            floatingStripPosition.lastResolvedEdge = .right
        }
        floatingStripPositionStore.save(floatingStripPosition)
        floatingPositionHandler?()
    }

    func saveFloatingStripPlacement(
        edge: FloatingStripEdge,
        normalizedCenterY: Double,
        screenIdentifier: String?
    ) {
        floatingStripPosition.lastResolvedEdge = edge
        floatingStripPosition.normalizedCenterY = normalizedCenterY
        floatingStripPosition.screenIdentifier = screenIdentifier
        floatingStripPositionStore.save(floatingStripPosition)
    }

    func migrateFloatingStripScreenIdentifier(from oldIdentifier: String, to newIdentifier: String) {
        guard floatingStripPosition.screenIdentifier == oldIdentifier,
              oldIdentifier != newIdentifier else { return }
        floatingStripPosition.screenIdentifier = newIdentifier
        floatingStripPositionStore.save(floatingStripPosition)
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
        publishWidgetSnapshot()
    }

    func setDetailAutoHideSeconds(_ seconds: Int) {
        let interval = DetailAutoHideInterval(storedSeconds: seconds)
        detailAutoHideSeconds = interval.rawValue
        detailAutoHidePreferenceStore.save(interval)
    }

    func setDisplayFontChoice(_ choice: DisplayFontChoice) {
        displayFontChoice = choice
        displayFontPreferenceStore.save(choice)
    }

    func restoreDefaultDisplayFont() {
        setDisplayFontChoice(.system)
    }

    func setLaunchAtLogin(_ isEnabled: Bool) {
        do {
            try launchAtLoginService.setEnabled(isEnabled)
            launchAtLoginEnabled = launchAtLoginService.isEnabled
            settingsMessage = nil
            settingsMessageKind = nil
        } catch {
            launchAtLoginEnabled = launchAtLoginService.isEnabled
            settingsMessage = "macOS could not update Login Items."
            settingsMessageKind = .launchAtLogin
        }
    }

    func openClaudeWorkspaceSetup() {
        do {
            try claudeWorkspaceSetupLauncher.open()
            settingsMessage = "Approve the private \(AppBrand.displayName) workspace in Terminal, then refresh."
            settingsMessageKind = .claudeWorkspace
        } catch {
            settingsMessage = "Claude Code workspace setup could not be opened."
            settingsMessageKind = .claudeWorkspace
        }
    }

    func refreshServiceAccounts() async {
        if isDemoMode {
            setDemoServiceAccounts()
            return
        }
        guard !isRefreshingServiceAccounts else { return }
        isRefreshingServiceAccounts = true
        defer { isRefreshingServiceAccounts = false }

        UsageProvider.allCases.forEach {
            serviceAccounts[$0] = .checking(provider: $0)
        }
        let statuses = await serviceAccountRefreshOperation(nil)
        for status in statuses {
            serviceAccounts[status.provider] = status
        }
        for provider in UsageProvider.allCases where serviceAccounts[provider]?.connectionState == .checking {
            serviceAccounts[provider] = ServiceAccountStatus(
                provider: provider,
                connectionState: .unavailable
            )
        }
        updateAPIKeyConfiguration(from: serviceAccounts[.deepSeek])
    }

    @discardableResult
    func checkServiceAccount(_ provider: UsageProvider) async -> ServiceAccountStatus {
        serviceAccounts[provider] = .checking(provider: provider)
        let status = await readServiceAccount(provider)
        serviceAccounts[provider] = status
        if provider == .deepSeek {
            updateAPIKeyConfiguration(from: status)
        }
        return status
    }

    func signInButtonTitle(for provider: UsageProvider) -> String {
        serviceAccounts[provider]?.connectionState == .connected
            ? "Sign in again"
            : "Sign in"
    }

    var shouldOfferCodexInstallGuide: Bool {
        serviceAccounts[.codex]?.connectionState == .notInstalled
    }

    func openCodexInstallGuide() {
        if codexInstallGuideOpenOperation() {
            settingsMessage = "Opened the official OpenAI Codex CLI installation guide."
        } else {
            settingsMessage = "The OpenAI Codex CLI installation guide could not be opened."
        }
        settingsMessageKind = .codexAuthentication
    }

    @discardableResult
    func beginSignIn(_ provider: UsageProvider) -> Task<Void, Never>? {
        guard provider == .claude || provider == .codex else { return nil }
        let originalStatus = serviceAccounts[provider]
        do {
            try authenticationOpenOperation(provider)
        } catch {
            settingsMessage = "\(provider.displayName) sign-in could not be opened."
            settingsMessageKind = authenticationMessageKind(for: provider)
            return nil
        }

        signInTasks[provider]?.cancel()
        let signInToken = UUID()
        signInTokens[provider] = signInToken
        settingsMessage = "Complete \(provider.displayName) sign-in in Terminal. Status will update automatically."
        settingsMessageKind = authenticationMessageKind(for: provider)
        let task = Task { [weak self] in
            guard let self else { return }
            var sawNonConnectedStatus = originalStatus?.connectionState != .connected
            for _ in 0..<signInPollAttempts {
                do {
                    try await signInSleep(signInPollInterval)
                } catch {
                    return
                }
                guard !Task.isCancelled,
                      signInTokens[provider] == signInToken else { return }
                let status = await readServiceAccount(provider)
                guard !Task.isCancelled,
                      signInTokens[provider] == signInToken else { return }
                serviceAccounts[provider] = status
                let identityChanged = status.accountLabel != originalStatus?.accountLabel
                    || status.accountDetail != originalStatus?.accountDetail
                if status.connectionState == .connected,
                   sawNonConnectedStatus || identityChanged {
                    settingsMessage = "\(provider.displayName) account connected."
                    settingsMessageKind = authenticationMessageKind(for: provider)
                    await refresh()
                    guard !Task.isCancelled,
                          signInTokens[provider] == signInToken else { return }
                    signInTasks[provider] = nil
                    signInTokens[provider] = nil
                    return
                }
                if status.connectionState != .connected {
                    sawNonConnectedStatus = true
                }
            }
            guard !Task.isCancelled,
                  signInTokens[provider] == signInToken else { return }
            settingsMessage = "Sign-in is still pending. Finish in Terminal, then choose Check Status."
            settingsMessageKind = authenticationMessageKind(for: provider)
            signInTasks[provider] = nil
            signInTokens[provider] = nil
        }
        signInTasks[provider] = task
        return task
    }

    func replaceDeepSeekAPIKey(_ apiKey: String) async -> Bool {
        guard !isReplacingDeepSeekAPIKey else { return false }
        isReplacingDeepSeekAPIKey = true
        defer { isReplacingDeepSeekAPIKey = false }

        do {
            let status = try await deepSeekReplaceOperation(apiKey)
            serviceAccounts[.deepSeek] = status
            apiKeyConfigured = status.connectionState == .connected
            settingsMessage = "DeepSeek API Key verified and saved in Keychain."
            settingsMessageKind = .deepSeekCredential
            await refresh()
            return true
        } catch let error as DeepSeekCredentialReplacementError {
            switch error {
            case .emptyCandidate:
                settingsMessage = "Enter a DeepSeek API Key first."
            case .invalidKey:
                settingsMessage = "DeepSeek rejected this API Key. The existing Key was kept."
            case .verificationUnavailable:
                settingsMessage = "DeepSeek could not verify this Key. The existing Key was kept."
            case .keychainFailure:
                settingsMessage = "The existing DeepSeek Key was kept because Keychain could not be updated."
            }
            settingsMessageKind = .deepSeekCredential
            return false
        } catch {
            settingsMessage = "DeepSeek could not verify this Key. The existing Key was kept."
            settingsMessageKind = .deepSeekCredential
            return false
        }
    }

    func saveDeepSeekAPIKey(_ apiKey: String) {
        guard !apiKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            settingsMessage = "Enter a DeepSeek API Key first."
            settingsMessageKind = .deepSeekCredential
            return
        }
        Task { _ = await replaceDeepSeekAPIKey(apiKey) }
    }

    func removeDeepSeekAPIKey() {
        do {
            try secretStore.delete()
            apiKeyConfigured = false
            serviceAccounts[.deepSeek] = ServiceAccountStatus(
                provider: .deepSeek,
                connectionState: .signInRequired
            )
            settingsMessage = "DeepSeek API Key removed."
            settingsMessageKind = .deepSeekCredential
            Task { await refresh() }
        } catch {
            settingsMessage = "The API Key could not be removed from Keychain."
            settingsMessageKind = .deepSeekCredential
        }
    }

    private func updateAPIKeyConfiguration(from snapshots: [UsageSnapshot]) {
        guard let deepSeek = snapshots.first(where: { $0.provider == .deepSeek }) else { return }
        if deepSeek.collectionStatus == .authenticationRequired {
            apiKeyConfigured = false
        } else if deepSeek.primaryMetric != nil {
            apiKeyConfigured = true
        }
    }

    private func updateAPIKeyConfiguration(from status: ServiceAccountStatus?) {
        guard let status else { return }
        apiKeyConfigured = status.connectionState == .connected
    }

    private func authenticationMessageKind(for provider: UsageProvider) -> SettingsMessageKind {
        provider == .claude ? .claudeAuthentication : .codexAuthentication
    }

    private func readServiceAccount(_ provider: UsageProvider) async -> ServiceAccountStatus {
        await serviceAccountRefreshOperation(provider).first
            ?? ServiceAccountStatus(provider: provider, connectionState: .unavailable)
    }

    private func setDemoServiceAccounts() {
        serviceAccounts = [
            .claude: ServiceAccountStatus(
                provider: .claude,
                connectionState: .connected,
                accountLabel: "Demo Claude Code account",
                accountDetail: "OAuth"
            ),
            .codex: ServiceAccountStatus(
                provider: .codex,
                connectionState: .connected,
                accountLabel: "demo@example.com",
                accountDetail: "ChatGPT · Pro"
            ),
            .deepSeek: ServiceAccountStatus(
                provider: .deepSeek,
                connectionState: .connected,
                accountLabel: "API Key ••••DEMO"
            ),
        ]
        apiKeyConfigured = true
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
            codexLocalActivity: snapshot.codexLocalActivity,
            claudeLocalActivity: snapshot.claudeLocalActivity,
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

    private func publishWidgetSnapshot() {
        widgetSnapshotPublisher?.publish(snapshots)
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
                ),
                claudeLocalActivity: ClaudeLocalActivitySummary(
                    days: (0..<30).map { offset in
                        let active = offset > 8 && !offset.isMultiple(of: 6)
                        return ClaudeDailyActivity(
                            date: Calendar.current.date(
                                byAdding: .day,
                                value: offset - 29,
                                to: Date()
                            ) ?? Date(),
                            inputTokens: active ? Int64(18_000 + offset * 1_100) : 0,
                            outputTokens: active ? Int64(9_000 + offset * 620) : 0,
                            cacheTokens: active ? Int64(31_000 + offset * 1_900) : 0
                        )
                    },
                    sessionCount: 46,
                    activeDayCount: 18,
                    models: [
                        ClaudeModelActivity(modelID: "claude-sonnet-4-6", tokenCount: 1_480_000),
                        ClaudeModelActivity(modelID: "claude-opus-4-1", tokenCount: 620_000),
                    ],
                    updatedAt: Date()
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
                ),
                codexLocalActivity: CodexLocalActivitySummary(
                    tokenCount: 31_400_000_000,
                    currentStreakDays: 54,
                    longestSessionDuration: 6_720
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
            codexLocalActivity: codexLocalActivity,
            claudeLocalActivity: claudeLocalActivity,
            deepSeekUsageHistory: history
        )
    }
}
