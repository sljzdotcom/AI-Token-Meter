import SwiftUI

struct SettingsView: View {
    @Bindable var model: AppModel
    @State private var selectedTab = SettingsTab.appearance
    @State private var pendingAPIKey = ""

    var body: some View {
        TabView(selection: $selectedTab) {
            AppearanceSettingsView(model: model)
                .tabItem {
                    Label(SettingsTab.appearance.title, systemImage: SettingsTab.appearance.systemImage)
                }
                .tag(SettingsTab.appearance)

            MonitoringSettingsView(model: model)
                .tabItem {
                    Label(SettingsTab.monitoring.title, systemImage: SettingsTab.monitoring.systemImage)
                }
                .tag(SettingsTab.monitoring)

            ServicesSettingsView(model: model, pendingAPIKey: $pendingAPIKey)
                .tabItem {
                    Label(SettingsTab.services.title, systemImage: SettingsTab.services.systemImage)
                }
                .tag(SettingsTab.services)

            AboutSettingsView()
                .tabItem {
                    Label(SettingsTab.about.title, systemImage: SettingsTab.about.systemImage)
                }
                .tag(SettingsTab.about)
        }
        .aiMeterFontScope(.settings)
    }
}
