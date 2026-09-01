import AIMeterCore
import SwiftUI
import Testing
@testable import AIMeterApp

@Suite("AI Meter typography")
struct TypographyTests {
    @Test("System is always available while custom choices use registered families")
    func availability() {
        let catalog = DisplayFontCatalog(availableFamilies: ["Antonio"])
        #expect(catalog.isAvailable(.system))
        #expect(catalog.isAvailable(.antonio))
        #expect(!catalog.isAvailable(.dinCondensed))
    }

    @Test("Unavailable saved custom fonts resolve safely to system")
    func fallback() {
        let catalog = DisplayFontCatalog(availableFamilies: [])
        #expect(AIMeterTypography.resolvedFamily(for: .antonio, catalog: catalog) == nil)
        #expect(AIMeterTypography.resolvedFamily(for: .dinCondensed, catalog: catalog) == nil)
    }

    @Test("Available choices resolve to the approved family names")
    func familyNames() {
        let catalog = DisplayFontCatalog(
            availableFamilies: ["Antonio", "DIN Condensed"]
        )
        #expect(AIMeterTypography.resolvedFamily(for: .antonio, catalog: catalog) == "Antonio")
        #expect(AIMeterTypography.resolvedFamily(for: .dinCondensed, catalog: catalog) == "DIN Condensed")
    }

    @Test("Settings always exposes three ordered choices and marks missing fonts")
    func settingsOptions() {
        let catalog = DisplayFontCatalog(availableFamilies: ["Antonio"])
        let options = DisplayFontSettingsPresentation.options(catalog: catalog)

        #expect(options.map(\.choice) == [.system, .antonio, .dinCondensed])
        #expect(options[0].isEnabled)
        #expect(options[1].isEnabled)
        #expect(!options[2].isEnabled)
        #expect(options[2].statusText == "Not installed")
    }

    @Test("Every semantic role preserves native system behavior and macOS custom base size")
    func semanticDescriptors() {
        let catalog = DisplayFontCatalog(
            availableFamilies: ["Antonio", "DIN Condensed"]
        )
        let expectedRoles: [(AIMeterTextStyle, CGFloat, Font.Weight?)] = [
            (.largeTitle, 26, nil),
            (.title, 22, nil),
            (.title2, 17, nil),
            (.title3, 15, nil),
            (.headline, 13, .semibold),
            (.subheadline, 11, nil),
            (.body, 13, nil),
            (.caption, 10, nil),
            (.caption2, 10, nil),
        ]

        for (style, expectedSize, expectedCustomWeight) in expectedRoles {
            let system = AIMeterTypography.resolvedDescriptor(
                token: .semantic(style),
                choice: .system,
                catalog: catalog,
                design: .default,
                weight: nil
            )
            guard case let .systemSemantic(resolvedStyle, _, resolvedWeight) = system else {
                Issue.record("System \(style) did not resolve as a native semantic font")
                continue
            }
            #expect(resolvedStyle == style)
            #expect(resolvedWeight == nil)

            for choice in [DisplayFontChoice.antonio, .dinCondensed] {
                let custom = AIMeterTypography.resolvedDescriptor(
                    token: .semantic(style),
                    choice: choice,
                    catalog: catalog,
                    design: .default,
                    weight: nil
                )
                guard case let .custom(_, size, relativeTo, resolvedWeight) = custom else {
                    Issue.record("\(choice) \(style) did not resolve as a custom font")
                    continue
                }
                #expect(size == expectedSize)
                #expect(relativeTo == style)
                #expect(resolvedWeight == expectedCustomWeight)
            }
        }
    }

    @Test("Explicit caller weight overrides semantic defaults for system and custom fonts")
    func explicitWeight() {
        let catalog = DisplayFontCatalog(availableFamilies: ["Antonio"])

        let system = AIMeterTypography.resolvedDescriptor(
            token: .semantic(.headline),
            choice: .system,
            catalog: catalog,
            design: .rounded,
            weight: .bold
        )
        guard case let .systemSemantic(style, design, weight) = system else {
            Issue.record("System headline did not resolve as a native semantic font")
            return
        }
        #expect(style == .headline)
        #expect(design == .rounded)
        #expect(weight == .bold)

        let custom = AIMeterTypography.resolvedDescriptor(
            token: .semantic(.headline),
            choice: .antonio,
            catalog: catalog,
            design: .rounded,
            weight: .bold
        )
        guard case let .custom(family, size, relativeTo, weight) = custom else {
            Issue.record("Antonio headline did not resolve as a custom font")
            return
        }
        #expect(family == "Antonio")
        #expect(size == 13)
        #expect(relativeTo == .headline)
        #expect(weight == .bold)
    }

    @Test("Unavailable custom fonts preserve native semantic fallback behavior")
    func unavailableDescriptorFallback() {
        let descriptor = AIMeterTypography.resolvedDescriptor(
            token: .semantic(.headline),
            choice: .antonio,
            catalog: DisplayFontCatalog(availableFamilies: []),
            design: .default,
            weight: nil
        )

        guard case let .systemSemantic(style, _, weight) = descriptor else {
            Issue.record("Unavailable Antonio did not resolve to a system semantic font")
            return
        }
        #expect(style == .headline)
        #expect(weight == nil)
    }

    @Test("Fixed fonts preserve size, relative role, and explicit weight")
    func fixedDescriptor() {
        let catalog = DisplayFontCatalog(availableFamilies: ["DIN Condensed"])
        let descriptor = AIMeterTypography.resolvedDescriptor(
            token: .fixed(size: 15, relativeTo: .body),
            choice: .dinCondensed,
            catalog: catalog,
            design: .default,
            weight: .semibold
        )

        guard case let .custom(family, size, relativeTo, weight) = descriptor else {
            Issue.record("Fixed DIN font did not resolve as a custom font")
            return
        }
        #expect(family == "DIN Condensed")
        #expect(size == 15)
        #expect(relativeTo == .body)
        #expect(weight == .semibold)
    }

    @Test("Font surfaces isolate settings and apply one point only to content")
    func fontSurfaceConfigurations() {
        #expect(AIMeterFontScopeConfiguration.settings.choice == .system)
        #expect(AIMeterFontScopeConfiguration.settings.pointOffset == 0)

        let content = AIMeterFontScopeConfiguration.content(.antonio)
        #expect(content.choice == .antonio)
        #expect(content.pointOffset == 1)

        let menuBarLabel = AIMeterFontScopeConfiguration.menuBarLabel(.dinCondensed)
        #expect(menuBarLabel.choice == .dinCondensed)
        #expect(menuBarLabel.pointOffset == 0)
    }

    @Test("Content offset adds exactly one point to system and custom semantic fonts")
    func contentSemanticOffset() {
        let catalog = DisplayFontCatalog(availableFamilies: ["Antonio"])

        let system = AIMeterTypography.resolvedDescriptor(
            token: .semantic(.headline),
            choice: .system,
            catalog: catalog,
            pointOffset: 1,
            design: .default,
            weight: nil
        )
        guard case let .systemFixed(size, _, weight) = system else {
            Issue.record("Offset system headline did not resolve to an exact point size")
            return
        }
        #expect(size == 14)
        #expect(weight == .semibold)

        let custom = AIMeterTypography.resolvedDescriptor(
            token: .semantic(.body),
            choice: .antonio,
            catalog: catalog,
            pointOffset: 1,
            design: .default,
            weight: nil
        )
        guard case let .custom(family, size, relativeTo, _) = custom else {
            Issue.record("Offset Antonio body did not remain a custom font")
            return
        }
        #expect(family == "Antonio")
        #expect(size == 14)
        #expect(relativeTo == .body)
    }

    @Test("Fixed fonts and unavailable custom fallback preserve the content offset")
    func fixedAndFallbackOffset() {
        let unavailable = DisplayFontCatalog(availableFamilies: [])
        let fallback = AIMeterTypography.resolvedDescriptor(
            token: .semantic(.caption),
            choice: .antonio,
            catalog: unavailable,
            pointOffset: 1,
            design: .default,
            weight: nil
        )
        guard case let .systemFixed(fallbackSize, _, _) = fallback else {
            Issue.record("Unavailable custom content font did not retain exact offset")
            return
        }
        #expect(fallbackSize == 11)

        let fixed = AIMeterTypography.resolvedDescriptor(
            token: .fixed(size: 15, relativeTo: .body),
            choice: .system,
            catalog: unavailable,
            pointOffset: 1,
            design: .default,
            weight: .semibold
        )
        guard case let .systemFixed(fixedSize, _, _) = fixed else {
            Issue.record("Fixed system font did not resolve with offset")
            return
        }
        #expect(fixedSize == 16)
    }

    @Test("Point offsets cannot produce a zero or negative font size")
    func pointSizeLowerBound() {
        #expect(
            AIMeterTypography.resolvedPointSize(
                token: .fixed(size: 2, relativeTo: .caption2),
                pointOffset: -10
        ) == 1
        )
    }

    @Test("Settings is system-only while content roots apply the content scale")
    func rootScopeWiring() throws {
        let settings = try viewSource("SettingsView.swift")
        #expect(settings.contains(".aiMeterFontScope(.settings)"))
        #expect(!settings.contains("aiMeterFontPreview"))

        let floating = try viewSource("FloatingStripView.swift")
        #expect(
            floating.components(
                separatedBy: ".aiMeterFontScope(.content(model.displayFontChoice))"
            ).count - 1 == 2
        )

        let menu = try viewSource("MenuBarPanel.swift")
        #expect(menu.contains(".aiMeterFontScope(.content(model.displayFontChoice))"))
        #expect(menu.contains(".aiMeterFontScope(.menuBarLabel(model.displayFontChoice))"))
    }

    private func viewSource(_ fileName: String) throws -> String {
        let projectRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let fileURL = projectRoot
            .appending(path: "Sources/AIMeterApp/Views")
            .appending(path: fileName)
        return try String(contentsOf: fileURL, encoding: .utf8)
    }

    @Test("Each content symbol keeps its declared semantic baseline")
    func symbolFontMappings() throws {
        let expectations = [
            SymbolSourceExpectation(
                fileName: "MenuBarPanel.swift",
                imageExpression: "Image(systemName: \"gauge.with.dots.needle.50percent\")",
                occurrence: 0,
                expectedModifier: ".aiMeterSymbolFont(.body)"
            ),
            SymbolSourceExpectation(
                fileName: "MenuBarPanel.swift",
                imageExpression: "Image(systemName: \"gauge.with.dots.needle.50percent\")",
                occurrence: 1,
                expectedModifier: nil
            ),
            SymbolSourceExpectation(
                fileName: "MenuBarPanel.swift",
                imageExpression: "Image(systemName: \"arrow.clockwise\")",
                expectedModifier: ".aiMeterSymbolFont(.body)"
            ),
            SymbolSourceExpectation(
                fileName: "MenuBarPanel.swift",
                imageExpression: "Image(systemName: \"gearshape\")",
                expectedModifier: ".aiMeterSymbolFont(.body)"
            ),
            SymbolSourceExpectation(
                fileName: "MenuBarPanel.swift",
                imageExpression: "Image(systemName: \"power\")",
                expectedModifier: ".aiMeterSymbolFont(.body)"
            ),
            SymbolSourceExpectation(
                fileName: "CodexDetailView.swift",
                imageExpression: "Image(systemName: symbol)",
                expectedModifier: ".aiMeterSymbolFont(.caption)"
            ),
            SymbolSourceExpectation(
                fileName: "CodexResetCreditsView.swift",
                imageExpression: "Image(systemName: \"info.circle\")",
                expectedModifier: ".aiMeterSymbolFont(.caption2)"
            ),
            SymbolSourceExpectation(
                fileName: "CodexResetCreditsView.swift",
                imageExpression: "Image(systemName: \"arrow.counterclockwise.circle\")",
                expectedModifier: ".aiMeterSymbolFont(.caption2)"
            ),
            SymbolSourceExpectation(
                fileName: "DeepSeekAnalyticsView.swift",
                imageExpression: "Image(systemName: \"arrow.clockwise\")",
                expectedModifier: ".aiMeterSymbolFont(.body)"
            ),
        ]

        for expectation in expectations {
            let source = try viewSource(expectation.fileName)
            let fragment = try #require(
                sourceFragment(
                    after: expectation.imageExpression,
                    occurrence: expectation.occurrence,
                    in: source
                )
            )
            if let expectedModifier = expectation.expectedModifier {
                #expect(fragment.contains(expectedModifier))
                #expect(!fragment.contains(".aiMeterSymbolFont()"))
            } else {
                #expect(!fragment.contains(".aiMeterSymbolFont"))
            }
        }
    }

    @Test("Symbol token rendering retains caption2 and body baselines inside content")
    @MainActor
    func symbolTokenRendering() throws {
        let caption2 = try renderedAlphaBounds(
            Image(systemName: "info.circle")
                .aiMeterSymbolFont(.caption2)
                .foregroundStyle(.black)
                .frame(width: 80, height: 80)
                .aiMeterFontScope(.content(.system))
        )
        let body = try renderedAlphaBounds(
            Image(systemName: "info.circle")
                .aiMeterSymbolFont(.body)
                .foregroundStyle(.black)
                .frame(width: 80, height: 80)
                .aiMeterFontScope(.content(.system))
        )

        #expect(caption2.height < body.height)
        #expect(caption2.width < body.width)
    }

    @Test("Content unavailable symbols retain the system empty-state scale")
    @MainActor
    func contentUnavailableSymbolRendering() throws {
        let systemSized = try renderedAlphaBounds(
            ContentUnavailableView {
                Label {
                    EmptyView()
                } icon: {
                    Image(systemName: "gauge.with.dots.needle.50percent")
                }
            }
            .foregroundStyle(.black)
            .frame(width: 240, height: 160)
            .aiMeterFontScope(.content(.system))
        )
        #expect(systemSized.height > Int(AIMeterTextStyle.body.pointSize))
        #expect(systemSized.width > Int(AIMeterTextStyle.body.pointSize))
    }

    private struct SymbolSourceExpectation {
        let fileName: String
        let imageExpression: String
        var occurrence = 0
        let expectedModifier: String?
    }

    private func sourceFragment(
        after imageExpression: String,
        occurrence: Int,
        in source: String
    ) -> String? {
        var range = source.startIndex..<source.endIndex
        for _ in 0...occurrence {
            guard let match = source.range(of: imageExpression, range: range) else {
                return nil
            }
            range = match.upperBound..<source.endIndex
        }
        let suffix = source[range]
        return String(suffix.prefix(180))
    }

    @MainActor
    private func renderedAlphaBounds<V: View>(_ view: V) throws -> RenderedAlphaBounds {
        let renderer = ImageRenderer(content: view)
        renderer.scale = 1
        let image = try #require(renderer.cgImage)
        let bitmap = NSBitmapImageRep(cgImage: image)
        var minX = image.width
        var maxX = -1
        var minY = image.height
        var maxY = -1

        for y in 0..<image.height {
            for x in 0..<image.width {
                guard let color = bitmap.colorAt(x: x, y: y) else { continue }
                if color.alphaComponent > 0.01 {
                    minX = min(minX, x)
                    maxX = max(maxX, x)
                    minY = min(minY, y)
                    maxY = max(maxY, y)
                }
            }
        }

        guard maxX >= minX, maxY >= minY else {
            Issue.record("Rendered symbol did not produce visible pixels")
            return RenderedAlphaBounds(width: 0, height: 0)
        }
        return RenderedAlphaBounds(width: maxX - minX + 1, height: maxY - minY + 1)
    }

    private struct RenderedAlphaBounds {
        let width: Int
        let height: Int
    }
}
