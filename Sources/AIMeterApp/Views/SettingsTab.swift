enum SettingsMessageKind: Equatable {
    case launchAtLogin
    case claudeWorkspace
    case claudeAuthentication
    case codexAuthentication
    case deepSeekCredential
}

enum SettingsTab: String, CaseIterable, Identifiable {
    case appearance
    case monitoring
    case services
    case about

    var id: Self { self }

    var title: String {
        switch self {
        case .appearance: "Appearance"
        case .monitoring: "Monitoring"
        case .services: "Services"
        case .about: "About"
        }
    }

    var systemImage: String {
        switch self {
        case .appearance: "paintbrush"
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
