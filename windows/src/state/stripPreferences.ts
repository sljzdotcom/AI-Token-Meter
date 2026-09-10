import type { ProviderId } from "./usage"
export type StripPreferences = {
  schemaVersion?: number
  density: "compact" | "comfortable"
  revealDelayMilliseconds: number
  collapseDelayMilliseconds: number
  orderedProviders: ProviderId[]
  hiddenProviders: ProviderId[]
  hiddenUntil: number | null
}
export const defaultStripPreferences: StripPreferences = {
  schemaVersion: 3, density: "compact", revealDelayMilliseconds: 150, collapseDelayMilliseconds: 800,
  orderedProviders: ["claude", "codex", "deepseek", "gemini"], hiddenProviders: [], hiddenUntil: null,
}
