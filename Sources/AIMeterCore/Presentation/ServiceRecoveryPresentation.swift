public enum ServiceRecoveryAction: Equatable, Sendable {
    case openServicesSettings
    case openClaudeWorkspace
    case geminiControls
}

public enum ServiceRecoveryPresentation {
    public static func detailAction(
        provider: UsageProvider,
        status: CollectionStatus,
        statusMessage: String? = nil
    ) -> ServiceRecoveryAction? {
        if provider == .gemini {
            return .geminiControls
        }
        if status == .cached {
            guard let cachedAction = cachedAction(statusMessage) else { return nil }
            if provider == .claude, cachedAction == .openClaudeWorkspace {
                return cachedAction
            }
            return .openServicesSettings
        }
        if [.fresh, .refreshing].contains(status) {
            return nil
        }
        if provider == .claude, status == .setupRequired {
            return .openClaudeWorkspace
        }
        return .openServicesSettings
    }

    private static func cachedAction(_ statusMessage: String?) -> ServiceRecoveryAction? {
        let message = statusMessage?.lowercased() ?? ""
        if message.contains("approve the private usage workspace")
            || message.contains("setup required") {
            return .openClaudeWorkspace
        }
        if message.contains("sign in required")
            || message.contains("authentication required")
            || message.contains("api key requires attention")
            || message.contains("cli not installed") {
            return .openServicesSettings
        }
        return nil
    }
}
