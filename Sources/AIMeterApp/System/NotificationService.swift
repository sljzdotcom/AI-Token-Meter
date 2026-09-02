import AIMeterCore
@preconcurrency import UserNotifications

@MainActor
final class NotificationService: NSObject, UNUserNotificationCenterDelegate {
    private let center = UNUserNotificationCenter.current()
    var onOpenProvider: ((UsageProvider) -> Void)?

    override init() {
        super.init()
        center.delegate = self
    }

    func requestAuthorization() {
        Task {
            _ = try? await center.requestAuthorization(options: [.alert, .sound])
        }
    }

    func send(_ events: [ThresholdEvent]) {
        for event in events {
            let content = UNMutableNotificationContent()
            content.title = event.level == .critical ? "Usage reached 90%" : "Usage reached 70%"
            content.body = Self.notificationBody(for: event)
            content.sound = .default
            content.userInfo = ["provider": event.provider.rawValue]
            let request = UNNotificationRequest(
                identifier: "\(event.provider.rawValue)-\(event.metricLabel)-\(event.level.rawValue)-\(UUID().uuidString)",
                content: content,
                trigger: nil
            )
            Task { try? await center.add(request) }
        }
    }

    static func notificationBody(for event: ThresholdEvent) -> String {
        "\(event.provider.displayName) · \(event.metricLabel) is at \(Int((event.usedFraction * 100).rounded()))%."
    }

    nonisolated func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse,
        withCompletionHandler completionHandler: @escaping @Sendable () -> Void
    ) {
        let rawProvider = response.notification.request.content.userInfo["provider"] as? String
        Task { @MainActor [weak self] in
            if let rawProvider, let provider = UsageProvider(rawValue: rawProvider) {
                self?.onOpenProvider?(provider)
            }
            completionHandler()
        }
    }
}
