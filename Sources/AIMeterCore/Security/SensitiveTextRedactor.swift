import Foundation

public enum SensitiveTextRedactor {
    private static let patterns: [(String, String)] = [
        (#"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]{8,}"#, "Bearer [REDACTED]"),
        (#"(?i)\b(?:sk|dk)-[A-Za-z0-9_-]{8,}"#, "[REDACTED]"),
        (#"(?i)\bCookie:\s*[^\r\n]+"#, "Cookie: [REDACTED]"),
        (#"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b"#, "[REDACTED]"),
        (#"(?<!\d)1[3-9]\d{9}(?!\d)"#, "[REDACTED]"),
    ]

    public static func redact(_ text: String) -> String {
        patterns.reduce(text) { result, rule in
            guard let expression = try? NSRegularExpression(pattern: rule.0) else {
                return result
            }
            let range = NSRange(result.startIndex..., in: result)
            return expression.stringByReplacingMatches(
                in: result,
                range: range,
                withTemplate: rule.1
            )
        }
    }
}

extension UsageMetric {
    func privacySanitized() -> UsageMetric {
        UsageMetric(
            id: id,
            label: SensitiveTextRedactor.redact(label),
            current: current,
            limit: limit,
            unit: unit,
            kind: kind,
            resetAt: resetAt,
            resetDescription: resetDescription.map(SensitiveTextRedactor.redact)
        )
    }
}

extension UsageSnapshot {
    func privacySanitized() -> UsageSnapshot {
        UsageSnapshot(
            provider: provider,
            primaryMetric: primaryMetric?.privacySanitized(),
            secondaryMetric: secondaryMetric?.privacySanitized(),
            availability: availability,
            fetchedAt: fetchedAt,
            staleAfter: staleAfter,
            sourceVersion: sourceVersion.map(SensitiveTextRedactor.redact),
            collectionStatus: collectionStatus,
            statusMessage: statusMessage.map(SensitiveTextRedactor.redact),
            codexResetCredits: codexResetCredits?.privacySanitized(),
            codexLocalActivity: codexLocalActivity,
            claudeLocalActivity: claudeLocalActivity,
            deepSeekUsageHistory: deepSeekUsageHistory?.privacySanitized()
        )
    }
}

private extension CodexResetCreditsSummary {
    func privacySanitized() -> CodexResetCreditsSummary {
        CodexResetCreditsSummary(
            availableCount: availableCount,
            credits: credits.map {
                CodexResetCreditDisplay(
                    title: $0.title.map(SensitiveTextRedactor.redact),
                    expiresAt: $0.expiresAt
                )
            },
            hasCompleteDetails: hasCompleteDetails
        )
    }
}

private extension DeepSeekUsageHistory {
    func privacySanitized() -> DeepSeekUsageHistory {
        DeepSeekUsageHistory(
            days: days,
            updatedAt: updatedAt,
            statusMessage: statusMessage.map(SensitiveTextRedactor.redact)
        )
    }
}
