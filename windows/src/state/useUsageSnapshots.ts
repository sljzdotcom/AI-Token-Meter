import { useEffect, useState } from "react"

import type { UsageSnapshot } from "./usage"
import { unavailableSnapshots } from "./usage"
import type { UsageBridge } from "./usageBridge"
import type { Code0SessionDTO } from "./code0Session"

export function useUsageSnapshots(bridge: UsageBridge, initialSnapshots?: UsageSnapshot[], _code0Session?: Code0SessionDTO) {
  const [snapshots, setSnapshots] = useState(initialSnapshots ?? unavailableSnapshots)

  useEffect(() => {
    if (initialSnapshots) {
      setSnapshots(initialSnapshots)
      return
    }
    let disposed = false
    let unlisten: (() => void) | undefined
    bridge.readSnapshots().then((values) => {
      if (!disposed && values.length) setSnapshots(ordered(values))
    }).catch(() => {})
    bridge.onSnapshotUpdated((snapshot) => {
      if (!disposed) setSnapshots((current) => ordered(upsert(current, snapshot)))
    }).then((stop) => {
      if (disposed) stop()
      else unlisten = stop
    }).catch(() => {})
    return () => {
      disposed = true
      unlisten?.()
    }
  }, [bridge, initialSnapshots])

  return snapshots
}

function upsert(snapshots: UsageSnapshot[], replacement: UsageSnapshot) {
  return snapshots.map((snapshot) => snapshot.providerId === replacement.providerId ? replacement : snapshot)
}

function ordered(snapshots: UsageSnapshot[]) {
  const order = { claude: 0, codex: 1, deepseek: 2, gemini: 3, code0: 4 } as const
  return [...snapshots].sort((left, right) => order[left.providerId] - order[right.providerId])
}
