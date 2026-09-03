import AIMeterCore

struct FloatingStripScreenResolution: Equatable {
    let selectedIdentifier: String?
    let usesFallbackScreen: Bool
    let migratedIdentifier: String?

    var identifier: String? { selectedIdentifier }
    var usesDefaultPlacement: Bool { usesFallbackScreen }
    var defaultEdge: FloatingStripEdge { .right }
    var defaultNormalizedCenterY: Double { 0.5 }
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

        if let savedIdentifier,
           let legacyMatch = screens.first(where: { $0.legacyIdentifier == savedIdentifier }) {
            return FloatingStripScreenResolution(
                selectedIdentifier: legacyMatch.stableIdentifier,
                usesFallbackScreen: false,
                migratedIdentifier: legacyMatch.stableIdentifier
            )
        }

        if let savedIdentifier,
           Int(savedIdentifier) != nil,
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

    static func resolve(
        savedIdentifier: String,
        availableIdentifiers: [String],
        mainIdentifier: String?
    ) -> FloatingStripScreenResolution {
        let isAvailable = availableIdentifiers.contains(savedIdentifier)
        return FloatingStripScreenResolution(
            selectedIdentifier: isAvailable
                ? savedIdentifier
                : (mainIdentifier ?? availableIdentifiers.first),
            usesFallbackScreen: !isAvailable,
            migratedIdentifier: nil
        )
    }
}
