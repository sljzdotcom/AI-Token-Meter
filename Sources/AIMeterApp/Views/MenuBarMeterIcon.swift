import SwiftUI

struct MenuBarMeterGeometry: Equatable {
    static let startDegrees = 135.0
    static let sweepDegrees = 270.0
    static let trackTrim = sweepDegrees / 360.0

    let normalizedFraction: Double?

    init(fraction: Double?) {
        guard let fraction, fraction.isFinite else {
            normalizedFraction = nil
            return
        }
        normalizedFraction = min(max(fraction, 0), 1)
    }

    var progressTrim: Double? {
        normalizedFraction.map { $0 * Self.trackTrim }
    }

    var pointerDegrees: Double {
        guard let normalizedFraction else { return 0 }
        return Self.startDegrees + normalizedFraction * Self.sweepDegrees
    }
}

@MainActor
enum MenuBarMeterTemplateImage {
    static let pointSize = NSSize(width: 18, height: 18)

    static func make(fraction: Double?, scale: CGFloat = 2) -> NSImage {
        let renderer = ImageRenderer(content:
            MenuBarMeterGlyph(fraction: fraction)
                .frame(width: pointSize.width, height: pointSize.height)
                .environment(\.colorScheme, .light)
        )
        renderer.scale = max(1, scale)

        let image: NSImage
        if let cgImage = renderer.cgImage {
            image = NSImage(cgImage: cgImage, size: pointSize)
        } else {
            image = NSImage(size: pointSize)
        }
        image.isTemplate = true
        return image
    }
}

struct MenuBarMeterIcon: View {
    let fraction: Double?

    var body: some View {
        Image(nsImage: MenuBarMeterTemplateImage.make(fraction: fraction))
            .renderingMode(.template)
            .resizable()
            .frame(width: 18, height: 18)
            .accessibilityHidden(true)
    }
}

private struct MenuBarMeterGlyph: View {
    let fraction: Double?

    private let tickFractions = [0.0, 0.5, 1.0]

    var body: some View {
        let geometry = MenuBarMeterGeometry(fraction: fraction)

        ZStack {
            Circle()
                .trim(from: 0, to: MenuBarMeterGeometry.trackTrim)
                .stroke(
                    Color.black.opacity(0.30),
                    style: StrokeStyle(lineWidth: 1.45, lineCap: .round)
                )
                .rotationEffect(.degrees(MenuBarMeterGeometry.startDegrees))

            if let progressTrim = geometry.progressTrim, progressTrim > 0 {
                Circle()
                    .trim(from: 0, to: progressTrim)
                    .stroke(
                        Color.black,
                        style: StrokeStyle(lineWidth: 1.55, lineCap: .round)
                    )
                    .rotationEffect(.degrees(MenuBarMeterGeometry.startDegrees))
            }

            dialTicks

            Capsule()
                .fill(Color.black)
                .frame(width: 5.7, height: 1.35)
                .offset(x: 2.55)
                .rotationEffect(.degrees(geometry.pointerDegrees))

            Circle()
                .fill(Color.black)
                .frame(width: 2.5, height: 2.5)
        }
        .padding(1.4)
        .frame(width: 18, height: 18)
    }

    private var dialTicks: some View {
        ForEach(tickFractions, id: \.self) { tickFraction in
            Capsule()
                .fill(Color.black)
                .frame(width: 1.15, height: 2.3)
                .offset(y: -6.1)
                .rotationEffect(.degrees(
                    MenuBarMeterGeometry.startDegrees
                        + tickFraction * MenuBarMeterGeometry.sweepDegrees
                        + 90
                ))
        }
    }
}
