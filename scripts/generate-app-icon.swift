#!/usr/bin/env swift

import AppKit
import Foundation

enum IconGenerationError: Error, CustomStringConvertible {
    case missingDestination
    case bitmapCreationFailed(Int)
    case pngEncodingFailed(Int)

    var description: String {
        switch self {
        case .missingDestination:
            "Usage: generate-app-icon.swift <AppIcon.iconset> [AppIcon.icns]"
        case .bitmapCreationFailed(let size):
            "Could not create the \(size)-pixel icon canvas."
        case .pngEncodingFailed(let size):
            "Could not encode the \(size)-pixel icon as PNG."
        }
    }
}

func appendBigEndian(_ value: UInt32, to data: inout Data) {
    var bigEndianValue = value.bigEndian
    withUnsafeBytes(of: &bigEndianValue) { bytes in
        data.append(contentsOf: bytes)
    }
}

func writeICNS(iconset: URL, destination: URL) throws {
    let elements: [(type: String, filename: String)] = [
        ("icp4", "icon_16x16.png"),
        ("icp5", "icon_32x32.png"),
        ("icp6", "icon_32x32@2x.png"),
        ("ic07", "icon_128x128.png"),
        ("ic08", "icon_256x256.png"),
        ("ic09", "icon_512x512.png"),
        ("ic10", "icon_512x512@2x.png"),
    ]

    var payload = Data()
    for element in elements {
        let png = try Data(contentsOf: iconset.appending(path: element.filename))
        payload.append(Data(element.type.utf8))
        appendBigEndian(UInt32(png.count + 8), to: &payload)
        payload.append(png)
    }

    var container = Data("icns".utf8)
    appendBigEndian(UInt32(payload.count + 8), to: &container)
    container.append(payload)
    try container.write(to: destination, options: .atomic)
}

struct IconVariant {
    let filename: String
    let pixels: Int
}

let variants = [
    IconVariant(filename: "icon_16x16.png", pixels: 16),
    IconVariant(filename: "icon_16x16@2x.png", pixels: 32),
    IconVariant(filename: "icon_32x32.png", pixels: 32),
    IconVariant(filename: "icon_32x32@2x.png", pixels: 64),
    IconVariant(filename: "icon_128x128.png", pixels: 128),
    IconVariant(filename: "icon_128x128@2x.png", pixels: 256),
    IconVariant(filename: "icon_256x256.png", pixels: 256),
    IconVariant(filename: "icon_256x256@2x.png", pixels: 512),
    IconVariant(filename: "icon_512x512.png", pixels: 512),
    IconVariant(filename: "icon_512x512@2x.png", pixels: 1_024),
]

func interpolatedColor(from start: NSColor, to end: NSColor, amount: CGFloat) -> NSColor {
    let start = start.usingColorSpace(.deviceRGB) ?? start
    let end = end.usingColorSpace(.deviceRGB) ?? end
    return NSColor(
        red: start.redComponent + (end.redComponent - start.redComponent) * amount,
        green: start.greenComponent + (end.greenComponent - start.greenComponent) * amount,
        blue: start.blueComponent + (end.blueComponent - start.blueComponent) * amount,
        alpha: 1
    )
}

func renderMeterIcon(pixels: Int, destination: URL) throws {
    guard let bitmap = NSBitmapImageRep(
        bitmapDataPlanes: nil,
        pixelsWide: pixels,
        pixelsHigh: pixels,
        bitsPerSample: 8,
        samplesPerPixel: 4,
        hasAlpha: true,
        isPlanar: false,
        colorSpaceName: .deviceRGB,
        bytesPerRow: 0,
        bitsPerPixel: 0
    ), let context = NSGraphicsContext(bitmapImageRep: bitmap) else {
        throw IconGenerationError.bitmapCreationFailed(pixels)
    }

    let size = CGFloat(pixels)
    let bounds = NSRect(x: 0, y: 0, width: size, height: size)
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = context

    NSColor.clear.setFill()
    bounds.fill()

    let plateInset = size * 0.035
    let plate = NSBezierPath(
        roundedRect: bounds.insetBy(dx: plateInset, dy: plateInset),
        xRadius: size * 0.23,
        yRadius: size * 0.23
    )
    NSGradient(colors: [
        NSColor(red: 0.08, green: 0.10, blue: 0.24, alpha: 1),
        NSColor(red: 0.03, green: 0.04, blue: 0.10, alpha: 1),
    ])?.draw(in: plate, angle: -55)

    let glow = NSBezierPath(ovalIn: bounds.insetBy(dx: size * 0.17, dy: size * 0.17))
    NSColor(red: 0.35, green: 0.40, blue: 1.0, alpha: 0.10).setFill()
    glow.fill()

    let center = NSPoint(x: size * 0.5, y: size * 0.49)
    let radius = size * 0.29
    let lineWidth = max(size * 0.072, 1.2)

    let track = NSBezierPath()
    track.appendArc(withCenter: center, radius: radius, startAngle: 135, endAngle: 405)
    track.lineWidth = lineWidth
    track.lineCapStyle = .round
    NSColor.white.withAlphaComponent(0.13).setStroke()
    track.stroke()

    let mint = NSColor(red: 0.33, green: 0.93, blue: 0.78, alpha: 1)
    let violet = NSColor(red: 0.47, green: 0.41, blue: 1.0, alpha: 1)
    let segmentCount = 28
    let startAngle: CGFloat = 135
    let totalAngle: CGFloat = 218
    let gap: CGFloat = 1.8
    for index in 0..<segmentCount {
        let progress = CGFloat(index) / CGFloat(segmentCount - 1)
        let segmentStart = startAngle + totalAngle * CGFloat(index) / CGFloat(segmentCount)
        let segmentEnd = startAngle + totalAngle * CGFloat(index + 1) / CGFloat(segmentCount) - gap
        let segment = NSBezierPath()
        segment.appendArc(
            withCenter: center,
            radius: radius,
            startAngle: segmentStart,
            endAngle: segmentEnd
        )
        segment.lineWidth = lineWidth
        segment.lineCapStyle = .round
        interpolatedColor(from: mint, to: violet, amount: progress).setStroke()
        segment.stroke()
    }

    let needleAngle = CGFloat(43) * .pi / 180
    let needleLength = radius * 0.76
    let needleEnd = NSPoint(
        x: center.x + cos(needleAngle) * needleLength,
        y: center.y + sin(needleAngle) * needleLength
    )
    let needle = NSBezierPath()
    needle.move(to: center)
    needle.line(to: needleEnd)
    needle.lineWidth = max(size * 0.055, 1.25)
    needle.lineCapStyle = .round
    NSColor(red: 0.96, green: 0.98, blue: 1, alpha: 1).setStroke()
    needle.stroke()

    let hubSize = max(size * 0.105, 2)
    let hub = NSBezierPath(ovalIn: NSRect(
        x: center.x - hubSize / 2,
        y: center.y - hubSize / 2,
        width: hubSize,
        height: hubSize
    ))
    NSColor(red: 0.96, green: 0.98, blue: 1, alpha: 1).setFill()
    hub.fill()

    context.flushGraphics()
    NSGraphicsContext.restoreGraphicsState()

    guard let data = bitmap.representation(using: .png, properties: [:]) else {
        throw IconGenerationError.pngEncodingFailed(pixels)
    }
    try data.write(to: destination, options: .atomic)
}

do {
    guard (2...3).contains(CommandLine.arguments.count) else {
        throw IconGenerationError.missingDestination
    }
    let destination = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
    let fileManager = FileManager.default
    if fileManager.fileExists(atPath: destination.path) {
        try fileManager.removeItem(at: destination)
    }
    try fileManager.createDirectory(at: destination, withIntermediateDirectories: true)

    for variant in variants {
        try renderMeterIcon(
            pixels: variant.pixels,
            destination: destination.appending(path: variant.filename)
        )
    }
    if CommandLine.arguments.count == 3 {
        try writeICNS(
            iconset: destination,
            destination: URL(fileURLWithPath: CommandLine.arguments[2])
        )
    }
} catch {
    FileHandle.standardError.write(Data("App icon generation failed: \(error)\n".utf8))
    exit(1)
}
