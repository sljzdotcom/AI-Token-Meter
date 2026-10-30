import type { ProviderId } from "./usage"
export type StripPreferences = {
  schemaVersion?: number
  density: "comfortable" | "compact"
  automaticallyCollapses: boolean
  revealDelayMilliseconds: number
  collapseDelayMilliseconds: number
  orderedProviders: ProviderId[]
  hiddenProviders: ProviderId[]
  hiddenUntil: number | null
}
export const defaultStripPreferences: StripPreferences = {
  schemaVersion: 5, density: "compact", automaticallyCollapses: true,
  revealDelayMilliseconds: 150, collapseDelayMilliseconds: 800,
  orderedProviders: ["claude", "codex", "deepseek", "gemini", "code0"], hiddenProviders: ["code0"], hiddenUntil: null,
}
