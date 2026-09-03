export type ProviderId = "claude" | "codex" | "deepseek"
export type UsageStatus =
  | "fresh"
  | "cached"
  | "refreshing"
  | "notInstalled"
  | "authenticationRequired"
  | "setupRequired"
  | "unavailable"
  | "unrecognizedOutput"

export type UsageMetric = {
  label: string
  current: number
  limit?: number | null
  unit: "percent" | "cny" | "usd" | "tokens" | "requests"
  kind: "officialLimit" | "balance" | "localBudget"
  resetAt?: string | null
  resetDescription?: string | null
}

export type UsageSnapshot = {
  schemaVersion: number
  providerId: ProviderId
  displayName: string
  status: UsageStatus
  usedRatio?: number | null
  primaryMetric?: UsageMetric | null
  secondaryMetric?: UsageMetric | null
  fetchedAt: string
  staleAfterSeconds: number
  sourceVersion?: string | null
  statusMessage?: string | null
  resetCredits?: Array<{ kind: "fullUsageReset"; count: number; expiresAt: string }>
  localActivity?: {
    periodDays: number
    sessions: number
    tokens: number
    activeDays: number
    longestSessionSeconds?: number | null
  } | null
  dailyHistory?: Array<{
    date: string
    costCny: number
    requests: number
    tokens: number
  }>
  historyFetchedAt?: string | null
}

export const unavailableSnapshots: UsageSnapshot[] = [
  ["claude", "Claude Code"],
  ["codex", "OpenAI Codex"],
  ["deepseek", "DeepSeek"],
].map(([providerId, displayName]) => ({
  schemaVersion: 1,
  providerId: providerId as ProviderId,
  displayName,
  status: "unavailable",
  usedRatio: null,
  fetchedAt: new Date(0).toISOString(),
  staleAfterSeconds: 300,
}))
