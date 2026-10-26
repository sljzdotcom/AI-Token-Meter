import Foundation
import Testing
@testable import AIMeterCore

@Suite("Code0 native bridge protocol")
struct Code0BridgeProtocolTests {
    private let epoch = UUID(uuidString: "11111111-1111-4111-8111-111111111111")!
    private let attempt = UUID(uuidString: "22222222-2222-4222-8222-222222222222")!
    private let nonce = UUID(uuidString: "33333333-4444-4555-8666-777777777777")!
    private let document = UUID(uuidString: "88888888-9999-4000-8111-222222222222")!

    private var context: Code0BridgeContext {
        Code0BridgeContext(connectionEpoch: epoch, attemptID: attempt,
                           sourceURL: "https://console.code0.ai/console", isMainFrame: true)
    }
    private func validator() -> Code0BridgeValidator {
        Code0BridgeValidator(connectionEpoch: epoch, attemptID: attempt, nonce: nonce, documentID: document)
    }
    private func message(_ kind: String, _ sequence: Int, _ fields: [String: Any] = [:]) throws -> Data {
        var object: [String: Any] = ["version": 1, "nonce": nonce.uuidString.lowercased(),
                                   "document": document.uuidString.lowercased(), "sequence": sequence, "kind": kind]
        object.merge(fields) { _, replacement in replacement }
        return try JSONSerialization.data(withJSONObject: object, options: [.sortedKeys])
    }
    private func capture(_ sequence: Int, request: Int = 1, revision: Int = 0) throws -> Data {
        try message("capture", sequence, ["request": request, "revision": revision,
                                         "quota": 1_250_000, "usedQuota": 750_000, "divisor": 500_000])
    }
    private func rejected(_ event: Code0BridgeEvent) -> Bool {
        if case .rejected = event { return true }
        return false
    }

    @Test("Ready and complete latest self produce one normalized capture; replay is denied")
    func completeTraceCapturesExactlyOnce() throws {
        var subject = validator()
        let ready = subject.receive(try message("ready", 1), context: context)
        let started = subject.receive(try message("selfStart", 2, ["request": 1, "revision": 0]), context: context)
        let accepted = subject.receive(try capture(3), context: context)
        let expected = try Code0WebsiteCapture.decode(filteredPayload: Data(#"{"quota":1250000,"usedQuota":750000,"divisor":500000}"#.utf8))
        #expect(accepted == .capture(expected))
        #expect(ready == .control)
        #expect(started == .control)
        #expect(rejected(subject.receive(try capture(4), context: context)))
    }

    @Test("Overlapping refreshes isolate premature self until a new self starts after both completions")
    func overlappingRefreshTraceRequiresNewSelf() throws {
        var subject = validator()
        let controls: [(String, [String: Any])] = [
            ("ready", [:]), ("selfStart", ["request": 1, "revision": 0]),
            ("refreshStart", ["request": 2, "revision": 1]),
            ("refreshStart", ["request": 3, "revision": 2]),
            ("refreshEnd", ["request": 2, "revision": 2]),
            ("selfStart", ["request": 4, "revision": 2]),
            ("refreshEnd", ["request": 3, "revision": 2]),
        ]
        for (offset, item) in controls.enumerated() {
            #expect(subject.receive(try message(item.0, offset + 1, item.1), context: context) == .control)
        }
        #expect(rejected(subject.receive(try capture(8, request: 4, revision: 2), context: context)))
        // A rejected forged capture is a driver-close boundary. A fresh trace proves
        // the independently valid post-refresh path without reusing that instance.
        var clean = validator()
        for (offset, item) in controls.enumerated() {
            _ = clean.receive(try message(item.0, offset + 1, item.1), context: context)
        }
        #expect(clean.receive(try message("selfStart", 8, ["request": 5, "revision": 2]), context: context) == .control)
        let event = clean.receive(try capture(9, request: 5, revision: 2), context: context)
        if case .capture(let values) = event {
            #expect(values.balance == 2.5 && values.cumulativeUsage == 1.5)
        } else { Issue.record("A valid post-refresh trace must produce a capture, received \(event)") }
    }

    @Test("Fixed invalidation and failure controls preserve their distinct native meaning")
    func terminalControlsAndDispose() throws {
        for (kind, reason, expected) in [
            ("invalidate", "authenticationChanged", Code0BridgeEvent.invalidated(.authenticationChanged)),
            ("failure", "invalidResponse", Code0BridgeEvent.failure(.invalidResponse)),
        ] {
            var subject = validator()
            _ = subject.receive(try message("ready", 1), context: context)
            #expect(subject.receive(try message(kind, 2, ["reason": reason]), context: context) == expected)
            #expect(rejected(subject.receive(try capture(3), context: context)))
        }
        var subject = validator()
        subject.dispose()
        #expect(rejected(subject.receive(try message("ready", 1), context: context)))
    }

    @Test("Owned callback identity and actual source are required", arguments: [
        "epoch", "attempt", "frame", "http", "subdomain", "port", "userinfo", "empty",
    ])
    func rejectsUnownedContext(_ variant: String) throws {
        let candidate = Code0BridgeContext(
            connectionEpoch: variant == "epoch" ? UUID() : epoch,
            attemptID: variant == "attempt" ? UUID() : attempt,
            sourceURL: ["http": "http://console.code0.ai/console", "subdomain": "https://sub.console.code0.ai/console",
                        "port": "https://console.code0.ai:444/console", "userinfo": "https://x@console.code0.ai/console",
                        "empty": ""][variant] ?? context.sourceURL,
            isMainFrame: variant != "frame")
        var subject = validator()
        #expect(rejected(subject.receive(try message("ready", 1), context: candidate)))
    }


    @Test("Capture rechecks callback identity and message association after valid controls")
    func lateCallbackAndChangedAssociationAreRejected() throws {
        for variant in ["epoch", "attempt", "frame", "source", "nonce", "document"] {
            var subject = validator()
            _ = subject.receive(try message("ready", 1), context: context)
            _ = subject.receive(try message("selfStart", 2, ["request": 1, "revision": 0]), context: context)
            let callback = Code0BridgeContext(connectionEpoch: variant == "epoch" ? UUID() : epoch,
                attemptID: variant == "attempt" ? UUID() : attempt,
                sourceURL: variant == "source" ? "https://other.invalid/console" : context.sourceURL,
                isMainFrame: variant != "frame")
            let payload: Data
            if variant == "nonce" || variant == "document" {
                payload = try message("capture", 3, ["request": 1, "revision": 0, "quota": 1_250_000,
                    "usedQuota": 750_000, "divisor": 500_000, variant: UUID().uuidString])
            } else { payload = try capture(3) }
            #expect(rejected(subject.receive(payload, context: callback)))
        }
    }

    @Test("Exact envelope rejects unknown fields, types, identifiers and unsafe counters")
    func rejectsMalformedEnvelope() throws {
        let mutations: [[String: Any]] = [
            ["version": 2], ["version": true], ["nonce": UUID().uuidString], ["document": UUID().uuidString],
            ["sequence": 0], ["sequence": -1], ["sequence": 1.5], ["sequence": 9_007_199_254_740_992 as Int64],
            ["sequence": true], ["kind": "unknown"], ["quota": 1], ["reason": "raw-response"], ["request": 1],
        ]
        for fields in mutations {
            var subject = validator()
            #expect(rejected(subject.receive(try message("ready", 1, fields), context: context)))
        }
        let normal = String(decoding: try message("ready", 1), as: UTF8.self)
        for raw in ["{\"version\":1," + String(normal.dropFirst()), normal + " ".padding(toLength: 2049, withPad: " ", startingAt: 0), "[]", "{}"] {
            var subject = validator()
            #expect(rejected(subject.receive(Data(raw.utf8), context: context)))
        }
    }

    @Test("Reordered sequences, forged requests and revisions cannot publish", arguments: [
        "noReady", "sequence", "olderSelf", "request", "revision", "refreshInFlight", "unknownEnd", "reusedRequest", "revisionJump",
    ])
    func rejectsInvalidTrace(_ variant: String) throws {
        var subject = validator()
        if variant != "noReady" { _ = subject.receive(try message("ready", 1), context: context) }
        _ = subject.receive(try message("selfStart", 2, ["request": 1, "revision": 0]), context: context)
        var last = try capture(3)
        switch variant {
        case "sequence": last = try capture(2)
        case "olderSelf":
            _ = subject.receive(try message("selfStart", 3, ["request": 2, "revision": 0]), context: context)
            last = try capture(4)
        case "request": last = try capture(3, request: 2)
        case "revision": last = try capture(3, revision: 1)
        case "refreshInFlight":
            _ = subject.receive(try message("refreshStart", 3, ["request": 2, "revision": 1]), context: context)
            last = try capture(4, revision: 1)
        case "unknownEnd": last = try message("refreshEnd", 3, ["request": 9, "revision": 0])
        case "reusedRequest": last = try message("selfStart", 3, ["request": 1, "revision": 0])
        case "revisionJump": last = try message("refreshStart", 3, ["request": 2, "revision": 2])
        default: break
        }
        #expect(rejected(subject.receive(last, context: context)))
    }

    @Test("Control kinds reject another kind's fields and invalid fixed reasons")
    func rejectsMixedControlFields() throws {
        let cases: [(String, [String: Any])] = [
            ("refreshStart", ["request": 1, "revision": 1, "quota": 1]),
            ("selfStart", ["request": true, "revision": 0]),
            ("selfStart", ["request": 1.5, "revision": 0]),
            ("selfStart", ["request": 1, "revision": -1]),
            ("invalidate", ["reason": "invalidResponse"]),
            ("failure", ["reason": "authenticationChanged"]),
            ("failure", ["reason": "raw-exception-message"]),
        ]
        for (kind, fields) in cases {
            var subject = validator()
            _ = subject.receive(try message("ready", 1), context: context)
            #expect(rejected(subject.receive(try message(kind, 2, fields), context: context)))
        }
    }

    @Test("Capture numeric syntax, duplicate keys, extra fields and wrong control keys are rejected")
    func rejectsCapturePayloads() throws {
        let normal = String(decoding: try capture(3), as: UTF8.self)
        let variants = [
            normal.replacingOccurrences(of: "1250000", with: "9007199254740991.1"),
            normal.replacingOccurrences(of: "1250000", with: "true"),
            normal.replacingOccurrences(of: "500000", with: "0"),
            "{\"quota\":1," + String(normal.dropFirst()),
            "{\"identity\":1," + String(normal.dropFirst()),
        ]
        for raw in variants {
            var subject = validator()
            _ = subject.receive(try message("ready", 1), context: context)
            _ = subject.receive(try message("selfStart", 2, ["request": 1, "revision": 0]), context: context)
            #expect(rejected(subject.receive(Data(raw.utf8), context: context)))
        }
    }
}
