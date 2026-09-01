import AppKit
import AIMeterCore
import SwiftUI

struct MenuBarLabel: View {
    @Bindable var model: AppModel

    var body: some View {
        Label {
            Text(model.menuBarSummary.valueText)
        } icon: {
            Image(systemName: "gauge.with.dots.needle.50percent")
                .aiMeterSymbolFont(.body)
        }
            .accessibilityLabel(model.menuBarSummary.accessibilityLabel)
            .aiMeterFontScope(.menuBarLabel(model.displayFontChoice))
    }
}

struct MenuBarPanel: View {
    @Bindable var model: AppModel
    @Environment(\.openSettings) private var openSettings

    var body: some View {
        VStack(spacing: 14) {
            header

            if model.snapshots.isEmpty {
                ContentUnavailableView {
                    Label {
                        Text("Checking usage")
                    } icon: {
                        Image(systemName: "gauge.with.dots.needle.50percent")
                    }
                } description: {
                    Text("Claude, Codex, and DeepSeek are being checked locally.")
                }
                .frame(height: 190)
            } else {
                VStack(spacing: 10) {
                    ForEach(model.snapshots) { snapshot in
                        ProviderCard(
                            snapshot: snapshot,
                            onClaudeSetup: {
                                model.openClaudeWorkspaceSetup()
                            }
                        )
                    }
                }
            }

            Divider()
            footer
        }
        .padding(16)
        .frame(width: 380)
        .aiMeterFontScope(.content(model.displayFontChoice))
    }

    private var header: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(AppBrand.displayName)
                    .aiMeterFont(.title2, weight: .bold)
                Text(AppBrand.subtitle)
                    .aiMeterFont(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            Button {
                Task { await model.refresh() }
            } label: {
                if model.isRefreshing {
                    ProgressView().controlSize(.small)
                } else {
                    Image(systemName: "arrow.clockwise")
                        .aiMeterSymbolFont(.body)
                }
            }
            .buttonStyle(.borderless)
            .disabled(model.isRefreshing)
            .help("Refresh now")
        }
    }

    private var footer: some View {
        HStack {
            if let date = model.lastUpdatedAt {
                Text("Updated \(date.formatted(date: .omitted, time: .shortened))")
                    .aiMeterFont(.caption2)
                    .foregroundStyle(.secondary)
            } else {
                Text("Waiting for first refresh")
                    .aiMeterFont(.caption2)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            Button {
                SettingsPresentationCommand(
                    activateApplication: {
                        NSApplication.shared.activate(ignoringOtherApps: true)
                    },
                    openSettings: { openSettings() }
                ).perform()
            } label: {
                Image(systemName: "gearshape")
                    .aiMeterSymbolFont(.body)
            }
            .buttonStyle(.borderless)
            .help("Settings")
            Button {
                NSApplication.shared.terminate(nil)
            } label: {
                Image(systemName: "power")
                    .aiMeterSymbolFont(.body)
            }
            .buttonStyle(.borderless)
            .help("Quit \(AppBrand.displayName)")
        }
    }
}
