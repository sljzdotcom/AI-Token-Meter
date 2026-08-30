import Foundation

enum TerminalUsageParser {
    static func parse(_ rawText: String, provider: UsageProvider) throws -> UsageSnapshot {
        let text = ANSITextSanitizer.sanitize(rawText)
        let lines = text.components(separatedBy: .newlines)
        var candidateLabel: String?
        var metrics: [UsageMetric] = []

        for (index, originalLine) in lines.enumerated() {
            let line = normalizeLine(originalLine)
            guard !line.isEmpty else { continue }

            guard let percentRange = line.range(
                of: #"[0-9]+(?:\.[0-9]+)?\s*%"#,
                options: .regularExpression
            ) else {
                if !isResetLine(line), let label = meaningfulText(line) {
                    candidateLabel = label
                }
                continue
            }
            guard isUsagePercentLine(line) else { continue }

            let percentToken = line[percentRange]
                .replacingOccurrences(of: "%", with: "")
                .trimmingCharacters(in: .whitespaces)
            guard let displayedPercent = Double(percentToken) else { continue }

            let lowercased = line.lowercased()
            let isRemaining = ["remaining", "left", "剩余", "可用"].contains {
                lowercased.contains($0)
            }
            let usedPercent = isRemaining ? 100 - displayedPercent : displayedPercent
            let prefix = String(line[..<percentRange.lowerBound])
            let label = meaningfulText(prefix) ?? candidateLabel ?? defaultLabel(for: provider)
            let resetDescription = inlineResetDescription(in: line, after: percentRange)
                ?? followingResetDescription(lines: lines, after: index)

            let metric = UsageMetric(
                label: label,
                current: usedPercent,
                limit: 100,
                unit: .percent,
                resetDescription: resetDescription
            )

            if let existing = metrics.firstIndex(where: { $0.label == metric.label }) {
                metrics[existing] = metric
            } else {
                metrics.append(metric)
            }
        }

        guard let primaryMetric = metrics.first else {
            throw UsageCollectionError.unrecognizedOutput
        }

        return UsageSnapshot(
            provider: provider,
            primaryMetric: primaryMetric,
            secondaryMetric: metrics.dropFirst().first,
            availability: .available,
            collectionStatus: .fresh
        )
    }

    private static func normalizeLine(_ line: String) -> String {
        line.trimmingCharacters(in: .whitespacesAndNewlines)
            .trimmingCharacters(in: CharacterSet(charactersIn: "│┃╭╮╰╯"))
            .trimmingCharacters(in: .whitespaces)
    }

    private static func meaningfulText(_ text: String) -> String? {
        let trimmed = text
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .trimmingCharacters(in: CharacterSet(charactersIn: ":：|()[]-–—"))
            .trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return nil }
        guard trimmed.unicodeScalars.contains(where: CharacterSet.alphanumerics.contains) else {
            return nil
        }
        return trimmed
    }

    private static func isResetLine(_ line: String) -> Bool {
        line.range(of: "reset", options: .caseInsensitive) != nil || line.contains("重置")
    }

    private static func isUsagePercentLine(_ line: String) -> Bool {
        let lowercased = line.lowercased()
        return ["used", "remaining", "left"].contains(where: lowercased.contains)
            || ["已用", "使用", "剩余", "可用"].contains(where: line.contains)
    }

    private static func followingResetDescription(lines: [String], after index: Int) -> String? {
        guard index + 1 < lines.count else { return nil }
        for nextIndex in (index + 1)..<min(lines.count, index + 3) {
            let next = normalizeLine(lines[nextIndex])
            if next.isEmpty { continue }
            return isResetLine(next) ? next : nil
        }
        return nil
    }

    private static func inlineResetDescription(
        in line: String,
        after percentRange: Range<String.Index>
    ) -> String? {
        let suffix = String(line[percentRange.upperBound...])
        if let resetRange = suffix.range(of: #"resets?"#, options: [.regularExpression, .caseInsensitive]) {
            return String(suffix[resetRange.lowerBound...])
                .trimmingCharacters(in: CharacterSet(charactersIn: " ()[]"))
        }
        if suffix.contains("重置") {
            return suffix.trimmingCharacters(in: CharacterSet(charactersIn: " ()[]"))
        }
        return nil
    }

    private static func defaultLabel(for provider: UsageProvider) -> String {
        switch provider {
        case .claude: "Claude usage"
        case .codex: "Codex usage"
        case .deepSeek: "DeepSeek usage"
        }
    }
}
