import Foundation

public struct AntigravityModelCatalog: Equatable, Sendable {
    public let modelCount: Int
    public let families: [String]

    public init(modelCount: Int, families: [String]) {
        self.modelCount = modelCount
        self.families = families
    }
}

public enum AntigravityCLIInfoParser {
    private static let maximumRows = 64
    private static let maximumValueLength = 120
    private static let maximumFamilies = 16

    public static func currentGeminiModel(from text: String) -> String? {
        guard let rows = parsedRows(from: text, allowsBanner: false),
              rows.count == 1,
              isGemini(rows[0]) else { return nil }
        return rows[0].name
    }

    public static func geminiCatalog(from text: String) -> AntigravityModelCatalog? {
        guard let rows = parsedRows(from: text, allowsBanner: true) else { return nil }
        guard !rows.contains(where: { $0.id.hasPrefix("gemini-") && !isGemini($0) }) else {
            return nil
        }
        let gemini = rows.filter(isGemini)
        guard !gemini.isEmpty else { return nil }

        var families: [String] = []
        for model in gemini {
            let family = familyName(model.name)
            if !families.contains(family) {
                guard families.count < maximumFamilies else { return nil }
                families.append(family)
            }
        }
        return AntigravityModelCatalog(modelCount: gemini.count, families: families)
    }

    private static func parsedRows(
        from text: String,
        allowsBanner: Bool
    ) -> [(id: String, name: String)]? {
        let lines = text.components(separatedBy: .newlines)
            .map { ANSITextSanitizer.sanitize($0).trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
        guard !lines.isEmpty, lines.count <= maximumRows + 1 else { return nil }

        var rows: [(id: String, name: String)] = []
        for line in lines {
            if allowsBanner && line == "Fetching available models..." {
                continue
            }
            let columns = line.split(separator: "\t", omittingEmptySubsequences: false).map(String.init)
            guard columns.count == 2,
                  valid(columns[0]),
                  valid(columns[1]) else { return nil }
            rows.append((columns[0], columns[1]))
        }
        guard !rows.isEmpty, rows.count <= maximumRows else { return nil }
        return rows
    }

    private static func valid(_ value: String) -> Bool {
        !value.isEmpty
            && value.count <= maximumValueLength
            && value.unicodeScalars.allSatisfy { !CharacterSet.controlCharacters.contains($0) }
    }

    private static func isGemini(_ row: (id: String, name: String)) -> Bool {
        validGeminiModelID(row.id) && AntigravityCLIInfo.isValidGeminiDisplayName(row.name)
    }

    private static func validGeminiModelID(_ value: String) -> Bool {
        guard value.hasPrefix("gemini-"), value.count <= maximumValueLength,
              value.last?.isLetter == true || value.last?.isNumber == true,
              !value.contains("--"), !value.contains("..") else { return false }
        return value.unicodeScalars.allSatisfy { scalar in
            scalar.isASCII && (CharacterSet.lowercaseLetters.contains(scalar)
                || CharacterSet.decimalDigits.contains(scalar) || scalar == "-" || scalar == ".")
        }
    }

    private static func familyName(_ displayName: String) -> String {
        guard displayName.last == ")",
              let opening = displayName.lastIndex(of: "("),
              opening > displayName.startIndex else { return displayName }
        return displayName[..<opening].trimmingCharacters(in: .whitespaces)
    }
}
