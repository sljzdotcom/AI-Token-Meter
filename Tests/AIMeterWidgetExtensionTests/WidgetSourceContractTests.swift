import Foundation
import Testing

@Suite("Widget source contracts")
struct WidgetSourceContractTests {
    @Test("Small Widget remains logo-only and accessible")
    func smallWidgetSourceContract() throws {
        let source = try String(contentsOf: Self.sourceURL("Views/SmallWidgetView.swift"))

        #expect(!source.contains("Text("))
        #expect(!source.contains("Button("))
        #expect(!source.contains("Link("))
        #expect(source.contains("accessibilityLabel"))
        #expect(source.contains("WidgetProviderLogo"))
        #expect(source.contains("WidgetProgressRing"))
    }

    @Test("Widget extension never reaches protected or external data sources")
    func noDirectCollectionPaths() throws {
        let root = Self.projectRoot.appending(path: "Sources/AIMeterWidgetExtension")
        let enumerator = try #require(FileManager.default.enumerator(at: root, includingPropertiesForKeys: nil))
        let forbidden = ["URLSession", "Process(", "Keychain", "CommandRunner", "PTYCommandRunner"]

        for case let fileURL as URL in enumerator where fileURL.pathExtension == "swift" {
            let source = try String(contentsOf: fileURL)
            for token in forbidden {
                #expect(!source.contains(token), "Widget source must not contain \(token)")
            }
        }
    }

    private static func sourceURL(_ relativePath: String) -> URL {
        projectRoot
            .appending(path: "Sources/AIMeterWidgetExtension")
            .appending(path: relativePath)
    }

    private static let projectRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
}
