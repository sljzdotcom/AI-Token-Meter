import SwiftUI

struct AppearanceSettingsView: View {
    @Bindable var model: AppModel

    private var localizer: AppLocalizer { AppLocalizer(language: model.appLanguage) }

    var body: some View {
        Form {
            Section(localizer.text("Display")) {
                Picker(
                    localizer.text("Language"),
                    selection: Binding(get: { model.appLanguage }, set: { model.setAppLanguage($0) })
                ) {
                    ForEach(AppLanguage.allCases) { language in
                        Text(language.displayName).tag(language)
                    }
                }
                Picker(
                    localizer.text("Display font"),
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
                Button(localizer.text("Restore Default Font")) {
                    model.restoreDefaultDisplayFont()
                }
                .disabled(!DisplayFontSettingsPresentation.canRestore(model.displayFontChoice))
                Text(localizer.text("Changes apply immediately. Install missing fonts in macOS to use them."))
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}
