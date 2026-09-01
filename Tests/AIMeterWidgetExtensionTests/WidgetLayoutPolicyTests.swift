import Testing
import WidgetKit
@testable import AIMeterWidgetExtension

@Suite("Widget layout policy")
struct WidgetLayoutPolicyTests {
    @Test("Supports exactly the approved three Widget families")
    func approvedFamilies() {
        #expect(
            WidgetLayoutPolicy.supportedFamilies
                == [.systemSmall, .systemMedium, .systemLarge]
        )
    }

    @Test("Small exposes only logos and rings")
    func smallIsLogoOnly() {
        #expect(
            WidgetLayoutPolicy.visibleFields(for: .systemSmall)
                == [.logos, .rings]
        )
    }

    @Test("Medium exposes the three quota cards")
    func mediumFields() {
        #expect(
            WidgetLayoutPolicy.visibleFields(for: .systemMedium)
                == [.logos, .providerNames, .values, .detailLabels, .progressBars]
        )
    }

    @Test("Large adds next reset and Codex reset credits")
    func largeFields() {
        #expect(
            WidgetLayoutPolicy.visibleFields(for: .systemLarge)
                == [
                    .logos,
                    .providerNames,
                    .values,
                    .detailLabels,
                    .progressBars,
                    .nextReset,
                    .resetCredits,
                ]
        )
    }
}
