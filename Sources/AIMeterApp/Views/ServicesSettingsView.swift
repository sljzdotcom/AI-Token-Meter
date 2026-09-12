import AIMeterCore
import SwiftUI

struct ServicesSettingsView: View {
    @Bindable var model: AppModel
    @Binding var pendingAPIKey: String

    private var localizer: AppLocalizer { AppLocalizer(language: model.appLanguage) }

    var body: some View {
        Form {
            Section(UsageProvider.claude.displayName) {
                ServiceAccountStatusView(status: status(for: .claude))
                Text(localizer.text("Authentication is handled by the official Claude Code CLI. AI Token Meter never receives your password or verification code."))
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
                HStack {
                    serviceActionButton(.claude)

                    if model.serviceAction(for: .claude).showsSeparateStatusCheck {
                        Button(localizer.text("Check Status")) {
                            Task { await model.checkServiceAccount(.claude) }
                        }
                        .disabled(!model.serviceAction(for: .claude).isEnabled)
                    }

                    Spacer()

                    Button(localizer.text("Authorize Usage Workspace")) {
                        model.openClaudeWorkspaceSetup()
                    }
                    .help(localizer.text("Only needed when Claude Code asks for workspace approval before /usage can run."))
                }
                installationNotice(.claude)
            }

            Section(UsageProvider.codex.displayName) {
                ServiceAccountStatusView(status: status(for: .codex))
                Text(localizer.text("Authentication is handled by the official OpenAI Codex CLI. AI Token Meter only reads the account identity that OpenAI Codex reports locally."))
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
                HStack {
                    serviceActionButton(.codex)

                    if model.serviceAction(for: .codex).showsSeparateStatusCheck {
                        Button(localizer.text("Check Status")) {
                            Task { await model.checkServiceAccount(.codex) }
                        }
                        .disabled(!model.serviceAction(for: .codex).isEnabled)
                    }
                }
                installationNotice(.codex)
            }

            Section("DeepSeek") {
                ServiceAccountStatusView(status: status(for: .deepSeek))

                HStack {
                    Text(localizer.text("Balance baseline"))
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
                Text(localizer.text("The ring shows how much of this reference balance has been depleted. The default is ¥100."))
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)

                SecureField(
                    localizer.text(model.apiKeyConfigured ? "Enter a replacement API Key" : "DeepSeek API Key"),
                    text: $pendingAPIKey
                )
                .disabled(model.isReplacingDeepSeekAPIKey)
                HStack {
                    Label(localizer.text("Stored only in macOS Keychain"), systemImage: "checkmark.shield")
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
                    Spacer()
                    if model.apiKeyConfigured {
                        Button(localizer.text("Remove"), role: .destructive) {
                            model.removeDeepSeekAPIKey()
                        }
                    }
                    Button(localizer.text(model.apiKeyConfigured ? "Replace API Key" : "Save API Key")) {
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
                        Text(localizer.text("Verifying…"))
                            .aiMeterFont(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                Text(localizer.text(model.apiKeyConfigured
                    ? "A replacement is saved only after DeepSeek verifies it. If verification fails, the existing Key remains active."
                    : "The API Key is saved only after DeepSeek verifies it. If verification fails, the new Key is not saved."))
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
            }

            Section("Google Antigravity") {
                let geminiStatus = status(for: .gemini)
                ServiceAccountStatusView(status: geminiStatus)
                Text(localizer.text("Reads official quota through the supported Antigravity CLI. Account identity is not provided by this view."))
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
                GeminiInstallationHelp(state: geminiStatus.connectionState)
                HStack {
                    Link(localizer.text("Antigravity CLI installation guide"), destination: GeminiInstallationGuide.url)
                    Button(localizer.text("Retry")) { Task { await model.checkServiceAccount(.gemini) } }
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

    private func status(for provider: UsageProvider) -> ServiceAccountStatus {
        model.serviceAccounts[provider]
            ?? ServiceAccountStatus(provider: provider, connectionState: .checking, checkedAt: nil)
    }

    private func serviceActionButton(_ provider: UsageProvider) -> some View {
        let action = model.serviceAction(for: provider)
        return Button(action.title) { model.performServiceAction(provider) }
            .tint(action.needsAttention ? .orange : .accentColor)
            .buttonStyle(.bordered)
            .disabled(!action.isEnabled)
    }

    @ViewBuilder
    private func installationNotice(_ provider: UsageProvider) -> some View {
        if status(for: provider).connectionState == .notInstalled {
            Text(localizer.text("Downloads and runs the official installer in Terminal."))
                .font(.caption).foregroundStyle(.secondary)
            Link(localizer.text("Official installation instructions"), destination: URL(string: provider == .claude ? "https://code.claude.com/docs/en/setup" : "https://learn.chatgpt.com/docs/codex/cli")!)
        }
    }
}
