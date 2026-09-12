import SwiftUI

enum SoftwareUpdateSettingsCopy {
    static let sectionTitle = "Software Update"
    static let currentVersion = "Current version"
    static let status = "Status"
    static let lastChecked = "Last checked"
    static let checkButton = "Check for Updates"
    static let installButton = "Update Now"
}

struct SoftwareUpdateSettingsView: View {
    let coordinator: SoftwareUpdateCoordinator
    @Environment(\.locale) private var locale

    private var localizer: AppLocalizer { AppLocalizer(language: AppLanguage(rawValue: locale.identifier) ?? .english) }

    var body: some View {
        Section(localizer.text(SoftwareUpdateSettingsCopy.sectionTitle)) {
            LabeledContent(
                localizer.text(SoftwareUpdateSettingsCopy.currentVersion),
                value: coordinator.currentVersionText
            )
            LabeledContent(
                localizer.text(SoftwareUpdateSettingsCopy.status),
                value: coordinator.state.statusText
            )

            if let lastCheckedAt = coordinator.lastCheckedAt {
                LabeledContent(
                    localizer.text(SoftwareUpdateSettingsCopy.lastChecked),
                    value: localizer.date(lastCheckedAt)
                )
            }

            if let release = coordinator.state.availableRelease,
               let summary = release.sanitizedSummary {
                Text(summary)
                    .foregroundStyle(.secondary)
            }

            HStack {
                Button(localizer.text(SoftwareUpdateSettingsCopy.checkButton)) {
                    coordinator.checkForUpdates()
                }
                .disabled(!coordinator.canCheck)

                Button(localizer.text(SoftwareUpdateSettingsCopy.installButton)) {
                    coordinator.installAvailableUpdate()
                }
                .buttonStyle(.borderedProminent)
                .disabled(!coordinator.canInstall)
            }
        }
    }
}
