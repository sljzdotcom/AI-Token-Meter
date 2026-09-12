import SwiftUI

@main
struct AIMeterApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate

    var body: some Scene {
        MenuBarExtra {
            AppLanguageRoot(model: appDelegate.model) {
                MenuBarPanel(model: appDelegate.model)
            }
        } label: {
            AppLanguageRoot(model: appDelegate.model) {
                MenuBarLabel(model: appDelegate.model)
            }
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

/// Keeps independently hosted windows subscribed to the shared language choice.
struct AppLanguageRoot<Content: View>: View {
    @Bindable var model: AppModel
    let content: Content

    init(model: AppModel, @ViewBuilder content: () -> Content) {
        self.model = model
        self.content = content()
    }

    var locale: Locale { model.appLanguage.locale }

    var body: some View {
        content.environment(\.locale, locale)
    }
}
