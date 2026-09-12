import AIMeterCore
import SwiftUI

struct MonitoringSettingsView: View {
    @Bindable var model: AppModel

    private var localizer: AppLocalizer { AppLocalizer(language: model.appLanguage) }

    var body: some View {
        Form {
            Section(localizer.text("Monitoring")) {
                LabeledContent(localizer.text("Refresh interval")) {
                    RefreshIntervalEditor(seconds: model.refreshIntervalSeconds) {
                        model.setRefreshInterval($0)
                    }
                }
                Toggle(
                    localizer.text("Usage alerts at 70% and 90%"),
                    isOn: Binding(
                        get: { model.notificationsEnabled },
                        set: { model.setNotificationsEnabled($0) }
                    )
                )
                Toggle(
                    localizer.text("Open %@ at login", AppBrand.displayName),
                    isOn: Binding(
                        get: { model.launchAtLoginEnabled },
                        set: { model.setLaunchAtLogin($0) }
                    )
                )
            }

            if model.settingsMessageKind == .launchAtLogin,
               let message = model.settingsMessage {
                Section {
                    Text(message)
                        .aiMeterFont(.caption)
                        .foregroundStyle(.secondary)
                }
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}
