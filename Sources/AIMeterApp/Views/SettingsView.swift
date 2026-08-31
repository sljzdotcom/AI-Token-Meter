import AIMeterCore
import SwiftUI

struct SettingsView: View {
    @Bindable var model: AppModel
    @State private var pendingAPIKey = ""

    var body: some View {
        Form {
            Section("Appearance") {
                Toggle(
                    "Show floating meter",
                    isOn: Binding(
                        get: { model.showFloatingStrip },
                        set: { model.setFloatingStripVisible($0) }
                    )
                )
                Text("The menu bar meter remains available when the floating meter is hidden.")
                    .font(.caption)
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
                    .font(.caption)
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

private extension FloatingStripEdgePreference {
    var displayName: String {
        switch self {
        case .automatic: "Automatic"
        case .left: "Left"
        case .right: "Right"
        }
    }
}
