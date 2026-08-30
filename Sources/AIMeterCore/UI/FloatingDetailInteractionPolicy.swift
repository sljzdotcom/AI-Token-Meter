public struct FloatingDetailInteractionPolicy: Equatable, Sendable {
    public let activatesApplication: Bool
    public let requestsWebFirstResponder: Bool

    public init(provider: UsageProvider) {
        let isInteractive = provider == .deepSeek
        activatesApplication = isInteractive
        requestsWebFirstResponder = isInteractive
    }
}
