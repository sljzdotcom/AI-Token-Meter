import AIMeterCore
import Foundation
import Testing
@testable import AIMeterApp

@Suite("Notification presentation")
struct NotificationServiceTests {
    @Test("Notification body uses the canonical provider name")
    @MainActor
    func canonicalProviderName() {
        let event = ThresholdEvent(
            provider: .codex,
            metricLabel: "Weekly limit",
            level: .warning,
            usedFraction: 0.73
        )

        #expect(
            NotificationService.notificationBody(for: event)
                == "OpenAI Codex · Weekly limit is at 73%."
        )
    }

    @Test("UserNotifications keeps cross-SDK concurrency compatibility")
    func userNotificationsImportIsPreconcurrency() throws {
        let projectRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let source = try String(contentsOf: projectRoot.appending(
            path: "Sources/AIMeterApp/System/NotificationService.swift"
        ))

        #expect(source.contains("@preconcurrency import UserNotifications"))
    }
}
