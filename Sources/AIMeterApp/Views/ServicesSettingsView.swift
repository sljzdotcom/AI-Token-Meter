import SwiftUI

struct ServicesSettingsView: View {
    @Bindable var model: AppModel
    @Binding var pendingAPIKey: String

    var body: some View {
        Form {
            Section("Claude") {
                Label("Authentication is managed by the Claude Code CLI.", systemImage: "terminal")
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
            }

            Section("Codex") {
                Label("Authentication is managed by the Codex CLI.", systemImage: "terminal")
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
            }

            Section("DeepSeek") {
                HStack {
                    Text("Balance baseline")
                    Spacer()
                    TextField(
                        "CNY",
                        value: Binding(
                            get: { model.deepSeekBalanceBaseline },
                            set: { model.setDeepSeekBalanceBaseline($0) }
                        ),
                        format: .number.precision(.fractionLength(0...2))
                    )
                    .frame(width: 110)
                    Text("CNY")
                        .foregroundStyle(.secondary)
                }
                Text("The ring shows how much of this reference balance has been depleted. The default is ¥100.")
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)

                SecureField("DeepSeek API Key", text: $pendingAPIKey)
                HStack {
                    Label(
                        model.apiKeyConfigured ? "Stored securely in Keychain" : "No API Key stored",
                        systemImage: model.apiKeyConfigured ? "checkmark.shield" : "key"
                    )
                    .aiMeterFont(.caption)
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

            if model.settingsMessageKind.map(SettingsTab.services.accepts) == true,
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
