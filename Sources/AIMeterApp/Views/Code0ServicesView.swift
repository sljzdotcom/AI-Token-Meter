import AIMeterCore
import SwiftUI

/// Appearance and timeline updates only render state. Only a pressed button
/// enters the explicit action API; no view lifecycle starts website work.
struct Code0ServicesView: View {
    @Bindable var model: AppModel
    private var localizer: AppLocalizer { AppLocalizer(language: model.appLanguage) }

    var body: some View {
        TimelineView(.periodic(from: .now, by: 1)) { _ in
            VStack(alignment: .leading, spacing: 10) {
                HStack {
                    Text(localizer.text(model.code0StatusKey))
                        .aiMeterFont(.caption)
                    Spacer()
                    if [.connecting, .syncing, .closing].contains(model.code0SessionState) {
                        ProgressView().controlSize(.small)
                    }
                }
                if let snapshot = model.snapshots.first(where: { $0.provider == .code0 }) {
                    Text(localizer.text("Last successful sync: %@", localizer.date(snapshot.fetchedAt)))
                        .aiMeterFont(.caption).foregroundStyle(.secondary)
                }
                HStack {
                    switch model.code0SessionState {
                    case .disconnected:
                        action(.connect, title: "Connect Code0")
                    case .connecting:
                        action(.loginCompleted, title: "I've finished signing in")
                        action(.disconnect, title: "Disconnect Code0")
                    case .paused, .active:
                        if model.snapshots.contains(where: { $0.provider == .code0 }) {
                            action(.syncNow, title: "Sync Code0")
                        } else { action(.connect, title: "Connect Code0") }
                        action(.disconnect, title: "Disconnect Code0")
                    case .syncing:
                        action(.syncNow, title: "Sync Code0").disabled(true)
                        action(.disconnect, title: "Disconnect Code0")
                    case .closing, .cleanupPending:
                        action(.connect, title: "Connect Code0").disabled(true)
                    }
                }
                if let message = model.code0ActionMessage(using: localizer) {
                    Text(message).aiMeterFont(.caption).foregroundStyle(.secondary)
                }
                Text(localizer.text("Sync attempts are at least five minutes apart. A failed attempt pauses automatic sync."))
                    .aiMeterFont(.caption).foregroundStyle(.secondary)
                Text(localizer.text("Code0 shows website balance and cumulative site credits. No API Key is needed."))
                    .aiMeterFont(.caption).foregroundStyle(.secondary)
            }
        }
    }
    private func action(_ action: Code0ServiceAction, title: String) -> some View {
        Button(localizer.text(title)) { model.performCode0ServiceAction(action) }
            .buttonStyle(.bordered)
            .disabled(!model.canPerformCode0Action(action))
    }
}
