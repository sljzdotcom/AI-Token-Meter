import SwiftUI

@main
struct AIMeterApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate

    var body: some Scene {
        MenuBarExtra {
            MenuBarPanel(model: appDelegate.model)
        } label: {
            MenuBarLabel(model: appDelegate.model)
        }
        .menuBarExtraStyle(.window)

        Settings {
            SettingsView(
                model: appDelegate.model,
                updateCoordinator: appDelegate.softwareUpdateCoordinator
            )
                .frame(width: 560, height: 540)
        }
    }
}
