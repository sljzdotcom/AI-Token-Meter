import AIMeterCore
import SwiftUI

struct FloatingStripSettingsView: View {
    @Bindable var model: AppModel

    var body: some View {
        Form {
            Section("Content and Size") {
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
                Picker("Floating strip size", selection: stripBinding(\.density)) {
                    Text("Comfortable").tag(FloatingStripDensity.comfortable)
                    Text("Compact").tag(FloatingStripDensity.compact)
                    Text("Mini").tag(FloatingStripDensity.mini)
                }
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

            Section("Screen and Position") {
                FloatingStripDisplaySettings(model: model)
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
            }

            Section("Behavior") {
                Toggle("Automatically collapse floating strip", isOn: stripBinding(\.automaticallyCollapses))
                Stepper(
                    "Show delay: \(model.stripPreferences.revealDelayMilliseconds) ms",
                    value: stripBinding(\.revealDelayMilliseconds),
                    in: 0...2_000,
                    step: 50
                )
                .disabled(!model.stripPreferences.automaticallyCollapses)
                Stepper(
                    "Hide delay: \(model.stripPreferences.collapseDelayMilliseconds) ms",
                    value: stripBinding(\.collapseDelayMilliseconds),
                    in: 0...5_000,
                    step: 50
                )
                .disabled(!model.stripPreferences.automaticallyCollapses)
                Text("Turn off automatic collapse to keep the meter expanded. Delay values are retained; open details, menus, dragging and refreshes also keep it expanded.")
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
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
