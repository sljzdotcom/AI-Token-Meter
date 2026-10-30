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

// No Tauri command, event channel or website operation is installed here.
// The reviewed runtime adapter is a separate integration checkpoint.
export const disabledCode0SessionBridge: Code0SessionBridge = {
  async read() { return emptyCode0Session },
  async onUpdated() { return () => {} },
  async perform() { return {kind: "denied"} },
}

const sessionKeys = ["revision", "state", "failure", "isAutomaticallySyncing", "retryAfterSeconds", "snapshot"] as const
const snapshotKeys = ["schemaVersion", "providerId", "displayName", "status", "usedRatio", "primaryMetric", "secondaryMetric", "fetchedAt", "staleAfterSeconds", "sourceVersion"] as const
const metricKeys = ["label", "current", "limit", "unit", "kind", "resetAt", "resetDescription"] as const
const states = new Set<string>(code0SessionStates)
const failures = new Set<string>(code0SessionFailures)

export function decodeCode0Session(value: unknown): Code0SessionDTO | null {
  try {
    if (!exactRecord(value, sessionKeys) || !safeUnsignedInteger(value.revision)
      || typeof value.state !== "string" || !states.has(value.state)
      || !(value.failure === null || typeof value.failure === "string" && failures.has(value.failure))
      || typeof value.isAutomaticallySyncing !== "boolean"
      || !(value.retryAfterSeconds === null || safeUnsignedInteger(value.retryAfterSeconds) && value.retryAfterSeconds <= 300)) return null
    const snapshot = value.snapshot === null ? null : decodeSnapshot(value.snapshot)
    if (value.snapshot !== null && snapshot === null) return null
    return Object.freeze({
      revision: value.revision, state: value.state as Code0SessionState,
      failure: value.failure as Code0SessionFailure | null,
      isAutomaticallySyncing: value.isAutomaticallySyncing,
      retryAfterSeconds: value.retryAfterSeconds, snapshot,
    })
  } catch { return null }
}

function decodeSnapshot(value: unknown): Code0Snapshot | null {
  if (!exactRecord(value, snapshotKeys) || value.schemaVersion !== 1 || value.providerId !== "code0"
    || value.displayName !== "Code0" || (value.status !== "fresh" && value.status !== "cached")
    || value.usedRatio !== null || value.staleAfterSeconds !== 300 || value.sourceVersion !== "website"
    || typeof value.fetchedAt !== "string" || !validNativeTime(value.fetchedAt)) return null
  const primaryMetric = decodeMetric(value.primaryMetric, "Balance", "balance")
  const secondaryMetric = decodeMetric(value.secondaryMetric, "Cumulative usage", "cumulativeUsage")
  if (!primaryMetric || !secondaryMetric) return null
  return Object.freeze({
    schemaVersion: 1, providerId: "code0", displayName: "Code0", status: value.status, usedRatio: null,
    primaryMetric, secondaryMetric, fetchedAt: value.fetchedAt, staleAfterSeconds: 300, sourceVersion: "website",
  } as const)
}

function decodeMetric<Label extends string, Kind extends string>(value: unknown, label: Label, kind: Kind): Code0Metric<Label, Kind> | null {
  if (!exactRecord(value, metricKeys) || value.label !== label || value.kind !== kind
    || value.unit !== "siteCredits" || value.limit !== null || value.resetAt !== null || value.resetDescription !== null
    || typeof value.current !== "number" || !Number.isFinite(value.current) || value.current < 0) return null
  return Object.freeze({label, current: value.current, limit: null, unit: "siteCredits", kind, resetAt: null, resetDescription: null} as const)
}

function safeUnsignedInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0
}

function exactRecord(value: unknown, keys: readonly string[]): value is Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return false
  const prototype = Object.getPrototypeOf(value)
  if (prototype !== Object.prototype && prototype !== null) return false
  const actual = Reflect.ownKeys(value)
  return actual.length === keys.length && keys.every(key => {
    const descriptor = Object.getOwnPropertyDescriptor(value, key)
    return descriptor !== undefined && "value" in descriptor
  })
}

function validNativeTime(value: string) {
  return /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/.test(value)
    && Number.isFinite(Date.parse(value))
}
