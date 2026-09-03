import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

import type { ProviderId, UsageSnapshot, UsageStatus } from "./usage"

const providers = new Set<ProviderId>(["claude", "codex", "deepseek"])
const statuses = new Set<UsageStatus>([
  "fresh",
  "cached",
  "refreshing",
  "notInstalled",
  "authenticationRequired",
  "setupRequired",
  "unavailable",
  "unrecognizedOutput",
])

export interface UsageBridge {
  readSnapshots(): Promise<UsageSnapshot[]>
  onSnapshotUpdated(handler: (snapshot: UsageSnapshot) => void): Promise<UnlistenFn>
}

export class TauriUsageBridge implements UsageBridge {
  async readSnapshots(): Promise<UsageSnapshot[]> {
    const value = await invoke<unknown>("usage_snapshots")
    return Array.isArray(value) ? value.map(decodeSnapshot).filter(isPresent) : []
  }

  async onSnapshotUpdated(handler: (snapshot: UsageSnapshot) => void): Promise<UnlistenFn> {
    return listen<unknown>("snapshot-updated", (event) => {
      const snapshot = decodeSnapshot(event.payload)
      if (snapshot) handler(snapshot)
    })
  }
}

function decodeSnapshot(value: unknown): UsageSnapshot | null {
  if (!isRecord(value)) return null
  if (!providers.has(value.providerId as ProviderId) || !statuses.has(value.status as UsageStatus)) return null
  if (typeof value.displayName !== "string" || typeof value.fetchedAt !== "string") return null
  if (typeof value.schemaVersion !== "number" || typeof value.staleAfterSeconds !== "number") return null
  if (value.usedRatio != null && (typeof value.usedRatio !== "number" || value.usedRatio < 0 || value.usedRatio > 1)) return null
  return value as UsageSnapshot
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null
}

function isPresent<T>(value: T | null): value is T {
  return value !== null
}
