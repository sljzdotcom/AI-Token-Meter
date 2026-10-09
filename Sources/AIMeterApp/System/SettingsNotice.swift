import AIMeterCore

/// Stores the result and its parameters; rendering always uses the selected language.
enum SettingsNotice: Equatable {
    case launchAtLoginFailed
    case workspaceSetupFailed
    case installationStarted
    case existingCLI
    case cliDetected
    case installationPending
    case installationFailed
    case codexGuideOpened
    case codexGuideFailed
    case signInPending
    case deepSeekSaved
    case deepSeekEmpty
    case deepSeekRejectedExisting
    case deepSeekRejectedNew
    case deepSeekUnverifiedExisting
    case deepSeekUnverifiedNew
    case deepSeekKeychainExisting
    case deepSeekKeychainNew
    case deepSeekRemoved
    case deepSeekRemovalFailed
    case workspaceApproval
    case signInFailed(UsageProvider)
    case signInStarted(UsageProvider)
    case accountConnected(UsageProvider)
    case recoveryLoginStarted
    case recoveryLoginFailed
    case recoveryLoginCancelled
    case recoveryLoginExpired
    case recoveryCheckingQuota
    case recoveryQuotaFailed
    case recoveryQuotaUpdated

    func text(using localizer: AppLocalizer) -> String {
        switch self {
        case .launchAtLoginFailed: localizer.text("macOS could not update Login Items.")
        case .workspaceSetupFailed: localizer.text("Claude Code workspace setup could not be opened.")
        case .installationStarted: localizer.text("Complete the official installation in Terminal. Status will update automatically.")
        case .existingCLI: localizer.text("An existing CLI was found. Checking its account…")
        case .cliDetected: localizer.text("CLI detected. You can now check the account or sign in.")
        case .installationPending: localizer.text("Installation is not confirmed. Finish in Terminal, then choose Check Status or retry.")
        case .installationFailed: localizer.text("The installation could not be opened or checked. Choose Check Status or retry.")
        case .codexGuideOpened: localizer.text("Opened the official OpenAI Codex CLI installation guide.")
        case .codexGuideFailed: localizer.text("The OpenAI Codex CLI installation guide could not be opened.")
        case .signInPending: localizer.text("Sign-in is still pending. Finish in Terminal, then choose Check Status.")
        case .deepSeekSaved: localizer.text("DeepSeek API Key verified and saved in Keychain.")
        case .deepSeekEmpty: localizer.text("Enter a DeepSeek API Key first.")
        case .deepSeekRejectedExisting: localizer.text("DeepSeek rejected this API Key. The existing Key was kept.")
        case .deepSeekRejectedNew: localizer.text("DeepSeek rejected this API Key. The new API Key was not saved.")
        case .deepSeekUnverifiedExisting: localizer.text("DeepSeek could not verify this Key. The existing Key was kept.")
        case .deepSeekUnverifiedNew: localizer.text("DeepSeek could not verify this Key. The new API Key was not saved.")
        case .deepSeekKeychainExisting: localizer.text("The existing DeepSeek Key was kept because Keychain could not be updated.")
        case .deepSeekKeychainNew: localizer.text("The DeepSeek API Key was not saved because Keychain could not be updated.")
        case .deepSeekRemoved: localizer.text("DeepSeek API Key removed.")
        case .deepSeekRemovalFailed: localizer.text("The API Key could not be removed from Keychain.")
        case .workspaceApproval:
            localizer.text("Approve the private %@ workspace in Terminal, then refresh.", AppBrand.displayName)
        case .signInFailed(let provider):
            localizer.text("%@ sign-in could not be opened.", provider.displayName)
        case .signInStarted(let provider):
            localizer.text("Complete %@ sign-in in Terminal. Status will update automatically.", provider.displayName)
        case .accountConnected(let provider):
            localizer.text("%@ account connected.", provider.displayName)
        case .recoveryLoginStarted:
            localizer.text("Complete the official Antigravity CLI sign-in in Terminal. A browser authorization page may open. This checks quota once; automatic refresh remains paused.")
        case .recoveryLoginFailed:
            localizer.text("Antigravity sign-in did not complete. Refresh remains paused and the previous quota is kept.")
        case .recoveryLoginCancelled:
            localizer.text("Antigravity sign-in was cancelled. Refresh remains paused and the previous quota is kept.")
        case .recoveryLoginExpired:
            localizer.text("Antigravity sign-in expired before completion. Start a new one-time check if you want to try again.")
        case .recoveryCheckingQuota:
            localizer.text("Sign-in completed. Checking Antigravity quota once…")
        case .recoveryQuotaFailed:
            localizer.text("The one-time quota check failed. Refresh remains paused and the previous quota is kept.")
        case .recoveryQuotaUpdated:
            localizer.text("Antigravity quota updated. Automatic refresh remains paused.")
        }
    }
}
