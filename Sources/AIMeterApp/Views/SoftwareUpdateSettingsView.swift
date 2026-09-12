import SwiftUI

enum SoftwareUpdateSettingsCopy {
    static let sectionTitle = "Software Update"
    static let currentVersion = "Current version"
    static let status = "Status"
    static let lastChecked = "Last checked"
    static let checkButton = "Check for Updates"
    static let installButton = "Update Now"

    static func versionText(version: String?, build: String?, localizer: AppLocalizer) -> String {
        // AppDelegate uses "Unavailable" when its bundle metadata is absent.
        guard let version, let build, !version.isEmpty, !build.isEmpty,
              version != "Unavailable", build != "Unavailable" else {
            return localizer.text("Version unavailable")
        }
        return localizer.text("Version %@ (%@)", version, build)
    }

    static func statusText(_ state: SoftwareUpdateState, localizer: AppLocalizer) -> String {
        switch state {
        case .idle: localizer.text("Not checked yet")
        case .checking: localizer.text("Checking…")
        case .upToDate: localizer.text("You’re up to date")
        case let .available(release): localizer.text("Version %@ is available", release.version)
        case let .installing(release): localizer.text("Preparing version %@…", release.version)
        // The coordinator only emits fixed SoftwareUpdateFailure messages.
        case let .failed(message): localizer.text(message)
        }
    }
}

struct SoftwareUpdateSettingsView: View {
    let coordinator: SoftwareUpdateCoordinator
    @Environment(\.locale) private var locale

    private var localizer: AppLocalizer { AppLocalizer(language: AppLanguage(rawValue: locale.identifier) ?? .english) }

    var body: some View {
        Section(localizer.text(SoftwareUpdateSettingsCopy.sectionTitle)) {
            LabeledContent(
                localizer.text(SoftwareUpdateSettingsCopy.currentVersion),
                value: SoftwareUpdateSettingsCopy.versionText(version: coordinator.currentVersion,
                    build: coordinator.currentBuild, localizer: localizer)
            )
            LabeledContent(
                localizer.text(SoftwareUpdateSettingsCopy.status),
                value: SoftwareUpdateSettingsCopy.statusText(coordinator.state, localizer: localizer)
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
