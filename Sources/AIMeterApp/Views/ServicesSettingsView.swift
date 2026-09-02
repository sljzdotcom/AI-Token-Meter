import AIMeterCore
import SwiftUI

struct ServicesSettingsView: View {
    @Bindable var model: AppModel
    @Binding var pendingAPIKey: String

    var body: some View {
        Form {
            Section(UsageProvider.claude.displayName) {
                ServiceAccountStatusView(status: status(for: .claude))
                Text("Authentication is handled by the official Claude Code CLI. AI Token Meter never receives your password or verification code.")
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
                HStack {
                    Button(model.signInButtonTitle(for: .claude)) {
                        model.beginSignIn(.claude)
                    }
                    .disabled(status(for: .claude).connectionState == .notInstalled)

                    Button("Check Status") {
                        Task { await model.checkServiceAccount(.claude) }
                    }

                    Spacer()

                    Button("Authorize Usage Workspace") {
                        model.openClaudeWorkspaceSetup()
                    }
                    .help("Only needed when Claude Code asks for workspace approval before /usage can run.")
                }
            }

            Section(UsageProvider.codex.displayName) {
                ServiceAccountStatusView(status: status(for: .codex))
                Text("Authentication is handled by the official OpenAI Codex CLI. AI Token Meter only reads the account identity that OpenAI Codex reports locally.")
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
                HStack {
                    Button(model.signInButtonTitle(for: .codex)) {
                        model.beginSignIn(.codex)
                    }
                    .disabled(status(for: .codex).connectionState == .notInstalled)

                    Button("Check Status") {
                        Task { await model.checkServiceAccount(.codex) }
                    }
                }
            }

            Section("DeepSeek") {
                ServiceAccountStatusView(status: status(for: .deepSeek))

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

                SecureField(
                    model.apiKeyConfigured ? "Enter a replacement API Key" : "DeepSeek API Key",
                    text: $pendingAPIKey
                )
                .disabled(model.isReplacingDeepSeekAPIKey)
                HStack {
                    Label("Stored only in macOS Keychain", systemImage: "checkmark.shield")
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
                    Spacer()
                    if model.apiKeyConfigured {
                        Button("Remove", role: .destructive) {
                            model.removeDeepSeekAPIKey()
                        }
                    }
                    Button(model.apiKeyConfigured ? "Replace API Key" : "Save API Key") {
                        Task {
                            if await model.replaceDeepSeekAPIKey(pendingAPIKey) {
                                pendingAPIKey = ""
                            }
                        }
                    }
                    .disabled(
                        pendingAPIKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                            || model.isReplacingDeepSeekAPIKey
                    )

                    if model.isReplacingDeepSeekAPIKey {
                        ProgressView()
                            .controlSize(.small)
                        Text("Verifying…")
                            .aiMeterFont(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                Text("A replacement is saved only after DeepSeek verifies it. If verification fails, the existing Key remains active.")
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
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

    private func status(for provider: UsageProvider) -> ServiceAccountStatus {
        model.serviceAccounts[provider]
            ?? ServiceAccountStatus(provider: provider, connectionState: .checking, checkedAt: nil)
    }
}
