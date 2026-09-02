import Foundation
import Testing

@Suite("GitHub Actions toolchain")
struct CIWorkflowTests {
    @Test("CI selects the Swift toolchain required by project language features")
    func selectsSupportedXcode() throws {
        let projectRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let workflow = try String(contentsOf: projectRoot.appending(
            path: ".github/workflows/ci.yml"
        ))

        #expect(workflow.contains("DEVELOPER_DIR: /Applications/Xcode_26.3.app/Contents/Developer"))
        #expect(workflow.contains("uses: actions/checkout@v5"))
        #expect(workflow.contains("run: swift --version"))
    }
}
