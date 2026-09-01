import SwiftUI

struct MonitoringSettingsView: View {
    @Bindable var model: AppModel

    var body: some View {
        Form {
            Section("Monitoring") {
                LabeledContent("Refresh interval", value: "5 minutes")
                Toggle(
                    "Usage alerts at 70% and 90%",
                    isOn: Binding(
                        get: { model.notificationsEnabled },
                        set: { model.setNotificationsEnabled($0) }
                    )
                )
                Toggle(
                    "Open AI Meter at login",
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
