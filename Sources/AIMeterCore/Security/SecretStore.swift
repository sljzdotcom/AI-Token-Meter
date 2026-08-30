import Foundation

public protocol SecretStore: Sendable {
    func read() throws -> String?
    func save(_ secret: String) throws
    func delete() throws
}

public enum SecretStoreError: Error, Equatable, Sendable {
    case unexpectedStatus(Int32)
    case invalidData
}
