import type { UsageSnapshot } from "../state/usage"

export const GEMINI_INSTALLATION_GUIDE_LABEL = "Gemini CLI 0.58.0 installation guide"
export const GEMINI_INSTALL_COMMAND = "npm install -g @google/gemini-cli@0.58.0"

export type GeminiSetupState = "connected" | "signInRequired" | "notInstalled" | "checking" | "unavailable"

export function geminiInstallationInstructions(state: GeminiSetupState): string[] {
  const signIn = "Run gemini and choose Sign in with Google."
  const finish = "Return to AI Token Meter and choose Check Status."
  if (state === "connected") return []
  if (state === "signInRequired") return [signIn, finish]
  return ["Requires Node.js 20 or later.", GEMINI_INSTALL_COMMAND, signIn, finish]
}

export function geminiSetupStateForSnapshot(status: UsageSnapshot["status"]): GeminiSetupState {
  if (status === "fresh" || status === "cached") return "connected"
  if (status === "authenticationRequired") return "signInRequired"
  if (status === "notInstalled") return "notInstalled"
  if (status === "refreshing") return "checking"
  return "unavailable"
}
