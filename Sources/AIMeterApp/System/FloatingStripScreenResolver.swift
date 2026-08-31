import AIMeterCore

struct FloatingStripScreenResolution: Equatable {
    let identifier: String?
    let usesDefaultPlacement: Bool
    let defaultEdge: FloatingStripEdge
    let defaultNormalizedCenterY: Double
}

enum FloatingStripScreenResolver {
    static func resolve(
        savedIdentifier: String,
        availableIdentifiers: [String],
        mainIdentifier: String?
    ) -> FloatingStripScreenResolution {
        if availableIdentifiers.contains(savedIdentifier) {
            return FloatingStripScreenResolution(
                identifier: savedIdentifier,
                usesDefaultPlacement: false,
                defaultEdge: .right,
                defaultNormalizedCenterY: 0.5
            )
        }
        return FloatingStripScreenResolution(
            identifier: mainIdentifier ?? availableIdentifiers.first,
            usesDefaultPlacement: true,
            defaultEdge: .right,
            defaultNormalizedCenterY: 0.5
        )
    }
}
