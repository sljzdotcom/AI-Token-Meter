import AppKit
import SwiftUI
import Testing

/// Read the actual SwiftUI/AppKit accessibility tree, including native controls.
/// Never derive expected text from a localizer or from the view's input model.
@MainActor
func hostedAccessibilityStrings(_ root: NSObject) -> [String] {
    hostedAccessibilityObjects(root).flatMap { object in
        ["accessibilityLabel", "accessibilityTitle", "accessibilityValue", "accessibilityHelp"]
            .compactMap { hostedAccessibilityAttribute(object, $0) as? String }
    }
}

@MainActor
func hostedAccessibilityAttribute(_ object: NSObject, _ name: String) -> Any? {
    guard object.responds(to: NSSelectorFromString(name)) else { return nil }
    return object.value(forKey: name)
}

@MainActor
func hostedAccessibilityObjects(_ root: NSObject) -> [NSObject] {
    var visited: Set<ObjectIdentifier> = []
    func walk(_ object: NSObject) -> [NSObject] {
        guard visited.insert(ObjectIdentifier(object)).inserted else { return [] }
        let children = (hostedAccessibilityAttribute(object, "accessibilityChildren") as? [NSObject] ?? [])
            + ((object as? NSView)?.subviews ?? [])
        return [object] + children.flatMap(walk)
    }
    return walk(root)
}

@MainActor
func assertHostedLabelFits(_ expected: String, in host: NSView) throws {
    let window = try #require(host.window)
    let visibleFrame = window.convertToScreen(host.convert(host.bounds, to: nil))
    let frames = hostedAccessibilityObjects(host).compactMap { object -> NSRect? in
        let values = ["accessibilityLabel", "accessibilityTitle", "accessibilityValue"]
            .compactMap { hostedAccessibilityAttribute(object, $0) as? String }
        guard values.contains(where: { $0.contains(expected) }) else { return nil }
        return (hostedAccessibilityAttribute(object, "accessibilityFrame") as? NSValue)?.rectValue
    }
    #expect(frames.contains { !$0.isEmpty && visibleFrame.insetBy(dx: -1, dy: -1).contains($0) },
        "Label \(expected) must have a visible, unclipped frame: \(frames) inside \(visibleFrame)")
}

@MainActor
func assertHostedRendering(_ host: NSView) throws {
    let bitmap = try #require(NSBitmapImageRep(bitmapDataPlanes: nil,
        pixelsWide: Int(host.bounds.width * 2), pixelsHigh: Int(host.bounds.height * 2),
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0))
    bitmap.size = host.bounds.size
    host.cacheDisplay(in: host.bounds, to: bitmap)
    let image = try #require(bitmap.cgImage)
    #expect(image.width == Int(host.bounds.width * 2))
    #expect(image.height == Int(host.bounds.height * 2))
    let pixels = try #require(bitmap.bitmapData)
    let shades = Set(stride(from: 0, to: bitmap.bytesPerRow * bitmap.pixelsHigh, by: 32).map { pixels[$0] })
    #expect(shades.count > 4, "Hosted content must render more than an empty background")
}

@MainActor
func localizationTestWindow<V: View>(_ host: NSHostingView<V>, width: CGFloat = 800, height: CGFloat = 1400) -> NSWindow {
    NSApplication.shared.accessibilitySetValue(true, forAttribute: NSAccessibility.Attribute(rawValue: "AXEnhancedUserInterface"))
    let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: width, height: height),
        styleMask: .borderless, backing: .buffered, defer: false)
    window.isReleasedWhenClosed = false
    window.contentView = host
    window.orderFrontRegardless()
    return window
}

@MainActor
func settleLocalizationHost(_ host: NSView) async throws {
    host.window?.layoutIfNeeded()
    host.layoutSubtreeIfNeeded()
    try await Task.sleep(for: .milliseconds(80))
    host.layoutSubtreeIfNeeded()
}
