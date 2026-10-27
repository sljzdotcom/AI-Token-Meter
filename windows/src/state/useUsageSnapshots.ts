import { useEffect, useState } from "react"

import type { ProviderId, UsageSnapshot } from "./usage"
import { unavailableSnapshots } from "./usage"
import type { UsageBridge } from "./usageBridge"
import { emptyCode0Session, type Code0SessionDTO } from "./code0Session"

export function useUsageSnapshots(bridge: UsageBridge, initialSnapshots?: UsageSnapshot[], code0Session: Code0SessionDTO = emptyCode0Session) {
  const [snapshots, setSnapshots] = useState(() => ordinary(initialSnapshots ?? unavailableSnapshots))

  useEffect(() => {
    if (initialSnapshots) {
      setSnapshots(ordinary(initialSnapshots))
      return
    }
    let disposed = false
    let unlisten: (() => void) | undefined
    const updated = new Set<ProviderId>()
    const attach = async () => {
      try {
        const stop = await bridge.onSnapshotUpdated((snapshot) => {
          if (disposed || !isOrdinary(snapshot.providerId)) return
          updated.add(snapshot.providerId)
          setSnapshots((current) => ordered(upsert(current, snapshot)))
        })
        if (disposed) { stop(); return }
        unlisten = stop
      } catch { /* A failed subscription does not prevent a passive read. */ }
      if (disposed) return
      try {
        const values = ordinary(await bridge.readSnapshots())
        if (!disposed && values.length) setSnapshots(current => ordered([
          ...values.filter(value => !updated.has(value.providerId)),
          ...current.filter(value => updated.has(value.providerId)),
        ]))
      } catch { /* Keep the current ordinary projection. */ }
    }
    void attach()
    return () => {
      disposed = true
      unlisten?.()
    }
  }, [bridge, initialSnapshots])

  // Only the independent session may supply Code0 numbers. Clone its narrow
  // immutable projection at this compatibility boundary; null always clears it.
  const value = code0Session.snapshot
  const code0: UsageSnapshot = value ? {
    ...value, primaryMetric: {...value.primaryMetric}, secondaryMetric: {...value.secondaryMetric},
  } : {...unavailableSnapshots.find(snapshot => snapshot.providerId === "code0")!}
  return ordered([...ordinary(snapshots), code0])
}

function isOrdinary(provider: ProviderId) {
  return provider === "claude" || provider === "codex" || provider === "deepseek" || provider === "gemini"
}

function ordinary(snapshots: UsageSnapshot[]) {
  return snapshots.filter(snapshot => isOrdinary(snapshot.providerId))
}

function upsert(snapshots: UsageSnapshot[], replacement: UsageSnapshot) {
  return [...snapshots.filter(snapshot => snapshot.providerId !== replacement.providerId), replacement]
}

function ordered(snapshots: UsageSnapshot[]) {
  const order = { claude: 0, codex: 1, deepseek: 2, gemini: 3, code0: 4 } as const
  return [...snapshots].sort((left, right) => order[left.providerId] - order[right.providerId])
}
