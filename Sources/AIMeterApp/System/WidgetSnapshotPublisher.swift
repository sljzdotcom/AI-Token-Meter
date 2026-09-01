import AIMeterCore
import Foundation
import WidgetKit

@MainActor
struct WidgetSnapshotPublisher {
    typealias Save = (WidgetSnapshotEnvelope) throws -> Void
    typealias Reload = () -> Void
    typealias Log = (String) -> Void

    private let builder: WidgetSnapshotBuilder
    private let now: () -> Date
    private let save: Save
    private let reload: Reload
    private let log: Log

    init(
        builder: WidgetSnapshotBuilder = WidgetSnapshotBuilder(),
        now: @escaping () -> Date = Date.init,
        save: @escaping Save,
        reload: @escaping Reload,
        log: @escaping Log = { _ in }
    ) {
        self.builder = builder
        self.now = now
        self.save = save
        self.reload = reload
        self.log = log
    }

    func publish(_ snapshots: [UsageSnapshot]) {
        do {
            try save(builder.build(snapshots: snapshots, generatedAt: now()))
            reload()
        } catch {
            log("Widget snapshot publication failed.")
        }
    }

    static func production(
        appGroupIdentifier: String? = Bundle.main.object(
            forInfoDictionaryKey: AITokenMeterWidgetContract.appGroupInfoKey
        ) as? String,
        fileManager: FileManager = .default,
        log: @escaping Log = { _ in }
    ) -> WidgetSnapshotPublisher? {
        guard let appGroupIdentifier = appGroupIdentifier?
            .trimmingCharacters(in: .whitespacesAndNewlines),
              !appGroupIdentifier.isEmpty,
              let store = WidgetSnapshotStore(
                appGroupIdentifier: appGroupIdentifier,
                fileManager: fileManager
              ) else {
            return nil
        }
        return WidgetSnapshotPublisher(
            save: store.save,
            reload: {
                WidgetCenter.shared.reloadTimelines(ofKind: AITokenMeterWidgetContract.kind)
            },
            log: log
        )
    }
}
