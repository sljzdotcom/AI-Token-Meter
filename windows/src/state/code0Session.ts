import type { UsageSnapshot } from "./usage"

export const code0SessionStates = ["disconnected", "connecting", "syncing", "closing", "active", "paused", "cleanupPending"] as const
export type Code0SessionState = typeof code0SessionStates[number]
export const code0SessionFailures = ["validationRequired", "profileBusy", "storageFailed", "cleanupFailed", "policyRejected", "navigationFailed", "invalidResponse", "authenticationChanged", "loginTimedOut", "syncTimedOut"] as const
export type Code0SessionFailure = typeof code0SessionFailures[number]
export type Code0Action = "connect" | "loginCompleted" | "syncNow" | "disconnect"
export type Code0ActionResult = Readonly<
  | {kind: "started"}
  | {kind: "busy"}
  | {kind: "throttled"; secondsRemaining: number}
  | {kind: "denied"}
>

type Code0Metric<Label extends string, Kind extends string> = Readonly<{
  label: Label
  current: number
  limit: null
  unit: "siteCredits"
  kind: Kind
  resetAt: null
  resetDescription: null
}>

// A numeric projection, not an account or webpage message. The native adapter
// selects these fields from its committed Code0 snapshot; no raw-body passthrough.
export type Code0Snapshot = Readonly<Pick<UsageSnapshot, "fetchedAt"> & {
  schemaVersion: 1
  providerId: "code0"
  displayName: "Code0"
  status: "fresh" | "cached"
  usedRatio: null
  primaryMetric: Code0Metric<"Balance", "balance">
  secondaryMetric: Code0Metric<"Cumulative usage", "cumulativeUsage">
  staleAfterSeconds: 300
  sourceVersion: "website"
}>

export type Code0SessionDTO = Readonly<{
  // Safe integer, monotonic within this native runtime. Same-revision conflicting
  // content is not a new publication. No epoch/nonce/account identity leaves native.
  revision: number
  state: Code0SessionState
  failure: Code0SessionFailure | null
  isAutomaticallySyncing: boolean
  // Presentation only. Native remains authoritative for every 300-second gate.
  retryAfterSeconds: number | null
  // Explicit null revokes displayed metrics and their successful time.
  snapshot: Code0Snapshot | null
}>

export interface Code0SessionBridge {
  read(): Promise<unknown>
  onUpdated(handler: (value: unknown) => void): Promise<() => void>
  perform(action: Code0Action): Promise<Code0ActionResult>
}

export const emptyCode0Session: Code0SessionDTO = Object.freeze({
  revision: 0, state: "disconnected", failure: null,
  isAutomaticallySyncing: false, retryAfterSeconds: null, snapshot: null,
} as const)

// First RED contract only. No Tauri command, event channel or website operation
// is installed until the later reviewed runtime integration checkpoint.
export const disabledCode0SessionBridge: Code0SessionBridge = {
  async read() { return emptyCode0Session },
  async onUpdated() { return () => {} },
  async perform() { return {kind: "denied"} },
}

export function decodeCode0Session(_value: unknown): Code0SessionDTO | null {
  // Deny-only scaffold; valid projections must produce behavioral RED, not a
  // missing type/import error. Unknown fields and failures never reach the UI.
  return null
}
