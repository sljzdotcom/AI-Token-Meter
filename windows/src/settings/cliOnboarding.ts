import type { ProviderId } from "../state/usage"
import type { ServiceAccountStatus } from "./SettingsWindow"

type CLIProvider = "claude" | "codex"
type Invoke = (command: string, args: { providerId: ProviderId; retryUsage?: boolean }) => Promise<unknown>

export function serviceAction(state: ServiceAccountStatus["connectionState"], busy: boolean) {
  return {
    title: busy ? "Waiting for Terminal…" : state === "notInstalled" ? "Install CLI" : state === "connected" ? "Sign in again" : state === "unavailable" ? "Check Status" : "Sign in",
    attention: !busy && (state === "notInstalled" || state === "signInRequired"),
    disabled: busy || state === "checking",
  }
}

/** Owns per-provider operations and rejects stale completions after close or newer checks. */
export class CLIOnboarding {
  private busy = new Set<ProviderId>()
  private revisions = new Map<ProviderId, number>()
  constructor(private invoke: Invoke, private apply: (status: ServiceAccountStatus) => void, private message: (value: string) => void, private changed: () => void, private sleep = () => new Promise<void>(resolve => setTimeout(resolve, 3000)), private attempts = 40) {}
  isBusy(provider: ProviderId) { return this.busy.has(provider) }
  dispose() { for (const p of ["claude", "codex", "deepseek"] as const) this.next(p); this.busy.clear() }
  private next(provider: ProviderId) { const n = (this.revisions.get(provider) ?? 0) + 1; this.revisions.set(provider, n); return n }
  private async read(providerId: ProviderId, retryUsage = false): Promise<ServiceAccountStatus> {
    try { return await this.invoke("service_account_status", { providerId, retryUsage }) as ServiceAccountStatus }
    catch { return { providerId, connectionState: "unavailable" } }
  }
  async check(providerId: ProviderId, retryUsage = false) {
    if (this.isBusy(providerId)) return
    const revision = this.next(providerId)
    this.busy.add(providerId); this.changed()
    this.apply({ providerId, connectionState: "checking" })
    const status = await this.read(providerId, retryUsage)
    if (this.revisions.get(providerId) !== revision) return
    this.apply(status); this.busy.delete(providerId); this.changed()
  }
  async initializeClaudeUsage() {
    const providerId = "claude" as const
    if (this.isBusy(providerId)) return
    const revision = this.next(providerId)
    const current = () => this.revisions.get(providerId) === revision
    this.busy.add(providerId); this.changed()
    try {
      const status = await this.read(providerId)
      if (!current()) return
      this.apply(status)
      if (status.connectionState !== "connected") return
      await this.invoke("begin_claude_usage_initialization", {providerId})
      if (current()) this.message("Complete Claude Code workspace setup in Terminal, then choose Check Status.")
    } catch {
      if (current()) this.message("The Claude Code setup window could not be opened. Choose Check Status or retry initialization.")
    } finally {
      if (current()) { this.busy.delete(providerId); this.changed() }
    }
  }
  async begin(providerId: CLIProvider, kind: "install" | "login") {
    if (this.isBusy(providerId)) return
    const revision = this.next(providerId)
    const current = () => this.revisions.get(providerId) === revision
    this.busy.add(providerId); this.changed()
    try {
      const original = await this.read(providerId)
      if (!current()) return
      this.apply(original)
      if (kind === "install" && original.connectionState !== "notInstalled") return
      if (kind === "login" && !["connected", "signInRequired"].includes(original.connectionState)) return
      const result = await this.invoke(kind === "install" ? "begin_service_installation" : "begin_service_sign_in", { providerId })
      if (!current()) return
      if (result === "unavailable") {
        this.apply({providerId, connectionState: "unavailable"})
        this.message("The CLI could not be checked. Choose Check Status or review the installation instructions.")
        return
      }
      if (result === "manualRequired") { this.message("Use the official instructions for WSL or correct the custom CLI path, then choose Check Status."); return }
      this.message(kind === "install" ? "Complete the official installation in Terminal. Status will update automatically." : "Complete sign-in in Terminal. Status will update automatically.")
      let disconnected = original.connectionState !== "connected"
      for (let i = 0; i < this.attempts; i++) {
        await this.sleep()
        if (!current()) return
        const status = await this.read(providerId)
        if (!current()) return
        this.apply(status)
        const connected = status.connectionState === "connected"
        if (kind === "install" ? connected || status.connectionState === "signInRequired" : connected && (disconnected || status.accountLabel !== original.accountLabel || status.accountDetail !== original.accountDetail)) {
          this.message(kind === "install" ? "CLI detected. You can now check the account or sign in." : "Account connected."); return
        }
        if (status.connectionState === "signInRequired") disconnected = true
      }
      this.message("The operation is not confirmed. Finish in Terminal, then choose Check Status or retry.")
    } catch { if (current()) this.message("The terminal could not be opened. Choose Check Status or retry.") }
    finally { if (current()) { this.busy.delete(providerId); this.changed() } }
  }
}
