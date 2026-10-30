import Foundation

/// Context is supplied by the owned native callback, never decoded from page JSON.
public struct Code0BridgeContext: Equatable, Sendable {
    public let connectionEpoch: UUID
    public let attemptID: UUID
    public let sourceURL: String
    public let isMainFrame: Bool

    public init(connectionEpoch: UUID, attemptID: UUID, sourceURL: String, isMainFrame: Bool) {
        self.connectionEpoch = connectionEpoch
        self.attemptID = attemptID
        self.sourceURL = sourceURL
        self.isMainFrame = isMainFrame
    }
}

public enum Code0BridgeReason: String, Equatable, Sendable {
    case authenticationChanged
    case invalidResponse
}

public enum Code0BridgeRejection: Equatable, Sendable {
    case validationRequired
    case invalidContext
    case invalidPayload
    case invalidSequence
    case invalidState
    case disposed
}

public enum Code0BridgeEvent: Equatable, Sendable {
    case control
    case capture(Code0WebsiteCapture)
    case invalidated(Code0BridgeReason)
    case failure(Code0BridgeReason)
    case rejected(Code0BridgeRejection)
}

/// A single native-owned attempt. Any rejected message revokes this instance;
/// the driver must pause/close rather than continue after a protocol violation.
public struct Code0BridgeValidator: Sendable {
    private let connectionEpoch: UUID
    private let attemptID: UUID
    private let nonce: UUID
    private let documentID: UUID
    private var ready = false
    private var disposed = false
    private var lastSequence: UInt64 = 0
    private var lastRequest: UInt64 = 0
    private var revision: UInt64 = 0
    private var refreshRequests: Set<UInt64> = []
    private var pendingSelf: PendingSelf?

    private struct PendingSelf: Sendable {
        let request: UInt64
        let revision: UInt64
        let eligible: Bool
    }

    public init(connectionEpoch: UUID, attemptID: UUID, nonce: UUID, documentID: UUID) {
        self.connectionEpoch = connectionEpoch
        self.attemptID = attemptID
        self.nonce = nonce
        self.documentID = documentID
    }

    public mutating func receive(_ payload: Data, context: Code0BridgeContext) -> Code0BridgeEvent {
        guard !disposed else { return .rejected(.disposed) }
        guard context.connectionEpoch == connectionEpoch, context.attemptID == attemptID,
              context.isMainFrame, Self.isOfficialSource(context.sourceURL) else { return reject(.invalidContext) }
        guard payload.count <= 2048 else { return reject(.invalidPayload) }
        do {
            var parser = BridgeFields(bytes: Array(payload))
            let fields = try parser.parse()
            guard Self.counter(fields["version"], positive: true) == 1,
                  let nonceText = fields["nonce"]?.stringValue, Self.messageUUID(nonceText) == nonce,
                  let documentText = fields["document"]?.stringValue, Self.messageUUID(documentText) == documentID,
                  let sequence = Self.counter(fields["sequence"], positive: true),
                  let kind = fields["kind"]?.stringValue else { return reject(.invalidPayload) }
            guard sequence > lastSequence else { return reject(.invalidSequence) }
            let base: Set<String> = ["version", "nonce", "document", "sequence", "kind"]
            let extra: Set<String>
            switch kind {
            case "ready": extra = []
            case "refreshStart", "refreshEnd", "selfStart": extra = ["request", "revision"]
            case "capture": extra = ["request", "revision", "quota", "usedQuota", "divisor"]
            case "invalidate", "failure": extra = ["reason"]
            default: return reject(.invalidPayload)
            }
            guard Set(fields.keys) == base.union(extra) else { return reject(.invalidPayload) }
            if kind == "ready" {
                guard !ready else { return reject(.invalidState) }
                ready = true
                lastSequence = sequence
                return .control
            }
            guard ready else { return reject(.invalidState) }
            if kind == "invalidate" || kind == "failure" {
                let reason = fields["reason"]?.stringValue
                guard (kind == "invalidate" && reason == "authenticationChanged") ||
                      (kind == "failure" && reason == "invalidResponse") else { return reject(.invalidPayload) }
                dispose()
                return kind == "invalidate" ? .invalidated(.authenticationChanged) : .failure(.invalidResponse)
            }
            guard let request = Self.counter(fields["request"], positive: true),
                  let messageRevision = Self.counter(fields["revision"], positive: false) else { return reject(.invalidPayload) }
            switch kind {
            case "refreshStart":
                guard request > lastRequest, revision < 9_007_199_254_740_991,
                      messageRevision == revision + 1 else { return reject(.invalidState) }
                revision = messageRevision
                lastRequest = request
                refreshRequests.insert(request)
                pendingSelf = nil
            case "refreshEnd":
                guard messageRevision == revision, refreshRequests.contains(request) else { return reject(.invalidState) }
                refreshRequests.remove(request)
                pendingSelf = nil
            case "selfStart":
                guard request > lastRequest, messageRevision == revision else { return reject(.invalidState) }
                lastRequest = request
                pendingSelf = PendingSelf(request: request, revision: revision, eligible: refreshRequests.isEmpty)
            case "capture":
                guard let pendingSelf, pendingSelf.eligible, refreshRequests.isEmpty,
                      pendingSelf.request == request, pendingSelf.revision == revision,
                      messageRevision == revision else { return reject(.invalidState) }
                guard let quota = fields["quota"]?.numberValue, let used = fields["usedQuota"]?.numberValue,
                      let divisor = fields["divisor"]?.numberValue else { return reject(.invalidPayload) }
                // Preserve original number tokens; dictionary NSNumber round trips
                // would lose the fractional/unsafe-boundary evidence.
                let filtered = Data("{\"quota\":\(quota),\"usedQuota\":\(used),\"divisor\":\(divisor)}".utf8)
                let capture = try Code0WebsiteCapture.decode(filteredPayload: filtered)
                dispose()
                return .capture(capture)
            default: return reject(.invalidPayload)
            }
            lastSequence = sequence
            return .control
        } catch { return reject(.invalidPayload) }
    }

    public mutating func dispose() {
        disposed = true
        pendingSelf = nil
        refreshRequests.removeAll()
    }

    private mutating func reject(_ reason: Code0BridgeRejection) -> Code0BridgeEvent {
        dispose()
        return .rejected(reason)
    }

    private static func messageUUID(_ text: String) -> UUID? {
        let bytes = Array(text.utf8)
        guard bytes.count == 36, bytes.enumerated().allSatisfy({ index, byte in
            if [8, 13, 18, 23].contains(index) { return byte == 45 }
            return (48...57).contains(byte) || (65...70).contains(byte) || (97...102).contains(byte)
        }) else { return nil }
        return UUID(uuidString: text)
    }

    private static func isOfficialSource(_ source: String) -> Bool {
        guard !source.utf8.contains(where: { $0 <= 32 || $0 == 127 || $0 == 92 }),
              let separator = source.range(of: "://") else { return false }
        let authority = source[separator.upperBound...].prefix { !"/?#".contains($0) }
        guard !authority.isEmpty, !authority.contains("@"),
              let url = URLComponents(string: source), url.url != nil,
              url.scheme?.lowercased() == "https", url.host?.lowercased() == "console.code0.ai",
              url.user == nil, url.password == nil, url.port == nil || url.port == 443 else { return false }
        return true
    }

    private static func counter(_ field: BridgeField?, positive: Bool) -> UInt64? {
        guard let raw = field?.numberValue, !raw.hasPrefix("-"),
              let value = try? Code0WebsiteCapture.decode(filteredPayload:
                Data("{\"quota\":\(raw),\"usedQuota\":0,\"divisor\":1}".utf8)),
              !positive || value.balance > 0 else { return nil }
        return UInt64(value.balance)
    }
}

private enum BridgeField: Sendable {
    case string(String)
    case number(String)
    var stringValue: String? { if case .string(let value) = self { return value }; return nil }
    var numberValue: String? { if case .number(let value) = self { return value }; return nil }
}

/// Flat, bounded JSON preserves number lexemes and observes decoded duplicate
/// keys directly. Booleans, containers, null and unknown fields are never valid.
private struct BridgeFields {
    enum Invalid: Error { case payload }
    let bytes: [UInt8]
    var index = 0
    private let allowed: Set<String> = ["version", "nonce", "document", "sequence", "kind", "request", "revision", "quota", "usedQuota", "divisor", "reason"]

    init(bytes: [UInt8]) { self.bytes = bytes }

    mutating func parse() throws -> [String: BridgeField] {
        guard take(123) else { throw Invalid.payload }
        var fields: [String: BridgeField] = [:]
        if take(125) { skipSpace(); guard index == bytes.count else { throw Invalid.payload }; return fields }
        while true {
            let key = try string()
            guard allowed.contains(key), fields[key] == nil, fields.count < 10, take(58) else { throw Invalid.payload }
            skipSpace()
            if index < bytes.count, bytes[index] == 34 { fields[key] = .string(try string()) }
            else { fields[key] = .number(try number()) }
            if take(125) { break }
            guard take(44) else { throw Invalid.payload }
        }
        skipSpace()
        guard index == bytes.count else { throw Invalid.payload }
        return fields
    }

    mutating func string() throws -> String {
        skipSpace()
        let start = index
        guard index < bytes.count, bytes[index] == 34 else { throw Invalid.payload }
        index += 1
        while index < bytes.count {
            let byte = bytes[index]
            index += 1
            if byte == 34 {
                return try JSONDecoder().decode(String.self, from: Data(bytes[start..<index]))
            }
            if byte == 92 { index += 1 }
        }
        throw Invalid.payload
    }

    mutating func number() throws -> String {
        let start = index
        if index < bytes.count, bytes[index] == 45 { index += 1 }
        guard index < bytes.count else { throw Invalid.payload }
        if bytes[index] == 48 { index += 1 }
        else {
            guard (49...57).contains(bytes[index]) else { throw Invalid.payload }
            digits()
        }
        if index < bytes.count, bytes[index] == 46 {
            index += 1
            let first = index
            digits()
            guard index > first else { throw Invalid.payload }
        }
        if index < bytes.count, bytes[index] == 69 || bytes[index] == 101 {
            index += 1
            if index < bytes.count, bytes[index] == 43 || bytes[index] == 45 { index += 1 }
            let first = index
            digits()
            guard index > first else { throw Invalid.payload }
        }
        return String(decoding: bytes[start..<index], as: UTF8.self)
    }

    mutating func take(_ byte: UInt8) -> Bool {
        skipSpace()
        guard index < bytes.count, bytes[index] == byte else { return false }
        index += 1
        return true
    }
    mutating func skipSpace() { while index < bytes.count, [9, 10, 13, 32].contains(bytes[index]) { index += 1 } }
    mutating func digits() { while index < bytes.count, (48...57).contains(bytes[index]) { index += 1 } }
}
