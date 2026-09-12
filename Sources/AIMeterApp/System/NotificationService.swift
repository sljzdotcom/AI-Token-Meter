import AIMeterCore
@preconcurrency import UserNotifications

@MainActor
final class NotificationService: NSObject, UNUserNotificationCenterDelegate {
    private let center: UNUserNotificationCenter?
    private let language: () -> AppLanguage
    private let enqueue: (UNNotificationRequest) -> Void
    var onOpenProvider: ((UsageProvider) -> Void)?

    init(language: @escaping () -> AppLanguage = { .english }, enqueue: ((UNNotificationRequest) -> Void)? = nil) {
        self.language = language
        if let enqueue {
            self.center = nil
            self.enqueue = enqueue
        } else {
            let center = UNUserNotificationCenter.current()
            self.center = center
            self.enqueue = { request in Task { try? await center.add(request) } }
        }
        super.init()
        center?.delegate = self
    }

    func requestAuthorization() {
        guard let center else { return }
        Task {
            _ = try? await center.requestAuthorization(options: [.alert, .sound])
        }
    }

    func send(_ events: [ThresholdEvent]) {
        let localizer = AppLocalizer(language: language())
        for event in events {
            let content = UNMutableNotificationContent()
            content.title = localizer.text("Usage reached %@", localizer.percentage(event.level == .critical ? 0.9 : 0.7))
            content.body = Self.notificationBody(for: event, localizer: localizer)
            content.sound = .default
            content.userInfo = ["provider": event.provider.rawValue]
            let request = UNNotificationRequest(
                identifier: "\(event.provider.rawValue)-\(event.metricLabel)-\(event.level.rawValue)-\(UUID().uuidString)",
                content: content,
                trigger: nil
            )
            enqueue(request)
        }
    }

    static func notificationBody(for event: ThresholdEvent, localizer: AppLocalizer = AppLocalizer(language: .english)) -> String {
        localizer.text("%@ · %@ is at %@.", event.provider.displayName,
                       ProviderDetailText.metricLabel(event.metricLabel, localizer: localizer),
                       localizer.percentage(event.usedFraction))
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
