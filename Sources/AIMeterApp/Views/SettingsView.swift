import SwiftUI

struct SettingsView: View {
    @Bindable var model: AppModel
    @State private var pendingAPIKey = ""

    var body: some View {
        Form {
            Section("Appearance") {
                Toggle(
                    "Show right-side floating meter",
                    isOn: Binding(
                        get: { model.showFloatingStrip },
                        set: { model.setFloatingStripVisible($0) }
                    )
                )
                Text("The menu bar meter remains available when the floating meter is hidden.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

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

            Section("DeepSeek") {
                HStack {
                    Text("Monthly local budget")
                    Spacer()
                    TextField(
                        "CNY",
                        value: Binding(
                            get: { model.monthlyBudget },
                            set: { model.setMonthlyBudget($0) }
                        ),
                        format: .number.precision(.fractionLength(0...2))
                    )
                    .frame(width: 110)
                    Text("CNY")
                        .foregroundStyle(.secondary)
                }
                Text("Spend is tracked locally from balance decreases observed by AI Meter; top-ups do not reduce tracked spend.")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                SecureField("DeepSeek API Key", text: $pendingAPIKey)
                HStack {
                    Label(
                        model.apiKeyConfigured ? "Stored securely in Keychain" : "No API Key stored",
                        systemImage: model.apiKeyConfigured ? "checkmark.shield" : "key"
                    )
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    Spacer()
                    if model.apiKeyConfigured {
                        Button("Remove", role: .destructive) {
                            model.removeDeepSeekAPIKey()
                        }
                    }
                    Button("Save") {
                        model.saveDeepSeekAPIKey(pendingAPIKey)
                        pendingAPIKey = ""
                    }
                    .disabled(pendingAPIKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
            }

            if let message = model.settingsMessage {
                Section {
                    Text(message)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}
