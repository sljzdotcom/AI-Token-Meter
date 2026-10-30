import { vi } from "vitest"
import type { Code0ActionResult, Code0SessionBridge, Code0SessionDTO, Code0Snapshot } from "../state/code0Session"

export function code0Snapshot(balance = 2.5, cumulativeUsage = 1.5): Code0Snapshot {
  return {
    schemaVersion: 1, providerId: "code0", displayName: "Code0", status: "cached", usedRatio: null,
    primaryMetric: {label: "Balance", current: balance, limit: null, unit: "siteCredits", kind: "balance", resetAt: null, resetDescription: null},
    secondaryMetric: {label: "Cumulative usage", current: cumulativeUsage, limit: null, unit: "siteCredits", kind: "cumulativeUsage", resetAt: null, resetDescription: null},
    fetchedAt: "2026-10-27T04:00:00Z", staleAfterSeconds: 300, sourceVersion: "website",
  }
}

export function code0Session(overrides: Partial<Code0SessionDTO> = {}): Code0SessionDTO {
  return {revision: 1, state: "paused", failure: null, isAutomaticallySyncing: false,
    retryAfterSeconds: null, snapshot: code0Snapshot(), ...overrides}
}

export function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: unknown) => void
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return {promise, resolve, reject}
}

export function code0BridgeFixture(initial: Promise<unknown> = Promise.resolve(code0Session())) {
  const handlers = new Set<(value: unknown) => void>()
  const stop = vi.fn()
  const bridge: Code0SessionBridge = {
    read: vi.fn(() => initial),
    onUpdated: vi.fn(async (handler: (value: unknown) => void) => {
      handlers.add(handler)
      return () => { handlers.delete(handler); stop() }
    }),
    perform: vi.fn(async (): Promise<Code0ActionResult> => ({kind: "denied"})),
  }
  return {bridge, stop, emit(value: unknown) { handlers.forEach(handler => handler(value)) }}
}
