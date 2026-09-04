import Foundation
import Testing

@Suite("Cross-platform product contract")
struct CrossPlatformContractTests {
    @Test("The shared version matches the macOS bundle version")
    func sharedVersionMatchesBundle() throws {
        let sharedVersion = try String(
            contentsOf: Self.repositoryRoot.appending(path: "VERSION"),
            encoding: .utf8
        ).trimmingCharacters(in: .whitespacesAndNewlines)
        let plistData = try Data(
            contentsOf: Self.repositoryRoot.appending(
                path: "Sources/AIMeterApp/Resources/Info.plist"
            )
        )
        let plist = try #require(
            PropertyListSerialization.propertyList(from: plistData, format: nil)
                as? [String: Any]
        )

        #expect(sharedVersion == "0.3.0-preview.3")
        #expect(sharedVersion == plist["CFBundleShortVersionString"] as? String)

        let packageData = try Data(
            contentsOf: Self.repositoryRoot.appending(path: "windows/package.json")
        )
        let package = try #require(
            JSONSerialization.jsonObject(with: packageData) as? [String: Any]
        )
        let tauriData = try Data(
            contentsOf: Self.repositoryRoot.appending(
                path: "windows/src-tauri/tauri.conf.json"
            )
        )
        let tauri = try #require(
            JSONSerialization.jsonObject(with: tauriData) as? [String: Any]
        )
        let cargo = try String(
            contentsOf: Self.repositoryRoot.appending(
                path: "windows/src-tauri/Cargo.toml"
            ),
            encoding: .utf8
        )

        #expect(package["version"] as? String == sharedVersion)
        #expect(tauri["version"] as? String == sharedVersion)
        #expect(cargo.contains("\nversion = \"\(sharedVersion)\"\n"))
    }

    @Test("The presentation contract preserves provider identity and progress semantics")
    func providerPresentationContract() throws {
        let data = try Data(
            contentsOf: Self.repositoryRoot.appending(
                path: "contracts/presentation/providers.json"
            )
        )
        let contract = try JSONDecoder().decode(PresentationContract.self, from: data)

        #expect(contract.schemaVersion == 1)
        #expect(contract.providers.map(\.id) == ["claude", "codex", "deepseek"])
        #expect(
            contract.providers.map(\.displayName)
                == ["Claude Code", "OpenAI Codex", "DeepSeek"]
        )
        #expect(
            contract.providers.map(\.progressSemantics)
                == ["usedQuota", "usedQuota", "consumedFromBalanceBaseline"]
        )
        #expect(Set(contract.providers.map(\.logoKey)).count == 3)
    }

    @Test("Every fixture conforms to the stable snapshot envelope")
    func fixtureEnvelope() throws {
        let fixtureDirectory = Self.repositoryRoot.appending(
            path: "contracts/fixtures",
            directoryHint: .isDirectory
        )
        let fixtureURLs = try FileManager.default.contentsOfDirectory(
            at: fixtureDirectory,
            includingPropertiesForKeys: nil
        ).filter { $0.pathExtension == "json" }

        #expect(fixtureURLs.count == 4)
        for fixtureURL in fixtureURLs {
            let fixture = try JSONDecoder().decode(
                SnapshotFixture.self,
                from: Data(contentsOf: fixtureURL)
            )
            #expect(fixture.schemaVersion == 1)
            #expect(["claude", "codex", "deepseek"].contains(fixture.providerId))
            #expect(Self.allowedStatuses.contains(fixture.status))
            if let usedRatio = fixture.usedRatio {
                #expect(usedRatio >= 0 && usedRatio <= 1)
            }
            #expect(ISO8601DateFormatter().date(from: fixture.fetchedAt) != nil)
        }
    }

    private struct PresentationContract: Decodable {
        let schemaVersion: Int
        let providers: [Provider]
    }

    private struct Provider: Decodable {
        let id: String
        let displayName: String
        let logoKey: String
        let progressSemantics: String
    }

    private struct SnapshotFixture: Decodable {
        let schemaVersion: Int
        let providerId: String
        let status: String
        let usedRatio: Double?
        let fetchedAt: String
    }

    private static let allowedStatuses: Set<String> = [
        "fresh",
        "cached",
        "refreshing",
        "notInstalled",
        "authenticationRequired",
        "setupRequired",
        "unavailable",
        "unrecognizedOutput",
    ]

    private static let repositoryRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
}
