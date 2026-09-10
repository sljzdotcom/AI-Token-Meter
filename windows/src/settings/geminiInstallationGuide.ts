import type { UsageSnapshot } from "../state/usage"

export const GEMINI_INSTALLATION_GUIDE_LABEL = "Antigravity CLI installation guide"
export const GEMINI_INSTALL_COMMAND = "irm https://antigravity.google/cli/install.ps1 | iex"

export type GeminiSetupState = "connected" | "signInRequired" | "notInstalled" | "checking" | "unavailable"

export function geminiInstallationInstructions(state: GeminiSetupState): string[] {
  const signIn = "Run agy and complete Google sign-in."
  const finish = "Return to AI Token Meter and choose Check Status."
  if (state === "connected") return []
  if (state === "signInRequired") return [signIn, finish]
  return [GEMINI_INSTALL_COMMAND, signIn, finish]
}

export function geminiSetupStateForSnapshot(snapshot: Pick<UsageSnapshot, "status" | "statusMessage">): GeminiSetupState {
  if (snapshot.status === "fresh") return "connected"
  if (snapshot.status === "cached") {
    const message = snapshot.statusMessage?.toLowerCase() ?? ""
    return message.includes("sign in required") || message.includes("authentication required") ? "signInRequired" : "connected"
  }
  if (snapshot.status === "authenticationRequired") return "signInRequired"
  if (snapshot.status === "notInstalled") return "notInstalled"
  if (snapshot.status === "refreshing") return "checking"
  return "unavailable"
}
