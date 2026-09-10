import AIMeterCore
import SwiftUI

struct AppearanceSettingsView: View {
    @Bindable var model: AppModel

    var body: some View {
        Form {
            Section("Floating meter") {
                Picker("Floating strip size", selection: stripBinding(\.density)) {
                    Text("Compact").tag(FloatingStripDensity.compact)
                    Text("Comfortable").tag(FloatingStripDensity.comfortable)
                }
                Stepper(
                    "Show delay: \(model.stripPreferences.revealDelayMilliseconds) ms",
                    value: stripBinding(\.revealDelayMilliseconds),
                    in: 0...2_000,
                    step: 50
                )
                Stepper(
                    "Hide delay: \(model.stripPreferences.collapseDelayMilliseconds) ms",
                    value: stripBinding(\.collapseDelayMilliseconds),
                    in: 0...5_000,
                    step: 50
                )
                Text("The show and hide delays apply immediately. Open details, menus, dragging and refreshes keep the meter expanded.")
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
                Toggle(
                    "Show floating meter",
                    isOn: Binding(
                        get: { model.showFloatingStrip },
                        set: { model.setFloatingStripVisible($0) }
                    )
                )
                Text("The menu bar meter remains available when the floating meter is hidden.")
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
                Picker(
                    "Screen edge",
                    selection: Binding(
                        get: { model.floatingStripPosition.preference },
                        set: { model.setFloatingStripEdgePreference($0) }
                    )
                ) {
                    ForEach(FloatingStripEdgePreference.allCases, id: \.self) { preference in
                        Text(preference.displayName).tag(preference)
                    }
                }
                .pickerStyle(.segmented)
                FloatingStripDisplaySettings(model: model)
                Picker(
                    "Detail auto-hide",
                    selection: Binding(
                        get: { model.detailAutoHideSeconds },
                        set: { model.setDetailAutoHideSeconds($0) }
                    )
                ) {
                    ForEach(DetailAutoHideInterval.allCases) { interval in
                        Text("\(interval.rawValue) seconds").tag(interval.rawValue)
                    }
                }
            }

            Section("Display") {
                Picker(
                    "Display font",
                    selection: Binding(
                        get: { model.displayFontChoice },
                        set: { choice in
                            guard DisplayFontCatalog.live.isAvailable(choice) else { return }
                            model.setDisplayFontChoice(choice)
                        }
                    )
                ) {
                    ForEach(DisplayFontSettingsPresentation.liveOptions()) { option in
                        HStack {
                            Text(option.choice.displayName)
                            if let status = option.statusText {
                                Text(status).foregroundStyle(.secondary)
                            }
                        }
                        .tag(option.choice)
                        .disabled(!option.isEnabled)
                    }
                }
                Button("Restore Default Font") {
                    model.restoreDefaultDisplayFont()
                }
                .disabled(!DisplayFontSettingsPresentation.canRestore(model.displayFontChoice))
                Text("Changes apply immediately. Install missing fonts in macOS to use them.")
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
            }
            Section("Floating strip services") {
                ForEach(model.stripPreferences.orderedProviders, id: \.self) { provider in
                    HStack {
                        Toggle(provider.displayName, isOn: Binding(
                            get: { model.stripPreferences.visibleProviders.contains(provider) },
                            set: { model.setStripPreferences(model.stripPreferences.settingVisible(provider, visible: $0)) }
                        ))
                        .disabled(model.stripPreferences.visibleProviders == [provider])
                        Button { move(provider, by: -1) } label: { Image(systemName: "arrow.up") }
                            .disabled(model.stripPreferences.orderedProviders.first == provider)
                            .accessibilityLabel("Move \(provider.displayName) up")
                        Button { move(provider, by: 1) } label: { Image(systemName: "arrow.down") }
                            .disabled(model.stripPreferences.orderedProviders.last == provider)
                            .accessibilityLabel("Move \(provider.displayName) down")
                    }
                    .draggable(provider.rawValue)
                    .dropDestination(for: String.self) { items, _ in
                        guard let raw = items.first, let source = UsageProvider(rawValue: raw), source != provider else { return false }
                        var value = model.stripPreferences
                        guard let from = value.orderedProviders.firstIndex(of: source),
                              let to = value.orderedProviders.firstIndex(of: provider) else { return false }
                        value.orderedProviders.remove(at: from)
                        value.orderedProviders.insert(source, at: to)
                        model.setStripPreferences(value)
                        return true
                    }
                }
                Text("Keep at least one service visible. Hidden services continue monitoring.")
                    .font(.caption).foregroundStyle(.secondary)
                Button("Restore default order") {
                    var value = model.stripPreferences
                    value.orderedProviders = UsageProvider.allCases
                    value.hiddenProviders = []
                    model.setStripPreferences(value)
                }
            }
        }
        .formStyle(.grouped)
        .padding()
    }

    private func stripBinding<T>(_ keyPath: WritableKeyPath<FloatingStripPreferences, T>) -> Binding<T> {
        Binding(get: { model.stripPreferences[keyPath: keyPath] }, set: {
            var value = model.stripPreferences
            value[keyPath: keyPath] = $0
            model.setStripPreferences(value)
        })
    }

    private func move(_ provider: UsageProvider, by offset: Int) {
        var value = model.stripPreferences
        guard let index = value.orderedProviders.firstIndex(of: provider),
              value.orderedProviders.indices.contains(index + offset) else { return }
        value.orderedProviders.swapAt(index, index + offset)
        model.setStripPreferences(value)
    }
}

private extension FloatingStripEdgePreference {
    var displayName: String {
        switch self {
        case .automatic: "Automatic"
        case .left: "Left"
        case .right: "Right"
        }
    }
}
