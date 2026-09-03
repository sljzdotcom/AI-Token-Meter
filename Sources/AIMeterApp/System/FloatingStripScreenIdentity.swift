import AppKit
import CoreGraphics
import Foundation

struct FloatingStripScreenIdentity: Equatable, Sendable {
    let stableIdentifier: String
    let legacyIdentifier: String?
    let isMain: Bool
}

enum FloatingStripScreenIdentifier {
    static func make(uuidString: String?, legacyIdentifier: String?) -> String? {
        if let uuidString, !uuidString.isEmpty {
            return "uuid:\(uuidString.lowercased())"
        }
        if let legacyIdentifier, !legacyIdentifier.isEmpty {
            return "legacy:\(legacyIdentifier)"
        }
        return nil
    }

    static func identity(for screen: NSScreen, mainScreen: NSScreen?) -> FloatingStripScreenIdentity? {
        let key = NSDeviceDescriptionKey("NSScreenNumber")
        guard let displayNumber = screen.deviceDescription[key] as? NSNumber else { return nil }
        let displayID = CGDirectDisplayID(displayNumber.uint32Value)
        let legacyIdentifier = displayNumber.stringValue
        let mainDisplayID = (mainScreen?.deviceDescription[key] as? NSNumber)?.uint32Value
        let uuidString = CGDisplayCreateUUIDFromDisplayID(displayID)
            .map { $0.takeRetainedValue() }
            .flatMap { CFUUIDCreateString(nil, $0) as String? }
        guard let stableIdentifier = make(
            uuidString: uuidString,
            legacyIdentifier: legacyIdentifier
        ) else { return nil }
        return FloatingStripScreenIdentity(
            stableIdentifier: stableIdentifier,
            legacyIdentifier: legacyIdentifier,
            isMain: mainDisplayID == displayID
        )
    }
}
