import AIMeterCore
import SwiftUI

struct AppearanceSettingsView: View {
    @Bindable var model: AppModel

    var body: some View {
        Form {
            Section("Floating meter") {
                Toggle(
                    "Show floating meter",
                    isOn: Binding(
                        get: { model.showFloatingStrip },
                        set: { model.setFloatingStripVisible($0) }
                    )
                )
                Text("The menu bar meter remains available when the floating meter is hidden.")
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
                Picker(
                    "Screen edge",
                    selection: Binding(
                        get: { model.floatingStripPosition.preference },
                        set: { model.setFloatingStripEdgePreference($0) }
                    )
                ) {
                    ForEach(FloatingStripEdgePreference.allCases, id: \.self) { preference in
                        Text(preference.displayName).tag(preference)
                    }
                }
                .pickerStyle(.segmented)
                Text("Automatic lets you drag the meter to either edge. Left and Right keep that edge fixed while still allowing vertical movement.")
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
                Picker(
                    "Detail auto-hide",
                    selection: Binding(
                        get: { model.detailAutoHideSeconds },
                        set: { model.setDetailAutoHideSeconds($0) }
                    )
                ) {
                    ForEach(DetailAutoHideInterval.allCases) { interval in
                        Text("\(interval.rawValue) seconds").tag(interval.rawValue)
                    }
                }
            }

            Section("Display") {
                Picker(
                    "Display font",
                    selection: Binding(
                        get: { model.displayFontChoice },
                        set: { choice in
                            guard DisplayFontCatalog.live.isAvailable(choice) else { return }
                            model.setDisplayFontChoice(choice)
                        }
                    )
                ) {
                    ForEach(DisplayFontSettingsPresentation.liveOptions()) { option in
                        HStack {
                            Text(option.choice.displayName)
                            if let status = option.statusText {
                                Text(status).foregroundStyle(.secondary)
                            }
                        }
                        .tag(option.choice)
                        .disabled(!option.isEnabled)
                    }
                }
                Button("Restore Default Font") {
                    model.restoreDefaultDisplayFont()
                }
                .disabled(!DisplayFontSettingsPresentation.canRestore(model.displayFontChoice))
                Text("Changes apply immediately. Install missing fonts in macOS to use them.")
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

private extension FloatingStripEdgePreference {
    var displayName: String {
        switch self {
        case .automatic: "Automatic"
        case .left: "Left"
        case .right: "Right"
        }
    }
}
