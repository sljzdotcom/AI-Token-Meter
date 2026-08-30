import Foundation
import Testing
@testable import AIMeterCore

@Suite("Refresh coordinator", .serialized)
struct RefreshCoordinatorTests {
    @Test("Runs independent provider collectors concurrently and sorts their results")
    func refreshesConcurrently() async {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let collectors = UsageProvider.allCases.reversed().map {
            ControlledCollector(provider: $0, delay: 0.2, result: .success(snapshot(for: $0)))
        }
        let coordinator = RefreshCoordinator(
            collectors: collectors,
            cache: SnapshotCache(directoryURL: directory)
        )
        let startedAt = Date()

        let snapshots = await coordinator.refresh()

        #expect(Date().timeIntervalSince(startedAt) < 0.5)
        #expect(snapshots.map(\.provider) == [.claude, .codex, .deepSeek])
        #expect(snapshots.allSatisfy { $0.collectionStatus == .fresh })
    }

    @Test("A failed provider falls back to its last good cache without blocking others")
    func fallsBackToCache() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let cache = SnapshotCache(directoryURL: directory)
        let oldCodex = UsageSnapshot(
            provider: .codex,
            primaryMetric: metric(value: 44),
            fetchedAt: Date(timeIntervalSince1970: 100),
            staleAfter: 60,
            collectionStatus: .fresh
        )
        try cache.save([oldCodex])
        let coordinator = RefreshCoordinator(
            collectors: [
                ControlledCollector(provider: .codex, result: .failure(.authenticationRequired)),
                ControlledCollector(provider: .claude, result: .success(snapshot(for: .claude))),
            ],
            cache: cache
        )

        let snapshots = await coordinator.refresh()
        let claude = try #require(snapshots.first(where: { $0.provider == .claude }))
        let codex = try #require(snapshots.first(where: { $0.provider == .codex }))

        #expect(claude.collectionStatus == .fresh)
        #expect(codex.collectionStatus == .cached)
        #expect(codex.primaryMetric?.current == 44)
        #expect(codex.fetchedAt == Date(timeIntervalSince1970: 100))
        #expect(codex.statusMessage == "Sign in required")
    }

    @Test("Two overlapping refresh requests share one collection pass")
    func mergesOverlappingRefreshes() async {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let collector = ControlledCollector(
            provider: .claude,
            delay: 0.15,
            result: .success(snapshot(for: .claude))
        )
        let coordinator = RefreshCoordinator(
            collectors: [collector],
            cache: SnapshotCache(directoryURL: directory)
        )

        async let first = coordinator.refresh()
        async let second = coordinator.refresh()
        let results = await (first, second)

        #expect(results.0 == results.1)
        #expect(collector.callCount == 1)
    }

    @Test("A failure without cache returns a sanitized actionable state")
    func reportsFailureWithoutCache() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let coordinator = RefreshCoordinator(
            collectors: [ControlledCollector(provider: .deepSeek, result: .failure(.rateLimited))],
            cache: SnapshotCache(directoryURL: directory)
        )

        let snapshot = try #require(await coordinator.refresh().first)

        #expect(snapshot.provider == .deepSeek)
        #expect(snapshot.collectionStatus == .unavailable)
        #expect(snapshot.statusMessage == "Rate limited; try again later")
        #expect(snapshot.primaryMetric == nil)
    }

    @Test("Claude workspace approval is distinct from a service outage")
    func reportsClaudeWorkspaceSetup() async throws {
        let directory = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let coordinator = RefreshCoordinator(
            collectors: [ControlledCollector(provider: .claude, result: .failure(.setupRequired))],
            cache: SnapshotCache(directoryURL: directory)
        )

        let snapshot = try #require(await coordinator.refresh().first)

        #expect(snapshot.provider == .claude)
        #expect(snapshot.collectionStatus == .setupRequired)
        #expect(snapshot.statusMessage == "Approve the private usage workspace once")
        #expect(snapshot.primaryMetric == nil)
    }

    private func temporaryDirectory() -> URL {
        FileManager.default.temporaryDirectory
            .appendingPathComponent("AI-Meter-RefreshTests-\(UUID().uuidString)", isDirectory: true)
    }

    private func snapshot(for provider: UsageProvider) -> UsageSnapshot {
        UsageSnapshot(
            provider: provider,
            primaryMetric: metric(value: Double(10 + UsageProvider.allCases.firstIndex(of: provider)!)),
            fetchedAt: Date(timeIntervalSince1970: 500),
            collectionStatus: .fresh
        )
    }

    private func metric(value: Double) -> UsageMetric {
        UsageMetric(label: "Usage", current: value, limit: 100, unit: .percent)
    }
}

private final class ControlledCollector: UsageCollector, @unchecked Sendable {
    let provider: UsageProvider
    let delay: TimeInterval
    let result: Result<UsageSnapshot, UsageCollectionError>

    private let lock = NSLock()
    private var calls = 0

    init(
        provider: UsageProvider,
        delay: TimeInterval = 0,
        result: Result<UsageSnapshot, UsageCollectionError>
    ) {
        self.provider = provider
        self.delay = delay
        self.result = result
    }

    var callCount: Int { lock.withLock { calls } }

    func collect() async throws -> UsageSnapshot {
        lock.withLock { calls += 1 }
        if delay > 0 {
            try await Task.sleep(for: .seconds(delay))
        }
        return try result.get()
    }
}
