import AIMeterCore
import AppKit
import SwiftUI

enum FloatingStripBackgroundAsset {
    static let filename = "floating-strip-deep-sea"
    @MainActor static let defaultImage = load()

    static func resourceURL(in bundle: Bundle = .main) -> URL? {
        AppResourceLocator.url(
            forResource: filename,
            withExtension: "png",
            subdirectory: "Backgrounds",
            primaryBundle: bundle
        )
    }

    static func load(in bundle: Bundle = .main) -> NSImage? {
        resourceURL(in: bundle).flatMap(NSImage.init(contentsOf:))
    }
}

enum FloatingStripBackgroundPresentation {
    static let scrimOpacity = 0.38
    static let contentScale: CGFloat = 1.22

    static func scale(for edge: FloatingStripEdge) -> CGSize {
        CGSize(
            width: edge == .left ? -contentScale : contentScale,
            height: contentScale
        )
    }
}

enum FloatingStripMaterialRendering: Equatable {
    case deepSea
    case nativeLiquidGlass
    case fallbackLiquidGlass
    case opaqueLiquidGlass
}

enum FloatingStripMaterialPolicy {
    static func rendering(
        for appearance: FloatingStripAppearance,
        nativeLiquidGlassAvailable: Bool,
        reduceTransparency: Bool
    ) -> FloatingStripMaterialRendering {
        guard appearance == .liquidGlass else { return .deepSea }
        if reduceTransparency { return .opaqueLiquidGlass }
        return nativeLiquidGlassAvailable ? .nativeLiquidGlass : .fallbackLiquidGlass
    }
}

private struct FloatingStripMaterialSurface<SurfaceShape: Shape>: View {
    let appearance: FloatingStripAppearance
    let shape: SurfaceShape
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @Environment(\.accessibilityDifferentiateWithoutColor) private var differentiateWithoutColor
    @Environment(\.colorSchemeContrast) private var colorSchemeContrast

    var body: some View {
        surface
            .overlay {
                if appearance == .liquidGlass {
                    shape.stroke(
                        Color.white.opacity(strongBoundary ? 0.28 : 0.11),
                        lineWidth: strongBoundary ? 1.2 : 0.75
                    )
                }
            }
            .clipShape(shape)
    }

    @ViewBuilder
    private var surface: some View {
        switch FloatingStripMaterialPolicy.rendering(
            for: appearance,
            nativeLiquidGlassAvailable: nativeLiquidGlassAvailable,
            reduceTransparency: reduceTransparency
        ) {
        case .deepSea:
            shape.fill(AIMeterVisualTheme.floatingGlass)
        case .nativeLiquidGlass:
            nativeLiquidGlass
        case .fallbackLiquidGlass:
            shape
                .fill(.ultraThinMaterial)
                .overlay { shape.fill(AIMeterVisualTheme.floatingLiquidGlassSmoke) }
        case .opaqueLiquidGlass:
            shape.fill(AIMeterVisualTheme.floatingLiquidGlassOpaque)
        }
    }

    @ViewBuilder
    private var nativeLiquidGlass: some View {
        if #available(macOS 26.0, *) {
            ZStack {
                shape.fill(AIMeterVisualTheme.floatingLiquidGlassSmoke)
                Color.clear
                    .glassEffect(
                        .regular.tint(AIMeterVisualTheme.floatingLiquidGlassTint),
                        in: shape
                    )
            }
        } else {
            shape
                .fill(.ultraThinMaterial)
                .overlay { shape.fill(AIMeterVisualTheme.floatingLiquidGlassSmoke) }
        }
    }

    private var nativeLiquidGlassAvailable: Bool {
        if #available(macOS 26.0, *) { return true }
        return false
    }

    private var strongBoundary: Bool {
        differentiateWithoutColor || colorSchemeContrast == .increased
    }
}

struct FloatingStripSurface: View {
    let edge: FloatingStripEdge
    var density: FloatingStripDensity
    var providerCount: Int
    var appearance: FloatingStripAppearance
    private let backgroundImage: NSImage?

    init(
        edge: FloatingStripEdge,
        density: FloatingStripDensity = .comfortable,
        providerCount: Int = 3,
        appearance: FloatingStripAppearance = .deepSea,
        backgroundImage: NSImage? = FloatingStripBackgroundAsset.defaultImage
    ) {
        self.edge = edge
        self.density = density
        self.providerCount = providerCount
        self.appearance = appearance
        self.backgroundImage = backgroundImage
    }

    var body: some View {
        let shape = FloatingStripShape(edge: edge, density: density, providerCount: providerCount)
        Group {
            if appearance == .deepSea {
                ZStack {
                    FloatingStripMaterialSurface(appearance: appearance, shape: shape)

                    if let backgroundImage {
                        let scale = FloatingStripBackgroundPresentation.scale(for: edge)
                        Image(nsImage: backgroundImage)
                            .resizable()
                            .scaledToFill()
                            .scaleEffect(x: scale.width, y: scale.height, anchor: .center)
                            .overlay {
                                Color.black.opacity(density == .comfortable ? FloatingStripBackgroundPresentation.scrimOpacity : 0.46)
                            }
                            .accessibilityHidden(true)
                    }
                }
            } else {
                FloatingStripMaterialSurface(appearance: appearance, shape: shape)
            }
        }
        .clipShape(shape)
    }
}

struct FloatingStripFoldedSurface: View {
    let edge: FloatingStripEdge
    var appearance: FloatingStripAppearance
    private let backgroundImage: NSImage?

    init(
        edge: FloatingStripEdge,
        appearance: FloatingStripAppearance = .deepSea,
        backgroundImage: NSImage? = FloatingStripBackgroundAsset.defaultImage
    ) {
        self.edge = edge
        self.appearance = appearance
        self.backgroundImage = backgroundImage
    }

    var body: some View {
        let shape = FloatingStripFoldedShape(edge: edge)
        Group {
            if appearance == .deepSea {
                ZStack {
                    shape.fill(Color(red: 0.015, green: 0.04, blue: 0.085))
                    if let backgroundImage {
                        Image(nsImage: backgroundImage)
                            .resizable()
                            .scaledToFill()
                            .scaleEffect(x: edge == .left ? -1 : 1, y: 1)
                            .overlay(Color.black.opacity(0.46))
                            .accessibilityHidden(true)
                    }
                }
            } else {
                FloatingStripMaterialSurface(appearance: appearance, shape: shape)
            }
        }
        .clipShape(shape)
    }
}
