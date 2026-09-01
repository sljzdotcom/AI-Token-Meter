import SwiftUI
import WidgetKit

struct WidgetRootView: View {
    @Environment(\.widgetFamily) private var family
    let entry: WidgetEntry

    var body: some View {
        Group {
            switch family {
            case .systemSmall:
                SmallWidgetView(providers: entry.envelope.providers)
            case .systemMedium:
                MediumWidgetView(providers: entry.envelope.providers)
            case .systemLarge:
                LargeWidgetView(envelope: entry.envelope)
            default:
                MediumWidgetView(providers: entry.envelope.providers)
            }
        }
        .widgetURL(URL(string: "aitokenmeter://open"))
        .containerBackground(for: .widget) {
            WidgetDeepSeaBackground()
        }
    }
}
