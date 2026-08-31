@MainActor
struct SettingsPresentationCommand {
    let activateApplication: () -> Void
    let openSettings: () -> Void

    func perform() {
        activateApplication()
        openSettings()
    }
}
