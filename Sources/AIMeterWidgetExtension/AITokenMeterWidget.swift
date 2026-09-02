import AIMeterCore
import SwiftUI
import WidgetKit

@main
struct AITokenMeterWidgetBundle: WidgetBundle {
    var body: some Widget {
        AITokenMeterUsageWidget()
    }
}

struct AITokenMeterUsageWidget: Widget {
    let kind = AITokenMeterWidgetContract.kind

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: WidgetTimelineSource()) { entry in
            WidgetRootView(entry: entry)
        }
        .configurationDisplayName("AI Token Meter")
        .description("Claude Code, OpenAI Codex, and DeepSeek usage at a glance.")
        .supportedFamilies(WidgetLayoutPolicy.supportedFamilies)
        .contentMarginsDisabled()
    }
}
