import AppKit
import AIMeterCore
import SwiftUI

struct AboutSettingsView: View {
    let updateCoordinator: SoftwareUpdateCoordinator
    let versionInfo: [String: Any]
    @Environment(\.locale) private var locale

    init(updateCoordinator: SoftwareUpdateCoordinator, versionInfo: [String: Any] = Bundle.main.infoDictionary ?? [:]) {
        self.updateCoordinator = updateCoordinator
        self.versionInfo = versionInfo
    }

    private var localizer: AppLocalizer { AppLocalizer(language: AppLanguage(rawValue: locale.identifier) ?? .english) }

    private var versionText: String {
        SoftwareUpdateSettingsCopy.versionText(version: versionInfo["CFBundleShortVersionString"] as? String,
            build: versionInfo["CFBundleVersion"] as? String, localizer: localizer)
    }

    var body: some View {
        Form {
            Section {
                HStack(spacing: 16) {
                    Image(nsImage: NSApplication.shared.applicationIconImage)
                        .resizable()
                        .frame(width: 64, height: 64)
                        .accessibilityHidden(true)
                    VStack(alignment: .leading, spacing: 4) {
                        Text(AppBrand.displayName)
                            .font(.title2.weight(.semibold))
                        Text(localizer.text(AppBrand.subtitle))
                            .foregroundStyle(.secondary)
                        Text(versionText)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        BrandLinksView()
                    }
                }
                .padding(.vertical, 8)
            }

            Section(localizer.text("Privacy")) {
                Text(localizer.text("Claude Code and OpenAI Codex credentials stay with their official CLIs. The DeepSeek API Key is stored in Keychain, and local history contains only normalized aggregate usage."))
                    .foregroundStyle(.secondary)
            }

            SoftwareUpdateSettingsView(coordinator: updateCoordinator)
        }
        .formStyle(.grouped)
        .padding()
    }
}
