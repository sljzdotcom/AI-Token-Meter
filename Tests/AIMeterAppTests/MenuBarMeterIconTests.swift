import AIMeterCore
import AppKit
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("Menu bar Quantum Dial")
struct MenuBarMeterIconTests {
    // These read the real hosted accessibility tree: untranslated Core labels or
    // a missing locale injection must fail at the output VoiceOver consumes.
    @Test("Existing menu and floating meter accessibility follow language changes")
    @MainActor
    func hostedAccessibilityFollowsLanguage() async throws {
        NSApplication.shared.accessibilitySetValue(true, forAttribute: NSAccessibility.Attribute(rawValue: "AXEnhancedUserInterface"))
        let name = "MenuBarAX.\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        let model = AppModel(defaults: defaults, widgetSnapshotPublisher: nil, isDemoMode: false,
            refreshOperation: { [.init(provider: .codex, primaryMetric: .init(label: "Weekly limit", current: 73, limit: 100, unit: .percent))] })
        await model.refresh()
        let session = FloatingDetailSession()
        defer { session.shutdown() }
        session.present(.codex, autoHideAfter: .seconds(300))
        let display = FloatingStripDisplayState(resolvedEdge: .left, normalizedCenterY: 0.37)
        let host = NSHostingView(rootView: AppLanguageRoot(model: model) {
            VStack {
                MenuBarLabel(model: model)
                FloatingStripView(model: model, session: session, displayState: display,
                                  onProviderTap: { _ in }, onAccessibilityMove: { _ in })
                    .frame(width: 108, height: 356)
            }
        })
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 400, height: 450),
                              styleMask: .borderless, backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = host
        window.orderFrontRegardless()
        defer { window.close() }
        for (language, menu, ring, position, opened, closed) in [
            (AppLanguage.english, "AI Token Meter, highest usage 73 percent", "OpenAI Codex, 73%, Weekly limit, Warning", "Left edge, vertical position 37 percent", "Detail open", "Detail closed"),
            (.simplifiedChinese, "AI Token Meter，最高用量百分之 73", "OpenAI Codex，73%，每周限额，警告", "左侧边缘，垂直位置百分之 37", "详情已打开", "详情已关闭"),
            (.traditionalChinese, "AI Token Meter，最高用量百分之 73", "OpenAI Codex，73%，每週限額，警告", "左側邊緣，垂直位置百分之 37", "詳細資料已開啟", "詳細資料已關閉"),
        ] {
            model.setAppLanguage(language)
            var output: [String] = []
            for _ in 0..<40 {
                window.layoutIfNeeded()
                host.layoutSubtreeIfNeeded()
                await Task.yield()
                output = accessibilityOutput(host)
                if output.contains(menu), output.contains(ring),
                   output.contains(where: { $0.hasSuffix(position) }),
                   output.contains(opened), output.contains(closed) {
                    break
                }
                try await Task.sleep(for: .milliseconds(25))
            }
            #expect(output.contains(menu), "Missing menu output in \(output)")
            #expect(output.contains(ring), "Missing ring output in \(output)")
            #expect(output.contains(where: { $0.hasSuffix(position) }), "Missing position output in \(output)")
            #expect(output.contains(opened), "Missing open state in \(output)")
            #expect(output.contains(closed), "Missing closed state in \(output)")
        }
    }

    @MainActor
    private func accessibilityOutput(_ root: NSObject) -> [String] {
        var visited: Set<ObjectIdentifier> = []
        func walk(_ object: NSObject) -> [String] {
            guard visited.insert(ObjectIdentifier(object)).inserted else { return [] }
            func attribute(_ name: String) -> Any? {
                let selector = NSSelectorFromString(name)
                guard object.responds(to: selector) else { return nil }
                return object.perform(selector)?.takeUnretainedValue()
            }
            let own = ["accessibilityLabel", "accessibilityTitle", "accessibilityValue", "accessibilityHelp"]
                .compactMap { attribute($0) as? String }
            let children = (attribute("accessibilityChildren") as? [NSObject] ?? []) + ((object as? NSView)?.subviews ?? [])
            return own + children.flatMap(walk)
        }
        return walk(root)
    }

    @Test("Hosted quota detail and reset credit accessibility use complete translated phrases")
    @MainActor
    func hostedDetailAccessibility() async throws {
        NSApplication.shared.accessibilitySetValue(true, forAttribute: NSAccessibility.Attribute(rawValue: "AXEnhancedUserInterface"))
        let snapshot = UsageSnapshot(provider: .claude,
            primaryMetric: .init(label: "Current session", current: 23, limit: 100, unit: .percent))
        let credits = CodexResetCreditsSummary(availableCount: 1,
            credits: [.init(title: "Usage reset", expiresAt: nil)], hasCompleteDetails: true)
        for (language, quota, credit) in [
            (AppLanguage.english, "Official quota, Current session, 23 percent used, Reset time unavailable", "Usage reset, Date unavailable, Expiration unavailable"),
            (.simplifiedChinese, "官方额度，当前会话，已用百分之 23，重置时间不可用", "用量重置，日期不可用，到期时间不可用"),
            (.traditionalChinese, "官方額度，目前工作階段，已用百分之 23，無法取得重設時間", "用量重設，無法取得日期，無法取得到期時間"),
        ] {
            let host = NSHostingView(rootView: VStack {
                ClaudeDetailView(snapshot: snapshot, onSetup: {}, onOpenServicesSettings: {})
                CodexResetCreditsView(summary: credits, mode: .detail)
            }.environment(\.locale, language.locale))
            let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 650, height: 650),
                                  styleMask: .borderless, backing: .buffered, defer: false)
            window.isReleasedWhenClosed = false
            window.contentView = host
            defer { window.close() }
            host.layoutSubtreeIfNeeded()
            try await Task.sleep(for: .milliseconds(100))
            let output = accessibilityOutput(host)
            #expect(output.contains(quota), "Missing quota label in \(output)")
            #expect(output.contains(credit), "Missing credit label in \(output)")
        }
    }

    @Test("Geometry maps usage to the approved 270 degree sweep")
    func mapsUsageToGeometry() throws {
        let zero = MenuBarMeterGeometry(fraction: 0)
        let twentyThree = MenuBarMeterGeometry(fraction: 0.23)
        let seventy = MenuBarMeterGeometry(fraction: 0.70)
        let ninety = MenuBarMeterGeometry(fraction: 0.90)
        let full = MenuBarMeterGeometry(fraction: 1)

        #expect(zero.progressTrim == 0)
        #expect(abs(try #require(twentyThree.progressTrim) - 0.1725) < 0.0001)
        #expect(abs(twentyThree.pointerDegrees - 197.1) < 0.0001)
        #expect(abs(try #require(seventy.progressTrim) - 0.525) < 0.0001)
        #expect(abs(try #require(ninety.progressTrim) - 0.675) < 0.0001)
        #expect(full.progressTrim == 0.75)
        #expect(full.pointerDegrees == 405)
    }

    @Test("Geometry clamps bounds and gives unavailable a neutral pointer")
    func normalizesGeometry() {
        #expect(MenuBarMeterGeometry(fraction: -1).progressTrim == 0)
        #expect(MenuBarMeterGeometry(fraction: 2).progressTrim == 0.75)
        #expect(MenuBarMeterGeometry(fraction: nil).progressTrim == nil)
        #expect(MenuBarMeterGeometry(fraction: nil).pointerDegrees == 0)
        #expect(MenuBarMeterGeometry(fraction: .nan).progressTrim == nil)
        #expect(MenuBarMeterGeometry(fraction: .infinity).progressTrim == nil)
    }

    @Test("The icon stays inside a transparent 18 point canvas in both appearances")
    @MainActor
    func rendersInsideMenuBarCanvas() throws {
        let light = try renderIcon(colorScheme: .light)
        let dark = try renderIcon(colorScheme: .dark)

        for image in [light, dark] {
            #expect(image.pixelsWide == 36)
            #expect(image.pixelsHigh == 36)
            #expect(try alpha(atX: 0, y: 0, in: image) == 0)
            #expect(try alpha(atX: 35, y: 0, in: image) == 0)
            #expect(try alpha(atX: 0, y: 35, in: image) == 0)
            #expect(try alpha(atX: 35, y: 35, in: image) == 0)
            #expect(try alpha(atX: 18, y: 18, in: image) > 0)

            let visiblePixels = try visiblePixelCount(in: image)
            #expect(visiblePixels > 80)
            #expect(visiblePixels < 700)
        }

        let lightCenter = try color(atX: 18, y: 18, in: light)
        let darkCenter = try color(atX: 18, y: 18, in: dark)
        #expect(lightCenter.brightnessComponent < darkCenter.brightnessComponent)
    }

    @Test("The menu bar image is a system-tinted template")
    @MainActor
    func producesTemplateImage() throws {
        let image = MenuBarMeterTemplateImage.make(fraction: 0.37, scale: 2)

        #expect(image.isTemplate)
        #expect(image.size == NSSize(width: 18, height: 18))

        let data = try #require(image.tiffRepresentation)
        let representation = try #require(NSBitmapImageRep(data: data))
        #expect(representation.pixelsWide == 36)
        #expect(representation.pixelsHigh == 36)
        #expect(try visiblePixelCount(in: representation) > 80)
    }

    @MainActor
    private func renderIcon(colorScheme: ColorScheme) throws -> NSBitmapImageRep {
        let renderer = ImageRenderer(content:
            MenuBarMeterIcon(fraction: 0.23)
                .environment(\.colorScheme, colorScheme)
                .frame(width: 18, height: 18)
        )
        renderer.scale = 2
        return NSBitmapImageRep(cgImage: try #require(renderer.cgImage))
    }

    private func alpha(atX x: Int, y: Int, in image: NSBitmapImageRep) throws -> CGFloat {
        try color(atX: x, y: y, in: image).alphaComponent
    }

    private func color(atX x: Int, y: Int, in image: NSBitmapImageRep) throws -> NSColor {
        try #require(
            image.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB)
        )
    }

    private func visiblePixelCount(in image: NSBitmapImageRep) throws -> Int {
        var count = 0
        for y in 0..<image.pixelsHigh {
            for x in 0..<image.pixelsWide where try alpha(atX: x, y: y, in: image) > 0.01 {
                count += 1
            }
        }
        return count
    }
}
