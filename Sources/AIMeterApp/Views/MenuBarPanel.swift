import AppKit
import SwiftUI

struct MenuBarLabel: View {
    @Bindable var model: AppModel

    var body: some View {
        Label(model.menuBarSummary.valueText, systemImage: "gauge.with.dots.needle.50percent")
            .accessibilityLabel(model.menuBarSummary.accessibilityLabel)
    }
}

struct MenuBarPanel: View {
    @Bindable var model: AppModel

    var body: some View {
        VStack(spacing: 14) {
            header

            if model.snapshots.isEmpty {
                ContentUnavailableView {
                    Label("Checking usage", systemImage: "gauge.with.dots.needle.50percent")
                } description: {
                    Text("Claude, Codex, and DeepSeek are being checked locally.")
                }
                .frame(height: 190)
            } else {
                VStack(spacing: 10) {
                    ForEach(model.snapshots) { snapshot in
                        ProviderCard(snapshot: snapshot)
                    }
                }
            }

            Divider()
            footer
        }
        .padding(16)
        .frame(width: 380)
    }

    private var header: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text("AI Meter")
                    .font(.title2.bold())
                Text("Private usage monitor")
                    .font(.caption)
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
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            } else {
                Text("Waiting for first refresh")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            SettingsLink {
                Image(systemName: "gearshape")
            }
            .buttonStyle(.borderless)
            .help("Settings")
            Button {
                NSApplication.shared.terminate(nil)
            } label: {
                Image(systemName: "power")
            }
            .buttonStyle(.borderless)
            .help("Quit AI Meter")
        }
    }
}
