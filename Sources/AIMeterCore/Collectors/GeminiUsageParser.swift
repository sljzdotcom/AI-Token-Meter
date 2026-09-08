import Foundation

/// Parses a visible, complete model dialog, never an accumulated terminal transcript.
public struct GeminiUsageParser: Sendable {
    public init() {}
    public func parse(_ text: String) throws -> UsageSnapshot {
        let lines = text.components(separatedBy: .newlines).map {
            $0.trimmingCharacters(in: .whitespaces).trimmingCharacters(in: CharacterSet(charactersIn: "│")).trimmingCharacters(in: .whitespaces)
        }
        guard let title = lines.firstIndex(of: "Select Model"),
              let end = lines[title...].firstIndex(of: "(Press Esc to close)"),
              lines.dropFirst(end + 1).contains(where: { $0.hasPrefix("╰") && $0.hasSuffix("╯") }),
              lines.filter({ $0 == "Select Model" }).count == 1 else { throw UsageCollectionError.unrecognizedOutput }
        guard let start = lines[title..<end].firstIndex(of: "Model usage") else {
            throw UsageCollectionError.geminiUnavailable("Gemini CLI did not provide quota")
        }
        let order = ["Pro", "Flash", "Flash Lite"]
        let pattern = #"^(Flash Lite|Flash|Pro)\s+[▬━─\s]*([0-9]{1,3})%(?:\s+(Resets: .+))?$"#
        let regex = try NSRegularExpression(pattern: pattern)
        var metrics: [UsageMetric] = []
        for line in lines[(start + 1)..<end] where !line.isEmpty {
            let range = NSRange(line.startIndex..., in: line)
            guard let match = regex.firstMatch(in: line, range: range),
                  let labelRange = Range(match.range(at: 1), in: line),
                  let valueRange = Range(match.range(at: 2), in: line),
                  let value = Double(line[valueRange]), value <= 100 else { throw UsageCollectionError.unrecognizedOutput }
            let label = String(line[labelRange])
            guard !metrics.contains(where: { $0.label == label }) else { throw UsageCollectionError.unrecognizedOutput }
            let reset = Range(match.range(at: 3), in: line).map { String(line[$0]) }
            metrics.append(UsageMetric(label: label, current: value, limit: 100, unit: .percent, resetDescription: reset))
        }
        guard !metrics.isEmpty else { throw UsageCollectionError.unrecognizedOutput }
        metrics.sort { order.firstIndex(of: $0.label)! < order.firstIndex(of: $1.label)! }
        let ranked = metrics.enumerated().sorted { a, b in a.element.current == b.element.current ? a.offset < b.offset : a.element.current > b.element.current }.map(\.element)
        return UsageSnapshot(provider: .gemini, primaryMetric: ranked[0], secondaryMetric: ranked.dropFirst().first,
                             sourceVersion: "0.58.0", geminiQuotaMetrics: metrics)
    }
}
