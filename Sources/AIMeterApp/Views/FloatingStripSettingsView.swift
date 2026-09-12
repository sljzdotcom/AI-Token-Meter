import AIMeterCore
import SwiftUI

struct FloatingStripSettingsView: View {
    @Bindable var model: AppModel

    private var localizer: AppLocalizer { AppLocalizer(language: model.appLanguage) }

    var body: some View {
        Form {
            Section(localizer.text("Content and Size")) {
                Toggle(
                    localizer.text("Show floating meter"),
                    isOn: Binding(
                        get: { model.showFloatingStrip },
                        set: { model.setFloatingStripVisible($0) }
                    )
                )
                Text(localizer.text("The menu bar meter remains available when the floating meter is hidden."))
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
                Picker(localizer.text("Floating strip size"), selection: stripBinding(\.density)) {
                    Text(localizer.text("Comfortable")).tag(FloatingStripDensity.comfortable)
                    Text(localizer.text("Compact")).tag(FloatingStripDensity.compact)
                    Text(localizer.text("Mini")).tag(FloatingStripDensity.mini)
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
                            .accessibilityLabel(localizer.text("Move %@ up", provider.displayName))
                        Button { move(provider, by: 1) } label: { Image(systemName: "arrow.down") }
                            .disabled(model.stripPreferences.orderedProviders.last == provider)
                            .accessibilityLabel(localizer.text("Move %@ down", provider.displayName))
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
                Text(localizer.text("Keep at least one service visible. Hidden services continue monitoring."))
                    .font(.caption).foregroundStyle(.secondary)
                Button(localizer.text("Restore default order")) {
                    var value = model.stripPreferences
                    value.orderedProviders = UsageProvider.allCases
                    value.hiddenProviders = []
                    model.setStripPreferences(value)
                }
            }

            Section(localizer.text("Screen and Position")) {
                FloatingStripDisplaySettings(model: model)
                Picker(
                    localizer.text("Screen edge"),
                    selection: Binding(
                        get: { model.floatingStripPosition.preference },
                        set: { model.setFloatingStripEdgePreference($0) }
                    )
                ) {
                    ForEach(FloatingStripEdgePreference.allCases, id: \.self) { preference in
                        Text(localizer.text(preference.displayName)).tag(preference)
                    }
                }
                .pickerStyle(.segmented)
            }

            Section(localizer.text("Behavior")) {
                Toggle(localizer.text("Automatically collapse floating strip"), isOn: stripBinding(\.automaticallyCollapses))
                Stepper(
                    localizer.text("Show delay: %lld ms", Int64(model.stripPreferences.revealDelayMilliseconds)),
                    value: stripBinding(\.revealDelayMilliseconds),
                    in: 0...2_000,
                    step: 50
                )
                .disabled(!model.stripPreferences.automaticallyCollapses)
                Stepper(
                    localizer.text("Hide delay: %lld ms", Int64(model.stripPreferences.collapseDelayMilliseconds)),
                    value: stripBinding(\.collapseDelayMilliseconds),
                    in: 0...5_000,
                    step: 50
                )
                .disabled(!model.stripPreferences.automaticallyCollapses)
                Text(localizer.text("Turn off automatic collapse to keep the meter expanded. Delay values are retained; open details, menus, dragging and refreshes also keep it expanded."))
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
                Picker(
                    localizer.text("Detail auto-hide"),
                    selection: Binding(
                        get: { model.detailAutoHideSeconds },
                        set: { model.setDetailAutoHideSeconds($0) }
                    )
                ) {
                    ForEach(DetailAutoHideInterval.allCases) { interval in
                        Text(localizer.text("%lld seconds", Int64(interval.rawValue))).tag(interval.rawValue)
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
