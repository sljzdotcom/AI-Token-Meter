import AppKit
import AIMeterCore
import SwiftUI

struct MenuBarLabel: View {
    @Bindable var model: AppModel

    var body: some View {
        Label {
            Text(model.menuBarSummary.valueText)
        } icon: {
            MenuBarMeterIcon(fraction: model.menuBarSummary.usageFraction)
        }
            .accessibilityLabel(model.menuBarSummary.accessibilityLabel)
            .aiMeterFontScope(.menuBarLabel(model.displayFontChoice))
    }
}

struct MenuBarPanel: View {
    @Bindable var model: AppModel
    let brandIcon: NSImage

    private var localizer: AppLocalizer { AppLocalizer(language: model.appLanguage) }
    @Environment(\.openSettings) private var openSettings

    init(
        model: AppModel,
        brandIcon: NSImage = NSApplication.shared.applicationIconImage
    ) {
        self.model = model
        self.brandIcon = brandIcon
    }

    var body: some View {
        VStack(spacing: 14) {
            header

            if model.snapshots.isEmpty {
                ContentUnavailableView {
                    Label {
                        Text(localizer.text("Checking usage"))
                    } icon: {
                        Image(systemName: "gauge.with.dots.needle.50percent")
                    }
                } description: {
                    Text(localizer.text("Claude Code, OpenAI Codex, and DeepSeek are being checked locally."))
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
        HStack(spacing: 10) {
            Image(nsImage: brandIcon)
                .resizable()
                .interpolation(.high)
                .frame(width: 40, height: 40)
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 2) {
                Text(AppBrand.displayName)
                    .aiMeterFont(.title2, weight: .bold)
                Text(localizer.text(AppBrand.subtitle))
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
            .help(localizer.text("Refresh now"))
        }
    }

    private var footer: some View {
        HStack {
            if model.stripPreferences.hiddenUntil != nil || !model.showFloatingStrip {
                Button(localizer.text("Show Floating Strip Now")) { model.setFloatingStripVisible(true) }
                    .buttonStyle(.borderless)
            }
            if let date = model.lastUpdatedAt {
                Text(localizer.text("Updated %@", localizer.date(date, dateStyle: .none, timeStyle: .short)))
                    .aiMeterFont(.caption2)
                    .foregroundStyle(.secondary)
            } else {
                Text(localizer.text("Waiting for first refresh"))
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
            .help(localizer.text("Settings"))
            Button {
                NSApplication.shared.terminate(nil)
            } label: {
                Image(systemName: "power")
                    .aiMeterSymbolFont(.body)
            }
            .buttonStyle(.borderless)
            .help(localizer.text("Quit %@", AppBrand.displayName))
        }
    }
}
