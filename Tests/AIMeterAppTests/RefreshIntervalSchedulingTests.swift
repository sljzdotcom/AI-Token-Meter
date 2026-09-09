import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("Refresh interval scheduling", .serialized)
@MainActor
struct RefreshIntervalSchedulingTests {
    @Test func replacesWaitingTimerAndRejectsLateCompletions() async throws {
        let context = IntervalContext()
        defer { context.clean() }
        let boundary = IntervalBoundary()
        let model = context.model(boundary)
        model.start()
        defer { model.stop() }
        try await eventually { await boundary.durations == [.seconds(300)] }
        #expect(await boundary.refreshCount == 1)
        #expect(model.setRefreshInterval("60"))
        try await eventually { await boundary.durations == [.seconds(300), .seconds(60)] }
        #expect(await boundary.cancelled == [0])
        #expect(await boundary.accountCount == 1)
        #expect(await boundary.refreshCount == 1)
        // A cancelled wait deliberately ignores cancellation and finishes late.
        await boundary.releaseWait(0)
        await settle()
        #expect(await boundary.refreshCount == 1)
        await boundary.releaseWait(1)
        try await eventually { await boundary.refreshCount == 2 }
        try await eventually { await boundary.durations.count == 3 }
        #expect(model.setRefreshInterval("60"))
        await settle()
        #expect(await boundary.durations.count == 3)
        #expect(model.setRefreshInterval("90"))
        try await eventually { await boundary.durations.count == 4 }
        model.stop()
        await boundary.releaseWait(2)
        await boundary.releaseWait(3)
        await settle()
        #expect(await boundary.refreshCount == 2)
        #expect(model.refreshIntervalSeconds == 90)
        let restarted = context.model(IntervalBoundary())
        #expect(restarted.refreshIntervalSeconds == 90)
    }

    @Test func editingDuringCollectionDoesNotCancelOrOverlap() async throws {
        let context = IntervalContext()
        defer { context.clean() }
        let boundary = IntervalBoundary(gateRefresh: true)
        let model = context.model(boundary)
        model.start()
        defer { model.stop() }
        try await eventually { await boundary.refreshCount == 1 }
        #expect(model.isRefreshing)
        #expect(model.setRefreshInterval("60"))
        #expect(model.setRefreshInterval("30"))
        await settle()
        #expect(await boundary.refreshCount == 1)
        #expect(await boundary.durations.isEmpty)
        await boundary.releaseRefresh()
        try await eventually { await boundary.durations == [.seconds(30)] }
        #expect(await boundary.collectionCancelled == false)
        #expect(!model.isRefreshing)
        model.stop()
        await boundary.releaseWait(0)
    }

    @Test(arguments: ["garbage", "29", "86401", "30.5", "true", "missing"])
    func corruptPersistenceFallsBack(_ value: String) {
        let context = IntervalContext()
        defer { context.clean() }
        switch value {
        case "true": context.defaults.set(true, forKey: "refreshIntervalSeconds")
        case "missing": break
        default:
            if let number = Double(value) { context.defaults.set(number, forKey: "refreshIntervalSeconds") }
            else { context.defaults.set(value, forKey: "refreshIntervalSeconds") }
        }
        #expect(context.model(IntervalBoundary()).refreshIntervalSeconds == 300)
    }
}

@MainActor
private struct IntervalContext {
    let suite = "RefreshIntervalSchedulingTests.\(UUID())"
    let defaults: UserDefaults
    init() { defaults = UserDefaults(suiteName: suite)! }
    func clean() { defaults.removePersistentDomain(forName: suite) }
    func model(_ boundary: IntervalBoundary) -> AppModel {
        AppModel(defaults: defaults, secretStore: SchedulingSecretStore(), widgetSnapshotPublisher: nil,
                 isDemoMode: false, refreshOperation: { await boundary.refresh() },
                 serviceAccountRefreshOperation: { _ in await boundary.accounts() },
                 refreshSleep: { try await boundary.sleep($0) })
    }
}
private struct SchedulingSecretStore: SecretStore {
    func read() throws -> String? { nil }
    func save(_ secret: String) throws {}
    func delete() throws {}
}
private actor IntervalBoundary {
    var durations: [Duration] = []
    var cancelled: [Int] = []
    var refreshCount = 0
    var accountCount = 0
    var collectionCancelled = false
    var waits: [Int: CheckedContinuation<Void, Never>] = [:]
    var refreshContinuation: CheckedContinuation<Void, Never>?
    let gateRefresh: Bool
    init(gateRefresh: Bool = false) { self.gateRefresh = gateRefresh }
    func sleep(_ duration: Duration) async throws {
        let index = durations.count
        durations.append(duration)
        await withTaskCancellationHandler {
            await withCheckedContinuation { waits[index] = $0 }
        } onCancel: {
            Task { await self.recordCancellation(index) }
        }
    }
    func recordCancellation(_ index: Int) { cancelled.append(index) }
    func releaseWait(_ index: Int) { waits.removeValue(forKey: index)?.resume() }
    func refresh() async -> [UsageSnapshot] {
        refreshCount += 1
        if gateRefresh { await withCheckedContinuation { refreshContinuation = $0 } }
        collectionCancelled = Task.isCancelled
        return []
    }
    func releaseRefresh() { refreshContinuation?.resume(); refreshContinuation = nil }
    func accounts() -> [ServiceAccountStatus] { accountCount += 1; return [] }
}
@MainActor
private func eventually(_ condition: () async -> Bool) async throws {
    let deadline = ContinuousClock.now.advanced(by: .seconds(2))
    while ContinuousClock.now < deadline {
        if await condition() { return }
        try await Task.sleep(for: .milliseconds(5))
    }
    #expect(await condition())
}
private func settle() async { try? await Task.sleep(for: .milliseconds(30)) }
