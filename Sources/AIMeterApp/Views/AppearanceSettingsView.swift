import SwiftUI

struct AppearanceSettingsView: View {
    @Bindable var model: AppModel

    var body: some View {
        Form {
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
