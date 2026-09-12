import Foundation

enum SettingsMessageKind: Equatable {
    case launchAtLogin
    case claudeWorkspace
    case claudeAuthentication
    case codexAuthentication
    case deepSeekCredential
}

enum SettingsTab: String, CaseIterable, Identifiable {
    case appearance
    case floatingStrip
    case monitoring
    case services
    case about

    var id: Self { self }

    var title: String {
        switch self {
        case .appearance: "Appearance"
        case .floatingStrip: "Floating Strip"
        case .monitoring: "Monitoring"
        case .services: "Services"
        case .about: "About"
        }
    }

    func title(language: AppLanguage) -> String {
        AppLocalizer(language: language).text(title)
    }

    var systemImage: String {
        switch self {
        case .appearance: "paintbrush"
        case .floatingStrip: "sidebar.right"
        case .monitoring: "waveform.path.ecg"
        case .services: "server.rack"
        case .about: "info.circle"
        }
    }

    func accepts(_ kind: SettingsMessageKind) -> Bool {
        switch (self, kind) {
        case (.monitoring, .launchAtLogin),
             (.services, .claudeWorkspace),
             (.services, .claudeAuthentication),
             (.services, .codexAuthentication),
             (.services, .deepSeekCredential):
            true
        default:
            false
        }
    }
}

extension Notification.Name {
    static let aiMeterOpenSettings = Notification.Name("AIMeterOpenSettings")
}
