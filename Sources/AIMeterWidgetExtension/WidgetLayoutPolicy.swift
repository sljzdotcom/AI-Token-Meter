import WidgetKit

enum WidgetVisibleField: Hashable {
    case logos
    case rings
    case providerNames
    case values
    case detailLabels
    case progressBars
    case nextReset
    case resetCredits
}

enum WidgetLayoutPolicy {
    static let supportedFamilies: [WidgetFamily] = [
        .systemSmall,
        .systemMedium,
        .systemLarge,
    ]

    static func visibleFields(for family: WidgetFamily) -> [WidgetVisibleField] {
        switch family {
        case .systemSmall:
            [.logos, .rings]
        case .systemMedium:
            [.logos, .providerNames, .values, .detailLabels, .progressBars]
        case .systemLarge:
            [
                .logos,
                .providerNames,
                .values,
                .detailLabels,
                .progressBars,
                .nextReset,
                .resetCredits,
            ]
        default:
            []
        }
    }
}
