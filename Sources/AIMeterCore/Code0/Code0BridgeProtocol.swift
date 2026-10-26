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

/// Phase-one deny-only contract. No page message can authorize a capture yet.
public struct Code0BridgeValidator: Sendable {
    public init(connectionEpoch: UUID, attemptID: UUID, nonce: UUID, documentID: UUID) {}

    public mutating func receive(_ payload: Data, context: Code0BridgeContext) -> Code0BridgeEvent {
        .rejected(.validationRequired)
    }

    public mutating func dispose() {}
}
