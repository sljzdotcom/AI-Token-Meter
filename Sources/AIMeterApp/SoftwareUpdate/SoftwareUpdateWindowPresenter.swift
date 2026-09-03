import AppKit

@MainActor
protocol SoftwareUpdatePresentableWindow: AnyObject {
    var softwareUpdateWindowIdentifier: String? { get }
    func hideForSoftwareUpdate()
}

extension NSWindow: SoftwareUpdatePresentableWindow {
    var softwareUpdateWindowIdentifier: String? {
        identifier?.rawValue
    }

    func hideForSoftwareUpdate() {
        orderOut(nil)
    }
}

@MainActor
struct SoftwareUpdateWindowPresenter {
    private static let settingsWindowIdentifier = "com_apple_SwiftUI_Settings_window"

    private let windows: @MainActor () -> [any SoftwareUpdatePresentableWindow]
    private let activate: @MainActor () -> Void

    init(
        windows: @escaping @MainActor () -> [any SoftwareUpdatePresentableWindow] = {
            NSApp.windows
        },
        activate: @escaping @MainActor () -> Void = {
            NSApp.activate(ignoringOtherApps: true)
        }
    ) {
        self.windows = windows
        self.activate = activate
    }

    func prepareForInstallation() {
        for window in windows()
        where window.softwareUpdateWindowIdentifier == Self.settingsWindowIdentifier {
            window.hideForSoftwareUpdate()
        }
        activate()
    }
}
