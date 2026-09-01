import AppKit
import SwiftUI

struct AboutSettingsView: View {
    private var displayName: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleDisplayName") as? String ?? "AI Meter"
    }

    private var versionText: String {
        guard let version = Bundle.main.object(
            forInfoDictionaryKey: "CFBundleShortVersionString"
        ) as? String,
        let build = Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String else {
            return "Version unavailable"
        }
        return "Version \(version) (\(build))"
    }

    var body: some View {
        Form {
            Section {
                HStack(spacing: 16) {
                    Image(nsImage: NSApplication.shared.applicationIconImage)
                        .resizable()
                        .frame(width: 64, height: 64)
                        .accessibilityHidden(true)
                    VStack(alignment: .leading, spacing: 4) {
                        Text(displayName)
                            .font(.title2.weight(.semibold))
                        Text("Private usage monitor")
                            .foregroundStyle(.secondary)
                        Text(versionText)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                .padding(.vertical, 8)
            }

            Section("Privacy") {
                Text("Claude and Codex credentials stay with their official CLIs. The DeepSeek API Key is stored in Keychain, and local history contains only normalized aggregate usage.")
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}
