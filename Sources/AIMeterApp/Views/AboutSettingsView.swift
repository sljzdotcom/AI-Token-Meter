import AppKit
import AIMeterCore
import SwiftUI

struct AboutSettingsView: View {
    private var versionText: String {
        AppBrand.versionText(info: Bundle.main.infoDictionary ?? [:])
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
                        Text(AppBrand.displayName)
                            .font(.title2.weight(.semibold))
                        Text(AppBrand.subtitle)
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
