import type { ProviderId, UsageStatus } from "../state/usage"

export type DetailRecovery = "services" | "geminiControls" | null

export function cachedStatusNeedsAction(statusMessage?: string | null) {
  const message = statusMessage?.toLowerCase() ?? ""
  return [
    "sign in required",
    "authentication required",
    "api key requires attention",
    "setup required",
    "cli not installed",
    "approve the private usage workspace",
  ].some(reason => message.includes(reason))
}

export function detailRecovery(providerId: ProviderId, status: UsageStatus, statusMessage?: string | null): DetailRecovery {
  if (providerId === "gemini") return "geminiControls"
  if (status === "cached") return cachedStatusNeedsAction(statusMessage) ? "services" : null
  if (["fresh", "refreshing"].includes(status)) return null
  return "services"
}
