import AppKit
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("Menu bar Quantum Dial")
struct MenuBarMeterIconTests {
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
