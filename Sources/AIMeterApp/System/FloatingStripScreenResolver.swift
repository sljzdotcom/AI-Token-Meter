struct FloatingStripScreenResolution: Equatable {
    let selectedIdentifier: String?
    let usesFallbackScreen: Bool
    let migratedIdentifier: String?
}

enum FloatingStripScreenResolver {
    static func resolve(
        savedIdentifier: String?,
        screens: [FloatingStripScreenIdentity]
    ) -> FloatingStripScreenResolution? {
        guard !screens.isEmpty else { return nil }

        if let savedIdentifier,
           let exact = screens.first(where: { $0.stableIdentifier == savedIdentifier }) {
            return FloatingStripScreenResolution(
                selectedIdentifier: exact.stableIdentifier,
                usesFallbackScreen: false,
                migratedIdentifier: nil
            )
        }

        let savedLegacyIdentifier = savedIdentifier.flatMap(legacyIdentifier(from:))
        if let savedLegacyIdentifier,
           let legacyMatch = screens.first(where: {
               $0.legacyIdentifier == savedLegacyIdentifier
           }) {
            return FloatingStripScreenResolution(
                selectedIdentifier: legacyMatch.stableIdentifier,
                usesFallbackScreen: false,
                migratedIdentifier: legacyMatch.stableIdentifier
            )
        }

        if savedLegacyIdentifier != nil,
           screens.count == 1,
           let onlyScreen = screens.first {
            return FloatingStripScreenResolution(
                selectedIdentifier: onlyScreen.stableIdentifier,
                usesFallbackScreen: false,
                migratedIdentifier: onlyScreen.stableIdentifier
            )
        }

        let fallback = screens.first(where: \.isMain) ?? screens[0]
        return FloatingStripScreenResolution(
            selectedIdentifier: fallback.stableIdentifier,
            usesFallbackScreen: savedIdentifier != nil,
            migratedIdentifier: nil
        )
    }

    private static func legacyIdentifier(from savedIdentifier: String) -> String? {
        if Int(savedIdentifier) != nil {
            return savedIdentifier
        }
        let prefix = "legacy:"
        guard savedIdentifier.hasPrefix(prefix) else { return nil }
        let value = String(savedIdentifier.dropFirst(prefix.count))
        return Int(value) == nil ? nil : value
    }
}
