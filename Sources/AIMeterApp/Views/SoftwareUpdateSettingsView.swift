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

    var body: some View {
        Section(SoftwareUpdateSettingsCopy.sectionTitle) {
            LabeledContent(
                SoftwareUpdateSettingsCopy.currentVersion,
                value: coordinator.currentVersionText
            )
            LabeledContent(
                SoftwareUpdateSettingsCopy.status,
                value: coordinator.state.statusText
            )

            if let lastCheckedAt = coordinator.lastCheckedAt {
                LabeledContent(
                    SoftwareUpdateSettingsCopy.lastChecked,
                    value: lastCheckedAt.formatted(date: .abbreviated, time: .shortened)
                )
            }

            if let release = coordinator.state.availableRelease,
               let summary = release.sanitizedSummary {
                Text(summary)
                    .foregroundStyle(.secondary)
            }

            HStack {
                Button(SoftwareUpdateSettingsCopy.checkButton) {
                    coordinator.checkForUpdates()
                }
                .disabled(!coordinator.canCheck)

                Button(SoftwareUpdateSettingsCopy.installButton) {
                    coordinator.installAvailableUpdate()
                }
                .buttonStyle(.borderedProminent)
                .disabled(!coordinator.canInstall)
            }
        }
    }
}
