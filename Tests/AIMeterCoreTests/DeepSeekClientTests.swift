import Foundation
import Testing
@testable import AIMeterCore

@Suite("DeepSeek balance client", .serialized)
struct DeepSeekClientTests {
    @Test("Reads the official balance without inventing a usage percentage")
    func readsOfficialBalance() async throws {
        let recorder = RequestRecorder(response: .json(200, """
        {
          "is_available": true,
          "balance_infos": [
            {
              "currency": "CNY",
              "total_balance": "110.00",
              "granted_balance": "10.00",
              "topped_up_balance": "100.00"
            }
          ]
        }
        """))
        let client = DeepSeekClient(session: recorder.session)

        let snapshot = try await client.collect(apiKey: "test-secret-value")

        #expect(snapshot.provider == .deepSeek)
        #expect(snapshot.availability == .available)
        #expect(snapshot.primaryMetric?.current == 110)
        #expect(snapshot.primaryMetric?.unit == .cny)
        #expect(snapshot.primaryMetric?.kind == .balance)
        #expect(snapshot.primaryMetric?.usedFraction == nil)
        let request = try #require(recorder.lastRequest)
        #expect(request.httpMethod == "GET")
        #expect(request.url == URL(string: "https://api.deepseek.com/user/balance"))
        #expect(request.value(forHTTPHeaderField: "Accept") == "application/json")
        #expect(request.value(forHTTPHeaderField: "Authorization") == "Bearer test-secret-value")
    }

    @Test("Selects the requested currency from a multi-currency response")
    func selectsPreferredCurrency() async throws {
        let recorder = RequestRecorder(response: .json(200, """
        {
          "is_available": true,
          "balance_infos": [
            {"currency":"CNY","total_balance":"8.50","granted_balance":"0","topped_up_balance":"8.50"},
            {"currency":"USD","total_balance":"42.25","granted_balance":"2.25","topped_up_balance":"40"}
          ]
        }
        """))
        let client = DeepSeekClient(session: recorder.session, preferredCurrency: .usd)

        let snapshot = try await client.collect(apiKey: "test-key")

        #expect(snapshot.primaryMetric?.current == 42.25)
        #expect(snapshot.primaryMetric?.unit == .usd)
    }

    @Test("Keeps a local budget explicitly separate from official balance")
    func marksLocalBudget() async throws {
        let recorder = RequestRecorder(response: .json(200, """
        {"is_available":true,"balance_infos":[{"currency":"CNY","total_balance":"61","granted_balance":"1","topped_up_balance":"60"}]}
        """))
        let client = DeepSeekClient(session: recorder.session)
        let budget = DeepSeekBudget(monthlyLimit: 100, trackedSpend: 35, currency: .cny)

        let snapshot = try await client.collect(apiKey: "test-key", budget: budget)

        #expect(snapshot.primaryMetric?.kind == .balance)
        #expect(snapshot.primaryMetric?.current == 61)
        #expect(snapshot.primaryMetric?.usedFraction == nil)
        #expect(snapshot.secondaryMetric?.kind == .localBudget)
        #expect(snapshot.secondaryMetric?.current == 35)
        #expect(snapshot.secondaryMetric?.limit == 100)
        #expect(snapshot.secondaryMetric?.usedFraction == 0.35)
    }

    @Test("Reports an unavailable account while retaining its balance")
    func reportsUnavailableAccount() async throws {
        let recorder = RequestRecorder(response: .json(200, """
        {"is_available":false,"balance_infos":[{"currency":"CNY","total_balance":"0","granted_balance":"0","topped_up_balance":"0"}]}
        """))

        let snapshot = try await DeepSeekClient(session: recorder.session).collect(apiKey: "test-key")

        #expect(snapshot.availability == .unavailable)
        #expect(snapshot.primaryMetric?.current == 0)
    }

    @Test("Rejects an empty or malformed balance response")
    func rejectsInvalidResponses() async {
        for body in [
            #"{"is_available":true,"balance_infos":[]}"#,
            #"{"is_available":true,"balance_infos":[{"currency":"CNY","total_balance":"not-a-number","granted_balance":"0","topped_up_balance":"0"}]}"#,
        ] {
            let recorder = RequestRecorder(response: .json(200, body))
            await #expect(throws: UsageCollectionError.invalidResponse) {
                try await DeepSeekClient(session: recorder.session).collect(apiKey: "test-key")
            }
        }
    }

    @Test("Maps authentication, rate limit, server, and timeout failures")
    func mapsTransportFailures() async {
        let cases: [(StubResponse, UsageCollectionError)] = [
            (.json(401, #"{"error":{"message":"invalid key"}}"#), .authenticationRequired),
            (.json(429, #"{"error":{"message":"rate limited"}}"#), .rateLimited),
            (.json(500, #"{"error":{"message":"server failure"}}"#), .transportFailure),
            (.failure(URLError(.timedOut)), .timedOut),
        ]

        for (response, expectedError) in cases {
            let recorder = RequestRecorder(response: response)
            await #expect(throws: expectedError) {
                try await DeepSeekClient(session: recorder.session).collect(apiKey: "test-key")
            }
        }
    }

    @Test("Collector requires a stored API key before making a request")
    func collectorRequiresStoredKey() async {
        let recorder = RequestRecorder(response: .json(500, "{}"))
        let collector = DeepSeekCollector(
            client: DeepSeekClient(session: recorder.session),
            secretStore: FixedSecretStore(secret: nil)
        )

        await #expect(throws: UsageCollectionError.authenticationRequired) {
            try await collector.collect()
        }
        #expect(recorder.lastRequest == nil)
    }

    @Test("Collector credential reads are not scheduled below user initiated work")
    func collectorCredentialReadPriority() async {
        let recorder = RequestRecorder(response: .json(500, "{}"))
        let store = PriorityRecordingSecretStore(secret: nil)
        let collector = DeepSeekCollector(
            client: DeepSeekClient(session: recorder.session),
            secretStore: store
        )

        await #expect(throws: UsageCollectionError.authenticationRequired) {
            try await collector.collect()
        }
        #expect(store.readPriority.rawValue >= TaskPriority.userInitiated.rawValue)
    }

    @Test("Direct client rejects an empty API key before making a request")
    func clientRejectsEmptyKey() async {
        let recorder = RequestRecorder(response: .json(500, "{}"))

        await #expect(throws: UsageCollectionError.authenticationRequired) {
            try await DeepSeekClient(session: recorder.session).collect(apiKey: "  \n")
        }
        #expect(recorder.lastRequest == nil)
    }

    @Test("Balance requests enforce the ten second collection deadline")
    func requestUsesTenSecondTimeout() async throws {
        let recorder = RequestRecorder(response: .json(200, """
        {"is_available":true,"balance_infos":[{"currency":"CNY","total_balance":"1","granted_balance":"0","topped_up_balance":"1"}]}
        """))

        _ = try await DeepSeekClient(session: recorder.session).collect(apiKey: "test-key")

        #expect(recorder.lastRequest?.timeoutInterval == 10)
    }

    @Test("Collector uses the Keychain abstraction without exposing it to callers")
    func collectorUsesSecretStore() async throws {
        let recorder = RequestRecorder(response: .json(200, """
        {"is_available":true,"balance_infos":[{"currency":"CNY","total_balance":"12.5","granted_balance":"0","topped_up_balance":"12.5"}]}
        """))
        let collector = DeepSeekCollector(
            client: DeepSeekClient(session: recorder.session),
            secretStore: FixedSecretStore(secret: "stored-test-key")
        )

        let snapshot = try await collector.collect()

        #expect(snapshot.provider == .deepSeek)
        #expect(snapshot.primaryMetric?.current == 12.5)
        #expect(recorder.lastRequest?.value(forHTTPHeaderField: "Authorization") == "Bearer stored-test-key")
    }

    @Test("A blocked Keychain read times out without blocking provider refresh")
    func blockedSecretReadTimesOut() async {
        let recorder = RequestRecorder(response: .json(500, "{}"))
        let collector = DeepSeekCollector(
            client: DeepSeekClient(session: recorder.session),
            secretStore: SlowSecretStore(delay: 0.2),
            secretReadTimeout: .milliseconds(20)
        )

        await #expect(throws: UsageCollectionError.timedOut) {
            try await collector.collect()
        }
        #expect(recorder.lastRequest == nil)
    }
}

private enum StubResponse {
    case json(Int, String)
    case failure(Error)
}

private struct FixedSecretStore: SecretStore {
    let secret: String?

    func read() throws -> String? { secret }
    func save(_ secret: String) throws {}
    func delete() throws {}
}

private struct SlowSecretStore: SecretStore {
    let delay: TimeInterval

    func read() throws -> String? {
        Thread.sleep(forTimeInterval: delay)
        return "late-secret"
    }

    func save(_ secret: String) throws {}
    func delete() throws {}
}

private final class PriorityRecordingSecretStore: SecretStore, @unchecked Sendable {
    private let lock = NSLock()
    private let secret: String?
    private var recordedPriority: TaskPriority = .background

    init(secret: String?) {
        self.secret = secret
    }

    func read() throws -> String? {
        lock.withLock { recordedPriority = Task.currentPriority }
        return secret
    }

    func save(_ secret: String) throws {}
    func delete() throws {}

    var readPriority: TaskPriority { lock.withLock { recordedPriority } }
}

private final class RequestRecorder: @unchecked Sendable {
    private let lock = NSLock()
    private var request: URLRequest?
    let session: URLSession

    init(response: StubResponse) {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [DeepSeekURLProtocol.self]
        let session = URLSession(configuration: configuration)
        self.session = session
        DeepSeekURLProtocol.install { [weak self] request in
            self?.lock.withLock { self?.request = request }
            switch response {
            case let .json(status, body):
                let response = HTTPURLResponse(
                    url: request.url!,
                    statusCode: status,
                    httpVersion: "HTTP/1.1",
                    headerFields: ["Content-Type": "application/json"]
                )!
                return (response, Data(body.utf8))
            case let .failure(error):
                throw error
            }
        }
    }

    var lastRequest: URLRequest? {
        lock.withLock { request }
    }

    deinit {
        session.invalidateAndCancel()
        DeepSeekURLProtocol.install(nil)
    }
}

private final class DeepSeekURLProtocol: URLProtocol, @unchecked Sendable {
    typealias Handler = @Sendable (URLRequest) throws -> (HTTPURLResponse, Data)
    private static let lock = NSLock()
    nonisolated(unsafe) private static var handler: Handler?

    static func install(_ handler: Handler?) {
        lock.withLock { self.handler = handler }
    }

    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        do {
            let handler = Self.lock.withLock { Self.handler }
            let (response, data) = try handler!(request)
            client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: data)
            client?.urlProtocolDidFinishLoading(self)
        } catch {
            client?.urlProtocol(self, didFailWithError: error)
        }
    }

    override func stopLoading() {}
}
