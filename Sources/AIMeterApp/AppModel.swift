import AIMeterCore
import AppKit
import Foundation
import Observation

enum Code0ServiceAction: CaseIterable, Equatable, Sendable {
    case connect, loginCompleted, syncNow, disconnect
}

@MainActor
@Observable
final class AppModel {
    enum GeminiOneTimeRecoveryState: Equatable {
        case idle
        case awaitingLogin
        case stoppingLogin
        case checkingQuota
        case succeeded
        case loginFailed
        case loginCancelled
        case loginExpired
        case quotaFailed
    }

    private enum DefaultsKey {
        static let refreshIntervalSeconds = "refreshIntervalSeconds"
        static let showFloatingStrip = "showFloatingStrip"
        static let notificationsEnabled = "notificationsEnabled"
        static let monthlyBudget = "monthlyBudget"
        static let deepSeekBalanceBaseline = "deepSeekBalanceBaseline"
        static let thresholdEvaluator = "thresholdEvaluator"
    }

    private let coordinator: RefreshCoordinator
    private let secretStore: any SecretStore
    private let launchAtLoginService: LaunchAtLoginService
    private let launchAtLoginStatusOperation: () -> Bool
    private let claudeWorkspaceSetupLauncher: ClaudeWorkspaceSetupLauncher
    private let systemActionPolicy: SystemActionPolicy
    private let defaults: UserDefaults
    private let detailAutoHidePreferenceStore: DetailAutoHidePreferenceStore
    private let displayFontPreferenceStore: DisplayFontPreferenceStore
    private let appLanguagePreferenceStore: AppLanguagePreferenceStore
    private let floatingStripPositionStore: FloatingStripPositionStore
    private let widgetSnapshotPublisher: WidgetSnapshotPublisher?
    private let refreshOperation: (@Sendable () async -> [UsageSnapshot])?
    private let geminiDiagnosticStore: GeminiDiagnosticStore
    private let geminiPauseReasonOperation: @Sendable () async -> GeminiPauseReason?
    private let geminiRecoveryBeginOperation: @Sendable (String) async -> Bool
    private let geminiRecoveryAuthorizeOperation: @Sendable (String) async -> GeminiQuotaRecoveryAuthorization?
    private let geminiQuotaCheckOperation: @Sendable (GeminiQuotaRecoveryAuthorization) async throws -> UsageSnapshot
    private let geminiResumeAuthorizationOperation: @Sendable (GeminiQuotaRecoveryAuthorization) async -> GeminiAutomaticRefreshResumeAuthorization?
    private let geminiAutomaticRefreshResumeOperation: @Sendable (GeminiAutomaticRefreshResumeAuthorization) async throws -> UsageSnapshot
    private let geminiResumeInvalidationOperation: @Sendable () async -> Void
    private let serviceAccountRefreshOperation: @Sendable (UsageProvider?) async -> [ServiceAccountStatus]
    private let authenticationOpenOperation: (UsageProvider) throws -> Void
    private let geminiAuthenticationOpenOperation: (String) throws -> Void
    private let geminiAuthenticationCancelOperation: (String) -> Void
    private let geminiAuthenticationStopOperation: (String) -> Task<Bool, Never>
    private let geminiAuthenticationFailureOperation: (String) -> Bool
    private let geminiAuthenticationCleanupUncertainOperation: (String) -> Bool
    private let geminiAuthenticationExecConfirmedOperation: (String) -> Bool
    private let geminiRecoveryTimeout: Duration
    private let installationOpenOperation: (UsageProvider) throws -> Bool
    private let codexInstallGuideOpenOperation: () -> Bool
    private let deepSeekReplaceOperation: @Sendable (String) async throws -> ServiceAccountStatus
    private let signInPollAttempts: Int
    private let signInPollInterval: Duration
    private let signInSleep: @Sendable (Duration) async throws -> Void
    private let refreshSleep: @Sendable (Duration) async throws -> Void
    private let isDemoMode: Bool
    private var thresholdEvaluator: ThresholdEvaluator
    private var refreshLoop: Task<Void, Never>?
    private var refreshWait: Task<Void, Never>?
    private var refreshRunID: UUID?
    private var refreshWaitID: UUID?
    private var signInTasks: [UsageProvider: Task<Void, Never>] = [:]
    private var signInTokens: [UsageProvider: UUID] = [:]
    private var pendingGeminiLoginToken: String?
    private var activeGeminiRecoveryToken: String?
    private var geminiExecConfirmationRecordedTokens: Set<String> = []
    private var geminiRecoveryStartedAt: Date?
    private var geminiRecoveryTimeoutTask: Task<Void, Never>?
    private var geminiQuotaTask: Task<Void, Never>?
    private var geminiQuotaTaskID: UUID?
    private var geminiQuotaSnapshotGeneration: UInt64 = 0
    private var geminiLifecycleID = UUID()
    private var geminiRecoverySessionID = UUID()
    private var geminiPresentationRevision: UInt64 = 0
    private var geminiResumeAuthorization: GeminiAutomaticRefreshResumeAuthorization?
    private var geminiResumeQuotaAuthorization: GeminiQuotaRecoveryAuthorization?
    private var geminiResumeTask: Task<Void, Never>?
    private var geminiResumeTaskID: UUID?
    private var geminiResumeExpiryTask: Task<Void, Never>?
    private var geminiResumeInvalidationTask: Task<Void, Never>?
    private var providersRequiringAction: Set<UsageProvider> = []
    private(set) var geminiPauseReason: GeminiPauseReason?
    var isAuthenticating: Bool { !signInTokens.isEmpty }
    private(set) var geminiOneTimeRecoveryState: GeminiOneTimeRecoveryState = .idle
    var isGeminiOneTimeRecoveryInProgress: Bool {
        geminiOneTimeRecoveryState == .awaitingLogin
            || geminiOneTimeRecoveryState == .stoppingLogin
            || geminiOneTimeRecoveryState == .checkingQuota
    }
    private var isRefreshingServiceAccounts = false

    let deepSeekWebSession: DeepSeekWebSession
    private var code0WebSession: (any Code0WebSessionProviding)?
    private let code0SessionFactory: (@MainActor () throws -> any Code0WebSessionProviding)?
    private let code0TickSleep: @Sendable (Duration) async throws -> Void
    private let code0MonotonicNow: @MainActor () -> ContinuousClock.Instant
    private var code0BindingID = UUID()
    private var code0Stopped = false
    private var code0Unavailable = false
    private var code0FrozenSnapshot: UsageSnapshot?
    private var code0PresentationRevision: UInt64 = 0
    private var code0TickTask: Task<Void, Never>?
    var currentCode0TickTaskForTesting: Task<Void, Never>? { code0TickTask }
    private var code0TickID: UUID?
    private var code0StopWatch: Task<Void, Never>?
    private var code0LastAction: Code0SessionActionResult?
    private var code0RetryDeadline: ContinuousClock.Instant?
    private var ordinarySnapshots: [UsageSnapshot] = [.geminiUnavailable]

    private(set) var snapshots: [UsageSnapshot] {
        get {
            _ = code0PresentationRevision
            var result = ordinarySnapshots
            if let current = code0CurrentSnapshot { result.append(current) }
            return result
        }
        set { ordinarySnapshots = newValue.filter { UsageProvider.refreshProviders.contains($0.provider) } }
    }
    private(set) var isRefreshing = false
    private(set) var refreshingProviders: Set<UsageProvider> = []
    private(set) var lastUpdatedAt: Date?
    private(set) var apiKeyConfigured = false
    private var ordinaryServiceAccounts: [UsageProvider: ServiceAccountStatus] = [
        .claude: .checking(provider: .claude),
        .codex: .checking(provider: .codex),
        .deepSeek: .checking(provider: .deepSeek),
        .gemini: .geminiUnavailable,
    ]
    private(set) var serviceAccounts: [UsageProvider: ServiceAccountStatus] {
        get {
            var result = ordinaryServiceAccounts
            if code0WebSession != nil || code0Unavailable { result[.code0] = code0AccountStatus }
            return result
        }
        set { ordinaryServiceAccounts = newValue.filter { UsageProvider.refreshProviders.contains($0.key) } }
    }
    private(set) var isReplacingDeepSeekAPIKey = false
    private(set) var launchAtLoginEnabled = false
    private(set) var settingsNotice: SettingsNotice?
    var settingsMessage: String? {
        settingsNotice?.text(using: AppLocalizer(language: appLanguage))
    }
    var geminiRecoveryMessage: String? {
        guard settingsMessageKind == .antigravityRecovery else { return nil }
        return settingsMessage
    }
    private(set) var settingsMessageKind: SettingsMessageKind?
    private(set) var requestedSettingsTab = SettingsTab.appearance
    private(set) var settingsRequestSequence = 0
    private(set) var displayFontChoice: DisplayFontChoice
    private(set) var appLanguage: AppLanguage
    private(set) var floatingStripPosition: FloatingStripPosition
    private(set) var floatingStripDisplays: FloatingStripDisplays
    private(set) var availableStripDisplays: [FloatingStripDisplayChoice] = []
    private(set) var stripPreferences: FloatingStripPreferences

    private(set) var refreshIntervalSeconds: Int

    var showFloatingStrip: Bool
    var notificationsEnabled: Bool
    var deepSeekBalanceBaseline: Double
    var detailAutoHideSeconds: Int

    var floatingVisibilityHandler: ((Bool) -> Void)?
    var floatingPositionHandler: (() -> Void)?
    var floatingAppearanceHandler: (() -> Void)?
    var floatingDisplaysHandler: (() -> Void)?
    var notificationHandler: (([ThresholdEvent]) -> Void)?
    var notificationPermissionHandler: (() -> Void)?

    init(
        defaults: UserDefaults = .standard,
        applicationSupportDirectoryURL: URL? = nil,
        secretStore: any SecretStore = KeychainStore(),
        launchAtLoginService: LaunchAtLoginService = LaunchAtLoginService(),
        launchAtLoginStatusOperation: (() -> Bool)? = nil,
        claudeWorkspaceSetupLauncher: ClaudeWorkspaceSetupLauncher = ClaudeWorkspaceSetupLauncher(),
        authenticationLauncher: CLIAuthenticationLauncher? = nil,
        systemActionPolicy: SystemActionPolicy = .current,
        widgetSnapshotPublisher: WidgetSnapshotPublisher? = WidgetSnapshotPublisher.production(),
        isDemoMode: Bool? = nil,
        geminiDiagnosticStore: GeminiDiagnosticStore? = nil,
        refreshCoordinator: RefreshCoordinator? = nil,
        code0WebSession: (any Code0WebSessionProviding)? = nil,
        code0SessionFactory: (@MainActor () throws -> any Code0WebSessionProviding)? = nil,
        code0TickSleep: @escaping @Sendable (Duration) async throws -> Void = { try await Task.sleep(for: $0) },
        code0MonotonicNow: @escaping @MainActor () -> ContinuousClock.Instant = { ContinuousClock.now },
        refreshOperation: (@Sendable () async -> [UsageSnapshot])? = nil,
        geminiPauseReasonOperation: (@Sendable () async -> GeminiPauseReason?)? = nil,
        geminiRecoveryBeginOperation: (@Sendable (String) async -> Bool)? = nil,
        geminiRecoveryAuthorizeOperation: (@Sendable (String) async -> GeminiQuotaRecoveryAuthorization?)? = nil,
        geminiQuotaCheckOperation: (@Sendable (GeminiQuotaRecoveryAuthorization) async throws -> UsageSnapshot)? = nil,
        geminiResumeAuthorizationOperation: (@Sendable (GeminiQuotaRecoveryAuthorization) async -> GeminiAutomaticRefreshResumeAuthorization?)? = nil,
        geminiAutomaticRefreshResumeOperation: (@Sendable (GeminiAutomaticRefreshResumeAuthorization) async throws -> UsageSnapshot)? = nil,
        geminiResumeInvalidationOperation: (@Sendable () async -> Void)? = nil,
        serviceAccountRefreshOperation: (@Sendable (UsageProvider?) async -> [ServiceAccountStatus])? = nil,
        authenticationOpenOperation: ((UsageProvider) throws -> Void)? = nil,
        geminiAuthenticationOpenOperation: ((String) throws -> Void)? = nil,
        geminiAuthenticationCancelOperation: ((String) -> Void)? = nil,
        geminiAuthenticationStopOperation: ((String) -> Task<Bool, Never>)? = nil,
        geminiAuthenticationFailureOperation: ((String) -> Bool)? = nil,
        geminiAuthenticationCleanupUncertainOperation: ((String) -> Bool)? = nil,
        geminiAuthenticationExecConfirmedOperation: ((String) -> Bool)? = nil,
        installationOpenOperation: ((UsageProvider) throws -> Bool)? = nil,
        codexInstallGuideOpenOperation: (() -> Bool)? = nil,
        deepSeekReplaceOperation: (@Sendable (String) async throws -> ServiceAccountStatus)? = nil,
        signInPollAttempts: Int = 40,
        signInPollInterval: Duration = .seconds(3),
        geminiRecoveryTimeout: Duration = .seconds(310),
        cacheDirectoryURL: URL? = nil,
        refreshSleep: @escaping @Sendable (Duration) async throws -> Void = { try await Task.sleep(for: $0) },
        signInSleep: @escaping @Sendable (Duration) async throws -> Void = { duration in
            try await Task.sleep(for: duration)
        }
    ) {
        self.defaults = defaults
        self.code0WebSession = code0WebSession
        self.code0TickSleep = code0TickSleep
        self.code0MonotonicNow = code0MonotonicNow
        let storedInterval = defaults.object(forKey: DefaultsKey.refreshIntervalSeconds) as? NSNumber
        let storedSeconds = storedInterval?.doubleValue ?? 300
        self.refreshIntervalSeconds = storedSeconds.isFinite && storedSeconds.rounded() == storedSeconds
            && (30...86400).contains(storedSeconds) ? Int(storedSeconds) : 300
        self.secretStore = secretStore
        self.launchAtLoginService = launchAtLoginService
        self.launchAtLoginStatusOperation = launchAtLoginStatusOperation ?? { launchAtLoginService.isEnabled }
        self.claudeWorkspaceSetupLauncher = claudeWorkspaceSetupLauncher
        self.systemActionPolicy = systemActionPolicy
        self.detailAutoHidePreferenceStore = DetailAutoHidePreferenceStore(defaults: defaults)
        self.displayFontPreferenceStore = DisplayFontPreferenceStore(defaults: defaults)
        self.displayFontChoice = self.displayFontPreferenceStore.load()
        self.appLanguagePreferenceStore = AppLanguagePreferenceStore(defaults: defaults)
        self.appLanguage = self.appLanguagePreferenceStore.load()
        self.floatingStripPositionStore = FloatingStripPositionStore(defaults: defaults)
        self.floatingStripPosition = self.floatingStripPositionStore.load()
        self.floatingStripDisplays = FloatingStripDisplaysStore(defaults: defaults).load()
        self.stripPreferences = FloatingStripPreferencesStore(defaults: defaults).load()
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
        let authenticationLauncher = authenticationLauncher
            ?? CLIAuthenticationLauncher(systemActionPolicy: systemActionPolicy)
        let installationLauncher = CLIInstallationLauncher(systemActionPolicy: systemActionPolicy)
        self.installationOpenOperation = installationOpenOperation ?? { provider in
            guard systemActionPolicy.allowsExternalOpen else {
                throw CLIInstallationLaunchError.externalActionsDisabled
            }
            return try installationLauncher.open(provider: provider)
        }
        self.authenticationOpenOperation = authenticationOpenOperation ?? { provider in
            guard systemActionPolicy.allowsExternalOpen else {
                throw CLIAuthenticationLaunchError.externalActionsDisabled
            }
            try authenticationLauncher.open(provider: provider)
        }
        self.geminiAuthenticationOpenOperation = geminiAuthenticationOpenOperation ?? { token in
            guard systemActionPolicy.allowsExternalOpen else {
                throw CLIAuthenticationLaunchError.externalActionsDisabled
            }
            try authenticationLauncher.open(provider: .gemini, completionToken: token)
        }
        self.geminiAuthenticationCancelOperation = geminiAuthenticationCancelOperation ?? { token in
            authenticationLauncher.cancelPendingGeminiLogin(token: token)
        }
        self.geminiAuthenticationStopOperation = geminiAuthenticationStopOperation ?? { token in
            authenticationLauncher.requestGeminiLoginStop(token: token)
        }
        self.geminiAuthenticationFailureOperation = geminiAuthenticationFailureOperation ?? { token in
            authenticationLauncher.hasGeminiTerminalFailure(token: token)
        }
        self.geminiAuthenticationCleanupUncertainOperation = geminiAuthenticationCleanupUncertainOperation ?? { token in
            authenticationLauncher.hasGeminiCleanupUncertainty(token: token)
        }
        self.geminiAuthenticationExecConfirmedOperation = geminiAuthenticationExecConfirmedOperation ?? { token in
            authenticationLauncher.hasGeminiCLIExecConfirmed(token: token)
        }
        let codexInstallationGuideLauncher = CodexInstallationGuideLauncher(systemActionPolicy: systemActionPolicy)
        self.codexInstallGuideOpenOperation = codexInstallGuideOpenOperation ?? {
            guard systemActionPolicy.allowsExternalOpen else { return false }
            return codexInstallationGuideLauncher.open()
        }
        self.deepSeekReplaceOperation = deepSeekReplaceOperation ?? { candidate in
            try await deepSeekCredentialManager.replace(with: candidate)
        }
        self.signInPollAttempts = max(signInPollAttempts, 1)
        self.signInPollInterval = signInPollInterval
        self.geminiRecoveryTimeout = geminiRecoveryTimeout
        self.signInSleep = signInSleep
        self.refreshSleep = refreshSleep
        self.isDemoMode = isDemoMode
            ?? (ProcessInfo.processInfo.environment["AI_METER_DEMO_MODE"] == "1")
        if let code0SessionFactory {
            self.code0SessionFactory = code0SessionFactory
        } else if code0WebSession == nil, !self.isDemoMode, systemActionPolicy.allowsExternalOpen {
            self.code0SessionFactory = { try Code0AppRuntime.makeSession(applicationSupportAnchor: applicationSupportDirectoryURL) }
        } else { self.code0SessionFactory = nil }
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
        let applicationSupportDirectory = applicationSupportDirectoryURL ?? FileManager.default.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        )[0]
        let cacheDirectory = cacheDirectoryURL
            ?? applicationSupportDirectory.appendingPathComponent("AI Meter", isDirectory: true)
        deepSeekWebSession = DeepSeekWebSession(
            historyStore: DeepSeekHistoryStore(directoryURL: cacheDirectory)
        )
        let diagnosticStore = geminiDiagnosticStore
            ?? GeminiDiagnosticStore(fileURL: cacheDirectory.appendingPathComponent("antigravity-diagnostics.json"))
        let coordinator = refreshCoordinator ?? RefreshCoordinator(
            collectors: self.isDemoMode
                ? []
                : [ClaudeCollector(), CodexCollector(), DeepSeekCollector(secretStore: secretStore), GeminiCollector(diagnostics: diagnosticStore)],
            cache: SnapshotCache(directoryURL: cacheDirectory)
        )
        self.coordinator = coordinator
        self.geminiDiagnosticStore = diagnosticStore
        self.refreshOperation = refreshOperation
        self.geminiPauseReasonOperation = geminiPauseReasonOperation ?? { await coordinator.geminiPauseReason() }
        self.geminiRecoveryBeginOperation = geminiRecoveryBeginOperation ?? { await coordinator.beginGeminiQuotaRecovery(token: $0) }
        self.geminiRecoveryAuthorizeOperation = geminiRecoveryAuthorizeOperation ?? {
            await coordinator.authorizeGeminiQuotaRecoveryAfterSignIn(token: $0)
        }
        self.geminiQuotaCheckOperation = geminiQuotaCheckOperation ?? {
            try await coordinator.collectGeminiQuotaOnce(authorization: $0)
        }
        self.geminiResumeAuthorizationOperation = geminiResumeAuthorizationOperation ?? {
            await coordinator.geminiAutomaticRefreshResumeAuthorization(for: $0)
        }
        self.geminiAutomaticRefreshResumeOperation = geminiAutomaticRefreshResumeOperation ?? {
            try await coordinator.resumeGeminiAutomaticRefresh(authorization: $0)
        }
        self.geminiResumeInvalidationOperation = geminiResumeInvalidationOperation ?? {
            await coordinator.invalidateGeminiAutomaticRefreshResumeAuthorization()
        }
        apiKeyConfigured = false
        launchAtLoginEnabled = self.launchAtLoginStatusOperation()
        if self.code0WebSession == nil, let factory = self.code0SessionFactory {
            do { self.code0WebSession = try factory() }
            catch { code0Unavailable = true }
        }
        bindCode0Session()
    }

    private var code0CurrentSnapshot: UsageSnapshot? {
        let value = code0Stopped ? code0FrozenSnapshot : code0WebSession?.snapshot
        return value?.provider == .code0 ? value : nil
    }
    var code0SessionState: Code0SessionState {
        let state = code0WebSession?.state ?? .disconnected
        if code0Stopped, state != .closing, state != .cleanupPending {
            return code0FrozenSnapshot == nil ? .disconnected : .paused
        }
        return state
    }
    var isCode0AutomaticallySyncing: Bool { !code0Stopped && code0WebSession?.isAutomaticallySyncing == true }
    private var code0RetiredSafely: Bool {
        guard let session = code0WebSession else { return true }
        return (session.state == .paused || session.state == .disconnected) && !session.isAutomaticallySyncing
    }
    private var code0CanAcceptActions: Bool {
        if code0Stopped { return code0RetiredSafely && code0SessionFactory != nil }
        return code0WebSession != nil || code0SessionFactory != nil
    }
    func canPerformCode0Action(_ action: Code0ServiceAction) -> Bool {
        guard code0CanAcceptActions else { return false }
        switch action {
        case .connect: return code0SessionState == .disconnected || code0SessionState == .paused
        case .loginCompleted: return code0SessionState == .connecting
        case .syncNow: return (code0SessionState == .paused || code0SessionState == .active) && code0RetrySeconds == nil
        case .disconnect: return [.connecting, .syncing, .active, .paused].contains(code0SessionState)
        }
    }
    var code0RetrySeconds: Int? {
        guard let deadline = code0RetryDeadline else { return nil }
        let now = code0MonotonicNow()
        guard now < deadline else { return nil }
        let duration = now.duration(to: deadline).components
        return Int(duration.seconds) + (duration.attoseconds > 0 ? 1 : 0)
    }
    var code0StatusKey: String {
        if code0Unavailable { return "Code0 connection could not be prepared." }
        if code0SessionState == .cleanupPending { return "Code0 cleanup is not confirmed. New connections are blocked." }
        if !code0Stopped, let failure = code0WebSession?.failure {
            switch failure {
            case .profileBusy: return "Code0 connection is unavailable or already in use. Close any other Code0 session, then try again."
            case .storageFailed: return "Code0 storage could not be confirmed. Automatic sync is paused."
            case .cleanupFailed: return "Code0 cleanup is not confirmed. New connections are blocked."
            case .authenticationChanged: return "Code0 authentication changed. Connect again to validate the current account."
            case .loginTimedOut: return "Code0 sign-in timed out. Connect again."
            case .syncTimedOut: return "Code0 sync timed out. Previous data and success time are unchanged."
            case .policyRejected: return "The Code0 page requested an unsupported action. Sync is paused."
            case .validationRequired, .navigationFailed, .invalidResponse: return "Code0 sync did not complete. Automatic sync is paused."
            }
        }
        switch code0SessionState {
        case .disconnected: return "Code0 is not connected."
        case .connecting: return "Finish signing in in the Code0 window, then confirm below."
        case .syncing: return "Checking Code0 website usage…"
        case .closing: return "Closing the Code0 window…"
        case .active: return "Code0 automatic sync is enabled for this run."
        case .paused: return "Showing cached Code0 data. Choose Sync Code0 to validate this connection."
        case .cleanupPending: return "Code0 cleanup is not confirmed. New connections are blocked."
        }
    }
    func code0ActionMessage(using localizer: AppLocalizer) -> String? {
        if let seconds = code0RetrySeconds {
            return localizer.text("Code0 can sync again in %@ seconds.", localizer.number(Int64(seconds)))
        }
        if code0LastAction == .busy, [.connecting, .syncing, .closing].contains(code0SessionState) {
            return localizer.text("Code0 is busy. Wait for the current operation to finish.")
        }
        if code0LastAction == .denied, code0WebSession?.failure == nil, !code0Unavailable {
            return localizer.text("The Code0 action could not start.")
        }
        return nil
    }
    private var code0AccountStatus: ServiceAccountStatus {
        let state: ServiceAccountConnectionState
        switch code0SessionState {
        case .active: state = isCode0AutomaticallySyncing ? .connected : .lastKnown
        case .connecting, .syncing, .closing: state = .checking
        case .cleanupPending: state = .unavailable
        case .paused: state = code0CurrentSnapshot == nil ? .unavailable : .lastKnown
        case .disconnected: state = code0Unavailable ? .unavailable : .signInRequired
        }
        return ServiceAccountStatus(provider: .code0, connectionState: state, checkedAt: code0CurrentSnapshot?.fetchedAt)
    }
    private func bindCode0Session() {
        let bindingID = UUID()
        code0BindingID = bindingID
        code0WebSession?.onSnapshotChange = { [weak self] _ in
            guard let self, !self.code0Stopped, self.code0BindingID == bindingID else { return }
            // Treat the callback as a signal; the getter reads the bound current,
            // including nil. Neither a stale payload nor ordinary A is a fallback.
            self.code0PresentationRevision &+= 1
            self.publishWidgetSnapshot()
        }
        code0PresentationRevision &+= 1
    }
    @discardableResult
    func performCode0ServiceAction(_ action: Code0ServiceAction) -> Code0SessionActionResult {
        restartCode0IfSafe()
        if !code0Stopped, code0WebSession == nil, let factory = code0SessionFactory {
            do { code0WebSession = try factory(); code0Unavailable = false; bindCode0Session() }
            catch { code0Unavailable = true }
        }
        guard !code0Stopped, let session = code0WebSession else { return .denied }
        // The session validates phases, serialization and deadlines. UI enabled
        // state is informative and never substitutes for those authoritative guards.
        let result: Code0SessionActionResult
        switch action {
        case .connect: result = session.connect()
        case .loginCompleted: result = session.loginCompleted()
        case .syncNow: result = session.syncNow()
        case .disconnect: result = session.disconnect()
        }
        code0LastAction = result
        if case .throttled(let seconds) = result { code0RetryDeadline = code0MonotonicNow().advanced(by: .seconds(seconds)) }
        else { code0RetryDeadline = nil }
        if result == .started { startCode0Ticks() }
        return result
    }
    private func startCode0Ticks() {
        guard code0TickID == nil, !code0Stopped else { return }
        let tickID = UUID(), bindingID = code0BindingID
        let sleep = code0TickSleep
        code0TickID = tickID
        code0TickTask = Task { @MainActor [weak self] in
            while !Task.isCancelled {
                do { try await sleep(.seconds(1)) } catch { break }
                guard let self, !self.code0Stopped, self.code0BindingID == bindingID, self.code0TickID == tickID else { break }
                self.code0WebSession?.processScheduledWork()
                guard [.connecting, .syncing, .closing, .active].contains(self.code0SessionState) else { break }
            }
            if let self, self.code0TickID == tickID { self.code0TickID = nil; self.code0TickTask = nil }
        }
    }
    private func stopCode0Session() {
        guard !code0Stopped else { return }
        code0FrozenSnapshot = code0CurrentSnapshot
        code0Stopped = true
        code0BindingID = UUID()
        code0WebSession?.onSnapshotChange = nil
        code0TickID = nil
        code0TickTask?.cancel(); code0TickTask = nil
        code0StopWatch?.cancel(); code0StopWatch = nil
        code0WebSession?.stop()
        code0PresentationRevision &+= 1
        guard let session = code0WebSession, session.state == .closing else { return }
        let bindingID = code0BindingID, sleep = code0TickSleep
        // Only drain the stopped session's close deadline. Its stopped guard
        // prevents automatic requests; never replace an uncertain owned instance.
        code0StopWatch = Task { @MainActor [weak self] in
            for _ in 0..<11 {
                do { try await sleep(.seconds(1)) } catch { return }
                guard let self, self.code0Stopped, self.code0BindingID == bindingID else { return }
                session.processScheduledWork()
                if session.state != .closing { return }
            }
        }
    }
    private func restartCode0IfSafe() {
        guard code0Stopped, code0RetiredSafely, let factory = code0SessionFactory else { return }
        do {
            let replacement = try factory()
            code0StopWatch?.cancel(); code0StopWatch = nil
            code0WebSession = replacement
            code0FrozenSnapshot = nil
            code0Stopped = false
            code0Unavailable = false
            code0LastAction = nil; code0RetryDeadline = nil
            bindCode0Session()
        } catch { code0Unavailable = true }
    }

    var presentations: [ProviderPresentation] {
        snapshots.map(ProviderPresentation.init(snapshot:))
    }

    var menuBarSnapshots: [UsageSnapshot] {
        stripPreferences.visibleProviders.compactMap { provider in
            snapshots.first(where: { $0.provider == provider })
        }
    }

    var menuBarSummary: MenuBarSummary {
        MenuBarSummary(snapshots: menuBarSnapshots)
    }

    var isRunningDemoMode: Bool { isDemoMode }
    var isGeminiRefreshPaused: Bool { geminiPauseReason != nil || providersRequiringAction.contains(.gemini) }
    var isGeminiSignInPending: Bool { geminiOneTimeRecoveryState == .awaitingLogin }

    var isGeminiAutomaticRefreshResumeInProgress: Bool { geminiResumeTaskID != nil }
    var canResumeGeminiAutomaticRefresh: Bool {
        guard let authorization = geminiResumeAuthorization else { return false }
        return authorization.expiresAt > ContinuousClock.now
            && isGeminiRefreshPaused
            && !isGeminiOneTimeRecoveryInProgress
            && !isGeminiAutomaticRefreshResumeInProgress
    }

    func resumeGeminiAutomaticRefresh() async {
        guard !isDemoMode, isGeminiRefreshPaused,
              !isGeminiOneTimeRecoveryInProgress,
              !isGeminiAutomaticRefreshResumeInProgress else { return }
        settingsMessageKind = .antigravityRecovery
        guard let authorization = geminiResumeAuthorization,
              authorization.expiresAt > ContinuousClock.now else {
            clearGeminiResumeOffer()
            settingsNotice = .recoveryResumeValidationRequired
            return
        }
        // Injected refreshes can bypass Core's inFlight guard. Keep their entire
        // presentation tail busy too, without queuing a later automatic retry.
        guard !isRefreshing, !isRefreshingServiceAccounts else {
            settingsNotice = .recoveryResumeBusy
            return
        }
        let sessionID = geminiRecoverySessionID
        let presentationRevision = geminiPresentationRevision
        let quotaAuthorization = geminiResumeQuotaAuthorization
        let taskID = UUID()
        let operation = geminiAutomaticRefreshResumeOperation
        let resumeAuthorizationOperation = geminiResumeAuthorizationOperation
        geminiResumeTaskID = taskID
        settingsNotice = .recoveryResumeStarted
        let task = Task { [weak self] in
            guard let self else { return }
            defer {
                if geminiResumeTaskID == taskID {
                    geminiResumeTaskID = nil
                    geminiResumeTask = nil
                }
            }
            do {
                try Task.checkCancellation()
                let quota = try await operation(authorization)
                guard geminiRecoverySessionID == sessionID, geminiResumeTaskID == taskID else { return }
                // The operation may have committed before the local TTL task ran.
                // Its captured task identity, rather than a still-held offer, owns
                // this result. A newer refresh still owns its newer presentation.
                clearGeminiResumeOffer()
                guard geminiPresentationRevision == presentationRevision else {
                    finishSupersededGeminiResumeNotice()
                    return
                }
                guard quota.provider == .gemini, quota.geminiQuotaMetrics?.isEmpty == false else {
                    settingsNotice = .recoveryResumeValidationRequired
                    return
                }
                geminiPauseReason = nil
                providersRequiringAction.remove(.gemini)
                if let index = snapshots.firstIndex(where: { $0.provider == .gemini }) {
                    snapshots[index] = applyingLocalBudget(to: quota)
                } else {
                    snapshots.append(applyingLocalBudget(to: quota))
                }
                geminiQuotaSnapshotGeneration &+= 1
                geminiPresentationRevision &+= 1
                updateGeminiAccountStatus()
                publishWidgetSnapshot()
                settingsNotice = .recoveryResumeSucceeded
            } catch {
                guard geminiRecoverySessionID == sessionID, geminiResumeTaskID == taskID else { return }
                if error as? GeminiAutomaticRefreshResumeError == .alreadyRunning {
                    // A harmless account/other-provider publication can advance
                    // the UI revision while Core still retains the original permit.
                    // Revalidate only the exact quota/permit pair; never replace it
                    // with a newer recovery's authority or extend its deadline.
                    if let quotaAuthorization,
                       authorization.expiresAt > ContinuousClock.now,
                       geminiResumeAuthorization == authorization,
                       isGeminiRefreshPaused {
                        let currentAuthorization = await resumeAuthorizationOperation(quotaAuthorization)
                        guard geminiRecoverySessionID == sessionID, geminiResumeTaskID == taskID else { return }
                        if currentAuthorization == authorization,
                           authorization.expiresAt > ContinuousClock.now,
                           geminiResumeAuthorization == authorization,
                           isGeminiRefreshPaused {
                            settingsNotice = .recoveryResumeBusy
                            return
                        }
                    }
                    clearGeminiResumeOffer()
                    settingsNotice = .recoveryResumeValidationRequired
                    return
                }
                guard geminiPresentationRevision == presentationRevision else {
                    clearGeminiResumeOffer()
                    finishSupersededGeminiResumeNotice()
                    return
                }
                clearGeminiResumeOffer()
                settingsNotice = error as? GeminiAutomaticRefreshResumeError == .persistenceFailed
                    ? .recoveryResumePersistenceFailed : .recoveryResumeValidationRequired
            }
        }
        geminiResumeTask = task
        await task.value
    }

    private func finishSupersededGeminiResumeNotice() {
        guard settingsNotice == .recoveryResumeStarted else { return }
        settingsNotice = isGeminiRefreshPaused ? .recoveryResumeValidationRequired : .recoveryResumeSucceeded
    }

    private func discardGeminiResumeOfferIfPauseChanged(to reason: GeminiPauseReason?) {
        guard reason != geminiPauseReason, geminiResumeAuthorization != nil else { return }
        clearGeminiResumeOffer()
        if reason != nil, !isGeminiAutomaticRefreshResumeInProgress {
            settingsMessageKind = .antigravityRecovery
            settingsNotice = .recoveryResumeValidationRequired
        }
    }

    private func clearGeminiResumeOffer() {
        geminiResumeAuthorization = nil
        geminiResumeQuotaAuthorization = nil
        geminiResumeExpiryTask?.cancel()
        geminiResumeExpiryTask = nil
    }

    private func offerGeminiAutomaticRefreshResume(
        _ authorization: GeminiAutomaticRefreshResumeAuthorization,
        quotaAuthorization: GeminiQuotaRecoveryAuthorization,
        sessionID: UUID
    ) {
        clearGeminiResumeOffer()
        guard authorization.expiresAt > ContinuousClock.now else {
            settingsNotice = .recoveryResumeValidationRequired
            return
        }
        geminiResumeAuthorization = authorization
        geminiResumeQuotaAuthorization = quotaAuthorization
        settingsNotice = .recoveryQuotaReadyToResume
        geminiResumeExpiryTask = Task { [weak self] in
            do {
                try await Task.sleep(until: authorization.expiresAt, clock: .continuous)
            } catch { return }
            guard let self, geminiRecoverySessionID == sessionID,
                  geminiResumeAuthorization == authorization else { return }
            clearGeminiResumeOffer()
            if !isGeminiAutomaticRefreshResumeInProgress {
                settingsNotice = .recoveryResumeValidationRequired
            }
        }
    }

    @discardableResult
    func setRefreshInterval(_ text: String) -> Bool {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, trimmed.utf8.allSatisfy({ (48...57).contains($0) }),
              let seconds = Int(trimmed), (30...86400).contains(seconds) else { return false }
        guard seconds != refreshIntervalSeconds else { return true }
        refreshIntervalSeconds = seconds
        defaults.set(seconds, forKey: DefaultsKey.refreshIntervalSeconds)
        // Only the sleeping timer is replaceable; never cancel an active collection.
        if let runID = refreshRunID, refreshWait != nil {
            scheduleAutomaticRefresh(runID: runID)
        }
        return true
    }

    func start() {
        guard refreshRunID == nil else { return }
        restartCode0IfSafe()
        deepSeekWebSession.onHistoryChange = { [weak self] history in
            self?.attachDeepSeekHistory(history)
        }
        if isDemoMode {
            snapshots = Self.demoSnapshots + [.geminiUnavailable]
            setDemoServiceAccounts()
            lastUpdatedAt = Date()
            publishWidgetSnapshot()
            return
        }
        if notificationsEnabled {
            notificationPermissionHandler?()
        }
        let runID = UUID()
        refreshRunID = runID
        beginAutomaticRefresh(runID: runID, checkAccounts: true)
    }

    private func beginAutomaticRefresh(runID: UUID, checkAccounts: Bool = false) {
        refreshLoop = Task { [weak self] in
            guard let self, self.refreshRunID == runID else { return }
            if checkAccounts { await refreshServiceAccounts() }
            guard !Task.isCancelled, refreshRunID == runID else { return }
            await refresh(manual: false)
            guard !Task.isCancelled, refreshRunID == runID else { return }
            refreshLoop = nil
            scheduleAutomaticRefresh(runID: runID)
        }
    }

    private func scheduleAutomaticRefresh(runID: UUID) {
        refreshWait?.cancel()
        let waitID = UUID()
        refreshWaitID = waitID
        let interval = Duration.seconds(refreshIntervalSeconds)
        refreshWait = Task { [weak self] in
            guard let self else { return }
            do { try await refreshSleep(interval) } catch { return }
            // Cancellation may arrive after a clock has completed, or a clock may
            // ignore cancellation. Both identities must still belong to this wait.
            guard !Task.isCancelled, refreshRunID == runID, refreshWaitID == waitID else { return }
            refreshWait = nil
            refreshWaitID = nil
            beginAutomaticRefresh(runID: runID)
        }
    }

    @discardableResult
    func stop() -> Task<Void, Never>? {
        stopCode0Session()
        let stoppedGeminiRecoveryTask = geminiRecoveryTimeoutTask
        geminiLifecycleID = UUID()
        geminiRecoverySessionID = UUID()
        geminiResumeTask?.cancel()
        geminiResumeTask = nil
        geminiResumeTaskID = nil
        clearGeminiResumeOffer()
        let previousInvalidation = geminiResumeInvalidationTask
        let invalidate = geminiResumeInvalidationOperation
        geminiResumeInvalidationTask = Task {
            await previousInvalidation?.value
            await invalidate()
        }
        refreshRunID = nil
        refreshWaitID = nil
        refreshWait?.cancel()
        refreshWait = nil
        refreshLoop?.cancel()
        refreshLoop = nil
        signInTasks.values.forEach { $0.cancel() }
        signInTasks.removeAll()
        signInTokens.removeAll()
        if let pendingGeminiLoginToken {
            _ = geminiAuthenticationStopOperation(pendingGeminiLoginToken)
        }
        geminiRecoveryTimeoutTask?.cancel()
        geminiRecoveryTimeoutTask = nil
        geminiQuotaTask?.cancel()
        geminiQuotaTask = nil
        geminiQuotaTaskID = nil
        pendingGeminiLoginToken = nil
        activeGeminiRecoveryToken = nil
        geminiRecoveryStartedAt = nil
        geminiOneTimeRecoveryState = .idle
        return stoppedGeminiRecoveryTask
    }

    func refresh(manual: Bool = true) async {
        if isDemoMode {
            snapshots = Self.demoSnapshots + [.geminiUnavailable]
            lastUpdatedAt = Date()
            publishWidgetSnapshot()
            return
        }
        guard !isRefreshing else { return }
        isRefreshing = true
        let lifecycleID = geminiLifecycleID
        let geminiQuotaGenerationAtStart = geminiQuotaSnapshotGeneration
        defer { isRefreshing = false; refreshingProviders.removeAll() }

        let collected: [UsageSnapshot]
        if let refreshOperation {
            refreshingProviders = Set(UsageProvider.refreshProviders)
            collected = await refreshOperation()
        } else {
            collected = await coordinator.refresh(manual: manual) { [weak self] provider, active in
                await self?.setProviderRefreshing(provider, active: active)
            }
        }
        guard !Task.isCancelled, geminiLifecycleID == lifecycleID else { return }
        let actionProviders = await coordinator.providersRequiringAction()
        guard !Task.isCancelled, geminiLifecycleID == lifecycleID else { return }
        let pauseReason = await geminiPauseReasonOperation()
        guard !Task.isCancelled, geminiLifecycleID == lifecycleID else { return }
        geminiPresentationRevision &+= 1
        providersRequiringAction = actionProviders
        // These actor reads can straddle a resume commit. Gemini's latest pause
        // reason also authoritatively determines its action-set membership.
        if pauseReason == nil { providersRequiringAction.remove(.gemini) }
        else { providersRequiringAction.insert(.gemini) }
        discardGeminiResumeOfferIfPauseChanged(to: pauseReason)
        geminiPauseReason = pauseReason
        let ordinaryCollected = collected.filter { UsageProvider.refreshProviders.contains($0.provider) }
        updateAPIKeyConfiguration(from: ordinaryCollected)
        var presentedSnapshots = ordinaryCollected.map(applyingLocalBudget).map(applyingDeepSeekHistory)
        if geminiQuotaSnapshotGeneration != geminiQuotaGenerationAtStart,
           let recoveredGemini = snapshots.first(where: { $0.provider == .gemini }) {
            if let index = presentedSnapshots.firstIndex(where: { $0.provider == .gemini }) {
                presentedSnapshots[index] = recoveredGemini
            } else {
                presentedSnapshots.append(recoveredGemini)
            }
        }
        snapshots = presentedSnapshots
        if !snapshots.contains(where: { $0.provider == .gemini }) { snapshots.append(.geminiUnavailable) }
        updateGeminiAccountStatus()
        lastUpdatedAt = Date()
        publishWidgetSnapshot()

        guard notificationsEnabled else { return }
        let events = ordinarySnapshots.flatMap { thresholdEvaluator.evaluate($0) }
        persistThresholdEvaluator()
        if !events.isEmpty {
            notificationHandler?(events)
        }
    }

    private func updateGeminiAccountStatus() {
        guard let snapshot = snapshots.first(where: { $0.provider == .gemini }), snapshot != .geminiUnavailable else {
            serviceAccounts[.gemini] = .geminiUnavailable
            return
        }
        serviceAccounts[.gemini] = .fromGeminiSnapshot(snapshot, pauseReason: geminiPauseReason)
    }

    private func setProviderRefreshing(_ provider: UsageProvider, active: Bool) {
        if active { refreshingProviders.insert(provider) } else { refreshingProviders.remove(provider) }
    }

    func operationState(for provider: UsageProvider) -> ProviderOperationState {
        let status = snapshots.first { $0.provider == provider }?.collectionStatus ?? .unavailable
        let needsAction = serviceAccounts[provider].map { [.signInRequired, .notInstalled].contains($0.connectionState) } ?? false
        let geminiPausedWithoutSignIn = provider == .gemini && geminiPauseReason != nil && geminiPauseReason != .authenticationRequired
        let historyNeedsAction = provider == .deepSeek && deepSeekWebSession.webView.url != nil && deepSeekWebSession.state == .signedOut
        return .resolve(status: status, refreshing: refreshingProviders.contains(provider),
                        needsAction: needsAction || historyNeedsAction || signInTokens[provider] != nil
                            || (provider == .gemini ? geminiPauseReason == .authenticationRequired : providersRequiringAction.contains(provider)),
                        paused: geminiPausedWithoutSignIn)
    }

    func copyGeminiDiagnosticSummary() async {
        let lastQuotaAt = snapshots.first(where: { $0.provider == .gemini })?.geminiQuotaFetchedAt
        let info = Bundle.main.infoDictionary ?? [:]
        let summary = await geminiDiagnosticStore.summary(
            pauseReason: geminiPauseReason,
            lastQuotaAt: lastQuotaAt,
            appVersion: info["CFBundleShortVersionString"] as? String,
            buildNumber: info["CFBundleVersion"] as? String
        )
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(summary, forType: .string)
    }

    func setFloatingStripVisible(_ isVisible: Bool) {
        if isVisible {
            stripPreferences.hiddenUntil = nil
            FloatingStripPreferencesStore(defaults: defaults).save(stripPreferences)
        }
        showFloatingStrip = isVisible
        defaults.set(isVisible, forKey: DefaultsKey.showFloatingStrip)
        floatingVisibilityHandler?(isVisible)
    }

    func setStripPreferences(_ value: FloatingStripPreferences) {
        var value = value
        value.normalize()
        stripPreferences = value
        FloatingStripPreferencesStore(defaults: defaults).save(value)
        floatingAppearanceHandler?()
    }

    func hideStripForOneHour() {
        var value = stripPreferences
        value.hiddenUntil = Date().timeIntervalSince1970 + 3600
        setStripPreferences(value)
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
        if let screenIdentifier {
            floatingStripDisplays.record(identifier: screenIdentifier, edge: edge, normalizedCenterY: normalizedCenterY)
            persistStripDisplays()
        }
    }

    func setFloatingStripDisplayMode(_ mode: FloatingStripDisplayMode) {
        floatingStripDisplays.mode = mode
        if mode == .selected, floatingStripDisplays.selectedIdentifier == nil {
            floatingStripDisplays.selectedIdentifier = availableStripDisplays.first(where: \.isPrimary)?.id
                ?? availableStripDisplays.first?.id
        }
        persistStripDisplays()
        floatingDisplaysHandler?()
    }

    func selectFloatingStripDisplay(_ identifier: String) {
        floatingStripDisplays.mode = .selected
        floatingStripDisplays.selectedIdentifier = identifier
        persistStripDisplays()
        floatingDisplaysHandler?()
    }

    func updateAvailableStripDisplays(_ choices: [FloatingStripDisplayChoice]) {
        availableStripDisplays = choices
    }

    private func persistStripDisplays() {
        FloatingStripDisplaysStore(defaults: defaults).save(floatingStripDisplays)
    }

    func migrateFloatingStripScreenIdentifier(from oldIdentifier: String, to newIdentifier: String) {
        guard oldIdentifier != newIdentifier else { return }
        if floatingStripPosition.screenIdentifier == oldIdentifier {
            floatingStripPosition.screenIdentifier = newIdentifier
            floatingStripPositionStore.save(floatingStripPosition)
        }
        floatingStripDisplays.migrate(from: oldIdentifier, to: newIdentifier)
        persistStripDisplays()
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

    func setAppLanguage(_ language: AppLanguage) {
        guard language != appLanguage else { return }
        appLanguage = language
        appLanguagePreferenceStore.save(language)
    }

    func restoreDefaultDisplayFont() {
        setDisplayFontChoice(.system)
    }

    func setLaunchAtLogin(_ isEnabled: Bool) {
        do {
            try launchAtLoginService.setEnabled(isEnabled)
            launchAtLoginEnabled = launchAtLoginService.isEnabled
            settingsNotice = nil
            settingsMessageKind = nil
        } catch {
            launchAtLoginEnabled = launchAtLoginService.isEnabled
            settingsNotice = .launchAtLoginFailed
            settingsMessageKind = .launchAtLogin
        }
    }

    func openClaudeWorkspaceSetup() {
        guard systemActionPolicy.allowsExternalOpen else {
            settingsNotice = .workspaceSetupFailed
            settingsMessageKind = .claudeWorkspace
            return
        }
        do {
            try claudeWorkspaceSetupLauncher.open()
            settingsNotice = .workspaceApproval
            settingsMessageKind = .claudeWorkspace
        } catch {
            settingsNotice = .workspaceSetupFailed
            settingsMessageKind = .claudeWorkspace
        }
    }

    func requestSettings(_ tab: SettingsTab) {
        requestedSettingsTab = tab
        settingsRequestSequence &+= 1
        NotificationCenter.default.post(name: .aiMeterOpenSettings, object: nil)
    }

    func refreshServiceAccounts() async {
        if isDemoMode {
            setDemoServiceAccounts()
            return
        }
        guard !isRefreshingServiceAccounts, !isAuthenticating else { return }
        isRefreshingServiceAccounts = true
        let lifecycleID = geminiLifecycleID
        defer { isRefreshingServiceAccounts = false }

        UsageProvider.refreshProviders.forEach {
            serviceAccounts[$0] = .checking(provider: $0)
        }
        let statuses = await serviceAccountRefreshOperation(nil)
        guard !Task.isCancelled, geminiLifecycleID == lifecycleID else { return }
        let actionProviders = await coordinator.providersRequiringAction()
        guard !Task.isCancelled, geminiLifecycleID == lifecycleID else { return }
        let pauseReason = await geminiPauseReasonOperation()
        guard !Task.isCancelled, geminiLifecycleID == lifecycleID else { return }
        geminiPresentationRevision &+= 1
        providersRequiringAction = actionProviders
        if pauseReason == nil { providersRequiringAction.remove(.gemini) }
        else { providersRequiringAction.insert(.gemini) }
        discardGeminiResumeOfferIfPauseChanged(to: pauseReason)
        geminiPauseReason = pauseReason
        for status in statuses where status.provider != .gemini && UsageProvider.refreshProviders.contains(status.provider) {
            serviceAccounts[status.provider] = status
        }
        updateGeminiAccountStatus()
        for provider in UsageProvider.refreshProviders where serviceAccounts[provider]?.connectionState == .checking {
            serviceAccounts[provider] = ServiceAccountStatus(
                provider: provider,
                connectionState: .unavailable
            )
        }
        updateAPIKeyConfiguration(from: serviceAccounts[.deepSeek])
    }

    @discardableResult
    func checkServiceAccount(_ provider: UsageProvider) async -> ServiceAccountStatus {
        if provider == .code0 { return code0AccountStatus }
        if provider == .gemini {
            guard !isGeminiRefreshPaused else {
                updateGeminiAccountStatus()
                return serviceAccounts[.gemini] ?? .geminiUnavailable
            }
            await refresh()
            updateGeminiAccountStatus()
            return serviceAccounts[.gemini] ?? .geminiUnavailable
        }
        guard signInTokens[provider] == nil, !isRefreshingServiceAccounts else {
            return serviceAccounts[provider] ?? .checking(provider: provider)
        }
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

    func serviceAction(for provider: UsageProvider) -> CLIServiceAction {
        CLIServiceAction(state: serviceAccounts[provider]?.connectionState ?? .checking, busy: signInTokens[provider] != nil)
    }

    func performServiceAction(_ provider: UsageProvider) {
        guard provider != .code0 else { return }
        guard serviceAction(for: provider).isEnabled else { return }
        switch serviceAccounts[provider]?.connectionState {
        case .notInstalled: beginCLIInstallation(provider)
        case .unavailable: Task { await checkServiceAccount(provider) }
        default: beginSignIn(provider)
        }
    }

    @discardableResult
    func beginCLIInstallation(_ provider: UsageProvider) -> Task<Void, Never>? {
        guard provider == .claude || provider == .codex, signInTokens[provider] == nil else { return nil }
        let token = UUID()
        signInTokens[provider] = token
        settingsMessageKind = authenticationMessageKind(for: provider)
        let task = Task { [weak self] in
            guard let self else { return }
            defer {
                if signInTokens[provider] == token {
                    signInTasks[provider] = nil
                    signInTokens[provider] = nil
                }
            }
            let current = await readServiceAccount(provider)
            guard !Task.isCancelled else { return }
            serviceAccounts[provider] = current
            guard current.connectionState == .notInstalled else { return }
            do {
                let launched = try installationOpenOperation(provider)
                settingsNotice = launched ? .installationStarted : .existingCLI
                for _ in 0..<signInPollAttempts {
                    try await signInSleep(signInPollInterval)
                    guard !Task.isCancelled else { return }
                    let status = await readServiceAccount(provider)
                    guard !Task.isCancelled else { return }
                    serviceAccounts[provider] = status
                    if [.connected, .signInRequired].contains(status.connectionState) {
                        settingsNotice = .cliDetected
                        return
                    }
                }
                settingsNotice = .installationPending
            } catch {
                if !Task.isCancelled { settingsNotice = .installationFailed }
            }
        }
        signInTasks[provider] = task
        return task
    }

    var shouldOfferCodexInstallGuide: Bool {
        serviceAccounts[.codex]?.connectionState == .notInstalled
    }

    func openCodexInstallGuide() {
        if codexInstallGuideOpenOperation() {
            settingsNotice = .codexGuideOpened
        } else {
            settingsNotice = .codexGuideFailed
        }
        settingsMessageKind = .codexAuthentication
    }

    @discardableResult
    func beginSignIn(_ provider: UsageProvider) -> Task<Void, Never>? {
        if provider == .gemini {
            Task { await beginGeminiOneTimeRecovery() }
            return nil
        }
        guard provider == .claude || provider == .codex else { return nil }
        guard signInTokens[provider] == nil else { return nil }
        let originalStatus = serviceAccounts[provider]
        do {
            try authenticationOpenOperation(provider)
        } catch {
            settingsNotice = .signInFailed(provider)
            settingsMessageKind = authenticationMessageKind(for: provider)
            return nil
        }

        signInTasks[provider]?.cancel()
        let signInToken = UUID()
        signInTokens[provider] = signInToken
        settingsNotice = .signInStarted(provider)
        settingsMessageKind = authenticationMessageKind(for: provider)
        let task = Task { [weak self] in
            guard let self else { return }
            defer {
                if signInTokens[provider] == signInToken {
                    signInTasks[provider] = nil
                    signInTokens[provider] = nil
                }
            }
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
                    settingsNotice = .accountConnected(provider)
                    settingsMessageKind = authenticationMessageKind(for: provider)
                    await coordinator.clearAuthenticationBackoff(for: provider)
                    await refresh()
                    guard !Task.isCancelled,
                          signInTokens[provider] == signInToken else { return }
                    signInTasks[provider] = nil
                    signInTokens[provider] = nil
                    return
                }
                if status.connectionState == .signInRequired {
                    sawNonConnectedStatus = true
                }
            }
            guard !Task.isCancelled,
                  signInTokens[provider] == signInToken else { return }
            settingsNotice = .signInPending
            settingsMessageKind = authenticationMessageKind(for: provider)
            signInTasks[provider] = nil
            signInTokens[provider] = nil
        }
        signInTasks[provider] = task
        return task
    }

    func beginGeminiOneTimeRecovery() async {
        guard !isDemoMode, isGeminiRefreshPaused,
              pendingGeminiLoginToken == nil,
              !isGeminiOneTimeRecoveryInProgress,
              !isGeminiAutomaticRefreshResumeInProgress else { return }
        clearGeminiResumeOffer()
        geminiRecoverySessionID = UUID()
        let sessionID = geminiRecoverySessionID
        let token = UUID().uuidString
        pendingGeminiLoginToken = token
        activeGeminiRecoveryToken = token
        geminiRecoveryStartedAt = Date()
        geminiOneTimeRecoveryState = .awaitingLogin
        // A queued stop must finish revoking its old Core generation before a
        // new login can register a token in the same coordinator.
        await geminiResumeInvalidationTask?.value
        guard !Task.isCancelled, geminiRecoverySessionID == sessionID,
              pendingGeminiLoginToken == token else { return }
        guard await geminiRecoveryBeginOperation(token) else {
            if pendingGeminiLoginToken == token {
                pendingGeminiLoginToken = nil
                activeGeminiRecoveryToken = nil
                geminiExecConfirmationRecordedTokens.remove(token)
                geminiRecoveryStartedAt = nil
                geminiOneTimeRecoveryState = .idle
            }
            return
        }
        guard !Task.isCancelled, geminiRecoverySessionID == sessionID,
              pendingGeminiLoginToken == token else { return }
        settingsMessageKind = .antigravityRecovery
        settingsNotice = .recoveryLoginStarted
        do {
            try geminiAuthenticationOpenOperation(token)
            geminiRecoveryTimeoutTask?.cancel()
            geminiRecoveryTimeoutTask = Task { [weak self] in
                guard let self else { return }
                let deadline = ContinuousClock.now.advanced(by: geminiRecoveryTimeout)
                while true {
                    guard !Task.isCancelled, pendingGeminiLoginToken == token else { return }
                    recordGeminiExecConfirmationIfNeeded(token: token)
                    if geminiAuthenticationFailureOperation(token) {
                        // A failure response may precede the helper's cleanup result.
                        // Keep launcher coordination until its stop operation settles.
                        geminiOneTimeRecoveryState = .stoppingLogin
                        _ = await geminiAuthenticationStopOperation(token).value
                        guard !Task.isCancelled,
                              pendingGeminiLoginToken == token,
                              activeGeminiRecoveryToken == token,
                              geminiOneTimeRecoveryState == .stoppingLogin else { return }
                        pendingGeminiLoginToken = nil
                        activeGeminiRecoveryToken = nil
                        geminiExecConfirmationRecordedTokens.remove(token)
                        geminiRecoveryTimeoutTask = nil
                        geminiOneTimeRecoveryState = .loginFailed
                        recordGeminiRecovery(.transportFailure)
                        settingsNotice = .recoveryLoginFailed
                        return
                    }
                    let now = ContinuousClock.now
                    guard now < deadline else { break }
                    let remaining = now.duration(to: deadline)
                    do { try await signInSleep(min(remaining, .milliseconds(100))) } catch { return }
                }
                guard !Task.isCancelled, pendingGeminiLoginToken == token else { return }
                pendingGeminiLoginToken = nil
                activeGeminiRecoveryToken = nil
                geminiOneTimeRecoveryState = .stoppingLogin
                let stopAcknowledged = await geminiAuthenticationStopOperation(token).value
                guard !Task.isCancelled, geminiOneTimeRecoveryState == .stoppingLogin else { return }
                geminiExecConfirmationRecordedTokens.remove(token)
                geminiRecoveryTimeoutTask = nil
                if stopAcknowledged {
                    geminiOneTimeRecoveryState = .loginExpired
                    recordGeminiRecovery(.timedOut)
                    settingsNotice = .recoveryLoginExpired
                } else {
                    geminiOneTimeRecoveryState = .loginFailed
                    recordGeminiRecovery(.transportFailure, stage: .login)
                    settingsNotice = .recoveryLoginFailed
                }
            }
        } catch {
            activeGeminiRecoveryToken = nil
            geminiExecConfirmationRecordedTokens.remove(token)
            pendingGeminiLoginToken = nil
            geminiOneTimeRecoveryState = .loginFailed
            recordGeminiRecovery(.transportFailure)
            settingsNotice = .recoveryLoginFailed
        }
    }

    func completeGeminiInteractiveSignIn(token: String, result: GeminiLoginResult) async {
        guard !isDemoMode, geminiOneTimeRecoveryState == .awaitingLogin,
              pendingGeminiLoginToken == token, UUID(uuidString: token) != nil else { return }
        if case .success = result,
           !geminiAuthenticationExecConfirmedOperation(token) {
            pendingGeminiLoginToken = nil
            geminiRecoveryTimeoutTask?.cancel()
            geminiRecoveryTimeoutTask = nil
            geminiOneTimeRecoveryState = .stoppingLogin
            _ = await geminiAuthenticationStopOperation(token).value
            guard activeGeminiRecoveryToken == token,
                  geminiOneTimeRecoveryState == .stoppingLogin else { return }
            activeGeminiRecoveryToken = nil
            geminiExecConfirmationRecordedTokens.remove(token)
            geminiOneTimeRecoveryState = .loginFailed
            recordGeminiRecovery(.transportFailure, stage: .launch)
            settingsNotice = .recoveryLoginFailed
            return
        }
        recordGeminiExecConfirmationIfNeeded(token: token)
        geminiAuthenticationCancelOperation(token)
        geminiExecConfirmationRecordedTokens.remove(token)
        pendingGeminiLoginToken = nil
        geminiRecoveryTimeoutTask?.cancel()
        geminiRecoveryTimeoutTask = nil
        switch result {
        case .failure:
            activeGeminiRecoveryToken = nil
            geminiOneTimeRecoveryState = .loginFailed
            recordGeminiRecovery(.loginFailed)
            settingsNotice = .recoveryLoginFailed
            return
        case .cancelled:
            activeGeminiRecoveryToken = nil
            geminiOneTimeRecoveryState = .loginCancelled
            recordGeminiRecovery(.loginCancelled)
            settingsNotice = .recoveryLoginCancelled
            return
        case .timedOut:
            activeGeminiRecoveryToken = nil
            geminiOneTimeRecoveryState = .loginExpired
            recordGeminiRecovery(.timedOut)
            settingsNotice = .recoveryLoginExpired
            return
        case .success:
            recordGeminiRecovery(.succeeded)
            break
        }
        guard let authorization = await geminiRecoveryAuthorizeOperation(token) else {
            guard activeGeminiRecoveryToken == token,
                  geminiOneTimeRecoveryState == .awaitingLogin else { return }
            activeGeminiRecoveryToken = nil
            geminiOneTimeRecoveryState = .loginFailed
            recordGeminiRecovery(.loginFailed)
            settingsNotice = .recoveryLoginFailed
            return
        }
        guard activeGeminiRecoveryToken == token,
              geminiOneTimeRecoveryState == .awaitingLogin else { return }
        activeGeminiRecoveryToken = nil
        geminiOneTimeRecoveryState = .checkingQuota
        settingsNotice = .recoveryCheckingQuota
        let operation = geminiQuotaCheckOperation
        let resumeAuthorizationOperation = geminiResumeAuthorizationOperation
        let sessionID = geminiRecoverySessionID
        let taskID = UUID()
        geminiQuotaTaskID = taskID
        geminiQuotaTask = Task { [weak self] in
            guard let self else { return }
            defer {
                if geminiQuotaTaskID == taskID {
                    geminiQuotaTaskID = nil
                    geminiQuotaTask = nil
                }
            }
            do {
                let quota = try await operation(authorization)
                // A successful coordinator return means the cache and persisted
                // suspension already committed. Finish the matching UI update even
                // if cancellation arrived after that commit boundary.
                guard geminiQuotaTaskID == taskID, geminiRecoverySessionID == sessionID else { return }
                guard quota.provider == .gemini,
                      let metrics = quota.geminiQuotaMetrics, !metrics.isEmpty else {
                    geminiOneTimeRecoveryState = .quotaFailed
                    recordGeminiRecovery(.invalidResponse, stage: .usage)
                    settingsNotice = .recoveryQuotaFailed
                    return
                }
                let resumeAuthorization = await resumeAuthorizationOperation(authorization)
                guard geminiQuotaTaskID == taskID, geminiRecoverySessionID == sessionID else { return }
                if let index = snapshots.firstIndex(where: { $0.provider == .gemini }) {
                    snapshots[index] = applyingLocalBudget(to: quota)
                } else {
                    snapshots.append(applyingLocalBudget(to: quota))
                }
                geminiQuotaSnapshotGeneration &+= 1
                updateGeminiAccountStatus()
                lastUpdatedAt = Date()
                publishWidgetSnapshot()
                geminiOneTimeRecoveryState = .succeeded
                settingsNotice = .recoveryQuotaUpdated
                if let resumeAuthorization {
                    offerGeminiAutomaticRefreshResume(resumeAuthorization, quotaAuthorization: authorization, sessionID: sessionID)
                }
            } catch is CancellationError {
                return
            } catch {
                guard !Task.isCancelled, geminiQuotaTaskID == taskID else { return }
                geminiOneTimeRecoveryState = .quotaFailed
                recordGeminiRecovery(.transportFailure, stage: .usage)
                settingsNotice = .recoveryQuotaFailed
            }
        }
        if let task = geminiQuotaTask { await task.value }
    }

    private func recordGeminiRecovery(
        _ category: GeminiDiagnosticCategory,
        stage: GeminiDiagnosticStage = .login
    ) {
        Task {
            let now = Date()
            let elapsed = geminiRecoveryStartedAt.map { Int(now.timeIntervalSince($0) * 1_000) } ?? 0
            if stage == .login { geminiRecoveryStartedAt = nil }
            await geminiDiagnosticStore.record(GeminiDiagnosticRecord(
                recordedAt: now,
                stage: stage,
                category: category,
                durationMilliseconds: elapsed,
                outputTruncated: false
            ))
        }
    }

    private func recordGeminiExecConfirmationIfNeeded(token: String) {
        guard !geminiExecConfirmationRecordedTokens.contains(token),
              geminiAuthenticationExecConfirmedOperation(token) else { return }
        geminiExecConfirmationRecordedTokens.insert(token)
        recordGeminiRecovery(.execConfirmed, stage: .launch)
    }

    func replaceDeepSeekAPIKey(_ apiKey: String) async -> Bool {
        guard !isReplacingDeepSeekAPIKey else { return false }
        let hadExistingKey = apiKeyConfigured
        isReplacingDeepSeekAPIKey = true
        defer { isReplacingDeepSeekAPIKey = false }

        do {
            let status = try await deepSeekReplaceOperation(apiKey)
            serviceAccounts[.deepSeek] = status
            apiKeyConfigured = status.connectionState == .connected
            settingsNotice = .deepSeekSaved
            settingsMessageKind = .deepSeekCredential
            await coordinator.clearAuthenticationBackoff(for: .deepSeek)
            await refresh()
            return true
        } catch let error as DeepSeekCredentialReplacementError {
            switch error {
            case .emptyCandidate:
                settingsNotice = .deepSeekEmpty
            case .invalidKey:
                settingsNotice = hadExistingKey
                    ? .deepSeekRejectedExisting
                    : .deepSeekRejectedNew
            case .verificationUnavailable:
                settingsNotice = hadExistingKey
                    ? .deepSeekUnverifiedExisting
                    : .deepSeekUnverifiedNew
            case .keychainFailure:
                settingsNotice = hadExistingKey
                    ? .deepSeekKeychainExisting
                    : .deepSeekKeychainNew
            }
            settingsMessageKind = .deepSeekCredential
            return false
        } catch {
            settingsNotice = hadExistingKey
                ? .deepSeekUnverifiedExisting
                : .deepSeekUnverifiedNew
            settingsMessageKind = .deepSeekCredential
            return false
        }
    }

    func saveDeepSeekAPIKey(_ apiKey: String) {
        guard !apiKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            settingsNotice = .deepSeekEmpty
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
            settingsNotice = .deepSeekRemoved
            settingsMessageKind = .deepSeekCredential
            Task { await refresh() }
        } catch {
            settingsNotice = .deepSeekRemovalFailed
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
            .gemini: .geminiUnavailable,
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
            antigravitySharedQuotaMetrics: snapshot.antigravitySharedQuotaMetrics,
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
            antigravitySharedQuotaMetrics: antigravitySharedQuotaMetrics,
            deepSeekUsageHistory: history
        )
    }
}
