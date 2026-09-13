import AIMeterCore
import Testing
@testable import AIMeterApp

@Suite("Floating strip material policy")
struct FloatingStripMaterialPolicyTests {
    @Test func deepSeaNeverSwitchesToGlassRendering() {
        for nativeAvailable in [false, true] {
            for reduceTransparency in [false, true] {
                #expect(FloatingStripMaterialPolicy.rendering(
                    for: .deepSea,
                    nativeLiquidGlassAvailable: nativeAvailable,
                    reduceTransparency: reduceTransparency
                ) == .deepSea)
            }
        }
    }

    @Test func liquidGlassUsesNativeFallbackAndOpaqueAccessibilityPaths() {
        #expect(FloatingStripMaterialPolicy.rendering(
            for: .liquidGlass,
            nativeLiquidGlassAvailable: true,
            reduceTransparency: false
        ) == .nativeLiquidGlass)
        #expect(FloatingStripMaterialPolicy.rendering(
            for: .liquidGlass,
            nativeLiquidGlassAvailable: false,
            reduceTransparency: false
        ) == .fallbackLiquidGlass)
        #expect(FloatingStripMaterialPolicy.rendering(
            for: .liquidGlass,
            nativeLiquidGlassAvailable: true,
            reduceTransparency: true
        ) == .opaqueLiquidGlass)
        #expect(FloatingStripMaterialPolicy.rendering(
            for: .liquidGlass,
            nativeLiquidGlassAvailable: false,
            reduceTransparency: true
        ) == .opaqueLiquidGlass)
    }
}
