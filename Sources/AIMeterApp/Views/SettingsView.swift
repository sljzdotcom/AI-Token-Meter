import SwiftUI

struct SettingsView: View {
    @Bindable var model: AppModel
    let updateCoordinator: SoftwareUpdateCoordinator
    @State private var selectedTab = SettingsTab.appearance
    @State private var pendingAPIKey = ""

    var body: some View {
        TabView(selection: $selectedTab) {
            AppearanceSettingsView(model: model)
                .tabItem {
                    Label(SettingsTab.appearance.title(language: model.appLanguage), systemImage: SettingsTab.appearance.systemImage)
                }
                .tag(SettingsTab.appearance)

            FloatingStripSettingsView(model: model)
                .tabItem {
                    Label(SettingsTab.floatingStrip.title(language: model.appLanguage), systemImage: SettingsTab.floatingStrip.systemImage)
                }
                .tag(SettingsTab.floatingStrip)

            MonitoringSettingsView(model: model)
                .tabItem {
                    Label(SettingsTab.monitoring.title(language: model.appLanguage), systemImage: SettingsTab.monitoring.systemImage)
                }
                .tag(SettingsTab.monitoring)

            ServicesSettingsView(model: model, pendingAPIKey: $pendingAPIKey)
                .tabItem {
                    Label(SettingsTab.services.title(language: model.appLanguage), systemImage: SettingsTab.services.systemImage)
                }
                .tag(SettingsTab.services)

            AboutSettingsView(updateCoordinator: updateCoordinator)
                .tabItem {
                    Label(SettingsTab.about.title(language: model.appLanguage), systemImage: SettingsTab.about.systemImage)
                }
                .tag(SettingsTab.about)
        }
        .environment(\.locale, model.appLanguage.locale)
        .aiMeterFontScope(.settings)
        .onAppear {
            selectedTab = model.requestedSettingsTab
        }
        .onChange(of: model.requestedSettingsTab) { _, tab in
            selectedTab = tab
        }
        .onChange(of: model.settingsRequestSequence) { _, _ in
            selectedTab = model.requestedSettingsTab
        }
        .task {
            await model.refreshServiceAccounts()
        }
    }
}
