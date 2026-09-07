import AIMeterCore
import SwiftUI

struct FloatingStripDisplaySettings: View {
    @Bindable var model: AppModel

    var body: some View {
        Picker("Show on", selection: Binding(get: { model.floatingStripDisplays.mode },
                                            set: { model.setFloatingStripDisplayMode($0) })) {
            Text("Primary display").tag(FloatingStripDisplayMode.primary)
            Text("Selected display").tag(FloatingStripDisplayMode.selected)
            Text("All displays").tag(FloatingStripDisplayMode.all)
        }
        if model.floatingStripDisplays.mode == .selected {
            Picker("Display", selection: Binding(
                get: { model.floatingStripDisplays.selectedIdentifier ?? "" },
                set: { model.selectFloatingStripDisplay($0) }
            )) {
                ForEach(model.availableStripDisplays) { choice in
                    Text(choice.title).tag(choice.id)
                }
                if let identifier = model.floatingStripDisplays.selectedIdentifier,
                   !model.availableStripDisplays.contains(where: { $0.id == identifier }) {
                    Text("Saved display · Disconnected").tag(identifier)
                }
            }
            Text("A disconnected display temporarily uses the primary display. Its saved position returns when reconnected.")
                .aiMeterFont(.caption)
                .foregroundStyle(.secondary)
        }
        Button("Move to primary display") { model.setFloatingStripDisplayMode(.primary) }
        Text(model.floatingStripDisplays.mode == .all
             ? "Each display has its own position. Drag within that display; all meters share the same data."
             : "Drag across displays to choose a screen. Left and Right control where the meter snaps after release.")
            .aiMeterFont(.caption)
            .foregroundStyle(.secondary)
    }
}
