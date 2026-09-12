import AIMeterCore
import Foundation
import Testing
@preconcurrency import UserNotifications
@testable import AIMeterApp

@Suite("Notification presentation")
struct NotificationServiceTests {
    @Test("Notification payloads read the selected language at each send")
    @MainActor
    func notificationPayloadUsesLatestLanguage() throws {
        var language = AppLanguage.english
        var requests: [UNNotificationRequest] = []
        let service = NotificationService(language: { language }, enqueue: { requests.append($0) })
        let event = ThresholdEvent(provider: .codex, metricLabel: "Weekly limit", level: .warning, usedFraction: 0.734)
        for (selected, title, body) in [
            (AppLanguage.english, "Usage reached 70%", "OpenAI Codex · Weekly limit is at 73%."),
            (.simplifiedChinese, "用量已达到 70%", "OpenAI Codex · 每周限额已用 73%。"),
            (.traditionalChinese, "用量已達到 70%", "OpenAI Codex · 每週限額已用 73%。"),
        ] {
            language = selected
            service.send([event])
            let request = try #require(requests.last)
            #expect(request.content.title == title)
            #expect(request.content.body == body)
            #expect(request.content.userInfo["provider"] as? String == "codex")
            #expect(request.content.sound != nil)
            #expect(request.trigger == nil)
        }
        #expect(requests.count == 3)
        #expect(Set(requests.map(\.identifier)).count == 3)
        language = .simplifiedChinese
        service.send([ThresholdEvent(provider: .gemini, metricLabel: "Claude/GPT · Weekly", level: .critical, usedFraction: 0.906)])
        #expect(requests.last?.content.title == "用量已达到 90%")
        #expect(requests.last?.content.body == "Google Antigravity · Claude/GPT · 每周已用 91%。")
        service.send([ThresholdEvent(provider: .codex, metricLabel: "External quota label", level: .warning, usedFraction: 0.7)])
        #expect(requests.last?.content.body == "OpenAI Codex · External quota label已用 70%。")
    }

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
