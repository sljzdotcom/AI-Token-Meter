import type { ProviderId } from "./usage"
export type StripPreferences = {
  schemaVersion?: number
  density: "compact" | "comfortable"
  foldDelay: number
  orderedProviders: ProviderId[]
  hiddenProviders: ProviderId[]
  hiddenUntil: number | null
}
export const defaultStripPreferences: StripPreferences = {
  schemaVersion: 2, density: "compact", foldDelay: 0,
  orderedProviders: ["claude", "codex", "deepseek", "gemini"], hiddenProviders: [], hiddenUntil: null,
}
