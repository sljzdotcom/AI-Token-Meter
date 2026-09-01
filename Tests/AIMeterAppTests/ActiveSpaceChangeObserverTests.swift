import Foundation
import Testing
@testable import AIMeterApp

@Suite("Active Space change observer")
struct ActiveSpaceChangeObserverTests {
    @Test("Every active Space notification is delivered once")
    @MainActor
    func delivery() {
        let center = NotificationCenter()
        let name = Notification.Name("ActiveSpaceChangeObserverTests.delivery")
        var deliveries = 0
        let observer = ActiveSpaceChangeObserver(center: center, name: name) {
            deliveries += 1
        }

        center.post(name: name, object: nil)
        center.post(name: name, object: nil)

        #expect(deliveries == 2)
        observer.invalidate()
    }

    @Test("Invalidation stops future delivery and is idempotent")
    @MainActor
    func invalidation() {
        let center = NotificationCenter()
        let name = Notification.Name("ActiveSpaceChangeObserverTests.invalidation")
        var deliveries = 0
        let observer = ActiveSpaceChangeObserver(center: center, name: name) {
            deliveries += 1
        }

        observer.invalidate()
        observer.invalidate()
        center.post(name: name, object: nil)

        #expect(deliveries == 0)
    }
}
